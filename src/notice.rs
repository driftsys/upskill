//! The session notice: when to print one line about available updates.
//!
//! The cache file records, for each scope root, the time of the last check
//! (a check that reached no source also sets it), the updates that check
//! found, and the set that the last printed notice reported. The functions
//! take the time as an argument. Only [`NoticeCache::load`] and
//! [`NoticeCache::save`] perform file I/O.
//!
//! Two runs that overlap each save the whole file. Before it saves, a run
//! loads the file again and applies only its own roots with [`merge_roots`],
//! which leaves the entries of the other run in place. The remaining race is
//! the time between that second load and the rename of the save: when another
//! run saves inside that time, its entries are lost, and its roots are checked
//! again at their next session start.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::pipeline::{ScopedCheck, SourceStatus};
use crate::scope::Scope;

/// The shortest time in seconds between two checks that contact the sources.
pub const NETWORK_PERIOD_SECS: u64 = 24 * 60 * 60;
/// The name of the cache file in the upskill cache directory.
pub const CACHE_FILE_NAME: &str = "notice.json";

/// State of one scope root in the cache file. A missing field of a file reads
/// as its default value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScopeState {
    /// Unix time in seconds of the last check. A check that reached no source
    /// also sets it, so the next attempt waits for the period.
    pub checked_at: u64,
    /// The available updates found by the checks, sorted. A check that
    /// reached only some of the sources of the scope keeps the earlier
    /// entries of the other sources (see [`record_check`]).
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
///   `<kind>:<name>` part is not in `updates` stays, because the source that
///   did not answer may still hold that update.
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
pub fn merge_roots(mut fresh: NoticeCache, mine: &NoticeCache, roots: &[PathBuf]) -> NoticeCache {
    for root in roots {
        match mine.scopes.get(&key(root)) {
            Some(state) => {
                fresh.scopes.insert(key(root), state.clone());
            }
            None => forget(&mut fresh, root),
        }
    }
    fresh
}

/// Remove each state whose root path `has_lockfile` rejects.
pub fn prune(cache: &mut NoticeCache, has_lockfile: impl Fn(&Path) -> bool) {
    cache.scopes.retain(|root, _| has_lockfile(Path::new(root)));
}

