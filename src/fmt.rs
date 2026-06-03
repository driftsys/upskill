//! `upskill fmt` — canonicalise YAML key order in SSOT files.
//!
//! Per [ADR-0004](../../docs/adr/0004-cli-surface.md), `fmt` and `lint`
//! are sibling author commands. `fmt` operates on YAML frontmatter and
//! bundle files; markdown body content is dprint's job and is preserved
//! byte-for-byte. Like `lint`, this command refuses to run inside a
//! consumer project (`.upskill-lock.json` at the path's root).
//!
//! What gets canonicalised:
//!
//! - **Key order** — fixed by priority tables derived from the
//!   [`crate::model`] struct field order.
//! - Unknown top-level keys (extras) sort alphabetically after all
//!   known keys.
//!
//! What is **preserved**:
//!
//! - YAML comments (both standalone and inline)
//! - Author formatting (indentation, line wrapping)
//! - Nested/indented content (travels with its parent key block)
//!
//! Implementation: line-level block reordering. The YAML is split into
//! top-level key blocks (each key + preceding comments + indented
//! children), reordered by canonical priority, and reassembled. Serde
//! is used only for **validation** (parse the result, don't serialize).

use anyhow::{Context, Result, anyhow};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::{Path, PathBuf};

use crate::lint::{discover, is_consumer_project};
use crate::model::{Agent, Bundle, Rule, Skill};
use crate::parse::frontmatter;

// ── Canonical key-order tables (mirror struct field declarations) ────

const BUNDLE_KEY_ORDER: &[&str] = &[
    "schema",
    "name",
    "description",
    "license",
    "items",
    "requires",
    "plugins",
    "mcps",
    "metadata",
];

const SKILL_KEY_ORDER: &[&str] = &[
    "schema",
    "name",
    "description",
    "audience",
    "license",
    "metadata",
    "requires",
    "ignore",
    "claude",
    "copilot",
    "opencode",
];

const RULE_KEY_ORDER: &[&str] = &[
    "schema",
    "name",
    "description",
    "audience",
    "license",
    "scope",
    "metadata",
    "requires",
    "ignore",
    "claude",
    "copilot",
    "opencode",
];

const AGENT_KEY_ORDER: &[&str] = &[
    "schema",
    "name",
    "description",
    "audience",
    "license",
    "mode",
    "model",
    "tools",
    "preload-skills",
    "metadata",
    "requires",
    "ignore",
    "claude",
    "copilot",
    "opencode",
];

// ── Public API ──────────────────────────────────────────────────────

/// Outcome of one `upskill fmt` run.
#[derive(Debug, Default, Clone)]
pub struct FmtReport {
    /// Files whose on-disk content differed from the canonical form
    /// and were rewritten.
    pub files_changed: Vec<PathBuf>,
    /// Total entrypoint files inspected.
    pub files_checked: usize,
}

/// Canonicalise YAML key order in every SSOT entrypoint discovered
/// under `paths`. With an empty `paths` slice, defaults to the current
/// working directory.
///
/// Files whose content was already canonical are left untouched
/// (no `mtime` thrash). Body content is preserved byte-for-byte.
/// Comments and author formatting are preserved.
pub fn fmt(paths: &[PathBuf]) -> Result<FmtReport> {
    let owned_cwd: Vec<PathBuf>;
    let roots: &[PathBuf] = if paths.is_empty() {
        owned_cwd = vec![std::env::current_dir().context("get current directory")?];
        &owned_cwd
    } else {
        paths
    };

    for root in roots {
        if is_consumer_project(root) {
            return Err(anyhow!(
                "{}: refusing to format — `.upskill-lock.json` indicates this is a consumer \
                 project, not a source registry. Run `upskill fmt` inside the SSOT tree instead.",
                root.display()
            ));
        }
    }

    let mut report = FmtReport::default();
    for root in roots {
        for file in discover(root)? {
            report.files_checked += 1;
            if format_file(&file)? {
                report.files_changed.push(file);
            }
        }
    }
    Ok(report)
}

// ── Internal ────────────────────────────────────────────────────────

/// Format one entrypoint file in place. Returns `true` if the file
/// changed on disk.
fn format_file(path: &Path) -> Result<bool> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let canonical = canonicalise(&raw, path)?;
    if canonical == raw {
        return Ok(false);
    }
    fs::write(path, &canonical).with_context(|| format!("write {}", path.display()))?;
    Ok(true)
}

