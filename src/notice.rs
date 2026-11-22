//! The session notice: when to print one line about available updates.
//!
//! The cache file records, for each scope root, the time of the last check
//! (a check that reached no source also sets it), the updates that check
//! found, and the set that the last printed notice reported. Time-dependent
//! functions take the time as an argument. Cache loading and saving, run lock
//! acquisition, run lock cleanup, and [`mark_current_in_file`] perform file I/O.
//!
//! Runs sharing a cache directory share a run lock. A run that finds a lock
//! younger than 60 seconds exits without printing a notice or changing the
//! cache, also with `--repeat` and when it handles a different repository. It can
//! print its notice at a later session start. Locks at least 60 seconds old
//! or dated after the current time are replaced. When the lock cannot be
//! acquired for another reason, a run continues without it. A completed run
//! removes its lock only when its ownership token still matches; an interrupted
//! run leaves the file until it expires. Checking the token and removing the
//! file are separate operations, so simultaneous replacements can still race.
//!
//! Before it saves, a notice run
//! loads the file again and applies only its own roots with [`merge_roots`],
//! which leaves the entries of the other run in place. For a root that the run
//! did not look up, the saved `reported` is the part of the reloaded `updates`
//! that either run had reported. An `update` saves
//! through [`mark_current_in_file`], which loads and saves without a lookup
//! between them. Applying `update` does not take the notice run lock. These
//! cases remain with an applying `update`, or between notice runs that continue
//! without a lock:
//!
//! - A run saves between the second load and the rename of another run. Its
//!   save is lost. A lost check of a notice run is made again at the next
//!   session start, because the root is still due. A lost removal by `update`
//!   is not: the root is not due, so the removed entries stay until the next
//!   check in which every source of the scope answers.
//! - A notice run looks up a root while an `update` of that root saves. The
//!   notice run writes its own state of the root, so the removed entries come
//!   back until the next check in which every source of the scope answers.
//!
//! A check that reaches only some sources keeps every stored entry that it did
//! not find (see [`record_check`]), so it does not remove such an entry.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::pipeline::{ScopedCheck, SourceStatus, UpdateReport, UpdateStatus};
use crate::scope::Scope;

/// The shortest time in seconds between two checks that contact the sources.
pub const NETWORK_PERIOD_SECS: u64 = 24 * 60 * 60;
/// The name of the cache file in the upskill cache directory.
pub const CACHE_FILE_NAME: &str = "notice.json";

/// The name of the run lock file in the upskill cache directory.
pub const LOCK_FILE_NAME: &str = "notice.lock";
/// The age in seconds at which a run lock can be replaced.
pub const LOCK_MAX_AGE_SECS: u64 = 60;

/// Removes the lock file on drop if its ownership token still matches.
pub struct RunLock {
    path: PathBuf,
    token: Vec<u8>,
}

impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = remove_lock_if_matches(&self.path, &self.token);
    }
}

fn remove_lock_if_matches(path: &Path, token: &[u8]) -> std::io::Result<bool> {
    if fs::read(path)? != token {
        return Ok(false);
    }
    fs::remove_file(path)?;
    Ok(true)
}

fn create_run_lock(path: &Path) -> std::io::Result<RunLock> {
    #[cfg(test)]
    tests::LOCK_CREATION_ATTEMPTS.with(|attempts| attempts.set(attempts.get() + 1));
    static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let token = format!(
        "{}:{timestamp}:{}",
        std::process::id(),
        NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
    )
    .into_bytes();
    // A failed write can leave a partial token, which expires like any lock.
    file.write_all(&token)?;
    Ok(RunLock {
        path: path.into(),
        token,
    })
}

pub enum LockAttempt {
    /// This run created the lock file.
    Held(RunLock),
    /// Another run holds a lock younger than `LOCK_MAX_AGE_SECS`, the
    /// observed ownership token changed before removal, or another run
    /// created a lock between the removal of a stale lock and the retry.
    Busy,
    /// The lock file cannot be created for another reason.
    Unavailable,
}

/// Create a run lock, replacing an expired or future-dated lock once.
pub fn acquire_run_lock(path: &Path, now: std::time::SystemTime) -> LockAttempt {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    if fs::create_dir_all(dir).is_err() {
        return LockAttempt::Unavailable;
    }
    match create_run_lock(path) {
        Ok(lock) => return LockAttempt::Held(lock),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return LockAttempt::Unavailable,
    }
    // Read the token and timestamp from the same file, even if the path changes.
    let observed = (|| -> std::io::Result<_> {
        let mut file = fs::File::open(path)?;
        let modified = file.metadata()?.modified()?;
        let mut token = Vec::new();
        file.read_to_end(&mut token)?;
        Ok((modified, token))
    })();
    let (modified, token) = match observed {
        Ok(observed) => observed,
        Err(_) => return LockAttempt::Unavailable,
    };
    if now
        .duration_since(modified)
        .is_ok_and(|age| age < std::time::Duration::from_secs(LOCK_MAX_AGE_SECS))
    {
        return LockAttempt::Busy;
    }
    #[cfg(test)]
    tests::BEFORE_STALE_LOCK_REMOVAL.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook(path);
        }
    });
    match remove_lock_if_matches(path, &token) {
        Ok(true) => {}
        Ok(false) => return LockAttempt::Busy,
        Err(_) => return LockAttempt::Unavailable,
    }
    #[cfg(test)]
    tests::AFTER_STALE_LOCK_REMOVAL.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook(path);
        }
    });
    match create_run_lock(path) {
        Ok(lock) => LockAttempt::Held(lock),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => LockAttempt::Busy,
        Err(_) => LockAttempt::Unavailable,
    }
}

