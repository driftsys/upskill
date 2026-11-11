//! Writes an install plan to disk and computes the ownership record of
//! each planned item.
//!
//! Symbolic link rule: the first two components of a layout path (the
//! client root and the kind directory, for example `.claude/skills`) may be
//! symbolic links. Every existing component from the third one to the last
//! one (the item directory, resource subdirectories and the file) must not
//! be a symbolic link. The checks that `apply_plan` runs before any change
//! on disk are described in the rustdoc of `apply_plan` and
//! `check_writable`.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use super::ItemKind;
use super::owned::{rel_to_record, sha256_hex};
use super::plan::PlannedItem;
use crate::lockfile::{Lockfile, OwnedFile};

/// The record of written files for each planned `(kind, installed name)`,
/// sorted by `client`, then `path`. `install_with_lockfile` copies it into
/// the `LockedItem` it upserts for each installed item.
pub(super) type RecordedFiles = BTreeMap<(ItemKind, String), Vec<OwnedFile>>;

/// Client roots under which a recorded path may be deleted.
const CLIENT_ROOTS: [&str; 4] = [".claude", ".github", ".agents", ".opencode"];

/// What `apply_plan` does for one item, computed before any change.
struct ItemChanges {
    files: Vec<OwnedFile>,
    stale: Vec<PathBuf>,
}

