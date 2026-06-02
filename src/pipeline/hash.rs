//! Content hashing for SSOT item directories.
//!
//! The pipeline records a SHA-256 of every installed item's source
//! directory in the lockfile so `doctor` can detect drift and
//! `update --dry-run` can compute would-be hashes without touching the
//! filesystem outputs.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::ItemKind;
use super::discovery::{find_registry_root, is_bundle_file, iter_item_dirs};

/// SHA-256 hash of every file under `dir`, with each file's path-relative
/// name folded into the hash so renames register as drift. Recursive,
/// deterministic (sorted file list), and `None` when `dir` is empty or
/// unreadable. Used by the pipeline to populate `LockedItem.hash` and by
/// `doctor` (Phase B3) to detect SSOT drift.
pub(crate) fn hash_item_dir(dir: &Path) -> Option<String> {
    let mut files = Vec::new();
    collect_files(dir, &mut files);
    if files.is_empty() {
        return None;
    }
    files.sort();
    let mut hasher = Sha256::new();
    for file in &files {
        let relative = file.strip_prefix(dir).unwrap_or(file);
        hasher.update(relative.to_string_lossy().as_bytes());
        if let Ok(content) = fs::read(file) {
            hasher.update(&content);
        }
    }
    Some(
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        // `file_type()` does not follow symlinks; skip them so a directory
        // symlink cycle cannot drive unbounded recursion (consistent with
        // `discovery::collect_resource_files`).
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

/// Compute the `(kind, name) -> hash` map a source would install,
/// mirroring `install::install_from_local_path`'s dispatch (bundle-aware)
/// but without writing any output. Backs `update --dry-run` so its plan
/// matches what an `Apply` run — which reinstalls through the real
/// pipeline — produces.
///
/// `root` is the resolved SSOT root from `git::fetch_ssot`: a directory
/// for item-root and whole-repo sources, or a `*.bundle.yaml` file for
/// bundle sources. For a bundle file the items are resolved through the
/// same registry walk + dependency resolution the installer uses (issue
/// #196); for a directory every discovered item is hashed.
pub(super) fn planned_source_hashes(
    root: &Path,
) -> Result<BTreeMap<(ItemKind, String), Option<String>>> {
    if is_bundle_file(root) {
        let registry_root = find_registry_root(root).with_context(|| {
            format!(
                "find SSOT registry root containing skills/, rules/, agents/, or bundles/ \
                 above {}",
                root.display()
            )
        })?;
        let entry = crate::parse::bundle::load(root)
            .with_context(|| format!("load entry bundle {}", root.display()))?;
        let available: Vec<crate::model::Bundle> = crate::parse::bundle::discover(&registry_root)
            .with_context(|| {
                format!(
                    "discover sibling bundles under registry root {}",
                    registry_root.display()
                )
            })?
            .into_iter()
            .map(|(_, b)| b)
            .collect();
        let resolved = crate::bundle::resolve(&entry, &available)?;
        Ok(hash_items(&registry_root, Some(&resolved.items)))
    } else {
        Ok(hash_items(root, None))
    }
}

/// Hash every item directory under `source_root`, keyed by `(kind, name)`,
/// mirroring the discovery in `install::install_skills` / `install_rules`
/// / `install_agents`: items are found via [`iter_item_dirs`] (which
/// handles both the flat `<root>/<item>/` and the category
/// `<root>/<category>/<item>/` layouts) and, when `filter` is `Some`,
/// restricted to the resolved bundle set. A co-located multi-kind item
/// contributes one entry per kind for which it holds an entrypoint.
fn hash_items(
    source_root: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> BTreeMap<(ItemKind, String), Option<String>> {
    let mut out = BTreeMap::new();
    let Ok(dirs) = iter_item_dirs(source_root) else {
        return out;
    };
    for (name, dir) in dirs {
        let hash = hash_item_dir(&dir);
        for (entrypoint, kind) in [
            ("RULE.md", ItemKind::Rule),
            ("SKILL.md", ItemKind::Skill),
            ("AGENT.md", ItemKind::Agent),
        ] {
            if dir.join(entrypoint).is_file() && filter.is_none_or(|f| f.contains(kind, &name)) {
                out.insert((kind, name.clone()), hash.clone());
            }
        }
    }
    out
}
