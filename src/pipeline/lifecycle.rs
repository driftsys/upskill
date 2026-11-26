//! Lifecycle operations over `.upskill-lock.json`: `remove`, `doctor`,
//! `update`, `list`.
//!
//! These commands consult the lockfile to know what is installed and
//! what to act on. They orchestrate the leaf modules (`output`,
//! `hash`, `apply`, `owned`) and the prepared install of one source
//! (`prepared::prepare_install`) — they own no parsing or render logic
//! themselves.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::scope::{Scope, ScopeRoot};

use super::apply::{
    check_item_records, check_lockfile_records, check_no_item_symlink_above, deletable,
    delete_recorded, prune_empty_parents,
};
use super::output::{output_path, released_output_path};
use super::owned::{FileState, adopt_schema1, adopt_schema1_in_memory, classify};
use super::prepared::prepare_install;
use super::{
    ALL_CLIENTS, AddOptions, AdoptedBeforeFailure, BothScopesItem, DoctorReport, DoctorScopes,
    EarlierLayoutFile, FetchFailed, InterruptedItem, ItemKind, KeptItem, ListReport, ListedBundle,
    ListedItem, MissingOutput, MissingPlugin, MissingRuleEntry, ModifiedFile, OrphanEntry,
    OrphanReason, OrphanedDependency, PreviewAction, PreviewItem, RemoveFilter, RemoveReport,
    RemovedItem, ScopedDoctor, ScopedList, SkipCause, SkippedBeforeFailure, SkippedPlugin,
    SkippedSource, SourceDecision, SourcePreview, SourceResult, StaleHash, StaleRuleEntry,
    UnreadableFile, UpdateObserver, UpdateReport, UpdateStatus, UpdatedItem, hash_item_dir,
};

/// Remove installed content recorded in `<target>/.upskill-lock.json`,
/// matching `filter`, and drop the matching entries from the lockfile.
///
/// A `schema: 1` lockfile is adopted first (see
/// `owned::adopt_schema1`), after the unknown-name check, so a
/// command that fails on an unknown name changes nothing.
///
/// Only the files in an item's `files` and `pending` records are
/// considered; a file is owned when it matches a hash of either record.
/// Before any change on disk, the records of every item in the lockfile
/// are checked (see `check_lockfile_records`): a recorded path that is
/// outside the layout of its item is an error. Then, for each item to
/// remove, a symbolic link in the
/// item part of a path's parent directories is an error. Then, per
/// recorded file: an unchanged regular file is deleted; a modified file is
/// kept and reported in `kept_modified`, or deleted when `force` is true
/// and it is a regular file; an absent file is ignored. A regular file that
/// exists but cannot be read is handled as a modified file; any other error
/// while inspecting a recorded path fails the command before any deletion.
/// A file that the records do not list is never deleted. Directories left
/// empty are removed, up to and excluding the kind directory (for example
/// `.claude/skills`).
///
/// Before the adoption and before any deletion, also when `force` is true,
/// `remove` fails when the source directory of another installed item,
/// recorded from a local-path source, contains a recorded file (any file of
/// the `files` and `pending` records, also a modified file that the removal
/// would keep) of an item that it removes (`check_removal_outside_sources`).
/// Only the recorded local sources of the items that stay installed are
/// read: such a source that does not exist is skipped; one that exists and
/// cannot be read fails `remove` with an error that names it.
///
/// Ancillary files (`CLAUDE.md`, `opencode.json`,
/// `.vscode/settings.json`) are user-owned after creation per ADR-0003, so
/// they are left alone with two exceptions. First, for each bundle that lists
/// a removed item (or has the removed source), the installed opencode plugin
/// entry of that bundle is deleted from the `plugin` and `plugins` arrays of
/// the opencode config file of the scope of the lockfile, not of the scope
/// that the entry records (`remove_opencode_plugin_uri`; an error is
/// ignored). Second, in the global
/// scope the `instructions` entry of each removed rule is deleted from the
/// global opencode config, other entries are kept, and an error is ignored
/// (`drop_global_opencode_rules`).
pub fn remove(at: &ScopeRoot, filter: RemoveFilter, force: bool) -> Result<RemoveReport> {
    let target = at.root.as_path();
    let lock = crate::lockfile::Lockfile::load(target)?;
    check_lockfile_records(target, &lock)?;

    let mut to_remove: Vec<crate::lockfile::LockedItem> = match &filter {
        RemoveFilter::ByNames(names) => lock
            .items
            .iter()
            .filter(|i| names.iter().any(|n| n == &i.name))
            .cloned()
            .collect(),
        RemoveFilter::BySource(source) => lock
            .items
            .iter()
            .filter(|i| &i.source == source)
            .cloned()
            .collect(),
    };

    if let RemoveFilter::ByNames(names) = &filter {
        let matched: std::collections::BTreeSet<&str> =
            to_remove.iter().map(|i| i.name.as_str()).collect();
        let unknown: Vec<&str> = names
            .iter()
            .filter(|n| !matched.contains(n.as_str()))
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            anyhow::bail!("not in lockfile: {}", unknown.join(", "));
        }
    }

    // Co-location coupling (§2.1, Task 6): removing any named member of a
    // `(source, group)` unit removes every other member, even when their
    // effective names diverge. Expand AFTER the unknown-name check so a
    // name absent from the lockfile still errors rather than being masked.
    if let RemoveFilter::ByNames(_) = &filter {
        let groups: std::collections::BTreeSet<(String, String)> = to_remove
            .iter()
            .filter_map(|i| i.group.clone().map(|g| (i.source.clone(), g)))
            .collect();
        for it in &lock.items {
            if it
                .group
                .as_ref()
                .is_some_and(|g| groups.contains(&(it.source.clone(), g.clone())))
                && !to_remove
                    .iter()
                    .any(|r| r.kind == it.kind && r.name == it.name)
            {
                to_remove.push(it.clone());
            }
        }
    }

    remove_items(at, lock, filter, &to_remove, force)
}

/// Remove exactly the items `keys`, without the co-location expansion of
/// [`remove`]. `add --exclude` uses it so that a member of the same group
/// that the command does not exclude stays installed. Bundle records stay
/// (issue #570).
///
/// An item with a modified recorded file is not removed: its files and its
/// lockfile entry stay, and it is returned in the second value with its
/// modified files, so that no file stays on disk without an owner.
pub(crate) fn remove_exact(
    at: &ScopeRoot,
    keys: &[(ItemKind, String)],
) -> Result<(RemoveReport, Vec<KeptItem>)> {
    let target = at.root.as_path();
    let lock = crate::lockfile::Lockfile::load(target)?;
    check_lockfile_records(target, &lock)?;
    let mut to_remove = Vec::new();
    let mut kept = Vec::new();
    for item in lock
        .items
        .iter()
        .filter(|i| keys.iter().any(|(k, n)| *k == i.kind && n == &i.name))
    {
        let plan = plan_removal(target, item, false)?;
        if plan.kept.is_empty() {
            to_remove.push(item.clone());
        } else {
            kept.push(KeptItem {
                kind: item.kind,
                name: item.name.clone(),
                modified: plan.kept,
            });
        }
    }
    if to_remove.is_empty() {
        return Ok((RemoveReport::default(), kept));
    }
    // An empty name list selects no bundle, so the bundle records, their
    // plugin entries and the opencode plugin URIs stay.
    let report = remove_items(
        at,
        lock,
        RemoveFilter::ByNames(Vec::new()),
        &to_remove,
        false,
    )?;
    Ok((report, kept))
}

/// The part of `remove` that runs once the items are selected: check, adopt
/// a `schema: 1` lockfile, then plan and delete.
fn remove_items(
    at: &ScopeRoot,
    mut lock: crate::lockfile::Lockfile,
    filter: RemoveFilter,
    to_remove: &[crate::lockfile::LockedItem],
    force: bool,
) -> Result<RemoveReport> {
    let target = at.root.as_path();
    let keys: Vec<(ItemKind, String)> =
        to_remove.iter().map(|i| (i.kind, i.name.clone())).collect();

    // Before the adoption and before any deletion, whatever `force` is.
    check_removal_outside_sources(target, &lock, to_remove)?;

    // The adoption is saved before any file is deleted. Every later error
    // carries the adopted paths, so that `main.rs` can still print the adoption notice.
    let adopted = adopt_schema1(target, &mut lock)?;
    let result = remove_selected(at, &mut lock, &filter, &keys, force);
    match (result, adopted) {
        (Ok(mut report), adopted) => {
            report.adopted = adopted;
            Ok(report)
        }
        (Err(err), Some(paths)) if !paths.is_empty() => {
            Err(err.context(AdoptedBeforeFailure { paths }))
        }
        (Err(err), _) => Err(err),
    }
}

/// Refuse a `remove` when the source directory of another installed item,
/// recorded from a local-path source, contains a recorded file of an item in
/// `to_remove` (issue #550). Every file of the `files` and `pending` records
/// is compared, also a modified file that the removal would keep. The recorded files are those of
/// the lockfile after an adoption in memory (`adopt_schema1_in_memory`), so
/// that a `schema: 1` lockfile is checked with the files that the adoption
/// records; nothing is written. The item directories are read as
/// in the overlap check of [`update`] (`collect_recorded_local_items`); a git
/// source is not fetched. Only the sources of the items that stay installed
/// are read: such a source that does not exist is skipped, and one that exists
/// and cannot be read is an error that names its label.
fn check_removal_outside_sources(
    target: &Path,
    lock: &crate::lockfile::Lockfile,
    to_remove: &[crate::lockfile::LockedItem],
) -> Result<()> {
    let removed = |e: &crate::lockfile::LockedItem| {
        to_remove
            .iter()
            .any(|r| r.kind == e.kind && r.name == e.name)
    };
    // The records are read after an adoption in memory, which writes
    // nothing: a `schema: 1` lockfile has no `files` record before it.
    let mut adopted_lock = lock.clone();
    adopt_schema1_in_memory(target, &mut adopted_lock)?;
    let recorded: Vec<super::output::RecordedPath> = adopted_lock
        .items
        .iter()
        .filter(|e| removed(e))
        .flat_map(super::output::recorded_paths)
        .collect();
    let others: Vec<&crate::lockfile::LockedItem> =
        lock.items.iter().filter(|e| !removed(e)).collect();
    let labels: std::collections::BTreeSet<&str> =
        others.iter().map(|e| e.source.as_str()).collect();
    for label in labels {
        let mut source_items = Vec::new();
        collect_recorded_local_items(label, &mut source_items).with_context(|| {
            format!(
                "read the recorded source `{label}` for the overlap check of `remove`; restore it \
                 or make it readable"
            )
        })?;
        // The installed item of `label` that an item directory holds.
        let owner = |item: &super::output::SourceItem| {
            others.iter().copied().find(|e| {
                e.source == label
                    && e.kind == item.kind
                    && e.source_name.as_deref().unwrap_or(&e.name) == item.name
            })
        };
        // Only the directory of an installed item is protected.
        source_items.retain(|item| owner(item).is_some());
        let Some((item, out, rec)) =
            super::output::find_recorded_overlap(target, &recorded, &source_items)
        else {
            continue;
        };
        let other = owner(item).map_or(item.name.as_str(), |e| e.name.as_str());
        anyhow::bail!(
            "the source directory ({}) of the installed {} '{other}' contains the recorded \
             path {} of {} '{}'; removing {} '{}' would delete the source of {} '{other}'. \
             Remove {} '{other}' first",
            item.dir.display(),
            item.kind,
            out.display(),
            rec.kind,
            rec.name,
            rec.kind,
            rec.name,
            item.kind,
            item.kind
        );
    }
    Ok(())
}

/// What `remove` does with the recorded files of one item, computed before
/// any change on disk.
struct RemovalPlan {
    kind: ItemKind,
    name: String,
    delete: Vec<PathBuf>,
    absent: Vec<PathBuf>,
    kept: Vec<PathBuf>,
    /// Modified files in `delete`, deleted because `force` is true.
    forced: Vec<PathBuf>,
}

/// Classify every recorded file of `entry` and decide its action. Fails on
/// a symbolic link in the item part of a recorded path's parents.
fn plan_removal(
    target: &Path,
    entry: &crate::lockfile::LockedItem,
    force: bool,
) -> Result<RemovalPlan> {
    check_item_records(target, entry)?;
    let mut plan = RemovalPlan {
        kind: entry.kind,
        name: entry.name.clone(),
        delete: Vec::new(),
        absent: Vec::new(),
        kept: Vec::new(),
        forced: Vec::new(),
    };
    // The `pending` files of an operation that did not complete are
    // recorded files too: a failed run may have written them.
    for file in entry.files.iter().chain(&entry.pending) {
        let Some(rel) = deletable(&file.path) else {
            continue;
        };
        check_no_item_symlink_above(target, &rel)?;
        let recorded: Vec<&str> = entry
            .files
            .iter()
            .chain(&entry.pending)
            .filter(|f| f.path == file.path)
            .map(|f| f.sha256.as_str())
            .collect();
        let state = match classify(target, &file.path, &recorded) {
            Ok(state) => state,
            // A regular file that exists but cannot be read is handled as
            // a modified file. Any other lookup error fails.
            Err(_)
                if fs::symlink_metadata(target.join(&rel))
                    .is_ok_and(|m| m.file_type().is_file()) =>
            {
                FileState::Modified
            }
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("cannot inspect the recorded file {}", file.path));
            }
        };
        match state {
            FileState::Owned => plan.delete.push(rel),
            FileState::Absent => plan.absent.push(rel),
            FileState::Modified | FileState::Unowned => {
                let full = target.join(&rel);
                let is_file = fs::symlink_metadata(&full)
                    .with_context(|| format!("inspecting {}", full.display()))?
                    .file_type()
                    .is_file();
                if force && is_file {
                    plan.forced.push(rel.clone());
                    plan.delete.push(rel);
                } else {
                    plan.kept.push(rel);
                }
            }
        }
    }
    plan.delete.sort();
    plan.delete.dedup();
    plan.kept.sort();
    plan.kept.dedup();
    // A path in both `files` and `pending` is classified twice.
    plan.forced.sort();
    plan.forced.dedup();
    plan.absent.sort();
    plan.absent.dedup();
    Ok(plan)
}

