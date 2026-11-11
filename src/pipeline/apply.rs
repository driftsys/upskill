//! Writes an install plan to disk and computes the ownership record of
//! each planned item.
//!
//! Symbolic link rule: the first two components of a layout path (the
//! client root and the kind directory, for example `.claude/skills`) may be
//! symbolic links. Every existing component from the third one to the last
//! one (the item directory, resource subdirectories and the file) must not
//! be a symbolic link. The checks that `apply_plan` runs before any change
//! on disk, and the conflict rules for modified and unowned files, are
//! described in the rustdoc of `apply_plan` and `check_writable`.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use super::ItemKind;
use super::owned::{FileState, classify, rel_to_record, sha256_hex};
use super::plan::PlannedItem;
use crate::lockfile::{Lockfile, OwnedFile};

/// The record of written files for each planned `(kind, installed name)`,
/// sorted by `client`, then `path`. `install_with_lockfile` copies it into
/// the `LockedItem` it upserts for each installed item.
pub(super) type RecordedFiles = BTreeMap<(ItemKind, String), Vec<OwnedFile>>;

/// Client roots under which a recorded path may be deleted.
const CLIENT_ROOTS: [&str; 4] = [".claude", ".github", ".agents", ".opencode"];

/// State of a destination that blocks the write of an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictState {
    /// upskill recorded the path, and its content changed since.
    Modified,
    /// The path exists and upskill has no record of it.
    Unowned,
}

/// A path that stops `add` or `update` from writing an item, because the
/// write would replace a file that upskill did not write or that was
/// changed after upskill wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileConflict {
    pub kind: ItemKind,
    /// Installed name of the item.
    pub name: String,
    /// Path relative to the install target root.
    pub path: PathBuf,
    pub state: ConflictState,
}

/// What `apply_plan` did.
#[derive(Debug, Default)]
pub(super) struct ApplyOutcome {
    /// Items that were not written because of a conflict.
    pub skipped: Vec<FileConflict>,
    /// Modified files of a previous record that the plan no longer
    /// contains. They are kept on disk and stay in the item's record.
    pub kept: Vec<PathBuf>,
    /// The record of every written item.
    pub recorded: RecordedFiles,
}

/// What `apply_plan` does for one item, computed before any change.
struct ItemChanges {
    files: Vec<OwnedFile>,
    /// Stale files to delete.
    delete: Vec<PathBuf>,
    /// Stale files that are kept, with their previous records.
    kept: Vec<(PathBuf, Vec<OwnedFile>)>,
    /// Destinations that stop the write of this item.
    conflicts: Vec<FileConflict>,
}

