//! Writes an install plan to disk and computes the ownership record of
//! each planned item.
//!
//! Symbolic link rule: the first two components of a layout path (the
//! client root and the kind directory, for example `.claude/skills`) may be
//! symbolic links. Every existing component from the third one to the last
//! one (the item directory, resource subdirectories and the file) must not
//! be a symbolic link. The checks that `decide_plan` runs before any
//! change on disk, and the conflict rules for modified and unowned files,
//! are described in the rustdoc of `decide_plan` and `check_writable`.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use super::ItemKind;
use super::owned::{FileState, classify, matches_recorded, rel_to_record, sha256_hex};
use super::plan::{PlannedFile, PlannedItem};
use crate::lockfile::{LockedItem, Lockfile, OwnedFile};

/// The record of written files for each planned `(kind, installed name)`,
/// sorted by `client`, then `path`. `PreparedInstall::commit` copies it
/// into the `LockedItem` it upserts for each installed item.
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
    /// The conflicts of the items that were not written because an update
    /// is withheld from them ([`Disposition::Skip`]).
    pub skipped: Vec<FileConflict>,
    /// The modified destinations of the items that were not written
    /// because nothing is withheld from them ([`Disposition::KeepEdits`]).
    pub kept_edits: Vec<FileConflict>,
    /// Modified files of a previous record that the plan no longer
    /// contains. They are kept on disk and stay in the item's record.
    pub kept: Vec<PathBuf>,
    /// The record of every written item.
    pub recorded: RecordedFiles,
    /// With `overwrite`: the modified or unowned files that were replaced,
    /// and the modified stale files that were deleted.
    pub forced: Vec<PathBuf>,
    /// Every file that was written with new content (it was absent or held
    /// other bytes), whose executable bits were set to the target executable
    /// bits (see [`changes_file`]), or that was deleted;
    /// see [`changes_file`].
    pub changed: Vec<PathBuf>,
}

/// What `execute_plan` will do, computed without any change on disk or
/// in `lock`. One entry per planned item, in the order of `planned`.
pub(super) struct PlanDecision {
    pub items: Vec<ItemChanges>,
}

/// What `execute_plan` does for one item, computed by `decide_plan`
/// before any change.
pub(super) struct ItemChanges {
    pub kind: ItemKind,
    /// Installed name of the item.
    pub name: String,
    /// The record of the planned files, sorted by `client`, then `path`.
    pub files: Vec<OwnedFile>,
    /// Stale files to delete.
    pub delete: Vec<PathBuf>,
    /// Stale files that are kept, with their previous records.
    pub kept: Vec<(PathBuf, Vec<OwnedFile>)>,
    /// Destinations that stop the write of this item.
    pub conflicts: Vec<FileConflict>,
    /// `Owned` destinations that are not rewritten: see the rule in the
    /// rustdoc of [`decide_plan`].
    pub unchanged: BTreeSet<PathBuf>,
    /// What [`execute_plan`] does with the item.
    pub disposition: Disposition,
}

impl ItemChanges {
    /// The `files` record that [`execute_plan`] saves for this item when it
    /// writes it: the planned files and the records of the kept stale
    /// files, sorted by `client`, then `path`, without duplicates.
    pub fn recorded_files(&self) -> Vec<OwnedFile> {
        let mut files = self.files.clone();
        for (_, old) in &self.kept {
            files.extend(old.iter().cloned());
        }
        files.sort_by(|a, b| (&a.client, &a.path).cmp(&(&b.client, &b.path)));
        files.dedup();
        files
    }
}

/// What [`execute_plan`] does with one item. See the rule in the rustdoc
/// of [`decide_plan`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Disposition {
    /// Written (no conflict, or `overwrite`).
    Write,
    /// Not written; an update is withheld. Reported in `ApplyOutcome::skipped`.
    Skip,
    /// Not written; nothing is withheld. Its modified destinations are
    /// reported in `ApplyOutcome::kept_edits`. Its lockfile entry is unchanged.
    KeepEdits,
}

/// Writes an install plan: [`decide_plan`] followed by [`execute_plan`].
/// See the rustdoc of both functions for the rules.
pub(super) fn apply_plan(
    target: &Path,
    lock: &mut Lockfile,
    planned: &[PlannedItem],
    overwrite: bool,
    new_entries: Option<&[LockedItem]>,
    keep_unwithheld_edits: bool,
) -> Result<ApplyOutcome> {
    let decision = decide_plan(
        target,
        lock,
        planned,
        overwrite,
        new_entries,
        keep_unwithheld_edits,
    )?;
    execute_plan(target, lock, planned, decision, new_entries)
}

