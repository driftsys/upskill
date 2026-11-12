//! Content hashing for SSOT item directories.
//!
//! The pipeline records a SHA-256 of every installed item's source
//! directory in the lockfile so that `doctor` can detect drift and
//! `update` can tell whether the source of an item changed.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use super::discovery::walk_files;

/// Best-effort legacy SHA-256 over sorted discovered paths and readable content.
/// Recursive discovery skips symlinks and traversal errors and can include
/// non-directory special entries. A read failure retains that entry's path in the
/// digest but omits its bytes. Returns `None` when discovery yields no entries.
/// Used for `LockedItem.hash` and `doctor` drift detection.
pub(crate) fn hash_item_dir(dir: &Path) -> Option<String> {
    let mut files = walk_files(dir);
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