/// Writes every planned item that has no conflict and updates
/// `lock.items[..].files` for the written items that already have a
/// lockfile entry. It does not save the lockfile.
///
/// Conflicts: every destination of every item is classified with
/// [`classify`] before anything is written. A destination whose content
/// already equals the planned bytes is not a conflict, in any state. A
/// destination that is `Modified` or `Unowned` is a conflict. When
/// `overwrite` is false, an item with at least one conflict is skipped as a
/// whole: none of its files is written or deleted, and its lockfile record
/// is unchanged. The other items are written. When `overwrite` is true, a
/// conflicting regular file is replaced.
///
/// Stale files: the files of an item's previous record that no planned item
/// of this run writes. Before writing an item, its stale files that are
/// unchanged (`Owned`) or absent are deleted, and directories left empty
/// are removed. A stale file that is `Modified`, or that cannot be read, is
/// kept and stays in the record; with `overwrite` it is deleted when it is
/// a regular file. A kept stale file that is a planned path, or a directory
/// on the way to one, is a conflict of the item.
///
/// Before any change on disk it fails when two planned items have the same
/// `(kind, name)` or an installed name contains a path separator, when two
/// planned files have the same path, when one planned file is a directory
/// on the way to another, when a path to write or delete breaks the
/// symbolic link rule of this module, or when a path to write is blocked:
/// the client root or kind directory is a symbolic link to a missing target
/// or is not a directory, a component on the way to the file is not a
/// directory, or the file exists and is not a regular file. A blocking
/// regular file, or a directory that holds only regular files, is accepted
/// when those files are stale recorded files of the same item. See
/// `check_writable`.
pub(super) fn apply_plan(
    target: &Path,
    lock: &mut Lockfile,
    planned: &[PlannedItem],
    overwrite: bool,
) -> Result<ApplyOutcome> {
    check_installed_names(planned)?;
    let mut paths_seen: BTreeMap<&Path, &str> = BTreeMap::new();
    let mut changes = Vec::with_capacity(planned.len());
    // No item deletes, as stale, a path that any item of this run writes.
    let all_planned: BTreeSet<&Path> = planned
        .iter()
        .flat_map(|item| item.files.iter().map(|file| file.rel.as_path()))
        .collect();
    for item in planned {
        let mut files = Vec::with_capacity(item.files.len());
        for file in &item.files {
            if let Some(other) = paths_seen.insert(&file.rel, &item.name) {
                anyhow::bail!(
                    "the items `{other}` and `{}` both write {}",
                    item.name,
                    file.rel.display()
                );
            }
            files.push(OwnedFile {
                client: file.client.name().to_string(),
                path: rel_to_record(&file.rel)?,
                sha256: sha256_hex(&file.bytes),
            });
        }
        files.sort_by(|a, b| (&a.client, &a.path).cmp(&(&b.client, &b.path)));

        let entry = lock
            .items
            .iter()
            .find(|i| i.kind == item.kind && i.name == item.name);
        // Every hash recorded for `path` by this item: current and pending.
        let recorded_for = |path: &str| -> Vec<OwnedFile> {
            entry
                .map(|e| {
                    e.files
                        .iter()
                        .chain(&e.pending)
                        .filter(|f| f.path == path)
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        };

        let kept_paths: BTreeSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
        let mut stale = Vec::new();
        if let Some(entry) = entry {
            for old in &entry.files {
                if kept_paths.contains(old.path.as_str()) {
                    continue;
                }
                if let Some(rel) = deletable(&old.path) {
                    if all_planned.contains(rel.as_path()) || stale.contains(&rel) {
                        continue;
                    }
                    check_no_item_symlink(target, &rel)?;
                    stale.push(rel);
                }
            }
        }
        // The stale files that the write loop may delete before it writes
        // this item: each planned path is judged against the state after
        // them.
        let mut stale_files = BTreeSet::new();
        for rel in &stale {
            if fs::symlink_metadata(target.join(rel)).is_ok_and(|m| m.file_type().is_file()) {
                stale_files.insert(rel.clone());
            }
        }
        for file in &item.files {
            check_writable(target, &file.rel, &stale_files)?;
        }

        let mut delete = Vec::new();
        let mut kept = Vec::new();
        for rel in stale {
            let path = rel_to_record(&rel)?;
            let records = recorded_for(&path);
            let hashes: Vec<&str> = records.iter().map(|f| f.sha256.as_str()).collect();
            let state = state_of(target, &rel, &path, &hashes)?;
            let is_file = stale_files.contains(&rel);
            match state {
                FileState::Owned | FileState::Absent => delete.push(rel),
                FileState::Modified | FileState::Unowned if overwrite && is_file => {
                    delete.push(rel);
                }
                FileState::Modified | FileState::Unowned => {
                    let old: Vec<OwnedFile> = records
                        .into_iter()
                        .filter(|f| entry.is_some_and(|e| e.files.contains(f)))
                        .collect();
                    kept.push((rel, old));
                }
            }
        }

        let mut conflicts = Vec::new();
        for file in &item.files {
            let path = rel_to_record(&file.rel)?;
            let records = recorded_for(&path);
            let hashes: Vec<&str> = records.iter().map(|f| f.sha256.as_str()).collect();
            if let Some(state) =
                destination_conflict(target, &file.rel, &path, &hashes, &file.bytes)?
            {
                conflicts.push(FileConflict {
                    kind: item.kind,
                    name: item.name.clone(),
                    path: file.rel.clone(),
                    state,
                });
            }
            if !overwrite {
                for (rel, _) in &kept {
                    if file.rel.starts_with(rel) || rel.starts_with(&file.rel) {
                        conflicts.push(FileConflict {
                            kind: item.kind,
                            name: item.name.clone(),
                            path: rel.clone(),
                            state: ConflictState::Modified,
                        });
                    }
                }
            }
        }
        conflicts.sort_by(|a, b| a.path.cmp(&b.path));
        conflicts.dedup();
        changes.push(ItemChanges {
            files,
            delete,
            kept,
            conflicts,
        });
    }

    // A planned file cannot also be a directory on the way to another one.
    let planned_paths: Vec<&Path> = paths_seen.keys().copied().collect();
    for pair in planned_paths.windows(2) {
        if pair[1].starts_with(pair[0]) {
            anyhow::bail!(
                "the planned files {} and {} conflict: the first is a directory of the second",
                pair[0].display(),
                pair[1].display()
            );
        }
    }

    let mut outcome = ApplyOutcome::default();
    for (item, changes) in planned.iter().zip(changes) {
        let ItemChanges {
            mut files,
            delete,
            kept,
            conflicts,
        } = changes;
        if !overwrite && !conflicts.is_empty() {
            outcome.skipped.extend(conflicts);
            continue;
        }
        for rel in &delete {
            delete_recorded(target, rel)?;
        }

        for file in &item.files {
            let full = target.join(&file.rel);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create dir {}", parent.display()))?;
            }
            // The pre-check accepted this path. An existing regular file is
            // removed first, so that a read-only file is replaced too.
            if fs::symlink_metadata(&full).is_ok_and(|m| m.file_type().is_file()) {
                fs::remove_file(&full).with_context(|| format!("remove {}", full.display()))?;
            }
            fs::write(&full, &file.bytes).with_context(|| format!("write {}", full.display()))?;
            if let Some(mode) = file.mode {
                set_mode(&full, mode)?;
            }
        }

        for (rel, old) in kept {
            files.extend(old);
            outcome.kept.push(rel);
        }
        files.sort_by(|a, b| (&a.client, &a.path).cmp(&(&b.client, &b.path)));
        files.dedup();

        if let Some(entry) = lock
            .items
            .iter_mut()
            .find(|i| i.kind == item.kind && i.name == item.name)
        {
            entry.files = files.clone();
        }
        outcome
            .recorded
            .insert((item.kind, item.name.clone()), files);
    }
    Ok(outcome)
}

/// State of the recorded or planned path `rel` (`path` is its record form).
/// A regular file that exists but cannot be read is `Modified` when
/// `recorded` is not empty and `Unowned` otherwise; any other error while
/// inspecting the path fails.
fn state_of(target: &Path, rel: &Path, path: &str, recorded: &[&str]) -> Result<FileState> {
    match classify(target, path, recorded) {
        Ok(state) => Ok(state),
        Err(_) if fs::symlink_metadata(target.join(rel)).is_ok_and(|m| m.file_type().is_file()) => {
            Ok(if recorded.is_empty() {
                FileState::Unowned
            } else {
                FileState::Modified
            })
        }
        Err(err) => Err(err).with_context(|| format!("cannot inspect {path}")),
    }
}

/// The conflict at the planned destination `rel`, if any. An absent path,
/// an `Owned` file, and a regular file whose content equals `bytes` are not
/// conflicts. A directory is not classified here: `check_writable` accepts
/// it only when it holds stale files of the same item, and those are judged
/// as stale files.
fn destination_conflict(
    target: &Path,
    rel: &Path,
    path: &str,
    recorded: &[&str],
    bytes: &[u8],
) -> Result<Option<ConflictState>> {
    let full = target.join(rel);
    if fs::symlink_metadata(&full).is_ok_and(|m| m.file_type().is_dir()) {
        return Ok(None);
    }
    let state = match state_of(target, rel, path, recorded)? {
        FileState::Absent | FileState::Owned => return Ok(None),
        FileState::Modified => ConflictState::Modified,
        FileState::Unowned => ConflictState::Unowned,
    };
    let same = fs::symlink_metadata(&full).is_ok_and(|m| m.file_type().is_file())
        && fs::read(&full).is_ok_and(|current| current == bytes);
    Ok((!same).then_some(state))
}

/// Fails when two planned items have the same `(kind, installed name)`, or
/// when an installed name contains a path separator (`/` or `\`).
/// Changes nothing on disk.
pub(super) fn check_installed_names(planned: &[PlannedItem]) -> Result<()> {
    let mut seen: BTreeSet<(ItemKind, &str)> = BTreeSet::new();
    for item in planned {
        if item.name.contains(['/', '\\']) {
            match &item.source_name {
                Some(source) => anyhow::bail!(
                    "cannot install the {} `{source}` as `{}`: an installed name must not \
                     contain a path separator",
                    item.kind,
                    item.name
                ),
                None => anyhow::bail!(
                    "cannot install the {} `{}`: an installed name must not contain a path \
                     separator",
                    item.kind,
                    item.name
                ),
            }
        }
        if !seen.insert((item.kind, item.name.as_str())) {
            anyhow::bail!(
                "two planned items install the {} `{}`; each installed name must be unique",
                item.kind,
                item.name
            );
        }
    }
    Ok(())
}

/// Check for a path to delete. Fails when a component of `rel`, from the
/// third one to the last one, exists and is a symbolic link. Links are not
/// followed. The check stops at the first component that does not exist or
/// that is a regular file (no path can exist below a regular file, so
/// there is nothing to delete). Planned files use [`check_writable`].
pub(super) fn check_no_item_symlink(target: &Path, rel: &Path) -> Result<()> {
    check_item_components(target, rel, false)
}

/// Check for a planned file `rel`, run before any change on disk.
/// `stale` holds the regular files of the same item that the write loop
/// deletes, followed by the removal of directories left empty, before it
/// writes `rel`. The check judges `rel` against the state after that.
///
/// - The client root and the kind directory (the first two components)
///   may be symbolic links and are followed. When one of them exists, it
///   must resolve to a directory. A symbolic link to a missing target
///   fails.
/// - A later component must not be a symbolic link.
/// - A component between the kind directory and the file that exists must
///   be a directory, or a regular file listed in `stale`.
/// - The file itself, when it exists, must be a regular file, or a
///   directory whose entries, recursively, are all regular files listed in
///   `stale` (it has at least one entry, and every subdirectory too).
/// - A component that does not exist ends the check.
fn check_writable(target: &Path, rel: &Path, stale: &BTreeSet<PathBuf>) -> Result<()> {
    let count = rel.components().count();
    let mut current = target.to_path_buf();
    let mut current_rel = PathBuf::new();
    for (index, component) in rel.components().enumerate() {
        current.push(component);
        current_rel.push(component);
        let last = index + 1 == count;
        let meta = if index < 2 {
            match fs::metadata(&current) {
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    if fs::symlink_metadata(&current).is_ok() {
                        anyhow::bail!(
                            "cannot write {}: {} is a symbolic link to a missing target",
                            target.join(rel).display(),
                            current.display()
                        );
                    }
                    return Ok(());
                }
                other => other,
            }
        } else {
            fs::symlink_metadata(&current)
        };
        let meta = match meta {
            Ok(meta) => meta,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
            Err(err) => {
                return Err(err).with_context(|| format!("inspecting {}", current.display()));
            }
        };
        let file_type = meta.file_type();
        if file_type.is_symlink() {
            anyhow::bail!(
                "refusing to change {}: {} is a symbolic link",
                target.join(rel).display(),
                current.display()
            );
        }
        if last {
            let replaceable_dir =
                file_type.is_dir() && only_stale_files(target, &current_rel, stale);
            if !file_type.is_file() && !replaceable_dir {
                anyhow::bail!(
                    "cannot write {}: it exists and is not a regular file",
                    target.join(rel).display()
                );
            }
        } else if file_type.is_file() && stale.contains(&current_rel) {
            // Deleted before the write; nothing exists below it.
            return Ok(());
        } else if !file_type.is_dir() {
            anyhow::bail!(
                "cannot write {}: {} is not a directory",
                target.join(rel).display(),
                current.display()
            );
        }
    }
    Ok(())
}

