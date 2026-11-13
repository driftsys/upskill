//! The update check: which sources of the lockfiles have a newer version.
//!
//! A git source is compared by commit: the commit that `git ls-remote` reports
//! for the ref of the source against the commit recorded in the lockfile. No
//! clone or fetch happens. A local path source is compared by content: the
//! hash of each item directory against the recorded hash. The module changes
//! no file.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::ItemKind;
use super::discovery::{find_registry_root, is_bundle_file, iter_item_dirs, probe_effective_name};
use super::git::clone_url;
use super::hash_item_dir;
use crate::fetch;
use crate::lockfile::{LockedItem, Lockfile};
use crate::scope::{Scope, ScopeRoot};
use crate::source::{InstallSource, parse_install_source_label};

/// Why a lookup of a remote commit gave no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    /// The time limit passed before the lookup finished.
    TimedOut,
    /// The lookup failed; the text is the message of the failure.
    Failed(String),
}

/// Resolves the commit of a git source. [`GitRemote`] is the implementation
/// that runs `git ls-remote`; tests pass their own.
pub trait RemoteLookup {
    /// The commit that the ref of `source` names, or `Ok(None)` when the
    /// remote has no such ref. The lookup ends at `deadline`.
    fn resolve(
        &mut self,
        source: &InstallSource,
        deadline: Instant,
    ) -> Result<Option<String>, LookupError>;
}

/// The lookup that runs `git ls-remote` against the clone URL of the source.
pub struct GitRemote;

impl RemoteLookup for GitRemote {
    fn resolve(
        &mut self,
        source: &InstallSource,
        deadline: Instant,
    ) -> Result<Option<String>, LookupError> {
        let Some(url) = clone_url(source) else {
            return Err(LookupError::Failed("not a git source".to_string()));
        };
        let git_ref = match source {
            InstallSource::Github(repo) => repo.git_ref.as_deref(),
            InstallSource::Git(repo) => repo.git_ref.as_deref(),
            InstallSource::LocalPath(_) => None,
        };
        fetch::ls_remote(&url, git_ref, deadline).map_err(|e| match e {
            fetch::RunError::TimedOut => LookupError::TimedOut,
            fetch::RunError::Failed(message) => LookupError::Failed(message),
        })
    }
}

/// The result of the check for one source of one scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceStatus {
    /// Every recorded commit equals the remote commit, or no item of a local
    /// path source differs from its recorded hash.
    UpToDate,
    /// `from` and `to` are commits for a git source and `None` for a local
    /// path source.
    UpdateAvailable {
        from: Option<String>,
        to: Option<String>,
    },
    /// The ref is a full commit hash. Not looked up.
    PinnedToCommit,
    /// A git source with an item that has no recorded commit.
    NoRecordedCommit,
    /// The source could not be checked. `reason` is one line.
    NotReachable { reason: String },
    /// The time limit passed before this source was looked up.
    NotChecked,
}

/// One source of one scope with its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedSource {
    /// The source label, as recorded in the lockfile.
    pub source: String,
    pub status: SourceStatus,
    /// For `UpdateAvailable`: every item of a git source; the items whose
    /// hash differs for a local path source. Sorted by kind, then name.
    /// Empty for every other status.
    pub items: Vec<(ItemKind, String)>,
}

/// The checked sources of one scope, in lockfile order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedCheck {
    pub scope: Scope,
    pub root: PathBuf,
    pub sources: Vec<CheckedSource>,
}

impl ScopedCheck {
    /// The number of items of the sources with status `UpdateAvailable`.
    pub fn update_count(&self) -> usize {
        self.sources
            .iter()
            .filter(|s| matches!(s.status, SourceStatus::UpdateAvailable { .. }))
            .map(|s| s.items.len())
            .sum()
    }
}

type Lookups = HashMap<(String, Option<String>), Result<Option<String>, LookupError>>;