/// The line to print for `root`, and the cache change that goes with it.
/// Returns `None` when `updates` is empty (and clears `reported`) or equals
/// `reported`, and when `root` has no state. Otherwise sets
/// `reported = updates` and returns the line.
pub fn decide(cache: &mut NoticeCache, scope: Scope, root: &Path) -> Option<String> {
    let state = cache.scopes.get_mut(&key(root))?;
    if state.updates.is_empty() {
        state.reported.clear();
        return None;
    }
    if state.updates == state.reported {
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
            decide(&mut cache, Scope::Project, &root()).as_deref(),
            Some("upskill: 3 updates available (project). Run \"upskill update\".")
        );
        assert_eq!(decide(&mut cache, Scope::Project, &root()), None);
    }

    #[test]
    fn a_changed_set_prints_again() {
        let mut cache = recorded(&["skill:a@c1"]);
        assert!(decide(&mut cache, Scope::Project, &root()).is_some());
        record_check(
            &mut cache,
            &root(),
            set(&["skill:a@c2"]),
            Reached::Fully,
            2_000,
        );
        assert!(decide(&mut cache, Scope::Project, &root()).is_some());
        assert_eq!(decide(&mut cache, Scope::Project, &root()), None);
    }

    #[test]
    fn an_empty_set_prints_nothing_and_clears_the_reported_set() {
        let mut cache = recorded(&["skill:a@c1"]);
        assert!(decide(&mut cache, Scope::Project, &root()).is_some());
        record_check(&mut cache, &root(), vec![], Reached::Fully, 2_000);
        assert_eq!(decide(&mut cache, Scope::Project, &root()), None);
        assert!(cache.scopes[&key(&root())].reported.is_empty());
        record_check(
            &mut cache,
            &root(),
            set(&["skill:a@c1"]),
            Reached::Fully,
            3_000,
        );
        assert!(decide(&mut cache, Scope::Project, &root()).is_some());
    }

    #[test]
    fn an_equal_set_leaves_the_cache_unchanged() {
        let mut cache = recorded(&["skill:a@c1"]);
        decide(&mut cache, Scope::Project, &root());
        let before = cache.clone();
        assert_eq!(decide(&mut cache, Scope::Project, &root()), None);
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
        let mut cache = recorded(&["rule:b@c1", "skill:a@c1"]);
        record_check(
            &mut cache,
            &root(),
            set(&["skill:a@c2", "skill:z@c2"]),
            Reached::Partly,
            2_000,
        );
        let state = &cache.scopes[&key(&root())];
        // `skill:a` is in the new set, so its earlier entry is replaced.
        assert_eq!(
            state.updates,
            set(&["rule:b@c1", "skill:a@c2", "skill:z@c2"])
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
        let mut cache = recorded(&["agent:local", "skill:a@c1"]);
        record_check(
            &mut cache,
            &root(),
            set(&["agent:local"]),
            Reached::Partly,
            2_000,
        );
        assert_eq!(
            cache.scopes[&key(&root())].updates,
            set(&["agent:local", "skill:a@c1"])
        );
    }

    #[test]
    fn a_subset_from_a_partly_reached_check_does_not_print_a_second_line() {
        let full = ["rule:b@c1", "skill:a@c1"];
        let mut cache = recorded(&full);
        assert!(decide(&mut cache, Scope::Project, &root()).is_some());
        record_check(
            &mut cache,
            &root(),
            set(&["skill:a@c1"]),
            Reached::Partly,
            2_000,
        );
        assert_eq!(decide(&mut cache, Scope::Project, &root()), None);
        record_check(&mut cache, &root(), set(&full), Reached::Fully, 3_000);
        assert_eq!(decide(&mut cache, Scope::Project, &root()), None);
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
        let merged = merge_roots(fresh, &mine, &[root()]);
        assert_eq!(merged.scopes[&key(&root())].updates, set(&["skill:new@c1"]));
        assert_eq!(
            merged.scopes[&key(&other_root())].updates,
            set(&["skill:o@c9"])
        );
    }

    #[test]
    fn merge_roots_removes_a_handled_root_that_is_absent_from_my_cache() {
        let fresh = cache_of(&[(&root(), &["skill:a@c1"]), (&other_root(), &["skill:o@c1"])]);
        let merged = merge_roots(fresh, &NoticeCache::default(), &[root()]);
        assert!(!merged.scopes.contains_key(&key(&root())));
        assert!(merged.scopes.contains_key(&key(&other_root())));
    }

    #[test]
    fn merge_roots_adds_a_handled_root_that_the_fresh_cache_lacks() {
        let mine = cache_of(&[(&root(), &["skill:a@c1"])]);
        let merged = merge_roots(NoticeCache::default(), &mine, &[root()]);
        assert_eq!(merged, mine);
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
            decide(&mut cache, Scope::Project, &root()).as_deref(),
            Some("upskill: 1 update available (project). Run \"upskill update\".")
        );
    }

    #[test]
    fn the_global_line_names_the_global_flag() {
        let mut cache = recorded(&["skill:a@c1"]);
        assert_eq!(
            decide(&mut cache, Scope::Global, &root()).as_deref(),
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
        decide(&mut cache, Scope::Project, &root());
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
        assert_eq!(decide(&mut cache, Scope::Project, &root()), None);
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
        assert!(decide(&mut cache, Scope::Project, &root()).is_some());
        record_check(
            &mut cache,
            &root(),
            set(&["rule:a@c1", "skill:b@c1"]),
            Reached::Fully,
            2_000,
        );
        assert_eq!(decide(&mut cache, Scope::Project, &root()), None);
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
