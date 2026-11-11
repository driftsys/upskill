//! Ownership of installed files: content hashes, file state
//! classification, and the one-time adoption of a `schema: 1` lockfile.
//!
//! Pure helpers over the filesystem. This module writes nothing to stdout
//! or stderr.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::ErrorKind;
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

/// State of one path at an item's output location, relative to the
/// lockfile. Only a regular file can be `Owned`; a directory, a symbolic
/// link of any kind, or another file type is never read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileState {
    /// The path does not exist.
    Absent,
    /// The path is a regular file whose content matches a recorded hash.
    Owned,
    /// A hash is recorded, and the path is a regular file with other
    /// content, or is not a regular file.
    Modified,
    /// The path exists and no hash is recorded for it.
    Unowned,
}

/// Classify `target/rel`. `recorded` holds every hash that counts as
/// upskill's own for this path: the current record and, when present, the
/// pending record.
///
/// The path is examined without following symbolic links. It is `Absent`
/// only when the lookup fails with "not found"; any other lookup error is
/// returned. A present path is `Unowned` when `recorded` is empty. A path
/// that is not a regular file is `Modified` when `recorded` is not empty.
/// A regular file is `Owned` when its hash is in `recorded`, and
/// `Modified` otherwise.
pub(super) fn classify(target: &Path, rel: &str, recorded: &[&str]) -> Result<FileState> {
    let path = target.join(rel);
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(FileState::Absent),
        Err(err) => {
            return Err(err).with_context(|| format!("inspecting {}", path.display()));
        }
    };
    if recorded.is_empty() {
        return Ok(FileState::Unowned);
    }
    if !meta.file_type().is_file() {
        return Ok(FileState::Modified);
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
/// with `/` on every platform. Fails on a component that is not a plain
/// name (`..`, `.`, a root or a prefix) and on a name that is not UTF-8.
pub(super) fn rel_to_record(rel: &Path) -> Result<String> {
    let mut parts = Vec::new();
    for component in rel.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .with_context(|| format!("path is not UTF-8: {}", rel.display()))?,
            ),
            _ => anyhow::bail!("path is not a plain relative path: {}", rel.display()),
        }
    }
    Ok(parts.join("/"))
}

/// One-time step for a schema-1 lockfile. Records for every item each
/// existing file at the item's layout paths with its current hash, then
/// copies the lockfile bytes to `SCHEMA1_BACKUP_NAME`, sets `schema` to 2
/// and saves.
/// Returns `None` and does nothing when `lock.needs_adoption()` is false.
/// Otherwise returns the recorded paths, sorted, for the adoption notice.
///
/// An existing backup file is kept. An item whose name is empty or is not
/// a plain relative path records no file. `lock` is changed only after
/// every item's files were computed.
pub(super) fn adopt_schema1(target: &Path, lock: &mut Lockfile) -> Result<Option<Vec<String>>> {
    if !lock.needs_adoption() {
        return Ok(None);
    }

    let mut all_files = Vec::with_capacity(lock.items.len());
    for item in &lock.items {
        all_files.push(existing_item_files(target, item)?);
    }

    // The backup is written only after every item was scanned, so a failed
    // scan leaves the disk unchanged.
    let lock_path = target.join(LOCKFILE_NAME);
    let backup_path = target.join(SCHEMA1_BACKUP_NAME);
    if lock_path.exists() && !backup_path.exists() {
        let tmp_path = target.join(format!("{SCHEMA1_BACKUP_NAME}.tmp"));
        fs::copy(&lock_path, &tmp_path).with_context(|| {
            format!("copying {} to {}", lock_path.display(), tmp_path.display())
        })?;
        fs::rename(&tmp_path, &backup_path).with_context(|| {
            format!(
                "renaming {} to {}",
                tmp_path.display(),
                backup_path.display()
            )
        })?;
    }

    let mut paths = Vec::new();
    for (item, files) in lock.items.iter_mut().zip(all_files) {
        paths.extend(files.iter().map(|f| f.path.clone()));
        item.files = files;
    }
    paths.sort();
    paths.dedup();

    lock.schema = 2;
    lock.save(target)?;
    Ok(Some(paths))
}

/// True when `name` is not empty and every component is a plain name.
/// An inner `/` (for example `a/b`) is allowed.
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
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
/// sorted by `(client, path)`. Empty for an item with an unsafe name.
fn existing_item_files(target: &Path, item: &LockedItem) -> Result<Vec<OwnedFile>> {
    let mut files = Vec::new();
    if !is_safe_name(&item.name) {
        return Ok(files);
    }
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
            let is_file = match fs::symlink_metadata(&abs) {
                Ok(meta) => meta.file_type().is_file(),
                Err(err) if err.kind() == ErrorKind::NotFound => false,
                Err(err) => {
                    return Err(err).with_context(|| format!("inspecting {}", abs.display()));
                }
            };
            if !is_file {
                continue;
            }
            let bytes = fs::read(&abs).with_context(|| format!("reading {}", abs.display()))?;
            files.push(OwnedFile {
                client: client.name().to_string(),
                path: rel_to_record(&rel)?,
                sha256: sha256_hex(&bytes),
            });
        }
    }
    files.sort_by(|a, b| (&a.client, &a.path).cmp(&(&b.client, &b.path)));
    Ok(files)
}

