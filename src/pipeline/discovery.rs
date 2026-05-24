//! SSOT layout introspection: detecting item directories, bundle files,
//! and registry roots on disk.
//!
//! The pipeline accepts several on-disk layouts (per format-spec §2.1):
//! a flat `<root>/<item>/ENTRY.md`, a categorised
//! `<root>/<category>/<item>/ENTRY.md`, or a bundle file pointing into
//! either. The helpers in this module classify paths without touching
//! the parser — they only check filesystem structure.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

use super::ItemKind;

/// Returns true when the directory contains at least one SSOT entrypoint
/// file (`RULE.md`, `SKILL.md`, or `AGENT.md`).
pub(super) fn is_item_dir(path: &Path) -> bool {
    path.join("RULE.md").is_file()
        || path.join("SKILL.md").is_file()
        || path.join("AGENT.md").is_file()
}

pub(super) fn is_bundle_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(crate::parse::bundle::BUNDLE_SUFFIX))
}

/// Search `root` recursively for a file named `<name>.bundle.yaml`.
/// Skips hidden directories. Returns the first match or `None`.
pub(super) fn find_bundle_by_name(root: &Path, name: &str) -> Option<PathBuf> {
    let target_filename = format!("{}{}", name, crate::parse::bundle::BUNDLE_SUFFIX);
    find_bundle_recursive(root, &target_filename)
}

fn find_bundle_recursive(dir: &Path, target: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let entry_name = entry.file_name();
        let name_str = entry_name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_file() && name_str == target {
            return Some(path);
        }
        if path.is_dir()
            && let Some(found) = find_bundle_recursive(&path, target)
        {
            return Some(found);
        }
    }
    None
}

/// Check whether a source directory contains any item (skill, rule, or
/// agent) with the given name. An item exists when a subdirectory named
/// `name` contains at least one of `SKILL.md`, `RULE.md`, or `AGENT.md`.
pub(super) fn has_matching_items(source: &Path, name: &str) -> bool {
    let item_dir = source.join(name);
    if !item_dir.is_dir() {
        return false;
    }
    item_dir.join("SKILL.md").is_file()
        || item_dir.join("RULE.md").is_file()
        || item_dir.join("AGENT.md").is_file()
}

/// Scan a source directory for item (kind, name) pairs without generating output.
pub(super) fn scan_source_items(source: &Path) -> Vec<(ItemKind, String)> {
    let mut items = Vec::new();
    let Ok(entries) = fs::read_dir(source) else {
        return items;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if is_item_dir(&path) {
            if path.join("SKILL.md").is_file() {
                items.push((ItemKind::Skill, name.to_string()));
            }
            if path.join("RULE.md").is_file() {
                items.push((ItemKind::Rule, name.to_string()));
            }
            if path.join("AGENT.md").is_file() {
                items.push((ItemKind::Agent, name.to_string()));
            }
        } else {
            // Category subdir: source/<category>/<item>/ENTRY.md
            if let Ok(sub_entries) = fs::read_dir(&path) {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    if !sub_path.is_dir() {
                        continue;
                    }
                    let Some(sub_name) = sub_path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if sub_name.starts_with('.') {
                        continue;
                    }
                    if sub_path.join("SKILL.md").is_file() {
                        items.push((ItemKind::Skill, sub_name.to_string()));
                    }
                    if sub_path.join("RULE.md").is_file() {
                        items.push((ItemKind::Rule, sub_name.to_string()));
                    }
                    if sub_path.join("AGENT.md").is_file() {
                        items.push((ItemKind::Agent, sub_name.to_string()));
                    }
                }
            }
        }
    }
    items
}

/// Walk up from `bundle_path`'s parent until a directory is found that
/// looks like an SSOT root — a directory whose direct children include
/// at least one item directory (containing `RULE.md`, `SKILL.md`, or
/// `AGENT.md`) or another bundle file. Falls back to the bundle's
/// parent directory if no such ancestor exists, so a flat layout
/// (bundle and items in the same dir) still works.
pub(super) fn find_registry_root(bundle_path: &Path) -> Result<PathBuf> {
    let parent = bundle_path
        .parent()
        .ok_or_else(|| anyhow!("bundle path {} has no parent", bundle_path.display()))?;
    let mut cursor = parent;
    loop {
        if has_ssot_layout(cursor) {
            return Ok(cursor.to_path_buf());
        }
        match cursor.parent() {
            Some(p) => cursor = p,
            None => break,
        }
    }
    Ok(parent.to_path_buf())
}

pub(super) fn has_ssot_layout(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Direct item check: dir/<item>/ENTRY.md
        if is_item_dir(&path) {
            return true;
        }
        // Grandchild check: dir/<category>/<item>/ENTRY.md
        // Handles sibling layouts where items live in subdirectories
        // (e.g. `skills/<item>/RULE.md` alongside `bundles/`).
        if let Ok(sub_entries) = fs::read_dir(&path) {
            for sub_entry in sub_entries.flatten() {
                let sub_path = sub_entry.path();
                if sub_path.is_dir() && is_item_dir(&sub_path) {
                    return true;
                }
            }
        }
    }
    false
}

/// Detect which entrypoint file (SKILL.md, RULE.md, AGENT.md) exists for
/// an item, for use in error messages.
pub(super) fn detect_item_entrypoint(source: &Path, name: &str) -> &'static str {
    let dir = source.join(name);
    if dir.join("SKILL.md").is_file() {
        "SKILL.md"
    } else if dir.join("RULE.md").is_file() {
        "RULE.md"
    } else if dir.join("AGENT.md").is_file() {
        "AGENT.md"
    } else {
        "SKILL.md" // fallback for error message
    }
}