/// Computes what [`execute_plan`] does for each planned item, without any
/// change on disk or in `lock`.
///
/// Conflicts: every destination of every item is classified with
/// [`classify`] before anything is written. A destination whose content
/// already equals the planned bytes is not a conflict, in any state; at a
/// `Modified` destination, a CRLF variant of the planned bytes (see
/// [`matches_recorded`]) also counts as equal. A destination that is
/// `Modified` or `Unowned` is a conflict. An item without a conflict, or
/// any item when `overwrite` is true, is written ([`Disposition::Write`]):
/// a conflicting regular file is replaced. When `overwrite` is false, an
/// item with at least one conflict is not written: none of its files is
/// written or deleted, and its lockfile record is unchanged. It is
/// [`Disposition::KeepEdits`] (nothing is withheld from it) when all of
/// these hold, and [`Disposition::Skip`] otherwise:
///
/// - `keep_unwithheld_edits` is true;
/// - every conflict is `Modified` and is a planned file of the item (not a
///   kept stale file);
/// - the item has a lockfile entry with an empty `pending` record;
/// - `new_entries` holds the item, its `hash` is `Some`, and its entry
///   equals the lockfile entry in every field except `files`, `pending`
///   and `required_by`
///   (the source of the item did not change, and no field of the record,
///   for example `clients`, would change);
/// - the item has no stale file, to delete or to keep;
/// - every planned file that is not a conflict is `unchanged`.
///
/// An `Owned` destination is `unchanged`: it is not rewritten when its
/// bytes equal the planned bytes, or when
/// they are a CRLF variant of the planned bytes (see [`matches_recorded`]) and the hash of the planned
/// bytes was already recorded for the file in its `files` record (a
/// `pending` hash does not count here). Its record is the hash of the
/// planned bytes. In that case a file with a planned mode is counted in
/// `changed`, and gets its executable bits set to the target executable
/// bits, only when its executable bits differ from that target (Unix only):
/// each executable bit of the planned mode for which the file has the
/// matching read bit, see [`target_executable_bits`]. Its other permission
/// bits are kept. A file without a planned mode keeps its current mode and
/// is not counted.
///
/// Stale files: the files of an item's previous `files` or `pending` record
/// that no planned item of this run writes. So a file that a failed run
/// wrote and that the run that completes the item no longer plans is
/// deleted when it is unchanged, or kept and recorded when it was modified.
/// Before writing an item, its stale files that are unchanged (`Owned`) or
/// absent are deleted, and directories left empty are removed. A stale file
/// that is `Modified`, or that cannot be read, is kept and stays in the
/// record; with `overwrite` it is deleted when it is a regular file. A kept stale file that is a planned path, or a directory
/// on the way to one, is a conflict of the item.
///
/// It fails when two planned items have the same
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
pub(super) fn decide_plan(
    target: &Path,
    lock: &Lockfile,
    planned: &[PlannedItem],
    overwrite: bool,
    new_entries: Option<&[LockedItem]>,
    keep_unwithheld_edits: bool,
) -> Result<PlanDecision> {
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
            // A `pending` path is a recorded path too: a failed run may have
            // written it, and this plan may no longer contain it.
            for old in entry.files.iter().chain(&entry.pending) {
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
        let mut conflicts = Vec::new();
        let mut unchanged = BTreeSet::new();
        for rel in stale {
            let path = rel_to_record(&rel)?;
            let records = recorded_for(&path);
            let hashes: Vec<&str> = records.iter().map(|f| f.sha256.as_str()).collect();
            let state = state_of(target, &rel, &path, &hashes)?;
            let is_file = stale_files.contains(&rel);
            match state {
                FileState::Owned | FileState::Absent => delete.push(rel),
                FileState::Modified | FileState::Unowned if overwrite && is_file => {
                    // Recorded as a conflict, so that it is reported as forced.
                    conflicts.push(FileConflict {
                        kind: item.kind,
                        name: item.name.clone(),
                        path: rel.clone(),
                        state: ConflictState::Modified,
                    });
                    delete.push(rel);
                }
                FileState::Modified | FileState::Unowned => {
                    // Every record of the path, `files` and `pending`, stays.
                    kept.push((rel, records));
                }
            }
        }

        for file in &item.files {
            let path = rel_to_record(&file.rel)?;
            let records = recorded_for(&path);
            let hashes: Vec<&str> = records.iter().map(|f| f.sha256.as_str()).collect();
            // Skip the write when the bytes are equal, or are a CRLF variant
            // of planned bytes whose hash is in the `files` record (the
            // generated content did not change since it was recorded). A
            // `pending` hash does not count: the run that set it may have
            // failed before it wrote this file.
            let planned_hash = sha256_hex(&file.bytes);
            let planned_recorded = entry.is_some_and(|e| {
                e.files
                    .iter()
                    .any(|f| f.path == path && f.sha256 == planned_hash)
            });
            if state_of(target, &file.rel, &path, &hashes)? == FileState::Owned
                && fs::read(target.join(&file.rel)).is_ok_and(|current| {
                    current == file.bytes
                        || (planned_recorded && matches_recorded(&current, &[&planned_hash]))
                })
            {
                unchanged.insert(file.rel.clone());
            }
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
        let disposition = if overwrite || conflicts.is_empty() {
            Disposition::Write
        } else if keep_unwithheld_edits
            && conflicts.iter().all(|c| {
                c.state == ConflictState::Modified
                    && item.files.iter().any(|file| file.rel == c.path)
            })
            && entry.is_some_and(|e| {
                e.pending.is_empty()
                    && e.hash.is_some()
                    && new_entries.is_some_and(|new| {
                        new.iter().any(|n| {
                            n.kind == item.kind && n.name == item.name && same_record(n, e)
                        })
                    })
            })
            && delete.is_empty()
            && kept.is_empty()
            && item.files.iter().all(|file| {
                unchanged.contains(&file.rel) || conflicts.iter().any(|c| c.path == file.rel)
            })
        {
            Disposition::KeepEdits
        } else {
            Disposition::Skip
        };
        changes.push(ItemChanges {
            kind: item.kind,
            name: item.name.clone(),
            files,
            delete,
            kept,
            conflicts,
            unchanged,
            disposition,
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
    Ok(PlanDecision { items: changes })
}

/// Writes every planned item whose disposition in `decision` is
/// [`Disposition::Write`] and updates `lock.items[..].files` for the
/// written items that have a lockfile entry. `decision` is the result of
/// [`decide_plan`] for the same `target`, `lock`, `planned`, `overwrite`
/// and `new_entries`. A `Skip` item is reported in `skipped`, a `KeepEdits`
/// item in `kept_edits`; neither is written, and neither gets a `pending`
/// record.
///
/// With `new_entries` (the lockfile entry of each planned item, given by
/// `PreparedInstall::commit`), the operation can be completed by a second
/// run after a failure. The lockfile is saved in step 2 when at least one
/// item gets a `pending` record, and always in step 4:
///
/// 1. [`decide_plan`] ran its checks; nothing is changed on disk yet.
/// 2. For each item that is written (`Write`) and that
///    writes at least one file or deletes at least one existing stale file,
///    the planned files are added to the `pending` record of its lockfile
///    entry. The old `pending` records that the plan does not repeat stay,
///    so a path and hash of a previous failed run stay recorded until the
///    promotion of step 4. An item that has no entry yet is inserted from
///    `new_entries`, with an empty `files` record. The lockfile is saved
///    when at least one item got a `pending` record.
/// 3. For each item, after the save of step 2: its stale files are deleted
///    first ([`decide_plan`] judges each planned path against the state
///    after that deletion), then each file that is not `unchanged` (see
///    [`decide_plan`]) is written to a stage file `.upskill-stage-<16 hex digits>` in
///    the destination directory, which gets the planned mode (a file
///    without a planned mode gets the default mode of a new file), and is
///    renamed to the destination. On an error the stage file is deleted and
///    the error is returned; files already renamed stay, and the lockfile
///    keeps the `pending` record of step 2.
/// 4. The `files` record of each written item is set to the planned files
///    and the records of its kept stale files, and its `pending` record is
///    cleared. The lockfile is saved.
///
/// A second run finds the files written by a failed run `Owned` through
/// their `pending` hash, and the files not yet written `Owned` through
/// their `files` hash, so neither is a conflict. This holds after several
/// failed runs, because step 2 keeps the old `pending` records. Without
/// `new_entries` (the library entry points), no `pending` record is set and
/// the lockfile is not saved.
pub(super) fn execute_plan(
    target: &Path,
    lock: &mut Lockfile,
    planned: &[PlannedItem],
    decision: PlanDecision,
    new_entries: Option<&[LockedItem]>,
) -> Result<ApplyOutcome> {
    let changes = decision.items;
    // Record the planned files of every item that this run changes on disk
    // as `pending`, and save, before the first change on disk.
    if let Some(new_entries) = new_entries {
        let mut any_pending = false;
        for (item, changes) in planned.iter().zip(&changes) {
            if changes.disposition != Disposition::Write {
                continue;
            }
            // An item that writes no file and deletes no existing file
            // changes nothing on disk, and gets no `pending` record.
            let writes = item
                .files
                .iter()
                .any(|file| !changes.unchanged.contains(&file.rel));
            let deletes = changes
                .delete
                .iter()
                .any(|rel| deletes_existing(target, rel));
            if !writes && !deletes {
                continue;
            }
            any_pending = true;
            let pending = changes.files.clone();
            match lock
                .items
                .iter_mut()
                .find(|i| i.kind == item.kind && i.name == item.name)
            {
                // The old `pending` records that the new plan does not
                // repeat stay: a failed run may have written those files,
                // and they stay owned until the item is promoted.
                Some(entry) => {
                    let mut merged = pending;
                    for old in &entry.pending {
                        if !merged.contains(old) {
                            merged.push(old.clone());
                        }
                    }
                    merged.sort_by(|a, b| (&a.client, &a.path).cmp(&(&b.client, &b.path)));
                    entry.pending = merged;
                }
                // An item without an entry in `new_entries` (for example
                // one with no file for the selected clients) is not
                // recorded in the lockfile, as before this step.
                None => {
                    if let Some(entry) = new_entries
                        .iter()
                        .find(|i| i.kind == item.kind && i.name == item.name)
                    {
                        let mut entry = entry.clone();
                        entry.files = Vec::new();
                        entry.pending = pending;
                        lock.upsert(entry);
                    }
                }
            }
        }
        if any_pending {
            lock.save(target)?;
        }
    }

    let mut outcome = ApplyOutcome::default();
    for (item, changes) in planned.iter().zip(changes) {
        let files = changes.recorded_files();
        let ItemChanges {
            kind,
            name,
            files: _,
            delete,
            kept,
            conflicts,
            unchanged,
            disposition,
        } = changes;
        match disposition {
            Disposition::Write => {}
            Disposition::Skip => {
                outcome.skipped.extend(conflicts);
                continue;
            }
            Disposition::KeepEdits => {
                outcome.kept_edits.extend(conflicts);
                continue;
            }
        }
        outcome
            .forced
            .extend(conflicts.into_iter().map(|conflict| conflict.path));
        for rel in &delete {
            if deletes_existing(target, rel) {
                outcome.changed.push(rel.clone());
            }
            delete_recorded(target, rel)?;
        }

        for file in &item.files {
            let full = target.join(&file.rel);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create dir {}", parent.display()))?;
            }
            if changes_file(target, file, &unchanged) {
                outcome.changed.push(file.rel.clone());
            }
            // An `Owned` destination that already holds the planned bytes, or
            // a CRLF variant of them, is not rewritten. Only its executable
            // bits are set to the target executable bits (the planned
            // executable bits for which the file has the matching read bit),
            // and only when they differ from that target.
            if unchanged.contains(&file.rel) {
                if let Some(mode) = file.mode.filter(|&mode| mode_differs(&full, mode)) {
                    set_executable_bits(&full, mode)?;
                }
                continue;
            }
            // The pre-check accepted this path.
            write_staged(&full, &file.bytes, file.mode)?;
        }

        outcome.kept.extend(kept.into_iter().map(|(rel, _)| rel));

        if let Some(entry) = lock
            .items
            .iter_mut()
            .find(|i| i.kind == kind && i.name == name)
        {
            entry.files = files.clone();
            entry.pending.clear();
        }
        outcome.recorded.insert((kind, name), files);
    }
    if new_entries.is_some() {
        lock.save(target)?;
    }
    Ok(outcome)
}

/// Whether `a` and `b` are equal in every field except `files`, `pending`
/// and `required_by`. The comparison clears those three fields on clones
/// and compares the rest, so a field added to `LockedItem` later is
/// compared too. `required_by` is provenance: the reinstall of a
/// dependency's own source computes it differently from the reinstall of
/// the source that requires the dependency, so it does not count as a
/// change here.
pub(super) fn same_record(a: &LockedItem, b: &LockedItem) -> bool {
    let strip = |item: &LockedItem| LockedItem {
        files: Vec::new(),
        pending: Vec::new(),
        required_by: Vec::new(),
        ..item.clone()
    };
    strip(a) == strip(b)
}

/// Whether [`execute_plan`] changes `file`, so that it counts the file in
/// `ApplyOutcome::changed`: the file is not in `unchanged` and is absent or
/// its bytes differ from the planned bytes, or the file has a planned mode
/// and its current executable bits (`0o111`) differ from the target
/// executable bits: each executable bit of the planned mode for which the
/// file has the matching read bit (Unix only, see [`target_executable_bits`]).
/// A difference in the other permission bits is not a change. For an item with
/// [`Disposition::Write`] only. The preview of an update uses the same
/// predicate, so that it lists exactly the files that the commit changes.
pub(super) fn changes_file(
    target: &Path,
    file: &PlannedFile,
    unchanged: &BTreeSet<PathBuf>,
) -> bool {
    let full = target.join(&file.rel);
    let new_content = !unchanged.contains(&file.rel)
        && !fs::read(&full).is_ok_and(|current| current == file.bytes);
    new_content || file.mode.is_some_and(|mode| mode_differs(&full, mode))
}

/// Whether the file at `path` exists and its executable bits (`0o111`)
/// differ from [`target_executable_bits`] of its mode and `mode`. The other
/// permission bits are not compared.
#[cfg(unix)]
fn mode_differs(path: &Path, mode: u32) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|meta| {
        let current = meta.permissions().mode();
        current & 0o111 != target_executable_bits(current, mode)
    })
}

/// The executable bits that a file with the mode `current` gets from the
/// planned mode `planned`: each executable bit of `planned` for which
/// `current` has the matching read bit (owner, group, other).
/// [`mode_differs`] and [`set_executable_bits`] both use it, so that they
/// agree.
#[cfg(unix)]
fn target_executable_bits(current: u32, planned: u32) -> u32 {
    planned & 0o111 & ((current & 0o444) >> 2)
}

#[cfg(not(unix))]
fn mode_differs(_path: &Path, _mode: u32) -> bool {
    false
}

/// Whether [`execute_plan`] deletes an existing path when it deletes the
/// stale path `rel`, so that it counts `rel` in `ApplyOutcome::changed`.
pub(super) fn deletes_existing(target: &Path, rel: &Path) -> bool {
    fs::symlink_metadata(target.join(rel)).is_ok()
}

/// Writes `bytes` to a new stage file `.upskill-stage-<16 hex digits>` in
/// the directory of `full`, sets `mode` on it when it is `Some` (a new file
/// otherwise has the default mode), and renames it to `full`, which
/// replaces an existing file, also a read-only one. On an error the stage
/// file is deleted. The parent directory of `full` must exist.
fn write_staged(full: &Path, bytes: &[u8], mode: Option<u32>) -> Result<()> {
    let dir = full
        .parent()
        .with_context(|| format!("no parent directory for {}", full.display()))?;
    let (stage, mut file) =
        create_stage_file(dir).with_context(|| format!("write {}", full.display()))?;
    let result = file
        .write_all(bytes)
        .with_context(|| format!("write {}", stage.display()))
        .and_then(|()| {
            // Closed before the rename, which Windows requires.
            drop(file);
            match mode {
                Some(mode) => set_mode(&stage, mode),
                None => Ok(()),
            }
        })
        .and_then(|()| replace_with(&stage, full));
    if result.is_err() {
        let _ = fs::remove_file(&stage);
    }
    result.with_context(|| format!("write {}", full.display()))
}

/// Creates a new stage file in `dir`. The name holds the process id and a
/// counter of this process, as 16 hex digits. A name that exists already
/// (left by a process that was stopped) is not opened; the next counter
/// value is used.
fn create_stage_file(dir: &Path) -> Result<(PathBuf, fs::File)> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    loop {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = dir.join(format!(".upskill-stage-{:08x}{n:08x}", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err).with_context(|| format!("create {}", path.display())),
        }
    }
}