/// Check every source of the lockfile of each root, in lockfile order.
/// `names` limits the check to the items with those names; empty means all.
/// A git source is looked up at most once for all roots (the key is the clone
/// URL and the ref). A source that shares its lookup with an earlier source
/// gets the result of that lookup, also after the time limit has passed; a
/// source that needs a new lookup after the time limit is `NotChecked`. A
/// lockfile that cannot be read is an error. Changes no file.
pub fn check_scopes(
    roots: &[ScopeRoot],
    names: &[String],
    deadline: Instant,
    remote: &mut dyn RemoteLookup,
) -> Result<Vec<ScopedCheck>> {
    let mut lookups = Lookups::new();
    let mut checks = Vec::new();
    for at in roots {
        let lock = Lockfile::load(&at.root)
            .with_context(|| format!("read the lockfile of the {} scope", at.scope))?;
        // Group the selected entries by source label, in lockfile order.
        let mut groups: Vec<(String, Vec<&LockedItem>)> = Vec::new();
        for entry in &lock.items {
            if !names.is_empty() && !names.iter().any(|n| n == &entry.name) {
                continue;
            }
            match groups.iter_mut().find(|(label, _)| label == &entry.source) {
                Some((_, entries)) => entries.push(entry),
                None => groups.push((entry.source.clone(), vec![entry])),
            }
        }
        let sources = groups
            .into_iter()
            .map(|(label, entries)| check_source(label, &entries, deadline, remote, &mut lookups))
            .collect();
        checks.push(ScopedCheck {
            scope: at.scope,
            root: at.root.clone(),
            sources,
        });
    }
    Ok(checks)
}

fn check_source(
    label: String,
    entries: &[&LockedItem],
    deadline: Instant,
    remote: &mut dyn RemoteLookup,
    lookups: &mut Lookups,
) -> CheckedSource {
    let unreachable = |label: String, reason: String| CheckedSource {
        source: label,
        status: SourceStatus::NotReachable { reason },
        items: Vec::new(),
    };
    let source = match parse_install_source_label(&label) {
        Ok(source) => source,
        Err(e) => return unreachable(label, one_line(&e.to_string())),
    };
    let git_ref = match &source {
        InstallSource::LocalPath(path) => return check_local(label, path, entries),
        InstallSource::Github(repo) => repo.git_ref.clone(),
        InstallSource::Git(repo) => repo.git_ref.clone(),
    };
    let done =
        |label: String, status: SourceStatus, items: Vec<(ItemKind, String)>| CheckedSource {
            source: label,
            status,
            items,
        };
    if git_ref.as_deref().is_some_and(fetch::is_full_commit_hash) {
        return done(label, SourceStatus::PinnedToCommit, Vec::new());
    }
    let key = (clone_url(&source).unwrap_or_default(), git_ref.clone());
    let result = match lookups.get(&key) {
        Some(cached) => cached.clone(),
        None => {
            if Instant::now() >= deadline {
                return done(label, SourceStatus::NotChecked, Vec::new());
            }
            let fresh = remote.resolve(&source, deadline);
            lookups.insert(key, fresh.clone());
            fresh
        }
    };
    let remote_commit = match result {
        Err(LookupError::TimedOut) => {
            return unreachable(label, "time limit reached".to_string());
        }
        Err(LookupError::Failed(message)) => return unreachable(label, one_line(&message)),
        Ok(None) => {
            let reason = match &git_ref {
                Some(r) => format!("ref {r} not found"),
                None => "HEAD not found".to_string(),
            };
            return unreachable(label, reason);
        }
        Ok(Some(commit)) => commit,
    };
    if entries.iter().any(|e| e.commit.is_none()) {
        return done(label, SourceStatus::NoRecordedCommit, Vec::new());
    }
    let first_differing = entries
        .iter()
        .filter_map(|e| e.commit.as_ref())
        .find(|c| !c.eq_ignore_ascii_case(&remote_commit));
    match first_differing {
        None => done(label, SourceStatus::UpToDate, Vec::new()),
        Some(from) => {
            let from = from.clone();
            done(
                label,
                SourceStatus::UpdateAvailable {
                    from: Some(from),
                    to: Some(remote_commit),
                },
                sorted_items(entries.iter().copied()),
            )
        }
    }
}

