//! Rewrite relative resource links in a rendered flat-kind body so they
//! address the per-item `<name>/` namespace directory.
//!
//! Used only for flat outputs (Claude/Copilot rules, all agents).
//! Directory-backed kinds (all skills, opencode rules) render the
//! entrypoint inside the resource directory, so their relative links
//! already resolve and this module is not invoked for them.

use anyhow::Result;
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::collections::HashSet;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};

/// Prefix every relative link/image destination that resolves to one of
/// `copied` (paths relative to the item directory) with `name/`, so the
/// destination addresses the namespaced resource directory. Destinations
/// that are URLs, absolute paths, bare fragments, `../`-escaping, or not
/// among `copied` are left unchanged — as is any link-like text inside
/// inline-code spans or fenced code blocks (pulldown-cmark reports those
/// as non-link events, so they never enter `edits`).
pub fn rewrite_resource_links(
    rendered: &str,
    name: &str,
    copied: &HashSet<PathBuf>,
) -> Result<String> {
    if copied.is_empty() {
        return Ok(rendered.to_string());
    }
    Ok(apply_dest_edits(rendered, |dest| {
        rewritten_dest(dest, name, copied)
    }))
}

/// Re-prefix already-namespaced resource links from `<old_name>/` to
/// `<new_name>/`. Used when an item that ships supporting resources is
/// relocated by `--as`: its resource directory moves from `<old_name>/` to
/// `<new_name>/`, so the entrypoint links — which a prior
/// [`rewrite_resource_links`] pass pointed at `<old_name>/` — must follow.
/// `copied` holds the resource paths relative to the (relocated) resource
/// directory; only destinations whose remainder resolves to one of them are
/// rewritten, so a coincidental `<old_name>/…` link that is not a resource is
/// left intact.
pub fn reprefix_resource_links(
    rendered: &str,
    old_name: &str,
    new_name: &str,
    copied: &HashSet<PathBuf>,
) -> Result<String> {
    if copied.is_empty() || old_name == new_name {
        return Ok(rendered.to_string());
    }
    Ok(apply_dest_edits(rendered, |dest| {
        reprefixed_dest(dest, old_name, new_name, copied)
    }))
}

/// Scan `rendered` for link, image, and reference-definition destinations,
/// ask `remap` for each one's replacement, and splice the accepted
/// replacements in. Inline-code and fenced-code spans are reported by
/// pulldown-cmark as non-link events, so they never reach `remap`.
fn apply_dest_edits(rendered: &str, remap: impl Fn(&str) -> Option<String>) -> String {
    // (absolute byte range of the destination token, replacement string).
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();

    // A single parse serves both passes: `OffsetIter` exposes the reference
    // definitions gathered during construction, and yields the inline link
    // and image events when iterated.
    let parser = Parser::new_ext(rendered, Options::empty()).into_offset_iter();

    // Reference definitions (`[id]: dest "title"`) are consumed during the
    // initial scan and are not emitted as link events, so collect them
    // separately. `RefDef.span` covers the whole definition line.
    for (_label, def) in parser.reference_definitions().iter() {
        if let Some(new_dest) = remap(&def.dest)
            && let Some(off) = locate_dest(&rendered[def.span.clone()], &def.dest, true)
        {
            let abs = def.span.start + off;
            edits.push((abs..abs + def.dest.len(), new_dest));
        }
    }

    // Inline links and images.
    for (event, range) in parser {
        let dest = match &event {
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. }) => dest_url.to_string(),
            _ => continue,
        };
        if let Some(new_dest) = remap(&dest)
            && let Some(off) = locate_dest(&rendered[range.clone()], &dest, false)
        {
            let abs = range.start + off;
            edits.push((abs..abs + dest.len(), new_dest));
        }
    }

    // Apply right-to-left so earlier ranges stay valid.
    edits.sort_by_key(|e| std::cmp::Reverse(e.0.start));
    let mut out = rendered.to_string();
    for (range, replacement) in edits {
        out.replace_range(range, &replacement);
    }
    out
}

