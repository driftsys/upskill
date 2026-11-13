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

use super::apply::{check_no_item_symlink_above, deletable, delete_recorded, prune_empty_parents};
use super::output::{output_path, released_output_path};
use super::owned::{FileState, adopt_schema1, adopt_schema1_in_memory, classify};
use super::prepared::prepare_install;
use super::{
    ALL_CLIENTS, AddOptions, AdoptedBeforeFailure, BothScopesItem, DoctorReport, DoctorScopes,
    EarlierLayoutFile, InterruptedItem, ItemKind, ListReport, ListedBundle, ListedItem,
    MissingOutput, MissingPlugin, ModifiedFile, OrphanEntry, OrphanReason, OrphanedDependency,
    PreviewAction, PreviewItem, RemoveFilter, RemoveReport, RemovedItem, ScopedDoctor, ScopedList,
    SkippedPlugin, SourceDecision, SourcePreview, SourceResult, StaleHash, UnreadableFile,
    UpdateObserver, UpdateReport, UpdateStatus, UpdatedItem, hash_item_dir,
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
/// Before any change on disk, every recorded path is checked: a path that
/// is not accepted by `deletable` is skipped, and a symbolic link in the
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
/// Ancillary files (`CLAUDE.md`, `opencode.json`,
/// `.vscode/settings.json`) are deliberately left alone — they are
/// user-owned after creation per ADR-0003.
pub fn remove(target: &Path, filter: RemoveFilter, force: bool) -> Result<RemoveReport> {
    let mut lock = crate::lockfile::Lockfile::load(target)?;

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

    let keys: Vec<(ItemKind, String)> =
        to_remove.iter().map(|i| (i.kind, i.name.clone())).collect();

    // The adoption is saved before any file is deleted. Every later error
    // carries the adopted paths, so that `main.rs` can still print the adoption notice.
    let adopted = adopt_schema1(target, &mut lock)?;
    let result = remove_selected(target, &mut lock, &filter, &keys, force);
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
    target: &Path,
    lock: &mut crate::lockfile::Lockfile,
    filter: &RemoveFilter,
    keys: &[(ItemKind, String)],
    force: bool,
) -> Result<RemoveReport> {
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
            let _ = crate::ancillary::remove_opencode_plugin_uri(target, &plugin.identifier);
        }
    }
    lock.plugins
        .retain(|p| !bundles_to_remove.contains(&p.bundle));
    lock.bundles
        .retain(|b| !bundles_to_remove.contains(&b.name));

    lock.save(target)?;
    Ok(report)
}