/// Renames `stage` to `full`. On Unix, a rename replaces a read-only file.
/// On Windows it fails on a read-only destination, so the read-only
/// attribute of an existing regular file is cleared first, and set again
/// when the rename fails.
fn replace_with(stage: &Path, full: &Path) -> Result<()> {
    let rename = || {
        fs::rename(stage, full)
            .with_context(|| format!("rename {} to {}", stage.display(), full.display()))
    };
    #[cfg(not(unix))]
    if let Ok(meta) = fs::symlink_metadata(full) {
        if meta.file_type().is_file() && meta.permissions().readonly() {
            let mut permissions = meta.permissions();
            // On Windows this clears the read-only attribute only.
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            fs::set_permissions(full, permissions)
                .with_context(|| format!("set permissions of {}", full.display()))?;
            let result = rename();
            if result.is_err() {
                // The destination was not replaced: set its read-only
                // attribute again.
                let _ = fs::set_permissions(full, meta.permissions());
            }
            return result;
        }
    }
    rename()
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
/// conflicts. For a `Modified` file, "equals" uses [`matches_recorded`], so
/// a CRLF variant of `bytes` is equal. A directory is not classified here:
/// `check_writable` accepts it only when it holds stale files of the same
/// item, and those are judged as stale files.
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
    let planned = sha256_hex(bytes);
    let same = fs::symlink_metadata(&full).is_ok_and(|m| m.file_type().is_file())
        && fs::read(&full).is_ok_and(|current| match state {
            ConflictState::Modified => matches_recorded(&current, &[&planned]),
            ConflictState::Unowned => current == bytes,
        });
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

/// Sets the executable bits (`0o111`) of the file at `path` to
/// [`target_executable_bits`] of its mode and `mode`, and keeps its other
/// permission bits.
#[cfg(unix)]
fn set_executable_bits(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let current = fs::metadata(path)
        .with_context(|| format!("read permissions of {}", path.display()))?
        .permissions()
        .mode()
        & 0o7777;
    set_mode(
        path,
        (current & !0o111) | target_executable_bits(current, mode),
    )
}

#[cfg(not(unix))]
fn set_executable_bits(_path: &Path, _mode: u32) -> Result<()> {
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
        let rec = apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true, None, false).unwrap();
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
        apply_plan(tmp.path(), &mut lock, &plan, true, None, false).unwrap();
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
        apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[])],
            true,
            None,
            false,
        )
        .unwrap();
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
        apply_plan(&target, &mut lock, &[planned("s", &[])], true, None, false).unwrap();
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
        let err =
            apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true, None, false).unwrap_err();
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
        apply_plan(tmp.path(), &mut Lockfile::new(), &[item], true, None, false).unwrap();
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
        let err = apply_plan(
            &tmp.path().join("t"),
            &mut Lockfile::new(),
            &plan,
            true,
            None,
            false,
        )
        .unwrap_err();
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
        apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true, None, false).unwrap();
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
        assert!(apply_plan(&target, &mut lock, &[planned("s", &[])], true, None, false).is_err());
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
        apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[])],
            true,
            None,
            false,
        )
        .unwrap();
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
        let err = apply_plan(
            tmp.path(),
            &mut Lockfile::new(),
            &same_name,
            true,
            None,
            false,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("`s`"), "{err}");
        let same_path = [
            planned("a", &[(".claude/skills/x/SKILL.md", b"a")]),
            planned("b", &[(".claude/skills/x/SKILL.md", b"b")]),
        ];
        let err = apply_plan(
            tmp.path(),
            &mut Lockfile::new(),
            &same_path,
            true,
            None,
            false,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("`a` and `b`"), "{err}");
        assert!(!tmp.path().join(".claude").exists());
    }

    /// Every path under `root` (directories with `None`, files with their
    /// bytes), sorted by path.
    fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        let mut out = BTreeMap::new();
        let mut dirs = vec![root.to_path_buf()];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                let rel = path.strip_prefix(root).unwrap().to_path_buf();
                if path.is_dir() {
                    out.insert(rel, None);
                    dirs.push(path);
                } else {
                    out.insert(rel, Some(fs::read(&path).unwrap()));
                }
            }
        }
        out
    }

    #[test]
    fn decide_plan_changes_nothing_on_disk_or_in_the_lockfile() {
        let tmp = tempfile::tempdir().unwrap();
        let rewritten = ".claude/skills/s/SKILL.md";
        let stale = ".claude/skills/s/old.md";
        let new = ".claude/skills/s/new.md";
        for (rel, bytes) in [(rewritten, b"old" as &[u8]), (stale, b"stale")] {
            let full = tmp.path().join(rel);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(full, bytes).unwrap();
        }
        let record = |path: &str, bytes: &[u8]| OwnedFile {
            client: "claude".into(),
            path: path.into(),
            sha256: sha256_hex(bytes),
        };
        let mut lock = Lockfile::new();
        lock.items.push(locked(
            "s",
            vec![record(rewritten, b"old"), record(stale, b"stale")],
        ));
        let plan = [planned("s", &[(rewritten, b"new body"), (new, b"added")])];

        let tree_before = tree(tmp.path());
        let lock_before = serde_json::to_string(&lock).unwrap();
        let decision = decide_plan(tmp.path(), &lock, &plan, false, None, false).unwrap();

        assert_eq!(tree(tmp.path()), tree_before);
        assert_eq!(serde_json::to_string(&lock).unwrap(), lock_before);
        assert_eq!(decision.items.len(), 1);
        assert_eq!(decision.items[0].delete, vec![PathBuf::from(stale)]);
        assert!(!decision.items[0].unchanged.contains(Path::new(new)));
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
        let err =
            apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true, None, false).unwrap_err();
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
        let err =
            apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true, None, false).unwrap_err();
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
        let err = apply_plan(tmp.path(), &mut lock, &plan, true, None, false).unwrap_err();
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
        let err =
            apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true, None, false).unwrap_err();
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
        let err =
            apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true, None, false).unwrap_err();
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
        let err =
            apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true, None, false).unwrap_err();
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
        apply_plan(&target, &mut lock, &[s], true, None, false).unwrap();
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
        assert!(apply_plan(tmp.path(), &mut lock, &plan, true, None, false).is_err());
        assert!(!tmp.path().join(".claude/skills/first").exists());
        assert_eq!(fs::read(outside.join("old.md")).unwrap(), b"x");
    }

    #[test]
    fn pre_check_rejects_an_empty_directory_where_a_file_is_planned() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".claude/skills/s/notes");
        fs::create_dir_all(&dir).unwrap();
        let plan = [planned("s", ".claude/skills/s/notes")];
        let err =
            apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true, None, false).unwrap_err();
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
        let rec = apply_plan(tmp.path(), &mut Lockfile::new(), &[item], true, None, false).unwrap();
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
            None,
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
            None,
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
            None,
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
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
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
        let outcome =
            apply_plan(tmp.path(), &mut Lockfile::new(), &plan, false, None, false).unwrap();
        assert_eq!(outcome.skipped[0].state, ConflictState::Unowned);
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"manual");
        let outcome =
            apply_plan(tmp.path(), &mut Lockfile::new(), &plan, true, None, false).unwrap();
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
            None,
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
            None,
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
            None,
            false,
        )
        .unwrap();
        assert!(!tmp.path().join(STALE).exists());
        assert!(outcome.kept.is_empty());
        assert_eq!(outcome.forced, vec![PathBuf::from(STALE)]);
    }

    #[test]
    fn an_absent_stale_file_is_dropped_from_the_record() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = lock_with("s", vec![owned(STALE, b"old")]);
        let outcome = apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[(ENTRY, b"e")])],
            false,
            None,
            false,
        )
        .unwrap();
        assert!(outcome.kept.is_empty());
        assert!(outcome.skipped.is_empty());
        assert_eq!(lock.items[0].files, vec![owned(ENTRY, b"e")]);
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_stale_file_is_kept_with_its_record() {
        use std::os::unix::fs::PermissionsExt;
        /// Restores the mode of the file when the test ends, also when an
        /// assertion fails.
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o644));
            }
        }
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), STALE, b"old");
        let stale = tmp.path().join(STALE);
        fs::set_permissions(&stale, fs::Permissions::from_mode(0o000)).unwrap();
        let _restore = Restore(stale.clone());
        if fs::read(&stale).is_ok() {
            // Running as root: permissions do not block the read, so the
            // test checks nothing and returns.
            return;
        }
        let mut lock = lock_with("s", vec![owned(STALE, b"old")]);
        let outcome = apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[(ENTRY, b"e")])],
            false,
            None,
            false,
        )
        .unwrap();
        assert_eq!(outcome.kept, vec![PathBuf::from(STALE)]);
        assert!(stale.exists());
        assert_eq!(
            lock.items[0].files,
            vec![owned(ENTRY, b"e"), owned(STALE, b"old")]
        );
    }

    #[test]
    fn a_pending_hash_counts_as_owned() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"mid");
        let mut lock = lock_with("s", vec![owned(ENTRY, b"old")]);
        lock.items[0].pending = vec![owned(ENTRY, b"mid")];
        let outcome = apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[(ENTRY, b"new")])],
            false,
            None,
            false,
        )
        .unwrap();
        assert!(outcome.skipped.is_empty(), "{:?}", outcome.skipped);
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"new");
    }

    #[test]
    fn an_owned_crlf_variant_of_the_planned_bytes_is_not_rewritten() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"a\r\nb\r\n");
        let mut lock = lock_with("s", vec![owned(ENTRY, b"a\nb\n")]);
        let outcome = apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[(ENTRY, b"a\nb\n")])],
            false,
            None,
            false,
        )
        .unwrap();
        assert!(outcome.changed.is_empty(), "{:?}", outcome.changed);
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"a\r\nb\r\n");
        assert_eq!(lock.items[0].files, vec![owned(ENTRY, b"a\nb\n")]);
    }

    #[test]
    fn an_owned_file_is_skipped_only_when_equal_or_a_crlf_variant_of_recorded_bytes() {
        // Exact equal: not written.
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"a\nb\n");
        let mut lock = lock_with("s", vec![owned(ENTRY, b"a\nb\n")]);
        let plan = [planned("s", &[(ENTRY, b"a\nb\n")])];
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert!(outcome.changed.is_empty(), "{:?}", outcome.changed);

        // CRLF variant, planned hash recorded: not written.
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"a\r\nb\r\n");
        let mut lock = lock_with("s", vec![owned(ENTRY, b"a\nb\n")]);
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert!(outcome.changed.is_empty(), "{:?}", outcome.changed);
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"a\r\nb\r\n");

        // CRLF variant, planned hash not recorded (the record is the CRLF
        // bytes, so the generated content changed): written.
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"a\r\nb\r\n");
        let mut lock = lock_with("s", vec![owned(ENTRY, b"a\r\nb\r\n")]);
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert_eq!(outcome.changed, vec![PathBuf::from(ENTRY)]);
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"a\nb\n");
        assert_eq!(lock.items[0].files, vec![owned(ENTRY, b"a\nb\n")]);
    }

    #[test]
    fn an_unmanaged_crlf_variant_of_the_planned_bytes_is_a_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"a\r\nb\r\n");
        let outcome = apply_plan(
            tmp.path(),
            &mut Lockfile::new(),
            &[planned("s", &[(ENTRY, b"a\nb\n")])],
            false,
            None,
            false,
        )
        .unwrap();
        assert_eq!(outcome.skipped.len(), 1, "{:?}", outcome.skipped);
        assert_eq!(outcome.skipped[0].state, ConflictState::Unowned);
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"a\r\nb\r\n");
    }

    #[test]
    fn a_crlf_variant_of_the_planned_bytes_is_not_a_conflict_when_modified() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"a\r\nb\r\n");
        let mut lock = lock_with("s", vec![owned(ENTRY, b"old")]);
        let outcome = apply_plan(
            tmp.path(),
            &mut lock,
            &[planned("s", &[(ENTRY, b"a\nb\n")])],
            false,
            None,
            false,
        )
        .unwrap();
        assert!(outcome.skipped.is_empty(), "{:?}", outcome.skipped);
        assert_eq!(fs::read(tmp.path().join(ENTRY)).unwrap(), b"a\nb\n");
    }

    #[test]
    fn a_kept_stale_file_below_a_planned_file_is_a_conflict_before_any_write() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = ".claude/skills/s/notes/a.md";
        put(tmp.path(), nested, b"edited");
        let mut lock = lock_with("s", vec![owned(nested, b"old")]);
        let plan = [
            planned("first", &[(".claude/skills/first/SKILL.md", b"f")]),
            planned("s", &[(".claude/skills/s/notes", b"file")]),
        ];
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert_eq!(outcome.skipped.len(), 1, "{:?}", outcome.skipped);
        assert_eq!(outcome.skipped[0].path, PathBuf::from(nested));
        assert_eq!(fs::read(tmp.path().join(nested)).unwrap(), b"edited");
        assert!(tmp.path().join(".claude/skills/first/SKILL.md").is_file());
    }

    #[test]
    fn a_kept_stale_file_on_the_way_to_a_planned_file_is_a_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = ".claude/skills/s/notes";
        put(tmp.path(), notes, b"edited");
        let mut lock = lock_with("s", vec![owned(notes, b"old")]);
        let plan = [planned("s", &[(".claude/skills/s/notes/a.md", b"a")])];
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
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
        apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
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
            let err = apply_plan(
                tmp.path(),
                &mut Lockfile::new(),
                &[aliased],
                true,
                None,
                false,
            )
            .unwrap_err();
            assert!(
                err.to_string()
                    .contains(&format!("the skill `orig` as `{name}`")),
                "{err}"
            );
        }
        assert!(!tmp.path().join(".claude").exists());
    }
}

