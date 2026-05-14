//! Bundle file parsing and discovery (§2.2 + §3.7).
//!
//! Bundle files are flat manifests named `<name>.bundle.md`, with a YAML
//! frontmatter block (the [`Bundle`] schema) and a free-form markdown
//! body. They MAY live anywhere within a source registry — `load` takes
//! a path; `discover` walks a directory tree for the `.bundle.md`
//! suffix.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::Bundle;
use crate::parse::frontmatter;

/// Filename suffix every bundle manifest carries (§2.2).
pub const BUNDLE_SUFFIX: &str = ".bundle.md";

/// Read and parse a single bundle file.
///
/// Validates that the filename stem (before `.bundle.md`) matches the
/// `name` field in the frontmatter, per §2.2.
pub fn load(path: &Path) -> Result<Bundle> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let (bundle, _body) = frontmatter::parse::<Bundle>(&raw)
        .with_context(|| format!("parse bundle {}", path.display()))?;

    let stem = filename_stem(path).ok_or_else(|| {
        anyhow::anyhow!(
            "{}: filename must end in `{}`",
            path.display(),
            BUNDLE_SUFFIX
        )
    })?;

    if stem != bundle.name {
        anyhow::bail!(
            "{}: filename stem `{}` does not match bundle.name `{}`",
            path.display(),
            stem,
            bundle.name
        );
    }

    Ok(bundle)
}

/// Recursively walk `source_root` for `*.bundle.md` files. Returns one
/// `(absolute path, parsed Bundle)` per file, sorted by bundle name (the
/// identifier users actually type on the CLI). Errors on the first parse
/// or filename-mismatch failure.
///
/// Matches the discovery contract in §2.2: bundles MAY live anywhere in
/// a source registry; the implementation MUST NOT depend on a specific
/// bundle-root path.
pub fn discover(source_root: &Path) -> Result<Vec<(PathBuf, Bundle)>> {
    let mut out: Vec<(PathBuf, Bundle)> = Vec::new();
    if !source_root.exists() {
        return Ok(out);
    }
    let mut paths = Vec::new();
    walk(source_root, &mut paths)?;
    for path in paths {
        let bundle = load(&path)?;
        out.push((path, bundle));
    }
    out.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            // Skip `.git` and other dot-directories — they never contain
            // user-authored bundles and would slow large-tree walks.
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'))
            {
                continue;
            }
            walk(&path, out)?;
        } else if file_type.is_file() && filename_stem(&path).is_some() {
            out.push(path);
        }
    }
    Ok(())
}