/// Byte offset of the destination within a link or reference-definition
/// source `span`. Anchors on the structural marker that precedes the
/// destination — `](` for inline links/images, `]:` for reference
/// definitions — so a link title or link text that happens to repeat the
/// destination string cannot cause a mis-splice. Returns `None` when the
/// anchor isn't found or the destination isn't where expected (the caller
/// then leaves the link unchanged rather than risk corrupting it).
fn locate_dest(span: &str, dest: &str, is_refdef: bool) -> Option<usize> {
    let mut pos = if is_refdef {
        span.find("]:")? + 2
    } else {
        span.rfind("](")? + 2
    };
    // Skip optional whitespace after the marker, then an optional opening
    // angle bracket (`[t](<dest>)` / `[id]: <dest>`).
    pos += span[pos..].len() - span[pos..].trim_start().len();
    if span[pos..].starts_with('<') {
        pos += 1;
    }
    span[pos..].starts_with(dest).then_some(pos)
}

/// Returns `Some(new_dest)` when `dest` is a relative path (optionally
/// with a `#fragment`) that resolves to a copied resource, else `None`.
fn rewritten_dest(dest: &str, name: &str, copied: &HashSet<PathBuf>) -> Option<String> {
    let (path_part, frag) = match dest.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (dest, None),
    };
    if path_part.is_empty() || has_scheme(path_part) || path_part.starts_with('/') {
        return None;
    }
    let normalized = path_part.strip_prefix("./").unwrap_or(path_part);
    // pulldown-cmark does not percent-decode `dest_url`, so a link like
    // `[n](my%20notes.md)` arrives literally. Decode for the path-semantic
    // checks below (parent-escape guard and `copied` membership, which holds
    // real decoded filenames) while keeping the original encoded text for
    // the rewritten output so the on-disk link stays valid.
    let decoded = percent_decode(normalized);
    if Path::new(&decoded)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return None;
    }
    if !copied.contains(&PathBuf::from(&decoded)) {
        return None;
    }
    let prefixed = if path_part.starts_with("./") {
        format!("./{name}/{normalized}")
    } else {
        format!("{name}/{normalized}")
    };
    Some(match frag {
        Some(f) => format!("{prefixed}#{f}"),
        None => prefixed,
    })
}

/// Best-effort percent-decoding of a link-destination path. Each valid
/// `%XX` escape becomes its byte; a malformed escape or a decoded byte
/// sequence that is not UTF-8 is left verbatim. Used only to compare a
/// destination against `copied` (which stores real, decoded filenames) —
/// never to build output.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            )
        {
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

/// Returns `Some(new_dest)` when `dest` is a relative path (optionally with a
/// `#fragment`) already namespaced under `old_name/` whose remainder is a
/// copied resource, with the leading `old_name/` segment swapped for
/// `new_name/`; else `None`. The `/` after `old_name` is required, so a
/// sibling prefix like `old_name-2/…` is not mistaken for a match.
fn reprefixed_dest(
    dest: &str,
    old_name: &str,
    new_name: &str,
    copied: &HashSet<PathBuf>,
) -> Option<String> {
    let (path_part, frag) = match dest.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (dest, None),
    };
    if path_part.is_empty() || has_scheme(path_part) || path_part.starts_with('/') {
        return None;
    }
    let had_dot_slash = path_part.starts_with("./");
    let normalized = path_part.strip_prefix("./").unwrap_or(path_part);
    // Strip the existing `<old_name>/` namespace segment, keeping the encoded
    // remainder for output. Decode it only for the path-semantic checks below
    // (parent-escape guard and `copied` membership, which holds real decoded
    // filenames), mirroring `rewritten_dest`.
    let rest = normalized
        .strip_prefix(old_name)
        .and_then(|r| r.strip_prefix('/'))?;
    let decoded = percent_decode(rest);
    if Path::new(&decoded)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return None;
    }
    if !copied.contains(&PathBuf::from(&decoded)) {
        return None;
    }
    let prefixed = if had_dot_slash {
        format!("./{new_name}/{rest}")
    } else {
        format!("{new_name}/{rest}")
    };
    Some(match frag {
        Some(f) => format!("{prefixed}#{f}"),
        None => prefixed,
    })
}