/// State of one scope root in the cache file. A missing field of a file reads
/// as its default value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScopeState {
    /// Unix time in seconds of the last check. A check that reached no source
    /// also sets it, so the next attempt waits for the period.
    pub checked_at: u64,
    /// The available updates found by the checks, sorted. A check that
    /// reached only some of the sources of the scope keeps every earlier entry
    /// whose `<kind>:<name>` is not in the set that it found (see
    /// [`record_check`]).
    pub updates: Vec<String>,
    /// The set that the last printed notice reported, sorted.
    pub reported: Vec<String>,
}

/// The cache file: scope root path to state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NoticeCache {
    pub scopes: BTreeMap<String, ScopeState>,
}

impl NoticeCache {
    /// An absent, unreadable or invalid file is an empty cache.
    pub fn load(path: &Path) -> NoticeCache {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Write to a temporary file in the same directory, then rename. Creates
    /// the directory when it is absent.
    pub fn save(&self, path: &Path) -> Result<()> {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
        let mut tmp_name = path
            .file_name()
            .map(|n| n.to_os_string())
            .unwrap_or_else(|| CACHE_FILE_NAME.into());
        tmp_name.push(format!(".{}.tmp", std::process::id()));
        let tmp = dir.join(tmp_name);
        let text = serde_json::to_string_pretty(self).context("failed to serialize the cache")?;
        fs::write(&tmp, text).with_context(|| {
            let _ = fs::remove_file(&tmp);
            format!("failed to write {}", tmp.display())
        })?;
        fs::rename(&tmp, path).with_context(|| {
            let _ = fs::remove_file(&tmp);
            format!("failed to replace {}", path.display())
        })
    }
}

fn key(root: &Path) -> String {
    root.display().to_string()
}

/// True when the state of `root` is older than the period, absent, or dated
/// after `now` (the clock moved back).
pub fn needs_check(cache: &NoticeCache, root: &Path, now: u64) -> bool {
    match cache.scopes.get(&key(root)) {
        None => true,
        Some(state) => state.checked_at > now || now - state.checked_at >= NETWORK_PERIOD_SECS,
    }
}

/// The identity of each available update of a scope, sorted:
/// `<kind>:<name>@<to>` for a git source and `<kind>:<name>` for a local path
/// source. Only sources with status `UpdateAvailable` contribute.
pub fn update_set(check: &ScopedCheck) -> Vec<String> {
    let mut set: Vec<String> = check
        .sources
        .iter()
        .filter_map(|s| match &s.status {
            SourceStatus::UpdateAvailable { to, .. } => Some((s, to)),
            _ => None,
        })
        .flat_map(|(s, to)| {
            s.items.iter().map(move |(kind, name)| match to {
                Some(to) => format!("{kind}:{name}@{to}"),
                None => format!("{kind}:{name}"),
            })
        })
        .collect();
    set.sort();
    set.dedup();
    set
}

/// How many sources of a scope answered a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reached {
    /// The scope has at least one source and every source is `NotReachable`
    /// or `NotChecked`.
    Nothing,
    /// At least one source answered and at least one is `NotReachable` or
    /// `NotChecked`.
    Partly,
    /// Every source answered. A scope without sources counts as fully
    /// reached.
    Fully,
}

/// How many sources of the scope answered `check`.
pub fn reached(check: &ScopedCheck) -> Reached {
    let missing = |s: &&crate::pipeline::CheckedSource| {
        matches!(
            s.status,
            SourceStatus::NotReachable { .. } | SourceStatus::NotChecked
        )
    };
    let missing_count = check.sources.iter().filter(missing).count();
    if missing_count == 0 {
        Reached::Fully
    } else if missing_count == check.sources.len() {
        Reached::Nothing
    } else {
        Reached::Partly
    }
}

/// The `<kind>:<name>` part of an entry of an update set.
fn item_of(entry: &str) -> &str {
    entry.split_once('@').map_or(entry, |(item, _)| item)
}

/// Store the result of a check: `checked_at = now` and the updates.
///
/// - [`Reached::Fully`]: `updates` replaces the stored set.
/// - [`Reached::Partly`]: `updates` is added, and each stored entry whose
///   `<kind>:<name>` part is not in `updates` stays. The check cannot tell
///   which source an earlier entry came from, so it keeps every such entry,
///   also one of a source that answered.
/// - [`Reached::Nothing`]: the stored set stays.
pub fn record_check(
    cache: &mut NoticeCache,
    root: &Path,
    updates: Vec<String>,
    reached: Reached,
    now: u64,
) {
    let state = cache.scopes.entry(key(root)).or_default();
    state.checked_at = now;
    let mut merged = match reached {
        Reached::Nothing => return,
        Reached::Fully => Vec::new(),
        Reached::Partly => state
            .updates
            .iter()
            .filter(|old| !updates.iter().any(|new| item_of(new) == item_of(old)))
            .cloned()
            .collect(),
    };
    merged.extend(updates);
    merged.sort();
    merged.dedup();
    state.updates = merged;
}

/// Remove the state of `root`.
pub fn forget(cache: &mut NoticeCache, root: &Path) {
    cache.scopes.remove(&key(root));
}