/// Produce the canonical form of `raw` for the kind inferred from
/// `path`'s filename. Pure function — no I/O, no mutation.
///
/// Items (Skill/Rule/Agent) are frontmatter+body — body is preserved
/// byte-for-byte. Bundles are pure YAML (§2.2, ADR-0007), so the file
/// content is the canonical YAML verbatim, no `---` wrapping.
fn canonicalise(raw: &str, path: &Path) -> Result<String> {
    let kind = file_kind(path)
        .ok_or_else(|| anyhow!("{}: unknown entrypoint filename", path.display()))?;

    match kind {
        EntryKind::Bundle => {
            let reordered = reorder_yaml_keys(raw, BUNDLE_KEY_ORDER);
            validate::<Bundle>(&reordered)
                .with_context(|| format!("validate bundle {}", path.display()))?;
            Ok(reordered)
        }
        EntryKind::Skill => canonicalise_item::<Skill>(raw, path, SKILL_KEY_ORDER),
        EntryKind::Rule => canonicalise_item::<Rule>(raw, path, RULE_KEY_ORDER),
        EntryKind::Agent => canonicalise_item::<Agent>(raw, path, AGENT_KEY_ORDER),
    }
}

/// Canonicalise an item file (frontmatter + body).
fn canonicalise_item<T: DeserializeOwned>(
    raw: &str,
    path: &Path,
    key_order: &[&str],
) -> Result<String> {
    let (yaml_str, body) = frontmatter::split(raw)
        .ok_or_else(|| anyhow!("{}: missing YAML frontmatter", path.display()))?;

    let reordered_yaml = reorder_yaml_keys(yaml_str, key_order);

    // Validate the reordered YAML parses correctly.
    serde_yaml_ng::from_str::<T>(&reordered_yaml)
        .with_context(|| format!("validate frontmatter {}", path.display()))?;

    let body_region = canonical_body(&reordered_yaml, body)
        .with_context(|| format!("format body {}", path.display()))?;

    Ok(format!("---\n{reordered_yaml}---\n{body_region}"))
}

/// Canonical body region for an item file, given its on-disk
/// frontmatter string and body. Formats via dprint the same way the
/// generation pipeline does: dprint treats `---…---` as opaque
/// frontmatter, so the YAML (content, comments, key order, wrapping) is
/// preserved byte-for-byte and only the body is formatted, with the seam
/// normalised to a single blank line. Returns the region after the
/// closing `---`, including that leading blank-line separator. Empty when
/// the body is blank. Used by both `fmt` (to write) and `lint::check_body_format`
/// (to detect drift) so checker and fixer never disagree.
pub(crate) fn canonical_body(frontmatter: &str, body: &str) -> Result<String> {
    if body.trim().is_empty() {
        return Ok(String::new());
    }
    let prefix = format!("---\n{frontmatter}---\n");
    let combined = format!("{prefix}{body}");
    let formatted =
        crate::generate::format::format_markdown(&combined).context("format item body")?;
    formatted
        .strip_prefix(&prefix)
        .map(|region| region.to_string())
        .ok_or_else(|| anyhow!("dprint altered the frontmatter region while formatting the body"))
}

/// Parse YAML string into `T` for validation only. Discards the result.
fn validate<T: DeserializeOwned>(yaml: &str) -> Result<()> {
    serde_yaml_ng::from_str::<T>(yaml).context("YAML validation failed")?;
    Ok(())
}

// ── Line-level key reordering ───────────────────────────────────────

/// A block of lines associated with one top-level YAML key (or a
/// preamble/trailer with no key).
#[derive(Debug)]
struct KeyBlock {
    /// The top-level key name, or `None` for preamble/trailer.
    key: Option<String>,
    /// The raw lines (including the key line, preceding comments, and
    /// indented children). Includes trailing newlines.
    lines: String,
}