/// True when the directory `rel` has at least one entry and every entry,
/// recursively, is a regular file listed in `stale` or a directory for
/// which this is also true. Links are not followed.
fn only_stale_files(target: &Path, rel: &Path, stale: &BTreeSet<PathBuf>) -> bool {
    let Ok(entries) = fs::read_dir(target.join(rel)) else {
        return false;
    };
    let mut any = false;
    for entry in entries {
        let Ok(entry) = entry else {
            return false;
        };
        let Ok(file_type) = entry.file_type() else {
            return false;
        };
        let child = rel.join(entry.file_name());
        let ok = if file_type.is_file() {
            stale.contains(&child)
        } else if file_type.is_dir() {
            only_stale_files(target, &child, stale)
        } else {
            false
        };
        if !ok {
            return false;
        }
        any = true;
    }
    any
}

/// Like [`check_no_item_symlink`], but the last component of `rel` is not
/// checked: only the directories on the way to it.
pub(super) fn check_no_item_symlink_above(target: &Path, rel: &Path) -> Result<()> {
    check_item_components(target, rel, true)
}

fn check_item_components(target: &Path, rel: &Path, skip_last: bool) -> Result<()> {
    let count = rel.components().count();
    let mut current = target.to_path_buf();
    for (index, component) in rel.components().enumerate() {
        current.push(component);
        if index < 2 {
            continue;
        }
        if skip_last && index + 1 == count {
            break;
        }
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => anyhow::bail!(
                "refusing to change {}: {} is a symbolic link",
                target.join(rel).display(),
                current.display()
            ),
            Ok(meta) if meta.file_type().is_file() => return Ok(()),
            Ok(_) => {}
            Err(err) if matches!(err.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
                return Ok(());
            }
            Err(err) => {
                return Err(err).with_context(|| format!("inspecting {}", current.display()));
            }
        }
    }
    Ok(())
}

