//! Writes an install plan to disk and computes the ownership record of
//! each planned item.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path};

use super::ItemKind;
use super::owned::{rel_to_record, sha256_hex};
use super::plan::PlannedItem;
use crate::lockfile::{Lockfile, OwnedFile};

/// The record of written files for each planned `(kind, installed name)`,
/// sorted by `client`, then `path`. `install_with_lockfile` copies it into
/// the `LockedItem` it builds for an item that has no lockfile entry yet.
pub(super) type RecordedFiles = BTreeMap<(ItemKind, String), Vec<OwnedFile>>;

/// Writes every planned file and updates `lock.items[..].files` for the
/// planned items that already have a lockfile entry. Before writing an
/// item, it deletes the files of that item's previous record that the plan
/// no longer contains, and the entrypoints of clients the item no longer
/// targets. It writes with `fs::write`, never through a symbolic link, and
/// does not save the lockfile. Returns the record of every planned item.
pub(super) fn apply_plan(
    target: &Path,
    lock: &mut Lockfile,
    planned: &[PlannedItem],
) -> Result<RecordedFiles> {
    // Refuse before any change on disk.
    for item in planned {
        for file in &item.files {
            refuse_symlink(&target.join(&file.rel))?;
        }
    }

    let mut recorded = RecordedFiles::new();
    for item in planned {
        let mut files = Vec::with_capacity(item.files.len());
        for file in &item.files {
            files.push(OwnedFile {
                client: file.client.name().to_string(),
                path: rel_to_record(&file.rel)?,
                sha256: sha256_hex(&file.bytes),
            });
        }
        files.sort_by(|a, b| (&a.client, &a.path).cmp(&(&b.client, &b.path)));

        let entry = lock
            .items
            .iter_mut()
            .find(|i| i.kind == item.kind && i.name == item.name);
        if let Some(previous) = entry.as_ref().map(|e| &e.files) {
            let kept: BTreeSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
            for old in previous {
                if !kept.contains(old.path.as_str()) {
                    delete_recorded(target, &old.path)?;
                }
            }
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

        if let Some(entry) = entry {
            entry.files = files.clone();
        }
        recorded.insert((item.kind, item.name.clone()), files);
    }
    Ok(recorded)
}

/// Fails when `path` exists and is a symbolic link. The link is not
/// followed.
fn refuse_symlink(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => anyhow::bail!(
            "refusing to write {}: the destination is a symbolic link",
            path.display()
        ),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("inspecting {}", path.display())),
    }
}

/// Deletes the recorded file `rel` (a `/`-separated path relative to
/// `target`), then each parent directory left empty. A path that is not a
/// plain relative path is skipped. The client root (`.claude`, `.github`,
/// `.agents`, `.opencode`) and the kind directory below it (for example
/// `.claude/skills`) are never removed.
fn delete_recorded(target: &Path, rel: &str) -> Result<()> {
    let rel = Path::new(rel);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Ok(());
    }
    let full = target.join(rel);
    match fs::symlink_metadata(&full) {
        Ok(meta) if meta.is_dir() => return Ok(()),
        Ok(_) => {
            fs::remove_file(&full).with_context(|| format!("remove {}", full.display()))?;
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(err).with_context(|| format!("inspecting {}", full.display())),
    }
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
    Ok(())
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
}
