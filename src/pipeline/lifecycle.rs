//! Lifecycle operations over `.upskill-lock.json`: `remove`, `doctor`,
//! `update`, `list`.
//!
//! These commands consult the lockfile to know what is installed and
//! what to act on. They orchestrate the leaf modules (`output`,
//! `hash`, `git`) and the top-level install coordinator
//! ([`super::install_with_lockfile`]) — they own no parsing or render
//! logic themselves.

use anyhow::{Context, Result};
use std::path::Path;

use super::git::fetch_ssot;
use super::hash::planned_source_hashes;
use super::output::{output_path, remove_item_outputs};
use super::{
    ALL_CLIENTS, AddOptions, DoctorReport, ItemKind, ListReport, ListedBundle, ListedItem,
    MissingOutput, MissingPlugin, OrphanEntry, OrphanReason, OrphanedDependency, RemoveFilter,
    RemoveReport, RemovedItem, SkippedPlugin, StaleHash, UpdateMode, UpdateReport, UpdateStatus,
    UpdatedItem, hash_item_dir, install_with_lockfile,
};

/// Remove installed content recorded in `<target>/.upskill-lock.json`,
/// matching `filter`. For each matching entry, deletes every per-client
/// output file (per [`output_path`]) and drops the entry from the
/// lockfile. Best-effort `rmdir` of empty per-item parent directories
/// (e.g., `.claude/skills/<name>/`) so the workspace stays clean.
///
/// Ancillary files (`CLAUDE.md`, `opencode.json`,
/// `.vscode/settings.json`) are deliberately left alone — they are
/// user-owned after creation per ADR-0003.
pub fn remove(target: &Path, filter: RemoveFilter) -> Result<RemoveReport> {
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
            if let Some(g) = &it.group
                && groups.contains(&(it.source.clone(), g.clone()))
                && !to_remove
                    .iter()
                    .any(|r| r.kind == it.kind && r.name == it.name)
            {
                to_remove.push(it.clone());
            }
        }
    }

    let mut report = RemoveReport::default();
    for entry in &to_remove {
        let kind = entry.kind;
        // Record which entrypoint outputs existed (for the user-facing
        // report) before deleting everything for this item.
        let mut deleted_files = Vec::new();
        for client in ALL_CLIENTS {
            let rel = output_path(kind, client, &entry.name);
            if target.join(&rel).exists() {
                deleted_files.push(rel);
            }
        }
        // Deletes the entrypoint files, directory-backed item dirs, and
        // flat-kind sibling resource namespace dirs in lockstep with install.
        remove_item_outputs(target, kind, &entry.name);
        lock.remove(entry.kind, &entry.name);
        report.items.push(RemovedItem {
            kind,
            name: entry.name.clone(),
            deleted_files,
        });
    }

    // Clean up plugins associated with removed bundles.
    let bundles_to_remove: Vec<String> = match &filter {
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
/// Three independent buckets per ADR-0004:
/// - **missing outputs** — file paths the lockfile says should exist
///   but don't. Reinstall (`upskill add <source>`) fixes it.
/// - **stale hashes** — for `local:` sources still on disk, the SSOT
///   item directory hashes to a value that does not match the lockfile.
///   `upskill update` (or `--dry-run`) fixes it.
/// - **orphan entries** — the lockfile points at a `local:` source that
///   is gone, or at an item that no longer exists in its source. The
///   user has to `upskill remove` to clear it.
///
/// Doctor never fetches. Remote-source drift is detected by
/// `update --dry-run`, which does fetch.
pub fn doctor(target: &Path) -> Result<DoctorReport> {
    let lock = crate::lockfile::Lockfile::load(target)?;
    let mut report = DoctorReport::default();

    for entry in &lock.items {
        let kind = entry.kind;

        let mut missing = Vec::new();
        for client in ALL_CLIENTS {
            // Only check clients this item was actually written for. An empty
            // `clients` list means "all" — pre-ADR-0012 lockfiles and
            // unrestricted installs both check every client. A narrowed
            // install's unselected outputs are a deliberate choice, not drift.
            if !entry.clients.is_empty() && !entry.clients.iter().any(|c| c == client.name()) {
                continue;
            }
            let rel = output_path(kind, client, &entry.name);
            if !target.join(&rel).exists() {
                missing.push(rel);
            }
        }
        if !missing.is_empty() {
            report.missing_outputs.push(MissingOutput {
                kind,
                name: entry.name.clone(),
                missing_files: missing,
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

    // -- MCP server reconciliation (ADR-0010, issue #237) --
    // Walk the lockfile's MCP entries and check whether each is still
    // configured for its target. Claude is queried via `claude mcp list`; the
    // config-write targets (copilot, vscode, opencode) are reconciled against
    // their config file. When that file is absent the state is undetermined,
    // so the entry is skipped rather than reported as drift (no false
    // positives for targets configured out-of-band).
    for mcp in &lock.mcps {
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
                    Some(true) => McpDoctorStatus::Ok,
                    Some(false) => McpDoctorStatus::NotRegistered,
                    None => continue,
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

/// Re-fetch every source recorded in `<target>/.upskill-lock.json` and
/// either reinstall (`Apply`) or report what would change (`DryRun`).
///
/// `names` selects which lockfile entries to update; empty means "all".
/// When names are given, the entries' source labels are used to fetch
/// — so `update foo` may also re-render other items installed from the
/// same source (the pipeline always reinstalls a source as a unit).
/// Names that match no lockfile entry are an error.
///
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

/// `update` always fetches per ADR-0004 — there is no `--offline`.
pub fn update(
    target: &Path,
    names: &[String],
    mode: UpdateMode,
    plugin_scope: crate::plugin::PluginScope,
    selection: &crate::select::ClientSelection,
) -> Result<UpdateReport> {
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

    // Group by source: installing or hashing once per source covers every
    // matching entry sharing it.
    let mut by_source: std::collections::BTreeMap<String, Vec<crate::lockfile::LockedItem>> =
        std::collections::BTreeMap::new();
    for entry in entries {
        by_source
            .entry(entry.source.clone())
            .or_default()
            .push(entry);
    }

    let mut report = UpdateReport::default();
    for (source_label, source_entries) in by_source {
        let source = crate::source::parse_install_source_label(&source_label)
            .with_context(|| format!("parse lockfile source label `{source_label}`"))?;

        match mode {
            UpdateMode::Apply => {
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
                    aliases,
                    excludes: vec![],
                    selection: effective_selection,
                };
                // Scope the reinstall to exactly the items this source already
                // contributes to the lockfile (by their in-source name), rather
                // than reinstalling every item the source vends. Without this, a
                // source that only contributed a single cross-source dependency
                // would pull in all of its unrelated siblings on `update`
                // (issue #211). An item's cross-source `requires` are still
                // resolved and refreshed because `install_with_lockfile`
                // re-expands the closure for each requested item.
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
                let install_report =
                    install_with_lockfile(&source, target, &item_filter, plugin_scope, &options)?;
                let mut new_hashes: std::collections::BTreeMap<(ItemKind, String), Option<String>> =
                    std::collections::BTreeMap::new();
                for it in &install_report.items {
                    new_hashes.insert((it.kind, it.name.clone()), it.source_hash.clone());
                }
                guard_against_empty_source(&new_hashes, &source_entries, &source_label)?;
                // Collect orphans first, then batch-remove from lockfile.
                let mut orphans: Vec<(&crate::lockfile::LockedItem, ItemKind)> = Vec::new();
                for entry in &source_entries {
                    let kind = entry.kind;
                    if !new_hashes.contains_key(&(kind, entry.name.clone())) {
                        orphans.push((entry, kind));
                    }
                }
                if !orphans.is_empty() {
                    let mut lock = crate::lockfile::Lockfile::load(target)?;
                    for (entry, kind) in &orphans {
                        remove_item_outputs(target, *kind, &entry.name);
                        lock.remove(entry.kind, &entry.name);
                        report.items.push(UpdatedItem {
                            kind: *kind,
                            name: entry.name.clone(),
                            source: source_label.clone(),
                            status: UpdateStatus::Removed,
                        });
                    }
                    lock.save(target)?;
                }
                for entry in &source_entries {
                    let kind = entry.kind;
                    if !new_hashes.contains_key(&(kind, entry.name.clone())) {
                        continue; // already handled as orphan
                    }
                    let new_hash = new_hashes
                        .get(&(kind, entry.name.clone()))
                        .cloned()
                        .flatten();
                    let status = if new_hash == entry.hash {
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
            }
            UpdateMode::DryRun => {
                let (root, _guard) = fetch_ssot(&source)?;
                let new_hashes = planned_source_hashes(&root)
                    .with_context(|| format!("resolve items for source `{source_label}`"))?;
                guard_against_empty_source(&new_hashes, &source_entries, &source_label)?;
                for entry in &source_entries {
                    let kind = entry.kind;
                    // The source-side hash map is keyed by the item's FOLDER
                    // (see `hash_items` / `iter_item_dirs`), which is the
                    // co-location group. A rule/agent whose frontmatter name
                    // diverges from its folder must be looked up by that folder,
                    // not its effective name, or it is falsely reported as
                    // `WouldRemove` (issue #214; mirrors the #208 doctor fix).
                    let lookup_name = entry
                        .group
                        .as_deref()
                        .or(entry.source_name.as_deref())
                        .unwrap_or(&entry.name);
                    if !new_hashes.contains_key(&(kind, lookup_name.to_string())) {
                        report.items.push(UpdatedItem {
                            kind,
                            name: entry.name.clone(),
                            source: source_label.clone(),
                            status: UpdateStatus::WouldRemove,
                        });
                        continue;
                    }
                    let new_hash = new_hashes
                        .get(&(kind, lookup_name.to_string()))
                        .cloned()
                        .flatten();
                    let status = if new_hash == entry.hash {
                        UpdateStatus::UpToDate
                    } else {
                        UpdateStatus::WouldChange {
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
            }
        }
    }

    Ok(report)
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