/// The recorded path `rel` as a relative path, when it may be deleted: only
/// plain components, at least three of them, and a first component that is
/// a client root. `None` for any other recorded path, which is skipped.
pub(super) fn deletable(rel: &str) -> Option<PathBuf> {
    let path = PathBuf::from(rel);
    let parts: Vec<Component> = path.components().collect();
    let plain = parts.iter().all(|c| matches!(c, Component::Normal(_)));
    let in_root = parts
        .first()
        .and_then(|c| c.as_os_str().to_str())
        .is_some_and(|first| CLIENT_ROOTS.contains(&first));
    (plain && parts.len() >= 3 && in_root).then_some(path)
}

/// Deletes the file `rel` (relative to `target`, accepted by [`deletable`]
/// and checked by [`check_no_item_symlink`] in `apply_plan` or by
/// [`check_no_item_symlink_above`] in `remove`), then each parent directory
/// left empty. The client root and the kind directory below it (for
/// example `.claude/skills`) are never removed.
pub(super) fn delete_recorded(target: &Path, rel: &Path) -> Result<()> {
    let full = target.join(rel);
    match fs::symlink_metadata(&full) {
        Ok(meta) if meta.is_dir() => return Ok(()),
        Ok(_) => {
            fs::remove_file(&full).with_context(|| format!("remove {}", full.display()))?;
        }
        // "Not a directory": a parent is a regular file, so the path
        // cannot exist and there is nothing to delete.
        Err(err) if matches!(err.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {}
        Err(err) => return Err(err).with_context(|| format!("inspecting {}", full.display())),
    }
    prune_empty_parents(target, rel);
    Ok(())
}

/// Removes each parent directory of `rel` that is empty, from the deepest
/// upward. Stops at the first directory that is not empty and never
/// removes the client root or the kind directory. Never deletes a file.
pub(super) fn prune_empty_parents(target: &Path, rel: &Path) {
    let mut dir = rel.parent();
    while let Some(d) = dir {
        if d.components().count() <= 2 {
            break;
        }
        // `remove_dir` fails on a directory that is not empty; stop there.
        if fs::remove_dir(target.join(d)).is_err() {
            break;
        }
        dir = d.parent();
    }
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("set permissions of {}", path.display()))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::Client;
    use crate::lockfile::LockedItem;
    use crate::pipeline::plan::PlannedFile;
    use std::path::PathBuf;

    fn planned(name: &str, files: &[(&str, &[u8])]) -> PlannedItem {
        PlannedItem {
            kind: ItemKind::Skill,
            name: name.to_string(),
            source_name: None,
            files: files
                .iter()
                .map(|(rel, bytes)| PlannedFile {
                    client: Client::Claude,
                    rel: PathBuf::from(rel),
                    bytes: bytes.to_vec(),
                    mode: None,
                })
                .collect(),
        }
    }

    fn locked(name: &str, files: Vec<OwnedFile>) -> LockedItem {
        LockedItem {
            kind: ItemKind::Skill,
            name: name.to_string(),
            source: "./src".to_string(),
            git_ref: None,
            hash: None,
            source_name: None,
            required_by: vec![],
            group: None,
            clients: vec![],
            files,
            pending: vec![],
        }
    }

    #[test]
    fn writes_files_and_returns_sorted_records() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = [planned(
            "s",
            &[
                (".claude/skills/s/z.md", b"z"),
                (".claude/skills/s/SKILL.md", b"body"),
            ],
        )];
        let rec = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true).unwrap();
        let files = &rec.recorded[&(ItemKind::Skill, "s".to_string())];
        assert_eq!(files[0].path, ".claude/skills/s/SKILL.md");
        assert_eq!(files[0].sha256, sha256_hex(b"body"));
        assert_eq!(files[1].path, ".claude/skills/s/z.md");
        assert_eq!(
            fs::read(tmp.path().join(".claude/skills/s/SKILL.md")).unwrap(),
            b"body"
        );
    }

    #[test]
    fn deletes_recorded_files_the_plan_no_longer_contains() {
        let tmp = tempfile::tempdir().unwrap();
        let stale = tmp.path().join(".claude/skills/s/old/stale.md");
        fs::create_dir_all(stale.parent().unwrap()).unwrap();
        fs::write(&stale, b"x").unwrap();
        let mut lock = Lockfile::new();
        lock.items.push(locked(
            "s",
            vec![OwnedFile {
                client: "claude".into(),
                path: ".claude/skills/s/old/stale.md".into(),
                sha256: sha256_hex(b"x"),
            }],
        ));
        let plan = [planned("s", &[(".claude/skills/s/SKILL.md", b"body")])];
        apply_plan(tmp.path(), &mut lock, &plan, true).unwrap();
        assert!(!stale.exists());
        assert!(!tmp.path().join(".claude/skills/s/old").exists());
        assert!(tmp.path().join(".claude/skills/s/SKILL.md").exists());
        assert_eq!(lock.items[0].files.len(), 1);
        assert_eq!(lock.items[0].files[0].path, ".claude/skills/s/SKILL.md");
    }

    #[test]
    fn keeps_the_kind_directory_when_the_last_item_file_is_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let stale = tmp.path().join(".claude/skills/s/SKILL.md");
        fs::create_dir_all(stale.parent().unwrap()).unwrap();
        fs::write(&stale, b"x").unwrap();
        let mut lock = Lockfile::new();
        lock.items.push(locked(
            "s",
            vec![OwnedFile {
                client: "claude".into(),
                path: ".claude/skills/s/SKILL.md".into(),
                sha256: sha256_hex(b"x"),
            }],
        ));
        apply_plan(tmp.path(), &mut lock, &[planned("s", &[])], true).unwrap();
        assert!(!tmp.path().join(".claude/skills/s").exists());
        assert!(tmp.path().join(".claude/skills").is_dir());
    }

    #[test]
    fn ignores_a_recorded_path_that_is_not_plain() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside.md");
        fs::write(&outside, b"keep").unwrap();
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        let mut lock = Lockfile::new();
        lock.items.push(locked(
            "s",
            vec![OwnedFile {
                client: "claude".into(),
                path: "../outside.md".into(),
                sha256: sha256_hex(b"keep"),
            }],
        ));
        apply_plan(&target, &mut lock, &[planned("s", &[])], true).unwrap();
        assert!(outside.exists());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_write_through_a_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside.md");
        fs::write(&outside, b"keep").unwrap();
        let link = tmp.path().join(".claude/skills/s/SKILL.md");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let plan = [planned("s", &[(".claude/skills/s/SKILL.md", b"body")])];
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true).unwrap_err();
        assert!(format!("{err:#}").contains("symbolic link"), "{err:#}");
        assert_eq!(fs::read(&outside).unwrap(), b"keep");
    }

    #[cfg(unix)]
    #[test]
    fn sets_the_planned_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let mut item = planned("s", &[(".claude/skills/s/run.sh", b"#!/bin/sh\n")]);
        item.files[0].mode = Some(0o755);
        apply_plan(tmp.path(), &mut Lockfile::new(), &[item], true).unwrap();
        let mode = fs::metadata(tmp.path().join(".claude/skills/s/run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
    }

    fn record(path: &str) -> OwnedFile {
        OwnedFile {
            client: "claude".into(),
            path: path.into(),
            sha256: sha256_hex(b"x"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_symlinked_item_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("SKILL.md"), b"keep").unwrap();
        fs::create_dir_all(tmp.path().join("t/.claude/skills")).unwrap();
        std::os::unix::fs::symlink(&outside, tmp.path().join("t/.claude/skills/s")).unwrap();
        let plan = [planned(
            "s",
            &[
                (".claude/skills/s/SKILL.md", b"body"),
                (".claude/skills/s/new.md", b"new"),
            ],
        )];
        let err = apply_plan(&tmp.path().join("t"), &mut Lockfile::new(), &plan, true).unwrap_err();
        assert!(format!("{err:#}").contains(".claude/skills/s"), "{err:#}");
        assert_eq!(fs::read(outside.join("SKILL.md")).unwrap(), b"keep");
        assert!(!outside.join("new.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn allows_a_symlinked_kind_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real-skills");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        std::os::unix::fs::symlink(&real, tmp.path().join(".claude/skills")).unwrap();
        let plan = [planned("s", &[(".claude/skills/s/SKILL.md", b"body")])];
        apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true).unwrap();
        assert_eq!(fs::read(real.join("s/SKILL.md")).unwrap(), b"body");
    }

    #[cfg(unix)]
    #[test]
    fn does_not_delete_through_a_symlinked_item_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("old.md"), b"keep").unwrap();
        let target = tmp.path().join("t");
        fs::create_dir_all(target.join(".claude/skills")).unwrap();
        std::os::unix::fs::symlink(&outside, target.join(".claude/skills/s")).unwrap();
        let mut lock = Lockfile::new();
        lock.items
            .push(locked("s", vec![record(".claude/skills/s/old.md")]));
        assert!(apply_plan(&target, &mut lock, &[planned("s", &[])], true).is_err());
        assert_eq!(fs::read(outside.join("old.md")).unwrap(), b"keep");
    }

    #[test]
    fn does_not_delete_a_recorded_path_outside_the_client_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let main = tmp.path().join("src/main.rs");
        fs::create_dir_all(main.parent().unwrap()).unwrap();
        fs::write(&main, b"fn main() {}").unwrap();
        let shallow = tmp.path().join(".claude/x.md");
        fs::create_dir_all(shallow.parent().unwrap()).unwrap();
        fs::write(&shallow, b"x").unwrap();
        let mut lock = Lockfile::new();
        lock.items.push(locked(
            "s",
            vec![record("src/main.rs"), record(".claude/x.md")],
        ));
        apply_plan(tmp.path(), &mut lock, &[planned("s", &[])], true).unwrap();
        assert!(main.exists());
        assert!(shallow.exists());
    }

    #[test]
    fn refuses_two_planned_items_with_the_same_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let same_name = [
            planned("s", &[(".claude/skills/s/SKILL.md", b"a")]),
            planned("s", &[(".claude/skills/s/OTHER.md", b"b")]),
        ];
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &same_name, true).unwrap_err();
        assert!(format!("{err}").contains("`s`"), "{err}");
        let same_path = [
            planned("a", &[(".claude/skills/x/SKILL.md", b"a")]),
            planned("b", &[(".claude/skills/x/SKILL.md", b"b")]),
        ];
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &same_path, true).unwrap_err();
        assert!(format!("{err}").contains("`a` and `b`"), "{err}");
        assert!(!tmp.path().join(".claude").exists());
    }
}