/// Reorder top-level YAML keys according to `key_order`, preserving
/// comments and formatting. Keys not in `key_order` sort
/// alphabetically after all known keys.
///
/// Algorithm:
/// 1. Split into top-level key blocks (key line + preceding comments +
///    indented children)
/// 2. Sort blocks by priority (known keys by position in `key_order`,
///    unknown keys alphabetically after)
/// 3. Reassemble
pub fn reorder_yaml_keys(yaml: &str, key_order: &[&str]) -> String {
    let blocks = parse_into_blocks(yaml);

    if blocks.is_empty() {
        return yaml.to_string();
    }

    // Separate preamble (comments before first key) from key blocks.
    let mut preamble: Option<&KeyBlock> = None;
    let mut key_blocks: Vec<&KeyBlock> = Vec::new();

    for block in &blocks {
        if block.key.is_none() && key_blocks.is_empty() {
            preamble = Some(block);
        } else {
            key_blocks.push(block);
        }
    }

    // Sort key blocks by canonical order.
    key_blocks.sort_by(|a, b| {
        let priority_a = block_sort_key(a, key_order);
        let priority_b = block_sort_key(b, key_order);
        priority_a.cmp(&priority_b)
    });

    // Reassemble.
    let mut out = String::with_capacity(yaml.len());

    if let Some(pre) = preamble {
        out.push_str(&pre.lines);
    }

    for block in &key_blocks {
        out.push_str(&block.lines);
    }

    // Ensure file ends with exactly one newline.
    if !out.ends_with('\n') {
        out.push('\n');
    }

    out
}

/// Parse YAML text into blocks. Each block is either:
/// - A preamble (comment/blank lines separated from the first key by a
///   blank line)
/// - A key block (comment lines preceding the key + the key line +
///   indented continuation lines)
///
/// Heuristic for pre-first-key content: if there's a blank line between
/// the accumulated comments and the first key, everything up to and
/// including the last blank line is preamble; the rest attaches to the
/// key. If there's no blank line, all comments attach to the key.
fn parse_into_blocks(yaml: &str) -> Vec<KeyBlock> {
    let mut blocks: Vec<KeyBlock> = Vec::new();
    let mut current_comments = String::new();
    let mut current_key: Option<String> = None;
    let mut current_lines = String::new();
    let mut seen_first_key = false;

    for line in yaml.lines() {
        let line_with_nl = format!("{line}\n");

        if let Some(key) = extract_top_level_key(line) {
            // We hit a new top-level key. Flush the previous block.
            if let Some(prev_key) = current_key.take() {
                blocks.push(KeyBlock {
                    key: Some(prev_key),
                    lines: current_lines,
                });
            } else if !seen_first_key {
                // First key — split accumulated content into preamble
                // vs key-attached comments.
                let accumulated = format!("{current_lines}{current_comments}");
                let (preamble, attached) = split_preamble(&accumulated);
                if !preamble.is_empty() {
                    blocks.push(KeyBlock {
                        key: None,
                        lines: preamble,
                    });
                }
                current_comments = attached;
            }

            seen_first_key = true;

            // Start new key block with any pending comments.
            current_key = Some(key);
            current_lines = format!("{current_comments}{line_with_nl}");
            current_comments = String::new();
        } else if current_key.is_some() {
            // Inside a key block. Check if this is a comment/blank that
            // might belong to the NEXT key, or indented content.
            if is_comment_or_blank(line) && !is_indented(line) {
                // Could be inter-block comment — buffer it.
                current_comments.push_str(&line_with_nl);
            } else if is_indented(line) {
                // Indented content belongs to current key.
                // Flush any buffered comments first (they were
                // within this block after all).
                current_lines.push_str(&current_comments);
                current_comments = String::new();
                current_lines.push_str(&line_with_nl);
            } else {
                // Non-indented, non-comment, non-key line — unusual
                // but treat as continuation of current block.
                current_lines.push_str(&current_comments);
                current_comments = String::new();
                current_lines.push_str(&line_with_nl);
            }
        } else {
            // Before any key — accumulate.
            current_lines.push_str(&line_with_nl);
        }
    }

    // Flush final block.
    if let Some(key) = current_key {
        // Trailing comments after the last key stay with that key.
        current_lines.push_str(&current_comments);
        blocks.push(KeyBlock {
            key: Some(key),
            lines: current_lines,
        });
    } else if !current_lines.is_empty() || !current_comments.is_empty() {
        blocks.push(KeyBlock {
            key: None,
            lines: format!("{current_lines}{current_comments}"),
        });
    }

    blocks
}