/// Apply the state of each of `roots` in `mine` to `fresh`, and return the
/// result. A root that `mine` does not hold is removed from `fresh`. The states
/// of all other roots stay as `fresh` holds them, so that a run that overlapped
/// another run does not drop the entries of the other run.
///
/// A root of `roots` that is not in `checked` was not looked up by this run,
/// so this run changed only its `reported`. When `fresh` holds that root, its
/// `checked_at` and `updates` stay as `fresh` holds them, and `reported`
/// becomes the entries of the `updates` of `fresh` that the `reported` of
/// `fresh` or of `mine` holds (in the order of `updates`, so sorted and
/// without duplicates). An entry that is no longer in `updates` is dropped:
/// a removal by [`mark_current_in_file`] stays removed, and an entry of an
/// older set cannot keep `reported` different from `updates`. When `fresh`
/// does not hold that root, the state of `mine` is inserted.
pub fn merge_roots(
    mut fresh: NoticeCache,
    mine: &NoticeCache,
    roots: &[PathBuf],
    checked: &[PathBuf],
) -> NoticeCache {
    for root in roots {
        match mine.scopes.get(&key(root)) {
            Some(state) => {
                let only_reported = !checked.contains(root);
                match fresh.scopes.get_mut(&key(root)) {
                    Some(current) if only_reported => {
                        current.reported = current
                            .updates
                            .iter()
                            .filter(|entry| {
                                current.reported.contains(entry) || state.reported.contains(entry)
                            })
                            .cloned()
                            .collect();
                    }
                    _ => {
                        fresh.scopes.insert(key(root), state.clone());
                    }
                }
            }
            None => forget(&mut fresh, root),
        }
    }
    fresh
}

/// The `<kind>:<name>` of each item that an applied `update` left at the
/// version of its source: the statuses `Updated`, `UpToDate`,
/// `CompletedInterrupted` and `Removed`. An item that was skipped, not
/// applied, or whose source was skipped is not current.
pub fn current_items(report: &UpdateReport) -> Vec<String> {
    report
        .items
        .iter()
        .filter(|item| {
            matches!(
                item.status,
                UpdateStatus::UpToDate
                    | UpdateStatus::Updated { .. }
                    | UpdateStatus::CompletedInterrupted
                    | UpdateStatus::Removed
            )
        })
        .map(|item| format!("{}:{}", item.kind, item.name))
        .collect()
}

/// Load the cache file at `path`, apply [`mark_current`] for `root`, and save
/// the file at once when it changed. The load and the save follow one another
/// without a lookup between them. A failed read is an empty cache and a failed
/// write is ignored: the caller is an `update` whose result does not depend on
/// the notice.
pub fn mark_current_in_file(path: &Path, root: &Path, current: &[String]) {
    if current.is_empty() {
        return;
    }
    let mut cache = NoticeCache::load(path);
    if mark_current(&mut cache, root, current) {
        let _ = cache.save(path);
    }
}

/// Remove each state whose root path `has_lockfile` rejects.
pub fn prune(cache: &mut NoticeCache, has_lockfile: impl Fn(&Path) -> bool) {
    cache.scopes.retain(|root, _| has_lockfile(Path::new(root)));
}

/// Remove from `updates` and `reported` of `root` each entry whose
/// `<kind>:<name>` part is in `current`, the items that an applied `update`
/// left at the version of their source. Returns true when the cache changed.
pub fn mark_current(cache: &mut NoticeCache, root: &Path, current: &[String]) -> bool {
    let Some(state) = cache.scopes.get_mut(&key(root)) else {
        return false;
    };
    let before = (state.updates.len(), state.reported.len());
    let keep = |entry: &String| !current.iter().any(|item| item == item_of(entry));
    state.updates.retain(keep);
    state.reported.retain(keep);
    before != (state.updates.len(), state.reported.len())
}