/// True for `scheme:` URLs (`https:`, `mailto:`, …). A relative file path
/// has no leading `scheme:` segment.
fn has_scheme(s: &str) -> bool {
    match s.find(':') {
        Some(idx) if idx > 0 => s[..idx]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copied(paths: &[&str]) -> HashSet<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    fn run(body: &str, paths: &[&str]) -> String {
        rewrite_resource_links(body, "demo", &copied(paths)).unwrap()
    }

    #[test]
    fn rewrites_inline_link_with_dot_slash() {
        assert_eq!(
            run("See [g](./scripts/gate.sh).", &["scripts/gate.sh"]),
            "See [g](./demo/scripts/gate.sh)."
        );
    }

    #[test]
    fn rewrites_inline_link_without_dot_slash() {
        assert_eq!(
            run("See [g](scripts/gate.sh).", &["scripts/gate.sh"]),
            "See [g](demo/scripts/gate.sh)."
        );
    }

    #[test]
    fn rewrites_image() {
        assert_eq!(
            run("![logo](./assets/logo.png)", &["assets/logo.png"]),
            "![logo](./demo/assets/logo.png)"
        );
    }

    #[test]
    fn rewrites_reference_definition() {
        let body = "Use [the gate][g].\n\n[g]: ./scripts/gate.sh\n";
        assert_eq!(
            run(body, &["scripts/gate.sh"]),
            "Use [the gate][g].\n\n[g]: ./demo/scripts/gate.sh\n"
        );
    }

    #[test]
    fn preserves_title_and_fragment() {
        assert_eq!(
            run(
                "[p](./refs/patterns.md#sec \"Patterns\")",
                &["refs/patterns.md"]
            ),
            "[p](./demo/refs/patterns.md#sec \"Patterns\")"
        );
    }

    #[test]
    fn leaves_urls_untouched() {
        let body = "[site](https://example.com/scripts/gate.sh) and [m](mailto:x@y.z)";
        assert_eq!(run(body, &["scripts/gate.sh"]), body);
    }

    #[test]
    fn leaves_absolute_path_untouched() {
        let body = "[g](/usr/local/bin/gate.sh)";
        assert_eq!(run(body, &["usr/local/bin/gate.sh"]), body);
    }

    #[test]
    fn leaves_bare_fragment_untouched() {
        let body = "[top](#section)";
        assert_eq!(run(body, &["section"]), body);
    }

    #[test]
    fn leaves_parent_escape_untouched() {
        let body = "[x](../other/gate.sh)";
        assert_eq!(run(body, &["other/gate.sh"]), body);
    }

    #[test]
    fn rewrites_percent_encoded_link() {
        // pulldown does not percent-decode `dest_url`, so `my%20notes.md`
        // must be decoded to match the copied resource `my notes.md`. The
        // rewritten destination keeps the original encoding.
        assert_eq!(
            run("See [n](my%20notes.md).", &["my notes.md"]),
            "See [n](demo/my%20notes.md)."
        );
    }

    #[test]
    fn rewrites_percent_encoded_link_with_dot_slash() {
        assert_eq!(
            run("![d](./a%20b/c.png)", &["a b/c.png"]),
            "![d](./demo/a%20b/c.png)"
        );
    }

    #[test]
    fn rewrites_percent_encoded_reference_definition() {
        let body = "Use [n][r].\n\n[r]: my%20notes.md\n";
        assert_eq!(
            run(body, &["my notes.md"]),
            "Use [n][r].\n\n[r]: demo/my%20notes.md\n"
        );
    }

    #[test]
    fn leaves_percent_encoded_parent_escape_untouched() {
        // `..%2Fgate.sh` decodes to `../gate.sh`: the parent-escape guard
        // must reject it rather than rewrite a path that climbs out of the
        // item directory.
        let body = "[x](..%2Fother%2Fgate.sh)";
        assert_eq!(run(body, &["other/gate.sh"]), body);
    }

    #[test]
    fn leaves_uncopied_target_untouched() {
        // Link resolves to a relative path, but it was not among the copied
        // resources (e.g. points at a sibling item) — do not rewrite.
        let body = "[x](./scripts/missing.sh)";
        assert_eq!(run(body, &["scripts/gate.sh"]), body);
    }

    #[test]
    fn leaves_inline_code_untouched() {
        let body = "Run `./scripts/gate.sh` manually.";
        assert_eq!(run(body, &["scripts/gate.sh"]), body);
    }

    #[test]
    fn leaves_fenced_code_untouched() {
        let body = "```sh\n./scripts/gate.sh\n```\n";
        assert_eq!(run(body, &["scripts/gate.sh"]), body);
    }

    #[test]
    fn empty_copied_set_is_identity() {
        let body = "[g](./scripts/gate.sh)";
        assert_eq!(
            rewrite_resource_links(body, "demo", &copied(&[])).unwrap(),
            body
        );
    }

    #[test]
    fn idempotent() {
        let once = run("[g](./scripts/gate.sh)", &["scripts/gate.sh"]);
        let twice = run(&once, &["scripts/gate.sh"]);
        assert_eq!(once, twice);
    }

    #[test]
    fn rewrites_url_not_title_when_title_repeats_dest() {
        // The title string is byte-identical to the destination: only the
        // URL must be rewritten, the title left intact.
        assert_eq!(
            run(
                "[g](scripts/gate.sh \"scripts/gate.sh\")",
                &["scripts/gate.sh"]
            ),
            "[g](demo/scripts/gate.sh \"scripts/gate.sh\")"
        );
    }

    #[test]
    fn rewrites_url_not_text_when_link_text_repeats_dest() {
        // The link text is byte-identical to the destination.
        assert_eq!(
            run("[scripts/gate.sh](scripts/gate.sh)", &["scripts/gate.sh"]),
            "[scripts/gate.sh](demo/scripts/gate.sh)"
        );
    }

    #[test]
    fn rewrites_refdef_url_not_title_when_title_repeats_dest() {
        assert_eq!(
            run(
                "[g]: scripts/gate.sh \"scripts/gate.sh\"\n",
                &["scripts/gate.sh"]
            ),
            "[g]: demo/scripts/gate.sh \"scripts/gate.sh\"\n"
        );
    }

    fn reprefix(body: &str, paths: &[&str]) -> String {
        reprefix_resource_links(body, "demo", "renamed", &copied(paths)).unwrap()
    }

    #[test]
    fn reprefix_swaps_namespaced_segment() {
        assert_eq!(
            reprefix("See [g](./demo/scripts/gate.sh).", &["scripts/gate.sh"]),
            "See [g](./renamed/scripts/gate.sh)."
        );
    }

    #[test]
    fn reprefix_without_dot_slash() {
        assert_eq!(
            reprefix("[g](demo/scripts/gate.sh)", &["scripts/gate.sh"]),
            "[g](renamed/scripts/gate.sh)"
        );
    }

    #[test]
    fn reprefix_preserves_fragment() {
        assert_eq!(
            reprefix("[p](./demo/refs/p.md#sec)", &["refs/p.md"]),
            "[p](./renamed/refs/p.md#sec)"
        );
    }

    #[test]
    fn reprefix_percent_encoded_remainder() {
        // The namespaced remainder is percent-decoded to match the copied
        // resource `my notes.md`; the rewritten destination keeps the encoding.
        assert_eq!(
            reprefix("[n](./demo/my%20notes.md)", &["my notes.md"]),
            "[n](./renamed/my%20notes.md)"
        );
    }

    #[test]
    fn reprefix_leaves_non_resource_coincidence_untouched() {
        // `demo/other.md` is namespaced under `demo/` but is not a copied
        // resource, so re-prefixing must not touch it.
        let body = "[x](./demo/other.md)";
        assert_eq!(reprefix(body, &["scripts/gate.sh"]), body);
    }

    #[test]
    fn reprefix_leaves_sibling_prefix_untouched() {
        // `demo-2/` shares a textual prefix with `demo` but is a different
        // namespace segment; the required `/` boundary protects it.
        let body = "[x](./demo-2/scripts/gate.sh)";
        assert_eq!(reprefix(body, &["scripts/gate.sh"]), body);
    }

    #[test]
    fn reprefix_identity_when_names_equal() {
        let body = "[g](./demo/scripts/gate.sh)";
        assert_eq!(
            reprefix_resource_links(body, "demo", "demo", &copied(&["scripts/gate.sh"])).unwrap(),
            body
        );
    }
}