/// Verify installed-state consistency against `.upskill-lock.json`.
/// The report has these buckets:
/// - **missing outputs** — file paths the lockfile says should exist
///   but don't: every recorded path that is absent, or, for an item
///   with an empty `files` record, each per-client entrypoint that is
///   absent. A recorded path that `deletable` rejects is skipped without
///   a report, here and in the modified and unreadable files.
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
///   is gone, or at an item that no longer exists in its source. The
///   user has to `upskill remove` to clear it.
/// - **orphaned dependencies** — items pulled in only as a dependency of
///   items that are no longer installed. Does not affect `is_clean`.
/// - **missing plugins** — plugins recorded as installed that the client
///   no longer has.
/// - **skipped plugins** (advisory) — plugins that were never installed
///   because the client CLI was missing.
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
///
/// Doctor never adopts a `schema: 1` lockfile and never saves. Doctor
/// never fetches. Remote-source drift is detected by
/// `update --dry-run`, which does fetch.
pub fn doctor(at: &ScopeRoot) -> Result<DoctorReport> {
    let target = at.root.as_path();
    let lock = crate::lockfile::Lockfile::load(target)?;
    let mut report = DoctorReport::default();

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
            let ssot_root = Path::new(local_path);
            if !ssot_root.is_dir() {
                report.orphan_entries.push(OrphanEntry {
                    kind,
                    name: entry.name.clone(),
                    source: entry.source.clone(),
                    reason: OrphanReason::LocalPathGone,
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
            let item_dir = ssot_root.join(folder);
            if !item_dir.is_dir() {
                report.orphan_entries.push(OrphanEntry {
                    kind,
                    name: entry.name.clone(),
                    source: entry.source.clone(),
                    reason: OrphanReason::ItemMissingInSource,
                });
                continue;
            }
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
        // here, see `update --dry-run`.
    }

    // -- Orphaned dependencies (advisory, issue #196) --
    // An item pulled in only as a dependency (`required_by` non-empty) whose
    // every recorded requirer is no longer installed. Surfaced as advisory
    // only — upskill never auto-removes; the user decides.
    let installed: std::collections::BTreeSet<String> = lock
        .items
        .iter()
        .map(|i| format!("{}:{}", i.kind, i.name))
        .collect();
    for entry in &lock.items {
        if entry.required_by.is_empty() {
            continue;
        }
        if entry.required_by.iter().all(|r| !installed.contains(r)) {
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
    // - skipped_plugins: status == Skipped (CLI was absent at install time).
    //   Always surface; does not affect is_clean().
    // - missing_plugins: status == Installed but the client no longer has the
    //   plugin.  This is drift → is_clean() returns false → exit 1.
    for plugin in &lock.plugins {
        use crate::lockfile::PluginInstallStatus;
        use crate::plugin::{
            PluginScope, check_claude_plugin_installed, check_opencode_plugin_installed,
            check_vscode_extension_installed,
        };

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
                });
            }
            PluginInstallStatus::Installed => {
                // Query the client to verify the plugin is still there.
                let check = match plugin.client.as_str() {
                    "claude" => {
                        let scope = match plugin.scope.as_deref() {
                            Some("user") => PluginScope::User,
                            _ => PluginScope::Project,
                        };
                        check_claude_plugin_installed(&plugin.name, scope)
                    }
                    "vscode" => check_vscode_extension_installed(&plugin.identifier),
                    "opencode" => check_opencode_plugin_installed(&plugin.identifier),
                    // Unknown client — skip silently.
                    _ => continue,
                };
                if check.is_not_installed() {
                    report.missing_plugins.push(MissingPlugin {
                        name: plugin.name.clone(),
                        client: plugin.client.clone(),
                        identifier: plugin.identifier.clone(),
                        bundle: plugin.bundle.clone(),
                    });
                }
                // CliNotFound or QueryFailed: cannot determine state — skip
                // silently to avoid false positives.
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
                    _ => crate::ancillary::opencode_mcp_state(target, &mcp.name),
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
    })
}

/// Rebuild the client selection a bare `update` should reuse for one source:
/// the union of its entries' recorded generation clients. An entry with an
/// empty `clients` list was installed for all clients, so it widens the result
/// back to all — the safe direction (never drops an entry's output trees).
/// Returns [`ClientSelection::all`] when nothing narrows it (ADR-0012).
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

/// Re-fetch every source recorded in `<target>/.upskill-lock.json` and
/// handle the sources one at a time, sorted by source label. `update`
/// always fetches per ADR-0004 — there is no `--offline`.
///
/// `names` selects which lockfile entries to update; empty means "all".
/// When names are given, the entries' source labels are used to fetch
/// — so `update foo` may also re-render other items installed from the
/// same source (the pipeline always reinstalls a source as a unit).
/// Names that match no lockfile entry are an error.
///
/// For each source, in this order:
///
/// 1. The source is fetched once (a source of a cross-source dependency is
///    fetched again when the reinstall of this source resolves it), resolved, checked (conflict check and
///    overlap check) and planned, without any write (`prepare_install`).
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
///    later source is fetched.
///
/// In an applied source, `overwrite` (`update --force`) replaces modified
/// and unowned files. Without it, an item with such a file at one of its
/// destinations is not written. When a source hash is recorded for it and
/// its source hash equals it, it has no `pending` record, every such file
/// is modified (none is unowned), nothing else of the item would be
/// written or deleted, no stale file of the item would be deleted or kept,
/// and no field of its lockfile record other than `files`, `pending` and
/// `required_by` would change (`apply::same_record`), the modified
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
///
/// An error after an adoption carries the adopted paths as
/// [`AdoptedBeforeFailure`]. Sources applied before an error stay applied.
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

    let mut report = UpdateReport::default();
    // The adoption of a `schema: 1` lockfile is saved. A later error must
    // still carry the adopted paths for the adoption notice.
    let result = (|| -> Result<()> {
        let total_sources = by_source.len();
        for (index, (source_label, source_entries)) in by_source.into_iter().enumerate() {
            let source = crate::source::parse_install_source_label(&source_label)
                .with_context(|| format!("parse lockfile source label `{source_label}`"))?;

            // Build aliases from lockfile entries that have source_name
            let aliases: Vec<(String, String)> = source_entries
                .iter()
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
            let effective_selection = if selection.is_all() {
                reselect_from_entries(&source_entries)
            } else {
                selection.clone()
            };
            let options = AddOptions {
                force: true,
                overwrite,
                aliases,
                excludes: vec![],
                selection: effective_selection,
                keep_unwithheld_edits: true,
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
            // Bundle sources are exempt: a bundle is its own unit and must
            // be reinstalled as a whole (an empty filter routes it through
            // the bundle-resolution path), so scoping to item names there
            // would mis-route a `.bundle.yaml` source into name resolution.
            let is_bundle_source = source_label.ends_with(crate::parse::bundle::BUNDLE_SUFFIX)
                || lock.bundles.iter().any(|b| b.source == source_label);
            let item_filter: Vec<String> = if is_bundle_source {
                Vec::new()
            } else {
                source_entries
                    .iter()
                    .map(|e| e.source_name.clone().unwrap_or_else(|| e.name.clone()))
                    .collect()
            };

            // Fetch, resolve, check and plan. A failure here (for example
            // the overlap check) happens before the adoption.
            let prepared = prepare_install(&source, target, &item_filter, &options)?;
            let new_hashes = prepared.planned_hashes();
            guard_against_empty_source(&new_hashes, &source_entries, &source_label)?;

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
            let install_preview = prepared.preview(target, &current)?;
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
                        source: source_label.clone(),
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
            let install_report = prepared.commit(target, plugin_scope)?;
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
                for plan in plans {
                    report
                        .changed
                        .extend(execute_removal(target, &plan, &mut reported)?);
                    report.kept_modified.extend(plan.kept.iter().cloned());
                    report.forced.extend(plan.forced.iter().cloned());
                    lock.remove(plan.kind, &plan.name);
                    report.items.push(UpdatedItem {
                        kind: plan.kind,
                        name: plan.name,
                        source: source_label.clone(),
                        status: UpdateStatus::Removed,
                    });
                }
                lock.save(target)?;
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
                let status = if let Some(files) = skipped.remove(&(kind, entry.name.clone())) {
                    UpdateStatus::SkippedModified { files }
                } else if new_hash == entry.hash && !entry.pending.is_empty() {
                    UpdateStatus::CompletedInterrupted
                } else if new_hash == entry.hash {
                    UpdateStatus::UpToDate
                } else {
                    UpdateStatus::Updated {
                        old_hash: entry.hash.clone(),
                        new_hash,
                    }
                };
                report.items.push(UpdatedItem {
                    kind,
                    name: entry.name.clone(),
                    source: source_label.clone(),
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
                match item.status {
                    UpdateStatus::Updated { .. } => result.updated += 1,
                    UpdateStatus::CompletedInterrupted => result.completed += 1,
                    UpdateStatus::Removed => result.removed += 1,
                    UpdateStatus::SkippedModified { .. } => result.skipped += 1,
                    UpdateStatus::UpToDate | UpdateStatus::NotApplied { .. } => {}
                }
            }
            observer.applied(&result);
        }
        Ok(())
    })();
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
        return Err(match &report.adopted {
            Some(paths)
                if !paths.is_empty() && err.downcast_ref::<AdoptedBeforeFailure>().is_none() =>
            {
                err.context(AdoptedBeforeFailure {
                    paths: paths.clone(),
                })
            }
            _ => err,
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

/// Guard against the data-loss footgun in issue #196: when re-resolving a
/// source yields **no** items but the lockfile still records entries for
/// it, treat that as a fetch/resolution anomaly and abort rather than
/// scheduling every entry for removal. A source whose items were all
/// genuinely deleted upstream is indistinguishable from a transient
/// failure here, so erring toward "do not delete" is the safe default —
/// the user can `upskill remove --source <label>` to clear such entries
/// deliberately.
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
    Ok(report)
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
    fn stop_does_not_fetch_later_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let first = tmp.path().join("s1");
        let second = tmp.path().join("s2");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&first, "a", "a");
        skill(&second, "b", "b");
        install(&first, &target);
        install(&second, &target);
        fs::remove_dir_all(&second).unwrap();

        let mut observer = Recorder::new(|_: &SourcePreview| SourceDecision::Stop);
        let report = run(&target, &mut observer).unwrap();
        assert!(report.stopped);
        assert_eq!(observer.previews.len(), 1);
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