/// Deletes the files that `plan` marks for deletion and removes the
/// directories left empty, also above the absent recorded files. Returns
/// the deleted paths that are not already in `reported`, and adds them.
fn execute_removal(
    target: &Path,
    plan: &RemovalPlan,
    reported: &mut std::collections::BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>> {
    let mut deleted_files = Vec::new();
    for rel in &plan.delete {
        delete_recorded(target, rel)?;
        if reported.insert(rel.clone()) {
            deleted_files.push(rel.clone());
        }
    }
    for rel in &plan.absent {
        prune_empty_parents(target, rel);
    }
    Ok(deleted_files)
}

/// The part of `remove` that runs after adoption: plan every selected
/// item, then delete, then clean up bundles and plugins and save.
fn remove_selected(
    at: &ScopeRoot,
    lock: &mut crate::lockfile::Lockfile,
    filter: &RemoveFilter,
    keys: &[(ItemKind, String)],
    force: bool,
) -> Result<RemoveReport> {
    let target = at.root.as_path();
    let mut plans = Vec::with_capacity(keys.len());
    for (kind, name) in keys {
        let Some(entry) = lock
            .items
            .iter()
            .find(|i| i.kind == *kind && &i.name == name)
        else {
            continue;
        };
        plans.push(plan_removal(target, entry, force)?);
    }

    // The rules whose opencode entry this removal deletes.
    let opencode_rules: std::collections::BTreeSet<String> = keys
        .iter()
        .filter_map(|(kind, name)| {
            lock.items
                .iter()
                .find(|i| i.kind == *kind && &i.name == name)
        })
        .filter(|i| i.is_opencode_rule())
        .map(|i| i.name.clone())
        .collect();

    let mut report = RemoveReport::default();
    // A path recorded by two removed items is reported once, for the first.
    let mut reported = std::collections::BTreeSet::new();
    for plan in plans {
        let deleted_files = execute_removal(target, &plan, &mut reported)?;
        lock.remove(plan.kind, &plan.name);
        report.items.push(RemovedItem {
            kind: plan.kind,
            name: plan.name,
            deleted_files,
            kept_modified: plan.kept,
        });
    }

    // Clean up plugins associated with removed bundles.
    let bundles_to_remove: Vec<String> = match filter {
        RemoveFilter::ByNames(names) => lock
            .bundles
            .iter()
            .filter(|b| b.items.iter().any(|item| names.contains(item)))
            .map(|b| b.name.clone())
            .collect(),
        RemoveFilter::BySource(source) => lock
            .bundles
            .iter()
            .filter(|b| &b.source == source)
            .map(|b| b.name.clone())
            .collect(),
    };

    for plugin in lock
        .plugins
        .iter()
        .filter(|p| bundles_to_remove.contains(&p.bundle))
    {
        if plugin.client == "opencode"
            && plugin.status == crate::lockfile::PluginInstallStatus::Installed
        {
            let _ = crate::ancillary::remove_opencode_plugin_uri(
                target,
                crate::plugin::PluginScope::of(at.scope),
                &plugin.identifier,
            );
        }
    }
    lock.plugins
        .retain(|p| !bundles_to_remove.contains(&p.bundle));
    lock.bundles
        .retain(|b| !bundles_to_remove.contains(&b.name));
    lock.save(target)?;
    drop_global_opencode_rules(at.scope, target, &opencode_rules);
    Ok(report)
}

/// In the global scope, delete from the global opencode config the entries of
/// the rules in `names`. Nothing happens in the project scope, and the file is
/// not read when `names` is empty. An error is ignored: the lockfile is
/// already saved, so the removal succeeds. When the file cannot be read as
/// JSON, or `instructions` is not an array, the entry of the removed rule stays
/// in the file, and the user deletes it by hand.
fn drop_global_opencode_rules(
    scope: Scope,
    root: &Path,
    names: &std::collections::BTreeSet<String>,
) {
    if scope == Scope::Global {
        let _ = crate::ancillary::sync_opencode_global_rules(
            root,
            &std::collections::BTreeSet::new(),
            names,
        );
    }
}

/// The `upskill add` command that installs again the plugins of `bundle`,
/// when `update` cannot. There are two cases. The lockfile records the
/// bundle's source, but no item from it, and `update` runs over the sources
/// of the recorded items. Or the source is under a kind restriction
/// ([`source_is_kind_restricted`]), for which `update` runs no plugin step
/// and no plugin repair; the command has no `--kind`, so it also removes the
/// kind restriction of the bundle. `None` in every other case, and when the
/// bundle is not recorded.
fn add_command_for_bundle(
    at: &ScopeRoot,
    lock: &crate::lockfile::Lockfile,
    bundle: &str,
) -> Option<String> {
    let recorded = lock.bundles.iter().find(|b| b.name == bundle)?;
    if lock.items.iter().any(|i| i.source == recorded.source)
        && !source_is_kind_restricted(lock, &recorded.source)
    {
        return None;
    }
    let label = recorded.source.as_str();
    let source = label
        .strip_prefix("local:")
        .or_else(|| label.strip_prefix("github:"))
        .unwrap_or(label);
    let name = if label.ends_with(crate::parse::bundle::BUNDLE_SUFFIX) {
        String::new()
    } else {
        format!(" {bundle}")
    };
    Some(format!("upskill add {source}{name} --{}", at.scope))
}

/// Whether `update` applies a kind restriction to `source`, by
/// [`source_kind_restriction`], with the dependency bundles that the lockfile
/// records (a bundle whose `required_by` chain reaches a recorded root
/// bundle). A full `update` that reads the source of every root uses the same
/// bundles.
fn source_is_kind_restricted(lock: &crate::lockfile::Lockfile, source: &str) -> bool {
    !source_kind_restriction(
        lock,
        source,
        |bundle| !lock.root_bundles_of(bundle).is_empty(),
        |bundle| {
            lock.root_bundles_of(bundle)
                .iter()
                .any(|root| root.source == source)
        },
    )
    .is_empty()
}

/// The kinds that `update` restricts `source` to; empty for no restriction
/// (#648). The kinds come from the root bundles of the source (its recorded
/// bundles for which `is_dependency` is false): a restriction applies only
/// when each of them has recorded kinds (`add --kind`), and is their union.
/// The recorded kinds of each dependency bundle that these roots reach
/// (`reached_from_source`) are added. A dependency bundle without recorded
/// kinds adds none, so it does not lift the restriction of its roots. With one
/// unrestricted root bundle, or no root bundle, no restriction applies.
fn source_kind_restriction(
    lock: &crate::lockfile::Lockfile,
    source: &str,
    is_dependency: impl Fn(&crate::lockfile::LockedBundle) -> bool,
    reached_from_source: impl Fn(&crate::lockfile::LockedBundle) -> bool,
) -> Vec<ItemKind> {
    let roots: Vec<_> = lock
        .bundles
        .iter()
        .filter(|bundle| bundle.source == source && !is_dependency(bundle))
        .collect();
    if roots.is_empty() || roots.iter().any(|bundle| bundle.kinds.is_empty()) {
        return Vec::new();
    }
    roots
        .iter()
        .flat_map(|bundle| bundle.kinds.iter().copied())
        .chain(
            lock.bundles
                .iter()
                .filter(|bundle| reached_from_source(bundle))
                .flat_map(|bundle| bundle.kinds.iter().copied()),
        )
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Verify installed-state consistency against `.upskill-lock.json`.
/// The report has these buckets:
/// - **missing outputs** — file paths the lockfile says should exist
///   but don't: every recorded path that is absent, except an absent
///   recorded path at an earlier global path (it is listed under earlier
///   layout instead), or, for an item with an empty `files` record, each
///   per-client entrypoint that is absent. A recorded path that `deletable`
///   rejects is skipped without a report, here and in the modified and
///   unreadable files.
///   Reinstall (`upskill add <source>`) fixes it.
/// - **modified files** — recorded files whose content does not match the
///   recorded hash through `matches_recorded` (as it is, or with every CRLF
///   replaced by LF), or that are no longer regular files.
/// - **unreadable files** — recorded files that could not be inspected.
///   `doctor` continues with the other files and items.
/// - **stale hashes** — for `local:` sources still on disk, the SSOT
///   item directory hashes to a value that does not match the lockfile.
///   `upskill update` (or `--dry-run`) fixes it.
/// - **orphan entries** — the lockfile points at a `local:` source that
///   is gone, at a source directory whose items cannot be listed, or at
///   an item that no longer exists in its source. The user has to
///   `upskill remove` to clear it.
/// - **orphaned dependencies** — items pulled in only as a dependency of
///   items that are no longer installed. Does not affect `is_clean`.
/// - **missing plugins** — plugins recorded as installed that the client
///   no longer has.
/// - **skipped plugins** (advisory) — plugins that were never installed
///   because the client CLI was missing, or opencode was below version 2 or
///   gave no version.
/// - **MCP entries** — MCP server reconciliation against each client.
/// - **interrupted** — items with a non-empty `pending` record: an `add` or
///   `update` failed or was stopped between its first and its last write.
///   Running the same command again completes it.
/// - **earlier layout** — for the global scope only: files at the Copilot
///   paths that a released version wrote, where the current version writes
///   to the personal Copilot directories. A path is reported when the
///   item's record names it, or, for an item without a record, when a file
///   exists at it. `upskill update --global` moves them. A path in this
///   bucket is not also reported as a missing output.
/// - **missing rule entries** — for the global scope only: a rule recorded for
///   opencode whose absolute path is not in the `instructions` array of the
///   global opencode config. A config that is not strict JSON (for example
///   with comments) reports every such rule, and `update` cannot repair it. A
///   config without an `instructions` array is repaired by `update`; a value
///   that is not an array is not.
/// - **stale rule entries** — for the global scope only: an entry of the form
///   `<home>/.agents/rules/<name>/RULE.md` in the `instructions` array of the
///   global opencode config, for a name that the lockfile does not record as
///   an opencode rule. When the lockfile records the rule for other clients
///   (`recorded: true`), `upskill add --global` and `upskill update --global`
///   delete the entry. An entry for a name of which the lockfile records no
///   rule (`recorded: false`), for example one left by a failed removal, is
///   reported and kept; the user deletes it by hand. Entries in another form
///   are not reported.
///
/// Doctor never adopts a `schema: 1` lockfile and never saves. Doctor
/// never fetches. Remote-source drift is detected by `update --check`, which
/// does not fetch, and by `update --dry-run`, which does fetch.
pub fn doctor(at: &ScopeRoot) -> Result<DoctorReport> {
    let target = at.root.as_path();
    let lock = crate::lockfile::Lockfile::load(target)?;
    let mut report = DoctorReport::default();

    // Items found by discovery, per source root (see the `local:` check).
    // A scan that fails keeps its error message.
    let mut source_items: std::collections::HashMap<
        PathBuf,
        std::result::Result<Vec<super::output::SourceItem>, String>,
    > = std::collections::HashMap::new();
    for entry in &lock.items {
        let kind = entry.kind;
        if !entry.pending.is_empty() {
            report.interrupted.push(InterruptedItem {
                kind,
                name: entry.name.clone(),
            });
        }

        let mut missing = Vec::new();
        let mut modified = Vec::new();
        let mut unreadable = Vec::new();
        let mut earlier = Vec::new();
        if entry.files.is_empty() {
            // No record (a `schema: 1` lockfile, which `doctor` does not
            // adopt, or an empty record): check the entrypoint of each
            // layout path.
            for client in ALL_CLIENTS {
                // Only check clients this item was actually written for. An
                // empty `clients` list means "all" — pre-ADR-0012 lockfiles
                // and unrestricted installs both check every client. A
                // narrowed install's unselected outputs are a deliberate
                // choice, not drift.
                if !entry.clients.is_empty() && !entry.clients.iter().any(|c| c == client.name()) {
                    continue;
                }
                let released = released_output_path(kind, client, &entry.name);
                let current = output_path(at.scope, kind, client, &entry.name);
                // A released version wrote the released path. A file there
                // is at an earlier path when the current path differs.
                if released != current && target.join(&released).exists() {
                    earlier.push(released);
                } else if !target.join(&current).exists() {
                    missing.push(current);
                }
            }
        } else {
            let earlier_paths = earlier_layout_paths(at.scope, kind, &entry.name);
            for file in &entry.files {
                let Some(rel) = deletable(&file.path) else {
                    continue;
                };
                if earlier_paths.contains(&rel) {
                    earlier.push(rel.clone());
                }
                let recorded: Vec<&str> = entry
                    .files
                    .iter()
                    .chain(&entry.pending)
                    .filter(|f| f.path == file.path)
                    .map(|f| f.sha256.as_str())
                    .collect();
                match classify(target, &file.path, &recorded) {
                    Ok(FileState::Absent) if earlier_paths.contains(&rel) => {}
                    Ok(FileState::Absent) => missing.push(rel),
                    Ok(FileState::Modified) => modified.push(rel),
                    Ok(FileState::Owned | FileState::Unowned) => {}
                    Err(err) => {
                        unreadable.push((rel, format!("{err:#}")));
                    }
                }
            }
            missing.sort();
            missing.dedup();
            modified.sort();
            modified.dedup();
        }
        earlier.sort();
        earlier.dedup();
        if !earlier.is_empty() {
            report.earlier_layout.push(EarlierLayoutFile {
                kind,
                name: entry.name.clone(),
                files: earlier,
            });
        }
        if !missing.is_empty() {
            report.missing_outputs.push(MissingOutput {
                kind,
                name: entry.name.clone(),
                missing_files: missing,
            });
        }
        if !modified.is_empty() {
            report.modified_files.push(ModifiedFile {
                kind,
                name: entry.name.clone(),
                files: modified,
            });
        }
        unreadable.sort();
        unreadable.dedup_by(|a, b| a.0 == b.0);
        if let Some((_, error)) = unreadable.first() {
            report.unreadable_files.push(UnreadableFile {
                kind,
                name: entry.name.clone(),
                error: error.clone(),
                files: unreadable.into_iter().map(|(rel, _)| rel).collect(),
            });
        }

        if let Some(local_path) = entry.source.strip_prefix("local:") {
            // A bundle-file source (issue #602): the items are in the
            // registry root that `add` resolves for the bundle file. A root
            // that cannot be resolved is reported as "local path gone".
            let ssot_root = match local_bundle_root(Path::new(local_path)) {
                Some(Ok(root)) => root,
                Some(Err(_)) => PathBuf::new(),
                None => PathBuf::from(local_path),
            };
            if !ssot_root.is_dir() {
                report.orphan_entries.push(OrphanEntry {
                    kind,
                    name: entry.name.clone(),
                    source: entry.source.clone(),
                    reason: OrphanReason::LocalPathGone,
                    error: None,
                });
                continue;
            }
            // On the SOURCE side an item lives in its FOLDER directory, which
            // can differ from the consumer-side effective `name` (via `--as`
            // aliasing, or relaxed rule/agent naming where the folder name and
            // the `name:` field diverge). Resolve the source dir from the
            // recorded folder key: `group` (the canonical source folder) ->
            // `source_name` (the original effective name, set when aliased) ->
            // `name`. See #208.
            let folder = entry
                .group
                .as_deref()
                .or(entry.source_name.as_deref())
                .unwrap_or(&entry.name);
            let direct = ssot_root.join(folder);
            let item_dir = if super::discovery::is_item_dir(&direct) {
                Some(direct)
            } else {
                // The item can be in a child or grandchild directory of the
                // root (`skills/<folder>`); `add` discovers items at those
                // depths. Candidates have the entry's kind and a directory
                // named `folder`. A sole candidate is the item; among several,
                // the one whose name is the entry's source name; several
                // without such a match leave the item missing. The scan runs
                // at most once per source root.
                let found = source_items.entry(ssot_root.clone()).or_insert_with(|| {
                    let mut found = Vec::new();
                    let none = crate::bundle::ResolvedItems::default();
                    let names = super::plan::PlanNames::default();
                    super::install::collect_source_items(
                        &ssot_root,
                        Some(&none),
                        &names,
                        &mut found,
                    )
                    .map(|()| found)
                    .map_err(|e| format!("{e:#}"))
                });
                let found = match found {
                    Ok(found) => found,
                    Err(error) => {
                        report.orphan_entries.push(OrphanEntry {
                            kind,
                            name: entry.name.clone(),
                            source: entry.source.clone(),
                            reason: OrphanReason::SourceUnreadable,
                            error: Some(error.clone()),
                        });
                        continue;
                    }
                };
                let candidates: Vec<_> = found
                    .iter()
                    .filter(|i| i.kind == kind && i.dir.file_name().is_some_and(|n| n == folder))
                    .collect();
                let source_name = entry.source_name.as_deref().unwrap_or(&entry.name);
                match candidates.as_slice() {
                    [only] => Some(only.dir.clone()),
                    _ => candidates
                        .iter()
                        .find(|i| i.name == source_name)
                        .map(|i| i.dir.clone()),
                }
            };
            let Some(item_dir) = item_dir else {
                report.orphan_entries.push(OrphanEntry {
                    kind,
                    name: entry.name.clone(),
                    source: entry.source.clone(),
                    reason: OrphanReason::ItemMissingInSource,
                    error: None,
                });
                continue;
            };
            let current = hash_item_dir(&item_dir);
            if current != entry.hash {
                report.stale_hashes.push(StaleHash {
                    kind,
                    name: entry.name.clone(),
                    source: entry.source.clone(),
                    stored_hash: entry.hash.clone(),
                    current_hash: current,
                });
            }
        }
        // Non-local sources: doctor only validates per-client outputs.
        // Hash comparison would require a network fetch — out of scope
        // here, see `update --check` and `update --dry-run`.
    }

    // -- Global opencode rule entries (ADR-0008, issue #581) --
    // A rule installed for opencode in the global scope is listed in the
    // `instructions` array of the global opencode config. An entry that is
    // missing is drift: opencode does not load the rule.
    //
    // An entry of the form that upskill writes, for a name that the lockfile
    // does not record as an opencode rule, is left from a removal that failed
    // after the lockfile was saved (or was written by hand in that form). It
    // is reported with `recorded` set when the lockfile records the rule for
    // other clients; `update --global` deletes only those entries. Entries in
    // another form are not upskill's and are not reported. The config is read
    // once (issue #596).
    if at.scope == Scope::Global {
        let config =
            crate::ancillary::opencode_config_dir(crate::plugin::PluginScope::User, target)
                .join("opencode.json");
        let listed = crate::ancillary::opencode_global_rule_names(target);
        let opencode_rules = lock.opencode_rule_names();
        let other_rules = lock.other_client_rule_names();
        for name in &opencode_rules {
            if !listed.contains(name) {
                report.missing_rule_entries.push(MissingRuleEntry {
                    name: name.clone(),
                    client: "opencode".to_string(),
                    config: config.clone(),
                });
            }
        }
        for name in listed.difference(&opencode_rules) {
            report.stale_rule_entries.push(StaleRuleEntry {
                name: name.clone(),
                config: config.clone(),
                recorded: other_rules.contains(name),
            });
        }
    }

    // -- Orphaned dependencies (advisory, issue #196) --
    // An item pulled in only as a dependency (`required_by` non-empty) whose
    // every recorded requirer is no longer installed. Surfaced as advisory
    // only — upskill never auto-removes; the user decides.
    for entry in &lock.items {
        if lock.is_orphaned_dependency(entry) {
            report.orphaned_dependencies.push(OrphanedDependency {
                kind: entry.kind,
                name: entry.name.clone(),
                former_requirers: entry.required_by.clone(),
            });
        }
    }

    // -- Plugin reconciliation (ADR-0008 / issue #151) --
    // Walk the lockfile's plugin entries and reconcile against each client's
    // installed plugin list.
    //
    // Two buckets:
    // - skipped_plugins: status == Skipped (CLI was absent, or opencode was
    //   below version 2, at install time).
    //   Always surface; does not affect is_clean().
    // - missing_plugins: status == Installed but the client no longer has the
    //   plugin.  This is drift → is_clean() returns false → exit 1.
    for plugin in &lock.plugins {
        use crate::lockfile::PluginInstallStatus;

        match &plugin.status {
            PluginInstallStatus::Skipped | PluginInstallStatus::Instructions => {
                // Plugin was never installed because the CLI was missing or
                // requires manual instructions.
                // Report it so it is not silently ignored.
                report.skipped_plugins.push(SkippedPlugin {
                    name: plugin.name.clone(),
                    client: plugin.client.clone(),
                    identifier: plugin.identifier.clone(),
                    bundle: plugin.bundle.clone(),
                    repair_command: add_command_for_bundle(at, &lock, &plugin.bundle),
                });
            }
            PluginInstallStatus::Installed => {
                if plugin_is_missing(target, crate::plugin::PluginScope::of(at.scope), plugin) {
                    report.missing_plugins.push(MissingPlugin {
                        name: plugin.name.clone(),
                        client: plugin.client.clone(),
                        identifier: plugin.identifier.clone(),
                        bundle: plugin.bundle.clone(),
                        repair_command: add_command_for_bundle(at, &lock, &plugin.bundle),
                    });
                }
            }
        }
    }

    // -- MCP server reconciliation (ADR-0010, issue #237, #241) --
    // Walk the lockfile's MCP entries and check whether each is still
    // configured for its target. Claude is queried via `claude mcp list`; the
    // config-write targets (copilot, vscode, opencode) are reconciled against
    // their config file. A present-but-missing entry is drift; an absent or
    // unreadable config file is surfaced as an informational "cannot verify"
    // state rather than drift, so it never produces a false-positive exit 1.
    for mcp in &lock.mcps {
        use crate::ancillary::McpConfigState;
        use crate::pipeline::report::{McpDoctorEntry, McpDoctorStatus};

        let status = match mcp.client.as_str() {
            "claude" => {
                use crate::plugin::PluginCheckResult;
                match crate::mcp::check_claude_installed(&mcp.name) {
                    PluginCheckResult::Installed => McpDoctorStatus::Ok,
                    PluginCheckResult::NotInstalled => McpDoctorStatus::NotRegistered,
                    PluginCheckResult::CliNotFound => McpDoctorStatus::CliNotFound,
                    PluginCheckResult::QueryFailed { stderr, .. } => {
                        McpDoctorStatus::QueryFailed { stderr }
                    }
                }
            }
            "copilot" | "vscode" | "opencode" => {
                let state = match mcp.client.as_str() {
                    "copilot" => crate::ancillary::copilot_mcp_state(&mcp.name),
                    "vscode" => crate::ancillary::vscode_mcp_state(target, &mcp.name),
                    _ => crate::ancillary::opencode_mcp_state(
                        target,
                        crate::plugin::PluginScope::of(at.scope),
                        &mcp.name,
                    ),
                };
                match state {
                    McpConfigState::Present => McpDoctorStatus::Ok,
                    McpConfigState::Missing => McpDoctorStatus::NotRegistered,
                    McpConfigState::FileAbsent => McpDoctorStatus::ConfigMissing,
                    McpConfigState::Unreadable(detail) => {
                        McpDoctorStatus::ConfigUnreadable { detail }
                    }
                }
            }
            _ => continue,
        };
        report.mcp_entries.push(McpDoctorEntry {
            name: mcp.name.clone(),
            client: mcp.client.clone(),
            bundle: mcp.bundle.clone(),
            status,
        });
    }

    Ok(report)
}

/// The entrypoint paths of `(kind, name)` that a released version wrote and
/// that the current version writes elsewhere in `scope`. Empty for the project
/// scope.
fn earlier_layout_paths(scope: Scope, kind: ItemKind, name: &str) -> Vec<PathBuf> {
    ALL_CLIENTS
        .into_iter()
        .map(|client| {
            (
                released_output_path(kind, client, name),
                output_path(scope, kind, client, name),
            )
        })
        .filter(|(released, current)| released != current)
        .map(|(released, _)| released)
        .collect()
}

/// [`doctor`] for each given scope root, in the given order, and the items
/// that the lockfiles of two of the roots both record.
pub fn doctor_scopes(roots: &[ScopeRoot]) -> Result<DoctorScopes> {
    let mut scopes = Vec::with_capacity(roots.len());
    let mut seen_in: std::collections::BTreeMap<(ItemKind, String), usize> =
        std::collections::BTreeMap::new();
    for at in roots {
        let report =
            doctor(at).with_context(|| format!("failed to check the {} scope", at.scope))?;
        let lock = crate::lockfile::Lockfile::load(&at.root)
            .with_context(|| format!("failed to check the {} scope", at.scope))?;
        let keys: std::collections::BTreeSet<(ItemKind, String)> = lock
            .items
            .iter()
            .map(|i| (i.kind, i.name.clone()))
            .collect();
        for key in keys {
            *seen_in.entry(key).or_insert(0) += 1;
        }
        scopes.push(ScopedDoctor {
            scope: at.scope,
            root: at.root.clone(),
            report,
        });
    }
    let in_both_scopes = seen_in
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|((kind, name), _)| BothScopesItem { kind, name })
        .collect();
    Ok(DoctorScopes {
        scopes,
        in_both_scopes,
        support: crate::support::SUPPORT.to_vec(),
    })
}

/// Rebuild the client selection a bare `update` should reuse for one source:
/// the union of its entries' recorded generation clients. An entry with an
/// empty `clients` list was installed for all clients, so it widens the result
/// back to all — the safe direction (never drops an entry's output trees).
/// Returns [`ClientSelection::all`](crate::select::ClientSelection::all) when nothing narrows it (ADR-0012).
fn reselect_from_entries(
    entries: &[crate::lockfile::LockedItem],
) -> crate::select::ClientSelection {
    use crate::select::{ClientSelection, SelectedClient};
    let mut set: std::collections::BTreeSet<SelectedClient> = std::collections::BTreeSet::new();
    for entry in entries {
        if entry.clients.is_empty() {
            return ClientSelection::all();
        }
        for name in &entry.clients {
            if let Ok(sc) = name.parse::<SelectedClient>() {
                set.insert(sc);
            }
        }
    }
    if set.is_empty() {
        ClientSelection::all()
    } else {
        ClientSelection::restrict(&set.into_iter().collect::<Vec<_>>())
    }
}

/// True when the client reports that `plugin`, recorded as installed, is
/// not installed. Used by `doctor` and by the plugin repair of `update`.
///
/// An opencode plugin whose identifier is in the `plugin` or `plugins` array
/// of the opencode config file of `file_scope`, the scope of the lockfile, is
/// installed, and no opencode command is run for it. An opencode plugin recorded with
/// `config_write` and absent from that array is missing, also without any
/// opencode command. Otherwise the client CLI is queried. A client CLI that is
/// absent, that has an unsupported version, or whose query fails gives `false`: the
/// state cannot be checked, and a false report of a missing plugin is
/// avoided. An unknown client gives `false`.
pub(super) fn plugin_is_missing(
    target: &Path,
    file_scope: crate::plugin::PluginScope,
    plugin: &crate::lockfile::LockedPlugin,
) -> bool {
    use crate::plugin::{
        PluginScope, check_claude_plugin_installed, check_opencode_plugin_installed,
        check_vscode_extension_installed,
    };

    let check = match plugin.client.as_str() {
        "claude" => {
            let scope = match plugin.scope.as_deref() {
                Some("user") => PluginScope::User,
                _ => PluginScope::Project,
            };
            check_claude_plugin_installed(&plugin.name, scope)
        }
        "vscode" => check_vscode_extension_installed(&plugin.identifier),
        "opencode" => {
            let listed =
                crate::ancillary::opencode_plugin_listed(target, file_scope, &plugin.identifier);
            // A plugin installed by a write to `opencode.json` is checked in
            // that file only, with any opencode version or none.
            if listed || plugin.config_write {
                return !listed;
            }
            check_opencode_plugin_installed(&plugin.identifier)
        }
        _ => return false,
    };
    check.is_not_installed()
}

/// Install again every plugin of `bundles` that the lockfile records as
/// skipped, or as installed when [`plugin_is_missing`] reports it missing,
/// and record the results in the lockfile. `update` calls it for a source
/// that it does not apply because its preview changes nothing; the plugin
/// step of an applied source attempts every plugin itself. Only the missing or skipped client of a plugin is installed.
/// Returns the results; empty when no plugin is missing or skipped, and then
/// nothing is written.
fn repair_missing_plugins(
    target: &Path,
    plugin_scope: crate::plugin::PluginScope,
    bundles: &[crate::model::Bundle],
) -> Result<Vec<super::PluginResult>> {
    use crate::lockfile::PluginInstallStatus;

    if bundles.is_empty() {
        return Ok(Vec::new());
    }
    let mut lock = crate::lockfile::Lockfile::load(target)?;
    let mut reduced = Vec::new();
    for bundle in bundles {
        let missing: Vec<(String, String)> = lock
            .plugins
            .iter()
            .filter(|p| p.bundle == bundle.name)
            .filter(|p| match p.status {
                // A skipped plugin is tried again: its client CLI may be on
                // PATH now (issue #229).
                PluginInstallStatus::Skipped => true,
                PluginInstallStatus::Installed => plugin_is_missing(target, plugin_scope, p),
                PluginInstallStatus::Instructions => false,
            })
            .map(|p| (p.name.clone(), p.client.clone()))
            .collect();
        if missing.is_empty() {
            continue;
        }
        let is_missing =
            |name: &str, client: &str| missing.iter().any(|(n, c)| n == name && c == client);
        let mut repaired = bundle.clone();
        repaired.plugins = bundle
            .plugins
            .iter()
            .filter_map(|(name, entry)| {
                let mut entry = entry.clone();
                if !is_missing(name, "claude") {
                    entry.claude = None;
                }
                if !is_missing(name, "copilot") {
                    entry.copilot = None;
                }
                if !is_missing(name, "vscode") {
                    entry.vscode = None;
                }
                if !is_missing(name, "opencode") {
                    entry.opencode = None;
                }
                let empty = entry.claude.is_none()
                    && entry.copilot.is_none()
                    && entry.vscode.is_none()
                    && entry.opencode.is_none();
                (!empty).then(|| (name.clone(), entry))
            })
            .collect();
        reduced.push(repaired);
    }
    if reduced.is_empty() {
        return Ok(Vec::new());
    }
    let results = super::install::install_plugins_from_bundles(&reduced, plugin_scope, target);
    super::prepared::record_plugin_results(&mut lock, &results, plugin_scope);
    lock.save(target)?;
    Ok(results)
}

/// Re-fetch every source recorded in `<target>/.upskill-lock.json` and
/// handle the sources one at a time, sorted by source label. `update`
/// always fetches per ADR-0004 — there is no `--offline`. `update --check`
/// and `update --notice` do not call this function: they fetch nothing and
/// change no installed file.
///
/// `names` selects which lockfile entries to update; empty means "all".
/// When names are given, the entries' source labels are used to fetch
/// — so `update foo` may also re-render other items installed from the
/// same source (the pipeline always reinstalls a source as a unit).
/// Names that match no lockfile entry are an error.
///
/// First, every source is fetched once (a source of a cross-source
/// dependency is fetched again when the reinstall of the source that
/// requires it resolves it), resolved, checked (conflict check and overlap
/// check) and planned, without any write (`prepare_install`), in the order
/// of the sources. Then the overlap check runs once more on the items of all
/// these sources together, and on the item directories of every local-path
/// source recorded in the lockfile (`lock.bundles` and `lock.items`) that the
/// run does not otherwise read (see `collect_recorded_local_items`): a
/// recorded directory that does not exist is skipped, a recorded bundle file,
/// also one that does not exist, protects the registry root that `add`
/// resolves for it when that root exists, a recorded source that exists and
/// cannot be read as a directory is an error that names its label (its hint
/// names `upskill remove --source` only when a lockfile item has that
/// source), and a git source is not
/// fetched for this check. The same item directories are then compared with
/// every recorded file (`files` and `pending`, also a modified file that the
/// removal keeps) of an item that its source no longer contains
/// (`output::check_outside_recorded_paths`); an item directory that contains
/// one fails the run before any write, also with `overwrite` and `dry_run`.
///
/// A source whose prepare fails with a [`FetchFailed`] in its error chain
/// (a `git clone` that fails, or a local-path source whose path does not
/// exist, of the source itself or of a cross-source dependency source that
/// it requires) is skipped: it is not prepared, none of its files is
/// written, its lockfile entries do not change except the `files` record
/// that the whole-lockfile adoption of a `schema: 1` lockfile writes, it is
/// in [`UpdateReport::skipped_sources`], and each of its selected entries is
/// reported as [`UpdateStatus::SourceSkipped`] with its [`SkipCause`]. A
/// prepared source whose plan writes an item that the lockfile records from
/// a skipped source is skipped too, until no further source is dropped.
/// [`UpdateObserver::interrupted`] is read before each source, after a fetch
/// failure and before that comparison; once it is true, no further source
/// is fetched, no further skip is recorded, and the run fails with the
/// skips recorded before it in [`SkippedBeforeFailure`]. The overlap check
/// treats a skipped source as a recorded source that the run does not read. Any other
/// error in this step stops the run before the adoption and before any
/// source is applied.
///
/// Then, for each source, in this order:
///
/// 1. The prepared install of the source is taken from the first step.
/// 2. A source that resolves to no items while the lockfile records items
///    from it is an error (`guard_against_empty_source`), before any write
///    of the source.
/// 3. A `schema: 1` lockfile is adopted (`adopt_schema1`) and the adopted
///    paths are in [`SourcePreview::adopted`] of this source. With
///    `dry_run`, the adoption is done on a copy of the lockfile in memory,
///    which is never saved, and no backup file is written.
/// 4. The [`SourcePreview`] is computed from the disk and the lockfile:
///    the planned items through `PreparedInstall::preview`, and each
///    lockfile entry of the source that the source no longer contains
///    through `plan_removal`, as a [`PreviewAction::Remove`] item.
/// 5. `observer.preview` decides. With `dry_run`, the decision is ignored
///    and nothing is applied.
/// 6. On [`SourceDecision::Apply`]: `PreparedInstall::commit` decides the
///    plan again from the disk and writes it, then the removed items are
///    planned again from the saved lockfile and deleted with the rules of
///    [`remove`], then `observer.applied` receives the counts. On
///    [`SourceDecision::Stop`]: [`UpdateReport::stopped`] is set and no
///    later source is previewed or applied.
///
/// In an applied source, `overwrite` (`update --force`) replaces modified
/// and unowned files. Without it, an item with such a file at one of its
/// destinations is not written. When a source hash is recorded for it and
/// its source hash equals it, it has no `pending` record, every such file
/// is modified (none is unowned), nothing else of the item would be
/// written or deleted, no stale file of the item would be deleted or kept,
/// and no field of its lockfile record other than `files`, `pending`,
/// `required_by` and `commit` would change (`apply::same_record`), the modified
/// files are listed in `kept_modified` and the item is reported as
/// [`UpdateStatus::UpToDate`]; otherwise it is reported as
/// [`UpdateStatus::SkippedModified`]. A modified file that the new version
/// no longer contains is kept and listed in `kept_modified`. An item that
/// its source no longer contains is removed with the rules of [`remove`]:
/// its unchanged recorded files are deleted, its modified files are kept
/// (deleted with `overwrite` when they are regular files) and listed in
/// `kept_modified`, and its lockfile entry is dropped. An item with a
/// non-empty `pending` record whose source hash did not change is reported
/// as [`UpdateStatus::CompletedInterrupted`], never as
/// [`UpdateStatus::UpToDate`].
///
/// After the decision, a git source without changes
/// ([`SourcePreview::has_changes`] is false) that is not applied, in a run
/// that is not a dry run and not stopped, gets the fetched commit in the
/// lockfile entries that the run selected (`names`, or every entry when
/// `names` is empty; `PreparedInstall::record_commit`) except the entry of an
/// item whose update is withheld (a [`PreviewAction::Skip`] item), which keeps
/// its recorded commit. An entry of the source that `names` did not select
/// keeps its recorded commit on this path. In an applied source the
/// reinstall covers the selected entries, the entries that a selected item
/// requires (from this source or from another one). For a directory source
/// with recorded bundles, it reinstalls the members of those bundles and the
/// directly installed items of that source. It also reinstalls each
/// dependency of that source whose requirers are all absent from the
/// lockfile (an orphaned dependency, #613), unless a recorded bundle of the
/// source lists it: a bundle member follows its bundle. It reinstalls no other
/// source item. It applies a kind restriction only when every root bundle of
/// the source (a recorded bundle that is not a dependency bundle, see below)
/// has recorded kinds, using their union together with the recorded kinds of
/// the dependency bundles that these roots reach. With one unrestricted root
/// bundle of the source, no restriction applies; a dependency bundle without
/// recorded kinds adds no kind. Every entry of that source
/// whose kind is outside the restriction is kept unchanged and is not updated.
/// If its source is skipped, the entry is reported as skipped too. To update
/// it, run `upskill add <source> <item>` again. A `.bundle.yaml` source is
/// reinstalled as a whole. Each reinstalled entry gets the commit of its own source,
/// unless its update is skipped; any other entry keeps its recorded commit.
/// Every source without changes that is not applied, in a run that is not
/// a dry run and not stopped, a git source or a local path, also gets
/// `required_by` of every entry refreshed by `record_commit`; on that path
/// the lockfile is saved only when a commit value or a `required_by` value
/// differs. This write is not in [`UpdateReport::changed`] and does not
/// make the source a source with changes. A declined source and a dry run
/// write no commit. In an applied source, an item whose modified
/// destinations are kept (`kept_modified`, see above) also gets the
/// fetched commit.
///
/// In a source that is not applied, an entry without a preview item is
/// [`UpdateStatus::UpToDate`], and an entry with a preview item is
/// [`UpdateStatus::NotApplied`], except a [`PreviewAction::Skip`] item of a
/// source without changes ([`SourcePreview::has_changes`] is false, not a
/// dry run), which is [`UpdateStatus::SkippedModified`]. The notes of
/// [`SourcePreview::kept_notes`] are listed in `kept_modified`, whether or
/// not the source is applied.
///
/// `at` selects the lockfile and the output paths. The reinstall plans the
/// paths of `at.scope`, so an item recorded at other paths (a global Copilot
/// item recorded at its `.github` paths) is written at the new paths and its
/// unchanged files at the old paths are deleted through the ownership record.
/// In a full update (no `names`), a dependency bundle (a recorded bundle with
/// `required_by` whose chain reaches recorded root bundles, all of whose
/// sources have entries in the run) is not resolved from its own record: the
/// roots reach it through `requires` (#648). Its entries leave their source
/// group before the prepare phase, carrying the union of the root sources of
/// every dependency bundle that lists them. After the prepare phase, each
/// joins the group of the first source whose plan provides it, otherwise the
/// group of its first prepared root source, which removes it; it is kept and
/// reported as skipped when a root source was skipped and no plan provides it.
/// An entry outside the kind restriction of its new group is kept and reported
/// as up to date. After each applied source, a requirer that the resolution of
/// the source evaluated and that no longer reaches a dependency bundle leaves
/// its `required_by`, and a dependency bundle that no prepared resolution
/// reaches is dropped once every one of its root sources is applied.
///
/// An error after an adoption carries the adopted paths as
/// [`AdoptedBeforeFailure`]. An error after a source was skipped carries the
/// skipped sources as [`SkippedBeforeFailure`], as the outermost context
/// layer. An error while one source is applied leaves the
/// sources applied before it applied.
pub fn update(
    at: &crate::scope::ScopeRoot,
    names: &[String],
    plugin_scope: crate::plugin::PluginScope,
    selection: &crate::select::ClientSelection,
    overwrite: bool,
    dry_run: bool,
    observer: &mut dyn UpdateObserver,
) -> Result<UpdateReport> {
    let target = at.root.as_path();
    let lock = crate::lockfile::Lockfile::load(target)?;
    // Before the first write of the run, for every item of the lockfile.
    check_lockfile_records(target, &lock)?;

    let entries: Vec<crate::lockfile::LockedItem> = if names.is_empty() {
        lock.items.clone()
    } else {
        let matched: Vec<crate::lockfile::LockedItem> = lock
            .items
            .iter()
            .filter(|i| names.iter().any(|n| n == &i.name))
            .cloned()
            .collect();
        let matched_names: std::collections::BTreeSet<&str> =
            matched.iter().map(|i| i.name.as_str()).collect();
        let unknown: Vec<&str> = names
            .iter()
            .filter(|n| !matched_names.contains(n.as_str()))
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            anyhow::bail!("not in lockfile: {}", unknown.join(", "));
        }
        matched
    };

    // Group by source: installing once per source covers every matching
    // entry sharing it.
    let mut by_source: std::collections::BTreeMap<String, Vec<crate::lockfile::LockedItem>> =
        std::collections::BTreeMap::new();
    for entry in entries {
        by_source
            .entry(entry.source.clone())
            .or_default()
            .push(entry);
    }

    // The items that the run covers: every lockfile entry that the run
    // selects, and below the items that the plans of the sources that were
    // fetched and not skipped install (a source skipped for `RequiresSkipped`
    // is removed from `prepared_sources` before its plan is counted).
    let mut run_items: std::collections::BTreeSet<(ItemKind, String)> = by_source
        .values()
        .flatten()
        .map(|e| (e.kind, e.name.clone()))
        .collect();

    // A dependency bundle (a bundle that recorded root bundles reach through
    // `requires`) is resolved again through its roots, not from its own
    // record (#648). This holds only for a full `update` in which the source
    // of every root of the dependency has a group. Otherwise (`update
    // <name>`, or a root source without selected entries) the dependency is
    // resolved from its own record, as a root, and its record is kept.
    let dependency_roots: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        if names.is_empty() {
            lock.bundles
                .iter()
                .filter_map(|b| {
                    let roots: std::collections::BTreeSet<String> = lock
                        .root_bundles_of(b)
                        .iter()
                        .map(|root| root.source.clone())
                        .collect();
                    (!roots.is_empty() && roots.iter().all(|r| by_source.contains_key(r)))
                        .then(|| (b.name.clone(), roots))
                })
                .collect()
        } else {
            std::collections::BTreeMap::new()
        };
    // The entries of those dependencies leave their source group. After the
    // prepare phase each one joins the group of a source whose plan provides
    // it, or, when no plan provides it, the group of a root source, which
    // removes it. Each entry carries the union of the root sources of every
    // dependency bundle that lists it. An entry that a bundle resolved as a
    // root also lists stays in its group.
    let mut dependency_entries: Vec<(
        crate::lockfile::LockedItem,
        std::collections::BTreeSet<String>,
    )> = Vec::new();
    for (label, group) in &mut by_source {
        let (moved, kept): (Vec<_>, Vec<_>) = std::mem::take(group).into_iter().partition(|e| {
            let name = e.source_name.as_ref().unwrap_or(&e.name);
            let lists =
                |b: &&crate::lockfile::LockedBundle| b.source == *label && b.items.contains(name);
            lock.bundles
                .iter()
                .filter(lists)
                .any(|b| dependency_roots.contains_key(&b.name))
                && !lock
                    .bundles
                    .iter()
                    .filter(lists)
                    .any(|b| !dependency_roots.contains_key(&b.name))
        });
        *group = kept;
        for entry in moved {
            let name = entry.source_name.as_ref().unwrap_or(&entry.name);
            let roots = lock
                .bundles
                .iter()
                .filter(|b| b.source == *label && b.items.contains(name))
                .filter_map(|b| dependency_roots.get(&b.name))
                .flatten()
                .cloned()
                .collect();
            dependency_entries.push((entry, roots));
        }
    }
    // A group left without entries is kept only when the run resolves a
    // root bundle of its source.
    by_source.retain(|label, group| {
        !group.is_empty()
            || lock
                .bundles
                .iter()
                .any(|b| b.source == *label && !dependency_roots.contains_key(&b.name))
    });

    let mut report = UpdateReport::default();
    // The adoption of a `schema: 1` lockfile is saved. A later error must
    // still carry the adopted paths for the adoption notice.
    let result = (|| -> Result<()> {
        // Prepare phase: fetch, resolve, check and plan every source, in the
        // order of the apply phase, without any write. A failure here (for
        // example the overlap check of one source) happens before the
        // adoption and before any source is applied, except a fetch failure,
        // which skips the source.
        let mut prepared_sources = Vec::with_capacity(by_source.len());
        // The kind restriction of each prepared source.
        let mut source_kinds: std::collections::BTreeMap<String, Vec<ItemKind>> =
            std::collections::BTreeMap::new();
        for (source_label, source_entries) in by_source {
            // An interrupt stops the prepare phase at once: no further
            // source is fetched.
            if observer.interrupted() {
                anyhow::bail!("interrupted before the source `{source_label}` was fetched");
            }
            let source = crate::source::parse_install_source_label(&source_label)
                .with_context(|| format!("parse lockfile source label `{source_label}`"))?;

            // Build aliases from lockfile entries that have source_name
            // A dependency entry carries its alias to the run of each of its
            // root sources, which installs it through `requires`.
            let aliases: Vec<(String, String)> = source_entries
                .iter()
                .chain(
                    dependency_entries
                        .iter()
                        .filter(|(_, roots)| roots.contains(&source_label))
                        .map(|(entry, _)| entry),
                )
                .filter_map(|e| {
                    e.source_name
                        .as_ref()
                        .map(|sn| (sn.clone(), e.name.clone()))
                })
                .collect();
            // Preserve each source's recorded client selection so a bare
            // `update` (no flags, no config → `is_all`) does not silently
            // re-expand a flag-narrowed install and overwrite its lockfile
            // `clients` record (ADR-0012). An explicit flag/config
            // selection still overrides. Note: recorded `clients` are
            // generation clients, so a `--vscode`-only install reconstructs
            // to Copilot on the MCP axis; MCP config is additive and never
            // auto-removed (see the MCP fan-out note in install.rs), so
            // this widens rather than loses.
            // The dependency entries that this source's roots reach join a
            // group only after the prepare phase, so their clients are added
            // here.
            let effective_selection = if selection.is_all() {
                let mut entries = source_entries.clone();
                entries.extend(
                    dependency_entries
                        .iter()
                        .filter(|(_, roots)| roots.contains(&source_label))
                        .map(|(entry, _)| entry.clone()),
                );
                reselect_from_entries(&entries)
            } else {
                selection.clone()
            };
            let recorded_bundles: Vec<_> = lock
                .bundles
                .iter()
                .filter(|bundle| bundle.source == source_label)
                .collect();
            let kinds = source_kind_restriction(
                &lock,
                &source_label,
                |bundle| dependency_roots.contains_key(&bundle.name),
                |bundle| {
                    dependency_roots
                        .get(&bundle.name)
                        .is_some_and(|roots| roots.contains(&source_label))
                },
            );
            source_kinds.insert(source_label.clone(), kinds.clone());
            let mut options = AddOptions {
                force: true,
                overwrite,
                aliases,
                excludes: Vec::new(),
                kinds,
                bundles: if source_label.ends_with(crate::parse::bundle::BUNDLE_SUFFIX) {
                    Vec::new()
                } else {
                    recorded_bundles
                        .iter()
                        .filter(|bundle| !dependency_roots.contains_key(&bundle.name))
                        .map(|bundle| bundle.name.clone())
                        .collect()
                },
                selection: effective_selection,
                keep_unwithheld_edits: true,
                keep_recorded_excludes: true,
                keep_recorded_requirers: true,
                scope: at.scope,
            };
            // Scope the reinstall to exactly the items this source already
            // contributes to the lockfile (by their in-source name), rather
            // than reinstalling every item the source vends. Without this, a
            // source that only contributed a single cross-source dependency
            // would pull in all of its unrelated siblings on `update`
            // (issue #211). An item's cross-source `requires` are still
            // resolved and refreshed because `prepare_install` re-expands
            // the closure for each requested item.
            //
            // Bundle-file sources resolve as a whole. Directory sources with
            // recorded bundles resolve those bundles and their direct items,
            // and the dependencies that no installed item requires any more
            // (the `doctor` condition), so that `update` does not remove an
            // orphaned dependency as `no longer in source` (#613). A member of
            // a recorded bundle is left to its bundle by the next filter.
            let item_filter: Vec<String> =
                if source_label.ends_with(crate::parse::bundle::BUNDLE_SUFFIX) {
                    Vec::new()
                } else if !recorded_bundles.is_empty() {
                    source_entries
                        .iter()
                        .filter(|entry| {
                            entry.required_by.is_empty() || lock.is_orphaned_dependency(entry)
                        })
                        .map(|entry| entry.source_name.as_ref().unwrap_or(&entry.name))
                        .filter(|name| {
                            !recorded_bundles
                                .iter()
                                .any(|bundle| bundle.items.contains(name))
                        })
                        .cloned()
                        .collect()
                } else {
                    source_entries
                        .iter()
                        .map(|e| e.source_name.clone().unwrap_or_else(|| e.name.clone()))
                        .collect()
                };

            // The recorded excludes apply to what the source pulls in, not to
            // an installed item: every installed item of the source is kept,
            // also a bundle member, which the item filter leaves out
            // (issue #570).
            let installed_names: Vec<String> = source_entries
                .iter()
                .map(|e| e.source_name.clone().unwrap_or_else(|| e.name.clone()))
                .collect();
            options.excludes = super::effective_excludes(&lock.excludes, &[], &installed_names);

            // A source that cannot be fetched (a [`FetchFailed`] in the
            // error chain, also for a dependency source) is skipped: it is
            // not written, and the other sources are applied. Every other
            // prepare error stops the run.
            let prepared = match prepare_install(&source, target, &item_filter, &options) {
                Ok(prepared) => prepared,
                // A fetch that an interrupt ended is not a fetch failure.
                Err(err)
                    if err.downcast_ref::<FetchFailed>().is_some() && !observer.interrupted() =>
                {
                    for entry in &source_entries {
                        report.items.push(UpdatedItem {
                            kind: entry.kind,
                            name: entry.name.clone(),
                            source: entry.source.clone(),
                            status: UpdateStatus::SourceSkipped {
                                cause: SkipCause::Unreachable,
                            },
                        });
                    }
                    report.skipped_sources.push(SkippedSource {
                        source: source_label,
                        cause: SkipCause::Unreachable,
                        error: format!("{err:#}"),
                    });
                    continue;
                }
                Err(err) => return Err(err),
            };
            for item in prepared.excluded() {
                if !report.excluded.contains(item) {
                    report.excluded.push(item.clone());
                }
            }
            for item in prepared.pack_only_skipped() {
                if !report.pack_only_skipped.contains(&item) {
                    report.pack_only_skipped.push(item);
                }
            }
            // Every entry outside the recorded kinds is kept unchanged.
            let source_entries: Vec<_> = source_entries
                .into_iter()
                .filter(|entry| {
                    let keep = !options.kinds.is_empty() && !options.kinds.contains(&entry.kind);
                    if keep {
                        report.items.push(UpdatedItem {
                            kind: entry.kind,
                            name: entry.name.clone(),
                            source: entry.source.clone(),
                            status: UpdateStatus::UpToDate,
                        });
                    }
                    !keep
                })
                .collect();
            prepared_sources.push((source_label, source_entries, prepared));
        }
        // One verdict per source: a prepared source whose plan writes an item
        // that the lockfile records from a skipped source is skipped too, so
        // that no item of a skipped source is written. Repeated until no
        // source is dropped, so that the order of the sources does not
        // matter. An interrupted run records no such skip.
        if observer.interrupted() {
            anyhow::bail!(
                "interrupted before the sources were checked against the skipped sources"
            );
        }
        loop {
            let skipped_items: std::collections::BTreeMap<(ItemKind, String), String> = lock
                .items
                .iter()
                .filter(|i| report.skipped_sources.iter().any(|s| s.source == i.source))
                .map(|i| ((i.kind, i.name.clone()), i.source.clone()))
                .collect();
            let found = prepared_sources
                .iter()
                .enumerate()
                .find_map(|(index, (_, _, p))| {
                    p.planned_hashes().into_keys().find_map(|key| {
                        skipped_items.get(&key).map(|src| (index, key, src.clone()))
                    })
                });
            let Some((index, (kind, name), unreachable)) = found else {
                break;
            };
            let (source_label, source_entries, _) = prepared_sources.remove(index);
            // Entries outside the kind restriction were already reported as
            // up to date. They share the skipped verdict of their source.
            for item in report.items.iter_mut().filter(|i| i.source == source_label) {
                item.status = UpdateStatus::SourceSkipped {
                    cause: SkipCause::RequiresSkipped,
                };
            }
            for entry in &source_entries {
                report.items.push(UpdatedItem {
                    kind: entry.kind,
                    name: entry.name.clone(),
                    source: entry.source.clone(),
                    status: UpdateStatus::SourceSkipped {
                        cause: SkipCause::RequiresSkipped,
                    },
                });
            }
            report.skipped_sources.push(SkippedSource {
                source: source_label,
                cause: SkipCause::RequiresSkipped,
                error: format!(
                    "it requires {kind} `{name}` of the source `{unreachable}`, which was skipped"
                ),
            });
        }
        // Each dependency entry joins the group of the first source whose
        // plan provides it. When no plan provides it and every root source
        // was prepared, it joins the group of its first root source, which
        // removes it. When a root source was skipped, the entry is kept and
        // reported with the skip of that source.
        let dependency_keys: std::collections::BTreeSet<(ItemKind, String)> = dependency_entries
            .iter()
            .map(|(entry, _)| (entry.kind, entry.name.clone()))
            .collect();
        for (entry, roots) in std::mem::take(&mut dependency_entries) {
            let key = (entry.kind, entry.name.clone());
            let provider = prepared_sources
                .iter()
                .position(|(_, _, p)| p.planned_hashes().contains_key(&key));
            let all_roots_prepared = roots
                .iter()
                .all(|root| prepared_sources.iter().any(|(label, _, _)| label == root));
            let first_root = prepared_sources
                .iter()
                .position(|(label, _, _)| roots.contains(label));
            match (provider, all_roots_prepared, first_root) {
                (Some(index), _, _) | (None, true, Some(index)) => {
                    // An entry outside the kind restriction of the group is
                    // kept unchanged, as an entry of the group is.
                    let label = &prepared_sources[index].0;
                    let kinds = source_kinds.get(label).map_or(&[][..], Vec::as_slice);
                    if !kinds.is_empty() && !kinds.contains(&entry.kind) {
                        report.items.push(UpdatedItem {
                            kind: entry.kind,
                            name: entry.name.clone(),
                            source: entry.source.clone(),
                            status: UpdateStatus::UpToDate,
                        });
                    } else {
                        prepared_sources[index].1.push(entry);
                    }
                }
                _ => {
                    let cause = report
                        .skipped_sources
                        .iter()
                        .find(|s| roots.contains(&s.source))
                        .map_or(SkipCause::RequiresSkipped, |s| s.cause);
                    report.items.push(UpdatedItem {
                        kind: entry.kind,
                        name: entry.name.clone(),
                        source: entry.source.clone(),
                        status: UpdateStatus::SourceSkipped { cause },
                    });
                }
            }
        }
        // The bundles that a prepared resolution reaches: a dependency bundle
        // outside this set is dropped once its root sources are applied.
        let reached_bundles: std::collections::BTreeSet<String> = prepared_sources
            .iter()
            .flat_map(|(_, _, p)| p.resolved_bundle_names())
            .collect();
        let mut applied_sources: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        let total_sources = prepared_sources.len();
        for (_, _, prepared) in &prepared_sources {
            run_items.extend(prepared.planned_hashes().into_keys());
        }
        report.items_in_run = run_items.len();

        // The overlap check of `add`, applied to the items of every source
        // that the run reads and to the items of every recorded local-path
        // source that the run does not read, before the first write.
        let at_check = crate::scope::ScopeRoot {
            scope: at.scope,
            root: target.to_path_buf(),
        };
        let mut source_items: Vec<super::output::SourceItem> = prepared_sources
            .iter()
            .flat_map(|(_, _, prepared)| prepared.source_items().iter().cloned())
            .collect();
        let recorded: std::collections::BTreeSet<&str> = lock
            .bundles
            .iter()
            .map(|b| b.source.as_str())
            .chain(lock.items.iter().map(|i| i.source.as_str()))
            .filter(|label| !prepared_sources.iter().any(|(l, _, _)| l == label))
            .collect();
        for label in recorded {
            collect_recorded_local_items(label, &mut source_items).with_context(|| {
                // `remove --source` matches the sources of items only.
                if lock.items.iter().any(|i| i.source == label) {
                    format!(
                        "read the recorded source `{label}` for the overlap check; restore it, \
                         or remove its items with `upskill remove --source {label}`"
                    )
                } else {
                    format!("read the recorded source `{label}` for the overlap check; restore it")
                }
            })?;
        }
        super::output::check_run_outside_outputs(&at_check, &source_items)?;
        // The recorded files that the run deletes for an item that its
        // source no longer contains are checked against the same item
        // directories (issue #550).
        // The records are read after an adoption in memory, which writes
        // nothing: a `schema: 1` lockfile has no `files` record before it.
        let mut adopted_lock = lock.clone();
        adopt_schema1_in_memory(target, &mut adopted_lock)?;
        let mut deleted = Vec::new();
        for (_, source_entries, prepared) in &prepared_sources {
            let planned = prepared.planned_hashes();
            for entry in source_entries {
                if planned.contains_key(&(entry.kind, entry.name.clone())) {
                    continue;
                }
                if let Some(adopted) = adopted_lock
                    .items
                    .iter()
                    .find(|i| i.kind == entry.kind && i.name == entry.name)
                {
                    deleted.extend(super::output::recorded_paths(adopted));
                }
            }
        }
        super::output::check_outside_recorded_paths(
            target,
            &deleted,
            &source_items,
            super::output::RecordedUse::UpdateDeletes,
        )?;

        // Apply phase: preview, decide and commit each source in turn.
        for (index, (source_label, source_entries, prepared)) in
            prepared_sources.into_iter().enumerate()
        {
            let new_hashes = prepared.planned_hashes();
            // A dependency entry is not counted: a root bundle that resolves
            // to no item, because it lists none and dropped its `requires`,
            // removes the entries of its former dependencies.
            let own_entries: Vec<_> = source_entries
                .iter()
                .filter(|e| !dependency_keys.contains(&(e.kind, e.name.clone())))
                .cloned()
                .collect();
            guard_against_empty_source(&new_hashes, &own_entries, &source_label)?;
            // The bundles that the lockfile records for this source, as the
            // fetched source declares them now: the plugin repair below
            // installs from them (issue #229). A source under a kind
            // restriction (see [`source_kind_restriction`]) runs no plugin
            // step, so it gets no repair either.
            let repair_bundles: Vec<crate::model::Bundle> = if source_kinds
                .get(&source_label)
                .is_some_and(|k| !k.is_empty())
            {
                Vec::new()
            } else {
                lock.bundles
                    .iter()
                    .filter(|b| b.source == source_label)
                    .filter_map(|b| prepared.source_bundle(&b.name))
                    .collect()
            };

            let mut current = crate::lockfile::Lockfile::load(target)?;
            let adopted = if dry_run {
                adopt_schema1_in_memory(target, &mut current)?;
                None
            } else {
                adopt_schema1(target, &mut current)?
            };
            if let Some(paths) = &adopted {
                let all = report.adopted.get_or_insert_with(Vec::new);
                all.extend(paths.iter().cloned());
                all.sort();
                all.dedup();
            }

            // An entry that the source no longer contains is removed. The
            // key is the installed `(kind, name)`, as in the plan.
            let is_orphan = |e: &crate::lockfile::LockedItem| {
                !new_hashes.contains_key(&(e.kind, e.name.clone()))
            };
            let orphan_rules: std::collections::BTreeSet<String> = source_entries
                .iter()
                .filter(|e| is_orphan(e) && e.is_opencode_rule())
                .map(|e| e.name.clone())
                .collect();
            let install_preview =
                prepared.preview(target, plugin_scope, &current, &orphan_rules)?;
            let mut items = install_preview.items;
            for orphan in source_entries.iter().filter(|e| is_orphan(e)) {
                if let Some(entry) = current
                    .items
                    .iter()
                    .find(|i| i.kind == orphan.kind && i.name == orphan.name)
                {
                    items.push(removal_preview(plan_removal(target, entry, overwrite)?));
                }
            }
            let preview = SourcePreview {
                source: source_label.clone(),
                items,
                kept_notes: install_preview.kept_notes,
                handshake: install_preview.handshake,
                adopted,
            };
            report.sources_considered += 1;
            let decision = observer.preview(&preview);

            if dry_run || decision != SourceDecision::Apply {
                // A source with changes that is not applied was declined (or
                // the run is a dry run): its `Skip` items are `NotApplied`
                // and do not cause exit code 1. A source without changes is
                // not applied because there is nothing to apply: its `Skip`
                // items are skipped, as in an applied source.
                let declined = dry_run || preview.has_changes();
                if preview.has_changes() {
                    report.sources_not_applied += 1;
                }
                report
                    .kept_modified
                    .extend(preview.kept_notes.iter().cloned());
                for entry in &source_entries {
                    let status = match preview
                        .items
                        .iter()
                        .find(|i| i.kind == entry.kind && i.name == entry.name)
                    {
                        Some(item) if item.action == PreviewAction::Skip && !declined => {
                            UpdateStatus::SkippedModified {
                                files: item.blocking.clone(),
                            }
                        }
                        Some(item) => UpdateStatus::NotApplied {
                            action: item.action,
                        },
                        None => UpdateStatus::UpToDate,
                    };
                    report.items.push(UpdatedItem {
                        kind: entry.kind,
                        name: entry.name.clone(),
                        source: entry.source.clone(),
                        status,
                    });
                }
                // A skipped item that is not an entry of this source: a
                // dependency from another source that the reinstall plans.
                for item in &preview.items {
                    if item.action != PreviewAction::Skip
                        || source_entries
                            .iter()
                            .any(|e| e.kind == item.kind && e.name == item.name)
                    {
                        continue;
                    }
                    report.items.push(UpdatedItem {
                        kind: item.kind,
                        name: item.name.clone(),
                        source: source_of(&lock, item.kind, &item.name, &source_label),
                        status: if declined {
                            UpdateStatus::NotApplied {
                                action: PreviewAction::Skip,
                            }
                        } else {
                            UpdateStatus::SkippedModified {
                                files: item.blocking.clone(),
                            }
                        },
                    });
                }
                // A source without changes still gets the fetched commit in
                // its selected lockfile entries. This write is not a change of the
                // source: it is neither in `report.changed` nor counted by
                // `has_changes`.
                if !dry_run && !preview.has_changes() && decision != SourceDecision::Stop {
                    let selected: std::collections::BTreeSet<(ItemKind, String)> = source_entries
                        .iter()
                        .filter(|e| {
                            !preview.items.iter().any(|i| {
                                i.action == PreviewAction::Skip
                                    && i.kind == e.kind
                                    && i.name == e.name
                            })
                        })
                        .map(|e| (e.kind, e.name.clone()))
                        .collect();
                    prepared.record_commit(target, &selected)?;
                    report.plugin_results.extend(repair_missing_plugins(
                        target,
                        plugin_scope,
                        &repair_bundles,
                    )?);
                }
                if !dry_run && decision == SourceDecision::Stop {
                    report.stopped = true;
                    report.sources_remaining = total_sources - index - 1;
                    break;
                }
                continue;
            }

            let first_item = report.items.len();
            let kept_before = report.kept_modified.len();
            let changed_before = report.changed.len();
            let requirers = prepared.resolved_requirers();
            let install_report = prepared.commit(target, plugin_scope)?;
            applied_sources.insert(source_label.clone());
            update_dependency_records(
                target,
                &dependency_roots,
                &reached_bundles,
                &applied_sources,
                &requirers,
            )?;
            // The plugin step of the apply attempts every plugin of the
            // recorded bundles of the source, so an applied source needs no
            // plugin repair.
            report
                .plugin_results
                .extend(install_report.plugin_results.iter().cloned());
            report
                .kept_modified
                .extend(install_report.kept_modified.iter().cloned());
            report.forced.extend(install_report.forced.iter().cloned());
            report
                .changed
                .extend(install_report.changed.iter().cloned());
            let mut skipped: std::collections::BTreeMap<(ItemKind, String), Vec<PathBuf>> =
                std::collections::BTreeMap::new();
            for conflict in &install_report.conflicts {
                skipped
                    .entry((conflict.kind, conflict.name.clone()))
                    .or_default()
                    .push(conflict.path.clone());
            }
            // An item that the source no longer contains is removed as
            // `remove` does: plan every orphan again from the saved
            // lockfile, then delete.
            let orphans: Vec<&crate::lockfile::LockedItem> =
                source_entries.iter().filter(|e| is_orphan(e)).collect();
            if !orphans.is_empty() {
                let mut lock = crate::lockfile::Lockfile::load(target)?;
                let mut plans = Vec::with_capacity(orphans.len());
                for orphan in &orphans {
                    if let Some(entry) = lock
                        .items
                        .iter()
                        .find(|i| i.kind == orphan.kind && i.name == orphan.name)
                    {
                        plans.push(plan_removal(target, entry, overwrite)?);
                    }
                }
                let mut reported = std::collections::BTreeSet::new();
                let dropped: std::collections::BTreeSet<String> = orphans
                    .iter()
                    .filter(|o| o.is_opencode_rule())
                    .map(|o| o.name.clone())
                    .collect();
                for plan in plans {
                    report
                        .changed
                        .extend(execute_removal(target, &plan, &mut reported)?);
                    report.kept_modified.extend(plan.kept.iter().cloned());
                    report.forced.extend(plan.forced.iter().cloned());
                    lock.remove(plan.kind, &plan.name);
                    let source = orphans
                        .iter()
                        .find(|o| o.kind == plan.kind && o.name == plan.name)
                        .map_or_else(|| source_label.clone(), |o| o.source.clone());
                    report.items.push(UpdatedItem {
                        kind: plan.kind,
                        name: plan.name,
                        source,
                        status: UpdateStatus::Removed,
                    });
                }
                lock.save(target)?;
                drop_global_opencode_rules(at.scope, target, &dropped);
            }
            for entry in &source_entries {
                let kind = entry.kind;
                if is_orphan(entry) {
                    continue; // already handled as orphan
                }
                let new_hash = new_hashes
                    .get(&(kind, entry.name.clone()))
                    .cloned()
                    .flatten();
                // The record before this commit, not the record at the start
                // of the run: an earlier source of the run can have written
                // this item as its dependency, and that write is not reported
                // again here.
                let recorded = current
                    .items
                    .iter()
                    .find(|i| i.kind == kind && i.name == entry.name)
                    .unwrap_or(entry);
                let status = if let Some(files) = skipped.remove(&(kind, entry.name.clone())) {
                    UpdateStatus::SkippedModified { files }
                } else if new_hash == recorded.hash && !recorded.pending.is_empty() {
                    UpdateStatus::CompletedInterrupted
                } else if new_hash == recorded.hash {
                    UpdateStatus::UpToDate
                } else {
                    UpdateStatus::Updated {
                        old_hash: recorded.hash.clone(),
                        new_hash,
                    }
                };
                report.items.push(UpdatedItem {
                    kind,
                    name: entry.name.clone(),
                    source: entry.source.clone(),
                    status,
                });
            }
            // An item that is not an entry of this source and that the
            // commit wrote: a dependency from another source. It is
            // reported here, as for an entry, against the lockfile record
            // before the commit.
            for ((kind, name), new_hash) in &new_hashes {
                if skipped.contains_key(&(*kind, name.clone()))
                    || source_entries
                        .iter()
                        .any(|e| e.kind == *kind && &e.name == name)
                {
                    continue;
                }
                let recorded = current
                    .items
                    .iter()
                    .find(|i| i.kind == *kind && &i.name == name);
                let status = match recorded {
                    Some(entry) if entry.hash == *new_hash && !entry.pending.is_empty() => {
                        UpdateStatus::CompletedInterrupted
                    }
                    Some(entry) if entry.hash == *new_hash => continue,
                    _ => UpdateStatus::Updated {
                        old_hash: recorded.and_then(|e| e.hash.clone()),
                        new_hash: new_hash.clone(),
                    },
                };
                report.items.push(UpdatedItem {
                    kind: *kind,
                    name: name.clone(),
                    source: source_of(&lock, *kind, name, &source_label),
                    status,
                });
            }
            // A skipped item that is not an entry of this source: a
            // dependency from another source that the reinstall
            // planned. It is reported here, because its own source
            // may not be part of this run.
            for ((kind, name), files) in skipped {
                let source = source_of(&lock, kind, &name, &source_label);
                report.items.push(UpdatedItem {
                    kind,
                    name,
                    source,
                    status: UpdateStatus::SkippedModified { files },
                });
            }
            report.sources_applied += 1;
            let mut result = SourceResult {
                source: source_label.clone(),
                kept_modified: report.kept_modified.len() - kept_before,
                changed_files: report.changed.len() - changed_before,
                ..SourceResult::default()
            };
            for item in &report.items[first_item..] {
                if item.status.is_change() {
                    match item.status {
                        UpdateStatus::Updated { .. } => result.updated += 1,
                        UpdateStatus::CompletedInterrupted => result.completed += 1,
                        UpdateStatus::Removed => result.removed += 1,
                        UpdateStatus::UpToDate
                        | UpdateStatus::SkippedModified { .. }
                        | UpdateStatus::NotApplied { .. }
                        | UpdateStatus::SourceSkipped { .. } => {}
                    }
                } else if matches!(item.status, UpdateStatus::SkippedModified { .. }) {
                    result.skipped += 1;
                }
            }
            observer.applied(&result);
        }
        Ok(())
    })();
    // An item that a source changed as a dependency of another source can
    // also have an `UpToDate` entry from a later source of the run (its own
    // source, which then finds it written). An `UpToDate` entry of an item
    // that has a change anywhere in the report is not reported. Only the
    // first change of an item is reported: two sources can plan different
    // content for one item when their `requires` name different sources or
    // refs for it.
    let changed_keys: std::collections::BTreeSet<(ItemKind, String)> = report
        .items
        .iter()
        .filter(|i| i.status.is_change())
        .map(|i| (i.kind, i.name.clone()))
        .collect();
    let mut seen_changed = std::collections::BTreeSet::new();
    report.items.retain(|item| {
        if item.status.is_change() {
            seen_changed.insert((item.kind, item.name.clone()))
        } else {
            item.status != UpdateStatus::UpToDate
                || !changed_keys.contains(&(item.kind, item.name.clone()))
        }
    });
    // An item skipped by the reinstall of two sources, or not applied as a
    // skip in two sources, is reported once.
    let mut seen_skipped = std::collections::BTreeSet::new();
    let mut seen_not_applied = std::collections::BTreeSet::new();
    report.items.retain(|item| match item.status {
        UpdateStatus::SkippedModified { .. } => seen_skipped.insert((item.kind, item.name.clone())),
        UpdateStatus::NotApplied {
            action: PreviewAction::Skip,
        } => seen_not_applied.insert((item.kind, item.name.clone())),
        _ => true,
    });
    // A file kept by the reinstall of two sources (a dependency from
    // another source) is reported once.
    report.kept_modified.sort();
    report.kept_modified.dedup();
    if let Err(err) = result {
        let err = match &report.adopted {
            Some(paths)
                if !paths.is_empty() && err.downcast_ref::<AdoptedBeforeFailure>().is_none() =>
            {
                err.context(AdoptedBeforeFailure {
                    paths: paths.clone(),
                })
            }
            _ => err,
        };
        // The skipped sources are reported also when a later step fails.
        return Err(if report.skipped_sources.is_empty() {
            err
        } else {
            err.context(SkippedBeforeFailure {
                sources: report.skipped_sources.clone(),
            })
        });
    }

    Ok(report)
}

