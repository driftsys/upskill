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

use super::ItemKind;

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
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

/// Hash every item directory under a SSOT root, keyed by `(kind, name)`.
/// Used by `update --dry-run` to compute would-be hashes without
/// installing. Mirrors the discovery of `install_from_local_path`: an
/// item directory `<source_root>/<name>/` contributes one entry per
/// kind for which it holds an entrypoint (so co-located multi-kind
/// items contribute multiple entries, one per kind).
pub(super) fn hash_source_items(
    source_root: &Path,
) -> BTreeMap<(ItemKind, String), Option<String>> {
    let mut out = BTreeMap::new();
    let Ok(entries) = fs::read_dir(source_root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let hash = hash_item_dir(&path);
        for (entrypoint, kind) in [
            ("RULE.md", ItemKind::Rule),
            ("SKILL.md", ItemKind::Skill),
            ("AGENT.md", ItemKind::Agent),
        ] {
            if path.join(entrypoint).is_file() {
                out.insert((kind, name.to_string()), hash.clone());
            }
        }
    }
    out
}