/// Writes every planned file and updates `lock.items[..].files` for the
/// planned items that already have a lockfile entry. Before writing an
/// item, it deletes the files of that item's previous record that no
/// planned item of this run writes (the stale files), and the entrypoints
/// of clients the item no longer targets. A recorded path that another
/// planned item writes is not deleted; that item overwrites it. It writes with `fs::write` and does not save the lockfile.
/// Returns the record of every planned item.
///
/// Before any change on disk it fails when two planned items have the same
/// `(kind, name)`, when two planned files have the same path, when one
/// planned file is a directory on the way to another, when a path to write
/// or delete breaks the symbolic link rule of this module, or when a path
/// to write is blocked: the client root or kind directory is a symbolic
/// link to a missing target or is not a directory, a component on the way
/// to the file is not a directory, or the file exists and is not a regular
/// file. A blocking regular file, or a directory that holds only regular
/// files, is accepted when those files are stale recorded files of the same
/// item, because they are deleted (and empty directories removed) before
/// the write. See `check_writable`.
pub(super) fn apply_plan(
    target: &Path,
    lock: &mut Lockfile,
    planned: &[PlannedItem],
) -> Result<RecordedFiles> {
    let mut items_seen: BTreeSet<(ItemKind, &str)> = BTreeSet::new();
    let mut paths_seen: BTreeMap<&Path, &str> = BTreeMap::new();
    let mut changes = Vec::with_capacity(planned.len());
    // No item deletes, as stale, a path that any item of this run writes.
    let all_planned: BTreeSet<&Path> = planned
        .iter()
        .flat_map(|item| item.files.iter().map(|file| file.rel.as_path()))
        .collect();
    for item in planned {
        if !items_seen.insert((item.kind, item.name.as_str())) {
            anyhow::bail!(
                "two planned items install the {} `{}`; each installed name must be unique",
                item.kind,
                item.name
            );
        }
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

        let kept: BTreeSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
        let mut stale = Vec::new();
        if let Some(entry) = lock
            .items
            .iter()
            .find(|i| i.kind == item.kind && i.name == item.name)
        {
            for old in &entry.files {
                if kept.contains(old.path.as_str()) {
                    continue;
                }
                if let Some(rel) = deletable(&old.path) {
                    if all_planned.contains(rel.as_path()) {
                        continue;
                    }
                    check_no_item_symlink(target, &rel)?;
                    stale.push(rel);
                }
            }
        }
        for rel in &item.deselected_entries {
            check_no_item_symlink(target, rel)?;
        }
        // The stale files that the write loop deletes before it writes this
        // item: each planned path is judged against the state after them.
        let mut stale_files = BTreeSet::new();
        for rel in &stale {
            if fs::symlink_metadata(target.join(rel)).is_ok_and(|m| m.file_type().is_file()) {
                stale_files.insert(rel.clone());
            }
        }
        for file in &item.files {
            check_writable(target, &file.rel, &stale_files)?;
        }
        changes.push(ItemChanges { files, stale });
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

    let mut recorded = RecordedFiles::new();
    for (item, ItemChanges { files, stale }) in planned.iter().zip(changes) {
        for rel in &stale {
            delete_recorded(target, rel)?;
        }
        for rel in &item.deselected_entries {
            delete_deselected_entry(target, rel, &item.name);
        }

        for file in &item.files {
            let full = target.join(&file.rel);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create dir {}", parent.display()))?;
            }
            fs::write(&full, &file.bytes).with_context(|| format!("write {}", full.display()))?;
            if let Some(mode) = file.mode {
                set_mode(&full, mode)?;
            }
        }

        if let Some(entry) = lock
            .items
            .iter_mut()
            .find(|i| i.kind == item.kind && i.name == item.name)
        {
            entry.files = files.clone();
        }
        recorded.insert((item.kind, item.name.clone()), files);
    }
    Ok(recorded)
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

/// Best-effort removal of the entrypoint of a client the item no longer
/// targets, and of its parent directory when that directory is named after
/// the item and is empty.
fn delete_deselected_entry(target: &Path, rel: &Path, name: &str) {
    let full = target.join(rel);
    if full.exists() {
        let _ = fs::remove_file(&full);
    }
    if let Some(parent) = full.parent().filter(|parent| {
        parent
            .file_name()
            .and_then(|f| f.to_str())
            .is_some_and(|f| f == name)
            && parent.is_dir()
    }) {
        let _ = fs::remove_dir(parent);
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
            deselected_entries: vec![],
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
        let rec = apply_plan(tmp.path(), &mut Lockfile::new(), &plan).unwrap();
        let files = &rec[&(ItemKind::Skill, "s".to_string())];
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
        apply_plan(tmp.path(), &mut lock, &plan).unwrap();
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
        apply_plan(tmp.path(), &mut lock, &[planned("s", &[])]).unwrap();
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
        apply_plan(&target, &mut lock, &[planned("s", &[])]).unwrap();
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
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan).unwrap_err();
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
        apply_plan(tmp.path(), &mut Lockfile::new(), &[item]).unwrap();
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
        let err = apply_plan(&tmp.path().join("t"), &mut Lockfile::new(), &plan).unwrap_err();
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
        apply_plan(tmp.path(), &mut Lockfile::new(), &plan).unwrap();
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
        assert!(apply_plan(&target, &mut lock, &[planned("s", &[])]).is_err());
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
        apply_plan(tmp.path(), &mut lock, &[planned("s", &[])]).unwrap();
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
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &same_name).unwrap_err();
        assert!(format!("{err}").contains("`s`"), "{err}");
        let same_path = [
            planned("a", &[(".claude/skills/x/SKILL.md", b"a")]),
            planned("b", &[(".claude/skills/x/SKILL.md", b"b")]),
        ];
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &same_path).unwrap_err();
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
            deselected_entries: vec![],
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
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan).unwrap_err();
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
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan).unwrap_err();
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
        let err = apply_plan(tmp.path(), &mut lock, &plan).unwrap_err();
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
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan).unwrap_err();
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
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan).unwrap_err();
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
        let err = apply_plan(tmp.path(), &mut Lockfile::new(), &plan).unwrap_err();
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
    fn prune_empty_parents_never_deletes_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join(".claude/skills/s/SKILL.md");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"keep").unwrap();
        prune_empty_parents(tmp.path(), Path::new(".claude/skills/s/SKILL.md"));
        assert_eq!(fs::read(&file).unwrap(), b"keep");
    }
}