/// The source label of the lockfile entry `(kind, name)`, or `fallback`
/// when the lockfile has no such entry.
fn source_of(
    lock: &crate::lockfile::Lockfile,
    kind: ItemKind,
    name: &str,
    fallback: &str,
) -> String {
    lock.items
        .iter()
        .find(|i| i.kind == kind && i.name == name)
        .map_or_else(|| fallback.to_string(), |i| i.source.clone())
}

/// The preview line of an item that its source no longer contains:
/// `delete` is the files to delete that are unchanged, `forced` the
/// modified files that `overwrite` deletes, and `kept_modified` the
/// modified files that are kept.
fn removal_preview(plan: RemovalPlan) -> PreviewItem {
    let delete = plan
        .delete
        .into_iter()
        .filter(|rel| !plan.forced.contains(rel))
        .collect();
    PreviewItem {
        kind: plan.kind,
        name: plan.name,
        action: PreviewAction::Remove,
        write: Vec::new(),
        delete,
        kept_modified: plan.kept,
        forced: plan.forced,
        blocking: Vec::new(),
        record_only: false,
    }
}

/// The registry root of `path` when `path` is a bundle-file source: a path
/// that is not a directory and whose name ends in `.bundle.yaml`, whether or
/// not the file exists. `None` for any other path.
fn local_bundle_root(path: &Path) -> Option<Result<PathBuf>> {
    let is_bundle_label = !path.is_dir()
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(crate::parse::bundle::BUNDLE_SUFFIX));
    is_bundle_label.then(|| super::install::bundle_file_root(path))
}