/// Split pre-first-key content into (preamble, key-attached comments).
///
/// If the content contains a blank line, everything up to and including
/// the last blank line is preamble. Comments after the last blank line
/// attach to the first key. If no blank line exists, everything
/// attaches to the first key (no preamble).
fn split_preamble(content: &str) -> (String, String) {
    if content.is_empty() {
        return (String::new(), String::new());
    }

    // Find the last blank line position.
    let lines: Vec<&str> = content.lines().collect();
    let last_blank = lines.iter().rposition(|l| l.trim().is_empty());

    match last_blank {
        Some(pos) => {
            let mut preamble = String::new();
            let mut attached = String::new();
            for (i, line) in lines.iter().enumerate() {
                if i <= pos {
                    preamble.push_str(line);
                    preamble.push('\n');
                } else {
                    attached.push_str(line);
                    attached.push('\n');
                }
            }
            (preamble, attached)
        }
        None => {
            // No blank line — everything attaches to the key.
            (String::new(), content.to_string())
        }
    }
}

/// Extract a top-level key name from a line. A top-level key is an
/// unindented line matching `^[a-zA-Z_][a-zA-Z0-9_-]*:`.
fn extract_top_level_key(line: &str) -> Option<String> {
    // Must start at column 0 with a letter or underscore.
    let first = line.as_bytes().first()?;
    if !first.is_ascii_alphabetic() && *first != b'_' {
        return None;
    }

    // Find the colon.
    let colon_pos = line.find(':')?;
    let key = &line[..colon_pos];

    // Validate key characters: [a-zA-Z0-9_-]
    if key
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        Some(key.to_string())
    } else {
        None
    }
}