/// Returns `Some("<name>")` for `<name>.bundle.md`, `None` otherwise.
fn filename_stem(path: &Path) -> Option<&str> {
    let name = path.file_name().and_then(|n| n.to_str())?;
    name.strip_suffix(BUNDLE_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_file(dir: &Path, rel: &str, content: &str) -> PathBuf {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    const PLATFORM: &str = "---
schema: 1
name: platform-baseline
description: Baseline rules, skills, and agents for all repositories
license: proprietary

items:
  rules:
    - api-conventions
    - license-awareness
  skills:
    - create-api-endpoint
  agents:
    - security-reviewer

metadata:
  version: \"1.0.0\"
  author: platform-dx
---

# Platform baseline
";

    #[test]
    fn load_round_trips_format_spec_example() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "platform-baseline.bundle.md", PLATFORM);

        let bundle = load(&path).expect("load");

        assert_eq!(bundle.name, "platform-baseline");
        assert_eq!(bundle.schema.get(), 1);
        assert_eq!(bundle.items.rules.len(), 2);
        assert_eq!(bundle.items.skills, vec!["create-api-endpoint"]);
        assert_eq!(bundle.items.agents, vec!["security-reviewer"]);
        assert!(bundle.requires.is_empty(), "no requires specified");
        assert_eq!(
            bundle.metadata.as_ref().and_then(|m| m.version.as_deref()),
            Some("1.0.0")
        );
    }

    #[test]
    fn load_parses_requires_with_and_without_version() {
        let content = "---
schema: 1
name: rust-baseline
description: Rust-specific baseline on top of platform
items:
  rules: []
requires:
  - { name: platform-baseline, version: \"^1.0.0\" }
  - { name: shared-conventions }
---
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "rust-baseline.bundle.md", content);

        let bundle = load(&path).expect("load");
        assert_eq!(bundle.requires.len(), 2);
        assert_eq!(bundle.requires[0].name, "platform-baseline");
        assert_eq!(bundle.requires[0].version.as_deref(), Some("^1.0.0"));
        assert_eq!(bundle.requires[1].name, "shared-conventions");
        assert_eq!(bundle.requires[1].version, None);
    }

    #[test]
    fn load_rejects_string_form_requires() {
        // Per §3.7 (post-PR #76): map-only `requires`. Bare strings must
        // fail to deserialize so a future polymorphic shape stays a
        // deliberate spec change, not a silent acceptance.
        let content = "---
schema: 1
name: bare
description: Should fail
items:
  rules: []
requires:
  - just-a-string
---
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "bare.bundle.md", content);

        let err = load(&path).expect_err("string-form requires must be rejected");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("requires") || msg.contains("Requires") || msg.contains("expected"),
            "expected serde error, got: {msg}"
        );
    }

    #[test]
    fn load_rejects_filename_stem_mismatch() {
        let content = "---
schema: 1
name: platform-baseline
description: filename mismatch
items:
  rules: []
---
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "wrong-name.bundle.md", content);

        let err = load(&path).expect_err("must reject filename mismatch");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("filename stem") && msg.contains("platform-baseline"),
            "got: {msg}"
        );
    }

    #[test]
    fn load_accepts_empty_items_when_only_requires() {
        // A meta-bundle that just composes other bundles is valid.
        let content = "---
schema: 1
name: meta
description: Composes other bundles
items: {}
requires:
  - { name: platform-baseline }
---
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "meta.bundle.md", content);
        let bundle = load(&path).expect("load");
        assert!(bundle.items.is_empty());
        assert_eq!(bundle.requires.len(), 1);
    }

    #[test]
    fn discover_finds_bundles_recursively_anywhere_in_tree() {
        // §2.2: bundles MAY live anywhere — at root, alongside item dirs,
        // in dedicated `bundles/`. Discovery must find all three.
        let tmp = tempfile::tempdir().unwrap();

        write_file(
            tmp.path(),
            "root-only.bundle.md",
            &renamed(PLATFORM, "root-only"),
        );
        write_file(
            tmp.path(),
            "bundles/in-bundles-dir.bundle.md",
            &renamed(PLATFORM, "in-bundles-dir"),
        );
        write_file(
            tmp.path(),
            "nested/alongside.bundle.md",
            &renamed(PLATFORM, "alongside"),
        );
        // Dot-directory contents should be skipped.
        write_file(
            tmp.path(),
            ".git/skipped.bundle.md",
            "---\nschema: 1\nname: skipped\ndescription: nope\nitems: {}\n---\n",
        );
        // Non-bundle file should be ignored.
        write_file(tmp.path(), "x/SKILL.md", "---\nname: x\n---\nbody");

        let found = discover(tmp.path()).expect("discover");
        let names: Vec<&str> = found.iter().map(|(_, b)| b.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["alongside", "in-bundles-dir", "root-only"],
            "deterministic sort by bundle name"
        );
    }

    #[test]
    fn discover_empty_when_root_missing_or_no_bundles() {
        let tmp = tempfile::tempdir().unwrap();
        let nonexistent = tmp.path().join("does-not-exist");
        assert!(discover(&nonexistent).unwrap().is_empty());

        let empty = tmp.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        assert!(discover(&empty).unwrap().is_empty());
    }

    /// Helper: produce a copy of a bundle YAML with the `name` field
    /// rewritten so multiple fixtures can share the same body shape but
    /// keep filename-stem agreement.
    fn renamed(template: &str, new_name: &str) -> String {
        template.replace("name: platform-baseline", &format!("name: {new_name}"))
    }
}