/// Compare each item directory of a local path source with the recorded hash.
fn check_local(label: String, path: &Path, entries: &[&LockedItem]) -> CheckedSource {
    if !path.exists() {
        return CheckedSource {
            source: label,
            status: SourceStatus::NotReachable {
                reason: "path not found".to_string(),
            },
            items: Vec::new(),
        };
    }
    // The install walks the registry root of a bundle file, and the path
    // itself otherwise.
    let root = if is_bundle_file(path) {
        find_registry_root(path).unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    let dirs = match iter_item_dirs(&root) {
        Ok(dirs) => dirs,
        Err(e) => {
            return CheckedSource {
                source: label,
                status: SourceStatus::NotReachable {
                    reason: one_line(&format!("{e:#}")),
                },
                items: Vec::new(),
            };
        }
    };
    let changed: Vec<&LockedItem> = entries
        .iter()
        .copied()
        .filter(|entry| {
            let wanted = entry.source_name.as_deref().unwrap_or(&entry.name);
            let dir = dirs.iter().find(|(folder, dir)| {
                let entrypoint = dir.join(entry.kind.entrypoint_filename());
                entrypoint.is_file()
                    && probe_effective_name(&entrypoint, entry.kind, folder) == wanted
            });
            match dir {
                Some((_, dir)) => hash_item_dir(dir) != entry.hash,
                None => true,
            }
        })
        .collect();
    if changed.is_empty() {
        return CheckedSource {
            source: label,
            status: SourceStatus::UpToDate,
            items: Vec::new(),
        };
    }
    CheckedSource {
        source: label,
        status: SourceStatus::UpdateAvailable {
            from: None,
            to: None,
        },
        items: sorted_items(changed),
    }
}

fn sorted_items<'a>(entries: impl IntoIterator<Item = &'a LockedItem>) -> Vec<(ItemKind, String)> {
    let mut items: Vec<(ItemKind, String)> = entries
        .into_iter()
        .map(|e| (e.kind, e.name.clone()))
        .collect();
    items.sort();
    items
}