#[cfg(test)]
mod pre_check_tests {
    use super::*;
    use crate::generate::Client;
    use crate::pipeline::plan::PlannedFile;

    fn planned(name: &str, rel: &str) -> PlannedItem {
        PlannedItem {
            kind: ItemKind::Skill,
            name: name.to_string(),
            source_name: None,
            files: vec![PlannedFile {
                client: Client::Claude,
                rel: PathBuf::from(rel),
                bytes: b"body".to_vec(),
                mode: None,
            }],
        }
    }

    #[test]
    fn pre_check_fails_when_the_kind_directory_is_a_regular_file() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        fs::write(tmp.path().join(".claude/rules"), b"a file").unwrap();
        let plan = [
            planned("first", ".claude/skills/first/SKILL.md"),
            planned("second", ".claude/rules/second.md"),
        ];
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains(".claude/rules/second.md"), "{text}");
        assert!(text.contains("is not a directory"), "{text}");
        assert!(!tmp.path().join(".claude/skills").exists());
        assert_eq!(
            fs::read(tmp.path().join(".claude/rules")).unwrap(),
            b"a file"
        );
    }

    #[test]
    fn pre_check_fails_when_the_item_directory_is_a_regular_file() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();
        fs::write(tmp.path().join(".claude/skills/second"), b"a file").unwrap();
        let plan = [
            planned("first", ".claude/skills/first/SKILL.md"),
            planned("second", ".claude/skills/second/SKILL.md"),
        ];
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains(".claude/skills/second/SKILL.md"), "{text}");
        assert!(text.contains("is not a directory"), "{text}");
        assert!(!tmp.path().join(".claude/skills/first").exists());
        assert_eq!(
            fs::read(tmp.path().join(".claude/skills/second")).unwrap(),
            b"a file"
        );
    }

    fn stale_record(path: &str, bytes: &[u8]) -> Lockfile {
        let mut lock = Lockfile::new();
        lock.items.push(crate::lockfile::LockedItem {
            kind: ItemKind::Skill,
            name: "s".into(),
            source: "./src".into(),
            git_ref: None,
            hash: None,
            source_name: None,
            required_by: vec![],
            group: None,
            clients: vec![],
            files: vec![OwnedFile {
                client: "claude".into(),
                path: path.into(),
                sha256: sha256_hex(bytes),
            }],
            pending: vec![],
        });
        lock
    }

    #[test]
    fn pre_check_rejects_a_directory_with_an_unrecorded_file_where_a_file_is_planned() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".claude/skills/s/notes");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.md"), b"old").unwrap();
        fs::write(dir.join("mine.md"), b"user file").unwrap();
        let mut lock = stale_record(".claude/skills/s/notes/a.md", b"old");
        let plan = [planned("s", ".claude/skills/s/notes")];
        let err = apply_plan(tmp.path(), &mut lock, &plan, true).unwrap_err();
        assert!(
            format!("{err:#}").contains(".claude/skills/s/notes"),
            "{err:#}"
        );
        assert_eq!(fs::read(dir.join("a.md")).unwrap(), b"old");
        assert_eq!(fs::read(dir.join("mine.md")).unwrap(), b"user file");
    }

    #[test]
    fn pre_check_rejects_a_regular_file_ancestor_that_is_not_stale() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".claude/skills/s")).unwrap();
        fs::write(tmp.path().join(".claude/skills/s/notes"), b"user file").unwrap();
        let plan = [planned("s", ".claude/skills/s/notes/a.md")];
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true).unwrap_err();
        assert!(format!("{err:#}").contains("is not a directory"), "{err:#}");
        assert_eq!(
            fs::read(tmp.path().join(".claude/skills/s/notes")).unwrap(),
            b"user file"
        );
    }

    #[cfg(unix)]
    #[test]
    fn pre_check_fails_when_the_kind_directory_is_a_dangling_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        std::os::unix::fs::symlink(tmp.path().join("missing"), tmp.path().join(".claude/rules"))
            .unwrap();
        let plan = [
            planned("first", ".claude/skills/first/SKILL.md"),
            planned("second", ".claude/rules/second.md"),
        ];
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains(".claude/rules/second.md"), "{text}");
        assert!(text.contains("missing target"), "{text}");
        assert!(!tmp.path().join(".claude/skills").exists());
    }

    #[test]
    fn pre_check_rejects_a_planned_file_that_is_a_prefix_of_another() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = [
            planned("first", ".claude/skills/x/notes"),
            planned("second", ".claude/skills/x/notes/a.md"),
        ];
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true).unwrap_err();
        let text = format!("{err:#}");
        assert!(
            text.contains(
                "the planned files .claude/skills/x/notes and .claude/skills/x/notes/a.md conflict"
            ),
            "{text}"
        );
        assert!(!tmp.path().join(".claude/skills").exists());
    }

    #[test]
    fn ignores_a_recorded_path_with_a_parent_component_inside_a_client_root() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside.md");
        fs::write(&outside, b"keep").unwrap();
        let target = tmp.path().join("t");
        fs::create_dir_all(target.join(".claude/skills")).unwrap();
        let mut lock = stale_record(".claude/skills/../../../outside.md", b"keep");
        let s = PlannedItem {
            files: vec![],
            ..planned("s", ".claude/skills/s/SKILL.md")
        };
        apply_plan(&target, &mut lock, &[s], true).unwrap();
        assert_eq!(fs::read(&outside).unwrap(), b"keep");
    }

    #[cfg(unix)]
    #[test]
    fn delete_side_symlink_check_runs_before_any_write() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("old.md"), b"x").unwrap();
        fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();
        std::os::unix::fs::symlink(&outside, tmp.path().join(".claude/skills/s")).unwrap();
        let mut lock = stale_record(".claude/skills/s/old.md", b"x");
        let s = PlannedItem {
            files: vec![],
            ..planned("s", ".claude/skills/s/SKILL.md")
        };
        let plan = [planned("first", ".claude/skills/first/SKILL.md"), s];
        assert!(apply_plan(tmp.path(), &mut lock, &plan, true).is_err());
        assert!(!tmp.path().join(".claude/skills/first").exists());
        assert_eq!(fs::read(outside.join("old.md")).unwrap(), b"x");
    }

    #[test]
    fn pre_check_rejects_an_empty_directory_where_a_file_is_planned() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".claude/skills/s/notes");
        fs::create_dir_all(&dir).unwrap();
        let plan = [planned("s", ".claude/skills/s/notes")];
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true).unwrap_err();
        assert!(
            format!("{err:#}").contains("is not a regular file"),
            "{err:#}"
        );
        assert!(dir.is_dir());
    }

    #[test]
    fn record_is_sorted_by_client_then_path() {
        let tmp = tempfile::tempdir().unwrap();
        let file = |client, rel: &str| PlannedFile {
            client,
            rel: PathBuf::from(rel),
            bytes: b"body".to_vec(),
            mode: None,
        };
        let item = PlannedItem {
            files: vec![
                file(Client::Copilot, ".github/skills/s/b.md"),
                file(Client::Copilot, ".claude/skills/s/a.md"),
                file(Client::Claude, ".github/skills/s/z.md"),
            ],
            ..planned("s", ".claude/skills/s/SKILL.md")
        };
        let rec = apply_plan(tmp.path(), &mut Lockfile::new(), &[item], true).unwrap();
        let order: Vec<(&str, &str)> = rec.recorded[&(ItemKind::Skill, "s".to_string())]
            .iter()
            .map(|f| (f.client.as_str(), f.path.as_str()))
            .collect();
        assert_eq!(
            order,
            [
                ("claude", ".github/skills/s/z.md"),
                ("copilot", ".claude/skills/s/a.md"),
                ("copilot", ".github/skills/s/b.md"),
            ]
        );
    }

    #[test]
    fn prune_empty_parents_never_deletes_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join(".claude/skills/s/SKILL.md");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"keep").unwrap();
        prune_empty_parents(tmp.path(), Path::new(".claude/skills/s/SKILL.md"));
        assert_eq!(fs::read(&file).unwrap(), b"keep");
    }
}