/// Append to `out` every regular file below `target/rel`, as paths
/// relative to `target`. Symbolic links are skipped and not followed. A
/// file or directory whose name is not UTF-8 is skipped, because the
/// lockfile cannot record its path exactly.
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
        if entry.file_name().to_str().is_none() {
            continue;
        }
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
            let rec = rel_to_record(rel).unwrap();
            let found = all.iter().find(|f| f.path == rec).expect("recorded");
            assert_eq!(found.client, "claude");
            assert_eq!(found.sha256, sha256_hex(bytes));
        }
        assert!(reloaded.items.iter().all(|i| i.pending.is_empty()));
        let mut expected_paths: Vec<String> = expect
            .iter()
            .map(|(r, _)| rel_to_record(r).unwrap())
            .collect();
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

    #[cfg(unix)]
    #[test]
    fn classify_symlink_to_recorded_content_is_modified() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), Path::new("outside.md"), b"hello");
        fs::create_dir_all(tmp.path().join("a")).unwrap();
        std::os::unix::fs::symlink(tmp.path().join("outside.md"), tmp.path().join("a/b.md"))
            .unwrap();
        let hash = sha256_hex(b"hello");
        let state = classify(tmp.path(), "a/b.md", &[&hash]).unwrap();
        assert_eq!(state, FileState::Modified);
    }

    #[cfg(unix)]
    #[test]
    fn classify_dangling_symlink_is_modified_when_recorded() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("a")).unwrap();
        std::os::unix::fs::symlink(tmp.path().join("missing"), tmp.path().join("a/b.md")).unwrap();
        let state = classify(tmp.path(), "a/b.md", &["00"]).unwrap();
        assert_eq!(state, FileState::Modified);
    }

    #[cfg(unix)]
    #[test]
    fn classify_symlink_is_unowned_when_nothing_recorded() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("a")).unwrap();
        std::os::unix::fs::symlink(tmp.path().join("missing"), tmp.path().join("a/b.md")).unwrap();
        let state = classify(tmp.path(), "a/b.md", &[]).unwrap();
        assert_eq!(state, FileState::Unowned);
    }

    #[test]
    fn classify_directory_at_path_is_modified_when_recorded() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("a/b.md")).unwrap();
        let state = classify(tmp.path(), "a/b.md", &["00"]).unwrap();
        assert_eq!(state, FileState::Modified);
    }

    #[test]
    fn classify_directory_at_path_is_unowned_when_nothing_recorded() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("a/b.md")).unwrap();
        let state = classify(tmp.path(), "a/b.md", &[]).unwrap();
        assert_eq!(state, FileState::Unowned);
    }

    #[cfg(unix)]
    #[test]
    fn classify_propagates_an_error_that_is_not_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), Path::new("a"), b"file, not a directory");
        assert!(classify(tmp.path(), "a/b.md", &["00"]).is_err());
    }

    #[test]
    fn rel_to_record_rejects_parent_components() {
        assert!(rel_to_record(Path::new("../x.md")).is_err());
        assert!(rel_to_record(Path::new("a/../x.md")).is_err());
    }

    #[test]
    fn rel_to_record_uses_forward_slashes() {
        let rel: PathBuf = ["a", "b", "c.md"].iter().collect();
        assert_eq!(rel_to_record(&rel).unwrap(), "a/b/c.md");
    }

    #[test]
    fn adoption_skips_an_item_with_a_parent_dir_name() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), Path::new(".claude/skills/SKILL.md"), b"x");
        write(tmp.path(), Path::new(".claude/rules/other.md"), b"x");
        let mut lock = schema1_lock(tmp.path(), vec![item(ItemKind::Skill, "..", &["claude"])]);
        let paths = adopt_schema1(tmp.path(), &mut lock).unwrap();
        assert_eq!(paths, Some(vec![]));
        assert!(lock.items[0].files.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn adoption_leaves_the_lock_unchanged_on_error() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let first = output_path(ItemKind::Rule, Client::Claude, "a");
        let second = output_path(ItemKind::Rule, Client::Claude, "b");
        write(tmp.path(), &first, b"a");
        write(tmp.path(), &second, b"b");
        let blocked = tmp.path().join(&second);
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&blocked).is_ok() {
            // Running as root: permissions do not block the read.
            return;
        }
        let mut lock = schema1_lock(
            tmp.path(),
            vec![
                item(ItemKind::Rule, "a", &["claude"]),
                item(ItemKind::Rule, "b", &["claude"]),
            ],
        );
        let before = lock.clone();
        assert!(adopt_schema1(tmp.path(), &mut lock).is_err());
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(lock, before);
        assert!(!tmp.path().join(SCHEMA1_BACKUP_NAME).exists());
    }

    #[cfg(unix)]
    #[test]
    fn adoption_skips_symlink_entries_in_the_walk() {
        let tmp = tempfile::tempdir().unwrap();
        let entry = output_path(ItemKind::Skill, Client::Claude, "demo");
        write(tmp.path(), &entry, b"body");
        write(tmp.path(), Path::new("outside.txt"), b"outside");
        let base = resource_base_path(ItemKind::Skill, Client::Claude, "demo");
        std::os::unix::fs::symlink(
            tmp.path().join("outside.txt"),
            tmp.path().join(&base).join("link.txt"),
        )
        .unwrap();
        std::os::unix::fs::symlink(tmp.path(), tmp.path().join(&base).join("loop")).unwrap();
        let mut lock = schema1_lock(tmp.path(), vec![item(ItemKind::Skill, "demo", &["claude"])]);
        let paths = adopt_schema1(tmp.path(), &mut lock).unwrap().unwrap();
        assert_eq!(paths, vec![rel_to_record(&entry).unwrap()]);
    }

    #[test]
    fn adoption_with_empty_clients_records_all_three_clients() {
        let tmp = tempfile::tempdir().unwrap();
        for client in ALL_CLIENTS {
            write(
                tmp.path(),
                &output_path(ItemKind::Skill, client, "demo"),
                b"body",
            );
        }
        let mut lock = schema1_lock(tmp.path(), vec![item(ItemKind::Skill, "demo", &[])]);
        adopt_schema1(tmp.path(), &mut lock).unwrap();
        let clients: Vec<&str> = lock.items[0]
            .files
            .iter()
            .map(|f| f.client.as_str())
            .collect();
        assert_eq!(clients, vec!["claude", "copilot", "opencode"]);
    }

    #[test]
    fn adoption_skips_an_unknown_client_name() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            &output_path(ItemKind::Skill, Client::Claude, "demo"),
            b"body",
        );
        let mut lock = schema1_lock(
            tmp.path(),
            vec![item(ItemKind::Skill, "demo", &["unknown", "claude"])],
        );
        adopt_schema1(tmp.path(), &mut lock).unwrap();
        let files = &lock.items[0].files;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].client, "claude");
    }

    #[test]
    fn adoption_sorts_files_by_client_then_path() {
        let tmp = tempfile::tempdir().unwrap();
        for client in [Client::Claude, Client::Copilot] {
            write(
                tmp.path(),
                &output_path(ItemKind::Skill, client, "demo"),
                b"body",
            );
            let base = resource_base_path(ItemKind::Skill, client, "demo");
            write(tmp.path(), &base.join("a.txt"), b"a");
        }
        let mut lock = schema1_lock(
            tmp.path(),
            vec![item(ItemKind::Skill, "demo", &["copilot", "claude"])],
        );
        adopt_schema1(tmp.path(), &mut lock).unwrap();
        let got: Vec<(String, String)> = lock.items[0]
            .files
            .iter()
            .map(|f| (f.client.clone(), f.path.clone()))
            .collect();
        let mut sorted = got.clone();
        sorted.sort();
        assert_eq!(got, sorted);
        assert_eq!(got.len(), 4);
        assert_eq!(got[0].0, "claude");
        assert_eq!(got[3].0, "copilot");
    }

    #[test]
    fn adoption_writes_no_backup_when_the_lockfile_is_missing_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = Lockfile::new();
        lock.schema = 1;
        adopt_schema1(tmp.path(), &mut lock).unwrap();
        assert!(!tmp.path().join(SCHEMA1_BACKUP_NAME).exists());
        assert!(
            !tmp.path()
                .join(format!("{SCHEMA1_BACKUP_NAME}.tmp"))
                .exists()
        );
    }

    #[test]
    fn adoption_records_a_flat_rule_resource() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), Path::new(".claude/rules/x.md"), b"x");
        write(
            tmp.path(),
            Path::new(".claude/rules/x/scripts/run.sh"),
            b"run",
        );
        write(tmp.path(), Path::new(".claude/rules/y.md"), b"y");
        let mut lock = schema1_lock(tmp.path(), vec![item(ItemKind::Rule, "x", &["claude"])]);
        let paths = adopt_schema1(tmp.path(), &mut lock).unwrap().unwrap();
        assert_eq!(
            paths,
            vec![
                ".claude/rules/x.md".to_string(),
                ".claude/rules/x/scripts/run.sh".to_string()
            ]
        );
    }
}