#[cfg(test)]
mod repeatable_tests {
    use super::*;
    use crate::generate::Client;
    use crate::pipeline::plan::PlannedFile;

    const A: &str = ".claude/skills/s/SKILL.md";
    const B: &str = ".claude/skills/s/b.md";

    fn planned(files: &[(&str, &[u8])]) -> PlannedItem {
        PlannedItem {
            kind: ItemKind::Skill,
            name: "s".to_string(),
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

    fn entry(files: Vec<OwnedFile>, pending: Vec<OwnedFile>) -> LockedItem {
        LockedItem {
            kind: ItemKind::Skill,
            name: "s".to_string(),
            source: "./src".to_string(),
            git_ref: None,
            hash: Some("new-hash".into()),
            source_name: None,
            required_by: vec![],
            group: None,
            clients: vec![],
            files,
            pending,
        }
    }

    fn put(root: &Path, rel: &str, bytes: &[u8]) {
        let full = root.join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, bytes).unwrap();
    }

    fn stage_files(dir: &Path) -> Vec<PathBuf> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".upskill-stage-")
            })
            .collect()
    }

    #[cfg(unix)]
    #[test]
    fn pending_is_saved_before_the_first_write() {
        use std::os::unix::fs::PermissionsExt;
        /// Restores the mode when the test ends, also when an assertion
        /// fails.
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
            }
        }
        let tmp = tempfile::tempdir().unwrap();
        let skills = tmp.path().join(".claude/skills");
        fs::create_dir_all(&skills).unwrap();
        fs::set_permissions(&skills, fs::Permissions::from_mode(0o555)).unwrap();
        let _restore = Restore(skills.clone());
        if fs::create_dir(skills.join("probe")).is_ok() {
            // Running as root: permissions do not block the write.
            return;
        }
        let mut lock = Lockfile::new();
        let new = [entry(vec![], vec![])];
        let plan = [planned(&[(A, b"a")])];
        assert!(apply_plan(tmp.path(), &mut lock, &plan, false, Some(&new), false).is_err());

        let on_disk = Lockfile::load(tmp.path()).unwrap();
        assert_eq!(on_disk.items.len(), 1);
        let item = &on_disk.items[0];
        assert!(item.files.is_empty());
        assert_eq!(item.pending, vec![owned(A, b"a")]);
        assert_eq!(item.hash.as_deref(), Some("new-hash"));
    }

    #[test]
    fn promotion_clears_pending_and_saves() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = Lockfile::new();
        let new = [entry(vec![], vec![])];
        let plan = [planned(&[(A, b"a"), (B, b"b")])];
        apply_plan(tmp.path(), &mut lock, &plan, false, Some(&new), false).unwrap();

        let on_disk = Lockfile::load(tmp.path()).unwrap();
        let item = &on_disk.items[0];
        assert!(item.pending.is_empty());
        assert_eq!(item.files, vec![owned(A, b"a"), owned(B, b"b")]);
        assert_eq!(lock, on_disk);
    }

    #[test]
    fn a_skipped_item_gets_no_pending_record() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), A, b"manual");
        let mut lock = Lockfile::new();
        let new = [entry(vec![], vec![])];
        let plan = [planned(&[(A, b"a")])];
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, Some(&new), false).unwrap();
        assert_eq!(outcome.skipped.len(), 1);
        assert!(lock.items.is_empty());
    }

    #[test]
    fn a_second_run_completes_with_one_file_new_and_one_file_old() {
        let tmp = tempfile::tempdir().unwrap();
        // The failed run wrote A with the new bytes; B still holds the old.
        put(tmp.path(), A, b"a2");
        put(tmp.path(), B, b"b1");
        let mut lock = Lockfile::new();
        lock.items.push(entry(
            vec![owned(A, b"a1"), owned(B, b"b1")],
            vec![owned(A, b"a2"), owned(B, b"b2")],
        ));
        let new = [entry(vec![], vec![])];
        let plan = [planned(&[(A, b"a2"), (B, b"b2")])];
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, Some(&new), false).unwrap();

        assert!(outcome.skipped.is_empty(), "{:?}", outcome.skipped);
        assert_eq!(outcome.changed, vec![PathBuf::from(B)]);
        assert_eq!(fs::read(tmp.path().join(A)).unwrap(), b"a2");
        assert_eq!(fs::read(tmp.path().join(B)).unwrap(), b"b2");
        let item = &Lockfile::load(tmp.path()).unwrap().items[0];
        assert!(item.pending.is_empty());
        assert_eq!(item.files, vec![owned(A, b"a2"), owned(B, b"b2")]);
    }

    #[test]
    fn a_pending_hash_does_not_skip_the_write_of_a_crlf_variant() {
        let tmp = tempfile::tempdir().unwrap();
        // `files` holds the CRLF bytes X, `pending` the LF bytes X', and the
        // disk holds X: the failed run did not write this file.
        put(tmp.path(), A, b"a\r\nb\r\n");
        let mut lock = Lockfile::new();
        lock.items.push(entry(
            vec![owned(A, b"a\r\nb\r\n")],
            vec![owned(A, b"a\nb\n")],
        ));
        let plan = [planned(&[(A, b"a\nb\n")])];
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert_eq!(outcome.changed, vec![PathBuf::from(A)]);
        assert_eq!(fs::read(tmp.path().join(A)).unwrap(), b"a\nb\n");
    }

    #[test]
    fn a_failed_write_leaves_no_stage_file() {
        let tmp = tempfile::tempdir().unwrap();
        // A destination that is a directory with an entry: the rename fails.
        let dest = tmp.path().join("dest");
        fs::create_dir_all(dest.join("inner")).unwrap();
        let err = write_staged(&dest, b"x", None).unwrap_err();
        assert!(
            format!("{err}").contains(&format!("write {}", dest.display())),
            "{err:#}"
        );
        assert!(stage_files(tmp.path()).is_empty());
        assert!(dest.join("inner").is_dir());
    }

    #[test]
    fn stage_file_names_have_16_hex_digits_and_differ() {
        let tmp = tempfile::tempdir().unwrap();
        let (first, _f1) = create_stage_file(tmp.path()).unwrap();
        let (second, _f2) = create_stage_file(tmp.path()).unwrap();
        assert_ne!(first, second);
        for path in [&first, &second] {
            let name = path.file_name().unwrap().to_str().unwrap();
            let digits = name.strip_prefix(".upskill-stage-").unwrap();
            assert_eq!(digits.len(), 16, "{name}");
            assert!(digits.chars().all(|c| c.is_ascii_hexdigit()), "{name}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn write_staged_sets_the_mode_on_a_replaced_file() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("run.sh");
        fs::write(&dest, b"old").unwrap();
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o644)).unwrap();
        write_staged(&dest, b"new", Some(0o755)).unwrap();
        let mode = fs::metadata(&dest).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
    }

    /// Sets `dir` to mode 0o555 and restores 0o755 when dropped, also when
    /// an assertion fails. `None` when the process can still write into
    /// `dir` (running as root).
    #[cfg(unix)]
    struct ReadOnly(PathBuf);

    #[cfg(unix)]
    impl ReadOnly {
        fn set(dir: &Path) -> Option<Self> {
            use std::os::unix::fs::PermissionsExt;
            fs::create_dir_all(dir).unwrap();
            fs::set_permissions(dir, fs::Permissions::from_mode(0o555)).unwrap();
            let guard = ReadOnly(dir.to_path_buf());
            let probe = dir.join(".probe");
            if fs::write(&probe, b"x").is_ok() {
                let _ = fs::remove_file(&probe);
                return None;
            }
            Some(guard)
        }
    }

    #[cfg(unix)]
    impl Drop for ReadOnly {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
        }
    }

    fn named(mut item: LockedItem, name: &str) -> LockedItem {
        item.name = name.to_string();
        item
    }

    /// Item `s` holds its planned bytes; item `t` fails on its first write.
    /// Only `t` is changed by the run, so only `t` gets a `pending` record.
    #[cfg(unix)]
    #[test]
    fn an_item_that_changes_nothing_gets_no_pending_record() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), A, b"a");
        let mut lock = Lockfile::new();
        lock.items.push(entry(vec![owned(A, b"a")], vec![]));
        lock.save(tmp.path()).unwrap();
        let t_rel = ".claude/skills/t/SKILL.md";
        let Some(_guard) = ReadOnly::set(&tmp.path().join(".claude/skills/t")) else {
            return;
        };
        let mut t = planned(&[(t_rel, b"t")]);
        t.name = "t".into();
        let plan = [planned(&[(A, b"a")]), t];
        let new = [entry(vec![], vec![]), named(entry(vec![], vec![]), "t")];
        assert!(apply_plan(tmp.path(), &mut lock, &plan, false, Some(&new), false).is_err());

        let on_disk = Lockfile::load(tmp.path()).unwrap();
        let s = on_disk.items.iter().find(|i| i.name == "s").unwrap();
        assert!(s.pending.is_empty(), "{s:?}");
        let t = on_disk.items.iter().find(|i| i.name == "t").unwrap();
        assert_eq!(t.pending, vec![owned(t_rel, b"t")]);
    }

    /// Runs `plan` followed by an item `t` whose first write fails, with
    /// `lock` saved first. Returns the lockfile on disk after the failure,
    /// or `None` when running as root.
    #[cfg(unix)]
    fn run_then_fail_in_t(
        root: &Path,
        mut lock: Lockfile,
        mut plan: Vec<PlannedItem>,
    ) -> Option<Lockfile> {
        lock.save(root).unwrap();
        let _guard = ReadOnly::set(&root.join(".claude/skills/t"))?;
        let mut t = planned(&[(".claude/skills/t/SKILL.md", b"t")]);
        t.name = "t".into();
        plan.push(t);
        let mut new: Vec<LockedItem> = plan
            .iter()
            .map(|item| named(entry(vec![], vec![]), &item.name))
            .collect();
        new.dedup_by(|a, b| a.name == b.name);
        assert!(apply_plan(root, &mut lock, &plan, false, Some(&new), false).is_err());
        Some(Lockfile::load(root).unwrap())
    }

    /// An item that changes nothing keeps the `pending` record that it
    /// already has (an interrupted item whose files are all written).
    #[cfg(unix)]
    #[test]
    fn an_item_that_changes_nothing_keeps_its_old_pending_record() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), A, b"a");
        let mut lock = Lockfile::new();
        lock.items
            .push(entry(vec![owned(A, b"a0")], vec![owned(A, b"a")]));
        let Some(on_disk) = run_then_fail_in_t(tmp.path(), lock, vec![planned(&[(A, b"a")])])
        else {
            return;
        };
        let s = on_disk.items.iter().find(|i| i.name == "s").unwrap();
        assert_eq!(s.pending, vec![owned(A, b"a")]);
    }

    /// An item whose planned files are unchanged gets `pending` when it
    /// deletes an existing stale file, and does not get it when its only
    /// stale file is absent.
    #[cfg(unix)]
    #[test]
    fn an_item_that_only_deletes_a_stale_file_gets_a_pending_record() {
        let tmp = tempfile::tempdir().unwrap();
        let u_a = ".claude/skills/u/SKILL.md";
        put(tmp.path(), A, b"a");
        put(tmp.path(), B, b"b");
        put(tmp.path(), u_a, b"u");
        let mut lock = Lockfile::new();
        lock.items
            .push(entry(vec![owned(A, b"a"), owned(B, b"b")], vec![]));
        // The stale file of `u` is absent on disk.
        lock.items.push(named(
            entry(
                vec![owned(u_a, b"u"), owned(".claude/skills/u/gone.md", b"g")],
                vec![],
            ),
            "u",
        ));
        let mut u = planned(&[(u_a, b"u")]);
        u.name = "u".into();
        let Some(on_disk) = run_then_fail_in_t(tmp.path(), lock, vec![planned(&[(A, b"a")]), u])
        else {
            return;
        };
        let s = on_disk.items.iter().find(|i| i.name == "s").unwrap();
        assert_eq!(s.pending, vec![owned(A, b"a")]);
        let u = on_disk.items.iter().find(|i| i.name == "u").unwrap();
        assert!(u.pending.is_empty(), "{u:?}");
    }

    /// The merged `pending` record is sorted by client and path, and a
    /// second failed run of the same plan adds no duplicate.
    #[cfg(unix)]
    #[test]
    fn the_merged_pending_record_is_sorted_and_has_no_duplicate() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = Lockfile::new();
        // A (pending only, absent) sorts before the planned B.
        lock.items.push(entry(vec![], vec![owned(A, b"a0")]));
        lock.save(tmp.path()).unwrap();
        // An unrecorded file keeps the item directory from being removed as
        // empty after the absent stale path A is handled.
        put(tmp.path(), ".claude/skills/s/keep.md", b"k");
        let Some(_guard) = ReadOnly::set(&tmp.path().join(".claude/skills/s")) else {
            return;
        };
        let new = [entry(vec![], vec![])];
        let plan = [planned(&[(B, b"b")])];
        for _ in 0..2 {
            let mut lock = Lockfile::load(tmp.path()).unwrap();
            assert!(apply_plan(tmp.path(), &mut lock, &plan, false, Some(&new), false).is_err());
            let on_disk = Lockfile::load(tmp.path()).unwrap();
            assert_eq!(
                on_disk.items[0].pending,
                vec![owned(A, b"a0"), owned(B, b"b")]
            );
        }
    }

    /// The pending save of a run that fails keeps the old `pending` records
    /// that the new plan does not repeat: R (no longer planned) and the
    /// older hash of A.
    #[cfg(unix)]
    fn second_failed_run(r_bytes: &[u8]) -> Option<(tempfile::TempDir, Lockfile)> {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), A, b"a2");
        put(tmp.path(), R, r_bytes);
        let mut lock = Lockfile::new();
        lock.items.push(entry(
            vec![owned(A, b"a1")],
            vec![owned(A, b"a2"), owned(R, b"r")],
        ));
        lock.save(tmp.path()).unwrap();
        let new = [entry(vec![], vec![])];
        let plan = [planned(&[(A, b"a3")])];
        {
            let _guard = ReadOnly::set(&tmp.path().join(".claude/skills/s"))?;
            assert!(apply_plan(tmp.path(), &mut lock, &plan, false, Some(&new), false).is_err());
            let on_disk = Lockfile::load(tmp.path()).unwrap();
            let pending = &on_disk.items[0].pending;
            for record in [owned(A, b"a2"), owned(A, b"a3"), owned(R, b"r")] {
                assert!(pending.contains(&record), "{pending:?}");
            }
            assert_eq!(pending.len(), 3, "{pending:?}");
            assert_eq!(on_disk.items[0].files, vec![owned(A, b"a1")]);
        }
        let mut lock = Lockfile::load(tmp.path()).unwrap();
        apply_plan(tmp.path(), &mut lock, &plan, false, Some(&new), false).unwrap();
        let lock = Lockfile::load(tmp.path()).unwrap();
        Some((tmp, lock))
    }

    const R: &str = ".claude/skills/s/scripts/run.sh";

    #[cfg(unix)]
    #[test]
    fn an_owned_pending_path_keeps_its_record_when_a_second_run_fails() {
        let Some((tmp, lock)) = second_failed_run(b"r") else {
            return;
        };
        assert!(!tmp.path().join(R).exists());
        assert_eq!(fs::read(tmp.path().join(A)).unwrap(), b"a3");
        assert!(lock.items[0].pending.is_empty());
        assert_eq!(lock.items[0].files, vec![owned(A, b"a3")]);
    }

    #[cfg(unix)]
    #[test]
    fn a_modified_pending_path_survives_a_second_failed_run_and_is_then_kept() {
        let Some((tmp, lock)) = second_failed_run(b"edited") else {
            return;
        };
        assert_eq!(fs::read(tmp.path().join(R)).unwrap(), b"edited");
        assert!(lock.items[0].pending.is_empty());
        assert_eq!(lock.items[0].files, vec![owned(A, b"a3"), owned(R, b"r")]);
    }

    /// With `overwrite`, an item with a conflict is written, so it gets a
    /// `pending` record before the first write.
    #[cfg(unix)]
    #[test]
    fn an_item_forced_over_a_conflict_gets_a_pending_record() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), A, b"manual");
        let sub = ".claude/skills/s/sub/b.md";
        let Some(_guard) = ReadOnly::set(&tmp.path().join(".claude/skills/s/sub")) else {
            return;
        };
        let mut lock = Lockfile::new();
        let new = [entry(vec![], vec![])];
        let plan = [planned(&[(A, b"a"), (sub, b"b")])];
        assert!(apply_plan(tmp.path(), &mut lock, &plan, true, Some(&new), false).is_err());
        assert_eq!(fs::read(tmp.path().join(A)).unwrap(), b"a");
        let on_disk = Lockfile::load(tmp.path()).unwrap();
        assert_eq!(on_disk.items.len(), 1);
        assert_eq!(
            on_disk.items[0].pending,
            vec![owned(A, b"a"), owned(sub, b"b")]
        );
    }

    /// A refusal raised inside `apply_plan`, after the classification and
    /// before the pending save, leaves the lockfile bytes unchanged.
    #[test]
    fn a_refusal_inside_apply_plan_saves_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = Lockfile::new();
        lock.items.push(entry(vec![owned(A, b"a")], vec![]));
        lock.save(tmp.path()).unwrap();
        let before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();
        let new = [entry(vec![], vec![])];
        let below = ".claude/skills/s/SKILL.md/x.md";
        let plan = [planned(&[(A, b"a2"), (below, b"x")])];
        let err = apply_plan(tmp.path(), &mut lock, &plan, false, Some(&new), false).unwrap_err();
        assert!(format!("{err}").contains("conflict"), "{err:#}");
        assert_eq!(
            fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
            before
        );
    }

    #[test]
    fn library_mode_does_not_save_the_lockfile() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = Lockfile::new();
        lock.items.push(entry(vec![], vec![]));
        let plan = [planned(&[(A, b"a")])];
        apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert_eq!(fs::read(tmp.path().join(A)).unwrap(), b"a");
        assert!(!tmp.path().join(".upskill-lock.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn an_unchanged_file_gets_its_planned_executable_bits_again_and_is_counted() {
        // The executable bits differ, so the file is counted as changed.
        let (changed, mode) = apply_unchanged_with_modes(0o644, 0o755);
        assert_eq!(changed, vec![PathBuf::from(A)]);
        assert_eq!(mode, 0o755);
    }

    /// Writes `A` with mode `on_disk`, records it as owned, and applies a
    /// plan with the same bytes and the planned mode `planned_mode`. Returns
    /// `ApplyOutcome::changed` and the mode of `A` after the apply.
    #[cfg(unix)]
    fn apply_unchanged_with_modes(on_disk: u32, planned_mode: u32) -> (Vec<PathBuf>, u32) {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), A, b"a");
        let full = tmp.path().join(A);
        fs::set_permissions(&full, fs::Permissions::from_mode(on_disk)).unwrap();
        let mut lock = Lockfile::new();
        lock.items.push(entry(vec![owned(A, b"a")], vec![]));
        let mut item = planned(&[(A, b"a")]);
        item.files[0].mode = Some(planned_mode);
        let outcome = apply_plan(tmp.path(), &mut lock, &[item], false, None, false).unwrap();
        let mode = fs::metadata(&full).unwrap().permissions().mode() & 0o7777;
        (outcome.changed, mode)
    }

    #[cfg(unix)]
    #[test]
    fn a_difference_in_the_other_permission_bits_is_not_a_change() {
        let (changed, mode) = apply_unchanged_with_modes(0o600, 0o644);
        assert!(changed.is_empty(), "{changed:?}");
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn added_executable_bits_follow_the_read_bits_on_disk() {
        for (on_disk, after) in [(0o600, 0o700), (0o640, 0o750), (0o644, 0o755)] {
            let (changed, mode) = apply_unchanged_with_modes(on_disk, 0o755);
            assert_eq!(changed, vec![PathBuf::from(A)], "{on_disk:o}");
            assert_eq!(mode, after, "{on_disk:o}");
        }
    }

    /// A second apply of the same plan finds the executable bits equal to
    /// the target and changes nothing.
    #[cfg(unix)]
    #[test]
    fn a_second_apply_after_adding_executable_bits_changes_nothing() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), A, b"a");
        let full = tmp.path().join(A);
        fs::set_permissions(&full, fs::Permissions::from_mode(0o600)).unwrap();
        let mut lock = Lockfile::new();
        lock.items.push(entry(vec![owned(A, b"a")], vec![]));
        let mut item = planned(&[(A, b"a")]);
        item.files[0].mode = Some(0o755);
        let plan = [item];
        let first = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert_eq!(first.changed, vec![PathBuf::from(A)]);
        let second = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert!(second.changed.is_empty(), "{:?}", second.changed);
        let mode = fs::metadata(&full).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, 0o700);
    }

    #[cfg(unix)]
    #[test]
    fn target_executable_bits_follow_the_read_bits() {
        assert_eq!(target_executable_bits(0o600, 0o755), 0o100);
        assert_eq!(target_executable_bits(0o640, 0o755), 0o110);
        assert_eq!(target_executable_bits(0o644, 0o755), 0o111);
        assert_eq!(target_executable_bits(0o755, 0o644), 0);
        assert_eq!(target_executable_bits(0o711, 0o755), 0o100);
        // No read bit at all: no executable bit.
        assert_eq!(target_executable_bits(0o200, 0o755), 0);
    }

    /// Adding executable bits keeps the setuid, setgid and sticky bits of
    /// the file, each one alone and all three together.
    #[cfg(unix)]
    #[test]
    fn special_bits_are_kept_when_executable_bits_are_added() {
        for (on_disk, after) in [
            (0o4644, 0o4755),
            (0o2644, 0o2755),
            (0o1644, 0o1755),
            (0o7644, 0o7755),
        ] {
            let (changed, mode) = apply_unchanged_with_modes(on_disk, 0o755);
            assert_eq!(changed, vec![PathBuf::from(A)], "{on_disk:o}");
            assert_eq!(mode, after, "{on_disk:o}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn removing_executable_bits_keeps_the_other_permission_bits() {
        let (changed, mode) = apply_unchanged_with_modes(0o750, 0o644);
        assert_eq!(changed, vec![PathBuf::from(A)]);
        assert_eq!(mode, 0o640);
    }

    #[cfg(unix)]
    #[test]
    fn a_read_only_file_gets_its_executable_bits() {
        let (changed, mode) = apply_unchanged_with_modes(0o444, 0o755);
        assert_eq!(changed, vec![PathBuf::from(A)]);
        assert_eq!(mode, 0o555);
    }

    /// The target executable bits are the planned ones where the file on
    /// disk has the matching read bit: 0o755 & 0o111 & (0o711 & 0o444) >> 2
    /// is 0o100. By that rule the group and other executable bits of 0o711
    /// are removed.
    #[cfg(unix)]
    #[test]
    fn executable_bits_without_a_matching_read_bit_are_removed() {
        let (changed, mode) = apply_unchanged_with_modes(0o711, 0o755);
        assert_eq!(changed, vec![PathBuf::from(A)]);
        assert_eq!(mode, 0o700);
    }

    #[cfg(unix)]
    #[test]
    fn removed_executable_bits_keep_the_other_permission_bits() {
        let (changed, mode) = apply_unchanged_with_modes(0o755, 0o644);
        assert_eq!(changed, vec![PathBuf::from(A)]);
        assert_eq!(mode, 0o644);
    }

    #[cfg(unix)]
    #[test]
    fn write_staged_without_a_mode_gives_the_default_mode_of_a_new_file() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let fresh = tmp.path().join("fresh");
        fs::write(&fresh, b"x").unwrap();
        let default = fs::metadata(&fresh).unwrap().permissions().mode() & 0o777;
        let dest = tmp.path().join("run.sh");
        fs::write(&dest, b"old").unwrap();
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o700)).unwrap();
        assert_ne!(default, 0o700);
        write_staged(&dest, b"new", None).unwrap();
        let mode = fs::metadata(&dest).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, default);
    }

    /// The names after the one just created exist already: the next stage
    /// file gets a name that does not exist, and no existing file changes.
    #[test]
    fn create_stage_file_does_not_open_an_existing_name() {
        let tmp = tempfile::tempdir().unwrap();
        let (first, f1) = create_stage_file(tmp.path()).unwrap();
        drop(f1);
        let name = first.file_name().unwrap().to_str().unwrap();
        let digits = name.strip_prefix(".upskill-stage-").unwrap();
        let (pid, n) = digits.split_at(8);
        assert_eq!(pid, format!("{:08x}", std::process::id()));
        let n = u32::from_str_radix(n, 16).unwrap();
        let mut existing = Vec::new();
        for next in n + 1..n + 65 {
            let path = tmp.path().join(format!(".upskill-stage-{pid}{next:08x}"));
            fs::write(&path, b"keep").unwrap();
            existing.push(path);
        }
        let (second, f2) = create_stage_file(tmp.path()).unwrap();
        drop(f2);
        assert!(!existing.contains(&second), "{second:?}");
        for path in &existing {
            assert_eq!(fs::read(path).unwrap(), b"keep", "{path:?}");
        }
    }

    #[test]
    fn write_staged_replaces_an_existing_file_and_leaves_no_stage_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("f.md");
        fs::write(&dest, b"old").unwrap();
        let mut permissions = fs::metadata(&dest).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&dest, permissions).unwrap();
        write_staged(&dest, b"new", None).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new");
        assert!(stage_files(tmp.path()).is_empty());
    }
}