/// The first non-empty line of `text`, trimmed.
fn one_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("unknown error")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{AddOptions, install_with_lockfile};
    use crate::plugin::PluginScope;
    use std::collections::HashMap;
    use std::fs;
    use std::time::Duration;

    const COMMIT_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const COMMIT_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const COMMIT_C: &str = "cccccccccccccccccccccccccccccccccccccccc";
    const SOURCE: &str = "github:acme/tools";

    /// Answers by source label and counts the calls.
    #[derive(Default)]
    struct FakeRemote {
        answers: HashMap<String, Result<Option<String>, LookupError>>,
        calls: usize,
    }

    impl FakeRemote {
        fn with(label: &str, answer: Result<Option<String>, LookupError>) -> Self {
            let mut fake = Self::default();
            fake.answers.insert(label.to_string(), answer);
            fake
        }
    }

    impl RemoteLookup for FakeRemote {
        fn resolve(
            &mut self,
            source: &InstallSource,
            _deadline: Instant,
        ) -> Result<Option<String>, LookupError> {
            self.calls += 1;
            self.answers
                .get(&source.to_string())
                .cloned()
                .unwrap_or(Ok(None))
        }
    }

    fn entry(kind: ItemKind, name: &str, source: &str, commit: Option<&str>) -> LockedItem {
        let json = serde_json::json!({
            "kind": kind, "name": name, "source": source,
        });
        let mut item: LockedItem = serde_json::from_value(json).unwrap();
        item.commit = commit.map(str::to_string);
        item
    }

    fn save(root: &Path, items: Vec<LockedItem>) {
        let mut lock = Lockfile::new();
        for item in items {
            lock.upsert(item);
        }
        lock.save(root).unwrap();
    }

    fn root(dir: &Path, scope: Scope) -> ScopeRoot {
        fs::create_dir_all(dir).unwrap();
        ScopeRoot {
            scope,
            root: dir.to_path_buf(),
        }
    }

    fn far() -> Instant {
        Instant::now() + Duration::from_secs(60)
    }

    fn status_of(checks: &[ScopedCheck]) -> &SourceStatus {
        &checks[0].sources[0].status
    }

    #[test]
    fn a_source_with_the_recorded_commit_is_up_to_date() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        save(
            &at.root,
            vec![entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A))],
        );
        // The comparison ignores case.
        let mut remote = FakeRemote::with(SOURCE, Ok(Some(COMMIT_A.to_uppercase())));
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert_eq!(status_of(&checks), &SourceStatus::UpToDate);
        assert!(checks[0].sources[0].items.is_empty());
        assert_eq!(checks[0].update_count(), 0);
    }

    #[test]
    fn a_source_with_another_commit_lists_every_item() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        save(
            &at.root,
            vec![
                entry(ItemKind::Skill, "zeta", SOURCE, Some(COMMIT_B)),
                entry(ItemKind::Rule, "beta", SOURCE, Some(COMMIT_A)),
                entry(ItemKind::Skill, "alpha", SOURCE, Some(COMMIT_B)),
            ],
        );
        let mut remote = FakeRemote::with(SOURCE, Ok(Some(COMMIT_C.to_string())));
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        // The lockfile keeps its entries sorted by kind, then name, so the
        // first entry is the rule `beta`. `from` is the recorded commit of the
        // first entry that differs from the remote commit; all three differ.
        assert_eq!(
            status_of(&checks),
            &SourceStatus::UpdateAvailable {
                from: Some(COMMIT_A.to_string()),
                to: Some(COMMIT_C.to_string()),
            }
        );
        assert_eq!(
            checks[0].sources[0].items,
            vec![
                (ItemKind::Rule, "beta".to_string()),
                (ItemKind::Skill, "alpha".to_string()),
                (ItemKind::Skill, "zeta".to_string()),
            ]
        );
        assert_eq!(checks[0].update_count(), 3);
    }

    #[test]
    fn a_full_commit_hash_ref_is_not_looked_up() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        let label = format!("github:acme/tools@{COMMIT_A}");
        save(&at.root, vec![entry(ItemKind::Skill, "a", &label, None)]);
        let mut remote = FakeRemote::default();
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert_eq!(status_of(&checks), &SourceStatus::PinnedToCommit);
        assert_eq!(remote.calls, 0);
    }

    #[test]
    fn an_entry_without_a_commit_is_no_recorded_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        save(
            &at.root,
            vec![
                entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A)),
                entry(ItemKind::Skill, "b", SOURCE, None),
            ],
        );
        let mut remote = FakeRemote::with(SOURCE, Ok(Some(COMMIT_B.to_string())));
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert_eq!(status_of(&checks), &SourceStatus::NoRecordedCommit);
    }

    #[test]
    fn a_failed_lookup_is_not_reachable_and_later_sources_are_checked() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        let other = "github:acme/other";
        save(
            &at.root,
            vec![
                entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A)),
                entry(ItemKind::Skill, "b", other, Some(COMMIT_A)),
            ],
        );
        let mut remote = FakeRemote::with(
            SOURCE,
            Err(LookupError::Failed("first line\nsecond line".to_string())),
        );
        remote
            .answers
            .insert(other.to_string(), Ok(Some(COMMIT_A.to_string())));
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        let sources = &checks[0].sources;
        assert_eq!(
            sources[0].status,
            SourceStatus::NotReachable {
                reason: "first line".to_string()
            }
        );
        assert_eq!(sources[1].status, SourceStatus::UpToDate);
    }

    #[test]
    fn a_timed_out_lookup_and_a_missing_ref_are_not_reachable() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        let versioned = "github:acme/other@v9";
        save(
            &at.root,
            vec![
                entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A)),
                entry(ItemKind::Skill, "b", versioned, Some(COMMIT_A)),
            ],
        );
        let mut remote = FakeRemote::with(SOURCE, Err(LookupError::TimedOut));
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        let sources = &checks[0].sources;
        assert_eq!(
            sources[0].status,
            SourceStatus::NotReachable {
                reason: "time limit reached".to_string()
            }
        );
        assert_eq!(
            sources[1].status,
            SourceStatus::NotReachable {
                reason: "ref v9 not found".to_string()
            }
        );
    }

    #[test]
    fn a_passed_deadline_is_not_checked() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        save(
            &at.root,
            vec![entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A))],
        );
        let mut remote = FakeRemote::with(SOURCE, Ok(Some(COMMIT_A.to_string())));
        let checks = check_scopes(&[at], &[], Instant::now(), &mut remote).unwrap();
        assert_eq!(status_of(&checks), &SourceStatus::NotChecked);
        assert_eq!(remote.calls, 0);
    }

    #[test]
    fn one_source_in_two_scopes_is_looked_up_once() {
        let tmp = tempfile::tempdir().unwrap();
        let project = root(&tmp.path().join("p"), Scope::Project);
        let global = root(&tmp.path().join("g"), Scope::Global);
        save(
            &project.root,
            vec![entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A))],
        );
        save(
            &global.root,
            vec![entry(ItemKind::Skill, "b", SOURCE, Some(COMMIT_B))],
        );
        let mut remote = FakeRemote::with(SOURCE, Ok(Some(COMMIT_B.to_string())));
        let checks = check_scopes(&[project, global], &[], far(), &mut remote).unwrap();
        assert_eq!(remote.calls, 1);
        assert!(matches!(
            checks[0].sources[0].status,
            SourceStatus::UpdateAvailable { .. }
        ));
        assert_eq!(checks[1].sources[0].status, SourceStatus::UpToDate);
        assert_eq!(checks[1].scope, Scope::Global);
    }

    #[test]
    fn a_root_without_a_lockfile_has_no_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        let mut remote = FakeRemote::default();
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert!(checks[0].sources.is_empty());
    }

    #[test]
    fn an_unparsable_label_is_not_reachable() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        save(
            &at.root,
            vec![entry(ItemKind::Skill, "a", "nonsense", None)],
        );
        let mut remote = FakeRemote::default();
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert!(matches!(
            status_of(&checks),
            SourceStatus::NotReachable { .. }
        ));
        assert_eq!(remote.calls, 0);
    }

    #[test]
    fn names_limit_the_check() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        let other = "github:acme/other";
        save(
            &at.root,
            vec![
                entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A)),
                entry(ItemKind::Skill, "b", SOURCE, Some(COMMIT_A)),
                entry(ItemKind::Skill, "c", other, Some(COMMIT_A)),
            ],
        );
        let mut remote = FakeRemote::with(SOURCE, Ok(Some(COMMIT_B.to_string())));
        let checks = check_scopes(&[at], &["b".to_string()], far(), &mut remote).unwrap();
        assert_eq!(checks[0].sources.len(), 1);
        assert_eq!(checks[0].sources[0].source, SOURCE);
        assert_eq!(
            checks[0].sources[0].items,
            vec![(ItemKind::Skill, "b".to_string())]
        );
        assert_eq!(remote.calls, 1);
    }

    /// Install two skills from a local source into a project root.
    fn install_local(tmp: &Path) -> (PathBuf, ScopeRoot) {
        let source = tmp.join("src");
        for name in ["one", "two"] {
            let dir = source.join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("SKILL.md"),
                format!("---\nschema: 1\nname: {name}\ndescription: test {name}\n---\n# {name}\n"),
            )
            .unwrap();
        }
        let at = root(&tmp.join("project"), Scope::Project);
        install_with_lockfile(
            &InstallSource::LocalPath(source.clone()),
            &at.root,
            &[],
            PluginScope::Project,
            &AddOptions::default(),
        )
        .unwrap();
        (source, at)
    }

    #[test]
    fn a_local_item_with_another_hash_has_an_update() {
        let tmp = tempfile::tempdir().unwrap();
        let (source, at) = install_local(tmp.path());
        fs::write(
            source.join("two").join("SKILL.md"),
            "---\nschema: 1\nname: two\ndescription: changed\n---\n# two\n",
        )
        .unwrap();
        let mut remote = FakeRemote::default();
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert_eq!(
            status_of(&checks),
            &SourceStatus::UpdateAvailable {
                from: None,
                to: None
            }
        );
        assert_eq!(
            checks[0].sources[0].items,
            vec![(ItemKind::Skill, "two".to_string())]
        );
        assert_eq!(remote.calls, 0);
    }

    #[test]
    fn a_local_item_with_the_recorded_hash_is_up_to_date() {
        let tmp = tempfile::tempdir().unwrap();
        let (_, at) = install_local(tmp.path());
        let mut remote = FakeRemote::default();
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert_eq!(status_of(&checks), &SourceStatus::UpToDate);
    }

    #[test]
    fn a_local_item_removed_from_the_source_has_an_update() {
        let tmp = tempfile::tempdir().unwrap();
        let (source, at) = install_local(tmp.path());
        fs::remove_dir_all(source.join("one")).unwrap();
        let mut remote = FakeRemote::default();
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert_eq!(
            checks[0].sources[0].items,
            vec![(ItemKind::Skill, "one".to_string())]
        );
    }

    #[test]
    fn a_missing_local_path_is_not_reachable() {
        let tmp = tempfile::tempdir().unwrap();
        let (source, at) = install_local(tmp.path());
        fs::remove_dir_all(&source).unwrap();
        let mut remote = FakeRemote::default();
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert_eq!(
            status_of(&checks),
            &SourceStatus::NotReachable {
                reason: "path not found".to_string()
            }
        );
    }

    #[test]
    fn the_check_changes_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        save(
            &at.root,
            vec![entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A))],
        );
        let path = at.root.join(".upskill-lock.json");
        let before = fs::read(&path).unwrap();
        let mut remote = FakeRemote::with(SOURCE, Ok(Some(COMMIT_B.to_string())));
        check_scopes(std::slice::from_ref(&at), &[], far(), &mut remote).unwrap();
        assert_eq!(fs::read(&path).unwrap(), before);
        let names: Vec<_> = fs::read_dir(&at.root).unwrap().flatten().collect();
        assert_eq!(names.len(), 1);
    }

    #[test]
    fn a_schema_1_lockfile_is_not_adopted_or_saved() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        let path = at.root.join(".upskill-lock.json");
        let raw = format!(
            r#"{{"schema":1,"items":[{{"kind":"skill","name":"a","source":"{SOURCE}","commit":"{COMMIT_A}"}}]}}"#
        );
        fs::write(&path, &raw).unwrap();
        let mut remote = FakeRemote::with(SOURCE, Ok(Some(COMMIT_B.to_string())));
        let checks = check_scopes(std::slice::from_ref(&at), &[], far(), &mut remote).unwrap();
        assert!(matches!(
            status_of(&checks),
            SourceStatus::UpdateAvailable { .. }
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), raw);
        assert!(!at.root.join(crate::lockfile::SCHEMA1_BACKUP_NAME).exists());
    }

    #[test]
    fn a_cached_lookup_is_used_after_the_deadline() {
        let tmp = tempfile::tempdir().unwrap();
        let project = root(&tmp.path().join("p"), Scope::Project);
        let global = root(&tmp.path().join("g"), Scope::Global);
        for at in [&project, &global] {
            save(
                &at.root,
                vec![entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A))],
            );
        }
        // The first lookup runs before the deadline; the deadline passes
        // while it runs.
        struct SlowRemote(Instant, usize);
        impl RemoteLookup for SlowRemote {
            fn resolve(
                &mut self,
                _source: &InstallSource,
                _deadline: Instant,
            ) -> Result<Option<String>, LookupError> {
                self.1 += 1;
                while Instant::now() < self.0 {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(Some(COMMIT_A.to_string()))
            }
        }
        let deadline = Instant::now() + Duration::from_millis(50);
        let mut remote = SlowRemote(deadline, 0);
        let checks = check_scopes(&[project, global], &[], deadline, &mut remote).unwrap();
        assert_eq!(remote.1, 1);
        assert_eq!(checks[0].sources[0].status, SourceStatus::UpToDate);
        assert_eq!(checks[1].sources[0].status, SourceStatus::UpToDate);
    }

    #[test]
    fn a_names_value_that_matches_nothing_gives_no_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        save(
            &at.root,
            vec![entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A))],
        );
        let mut remote = FakeRemote::default();
        let checks = check_scopes(&[at], &["zzz".to_string()], far(), &mut remote).unwrap();
        assert!(checks[0].sources.is_empty());
        assert_eq!(remote.calls, 0);
    }

    #[test]
    fn an_unreadable_lockfile_is_an_error_and_writes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        let path = at.root.join(".upskill-lock.json");
        fs::write(&path, "{ not json").unwrap();
        let mut remote = FakeRemote::default();
        assert!(check_scopes(std::slice::from_ref(&at), &[], far(), &mut remote).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");
        assert_eq!(fs::read_dir(&at.root).unwrap().count(), 1);
    }

    #[test]
    fn a_remote_without_head_gives_the_head_reason() {
        let tmp = tempfile::tempdir().unwrap();
        let at = root(&tmp.path().join("p"), Scope::Project);
        save(
            &at.root,
            vec![entry(ItemKind::Skill, "a", SOURCE, Some(COMMIT_A))],
        );
        let mut remote = FakeRemote::with(SOURCE, Ok(None));
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert_eq!(
            status_of(&checks),
            &SourceStatus::NotReachable {
                reason: "HEAD not found".to_string()
            }
        );
    }

    #[test]
    fn an_aliased_local_item_is_up_to_date_when_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("src");
        let dir = source.join("one");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nschema: 1\nname: one\ndescription: test one\n---\n# one\n",
        )
        .unwrap();
        let at = root(&tmp.path().join("project"), Scope::Project);
        let options = AddOptions {
            aliases: vec![("".to_string(), "uno".to_string())],
            ..AddOptions::default()
        };
        install_with_lockfile(
            &InstallSource::LocalPath(source.clone()),
            &at.root,
            &[],
            PluginScope::Project,
            &options,
        )
        .unwrap();
        let lock = Lockfile::load(&at.root).unwrap();
        assert_eq!(lock.items[0].name, "uno");
        assert_eq!(lock.items[0].source_name.as_deref(), Some("one"));
        let mut remote = FakeRemote::default();
        let checks = check_scopes(std::slice::from_ref(&at), &[], far(), &mut remote).unwrap();
        assert_eq!(status_of(&checks), &SourceStatus::UpToDate);
        fs::write(
            dir.join("SKILL.md"),
            "---\nschema: 1\nname: one\ndescription: changed\n---\n# one\n",
        )
        .unwrap();
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        assert_eq!(
            checks[0].sources[0].items,
            vec![(ItemKind::Skill, "uno".to_string())]
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_local_source_is_not_reachable() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let (source, at) = install_local(tmp.path());
        fs::set_permissions(&source, fs::Permissions::from_mode(0o000)).unwrap();
        let readable = fs::read_dir(&source).is_ok();
        let mut remote = FakeRemote::default();
        let checks = check_scopes(&[at], &[], far(), &mut remote).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
        if readable {
            // Running as a user that ignores permissions (root).
            return;
        }
        match status_of(&checks) {
            SourceStatus::NotReachable { reason } => {
                assert!(!reason.is_empty());
                assert_eq!(reason.lines().count(), 1);
            }
            other => panic!("expected NotReachable, got {other:?}"),
        }
        assert!(checks[0].sources[0].items.is_empty());
    }
}