/// Push to `out` the item directories of the source recorded as `label`,
/// for the overlap checks of [`update`] and of [`remove`]
/// (`check_removal_outside_sources`), when `label` is a local path. Every
/// item is pushed without an installed name, so that its directory is
/// protected. A directory that does not exist holds no item and pushes
/// nothing. For a bundle-file label, whether or not the bundle file still
/// exists, the items are those of the registry root that `add` resolves for
/// it ([`super::install::bundle_file_root`]); a root that does not exist
/// pushes nothing. A git source is not
/// fetched: its clone is a temporary directory, which cannot overlap an
/// output path. A path that exists and cannot be read as a directory (for
/// example a plain file that is not a bundle file) is an error that names
/// `label`.
fn collect_recorded_local_items(
    label: &str,
    out: &mut Vec<super::output::SourceItem>,
) -> Result<()> {
    let Ok(crate::source::InstallSource::LocalPath(path)) =
        crate::source::parse_install_source_label(label)
    else {
        return Ok(());
    };
    // A bundle-file label, whether or not the file still exists: the root
    // that `add` resolves for it. A path that is a directory is a directory
    // source, whatever its name.
    let root = match local_bundle_root(&path) {
        Some(root) => root?,
        None => path,
    };
    let none = crate::bundle::ResolvedItems::default();
    super::install::collect_source_items(
        &root,
        Some(&none),
        &super::plan::PlanNames::default(),
        out,
    )
}