#[cfg(test)]
mod pending_stale_tests {
    use super::*;
    use crate::generate::Client;
    use crate::pipeline::plan::PlannedFile;

    const A: &str = ".claude/skills/s/SKILL.md";
    const R: &str = ".claude/skills/s/scripts/run.sh";

    fn planned(files: &[(&str, &[u8])]) -> PlannedItem {
        PlannedItem {
            kind: ItemKind::Skill,
            name: "s".to_string(),
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

    fn lock_with(files: Vec<OwnedFile>, pending: Vec<OwnedFile>) -> Lockfile {
        let mut lock = Lockfile::new();
        lock.items.push(LockedItem {
            kind: ItemKind::Skill,
            name: "s".to_string(),
            source: "./src".to_string(),
            git_ref: None,
            hash: None,
            source_name: None,
            required_by: vec![],
            group: None,
            clients: vec![],
            files,
            pending,
        });
        lock
    }

    fn put(root: &Path, rel: &str, bytes: &[u8]) {
        let full = root.join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, bytes).unwrap();
    }

    /// The failed run wrote the resource R (only in `pending`); the source
    /// dropped R before the next run.
    #[test]
    fn an_owned_pending_path_that_is_no_longer_planned_is_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), A, b"a1");
        put(tmp.path(), R, b"r");
        let mut lock = lock_with(vec![owned(A, b"a1")], vec![owned(A, b"a2"), owned(R, b"r")]);
        let plan = [planned(&[(A, b"a3")])];
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert!(outcome.skipped.is_empty(), "{:?}", outcome.skipped);
        assert!(!tmp.path().join(R).exists());
        assert!(!tmp.path().join(".claude/skills/s/scripts").exists());
        assert_eq!(lock.items[0].files, vec![owned(A, b"a3")]);
    }

    #[test]
    fn a_modified_pending_path_that_is_no_longer_planned_is_kept_and_recorded() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), A, b"a1");
        put(tmp.path(), R, b"edited");
        let mut lock = lock_with(vec![owned(A, b"a1")], vec![owned(R, b"r")]);
        let plan = [planned(&[(A, b"a3")])];
        let outcome = apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert_eq!(outcome.kept, vec![PathBuf::from(R)]);
        assert_eq!(fs::read(tmp.path().join(R)).unwrap(), b"edited");
        assert_eq!(lock.items[0].files, vec![owned(A, b"a3"), owned(R, b"r")]);
    }

    #[test]
    fn a_pending_path_that_another_item_plans_is_not_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let shared = ".claude/skills/x/SKILL.md";
        put(tmp.path(), shared, b"r");
        let mut lock = lock_with(vec![], vec![owned(shared, b"r")]);
        let mut other = planned(&[(shared, b"r")]);
        other.kind = ItemKind::Agent;
        let plan = [other, planned(&[(A, b"a")])];
        apply_plan(tmp.path(), &mut lock, &plan, false, None, false).unwrap();
        assert_eq!(fs::read(tmp.path().join(shared)).unwrap(), b"r");
    }
}