/// True if the line is a comment (`# ...`) or blank/whitespace-only.
fn is_comment_or_blank(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

/// True if the line starts with whitespace (indented content).
fn is_indented(line: &str) -> bool {
    line.starts_with(' ') || line.starts_with('\t')
}

/// Compute a sort key for a block. Known keys get their index in
/// `key_order`; unknown keys get `key_order.len()` plus their
/// alphabetical position among unknowns.
fn block_sort_key(block: &KeyBlock, key_order: &[&str]) -> (usize, String) {
    match &block.key {
        Some(key) => {
            if let Some(pos) = key_order.iter().position(|k| *k == key.as_str()) {
                (pos, String::new())
            } else {
                // Unknown key — sort alphabetically after known keys.
                (key_order.len(), key.clone())
            }
        }
        None => {
            // Preamble/trailer — should not appear in sorted list,
            // but if it does, put it first.
            (0, String::new())
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum EntryKind {
    Skill,
    Rule,
    Agent,
    Bundle,
}

fn file_kind(path: &Path) -> Option<EntryKind> {
    match path.file_name().and_then(|n| n.to_str())? {
        "SKILL.md" => Some(EntryKind::Skill),
        "RULE.md" => Some(EntryKind::Rule),
        "AGENT.md" => Some(EntryKind::Agent),
        n if n.ends_with(crate::parse::bundle::BUNDLE_SUFFIX) => Some(EntryKind::Bundle),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    // ── reorder_yaml_keys unit tests ────────────────────────────────

    #[test]
    fn reorder_preserves_comments() {
        let input = concat!(
            "# This is the name\n",
            "name: example\n",
            "schema: 1\n",
            "description: a bundle.\n",
            "items: {}\n",
        );
        let out = reorder_yaml_keys(input, BUNDLE_KEY_ORDER);
        assert!(out.contains("# This is the name\nname: example\n"));
        // schema should come before name
        let schema_pos = out.find("schema:").unwrap();
        let name_pos = out.find("name:").unwrap();
        assert!(schema_pos < name_pos, "schema before name:\n{out}");
    }

    #[test]
    fn reorder_preserves_inline_comments() {
        let input = concat!(
            "schema: 1\n",
            "name: test # inline comment\n",
            "description: foo.\n",
            "items: {}\n",
        );
        let out = reorder_yaml_keys(input, BUNDLE_KEY_ORDER);
        assert!(
            out.contains("name: test # inline comment"),
            "inline comment lost:\n{out}"
        );
    }

    #[test]
    fn reorder_moves_blocks_to_canonical_order() {
        let input = concat!(
            "metadata:\n",
            "  version: 0.1.0\n",
            "name: scrambled\n",
            "schema: 1\n",
            "description: test.\n",
            "items: {}\n",
        );
        let out = reorder_yaml_keys(input, BUNDLE_KEY_ORDER);
        let s = out.find("schema:").unwrap();
        let n = out.find("name:").unwrap();
        let d = out.find("description:").unwrap();
        let i = out.find("items:").unwrap();
        let m = out.find("metadata:").unwrap();
        assert!(s < n && n < d && d < i && i < m, "wrong order:\n{out}");
    }

    #[test]
    fn reorder_preserves_indented_children() {
        let input = concat!(
            "items:\n",
            "  skills:\n",
            "    - foo\n",
            "    - bar\n",
            "schema: 1\n",
            "name: test\n",
            "description: d.\n",
        );
        let out = reorder_yaml_keys(input, BUNDLE_KEY_ORDER);
        assert!(
            out.contains("items:\n  skills:\n    - foo\n    - bar\n"),
            "indented children lost:\n{out}"
        );
    }

    #[test]
    fn reorder_unknown_keys_sort_alphabetically_at_end() {
        let input = concat!(
            "schema: 1\n",
            "name: test\n",
            "description: d.\n",
            "items: {}\n",
            "zebra: z\n",
            "alpha: a\n",
        );
        let out = reorder_yaml_keys(input, BUNDLE_KEY_ORDER);
        let alpha_pos = out.find("alpha:").unwrap();
        let zebra_pos = out.find("zebra:").unwrap();
        let items_pos = out.find("items:").unwrap();
        assert!(items_pos < alpha_pos, "extras after known keys:\n{out}");
        assert!(alpha_pos < zebra_pos, "extras alphabetical:\n{out}");
    }

    #[test]
    fn reorder_requires_ignore_after_metadata_before_claude() {
        // requires/ignore deliberately scrambled: ignore before metadata,
        // requires after a claude passthrough block.
        let input = concat!(
            "schema: 1\n",
            "name: test\n",
            "description: d.\n",
            "ignore:\n",
            "  - \"*.tmp\"\n",
            "metadata:\n",
            "  version: 0.1.0\n",
            "claude:\n",
            "  body: hi\n",
            "requires:\n",
            "  skills:\n",
            "    - foo\n",
        );
        for key_order in [RULE_KEY_ORDER, SKILL_KEY_ORDER, AGENT_KEY_ORDER] {
            let out = reorder_yaml_keys(input, key_order);
            let metadata = out.find("metadata:").unwrap();
            let requires = out.find("requires:").unwrap();
            let ignore = out.find("ignore:").unwrap();
            let claude = out.find("claude:").unwrap();
            assert!(
                metadata < requires && metadata < ignore,
                "requires/ignore must come after metadata:\n{out}"
            );
            assert!(
                requires < claude && ignore < claude,
                "requires/ignore must come before claude:\n{out}"
            );
        }
    }

    #[test]
    fn reorder_is_idempotent() {
        let input = concat!(
            "# A comment\n",
            "name: test\n",
            "schema: 1\n",
            "# Description comment\n",
            "description: d.\n",
            "items:\n",
            "  skills:\n",
            "    - foo\n",
        );
        let pass1 = reorder_yaml_keys(input, BUNDLE_KEY_ORDER);
        let pass2 = reorder_yaml_keys(&pass1, BUNDLE_KEY_ORDER);
        assert_eq!(pass1, pass2, "reorder must be idempotent");
    }

    #[test]
    fn reorder_preamble_stays_at_top() {
        let input = concat!(
            "# File-level comment\n",
            "# Another preamble line\n",
            "\n",
            "name: test\n",
            "schema: 1\n",
            "description: d.\n",
            "items: {}\n",
        );
        let out = reorder_yaml_keys(input, BUNDLE_KEY_ORDER);
        assert!(
            out.starts_with("# File-level comment\n# Another preamble line\n\n"),
            "preamble moved:\n{out}"
        );
    }

    #[test]
    fn reorder_with_multiline_values() {
        let input = concat!(
            "description: >\n",
            "  A long description that spans\n",
            "  multiple lines using folded style.\n",
            "schema: 1\n",
            "name: test\n",
            "items: {}\n",
        );
        let out = reorder_yaml_keys(input, BUNDLE_KEY_ORDER);
        // description block should stay together
        assert!(
            out.contains(
                "description: >\n  A long description that spans\n  multiple lines using folded style.\n"
            ),
            "multiline value broken:\n{out}"
        );
        // schema should come before description
        let s = out.find("schema:").unwrap();
        let d = out.find("description:").unwrap();
        assert!(s < d, "schema before description:\n{out}");
    }

    // ── canonicalise integration tests ──────────────────────────────

    #[test]
    fn canonicalise_reorders_keys() {
        let raw = concat!(
            "---\n",
            "name: scrambled\n",
            "schema: 1\n",
            "description: shuffled keys.\n",
            "license: proprietary\n",
            "---\n",
            "## body\n",
        );
        let out = canonicalise(raw, Path::new("scrambled/SKILL.md")).unwrap();
        let yaml = &out[4..out[4..].find("\n---\n").unwrap() + 4];
        let s = yaml.find("schema:").unwrap();
        let n = yaml.find("name:").unwrap();
        let d = yaml.find("description:").unwrap();
        let l = yaml.find("license:").unwrap();
        assert!(s < n && n < d && d < l, "wrong order:\n{yaml}");
    }

    #[test]
    fn canonicalise_formats_body() {
        // `*` bullets become `-`, extra blank lines collapse, and the
        // single blank-line separator after the frontmatter is kept.
        let raw = concat!(
            "---\n",
            "schema: 1\n",
            "name: dirty\n",
            "description: body needs formatting.\n",
            "---\n",
            "\n",
            "## Body\n",
            "\n",
            "\n",
            "* one\n",
            "* two\n",
        );
        let out = canonicalise(raw, Path::new("dirty/SKILL.md")).unwrap();
        assert!(
            out.ends_with("---\n\n## Body\n\n- one\n- two\n"),
            "body not formatted canonically:\n{out}"
        );
    }

    #[test]
    fn canonicalise_is_idempotent() {
        let raw = concat!(
            "---\n",
            "name: out\n",
            "schema: 1\n",
            "description: shuffled.\n",
            "---\n",
            "## body\n",
        );
        let path = Path::new("out/SKILL.md");
        let pass1 = canonicalise(raw, path).unwrap();
        let pass2 = canonicalise(&pass1, path).unwrap();
        assert_eq!(pass1, pass2, "fmt must be idempotent");
    }

    #[test]
    fn canonicalise_preserves_frontmatter_comments() {
        let raw = concat!(
            "---\n",
            "schema: 1\n",
            "name: test\n",
            "# This explains the description\n",
            "description: a skill with comments.\n",
            "---\n",
            "## body\n",
        );
        let out = canonicalise(raw, Path::new("test/SKILL.md")).unwrap();
        assert!(
            out.contains("# This explains the description"),
            "comment stripped:\n{out}"
        );
    }

    #[test]
    fn canonicalise_preserves_bundle_comments() {
        let raw = concat!(
            "schema: 1\n",
            "name: test\n",
            "description: a bundle.\n",
            "items:\n",
            "  skills:\n",
            "    - foo\n",
            "# Plugin documentation\n",
            "# explaining why this exists\n",
            "plugins:\n",
            "  superpowers:\n",
            "    claude:\n",
            "      source: marketplace\n",
            "      plugin: superpowers\n",
        );
        let out = canonicalise(raw, Path::new("test.bundle.yaml")).unwrap();
        assert!(
            out.contains("# Plugin documentation\n# explaining why this exists\nplugins:"),
            "bundle comments stripped:\n{out}"
        );
    }

    #[test]
    fn canonicalise_preserves_description_wrapping() {
        let raw = concat!(
            "---\n",
            "schema: 1\n",
            "name: test\n",
            "description: Use when authoring or editing upskill .bundle.yaml\n",
            "  manifests — declaring items, plugins, requires dependencies.\n",
            "---\n",
            "## body\n",
        );
        let out = canonicalise(raw, Path::new("test/SKILL.md")).unwrap();
        assert!(
            out.contains(
                "description: Use when authoring or editing upskill .bundle.yaml\n  manifests"
            ),
            "wrapping destroyed:\n{out}"
        );
    }

    // ── fmt end-to-end tests ────────────────────────────────────────

    #[test]
    fn fmt_skips_already_canonical_files() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("clean/SKILL.md");
        let canonical = concat!(
            "---\n",
            "schema: 1\n",
            "name: clean\n",
            "description: already canonical.\n",
            "---\n",
            "\n",
            "## body\n",
        );
        write(&item, canonical);
        let mtime_before = fs::metadata(&item).unwrap().modified().unwrap();

        let report = fmt(&[tmp.path().to_path_buf()]).unwrap();
        assert!(report.files_changed.is_empty(), "{report:?}");

        let mtime_after = fs::metadata(&item).unwrap().modified().unwrap();
        assert_eq!(mtime_before, mtime_after, "mtime should not change");
    }

    #[test]
    fn fmt_refuses_consumer_project() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join(".upskill-lock.json"),
            r#"{"schema":2,"items":[]}"#,
        )
        .unwrap();
        let err = fmt(&[tmp.path().to_path_buf()]).expect_err("must refuse");
        assert!(format!("{err:#}").contains("consumer project"));
    }

    #[test]
    fn fmt_handles_rule_with_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("api/RULE.md");
        write(
            &item,
            concat!(
                "---\n",
                "name: api\n",
                "schema: 1\n",
                "description: rule with scope.\n",
                "scope:\n",
                "  paths:\n",
                "    - \"src/**/*.ts\"\n",
                "---\n",
                "## body\n",
            ),
        );
        let report = fmt(&[tmp.path().to_path_buf()]).unwrap();
        assert_eq!(report.files_changed, vec![item.clone()]);
        let after = fs::read_to_string(&item).unwrap();
        // scope must survive the round-trip.
        assert!(after.contains("src/**/*.ts"), "scope.paths lost:\n{after}");
    }

    #[test]
    fn reorder_bundle_mcps_after_plugins_before_metadata() {
        // mcps deliberately placed out of canonical order (before plugins).
        let input = concat!(
            "schema: 1\n",
            "name: test\n",
            "description: d.\n",
            "mcps:\n",
            "  drawio:\n",
            "    remote:\n",
            "      type: http\n",
            "      url: https://example.com/mcp\n",
            "metadata:\n",
            "  version: 0.1.0\n",
            "plugins:\n",
            "  sp:\n",
            "    vscode:\n",
            "      extension: pub.ext\n",
        );
        let out = reorder_yaml_keys(input, BUNDLE_KEY_ORDER);
        let plugins = out.find("plugins:").unwrap();
        let mcps = out.find("mcps:").unwrap();
        let metadata = out.find("metadata:").unwrap();
        assert!(
            plugins < mcps && mcps < metadata,
            "bundle key order must be plugins < mcps < metadata:\n{out}"
        );
    }

    #[test]
    fn fmt_handles_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("bundles/baseline.bundle.yaml");
        write(
            &item,
            concat!(
                "name: baseline\n",
                "schema: 1\n",
                "description: a bundle.\n",
                "items:\n",
                "  rules:\n",
                "    - api\n",
            ),
        );
        let report = fmt(&[tmp.path().to_path_buf()]).unwrap();
        assert_eq!(report.files_changed.len(), 1);
        let after = fs::read_to_string(&item).unwrap();
        assert!(after.contains("- api"), "items.rules lost:\n{after}");
        assert!(
            !after.starts_with("---"),
            "bundle file must not be wrapped in `---`:\n{after}"
        );
        assert!(after.starts_with("schema:"), "key order:\n{after}");
    }

    #[test]
    fn fmt_preserves_bundle_comments_end_to_end() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("bundles/commented.bundle.yaml");
        write(
            &item,
            concat!(
                "# upskill bundle manifest\n",
                "\n",
                "schema: 1\n",
                "name: commented\n",
                "description: has comments.\n",
                "items:\n",
                "  skills:\n",
                "    - foo # the foo skill\n",
                "# Metadata about this bundle\n",
                "metadata:\n",
                "  version: 0.1.0\n",
            ),
        );
        let report = fmt(&[tmp.path().to_path_buf()]).unwrap();
        // Already in canonical order — should not change.
        assert!(
            report.files_changed.is_empty(),
            "should not change: {report:?}"
        );
        let after = fs::read_to_string(&item).unwrap();
        assert!(
            after.contains("# upskill bundle manifest"),
            "preamble comment lost"
        );
        assert!(
            after.contains("- foo # the foo skill"),
            "inline comment lost"
        );
        assert!(
            after.contains("# Metadata about this bundle"),
            "block comment lost"
        );
    }
}
