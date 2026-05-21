//! `upskill fmt` — canonicalise YAML frontmatter in SSOT files.
//!
//! Per [ADR-0004](../../docs/adr/0004-cli-surface.md), `fmt` and `lint`
//! are sibling author commands. `fmt` operates on YAML frontmatter
//! only; markdown body content is dprint's job and is preserved
//! byte-for-byte. Like `lint`, this command refuses to run inside a
//! consumer project (`.upskill-lock.json` at the path's root).
//!
//! What gets canonicalised:
//!
//! - Key order — fixed by the [`crate::model`] struct field order
//!   (`schema → name → description → audience → license → metadata →
//!   kind-specific → passthroughs → extras`).
//! - Indentation — `serde_yaml_ng`'s default emit (two-space).
//! - Unknown top-level keys (the `extra` flatten map) come out
//!   alphabetically — predictable, even if not the author's order.
//!
//! Implementation: parse the frontmatter into the typed model, then
//! serialise it back with `serde_yaml_ng::to_string`. The body slice
//! is reattached unchanged.

use anyhow::{Context, Result, anyhow};
use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::path::{Path, PathBuf};

use crate::lint::{discover, is_consumer_project};
use crate::model::{Agent, Bundle, Rule, Skill};
use crate::parse::frontmatter;

/// Outcome of one `upskill fmt` run.
#[derive(Debug, Default, Clone)]
pub struct FmtReport {
    /// Files whose on-disk content differed from the canonical form
    /// and were rewritten.
    pub files_changed: Vec<PathBuf>,
    /// Total entrypoint files inspected.
    pub files_checked: usize,
}

/// Canonicalise YAML frontmatter in every SSOT entrypoint discovered
/// under `paths`. With an empty `paths` slice, defaults to the current
/// working directory.
///
/// Files whose frontmatter was already canonical are left untouched
/// (no `mtime` thrash). Body content is preserved byte-for-byte.
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

    if let EntryKind::Bundle = kind {
        let bundle: Bundle = serde_yaml_ng::from_str(raw)
            .with_context(|| format!("parse bundle {}", path.display()))?;
        return serde_yaml_ng::to_string(&bundle).context("serialise canonical bundle");
    }

    let body = frontmatter::split(raw)
        .map(|(_, body)| body)
        .ok_or_else(|| anyhow!("{}: missing YAML frontmatter", path.display()))?;

    let yaml = match kind {
        EntryKind::Skill => roundtrip::<Skill>(raw)?,
        EntryKind::Rule => roundtrip::<Rule>(raw)?,
        EntryKind::Agent => roundtrip::<Agent>(raw)?,
        EntryKind::Bundle => unreachable!("bundle handled above"),
    };

    Ok(format!("---\n{yaml}---\n{body}"))
}

/// Parse `raw`'s frontmatter into `T`, then serialise `T` back to
/// YAML. `serde_yaml_ng::to_string` already terminates the output with
/// a trailing newline — no extra padding needed.
fn roundtrip<T: DeserializeOwned + Serialize>(raw: &str) -> Result<String> {
    let (value, _body) =
        frontmatter::parse::<T>(raw).with_context(|| "parsing frontmatter for fmt")?;
    serde_yaml_ng::to_string(&value).context("serialise canonical frontmatter")
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
    fn canonicalise_preserves_body_byte_for_byte() {
        let body = concat!(
            "\n",
            "## A heading\n",
            "\n",
            "```rust\n",
            "fn x() {}\n",
            "```\n",
            "\n",
            "<!-- a comment -->\n",
        );
        let raw =
            format!("---\nschema: 1\nname: preserve\ndescription: do not touch body.\n---\n{body}");
        let out = canonicalise(&raw, Path::new("preserve/SKILL.md")).unwrap();
        assert!(out.ends_with(body), "body changed:\n{out}");
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
    fn fmt_skips_already_canonical_files() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("clean/SKILL.md");
        let canonical = concat!(
            "---\n",
            "schema: 1\n",
            "name: clean\n",
            "description: already canonical.\n",
            "---\n",
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
    fn fmt_handles_bundle() {
        // Bundles are pure YAML (§2.2, ADR-0007). fmt reorders keys and
        // strips the file down to canonical YAML — no `---` wrapping.
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("bundles/baseline.bundle.yaml");
        write(
            &item,
            concat!(
                "name: baseline\n",
                "schema: 1\n",
                "description: a bundle.\n",
                "items:\n",
                "  rules: [api]\n",
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
}