#[cfg(test)]
mod disposition_tests {
    use super::*;
    use crate::generate::Client;
    use crate::pipeline::plan::PlannedFile;

    const ENTRY: &str = ".claude/skills/s/SKILL.md";
    const NOTES: &str = ".claude/skills/s/notes.md";

    fn planned(files: &[(&str, &[u8])]) -> PlannedItem {
        PlannedItem {
            kind: ItemKind::Skill,
            name: "s".to_string(),
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

    fn entry(files: Vec<OwnedFile>) -> LockedItem {
        LockedItem {
            kind: ItemKind::Skill,
            name: "s".to_string(),
            source: "./src".to_string(),
            git_ref: None,
            hash: Some("h".to_string()),
            source_name: None,
            required_by: vec![],
            group: None,
            clients: vec![],
            files,
            pending: vec![],
        }
    }

    fn put(root: &Path, rel: &str, bytes: &[u8]) {
        let full = root.join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, bytes).unwrap();
    }

    /// The case where every condition of `KeepEdits` holds: `SKILL.md` is
    /// modified, `notes.md` is unchanged, and the source hash is unchanged.
    struct Case {
        tmp: tempfile::TempDir,
        lock: Lockfile,
        plan: Vec<PlannedItem>,
        new: Vec<LockedItem>,
    }

    fn case() -> Case {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), ENTRY, b"my edit");
        put(tmp.path(), NOTES, b"notes");
        let mut lock = Lockfile::new();
        lock.items
            .push(entry(vec![owned(ENTRY, b"gen"), owned(NOTES, b"notes")]));
        let new = lock.items.clone();
        let plan = vec![planned(&[(ENTRY, b"gen"), (NOTES, b"notes")])];
        Case {
            tmp,
            lock,
            plan,
            new,
        }
    }

    fn decide(case: &Case, overwrite: bool, new: Option<&[LockedItem]>, keep: bool) -> Disposition {
        let decision = decide_plan(
            case.tmp.path(),
            &case.lock,
            &case.plan,
            overwrite,
            new,
            keep,
        )
        .unwrap();
        decision.items[0].disposition
    }

    fn decide_default(case: &Case) -> Disposition {
        decide(case, false, Some(&case.new), true)
    }

    #[test]
    fn keeps_the_edits_when_every_condition_holds() {
        let case = case();
        assert_eq!(decide_default(&case), Disposition::KeepEdits);
    }

    #[test]
    fn skips_without_keep_unwithheld_edits() {
        let case = case();
        assert_eq!(
            decide(&case, false, Some(&case.new), false),
            Disposition::Skip
        );
    }

    #[test]
    fn writes_with_overwrite() {
        let case = case();
        assert_eq!(
            decide(&case, true, Some(&case.new), true),
            Disposition::Write
        );
    }

    #[test]
    fn writes_without_a_conflict() {
        let case = case();
        put(case.tmp.path(), ENTRY, b"gen");
        assert_eq!(decide_default(&case), Disposition::Write);
    }

    #[test]
    fn skips_with_an_unowned_conflict() {
        let mut case = case();
        let extra = ".claude/skills/s/extra.md";
        put(case.tmp.path(), extra, b"mine");
        case.plan = vec![planned(&[
            (ENTRY, b"gen"),
            (NOTES, b"notes"),
            (extra, b"generated"),
        ])];
        assert_eq!(decide_default(&case), Disposition::Skip);
    }

    #[test]
    fn skips_when_a_conflict_is_a_kept_stale_file() {
        // `ref` was a recorded file and is modified; the plan now writes
        // `ref/x.md`, so the kept stale file `ref` is a conflict.
        let mut case = case();
        let stale = ".claude/skills/s/ref";
        put(case.tmp.path(), stale, b"my ref");
        case.lock.items[0].files.push(owned(stale, b"ref"));
        case.new = case.lock.items.clone();
        put(case.tmp.path(), ENTRY, b"gen");
        case.plan = vec![planned(&[
            (ENTRY, b"gen"),
            (NOTES, b"notes"),
            (".claude/skills/s/ref/x.md", b"x"),
        ])];
        let decision = decide_plan(
            case.tmp.path(),
            &case.lock,
            &case.plan,
            false,
            Some(&case.new),
            true,
        )
        .unwrap();
        assert_eq!(decision.items[0].conflicts.len(), 1);
        assert_eq!(decision.items[0].disposition, Disposition::Skip);
    }

    #[test]
    fn skips_with_a_pending_record() {
        let mut case = case();
        case.lock.items[0].pending = vec![owned(ENTRY, b"other")];
        assert_eq!(decide_default(&case), Disposition::Skip);
    }

    #[test]
    fn skips_without_a_lockfile_entry() {
        let mut case = case();
        case.lock = Lockfile::new();
        assert_eq!(decide_default(&case), Disposition::Skip);
    }

    #[test]
    fn skips_without_new_entries() {
        let case = case();
        assert_eq!(decide(&case, false, None, true), Disposition::Skip);
    }

    #[test]
    fn skips_when_new_entries_do_not_hold_the_item() {
        let case = case();
        assert_eq!(decide(&case, false, Some(&[]), true), Disposition::Skip);
    }

    #[test]
    fn skips_when_the_source_hash_changed() {
        let mut case = case();
        case.new[0].hash = Some("changed".to_string());
        assert_eq!(decide_default(&case), Disposition::Skip);
    }

    #[test]
    fn skips_when_no_source_hash_is_recorded() {
        let mut case = case();
        case.lock.items[0].hash = None;
        case.new[0].hash = None;
        assert_eq!(decide_default(&case), Disposition::Skip);
    }

    #[test]
    fn skips_when_a_stale_file_on_disk_would_be_deleted() {
        let mut case = case();
        let stale = ".claude/skills/s/old.md";
        put(case.tmp.path(), stale, b"old");
        case.lock.items[0].files.push(owned(stale, b"old"));
        assert_eq!(decide_default(&case), Disposition::Skip);
    }

    #[test]
    fn skips_when_the_stale_file_to_delete_is_absent() {
        let mut case = case();
        case.lock.items[0]
            .files
            .push(owned(".claude/skills/s/old.md", b"old"));
        case.new = case.lock.items.clone();
        assert_eq!(decide_default(&case), Disposition::Skip);
    }

    #[test]
    fn skips_with_a_kept_modified_stale_file_that_is_not_a_conflict() {
        let mut case = case();
        let stale = ".claude/skills/s/old.md";
        put(case.tmp.path(), stale, b"my old");
        case.lock.items[0].files.push(owned(stale, b"old"));
        case.new = case.lock.items.clone();
        let decision = decide_plan(
            case.tmp.path(),
            &case.lock,
            &case.plan,
            false,
            Some(&case.new),
            true,
        )
        .unwrap();
        assert_eq!(decision.items[0].kept.len(), 1);
        assert!(
            decision.items[0]
                .conflicts
                .iter()
                .all(|c| c.path != Path::new(stale))
        );
        assert_eq!(decision.items[0].disposition, Disposition::Skip);
    }

    #[test]
    fn keeps_the_edits_when_the_new_entry_differs_only_in_required_by() {
        let mut case = case();
        case.new[0].required_by = vec!["agent:other".to_string()];
        assert_eq!(decide_default(&case), Disposition::KeepEdits);
    }

    #[test]
    fn skips_when_the_new_entry_differs_only_in_clients() {
        let mut case = case();
        case.new[0].clients = vec!["claude".to_string()];
        assert_eq!(decide_default(&case), Disposition::Skip);
    }

    #[test]
    fn skips_when_another_planned_file_would_be_written() {
        let case = case();
        fs::remove_file(case.tmp.path().join(NOTES)).unwrap();
        assert_eq!(decide_default(&case), Disposition::Skip);
    }

    #[test]
    fn a_kept_item_is_not_written_and_its_entry_is_unchanged() {
        let mut case = case();
        let before = case.lock.items.clone();
        let outcome = apply_plan(
            case.tmp.path(),
            &mut case.lock,
            &case.plan,
            false,
            Some(&case.new.clone()),
            true,
        )
        .unwrap();
        assert!(outcome.skipped.is_empty());
        assert_eq!(outcome.kept_edits.len(), 1);
        assert_eq!(outcome.kept_edits[0].path, PathBuf::from(ENTRY));
        assert!(outcome.recorded.is_empty());
        assert!(outcome.changed.is_empty());
        assert_eq!(case.lock.items, before);
        assert_eq!(fs::read(case.tmp.path().join(ENTRY)).unwrap(), b"my edit");
    }

    #[test]
    fn an_item_planned_after_a_kept_item_is_still_written() {
        let mut case = case();
        let other = ".claude/agents/t.md";
        let mut second = planned(&[(other, b"agent")]);
        second.kind = ItemKind::Agent;
        second.name = "t".to_string();
        case.plan.push(second);
        // `t` is a new item: only step 2 of `execute_plan` adds its entry
        // to the lockfile, with a `pending` record, before the write.
        let mut agent = entry(vec![]);
        agent.kind = ItemKind::Agent;
        agent.name = "t".to_string();
        case.new.push(agent);
        let kept_before = case.lock.items[0].clone();
        let outcome = apply_plan(
            case.tmp.path(),
            &mut case.lock,
            &case.plan,
            false,
            Some(&case.new.clone()),
            true,
        )
        .unwrap();
        assert!(outcome.skipped.is_empty());
        assert_eq!(outcome.kept_edits.len(), 1);
        assert_eq!(outcome.kept_edits[0].path, PathBuf::from(ENTRY));
        let kept_after = case
            .lock
            .items
            .iter()
            .find(|i| i.kind == ItemKind::Skill && i.name == "s")
            .unwrap();
        assert_eq!(*kept_after, kept_before);
        let written = case
            .lock
            .items
            .iter()
            .find(|i| i.kind == ItemKind::Agent && i.name == "t")
            .expect("the lockfile holds an entry for the agent t");
        assert_eq!(written.files, vec![owned(other, b"agent")]);
        assert!(written.pending.is_empty());
        assert_eq!(fs::read(case.tmp.path().join(ENTRY)).unwrap(), b"my edit");
        assert_eq!(fs::read(case.tmp.path().join(other)).unwrap(), b"agent");
        assert_eq!(outcome.changed, vec![PathBuf::from(other)]);
        assert_eq!(outcome.recorded.len(), 1);
        assert_eq!(
            outcome.recorded[&(ItemKind::Agent, "t".to_string())],
            vec![owned(other, b"agent")]
        );
    }
}