#[cfg(test)]
mod conflict_tests {
    use super::*;
    use crate::generate::Client;
    use crate::lockfile::LockedItem;
    use crate::pipeline::plan::PlannedFile;

    fn planned(name: &str, files: &[(&str, &[u8])]) -> PlannedItem {
        PlannedItem {
            kind: ItemKind::Skill,
            name: name.to_string(),
            source_name: None,
            files: files
                .iter()
                .map(|(rel, bytes)| PlannedFile {
                    client: Client::Claude,
                    rel: PathBuf::from(rel),
                    bytes: bytes.to_vec(),
                    mode: None,
                })
                .collect(),
        }
    }

    fn owned(path: &str, bytes: &[u8]) -> OwnedFile {
        OwnedFile {
            client: "claude".into(),
            path: path.into(),
            sha256: sha256_hex(bytes),
        }
    }

    fn lock_with(name: &str, files: Vec<OwnedFile>) -> Lockfile {
        let mut lock = Lockfile::new();
        lock.items.push(LockedItem {
            kind: ItemKind::Skill,
            name: name.to_string(),
            source: "./src".to_string(),
            git_ref: None,
            hash: None,
            source_name: None,
            required_by: vec![],
            group: None,
            clients: vec![],
            files,
            pending: vec![],
        });
        lock
    }