/// Guard against the data-loss footgun in issue #196: when re-resolving a
/// source yields **no** items but the lockfile still records entries for
/// it, treat that as a fetch/resolution anomaly and abort rather than
/// scheduling every entry for removal. A source whose items were all
/// genuinely deleted upstream is indistinguishable from a transient
/// failure here, so erring toward "do not delete" is the safe default —
/// the user can `upskill remove --source <label>` to clear such entries
/// deliberately.
/// Brings the records of the dependency bundles of `dependency_roots` up to
/// date after an applied source whose resolution reached the bundles of
/// `requirers` (each with the requirers that this resolution found): a
/// requirer that this resolution evaluated and that no longer reaches a
/// dependency leaves its `required_by` (#648). A dependency bundle outside
/// `reached` is dropped once every one of its root sources is in `applied`.
/// Runs after the commit of each applied source, so that an error in a
/// later source leaves the records of the applied sources up to date.
fn update_dependency_records(
    target: &Path,
    dependency_roots: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    reached: &std::collections::BTreeSet<String>,
    applied: &std::collections::BTreeSet<String>,
    requirers: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<()> {
    if dependency_roots.is_empty() {
        return Ok(());
    }
    let mut lock = crate::lockfile::Lockfile::load(target)?;
    let before = lock.bundles.clone();
    let evaluated: std::collections::BTreeSet<String> = requirers
        .keys()
        .map(|name| format!("bundle:{name}"))
        .collect();
    for bundle in &mut lock.bundles {
        if !dependency_roots.contains_key(&bundle.name) || bundle.required_by.is_empty() {
            continue;
        }
        let mut merged: std::collections::BTreeSet<String> = bundle
            .required_by
            .iter()
            .filter(|r| !evaluated.contains(*r))
            .cloned()
            .collect();
        if let Some(found) = requirers.get(&bundle.name) {
            merged.extend(found.iter().cloned());
        }
        if !merged.is_empty() {
            bundle.required_by = merged.into_iter().collect();
        }
    }
    lock.bundles.retain(|b| {
        dependency_roots
            .get(&b.name)
            .is_none_or(|roots| reached.contains(&b.name) || !roots.is_subset(applied))
    });
    if lock.bundles != before {
        lock.save(target)?;
    }
    Ok(())
}

fn guard_against_empty_source(
    new_hashes: &std::collections::BTreeMap<(ItemKind, String), Option<String>>,
    source_entries: &[crate::lockfile::LockedItem],
    source_label: &str,
) -> Result<()> {
    if new_hashes.is_empty() && !source_entries.is_empty() {
        let count = source_entries.len();
        let noun = if count == 1 { "item" } else { "items" };
        anyhow::bail!(
            "source `{source_label}` resolved to no items, but the lockfile records \
             {count} {noun} from it — refusing to remove them (the source may be \
             unreachable or its layout changed). Re-check the source, or run \
             `upskill remove --source {source_label}` to clear these entries deliberately."
        );
    }
    Ok(())
}

/// List installed content from `<target>/.upskill-lock.json`. No
/// filesystem walk, no fetch — pure lockfile dump grouped by kind.
/// Empty lockfile (or missing file) is not an error; the returned
/// report is empty.
pub fn list(target: &Path) -> Result<ListReport> {
    let lock = crate::lockfile::Lockfile::load(target)?;
    let mut report = ListReport::default();
    for entry in &lock.items {
        let kind = entry.kind;
        let listed = ListedItem {
            kind,
            name: entry.name.clone(),
            source: entry.source.clone(),
            git_ref: entry.git_ref.clone(),
        };
        match kind {
            ItemKind::Rule => report.rules.push(listed),
            ItemKind::Skill => report.skills.push(listed),
            ItemKind::Agent => report.agents.push(listed),
        }
    }
    for bucket in [&mut report.rules, &mut report.skills, &mut report.agents] {
        bucket.sort_by(|a, b| a.name.cmp(&b.name));
    }
    for bundle in &lock.bundles {
        report.bundles.push(ListedBundle {
            name: bundle.name.clone(),
            source: bundle.source.clone(),
            git_ref: bundle.git_ref.clone(),
            items: bundle.items.clone(),
        });
    }
    report.bundles.sort_by(|a, b| a.name.cmp(&b.name));
    report.excludes = lock.excludes.clone();
    Ok(report)
}

/// The names of `names` that the lockfile under `root` holds as an item.
/// Empty when the lockfile is missing or cannot be read.
pub fn installed_names(root: &Path, names: &[String]) -> Vec<String> {
    let Ok(lock) = crate::lockfile::Lockfile::load(root) else {
        return Vec::new();
    };
    names
        .iter()
        .filter(|n| lock.items.iter().any(|i| &i.name == *n))
        .cloned()
        .collect()
}

/// [`list`] for each given scope root, in the given order. A scope whose
/// lockfile is absent gives an empty report.
pub fn list_scopes(roots: &[ScopeRoot]) -> Result<Vec<ScopedList>> {
    roots
        .iter()
        .map(|at| {
            Ok(ScopedList {
                scope: at.scope,
                root: at.root.clone(),
                report: list(&at.root)
                    .with_context(|| format!("failed to list the {} scope", at.scope))?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_with_bundle(source: &str, item_source: Option<&str>) -> crate::lockfile::Lockfile {
        let mut lock = crate::lockfile::Lockfile::new();
        lock.bundles.push(crate::lockfile::LockedBundle {
            name: "oc".into(),
            source: source.into(),
            git_ref: None,
            items: vec![],
            kinds: vec![],
            required_by: vec![],
        });
        if let Some(item_source) = item_source {
            let mut lock2: crate::lockfile::Lockfile = serde_json::from_value(serde_json::json!({
                "schema": 2,
                "items": [{"kind": "skill", "name": "alpha", "source": item_source}]
            }))
            .unwrap();
            lock2.bundles = lock.bundles;
            return lock2;
        }
        lock
    }

    #[test]
    fn add_command_for_bundle_names_the_command_update_cannot_run() {
        let project = ScopeRoot {
            scope: Scope::Project,
            root: PathBuf::from("/p"),
        };
        let global = ScopeRoot {
            scope: Scope::Global,
            root: PathBuf::from("/h"),
        };
        let dir = lock_with_bundle("local:/r", None);
        assert_eq!(
            add_command_for_bundle(&project, &dir, "oc").as_deref(),
            Some("upskill add /r oc --project")
        );
        assert_eq!(
            add_command_for_bundle(&global, &dir, "oc").as_deref(),
            Some("upskill add /r oc --global")
        );
        let file = lock_with_bundle("local:/r/oc.bundle.yaml", None);
        assert_eq!(
            add_command_for_bundle(&project, &file, "oc").as_deref(),
            Some("upskill add /r/oc.bundle.yaml --project")
        );
        let github = lock_with_bundle("github:acme/skills", None);
        assert_eq!(
            add_command_for_bundle(&project, &github, "oc").as_deref(),
            Some("upskill add acme/skills oc --project")
        );
        // An item from another source does not make `update` reach the bundle.
        let other = lock_with_bundle("local:/r", Some("local:/other"));
        assert_eq!(
            add_command_for_bundle(&project, &other, "oc").as_deref(),
            Some("upskill add /r oc --project")
        );
        let reached = lock_with_bundle("local:/r", Some("local:/r"));
        assert_eq!(add_command_for_bundle(&project, &reached, "oc"), None);
        // `update` reaches the source, but skips its plugins under a kind
        // restriction.
        let mut restricted = lock_with_bundle("local:/r", Some("local:/r"));
        restricted.bundles[0].kinds = vec![ItemKind::Skill];
        assert_eq!(
            add_command_for_bundle(&project, &restricted, "oc").as_deref(),
            Some("upskill add /r oc --project")
        );
    }
    use crate::plugin::PluginScope;
    use crate::select::ClientSelection;
    use crate::source::InstallSource;

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let full = root.join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, bytes).unwrap();
    }

    fn skill(src: &Path, name: &str, body: &str) {
        write(
            src,
            &format!("{name}/SKILL.md"),
            format!("---\nschema: 1\nname: {name}\ndescription: test {name}\n---\n{body}\n")
                .as_bytes(),
        );
    }

    fn install(src: &Path, target: &Path) {
        super::super::prepared::prepare_install(
            &InstallSource::LocalPath(src.to_path_buf()),
            target,
            &[],
            &AddOptions::default(),
        )
        .unwrap()
        .commit(target, PluginScope::Project)
        .unwrap();
    }

    /// The installed Claude file of the skill `name`.
    fn claude_skill(name: &str) -> PathBuf {
        Path::new(".claude")
            .join("skills")
            .join(name)
            .join("SKILL.md")
    }

    /// Records every preview and every result; `decide` gives the decision.
    struct Recorder<F: FnMut(&SourcePreview) -> SourceDecision> {
        previews: Vec<SourcePreview>,
        results: Vec<SourceResult>,
        decide: F,
    }

    impl<F: FnMut(&SourcePreview) -> SourceDecision> Recorder<F> {
        fn new(decide: F) -> Self {
            Recorder {
                previews: Vec::new(),
                results: Vec::new(),
                decide,
            }
        }
    }

    impl<F: FnMut(&SourcePreview) -> SourceDecision> UpdateObserver for Recorder<F> {
        fn preview(&mut self, preview: &SourcePreview) -> SourceDecision {
            self.previews.push(preview.clone());
            (self.decide)(preview)
        }

        fn applied(&mut self, result: &SourceResult) {
            self.results.push(result.clone());
        }
    }

    fn run(target: &Path, observer: &mut dyn UpdateObserver) -> Result<UpdateReport> {
        run_with(target, false, false, observer)
    }

    fn run_with(
        target: &Path,
        overwrite: bool,
        dry_run: bool,
        observer: &mut dyn UpdateObserver,
    ) -> Result<UpdateReport> {
        update(
            &crate::scope::ScopeRoot {
                scope: crate::scope::Scope::Project,
                root: target.to_path_buf(),
            },
            &[],
            PluginScope::Project,
            &ClientSelection::all(),
            overwrite,
            dry_run,
            observer,
        )
    }

    fn lock_bytes(target: &Path) -> Vec<u8> {
        fs::read(target.join(".upskill-lock.json")).unwrap()
    }

    fn status_of<'a>(report: &'a UpdateReport, name: &str) -> &'a UpdateStatus {
        &report
            .items
            .iter()
            .find(|i| i.name == name)
            .unwrap_or_else(|| panic!("no item {name} in {:?}", report.items))
            .status
    }

    /// An observer that reports an interrupt.
    struct Interrupted {
        previews: usize,
    }

    impl UpdateObserver for Interrupted {
        fn preview(&mut self, _preview: &SourcePreview) -> SourceDecision {
            self.previews += 1;
            SourceDecision::Apply
        }

        fn applied(&mut self, _result: &SourceResult) {}

        fn interrupted(&self) -> bool {
            true
        }
    }

    /// An interrupt stops the prepare phase at once: the source that cannot
    /// be fetched is not recorded as skipped, and nothing is previewed.
    #[test]
    fn an_interrupt_stops_the_prepare_phase_without_a_skip() {
        let tmp = tempfile::tempdir().unwrap();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&a, "x", "body");
        skill(&b, "y", "body");
        install(&a, &target);
        install(&b, &target);
        fs::remove_dir_all(&a).unwrap();
        let lock_before = lock_bytes(&target);

        let mut observer = Interrupted { previews: 0 };
        let err = run(&target, &mut observer).unwrap_err();

        assert!(
            err.downcast_ref::<SkippedBeforeFailure>().is_none(),
            "{err:#}"
        );
        assert!(format!("{err:#}").contains("interrupted"), "{err:#}");
        assert_eq!(observer.previews, 0);
        assert_eq!(lock_bytes(&target), lock_before);
    }

    /// An observer whose interrupt is set after its first query: the check
    /// before the first source passes, and the interrupt is seen when the
    /// fetch of that source has failed.
    struct InterruptedDuringFetch {
        queries: std::cell::Cell<usize>,
    }

    impl UpdateObserver for InterruptedDuringFetch {
        fn preview(&mut self, _preview: &SourcePreview) -> SourceDecision {
            SourceDecision::Apply
        }

        fn applied(&mut self, _result: &SourceResult) {}

        fn interrupted(&self) -> bool {
            let n = self.queries.get();
            self.queries.set(n + 1);
            n > 0
        }
    }

    /// A fetch that fails after an interrupt is not a fetch failure: no
    /// skipped source is recorded and the run stops with the fetch error.
    #[test]
    fn a_fetch_failure_after_an_interrupt_records_no_skip_and_stops() {
        let tmp = tempfile::tempdir().unwrap();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&a, "x", "body");
        skill(&b, "y", "body");
        install(&a, &target);
        install(&b, &target);
        fs::remove_dir_all(&a).unwrap();
        let lock_before = lock_bytes(&target);

        let mut observer = InterruptedDuringFetch {
            queries: std::cell::Cell::new(0),
        };
        let err = run(&target, &mut observer).unwrap_err();

        assert!(err.downcast_ref::<FetchFailed>().is_some(), "{err:#}");
        assert!(
            err.downcast_ref::<SkippedBeforeFailure>().is_none(),
            "{err:#}"
        );
        assert_eq!(lock_bytes(&target), lock_before);
    }

    /// An observer whose interrupt is set from query `at` on, and that
    /// counts its queries.
    struct InterruptedAt {
        at: usize,
        queries: std::cell::Cell<usize>,
    }

    impl UpdateObserver for InterruptedAt {
        fn preview(&mut self, _preview: &SourcePreview) -> SourceDecision {
            SourceDecision::Skip
        }

        fn applied(&mut self, _result: &SourceResult) {}

        fn interrupted(&self) -> bool {
            let n = self.queries.get();
            self.queries.set(n + 1);
            n >= self.at
        }
    }

    /// An interrupt after the first source was prepared: the second source
    /// is not fetched. Its path is missing, so a fetch of it would fail with
    /// the fetch error instead of the interrupt error.
    #[test]
    fn an_interrupt_after_the_first_source_fetches_no_further_source() {
        let tmp = tempfile::tempdir().unwrap();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&a, "x", "body");
        skill(&b, "y", "body");
        install(&a, &target);
        install(&b, &target);
        fs::remove_dir_all(&b).unwrap();

        let mut observer = InterruptedAt {
            at: 1,
            queries: std::cell::Cell::new(0),
        };
        let err = run(&target, &mut observer).unwrap_err();

        let label = InstallSource::LocalPath(b.clone()).to_string();
        assert_eq!(
            format!("{err:#}"),
            format!("interrupted before the source `{label}` was fetched")
        );
    }

    /// The cascade layout with local paths: `a` holds `x` (requires `y` of
    /// the missing directory `m`) and `z`; `c` holds `w` (requires `z` of
    /// `a`). `m` and `a` cannot be fetched, and `c` requires an item of the
    /// skipped `a`.
    fn stage_local_cascade(tmp: &Path) -> PathBuf {
        let (a, c, m) = (tmp.join("a"), tmp.join("c"), tmp.join("m"));
        let target = tmp.join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&m, "y", "body");
        write(
            &a,
            "x/SKILL.md",
            format!(
                "---\nschema: 1\nname: x\ndescription: test x\nrequires:\n  skills: [{{ name: y, source: {} }}]\n---\nbody\n",
                m.display()
            )
            .as_bytes(),
        );
        skill(&a, "z", "body");
        write(
            &c,
            "w/SKILL.md",
            format!(
                "---\nschema: 1\nname: w\ndescription: test w\nrequires:\n  skills: [{{ name: z, source: {} }}]\n---\nbody\n",
                a.display()
            )
            .as_bytes(),
        );
        install(&a, &target);
        install(&c, &target);
        fs::remove_dir_all(&m).unwrap();
        target
    }

    /// Without an interrupt the local cascade layout skips `c` for its
    /// requirement; with an interrupt at the cascade check, no such skip is
    /// recorded.
    #[test]
    fn an_interrupt_before_the_cascade_records_no_cascade_skip() {
        let tmp = tempfile::tempdir().unwrap();
        let target = stage_local_cascade(tmp.path());

        // Count the queries of a run without an interrupt; the last one is
        // the check before the cascade.
        let mut counting = InterruptedAt {
            at: usize::MAX,
            queries: std::cell::Cell::new(0),
        };
        let report = run(&target, &mut counting).unwrap();
        assert!(
            report
                .skipped_sources
                .iter()
                .any(|s| s.cause == SkipCause::RequiresSkipped),
            "{:?}",
            report.skipped_sources
        );
        let last = counting.queries.get() - 1;

        let mut observer = InterruptedAt {
            at: last,
            queries: std::cell::Cell::new(0),
        };
        let err = run(&target, &mut observer).unwrap_err();
        assert!(format!("{err:#}").contains("interrupted"), "{err:#}");
        // The skips recorded before the interrupt are still carried: `a`
        // and `m`, which could not be fetched; `c` is not skipped.
        let skipped = &err
            .downcast_ref::<SkippedBeforeFailure>()
            .expect("the skips before the interrupt are carried")
            .sources;
        let got: Vec<(String, SkipCause)> = skipped
            .iter()
            .map(|s| (s.source.clone(), s.cause))
            .collect();
        let label = |dir: &str| InstallSource::LocalPath(tmp.path().join(dir)).to_string();
        assert_eq!(
            got,
            vec![
                (label("a"), SkipCause::Unreachable),
                (label("m"), SkipCause::Unreachable),
            ]
        );
    }

    /// #504: a source whose preview changes nothing and that is not applied
    /// (the decision of a terminal run) still gets `required_by` refreshed,
    /// through `PreparedInstall::record_commit`, also for a local path.
    #[test]
    fn a_source_without_changes_that_is_not_applied_refreshes_required_by() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        write(
            &src,
            "a/AGENT.md",
            b"---\nschema: 1\nname: a\ndescription: test a\nrequires:\n  skills: [x]\n---\nbody\n",
        );
        skill(&src, "x", "body");
        install(&src, &target);
        let mut lock = crate::lockfile::Lockfile::load(&target).unwrap();
        for item in &mut lock.items {
            item.required_by.clear();
        }
        lock.save(&target).unwrap();

        let mut observer = Recorder::new(|preview: &SourcePreview| {
            assert!(!preview.has_changes(), "{preview:?}");
            SourceDecision::Skip
        });
        run(&target, &mut observer).unwrap();

        let lock = crate::lockfile::Lockfile::load(&target).unwrap();
        let x = lock.items.iter().find(|i| i.name == "x").unwrap();
        assert_eq!(x.required_by, vec!["agent:a".to_string()]);
    }

    /// Installs agent `a`, which requires skill `x`, from a new source in
    /// `tmp`, and returns `(src, target)`.
    fn install_a_requiring_x(tmp: &Path) -> (PathBuf, PathBuf) {
        let src = tmp.join("src");
        let target = tmp.join("t");
        fs::create_dir_all(&target).unwrap();
        write(
            &src,
            "a/AGENT.md",
            b"---\nschema: 1\nname: a\ndescription: test a\nrequires:\n  skills: [x]\n---\nbody\n",
        );
        skill(&src, "x", "body");
        install(&src, &target);
        (src, target)
    }

    /// #504: a source at which the run stops gets no `required_by` refresh.
    #[test]
    fn a_source_at_which_the_run_stops_refreshes_no_required_by() {
        let tmp = tempfile::tempdir().unwrap();
        let (_src, target) = install_a_requiring_x(tmp.path());
        let mut lock = crate::lockfile::Lockfile::load(&target).unwrap();
        for item in &mut lock.items {
            item.required_by.clear();
        }
        lock.save(&target).unwrap();
        let before = lock_bytes(&target);

        let mut observer = Recorder::new(|_: &SourcePreview| SourceDecision::Stop);
        run(&target, &mut observer).unwrap();

        assert_eq!(lock_bytes(&target), before);
    }

    /// #504: a local source without changes that is not applied does not
    /// save the lockfile when no `required_by` value differs. A local source
    /// has no commit; the commit comparison is pinned with a git source in
    /// `tests/cli_update_check.rs`.
    #[test]
    fn a_local_source_without_changes_saves_no_lockfile_when_required_by_is_current() {
        let tmp = tempfile::tempdir().unwrap();
        let (_src, target) = install_a_requiring_x(tmp.path());
        // A compact form that a save would rewrite as pretty-printed JSON.
        let lock = crate::lockfile::Lockfile::load(&target).unwrap();
        let compact = serde_json::to_string(&lock).unwrap();
        fs::write(target.join(".upskill-lock.json"), &compact).unwrap();

        let mut observer = Recorder::new(|_: &SourcePreview| SourceDecision::Skip);
        run(&target, &mut observer).unwrap();

        assert_eq!(lock_bytes(&target), compact.as_bytes());
    }

    /// Installs `a` requiring `x`, clears `required_by` in the lockfile and
    /// returns `(src, target, lockfile bytes)`.
    fn install_with_an_empty_required_by(tmp: &Path) -> (PathBuf, PathBuf, Vec<u8>) {
        let (src, target) = install_a_requiring_x(tmp);
        let mut lock = crate::lockfile::Lockfile::load(&target).unwrap();
        for item in &mut lock.items {
            item.required_by.clear();
        }
        lock.save(&target).unwrap();
        let bytes = lock_bytes(&target);
        (src, target, bytes)
    }

    /// #504: a dry run refreshes no `required_by`.
    #[test]
    fn a_dry_run_refreshes_no_required_by() {
        let tmp = tempfile::tempdir().unwrap();
        let (_src, target, before) = install_with_an_empty_required_by(tmp.path());
        let mut observer = Recorder::new(|_: &SourcePreview| SourceDecision::Skip);
        run_with(&target, false, true, &mut observer).unwrap();
        assert_eq!(lock_bytes(&target), before);
    }

    /// #504: a source with changes that the answer does not apply refreshes
    /// no `required_by`.
    #[test]
    fn a_declined_source_refreshes_no_required_by() {
        let tmp = tempfile::tempdir().unwrap();
        let (src, target, before) = install_with_an_empty_required_by(tmp.path());
        skill(&src, "x", "changed");
        let mut observer = Recorder::new(|preview: &SourcePreview| {
            assert!(preview.has_changes(), "{preview:?}");
            SourceDecision::Skip
        });
        run(&target, &mut observer).unwrap();
        assert_eq!(lock_bytes(&target), before);
    }

    #[test]
    fn the_commit_decides_again_after_the_preview() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "first");
        install(&src, &target);
        skill(&src, "a", "second");

        let edited = target.join(claude_skill("a"));
        let mut observer = Recorder::new(|preview: &SourcePreview| {
            let item = preview.items.iter().find(|i| i.name == "a").unwrap();
            assert!(item.write.contains(&claude_skill("a")), "{item:?}");
            fs::write(&edited, "my edit\n").unwrap();
            SourceDecision::Apply
        });
        let report = run(&target, &mut observer).unwrap();
        assert!(
            matches!(
                status_of(&report, "a"),
                UpdateStatus::SkippedModified { .. }
            ),
            "{:?}",
            report.items
        );
        assert_eq!(
            fs::read(target.join(claude_skill("a"))).unwrap(),
            b"my edit\n"
        );
        assert_eq!(observer.results.len(), 1);
        assert_eq!(observer.results[0].skipped, 1);
    }

    #[test]
    fn stop_does_not_preview_or_apply_later_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let first = tmp.path().join("s1");
        let second = tmp.path().join("s2");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&first, "a", "a");
        skill(&second, "b", "b");
        install(&first, &target);
        install(&second, &target);
        skill(&first, "a", "a changed");
        skill(&second, "b", "b changed");
        let a_before = fs::read(target.join(claude_skill("a"))).unwrap();
        let b_before = fs::read(target.join(claude_skill("b"))).unwrap();

        // Every source is fetched before the first preview; a stop at the
        // first source previews and applies no later source. A later
        // preview would get `Apply`, so a later source that is still
        // handled after the stop is written.
        let mut calls = 0;
        let mut observer = Recorder::new(move |_: &SourcePreview| {
            calls += 1;
            if calls == 1 {
                SourceDecision::Stop
            } else {
                SourceDecision::Apply
            }
        });
        let report = run(&target, &mut observer).unwrap();
        assert!(report.stopped);
        assert_eq!(observer.previews.len(), 1);
        assert_eq!(fs::read(target.join(claude_skill("a"))).unwrap(), a_before);
        assert_eq!(fs::read(target.join(claude_skill("b"))).unwrap(), b_before);
        assert!(observer.results.is_empty());
        assert_eq!(report.sources_considered, 1);
        assert_eq!(report.sources_applied, 0);
    }

    #[test]
    fn a_source_is_fetched_once() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "first");
        install(&src, &target);
        skill(&src, "a", "second");

        // A second fetch of the source would fail: the directory is gone.
        let mut observer = Recorder::new(|_: &SourcePreview| {
            fs::remove_dir_all(&src).unwrap();
            SourceDecision::Apply
        });
        let report = run(&target, &mut observer).unwrap();
        assert!(
            matches!(status_of(&report, "a"), UpdateStatus::Updated { .. }),
            "{:?}",
            report.items
        );
        let text = fs::read_to_string(target.join(claude_skill("a"))).unwrap();
        assert!(text.contains("second"), "{text}");
        assert_eq!(observer.previews.len(), 1);
        assert_eq!(observer.results[0].updated, 1);
        assert_eq!(report.sources_applied, 1);
    }

    #[test]
    fn preview_lists_a_removed_item_and_its_kept_file() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "a");
        skill(&src, "b", "b");
        install(&src, &target);
        fs::write(target.join(claude_skill("b")), "my edit\n").unwrap();
        fs::remove_dir_all(src.join("b")).unwrap();
        let lock_before = lock_bytes(&target);

        let mut observer = Recorder::new(|_: &SourcePreview| SourceDecision::Skip);
        let report = run(&target, &mut observer).unwrap();
        let preview = &observer.previews[0];
        let removed = preview.items.iter().find(|i| i.name == "b").unwrap();
        assert_eq!(removed.action, PreviewAction::Remove);
        assert_eq!(removed.kept_modified, vec![claude_skill("b")]);
        assert!(!removed.delete.contains(&claude_skill("b")), "{removed:?}");
        assert!(removed.forced.is_empty());
        assert!(preview.has_changes());
        assert_eq!(
            status_of(&report, "b"),
            &UpdateStatus::NotApplied {
                action: PreviewAction::Remove
            }
        );
        assert_eq!(report.sources_not_applied, 1);
        // Nothing was applied.
        assert_eq!(lock_bytes(&target), lock_before);
        assert_eq!(
            fs::read(target.join(claude_skill("b"))).unwrap(),
            b"my edit\n"
        );
        assert!(target.join(".claude/skills/b").is_dir());
    }

    #[test]
    fn skip_only_source_has_no_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "first");
        install(&src, &target);
        fs::write(target.join(claude_skill("a")), "my edit\n").unwrap();
        skill(&src, "a", "second");

        let mut observer = Recorder::new(|_: &SourcePreview| SourceDecision::Skip);
        let report = run(&target, &mut observer).unwrap();
        let preview = &observer.previews[0];
        assert_eq!(preview.items.len(), 1, "{:?}", preview.items);
        assert_eq!(preview.items[0].action, PreviewAction::Skip);
        assert_eq!(preview.items[0].blocking, vec![claude_skill("a")]);
        assert!(!preview.has_changes());
        assert!(
            matches!(
                status_of(&report, "a"),
                UpdateStatus::SkippedModified { .. }
            ),
            "{:?}",
            report.items
        );
        assert_eq!(report.sources_not_applied, 0);
    }

    fn preview_with(items: Vec<PreviewItem>, notes: Vec<PathBuf>) -> SourcePreview {
        SourcePreview {
            source: "s".to_string(),
            items,
            kept_notes: notes,
            handshake: Vec::new(),
            adopted: None,
        }
    }

    fn preview_item(action: PreviewAction) -> PreviewItem {
        PreviewItem {
            kind: ItemKind::Skill,
            name: "a".to_string(),
            action,
            write: Vec::new(),
            delete: Vec::new(),
            kept_modified: Vec::new(),
            forced: Vec::new(),
            blocking: Vec::new(),
            record_only: false,
        }
    }

    #[test]
    fn has_changes_follows_the_rule() {
        // A note is not a change.
        assert!(!preview_with(vec![], vec![claude_skill("a")]).has_changes());
        // A skip is not a change.
        assert!(!preview_with(vec![preview_item(PreviewAction::Skip)], vec![]).has_changes());
        // An update item with only a kept file is not a change.
        let mut kept_only = preview_item(PreviewAction::Update);
        kept_only.kept_modified = vec![claude_skill("a")];
        assert!(!preview_with(vec![kept_only], vec![]).has_changes());
        let changes: [fn(&mut PreviewItem); 6] = [
            |i| i.write = vec![claude_skill("a")],
            |i| i.delete = vec![claude_skill("a")],
            |i| i.forced = vec![claude_skill("a")],
            |i| i.record_only = true,
            |i| i.action = PreviewAction::Complete,
            |i| i.action = PreviewAction::Remove,
        ];
        for change in changes {
            let mut item = preview_item(PreviewAction::Update);
            change(&mut item);
            assert!(preview_with(vec![item], vec![]).has_changes());
        }
        let mut handshake = preview_with(vec![], vec![]);
        handshake.handshake = vec![PathBuf::from("CLAUDE.md")];
        assert!(handshake.has_changes());
    }

    #[test]
    fn a_kept_stale_file_without_another_change_is_a_note() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "a");
        write(&src, "a/notes.md", b"notes");
        install(&src, &target);
        let notes = Path::new(".claude")
            .join("skills")
            .join("a")
            .join("notes.md");
        fs::write(target.join(&notes), "my notes\n").unwrap();
        fs::remove_file(src.join("a/notes.md")).unwrap();
        skill(&src, "a", "a, changed");
        // The first update writes the new version and keeps the edited file.
        run(
            &target,
            &mut Recorder::new(|_: &SourcePreview| SourceDecision::Apply),
        )
        .unwrap();

        let mut observer = Recorder::new(|_: &SourcePreview| SourceDecision::Skip);
        let report = run(&target, &mut observer).unwrap();
        let preview = &observer.previews[0];
        assert!(preview.items.is_empty(), "{:?}", preview.items);
        assert!(preview.kept_notes.contains(&notes), "{preview:?}");
        assert!(!preview.has_changes());
        assert_eq!(status_of(&report, "a"), &UpdateStatus::UpToDate);
        assert!(report.kept_modified.contains(&notes));
        assert_eq!(report.sources_not_applied, 0);
    }

    #[test]
    fn a_declined_source_reports_a_skip_item_as_not_applied() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "a");
        skill(&src, "b", "b");
        install(&src, &target);
        fs::write(target.join(claude_skill("a")), "my edit\n").unwrap();
        skill(&src, "a", "a, changed");
        skill(&src, "b", "b, changed");

        for decision in [SourceDecision::Skip, SourceDecision::Stop] {
            let mut observer = Recorder::new(|_: &SourcePreview| decision);
            let report = run(&target, &mut observer).unwrap();
            assert!(observer.previews[0].has_changes());
            assert_eq!(
                status_of(&report, "a"),
                &UpdateStatus::NotApplied {
                    action: PreviewAction::Skip
                }
            );
            assert_eq!(
                status_of(&report, "b"),
                &UpdateStatus::NotApplied {
                    action: PreviewAction::Update
                }
            );
        }
    }

    /// Two sources `s1` (skill `a`) and `s2` (skill `b`), both changed
    /// after the install.
    fn two_changed_sources() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        for (dir, name) in [("s1", "a"), ("s2", "b")] {
            let src = tmp.path().join(dir);
            skill(&src, name, "first");
            install(&src, &target);
            skill(&src, name, "second");
        }
        (tmp, target)
    }

    #[test]
    fn a_dry_run_ignores_the_decision() {
        // With `dry_run`, the decision is ignored: `Apply` writes nothing,
        // and `Stop` does not stop, so every source is still previewed.
        for decision in [SourceDecision::Apply, SourceDecision::Stop] {
            let (_tmp, target) = two_changed_sources();
            let lock_before = lock_bytes(&target);
            let a_before = fs::read(target.join(claude_skill("a"))).unwrap();
            let mut observer = Recorder::new(|_: &SourcePreview| decision);
            let report = run_with(&target, false, true, &mut observer).unwrap();
            assert_eq!(observer.previews.len(), 2, "{decision:?}");
            assert!(observer.results.is_empty());
            assert!(!report.stopped);
            assert_eq!(report.sources_applied, 0);
            assert_eq!(report.sources_not_applied, 2);
            assert_eq!(lock_bytes(&target), lock_before);
            assert_eq!(fs::read(target.join(claude_skill("a"))).unwrap(), a_before);
        }
    }

    #[test]
    fn the_adoption_is_on_the_preview_of_the_first_source_only() {
        let (_tmp, target) = two_changed_sources();
        let path = target.join(".upskill-lock.json");
        let mut lock: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        lock["schema"] = 1.into();
        for entry in lock["items"].as_array_mut().unwrap() {
            entry.as_object_mut().unwrap().remove("files");
        }
        fs::write(&path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();

        let mut observer = Recorder::new(|_: &SourcePreview| SourceDecision::Skip);
        let report = run(&target, &mut observer).unwrap();
        assert_eq!(observer.previews.len(), 2);
        assert!(
            observer.previews[0]
                .adopted
                .as_ref()
                .is_some_and(|paths| !paths.is_empty())
        );
        assert!(observer.previews[1].adopted.is_none());
        assert!(report.adopted.is_some());
    }

    #[test]
    fn a_removed_item_with_overwrite_divides_forced_and_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "a");
        skill(&src, "b", "b");
        install(&src, &target);
        fs::write(target.join(claude_skill("b")), "my edit\n").unwrap();
        fs::remove_dir_all(src.join("b")).unwrap();

        let mut observer = Recorder::new(|_: &SourcePreview| SourceDecision::Skip);
        run_with(&target, true, false, &mut observer).unwrap();
        let removed = observer.previews[0]
            .items
            .iter()
            .find(|i| i.name == "b")
            .unwrap();
        assert_eq!(removed.action, PreviewAction::Remove);
        assert_eq!(removed.forced, vec![claude_skill("b")]);
        assert!(!removed.delete.contains(&claude_skill("b")), "{removed:?}");
        assert!(!removed.delete.is_empty(), "the files of the other clients");
        assert!(removed.kept_modified.is_empty());
    }
}