/// The line to print for `root`, and the cache change that goes with it.
/// Returns `None` when `updates` is empty (and clears `reported`), when
/// `updates` equals `reported` and `repeat` is false, and when `root` has no
/// state. Otherwise sets `reported = updates` and returns the line.
pub fn decide(cache: &mut NoticeCache, scope: Scope, root: &Path, repeat: bool) -> Option<String> {
    let state = cache.scopes.get_mut(&key(root))?;
    if state.updates.is_empty() {
        state.reported.clear();
        return None;
    }
    if state.updates == state.reported && !repeat {
        return None;
    }
    state.reported = state.updates.clone();
    let count = state.updates.len();
    let noun = if count == 1 { "update" } else { "updates" };
    let command = match scope {
        Scope::Project => "upskill update",
        Scope::Global => "upskill update --global",
    };
    Some(format!(
        "upskill: {count} {noun} available ({scope}). Run \"{command}\"."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{CheckedSource, ItemKind};
    use std::path::PathBuf;

    use std::time::{Duration, SystemTime};

    type LockRemovalHook = Box<dyn FnOnce(&Path)>;

    thread_local! {
        pub(super) static BEFORE_STALE_LOCK_REMOVAL: std::cell::RefCell<Option<LockRemovalHook>> =
            const { std::cell::RefCell::new(None) };
        pub(super) static AFTER_STALE_LOCK_REMOVAL: std::cell::RefCell<Option<LockRemovalHook>> =
            const { std::cell::RefCell::new(None) };
        pub(super) static LOCK_CREATION_ATTEMPTS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    #[test]
    fn acquiring_a_stale_lock_stops_when_the_observed_token_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "expired owner").unwrap();
        let now = SystemTime::now();
        set_age(&path, now, 60);
        BEFORE_STALE_LOCK_REMOVAL.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(|path| {
                fs::remove_file(path).unwrap();
                fs::write(path, "replacement owner").unwrap();
            }));
        });
        let attempts_before = LOCK_CREATION_ATTEMPTS.with(|attempts| attempts.get());

        let attempt = acquire_run_lock(&path, now);

        assert!(
            matches!(attempt, LockAttempt::Busy),
            "a changed token must be busy"
        );
        assert_eq!(fs::read(&path).unwrap(), b"replacement owner");
        LOCK_CREATION_ATTEMPTS.with(|attempts| {
            assert_eq!(
                attempts.get() - attempts_before,
                1,
                "a changed token must stop before another creation attempt"
            );
        });
    }

    #[test]
    fn acquiring_a_stale_lock_is_unavailable_when_the_removal_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "expired owner").unwrap();
        let now = SystemTime::now();
        set_age(&path, now, 60);
        // The lock disappears after it was read, so the token check cannot
        // read it again.
        BEFORE_STALE_LOCK_REMOVAL.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(|path| fs::remove_file(path).unwrap()));
        });

        let attempt = acquire_run_lock(&path, now);

        assert!(
            matches!(attempt, LockAttempt::Unavailable),
            "a failed removal must be unavailable"
        );
    }

    #[test]
    fn acquiring_a_stale_lock_is_busy_when_another_run_creates_the_lock_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "expired owner").unwrap();
        let now = SystemTime::now();
        set_age(&path, now, 60);
        AFTER_STALE_LOCK_REMOVAL.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(|path| fs::write(path, "other run").unwrap()));
        });

        let attempt = acquire_run_lock(&path, now);

        assert!(
            matches!(attempt, LockAttempt::Busy),
            "a lock created before the retry must be busy"
        );
        assert_eq!(fs::read(&path).unwrap(), b"other run");
    }

    fn lock_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("cache").join(LOCK_FILE_NAME)
    }

    fn set_age(path: &Path, now: SystemTime, secs: u64) {
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(now - Duration::from_secs(secs))
            .unwrap();
    }

    #[test]
    fn the_first_run_holds_the_lock_and_creates_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let attempt = acquire_run_lock(&path, SystemTime::now());
        assert!(matches!(attempt, LockAttempt::Held(_)));
        assert!(path.is_file());
    }

    #[test]
    fn dropping_the_lock_deletes_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let LockAttempt::Held(lock) = acquire_run_lock(&path, SystemTime::now()) else {
            panic!("expected a held lock");
        };
        drop(lock);
        assert!(!path.exists());
    }

    #[test]
    fn dropping_a_replaced_run_lock_preserves_the_new_lock() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        let now = SystemTime::now();
        let LockAttempt::Held(first) = acquire_run_lock(&path, now) else {
            panic!("expected the first run to hold the lock");
        };
        set_age(&path, now, 60);
        let LockAttempt::Held(second) = acquire_run_lock(&path, now) else {
            panic!("expected the second run to replace the expired lock");
        };

        drop(first);
        assert!(
            path.is_file(),
            "the first run deleted the second run's lock"
        );
        assert!(matches!(
            acquire_run_lock(&path, now + Duration::from_secs(1)),
            LockAttempt::Busy
        ));

        set_age(&path, now, 60);
        let LockAttempt::Held(third) = acquire_run_lock(&path, now) else {
            panic!("expected the third run to replace the expired lock");
        };
        drop(second);
        assert!(
            path.is_file(),
            "the second run deleted the third run's lock"
        );
        drop(third);
        assert!(!path.exists(), "the third run did not remove its own lock");
    }

    #[test]
    fn a_stale_lock_snapshot_cannot_remove_a_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "expired owner").unwrap();
        let now = SystemTime::now();
        set_age(&path, now, 60);
        let observed_token = fs::read(&path).unwrap();

        let LockAttempt::Held(replacement) = acquire_run_lock(&path, now) else {
            panic!("expected a replacement lock");
        };
        assert!(!remove_lock_if_matches(&path, &observed_token).unwrap());
        assert!(matches!(
            acquire_run_lock(&path, now + Duration::from_secs(1)),
            LockAttempt::Busy
        ));
        drop(replacement);
        assert!(!path.exists());
    }

    #[test]
    fn a_lock_younger_than_60_seconds_is_busy() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();
        let now = SystemTime::now();
        set_age(&path, now, 59);
        assert!(matches!(acquire_run_lock(&path, now), LockAttempt::Busy));
        assert!(path.is_file());
    }

    #[test]
    fn a_lock_of_60_seconds_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();
        let now = SystemTime::now();
        set_age(&path, now, 60);
        let attempt = acquire_run_lock(&path, now);
        assert!(matches!(attempt, LockAttempt::Held(_)));
        assert!(path.is_file());
    }

    #[test]
    fn a_lock_dated_after_now_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let now = SystemTime::now();
        fs::File::create(&path)
            .unwrap()
            .set_modified(now + Duration::from_secs(3600))
            .unwrap();
        let attempt = acquire_run_lock(&path, now);
        assert!(matches!(attempt, LockAttempt::Held(_)));
        assert!(path.is_file());
    }

    #[test]
    fn a_directory_that_cannot_be_created_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let path = lock_path(&dir);
        fs::write(path.parent().unwrap(), "").unwrap();
        assert!(matches!(
            acquire_run_lock(&path, SystemTime::now()),
            LockAttempt::Unavailable
        ));
    }

    fn root() -> PathBuf {
        PathBuf::from("/work/project")
    }

    fn set(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn source(status: SourceStatus, items: Vec<(ItemKind, &str)>) -> CheckedSource {
        CheckedSource {
            source: "owner/repo".into(),
            status,
            items: items.into_iter().map(|(k, n)| (k, n.to_string())).collect(),
        }
    }

    fn update(to: Option<&str>) -> SourceStatus {
        SourceStatus::UpdateAvailable {
            from: None,
            to: to.map(String::from),
        }
    }

    fn check(sources: Vec<CheckedSource>) -> ScopedCheck {
        ScopedCheck {
            scope: Scope::Project,
            root: root(),
            sources,
        }
    }

    fn recorded(updates: &[&str]) -> NoticeCache {
        let mut cache = NoticeCache::default();
        record_check(&mut cache, &root(), set(updates), Reached::Fully, 1_000);
        cache
    }

    #[test]
    fn an_absent_cache_needs_a_check() {
        assert!(needs_check(&NoticeCache::default(), &root(), 5));
    }

    #[test]
    fn a_state_younger_than_the_period_needs_no_check() {
        let cache = recorded(&[]);
        assert!(!needs_check(&cache, &root(), 1_000 + 86_399));
        assert!(needs_check(&cache, &root(), 1_000 + 86_400));
    }

    #[test]
    fn a_new_set_prints_once() {
        let mut cache = recorded(&["skill:a@c1", "rule:b@c1", "agent:c@c1"]);
        assert_eq!(
            decide(&mut cache, Scope::Project, &root(), false).as_deref(),
            Some("upskill: 3 updates available (project). Run \"upskill update\".")
        );
        assert_eq!(decide(&mut cache, Scope::Project, &root(), false), None);
    }

    #[test]
    fn a_reported_set_prints_again_with_repeat() {
        let mut cache = recorded(&["skill:a@c1"]);
        assert!(decide(&mut cache, Scope::Project, &root(), true).is_some());
        assert!(decide(&mut cache, Scope::Project, &root(), true).is_some());
        assert_eq!(cache.scopes[&key(&root())].reported, set(&["skill:a@c1"]));
        assert_eq!(decide(&mut cache, Scope::Project, &root(), false), None);
    }

    #[test]
    fn mark_current_removes_the_named_items_from_both_sets() {
        let mut cache = recorded(&["agent:local", "skill:a@c1", "skill:b@c1"]);
        decide(&mut cache, Scope::Project, &root(), false);
        let current = set(&["skill:a", "agent:local", "rule:a"]);
        assert!(mark_current(&mut cache, &root(), &current));
        let state = &cache.scopes[&key(&root())];
        assert_eq!(state.updates, set(&["skill:b@c1"]));
        assert_eq!(state.reported, set(&["skill:b@c1"]));
        assert!(!mark_current(&mut cache, &root(), &current));
        assert!(!mark_current(&mut cache, &other_root(), &current));
    }

    #[test]
    fn a_changed_set_prints_again() {
        let mut cache = recorded(&["skill:a@c1"]);
        assert!(decide(&mut cache, Scope::Project, &root(), false).is_some());
        record_check(
            &mut cache,
            &root(),
            set(&["skill:a@c2"]),
            Reached::Fully,
            2_000,
        );
        assert!(decide(&mut cache, Scope::Project, &root(), false).is_some());
        assert_eq!(decide(&mut cache, Scope::Project, &root(), false), None);
    }

    #[test]
    fn an_empty_set_prints_nothing_and_clears_the_reported_set() {
        let mut cache = recorded(&["skill:a@c1"]);
        assert!(decide(&mut cache, Scope::Project, &root(), false).is_some());
        record_check(&mut cache, &root(), vec![], Reached::Fully, 2_000);
        assert_eq!(decide(&mut cache, Scope::Project, &root(), false), None);
        assert!(cache.scopes[&key(&root())].reported.is_empty());
        record_check(
            &mut cache,
            &root(),
            set(&["skill:a@c1"]),
            Reached::Fully,
            3_000,
        );
        assert!(decide(&mut cache, Scope::Project, &root(), false).is_some());
    }

    #[test]
    fn an_equal_set_leaves_the_cache_unchanged() {
        let mut cache = recorded(&["skill:a@c1"]);
        decide(&mut cache, Scope::Project, &root(), false);
        let before = cache.clone();
        assert_eq!(decide(&mut cache, Scope::Project, &root(), false), None);
        assert_eq!(cache, before);
    }

    #[test]
    fn a_check_that_reached_no_source_keeps_the_updates_and_sets_checked_at() {
        let mut cache = recorded(&["skill:a@c1"]);
        record_check(&mut cache, &root(), vec![], Reached::Nothing, 5_000);
        let state = &cache.scopes[&key(&root())];
        assert_eq!(state.updates, set(&["skill:a@c1"]));
        assert_eq!(state.checked_at, 5_000);
    }

    #[test]
    fn reached_tells_whether_no_some_or_every_source_answered() {
        let failed = || {
            source(
                SourceStatus::NotReachable {
                    reason: "offline".into(),
                },
                vec![],
            )
        };
        let skipped = || source(SourceStatus::NotChecked, vec![]);
        let fine = || source(SourceStatus::UpToDate, vec![]);
        assert_eq!(reached(&check(vec![])), Reached::Fully);
        assert_eq!(reached(&check(vec![failed()])), Reached::Nothing);
        assert_eq!(reached(&check(vec![failed(), skipped()])), Reached::Nothing);
        assert_eq!(reached(&check(vec![failed(), fine()])), Reached::Partly);
        assert_eq!(reached(&check(vec![skipped(), fine()])), Reached::Partly);
        assert_eq!(reached(&check(vec![fine(), fine()])), Reached::Fully);
        assert_eq!(
            reached(&check(vec![source(update(Some("c1")), vec![])])),
            Reached::Fully
        );
    }

    #[test]
    fn a_partly_reached_check_keeps_the_earlier_entries_that_it_did_not_find() {
        let mut cache = recorded(&["rule:b@c1", "skill:a@c1", "skill:b@c1"]);
        record_check(
            &mut cache,
            &root(),
            set(&["skill:a@c2", "skill:z@c2"]),
            Reached::Partly,
            2_000,
        );
        let state = &cache.scopes[&key(&root())];
        // `skill:a` is in the new set, so its earlier entry is replaced.
        // `skill:b` has the kind of `skill:a` and another name: it stays.
        assert_eq!(
            state.updates,
            set(&["rule:b@c1", "skill:a@c2", "skill:b@c1", "skill:z@c2"])
        );
        assert_eq!(state.checked_at, 2_000);
    }

    #[test]
    fn a_fully_reached_check_replaces_the_set() {
        let mut cache = recorded(&["rule:b@c1", "skill:a@c1"]);
        record_check(
            &mut cache,
            &root(),
            set(&["skill:a@c2"]),
            Reached::Fully,
            2_000,
        );
        assert_eq!(cache.scopes[&key(&root())].updates, set(&["skill:a@c2"]));
    }

    #[test]
    fn an_entry_without_a_commit_is_matched_by_kind_and_name() {
        let mut cache = recorded(&["agent:local", "agent:other", "skill:local@c1"]);
        record_check(
            &mut cache,
            &root(),
            set(&["agent:local@c2"]),
            Reached::Partly,
            2_000,
        );
        // The new entry replaces `agent:local`; the other name and the other
        // kind with the same name stay.
        assert_eq!(
            cache.scopes[&key(&root())].updates,
            set(&["agent:local@c2", "agent:other", "skill:local@c1"])
        );
    }

    #[test]
    fn a_subset_from_a_partly_reached_check_does_not_print_a_second_line() {
        let full = ["rule:b@c1", "skill:a@c1"];
        let mut cache = recorded(&full);
        assert!(decide(&mut cache, Scope::Project, &root(), false).is_some());
        record_check(
            &mut cache,
            &root(),
            set(&["skill:a@c1"]),
            Reached::Partly,
            2_000,
        );
        assert_eq!(decide(&mut cache, Scope::Project, &root(), false), None);
        record_check(&mut cache, &root(), set(&full), Reached::Fully, 3_000);
        assert_eq!(decide(&mut cache, Scope::Project, &root(), false), None);
    }

    fn other_root() -> PathBuf {
        PathBuf::from("/work/other")
    }

    fn cache_of(entries: &[(&Path, &[&str])]) -> NoticeCache {
        let mut cache = NoticeCache::default();
        for (root, updates) in entries {
            record_check(&mut cache, root, set(updates), Reached::Fully, 1_000);
        }
        cache
    }

    #[test]
    fn merge_roots_applies_only_the_handled_roots_to_the_fresh_cache() {
        let fresh = cache_of(&[
            (&root(), &["skill:old@c0"]),
            (&other_root(), &["skill:o@c9"]),
        ]);
        let mine = cache_of(&[
            (&root(), &["skill:new@c1"]),
            (&other_root(), &["skill:o@c1"]),
        ]);
        let merged = merge_roots(fresh, &mine, &[root()], &[root()]);
        assert_eq!(merged.scopes[&key(&root())].updates, set(&["skill:new@c1"]));
        assert_eq!(
            merged.scopes[&key(&other_root())].updates,
            set(&["skill:o@c9"])
        );
    }

    #[test]
    fn merge_roots_removes_a_handled_root_that_is_absent_from_my_cache() {
        let fresh = cache_of(&[(&root(), &["skill:a@c1"]), (&other_root(), &["skill:o@c1"])]);
        let merged = merge_roots(fresh, &NoticeCache::default(), &[root()], &[]);
        assert!(!merged.scopes.contains_key(&key(&root())));
        assert!(merged.scopes.contains_key(&key(&other_root())));
    }

    #[test]
    fn merge_roots_adds_a_handled_root_that_the_fresh_cache_lacks() {
        let mine = cache_of(&[(&root(), &["skill:a@c1"])]);
        let merged = merge_roots(NoticeCache::default(), &mine, &[root()], &[]);
        assert_eq!(merged, mine);
    }

    #[test]
    fn merge_roots_keeps_the_updates_of_fresh_for_a_root_that_was_not_checked() {
        // `mine` loaded the cache before an `update` removed `skill:a` from
        // the file, and printed both entries without a lookup.
        let mut mine = cache_of(&[(&root(), &["skill:a@c1", "skill:b@c1"])]);
        decide(&mut mine, Scope::Project, &root(), true);
        let mut fresh = cache_of(&[(&root(), &["skill:a@c1", "skill:b@c1"])]);
        mark_current(&mut fresh, &root(), &set(&["skill:a"]));
        let merged = merge_roots(fresh, &mine, &[root()], &[]);
        let state = &merged.scopes[&key(&root())];
        assert_eq!(state.updates, set(&["skill:b@c1"]));
        assert_eq!(state.reported, set(&["skill:b@c1"]));
    }

    #[test]
    fn merge_roots_keeps_the_set_that_another_run_printed_after_a_lookup() {
        // Run A loaded the old set and did not look up the root. Run B looked
        // it up, found a new set, printed it and saved. A then saves.
        let mut mine = cache_of(&[(&root(), &["skill:a@c1"])]);
        decide(&mut mine, Scope::Project, &root(), false);
        let mut fresh = cache_of(&[(&root(), &["skill:a@c2", "skill:b@c2"])]);
        decide(&mut fresh, Scope::Project, &root(), false);
        let mut merged = merge_roots(fresh, &mine, &[root()], &[]);
        assert_eq!(
            merged.scopes[&key(&root())].reported,
            set(&["skill:a@c2", "skill:b@c2"])
        );
        assert_eq!(decide(&mut merged, Scope::Project, &root(), false), None);
    }

    /// `runs` plain notice runs in sequence, each loading the file that the
    /// previous one saved. No run looks up the root, so `checked` is empty in
    /// every run.
    fn plain_runs(mut file: NoticeCache, runs: usize) -> (NoticeCache, usize) {
        let mut printed = 0;
        for _ in 0..runs {
            let mut mine = file.clone();
            if decide(&mut mine, Scope::Project, &root(), false).is_some() {
                printed += 1;
            }
            file = merge_roots(file, &mine, &[root()], &[]);
        }
        (file, printed)
    }

    #[test]
    fn a_plain_run_after_a_quiet_check_of_a_changed_set_prints_once() {
        // A quiet check recorded `skill:a@c2`; `reported` still names `c1`.
        let mut file = recorded(&["skill:a@c1"]);
        decide(&mut file, Scope::Project, &root(), false);
        record_check(
            &mut file,
            &root(),
            set(&["skill:a@c2"]),
            Reached::Fully,
            2_000,
        );
        let (file, printed) = plain_runs(file, 3);
        assert_eq!(printed, 1);
        assert_eq!(file.scopes[&key(&root())].reported, set(&["skill:a@c2"]));
    }

    #[test]
    fn a_run_that_overlapped_a_run_that_printed_a_new_set_causes_no_second_print() {
        let mut mine = recorded(&["skill:a@c1"]);
        decide(&mut mine, Scope::Project, &root(), false);
        let mut fresh = recorded(&["skill:a@c2", "skill:b@c2"]);
        decide(&mut fresh, Scope::Project, &root(), false);
        let merged = merge_roots(fresh, &mine, &[root()], &[]);
        let (_, printed) = plain_runs(merged, 2);
        assert_eq!(printed, 0);
    }

    #[test]
    fn a_run_that_overlapped_an_update_does_not_bring_back_the_removed_items() {
        let mut mine = recorded(&["skill:a@c1", "skill:b@c1"]);
        decide(&mut mine, Scope::Project, &root(), false);
        let mut fresh = mine.clone();
        mark_current(&mut fresh, &root(), &set(&["skill:a"]));
        let merged = merge_roots(fresh, &mine, &[root()], &[]);
        let state = &merged.scopes[&key(&root())];
        assert_eq!(state.updates, set(&["skill:b@c1"]));
        assert_eq!(state.reported, set(&["skill:b@c1"]));
    }

    #[test]
    fn a_quiet_run_on_a_root_that_was_not_checked_leaves_the_set_unreported() {
        // A plain run printed `skill:a@c1`, then a quiet check stored the new
        // set `skill:a@c2`, so `reported` holds the stale entry. A quiet run
        // does not call `decide`, so its `reported` equals the one it loaded.
        let mut file = recorded(&["skill:a@c1"]);
        decide(&mut file, Scope::Project, &root(), false);
        record_check(
            &mut file,
            &root(),
            set(&["skill:a@c2"]),
            Reached::Fully,
            2_000,
        );
        let mine = file.clone();
        let merged = merge_roots(file, &mine, &[root()], &[]);
        assert!(merged.scopes[&key(&root())].reported.is_empty());
        let (_, printed) = plain_runs(merged, 2);
        assert_eq!(printed, 1);
    }

    #[test]
    fn the_merged_reported_set_follows_the_order_of_the_updates() {
        let mut fresh = recorded(&["agent:a", "rule:b@c1", "skill:c@c1"]);
        fresh.scopes.get_mut(&key(&root())).unwrap().reported = set(&["rule:b@c1"]);
        let mut mine = fresh.clone();
        mine.scopes.get_mut(&key(&root())).unwrap().reported = set(&["agent:a", "skill:c@c1"]);
        let merged = merge_roots(fresh, &mine, &[root()], &[]);
        let state = &merged.scopes[&key(&root())];
        assert_eq!(state.reported, state.updates);
    }

    #[test]
    fn three_plain_sessions_without_overlap_print_a_set_once() {
        let (file, printed) = plain_runs(recorded(&["skill:b@c1", "rule:a@c1", "agent:z"]), 3);
        assert_eq!(printed, 1);
        let state = &file.scopes[&key(&root())];
        assert_eq!(state.reported, state.updates);
    }

    #[test]
    fn mark_current_matches_the_kind_and_the_whole_name() {
        let mut cache = recorded(&["rule:a@c1", "skill:a@c1", "skill:ab@c1"]);
        assert!(mark_current(&mut cache, &root(), &set(&["skill:a"])));
        assert_eq!(
            cache.scopes[&key(&root())].updates,
            set(&["rule:a@c1", "skill:ab@c1"])
        );
    }

    #[test]
    fn mark_current_in_file_saves_the_removal_and_ignores_an_absent_root() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CACHE_FILE_NAME);
        recorded(&["skill:a@c1", "skill:b@c1"]).save(&path).unwrap();
        mark_current_in_file(&path, &root(), &set(&["skill:a"]));
        assert_eq!(
            NoticeCache::load(&path).scopes[&key(&root())].updates,
            set(&["skill:b@c1"])
        );
        mark_current_in_file(&path, &other_root(), &set(&["skill:b"]));
        assert_eq!(
            NoticeCache::load(&path).scopes[&key(&root())].updates,
            set(&["skill:b@c1"])
        );
    }

    #[test]
    fn current_items_takes_the_statuses_that_leave_an_item_current() {
        use crate::pipeline::{PreviewAction, SkipCause, UpdatedItem};
        let item = |name: &str, status| UpdatedItem {
            kind: ItemKind::Skill,
            name: name.into(),
            source: "owner/repo".into(),
            status,
        };
        let report = UpdateReport {
            items: vec![
                item("up-to-date", UpdateStatus::UpToDate),
                item(
                    "updated",
                    UpdateStatus::Updated {
                        old_hash: None,
                        new_hash: None,
                    },
                ),
                item("completed", UpdateStatus::CompletedInterrupted),
                UpdatedItem {
                    kind: ItemKind::Agent,
                    name: "removed".into(),
                    source: "owner/repo".into(),
                    status: UpdateStatus::Removed,
                },
                item("skipped", UpdateStatus::SkippedModified { files: vec![] }),
                item(
                    "not-applied",
                    UpdateStatus::NotApplied {
                        action: PreviewAction::Skip,
                    },
                ),
                item(
                    "source-skipped",
                    UpdateStatus::SourceSkipped {
                        cause: SkipCause::Unreachable,
                    },
                ),
            ],
            ..UpdateReport::default()
        };
        assert_eq!(
            current_items(&report),
            set(&[
                "skill:up-to-date",
                "skill:updated",
                "skill:completed",
                "agent:removed"
            ])
        );
    }

    #[test]
    fn forget_removes_the_state_of_a_root() {
        let mut cache = cache_of(&[(&root(), &["skill:a@c1"]), (&other_root(), &[])]);
        forget(&mut cache, &root());
        assert_eq!(cache, cache_of(&[(&other_root(), &[])]));
    }

    #[test]
    fn prune_drops_the_roots_that_the_predicate_rejects() {
        let mut cache = cache_of(&[(&root(), &["skill:a@c1"]), (&other_root(), &[])]);
        prune(&mut cache, |r| r == other_root());
        assert_eq!(cache, cache_of(&[(&other_root(), &[])]));
    }

    #[test]
    fn one_update_uses_the_singular_form() {
        let mut cache = recorded(&["skill:a@c1"]);
        assert_eq!(
            decide(&mut cache, Scope::Project, &root(), false).as_deref(),
            Some("upskill: 1 update available (project). Run \"upskill update\".")
        );
    }

    #[test]
    fn the_global_line_names_the_global_flag() {
        let mut cache = recorded(&["skill:a@c1"]);
        assert_eq!(
            decide(&mut cache, Scope::Global, &root(), false).as_deref(),
            Some("upskill: 1 update available (global). Run \"upskill update --global\".")
        );
    }

    #[test]
    fn an_invalid_cache_file_loads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CACHE_FILE_NAME);
        assert_eq!(NoticeCache::load(&path), NoticeCache::default());
        fs::write(&path, "not json {").unwrap();
        assert_eq!(NoticeCache::load(&path), NoticeCache::default());
    }

    #[test]
    fn save_then_load_returns_the_same_cache() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/cache").join(CACHE_FILE_NAME);
        let mut cache = recorded(&["skill:a@c1", "rule:b"]);
        decide(&mut cache, Scope::Project, &root(), false);
        cache.save(&path).unwrap();
        assert_eq!(NoticeCache::load(&path), cache);
        let names: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 1, "no temporary file stays: {names:?}");
    }

    #[test]
    fn update_set_is_sorted_and_names_the_target_commit() {
        let check = check(vec![
            source(
                update(Some("c9")),
                vec![(ItemKind::Skill, "zeta"), (ItemKind::Rule, "alpha")],
            ),
            source(update(None), vec![(ItemKind::Agent, "local")]),
            source(SourceStatus::UpToDate, vec![(ItemKind::Skill, "ignored")]),
            source(SourceStatus::PinnedToCommit, vec![]),
        ]);
        assert_eq!(
            update_set(&check),
            set(&["agent:local", "rule:alpha@c9", "skill:zeta@c9"])
        );
    }

    #[test]
    fn a_check_time_in_the_future_needs_a_check() {
        let cache = recorded(&[]);
        assert!(needs_check(&cache, &root(), 999));
    }

    #[test]
    fn a_file_of_the_wrong_shape_loads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CACHE_FILE_NAME);
        for text in ["[]", "{\"scopes\": 3}", "3", "null"] {
            fs::write(&path, text).unwrap();
            assert_eq!(NoticeCache::load(&path), NoticeCache::default(), "{text}");
        }
    }

    #[test]
    fn a_scope_without_the_reported_field_keeps_the_other_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CACHE_FILE_NAME);
        fs::write(
            &path,
            r#"{"scopes": {"/work/project": {"checked_at": 7, "updates": ["skill:a@c1"]}}}"#,
        )
        .unwrap();
        let cache = NoticeCache::load(&path);
        let state = &cache.scopes["/work/project"];
        assert_eq!(state.checked_at, 7);
        assert_eq!(state.updates, set(&["skill:a@c1"]));
        assert!(state.reported.is_empty());
    }

    #[test]
    fn a_root_without_state_prints_nothing() {
        let mut cache = NoticeCache::default();
        assert_eq!(decide(&mut cache, Scope::Project, &root(), false), None);
        assert_eq!(cache, NoticeCache::default());
    }

    #[test]
    fn record_check_stores_the_updates_sorted() {
        let cache = recorded(&["skill:b@c1", "rule:a@c1"]);
        assert_eq!(
            cache.scopes[&key(&root())].updates,
            set(&["rule:a@c1", "skill:b@c1"])
        );
    }

    #[test]
    fn two_orders_of_one_set_print_once() {
        let mut cache = recorded(&["skill:b@c1", "rule:a@c1"]);
        assert!(decide(&mut cache, Scope::Project, &root(), false).is_some());
        record_check(
            &mut cache,
            &root(),
            set(&["rule:a@c1", "skill:b@c1"]),
            Reached::Fully,
            2_000,
        );
        assert_eq!(decide(&mut cache, Scope::Project, &root(), false), None);
    }

    #[test]
    fn the_same_item_listed_twice_counts_once() {
        let check = check(vec![
            source(update(Some("c9")), vec![(ItemKind::Skill, "a")]),
            source(update(Some("c9")), vec![(ItemKind::Skill, "a")]),
        ]);
        assert_eq!(update_set(&check), set(&["skill:a@c9"]));
    }
}