/// Iterate `(name, dir)` for every immediate subdirectory of `kind_root`.
/// Returns an empty iterator when the kind root does not exist (treating
/// "no items of this kind" as a non-error).
pub(super) fn iter_item_dirs(kind_root: &Path) -> Result<Vec<(String, PathBuf)>> {
    if !kind_root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in
        fs::read_dir(kind_root).with_context(|| format!("read_dir {}", kind_root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if is_item_dir(&path) {
            // Direct item: kind_root/<item>/ENTRY.md
            let name = entry
                .file_name()
                .to_str()
                .map(str::to_owned)
                .with_context(|| format!("non-UTF8 name in {}", kind_root.display()))?;
            out.push((name, path));
        } else {
            // Category subdir: kind_root/<category>/<item>/ENTRY.md
            // Descend one level to find items in subdirectories (format-spec §2.2).
            if let Ok(sub_entries) = fs::read_dir(&path) {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    if sub_path.is_dir() && is_item_dir(&sub_path) {
                        let name = sub_entry
                            .file_name()
                            .to_str()
                            .map(str::to_owned)
                            .with_context(|| format!("non-UTF8 name in {}", path.display()))?;
                        out.push((name, sub_path));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_bundle_by_name_finds_nested_bundle_file() {
        let tmp = tempfile::tempdir().unwrap();
        let bundles_dir = tmp.path().join("bundles");
        std::fs::create_dir_all(&bundles_dir).unwrap();
        std::fs::write(
            bundles_dir.join("baseline.bundle.yaml"),
            "schema: 1\nname: baseline\ndescription: test\nitems:\n  rules: []\n",
        )
        .unwrap();

        let result = find_bundle_by_name(tmp.path(), "baseline");
        assert_eq!(result, Some(bundles_dir.join("baseline.bundle.yaml")));
    }

    #[test]
    fn find_bundle_by_name_returns_none_when_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("skills/foo")).unwrap();
        std::fs::write(
            tmp.path().join("skills/foo/SKILL.md"),
            "---\nschema: 1\nname: foo\n---\n# body\n",
        )
        .unwrap();

        let result = find_bundle_by_name(tmp.path(), "foo");
        assert!(result.is_none());
    }

    #[test]
    fn find_bundle_by_name_skips_hidden_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let hidden = tmp.path().join(".hidden");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(
            hidden.join("secret.bundle.yaml"),
            "schema: 1\nname: secret\ndescription: x\nitems:\n  rules: []\n",
        )
        .unwrap();

        let result = find_bundle_by_name(tmp.path(), "secret");
        assert!(result.is_none());
    }

    #[test]
    fn has_matching_items_true_when_skill_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("code-review");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nschema: 1\nname: code-review\n---\n# body\n",
        )
        .unwrap();

        assert!(has_matching_items(tmp.path(), "code-review"));
    }

    #[test]
    fn has_matching_items_false_when_no_item_exists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("other")).unwrap();

        assert!(!has_matching_items(tmp.path(), "nonexistent"));
    }

    #[test]
    fn has_matching_items_true_for_rules_and_agents() {
        let tmp = tempfile::tempdir().unwrap();
        let rule_dir = tmp.path().join("my-rule");
        std::fs::create_dir_all(&rule_dir).unwrap();
        std::fs::write(
            rule_dir.join("RULE.md"),
            "---\nschema: 1\nname: my-rule\n---\n# body\n",
        )
        .unwrap();

        assert!(has_matching_items(tmp.path(), "my-rule"));
    }

    #[test]
    fn has_ssot_layout_detects_direct_children() {
        // Flat layout: root/<item>/RULE.md
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("my-rule");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("RULE.md"), "").unwrap();

        assert!(has_ssot_layout(tmp.path()));
    }

    #[test]
    fn has_ssot_layout_detects_grandchild_entrypoints() {
        // Sibling layout: root/skills/<item>/RULE.md
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("skills/my-rule");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("RULE.md"), "").unwrap();

        assert!(
            has_ssot_layout(tmp.path()),
            "has_ssot_layout must detect items nested one level deeper"
        );
    }

    #[test]
    fn has_ssot_layout_returns_false_for_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!has_ssot_layout(tmp.path()));
    }

    #[test]
    fn find_registry_root_returns_parent_for_sibling_layout() {
        // registry/bundles/x.bundle.yaml + registry/skills/<item>/RULE.md
        // → find_registry_root must return registry/
        let tmp = tempfile::tempdir().unwrap();
        let registry = tmp.path().join("registry");
        std::fs::create_dir_all(registry.join("bundles")).unwrap();
        std::fs::create_dir_all(registry.join("skills/my-rule")).unwrap();
        std::fs::write(registry.join("skills/my-rule/RULE.md"), "").unwrap();
        let bundle = registry.join("bundles/test.bundle.yaml");
        std::fs::write(&bundle, "").unwrap();

        let root = find_registry_root(&bundle).unwrap();
        assert_eq!(root, registry);
    }

    #[test]
    fn iter_item_dirs_finds_items_in_category_subdirs() {
        // registry/skills/<item>/SKILL.md should be discovered
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("skills/my-skill");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("SKILL.md"), "").unwrap();

        let dirs = iter_item_dirs(tmp.path()).unwrap();
        let names: Vec<&str> = dirs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.contains(&"my-skill"),
            "iter_item_dirs must find items in category subdirectories: {names:?}"
        );
    }

    #[test]
    fn iter_item_dirs_still_finds_direct_children() {
        // Flat layout: root/<item>/RULE.md must still work
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("my-rule");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("RULE.md"), "").unwrap();

        let dirs = iter_item_dirs(tmp.path()).unwrap();
        let names: Vec<&str> = dirs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.contains(&"my-rule"),
            "iter_item_dirs must still find direct item children: {names:?}"
        );
    }
}
