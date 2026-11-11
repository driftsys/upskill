//! Ownership of installed files: content hashes, file state
//! classification, and the one-time adoption of a `schema: 1` lockfile.
//!
//! Pure helpers over the filesystem. This module writes nothing to stdout
//! or stderr.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::ALL_CLIENTS;
use super::output::{output_path, resource_base_path};
use crate::generate::Client;
use crate::lockfile::{LOCKFILE_NAME, LockedItem, Lockfile, OwnedFile, SCHEMA1_BACKUP_NAME};

/// Lowercase hex SHA-256 of `bytes`.
pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// State of one file at an item's output path, relative to the lockfile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileState {
    /// The path does not exist.
    Absent,
    /// The file content matches a recorded hash.
    Owned,
    /// The file exists, a hash is recorded, and the content differs.
    Modified,
    /// The file exists and no hash is recorded for it.
    Unowned,
}

/// `recorded` holds every hash that counts as upskill's own for this path:
/// the current record and, when present, the pending record.
pub(super) fn classify(target: &Path, rel: &str, recorded: &[&str]) -> Result<FileState> {
    let path = target.join(rel);
    if fs::symlink_metadata(&path).is_err() {
        return Ok(FileState::Absent);
    }
    if recorded.is_empty() {
        return Ok(FileState::Unowned);
    }
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let hash = sha256_hex(&bytes);
    if recorded.contains(&hash.as_str()) {
        Ok(FileState::Owned)
    } else {
        Ok(FileState::Modified)
    }
}

