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

    // (absolute byte range of the destination token, replacement string).
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();

    // Reference definitions (`[id]: dest "title"`) are consumed during the
    // initial scan and are not emitted as link events, so collect them
    // separately. `RefDef.span` covers the whole definition line.
    let ref_parser = Parser::new_ext(rendered, Options::empty());
    for (_label, def) in ref_parser.reference_definitions().iter() {
        if let Some(new_dest) = rewritten_dest(&def.dest, name, copied)
            && let Some(off) = rendered[def.span.clone()].rfind(def.dest.as_ref())
        {
            let abs = def.span.start + off;
            edits.push((abs..abs + def.dest.len(), new_dest));
        }
    }

    // Inline links and images.
    let parser = Parser::new_ext(rendered, Options::empty());
    for (event, range) in parser.into_offset_iter() {
        let dest = match &event {
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. }) => dest_url.to_string(),
            _ => continue,
        };
        if let Some(new_dest) = rewritten_dest(&dest, name, copied)
            && let Some(off) = rendered[range.clone()].rfind(&dest)
        {
            let abs = range.start + off;
            edits.push((abs..abs + dest.len(), new_dest));
        }
    }

    // Apply right-to-left so earlier ranges stay valid.
    edits.sort_by(|a, b| b.0.start.cmp(&a.0.start));
    let mut out = rendered.to_string();
    for (range, replacement) in edits {
        out.replace_range(range, &replacement);
    }
    Ok(out)
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
    if Path::new(normalized)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return None;
    }
    if !copied.contains(&PathBuf::from(normalized)) {
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
}