    fn put(root: &Path, rel: &str, bytes: &[u8]) {
        let full = root.join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, bytes).unwrap();
    }

    const ENTRY: &str = ".claude/skills/s/SKILL.md";

    #[test]
    fn identical_content_is_not_a_conflict_when_unowned() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"body");
        let outcome = apply_plan(
            tmp.path(),
            &mut Lockfile::new(),
            &[planned("s", &[(ENTRY, b"body")])],
            false,
        )
        .unwrap();
        assert!(outcome.skipped.is_empty());
        assert!(
            outcome
                .recorded
                .contains_key(&(ItemKind::Skill, "s".into()))
        );
    }

    #[test]
    fn identical_content_is_not_a_conflict_when_modified() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"new");
        let mut lock = lock_with("s", vec![owned(ENTRY, b"old")]);
        let outcome = apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[(ENTRY, b"new")])],
            false,
        )
        .unwrap();
        assert!(outcome.skipped.is_empty());
        assert_eq!(lock.items[0].files, vec![owned(ENTRY, b"new")]);
    }

    #[test]
    fn owned_destination_is_replaced_without_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"old");
        let mut lock = lock_with("s", vec![owned(ENTRY, b"old")]);
        let outcome = apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[(ENTRY, b"new")])],
            false,
        )
        .unwrap();
        assert!(outcome.skipped.is_empty());
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"new");
    }

    #[test]
    fn an_item_with_a_conflict_is_skipped_as_a_whole() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"edited");
        put(tmp.path(), ".claude/skills/s/old.md", b"old");
        let before = vec![
            owned(".claude/skills/s/old.md", b"old"),
            owned(ENTRY, b"v1"),
        ];
        let mut lock = lock_with("s", before.clone());
        let plan = [
            planned("s", &[(ENTRY, b"v2"), (".claude/skills/s/new.md", b"new")]),
            planned("t", &[(".claude/skills/t/SKILL.md", b"t")]),
        ];
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false).unwrap();
        assert_eq!(
            outcome.skipped,
            vec![FileConflict {
                kind: ItemKind::Skill,
                name: "s".into(),
                path: PathBuf::from(ENTRY),
                state: ConflictState::Modified,
            }]
        );
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"edited");
        assert!(!tmp.path().join(".claude/skills/s/new.md").exists());
        assert!(tmp.path().join(".claude/skills/s/old.md").exists());
        assert_eq!(lock.items[0].files, before);
        assert!(
            !outcome
                .recorded
                .contains_key(&(ItemKind::Skill, "s".into()))
        );
        assert!(tmp.path().join(".claude/skills/t/SKILL.md").is_file());
    }

    #[test]
    fn an_unowned_destination_is_a_conflict_and_overwrite_replaces_it() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"manual");
        let plan = [planned("s", &[(ENTRY, b"body")])];
        let outcome = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, false).unwrap();
        assert_eq!(outcome.skipped[0].state, ConflictState::Unowned);
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"manual");
        let outcome = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true).unwrap();
        assert!(outcome.skipped.is_empty());
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"body");
    }

    const STALE: &str = ".claude/skills/s/old.md";

    #[test]
    fn an_owned_stale_file_is_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), STALE, b"old");
        let mut lock = lock_with("s", vec![owned(STALE, b"old")]);
        let outcome = apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[(ENTRY, b"e")])],
            false,
        )
        .unwrap();
        assert!(!tmp.path().join(STALE).exists());
        assert!(outcome.kept.is_empty());
        assert_eq!(lock.items[0].files, vec![owned(ENTRY, b"e")]);
    }

    #[test]
    fn a_modified_stale_file_is_kept_and_stays_recorded() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), STALE, b"edited");
        let mut lock = lock_with("s", vec![owned(STALE, b"old")]);
        let outcome = apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[(ENTRY, b"e")])],
            false,
        )
        .unwrap();
        assert_eq!(fs::read(tmp.path().join(STALE)).unwrap(), b"edited");
        assert_eq!(outcome.kept, vec![PathBuf::from(STALE)]);
        assert!(outcome.skipped.is_empty());
        assert_eq!(
            lock.items[0].files,
            vec![owned(ENTRY, b"e"), owned(STALE, b"old")]
        );
    }

    #[test]
    fn overwrite_deletes_a_modified_stale_file() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), STALE, b"edited");
        let mut lock = lock_with("s", vec![owned(STALE, b"old")]);
        let outcome = apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[(ENTRY, b"e")])],
            true,
        )
        .unwrap();
        assert!(!tmp.path().join(STALE).exists());
        assert!(outcome.kept.is_empty());
    }

    #[test]
    fn a_kept_stale_file_on_the_way_to_a_planned_file_is_a_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = ".claude/skills/s/notes";
        put(tmp.path(), notes, b"edited");
        let mut lock = lock_with("s", vec![owned(notes, b"old")]);
        let plan = [planned("s", &[(".claude/skills/s/notes/a.md", b"a")])];
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false).unwrap();
        assert_eq!(outcome.skipped[0].path, PathBuf::from(notes));
        assert_eq!(fs::read(tmp.path().join(notes)).unwrap(), b"edited");
    }

    #[test]
    fn stale_file_of_one_item_is_not_deleted_when_another_item_writes_it() {
        let tmp = tempfile::tempdir().unwrap();
        let shared = ".claude/skills/x/SKILL.md";
        put(tmp.path(), shared, b"old");
        let mut lock = lock_with("s", vec![owned(shared, b"old")]);
        let mut writer = planned("t", &[(shared, b"old")]);
        writer.kind = ItemKind::Agent;
        let plan = [writer, planned("s", &[(ENTRY, b"e")])];
        apply_plan(tmp.path(), &mut lock, &plan, false).unwrap();
        assert_eq!(fs::read(tmp.path().join(shared)).unwrap(), b"old");
        assert_eq!(lock.items[0].files, vec![owned(ENTRY, b"e")]);
    }

    #[test]
    fn refuses_an_installed_name_with_a_path_separator() {
        let tmp = tempfile::tempdir().unwrap();
        for name in ["a/b", "a\\b"] {
            let item = planned(name, &[(".claude/skills/a/SKILL.md", b"x")]);
            let err = check_installed_names(std::slice::from_ref(&item)).unwrap_err();
            assert!(
                err.to_string()
                    .contains(&format!("the skill `{name}`: an installed name must not")),
                "{err}"
            );
            let aliased = PlannedItem {
                source_name: Some("orig".into()),
                ..item
            };
            let err = apply_plan(tmp.path(), &mut Lockfile::new(), &[aliased], true).unwrap_err();
            assert!(
                err.to_string()
                    .contains(&format!("the skill `orig` as `{name}`")),
                "{err}"
            );
        }
        assert!(!tmp.path().join(".claude").exists());
    }
}