/// Portable form used in `OwnedFile.path`: the path components joined
/// with `/` on every platform.
pub(super) fn rel_to_record(rel: &Path) -> String {
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// One-time step for a schema-1 lockfile. Copies the lockfile bytes to
/// `SCHEMA1_BACKUP_NAME`, records for every item each existing file at the
/// item's layout paths with its current hash, sets `schema` to 2 and saves.
/// Returns `None` and does nothing when `lock.needs_adoption()` is false.
/// Otherwise returns the recorded paths, sorted, for the adoption notice.
pub(super) fn adopt_schema1(target: &Path, lock: &mut Lockfile) -> Result<Option<Vec<String>>> {
    if !lock.needs_adoption() {
        return Ok(None);
    }

    let lock_path = target.join(LOCKFILE_NAME);
    let backup_path = target.join(SCHEMA1_BACKUP_NAME);
    if lock_path.exists() && !backup_path.exists() {
        fs::copy(&lock_path, &backup_path).with_context(|| {
            format!(
                "copying {} to {}",
                lock_path.display(),
                backup_path.display()
            )
        })?;
    }

    let mut paths = Vec::new();
    for item in &mut lock.items {
        item.files = existing_item_files(target, item)?;
        paths.extend(item.files.iter().map(|f| f.path.clone()));
    }
    paths.sort();
    paths.dedup();

    lock.schema = 2;
    lock.save(target)?;
    Ok(Some(paths))
}

/// Clients an item was written for. An empty list means every client;
/// an unknown client name is skipped.
fn item_clients(item: &LockedItem) -> Vec<Client> {
    if item.clients.is_empty() {
        return ALL_CLIENTS.to_vec();
    }
    item.clients
        .iter()
        .filter_map(|name| name.parse::<Client>().ok())
        .collect()
}

/// Every existing regular file at the item's layout paths, hashed and
/// sorted by `(client, path)`.
fn existing_item_files(target: &Path, item: &LockedItem) -> Result<Vec<OwnedFile>> {
    let mut files = Vec::new();
    for client in item_clients(item) {
        let mut rels = vec![output_path(item.kind, client, &item.name)];
        walk_regular_files(
            target,
            &resource_base_path(item.kind, client, &item.name),
            &mut rels,
        )?;
        rels.sort();
        rels.dedup();
        for rel in rels {
            let abs = target.join(&rel);
            let is_file = fs::symlink_metadata(&abs)
                .map(|m| m.file_type().is_file())
                .unwrap_or(false);
            if !is_file {
                continue;
            }
            let bytes = fs::read(&abs).with_context(|| format!("reading {}", abs.display()))?;
            files.push(OwnedFile {
                client: client.name().to_string(),
                path: rel_to_record(&rel),
                sha256: sha256_hex(&bytes),
            });
        }
    }
    files.sort_by(|a, b| (&a.client, &a.path).cmp(&(&b.client, &b.path)));
    Ok(files)
}

/// Append to `out` every regular file below `target/rel`, as paths
/// relative to `target`. Symbolic links are skipped and not followed.
fn walk_regular_files(target: &Path, rel: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let dir = target.join(rel);
    let is_dir = fs::symlink_metadata(&dir)
        .map(|m| m.file_type().is_dir())
        .unwrap_or(false);
    if !is_dir {
        return Ok(());
    }
    let entries = fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", dir.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("reading {}", entry.path().display()))?;
        let child = rel.join(entry.file_name());
        if file_type.is_dir() {
            walk_regular_files(target, &child, out)?;
        } else if file_type.is_file() {
            out.push(child);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::ItemKind;

    fn write(target: &Path, rel: &Path, bytes: &[u8]) {
        let path = target.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn item(kind: ItemKind, name: &str, clients: &[&str]) -> LockedItem {
        LockedItem {
            kind,
            name: name.to_string(),
            source: "./src".to_string(),
            git_ref: None,
            hash: None,
            source_name: None,
            required_by: vec![],
            group: None,
            clients: clients.iter().map(|c| c.to_string()).collect(),
            files: vec![],
            pending: vec![],
        }
    }

    fn schema1_lock(target: &Path, items: Vec<LockedItem>) -> Lockfile {
        let mut lock = Lockfile::new();
        lock.schema = 1;
        lock.items = items;
        lock.save(target).unwrap();
        Lockfile::load(target).unwrap()
    }

    #[test]
    fn classify_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let state = classify(tmp.path(), "a/b.md", &["00"]).unwrap();
        assert_eq!(state, FileState::Absent);
    }

    #[test]
    fn classify_unowned_when_nothing_recorded() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), Path::new("a/b.md"), b"hello");
        let state = classify(tmp.path(), "a/b.md", &[]).unwrap();
        assert_eq!(state, FileState::Unowned);
    }

    #[test]
    fn classify_owned_on_equal_hash() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), Path::new("a/b.md"), b"hello");
        let hash = sha256_hex(b"hello");
        let state = classify(tmp.path(), "a/b.md", &[&hash]).unwrap();
        assert_eq!(state, FileState::Owned);
    }

    #[test]
    fn classify_owned_on_pending_hash() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), Path::new("a/b.md"), b"new");
        let current = sha256_hex(b"old");
        let pending = sha256_hex(b"new");
        let state = classify(tmp.path(), "a/b.md", &[&current, &pending]).unwrap();
        assert_eq!(state, FileState::Owned);
    }

    #[test]
    fn classify_modified_on_different_hash() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), Path::new("a/b.md"), b"edited");
        let hash = sha256_hex(b"hello");
        let state = classify(tmp.path(), "a/b.md", &[&hash]).unwrap();
        assert_eq!(state, FileState::Modified);
    }

    #[test]
    fn adoption_records_existing_files_and_keeps_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path();
        let skill_entry = output_path(ItemKind::Skill, Client::Claude, "demo");
        let skill_script =
            resource_base_path(ItemKind::Skill, Client::Claude, "demo").join("scripts/run.sh");
        let rule_entry = output_path(ItemKind::Rule, Client::Claude, "style");
        write(target, &skill_entry, b"skill body");
        write(target, &skill_script, b"echo run");
        write(target, &rule_entry, b"rule body");

        let mut lock = schema1_lock(
            target,
            vec![
                item(ItemKind::Skill, "demo", &["claude"]),
                item(ItemKind::Rule, "style", &["claude"]),
            ],
        );
        let original = fs::read(target.join(LOCKFILE_NAME)).unwrap();

        let paths = adopt_schema1(target, &mut lock).unwrap().unwrap();

        let reloaded = Lockfile::load(target).unwrap();
        assert_eq!(reloaded.schema, 2);
        let all: Vec<&OwnedFile> = reloaded.items.iter().flat_map(|i| &i.files).collect();
        assert_eq!(all.len(), 3);
        let expect = [
            (&skill_entry, &b"skill body"[..]),
            (&skill_script, &b"echo run"[..]),
            (&rule_entry, &b"rule body"[..]),
        ];
        for (rel, bytes) in expect {
            let rec = rel_to_record(rel);
            let found = all.iter().find(|f| f.path == rec).expect("recorded");
            assert_eq!(found.client, "claude");
            assert_eq!(found.sha256, sha256_hex(bytes));
        }
        assert!(reloaded.items.iter().all(|i| i.pending.is_empty()));
        let mut expected_paths: Vec<String> =
            expect.iter().map(|(r, _)| rel_to_record(r)).collect();
        expected_paths.sort();
        assert_eq!(paths, expected_paths);
        assert_eq!(
            fs::read(target.join(SCHEMA1_BACKUP_NAME)).unwrap(),
            original
        );
    }

    #[test]
    fn adoption_skips_missing_outputs() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = schema1_lock(tmp.path(), vec![item(ItemKind::Skill, "demo", &[])]);
        let paths = adopt_schema1(tmp.path(), &mut lock).unwrap();
        assert_eq!(paths, Some(vec![]));
        let reloaded = Lockfile::load(tmp.path()).unwrap();
        assert_eq!(reloaded.schema, 2);
        assert!(reloaded.items[0].files.is_empty());
    }

    #[test]
    fn adoption_is_a_no_op_on_schema_2() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = Lockfile::new();
        lock.items = vec![item(ItemKind::Skill, "demo", &["claude"])];
        write(
            tmp.path(),
            &output_path(ItemKind::Skill, Client::Claude, "demo"),
            b"body",
        );
        assert_eq!(adopt_schema1(tmp.path(), &mut lock).unwrap(), None);
        assert!(lock.items[0].files.is_empty());
        assert!(!tmp.path().join(LOCKFILE_NAME).exists());
        assert!(!tmp.path().join(SCHEMA1_BACKUP_NAME).exists());
    }

    #[test]
    fn adoption_keeps_an_existing_backup() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(SCHEMA1_BACKUP_NAME), b"older backup").unwrap();
        let mut lock = schema1_lock(tmp.path(), vec![]);
        adopt_schema1(tmp.path(), &mut lock).unwrap();
        assert_eq!(
            fs::read(tmp.path().join(SCHEMA1_BACKUP_NAME)).unwrap(),
            b"older backup"
        );
    }
}
