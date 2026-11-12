//! One install of one source, divided into two steps so that a caller can
//! stop between them: [`prepare_install`] fetches, resolves, checks and
//! plans, and writes nothing; [`PreparedInstall::commit`] adopts a
//! `schema: 1` lockfile when needed, writes the plan, installs plugins and
//! MCP servers, saves the lockfile and writes the hand-shake files.
//! [`PreparedInstall::preview`] tells, without any change, what `commit`
//! would do.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use super::apply::{self, Disposition, PlanDecision};
use super::discovery::{self, scan_source_items};
use super::install::{
    self, DependencyClosure, ResolvedBundle, install_from_local_path_selective,
    install_mcps_from_bundles, install_plugins_from_bundles,
    install_with_name_resolution_from_local, resolve_bundle_request, resolve_requires_closure,
};
use super::plan::{self, PlannedItem};
use super::{
    AddOptions, AdoptedBeforeFailure, InstallReport, ItemKind, PreviewAction, PreviewItem,
    bundle_item_names, fetch_ssot, owned,
};
use crate::lockfile::{LockedItem, Lockfile};
use crate::plugin::PluginScope;
use crate::source::InstallSource;

/// One source after fetch, resolution, the conflict check, the overlap
/// check and planning. Owns the fetch guards. Nothing was written.
pub(super) struct PreparedInstall {
    /// The [`InstallSource`] `Display` form.
    label: String,
    /// The pinned ref of the entry source; `None` for a local path.
    git_ref: Option<String>,
    options: AddOptions,
    /// The plan, after `plan::merge_duplicates`.
    report: InstallReport,
    /// Holds the clone guards of the cross-source fetches.
    closure: Option<DependencyClosure>,
    resolved_bundles: Vec<ResolvedBundle>,
    /// Deletes the clone of the entry source when dropped.
    _tmp_guard: Option<tempfile::TempDir>,
}

/// What [`PreparedInstall::commit`] would do now. Computed without any
/// change on disk or in the lockfile.
pub(super) struct InstallPreview {
    pub items: Vec<PreviewItem>,
    /// Modified destinations of items with `Disposition::KeepEdits`.
    pub kept_edits: Vec<PathBuf>,
    pub handshake: Vec<PathBuf>,
}

/// Fetches `source`, resolves its items and dependencies, runs the
/// cross-source conflict check and the overlap check, plans every output
/// file, merges duplicate planned items and checks the installed names.
/// Writes nothing in `target`: no file, no directory and no lockfile. The conflict
/// check reads the lockfile and does not keep it.
pub(super) fn prepare_install(
    source: &InstallSource,
    target: &Path,
    items: &[String],
    options: &AddOptions,
) -> Result<PreparedInstall> {
    let label = source.to_string();

    // -- Pre-flight: fetch SSOT and scan for items before writing anything --
    let (local_source, tmp_guard) = fetch_ssot(source)?;
    let scanned_items = scan_source_items(&local_source);

    // -- Validate bare --as with multi-item source --
    if options.aliases.iter().any(|(from, _)| from.is_empty()) {
        let unique_names: BTreeSet<&str> = scanned_items
            .iter()
            .filter(|(_, n)| !options.excludes.contains(n))
            .map(|(_, n)| n.as_str())
            .collect();
        if unique_names.len() > 1 {
            anyhow::bail!(
                "--as <alias> cannot be used with a source containing multiple items ({} found). \
                 Use --as <original>=<alias> syntax to alias specific items.",
                unique_names.len()
            );
        }
    }

    // -- `requires` dependency closure (same- and cross-source) --
    // The directly-requested item set, keyed on effective names from
    // `scan_source_items` and respecting the `items` filter and excludes.
    // Bundle-file sources resolve their own items, so the item-level
    // closure is skipped for them.
    let requested: Vec<(ItemKind, String)> = {
        let mut seen = BTreeSet::new();
        scanned_items
            .iter()
            .filter(|(_, n)| !options.excludes.contains(n))
            .filter(|(_, n)| items.is_empty() || items.contains(n))
            .filter(|(kind, name)| seen.insert((*kind, name.clone())))
            .map(|(kind, name)| (*kind, name.clone()))
            .collect()
    };
    // Defer to `install_with_name_resolution_from_local` (closure skipped)
    // when positional names are given that the item closure cannot serve:
    // a name referencing a bundle by name inside a directory source (the
    // name-resolution branch detects item/bundle ambiguity and resolves
    // bundles), or a name matching nothing (so that branch emits its
    // "no matching items or bundles" error rather than silently installing
    // zero items). Bundle-file sources own their own resolution too.
    let defer_to_name_resolution = !items.is_empty()
        && !discovery::is_bundle_file(&local_source)
        && (requested.is_empty()
            || items
                .iter()
                .any(|n| discovery::find_bundle_by_name(&local_source, n).is_some()));
    let git_ref = match source {
        InstallSource::Github(r) => r.git_ref.as_deref(),
        InstallSource::Git(r) => r.git_ref.as_deref(),
        InstallSource::LocalPath(_) => None,
    };
    // A bundle-shaped request (`*.bundle.yaml` source or positional names that
    // all resolve to bundles) resolves its own — possibly cross-source — bundle
    // closure (ADR-0009). It produces the same `DependencyClosure` the
    // item-level path uses, so the install + item-lockfile machinery is shared;
    // `resolved_bundles` carries each bundle's own source for the bundle
    // lockfile recording and plugin/MCP install.
    let bundle_resolution = resolve_bundle_request(&label, &local_source, git_ref, items)?;
    let (closure, resolved_bundles): (Option<DependencyClosure>, Vec<ResolvedBundle>) =
        if let Some((c, bundles)) = bundle_resolution {
            (Some(c), bundles)
        } else if discovery::is_bundle_file(&local_source) || defer_to_name_resolution {
            (None, Vec::new())
        } else {
            (
                Some(resolve_requires_closure(
                    &label,
                    &local_source,
                    git_ref,
                    &requested,
                )?),
                Vec::new(),
            )
        };

    // Installed names of entry-source items. The first mapping given for
    // a name wins; an exact `orig=alias` wins over a bare `--as`. The
    // conflict check below and the plan use the same names.
    let mut entry_names = plan::PlanNames {
        excludes: options.excludes.clone(),
        ..plan::PlanNames::default()
    };
    for (from, to) in &options.aliases {
        entry_names
            .aliases
            .entry(from.clone())
            .or_insert_with(|| to.clone());
    }

    // -- Conflict detection (before any files are written) --
    // This copy of the lockfile is used for the check only; `commit` loads
    // the lockfile again.
    let lock = Lockfile::load(target)?;
    let mut conflicts = Vec::new();
    if let Some(closure) = &closure {
        // Group incoming items by their canonical source label; conflicts
        // are per (kind, name, source). Aliases apply only to entry-source
        // items — dependency-pulled items are never aliased.
        let mut grouped: BTreeMap<String, Vec<(ItemKind, String)>> = BTreeMap::new();
        for ((kind, name), src_label) in &closure.item_source {
            let effective = if *src_label == label {
                entry_names
                    .alias_for(name)
                    .cloned()
                    .unwrap_or_else(|| name.clone())
            } else {
                name.clone()
            };
            grouped
                .entry(src_label.clone())
                .or_default()
                .push((*kind, effective));
        }
        for (src_label, items) in &grouped {
            conflicts.extend(crate::conflict::detect_conflicts(items, &lock, src_label));
        }
    } else {
        let mut seen = BTreeSet::new();
        let incoming: Vec<(ItemKind, String)> = scanned_items
            .iter()
            .filter(|(_, n)| !options.excludes.contains(n))
            .filter(|(_, n)| items.is_empty() || items.contains(n))
            .filter_map(|(kind, name)| {
                let effective_name = entry_names
                    .alias_for(name)
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                if seen.insert((*kind, effective_name.clone())) {
                    Some((*kind, effective_name))
                } else {
                    None
                }
            })
            .collect();
        conflicts.extend(crate::conflict::detect_conflicts(&incoming, &lock, &label));
    }
    if !conflicts.is_empty() && !options.force {
        anyhow::bail!("{}", crate::conflict::format_conflict_error(&conflicts));
    }
    drop(lock);

    // -- Refuse a source item that overlaps its own output paths --
    // Runs before the adoption and before any write, so a refused install
    // changes no file. Aliases apply to entry-source items only, as in the
    // plan below.
    if let Some(closure) = &closure {
        let no_aliases = plan::PlanNames::default();
        for (src_label, si) in &closure.by_source {
            let names = if *src_label == label {
                &entry_names
            } else {
                &no_aliases
            };
            install::check_sources_outside_outputs(&si.root, target, Some(&si.items), names)?;
        }
    } else {
        // Without a closure, `items` is not empty: a bundle-file source always
        // gets a closure, and only a request with names defers to name
        // resolution.
        install::check_name_resolution_outside_outputs(&local_source, target, items, &entry_names)?;
    }

    // -- Plan every output; nothing is written until `commit` --
    // With a closure, plan each source's resolved items from that
    // source's own fetched root. Without one (bundle file / name
    // resolution), keep the existing dispatch. Aliases (source name ->
    // installed name; the key "" applies to every item) apply only to
    // entry-source items: dependency-pulled (cross-source) items are never
    // renamed. Without a closure, every item is entry-source. `--exclude`
    // matches source names and applies to every source.
    let dependency_names = plan::PlanNames {
        excludes: options.excludes.clone(),
        ..plan::PlanNames::default()
    };
    let mut report = if let Some(closure) = &closure {
        // The cross-source clone guards in `closure.guards` are held alive by
        // the owned `closure`, which `PreparedInstall` keeps; planning reads
        // every source file into memory, and dropping the guards early would
        // delete the fetched roots before they are read.
        let mut r = InstallReport::default();
        for (src_label, si) in &closure.by_source {
            let names = if *src_label == label {
                &entry_names
            } else {
                &dependency_names
            };
            let part = install_from_local_path_selective(
                &si.root,
                Some(&si.items),
                &options.selection,
                names,
            )?;
            r.items.extend(part.items);
            r.planned.extend(part.planned);
            r.bundles.extend(part.bundles);
            r.selection_skipped.extend(part.selection_skipped);
        }
        r
    } else if items.is_empty() {
        install_from_local_path_selective(&local_source, None, &options.selection, &entry_names)?
    } else {
        install_with_name_resolution_from_local(
            &local_source,
            items,
            &options.selection,
            &entry_names,
        )?
    };

    // An item planned twice from the same source item (for example named
    // and also a member of a named bundle) is kept once. Two items with one
    // installed name fail here, before the adoption.
    plan::merge_duplicates(&mut report.planned);
    apply::check_installed_names(&report.planned)?;

    Ok(PreparedInstall {
        label,
        git_ref: git_ref.map(str::to_string),
        options: options.clone(),
        report,
        closure,
        resolved_bundles,
        _tmp_guard: tmp_guard,
    })
}

impl PreparedInstall {
    /// `(kind, installed name) -> source hash` of every planned item.
    pub fn planned_hashes(&self) -> BTreeMap<(ItemKind, String), Option<String>> {
        self.report
            .items
            .iter()
            .map(|it| ((it.kind, it.name.clone()), it.source_hash.clone()))
            .collect()
    }

    /// Whether the install includes a rule: the `has_rules` argument of the
    /// hand-shake steps.
    fn has_rules(&self) -> bool {
        self.report.items.iter().any(|i| i.kind == ItemKind::Rule)
    }

    /// The lockfile entry of each planned item, with an empty `files`
    /// record. Used by `preview` and `commit`.
    fn new_items(&self) -> Vec<LockedItem> {
        // Aliases were applied at plan time; map each alias to its source
        // name for the lockfile `source_name` and the provenance below.
        let mut aliased_names: HashMap<String, String> = HashMap::new();
        for planned in &self.report.planned {
            if let Some(original) = &planned.source_name {
                aliased_names
                    .entry(planned.name.clone())
                    .or_insert_with(|| original.clone());
            }
        }

        let hashes = self.planned_hashes();
        // Translate `requires` provenance through any applied `--as` aliases so
        // the recorded `required_by` is consistent with the post-alias names that
        // `doctor` matches against (#205). The closure keys dependencies and labels
        // requirers by ORIGINAL effective name; the plan already installed the
        // items under their alias. `aliased_names` maps `alias ->
        // original`; invert it to `original -> alias` and rewrite (a) each
        // dependency key's name and (b) each `"{kind}:{name}"` requirer label.
        let original_to_alias: HashMap<&str, &str> = aliased_names
            .iter()
            .map(|(alias, original)| (original.as_str(), alias.as_str()))
            .collect();
        let alias_label = |label: &str| -> String {
            match label.split_once(':') {
                Some((kind, name)) => match original_to_alias.get(name) {
                    Some(alias) => format!("{kind}:{alias}"),
                    None => label.to_string(),
                },
                None => label.to_string(),
            }
        };
        let provenance: BTreeMap<(ItemKind, String), Vec<String>> =
            if let Some(closure) = &self.closure {
                closure
                    .required_by
                    .iter()
                    .map(|((kind, name), requirers)| {
                        let aliased_name = original_to_alias
                            .get(name.as_str())
                            .map(|a| a.to_string())
                            .unwrap_or_else(|| name.clone());
                        let aliased_requirers =
                            requirers.iter().map(|r| alias_label(r)).collect::<Vec<_>>();
                        ((*kind, aliased_name), aliased_requirers)
                    })
                    .collect()
            } else {
                BTreeMap::new()
            };
        let item_source_lookup = |kind: ItemKind, name: &str| -> (String, Option<String>) {
            if let Some(closure) = &self.closure {
                if let Some(src_label) = closure.item_source.get(&(kind, name.to_string())) {
                    let gr = closure
                        .by_source
                        .get(src_label)
                        .and_then(|si| si.git_ref.clone());
                    return (src_label.clone(), gr);
                }
            }
            (self.label.clone(), self.git_ref.clone())
        };
        let mut new_items = crate::lockfile::items_from_report(
            &self.report,
            item_source_lookup,
            &provenance,
            |k, n| hashes.get(&(k, n.to_string())).cloned().flatten(),
        );
        for item in &mut new_items {
            if let Some(original) = aliased_names.get(&item.name) {
                item.source_name = Some(original.clone());
            }
        }
        new_items
    }

    /// What `commit` would do now, against `lock`. Writes nothing.
    ///
    /// `lock` is the lockfile that `commit` would use: for a `schema: 1`
    /// lockfile, the caller passes it after `owned::adopt_schema1_in_memory`.
    /// The preview is computed by [`apply::decide_plan`], as `commit` does;
    /// see [`preview_items`] for how each item is reported.
    pub fn preview(&self, target: &Path, lock: &Lockfile) -> Result<InstallPreview> {
        let new_items = self.new_items();
        let decision = apply::decide_plan(
            target,
            lock,
            &self.report.planned,
            self.options.overwrite,
            Some(&new_items),
            self.options.keep_unwithheld_edits,
        )?;
        let (items, kept_edits) =
            preview_items(target, lock, &self.report.planned, decision, &new_items);
        let handshake = crate::ancillary::pending_handshake_changes(target, self.has_rules())?;
        Ok(InstallPreview {
            items,
            kept_edits,
            handshake,
        })
    }

    /// Writes the prepared install: adopts a `schema: 1` lockfile if
    /// needed, writes the plan, runs the plugin and MCP steps, saves the
    /// lockfile and writes the hand-shake files.
    ///
    /// Loads the lockfile, adopts a `schema: 1` lockfile (the adoption is
    /// saved before any file is written), decides the plan again from the
    /// disk with [`apply::decide_plan`] and executes it with
    /// [`apply::execute_plan`]. Every error after the adoption carries the
    /// adopted paths as [`AdoptedBeforeFailure`], so that `main.rs` can
    /// still print the adoption notice.
    pub fn commit(self, target: &Path, plugin_scope: PluginScope) -> Result<InstallReport> {
        let mut lock = Lockfile::load(target)?;
        let adopted = owned::adopt_schema1(target, &mut lock)?;
        let result = self.commit_adopted(target, plugin_scope, lock, adopted.clone());
        match (result, adopted) {
            (Err(err), Some(paths)) if !paths.is_empty() => {
                Err(err.context(AdoptedBeforeFailure { paths }))
            }
            (result, _) => result,
        }
    }

    /// The part of `commit` after the adoption.
    fn commit_adopted(
        self,
        target: &Path,
        plugin_scope: PluginScope,
        mut lock: Lockfile,
        adopted: Option<Vec<String>>,
    ) -> Result<InstallReport> {
        let new_items = self.new_items();
        let has_rules = self.has_rules();
        let PreparedInstall {
            label,
            git_ref,
            options,
            mut report,
            closure: _closure,
            resolved_bundles,
            _tmp_guard,
        } = self;
        report.adopted = adopted;

        let decision = apply::decide_plan(
            target,
            &lock,
            &report.planned,
            options.overwrite,
            Some(&new_items),
            options.keep_unwithheld_edits,
        )?;
        let outcome = apply::execute_plan(
            target,
            &mut lock,
            &report.planned,
            decision,
            Some(&new_items),
        )?;
        let mut recorded = outcome.recorded;
        report.kept_modified = outcome.kept;
        report
            .kept_modified
            .extend(outcome.kept_edits.iter().map(|c| c.path.clone()));
        report.conflicts = outcome.skipped;
        report.forced = outcome.forced;
        report.changed = outcome.changed;
        // A skipped item keeps its lockfile entry unchanged, or gets none. An
        // item kept for its modified files keeps its lockfile entry unchanged.
        let skipped: BTreeSet<(ItemKind, String)> = report
            .conflicts
            .iter()
            .chain(&outcome.kept_edits)
            .map(|c| (c.kind, c.name.clone()))
            .collect();

        // A bundle-shaped install resolves its items per source above (no bundles
        // in the per-source reports); attach the resolved bundles so plugin/MCP
        // install and the lockfile see them with their own sources.
        if !resolved_bundles.is_empty() {
            report.bundles = resolved_bundles
                .iter()
                .map(|rb| rb.bundle.clone())
                .collect();
        }

        // -- Plugin installation (ADR-0008) --
        report.plugin_results = install_plugins_from_bundles(&report.bundles, plugin_scope, target);

        // -- MCP server configuration (ADR-0010), restricted to the selection --
        report.mcp_results =
            install_mcps_from_bundles(&report.bundles, plugin_scope, target, &options.selection);

        for mut item in new_items {
            if skipped.contains(&(item.kind, item.name.clone())) {
                continue;
            }
            item.files = recorded
                .remove(&(item.kind, item.name.clone()))
                .unwrap_or_default();
            lock.upsert(item);
        }
        if resolved_bundles.is_empty() {
            // Same-source bundle install (no cross-source provenance): every bundle
            // came from the entry source.
            for bundle in &report.bundles {
                lock.upsert_bundle(crate::lockfile::LockedBundle {
                    name: bundle.name.clone(),
                    source: label.clone(),
                    git_ref: git_ref.clone(),
                    items: bundle_item_names(bundle),
                });
            }
        } else {
            // Bundle closure: record each bundle under its OWN source and ref.
            for rb in &resolved_bundles {
                lock.upsert_bundle(crate::lockfile::LockedBundle {
                    name: rb.bundle.name.clone(),
                    source: rb.source_label.clone(),
                    git_ref: rb.git_ref.clone(),
                    items: bundle_item_names(&rb.bundle),
                });
            }
        }
        // Record installed and warn-skipped plugins in the lockfile so that
        // `doctor` can surface skipped ones and verify installed ones.
        // Plugins that failed (non-zero exit) are NOT recorded — they are
        // transient errors; the CLI is present but misbehaving.
        for pr in &report.plugin_results {
            use crate::lockfile::PluginInstallStatus;
            use crate::plugin::PluginOutcome;

            let status = match &pr.outcome {
                PluginOutcome::Success => PluginInstallStatus::Installed,
                PluginOutcome::CliNotFound => PluginInstallStatus::Skipped,
                PluginOutcome::ManualInstructions => PluginInstallStatus::Instructions,
                PluginOutcome::Failed { .. } => continue,
            };
            lock.upsert_plugin(crate::lockfile::LockedPlugin {
                name: pr.name.clone(),
                client: pr.client.clone(),
                identifier: pr.identifier.clone(),
                scope: match plugin_scope {
                    PluginScope::Project => Some("project".into()),
                    PluginScope::User => Some("user".into()),
                },
                bundle: pr.bundle.clone(),
                status,
            });
        }
        // Record configured and warn-skipped MCP servers in the lockfile.
        for mr in &report.mcp_results {
            use crate::lockfile::McpInstallStatus;
            use crate::plugin::PluginOutcome;

            let status = match &mr.outcome {
                PluginOutcome::Success => McpInstallStatus::Installed,
                PluginOutcome::CliNotFound => McpInstallStatus::Skipped,
                // ManualInstructions is unused for MCP; Failed is transient.
                PluginOutcome::ManualInstructions | PluginOutcome::Failed { .. } => continue,
            };
            lock.upsert_mcp(crate::lockfile::LockedMcp {
                name: mr.name.clone(),
                client: mr.client.clone(),
                scope: match plugin_scope {
                    PluginScope::Project => Some("project".into()),
                    PluginScope::User => Some("user".into()),
                },
                bundle: mr.bundle.clone(),
                status,
            });
        }
        lock.save(target)?;

        // Per ADR-0003 / format-spec §7.4: ensure the Claude Code bridge file
        // exists at the consumer-project root. Created once with `@AGENTS.md`
        // content, never overwritten — protects user customisations.
        crate::ancillary::ensure_claude_bridge(target)?;

        // Per ADR-0003 / format-spec §7.4: when the install includes any rule,
        // register the opencode.json `instructions[]` glob so opencode picks up
        // generated rules under `.agents/rules/`. Idempotent; preserves other
        // keys.
        crate::ancillary::ensure_opencode_rules_registered(target, has_rules)?;

        // Per ADR-0003: when the install includes any rule, register
        // `.github/instructions` in `.vscode/settings.json`'s
        // `chat.instructionsFilesLocations` so VS Code Copilot picks up the
        // generated `<name>.instructions.md` files.
        crate::ancillary::ensure_vscode_instructions_registered(target, has_rules)?;

        Ok(report)
    }
}

/// Maps each item of `decision` to a [`PreviewItem`], and returns the
/// modified destinations of the items with [`Disposition::KeepEdits`].
///
/// - `KeepEdits`: no `PreviewItem`; its conflict paths are returned.
/// - `Skip`: action `Skip`, `blocking` is the conflict paths.
/// - `Write`: `write` and `delete` are the paths that `execute_plan` would
///   count in `changed` (see [`apply::writes_new_content`] and
///   [`apply::deletes_existing`]); `kept_modified` is the kept stale files;
///   `forced` is the conflict paths (there are some only with `overwrite`).
///   The action is `Complete` when the lockfile entry has a `pending` record
///   and the planned source hash equals the recorded hash, and `Update`
///   otherwise. `record_only` is true when nothing is written or deleted,
///   `new_items` holds an entry for the item, and the lockfile record of
///   the item would change: the lockfile has no entry for the item yet, a
///   field other than `files`, `pending` and `required_by` differs (see
///   [`apply::same_record`]), or the `files` record that `execute_plan`
///   would save ([`apply::ItemChanges::recorded_files`]: the planned files
///   and the records of the kept stale files) differs from the `files`
///   record of the entry, compared as a set of (client, path, hash). An
///   item with nothing to write, delete or keep, that is not `Complete` and
///   not `record_only`, is up to date and gets no `PreviewItem`.
fn preview_items(
    target: &Path,
    lock: &Lockfile,
    planned: &[PlannedItem],
    decision: PlanDecision,
    new_items: &[LockedItem],
) -> (Vec<PreviewItem>, Vec<PathBuf>) {
    let mut items = Vec::new();
    let mut kept_edits = Vec::new();
    for (item, changes) in planned.iter().zip(decision.items) {
        let recorded = changes.recorded_files();
        let conflict_paths: Vec<PathBuf> = changes.conflicts.into_iter().map(|c| c.path).collect();
        match changes.disposition {
            Disposition::KeepEdits => {
                kept_edits.extend(conflict_paths);
                continue;
            }
            Disposition::Skip => {
                items.push(PreviewItem {
                    kind: changes.kind,
                    name: changes.name,
                    action: PreviewAction::Skip,
                    write: Vec::new(),
                    delete: Vec::new(),
                    kept_modified: Vec::new(),
                    forced: Vec::new(),
                    blocking: conflict_paths,
                    record_only: false,
                });
                continue;
            }
            Disposition::Write => {}
        }
        let write: Vec<PathBuf> = item
            .files
            .iter()
            .filter(|file| apply::writes_new_content(target, file, &changes.unchanged))
            .map(|file| file.rel.clone())
            .collect();
        let delete: Vec<PathBuf> = changes
            .delete
            .into_iter()
            .filter(|rel| apply::deletes_existing(target, rel))
            .collect();
        let kept_modified: Vec<PathBuf> = changes.kept.into_iter().map(|(rel, _)| rel).collect();
        let entry = lock
            .items
            .iter()
            .find(|i| i.kind == changes.kind && i.name == changes.name);
        let new_entry = new_items
            .iter()
            .find(|i| i.kind == changes.kind && i.name == changes.name);
        let complete = entry
            .is_some_and(|e| !e.pending.is_empty() && new_entry.is_some_and(|n| n.hash == e.hash));
        let record_only = write.is_empty()
            && delete.is_empty()
            && new_entry.is_some_and(|n| {
                entry.is_none_or(|e| {
                    !apply::same_record(n, e)
                        || recorded.iter().collect::<BTreeSet<_>>()
                            != e.files.iter().collect::<BTreeSet<_>>()
                })
            });
        if write.is_empty()
            && delete.is_empty()
            && !complete
            && kept_modified.is_empty()
            && !record_only
        {
            continue;
        }
        items.push(PreviewItem {
            kind: changes.kind,
            name: changes.name,
            action: if complete {
                PreviewAction::Complete
            } else {
                PreviewAction::Update
            },
            write,
            delete,
            kept_modified,
            forced: conflict_paths,
            blocking: Vec::new(),
            record_only,
        });
    }
    (items, kept_edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::Client;
    use crate::lockfile::OwnedFile;
    use crate::pipeline::owned::sha256_hex;
    use crate::pipeline::plan::PlannedFile;
    use std::fs;

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

    /// Every file under `root`, with its bytes.
    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    let rel = path.strip_prefix(root).unwrap().to_path_buf();
                    out.insert(rel, fs::read(&path).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    /// A source with skills `a` (with `a/notes.md`) and `b`, installed
    /// into an empty target. Then `a` is changed and `a/notes.md` is
    /// removed from the source; `b` is unchanged.
    fn installed_then_changed() -> (tempfile::TempDir, PathBuf, InstallSource) {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "first");
        write(&src, "a/notes.md", b"notes");
        skill(&src, "b", "same");
        let source = InstallSource::LocalPath(src.clone());
        prepare_install(&source, &target, &[], &AddOptions::default())
            .unwrap()
            .commit(&target, PluginScope::Project)
            .unwrap();
        skill(&src, "a", "second");
        fs::remove_file(src.join("a/notes.md")).unwrap();
        (tmp, target, source)
    }

    #[test]
    fn preview_equals_what_commit_changes() {
        let (_tmp, target, source) = installed_then_changed();
        let prepared = prepare_install(&source, &target, &[], &AddOptions::default()).unwrap();
        let preview = prepared
            .preview(&target, &Lockfile::load(&target).unwrap())
            .unwrap();
        assert_eq!(preview.items.len(), 1, "only `a` changes");
        assert_eq!(preview.items[0].name, "a");
        let previewed: BTreeSet<PathBuf> = preview
            .items
            .iter()
            .flat_map(|i| i.write.iter().chain(&i.delete).cloned())
            .collect();
        assert!(
            previewed.contains(&PathBuf::from(".claude/skills/a/notes.md")),
            "the stale file is in the preview: {previewed:?}"
        );
        let report = prepared.commit(&target, PluginScope::Project).unwrap();
        let changed: BTreeSet<PathBuf> = report.changed.into_iter().collect();
        assert_eq!(previewed, changed);
    }

    #[test]
    fn prepare_and_preview_write_nothing() {
        let (tmp, target, source) = installed_then_changed();
        let before = snapshot(tmp.path());
        let prepared = prepare_install(&source, &target, &[], &AddOptions::default()).unwrap();
        prepared
            .preview(&target, &Lockfile::load(&target).unwrap())
            .unwrap();
        drop(prepared);
        assert_eq!(snapshot(tmp.path()), before);
    }

    #[test]
    fn prepare_into_an_empty_target_writes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "first");
        let prepared = prepare_install(
            &InstallSource::LocalPath(src),
            &target,
            &[],
            &AddOptions::default(),
        )
        .unwrap();
        let preview = prepared.preview(&target, &Lockfile::new()).unwrap();
        assert!(!preview.handshake.is_empty());
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    }

    // -- The mapping of `preview_items` --

    const ENTRY: &str = ".claude/skills/s/SKILL.md";
    const NOTES: &str = ".claude/skills/s/notes.md";

    fn planned(files: &[(&str, &[u8])]) -> Vec<PlannedItem> {
        vec![PlannedItem {
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
        }]
    }

    fn owned_file(path: &str, bytes: &[u8]) -> OwnedFile {
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

    fn lock_of(items: Vec<LockedItem>) -> Lockfile {
        let mut lock = Lockfile::new();
        lock.items = items;
        lock
    }

    /// Runs `decide_plan` and `preview_items` for `plan` against `lock`;
    /// the new lockfile entry is `new`.
    fn map(
        target: &Path,
        lock: &Lockfile,
        plan: &[PlannedItem],
        new: &[LockedItem],
        overwrite: bool,
        keep: bool,
    ) -> (Vec<PreviewItem>, Vec<PathBuf>) {
        let decision = apply::decide_plan(target, lock, plan, overwrite, Some(new), keep).unwrap();
        preview_items(target, lock, plan, decision, new)
    }

    #[test]
    fn write_lists_new_files_and_existing_stale_files() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), ENTRY, b"old");
        write(tmp.path(), NOTES, b"notes");
        let lock = lock_of(vec![entry(vec![
            owned_file(ENTRY, b"old"),
            owned_file(NOTES, b"notes"),
        ])]);
        let new = vec![entry(vec![])];
        let (items, kept) = map(
            tmp.path(),
            &lock,
            &planned(&[(ENTRY, b"new")]),
            &new,
            false,
            false,
        );
        assert!(kept.is_empty());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, PreviewAction::Update);
        assert_eq!(items[0].write, vec![PathBuf::from(ENTRY)]);
        assert_eq!(items[0].delete, vec![PathBuf::from(NOTES)]);
        assert!(!items[0].record_only);
    }

    #[test]
    fn keep_edits_gives_no_item_and_lists_the_modified_file() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), ENTRY, b"my edit");
        let lock = lock_of(vec![entry(vec![owned_file(ENTRY, b"gen")])]);
        let new = lock.items.clone();
        let (items, kept) = map(
            tmp.path(),
            &lock,
            &planned(&[(ENTRY, b"gen")]),
            &new,
            false,
            true,
        );
        assert!(items.is_empty());
        assert_eq!(kept, vec![PathBuf::from(ENTRY)]);
    }

    #[test]
    fn skip_lists_the_blocking_files() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), ENTRY, b"my edit");
        let lock = lock_of(vec![entry(vec![owned_file(ENTRY, b"gen")])]);
        let new = lock.items.clone();
        let (items, kept) = map(
            tmp.path(),
            &lock,
            &planned(&[(ENTRY, b"new")]),
            &new,
            false,
            false,
        );
        assert!(kept.is_empty());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, PreviewAction::Skip);
        assert_eq!(items[0].blocking, vec![PathBuf::from(ENTRY)]);
        assert!(items[0].write.is_empty() && items[0].delete.is_empty());
    }

    #[test]
    fn forced_lists_the_replaced_modified_file() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), ENTRY, b"my edit");
        let lock = lock_of(vec![entry(vec![owned_file(ENTRY, b"gen")])]);
        let new = lock.items.clone();
        let (items, _) = map(
            tmp.path(),
            &lock,
            &planned(&[(ENTRY, b"new")]),
            &new,
            true,
            false,
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, PreviewAction::Update);
        assert_eq!(items[0].forced, vec![PathBuf::from(ENTRY)]);
        assert_eq!(items[0].write, vec![PathBuf::from(ENTRY)]);
    }

    #[test]
    fn pending_record_with_the_same_hash_is_complete() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), ENTRY, b"gen");
        let mut old = entry(vec![owned_file(ENTRY, b"gen")]);
        old.pending = vec![owned_file(ENTRY, b"gen")];
        let lock = lock_of(vec![old]);
        let new = vec![entry(vec![])];
        let (items, _) = map(
            tmp.path(),
            &lock,
            &planned(&[(ENTRY, b"gen")]),
            &new,
            false,
            false,
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, PreviewAction::Complete);
        assert!(items[0].write.is_empty());
    }

    #[test]
    fn changed_record_without_file_changes_is_record_only() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), ENTRY, b"gen");
        let lock = lock_of(vec![entry(vec![owned_file(ENTRY, b"gen")])]);
        let mut changed = entry(vec![]);
        changed.hash = Some("other".to_string());
        let (items, _) = map(
            tmp.path(),
            &lock,
            &planned(&[(ENTRY, b"gen")]),
            &[changed],
            false,
            false,
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, PreviewAction::Update);
        assert!(items[0].record_only);
        assert!(items[0].write.is_empty() && items[0].delete.is_empty());
    }

    #[test]
    fn an_unowned_file_with_the_planned_bytes_is_record_only() {
        // `commit` records the file, although it writes no new content.
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), ENTRY, b"gen");
        let lock = lock_of(vec![entry(vec![])]);
        let new = lock.items.clone();
        let (items, kept) = map(
            tmp.path(),
            &lock,
            &planned(&[(ENTRY, b"gen")]),
            &new,
            false,
            false,
        );
        assert!(kept.is_empty());
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].action, PreviewAction::Update);
        assert!(items[0].record_only);
        assert!(items[0].write.is_empty() && items[0].delete.is_empty());
    }

    #[test]
    fn absent_stale_files_are_record_only() {
        // `commit` drops the absent stale file from the `files` record.
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), ENTRY, b"gen");
        let lock = lock_of(vec![entry(vec![
            owned_file(ENTRY, b"gen"),
            owned_file(NOTES, b"notes"),
        ])]);
        let new = lock.items.clone();
        let (items, _) = map(
            tmp.path(),
            &lock,
            &planned(&[(ENTRY, b"gen")]),
            &new,
            false,
            false,
        );
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].action, PreviewAction::Update);
        assert!(items[0].record_only);
        assert!(items[0].write.is_empty() && items[0].delete.is_empty());
    }

    #[test]
    fn up_to_date_item_gives_no_item() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), ENTRY, b"gen");
        let lock = lock_of(vec![entry(vec![owned_file(ENTRY, b"gen")])]);
        let new = lock.items.clone();
        let (items, kept) = map(
            tmp.path(),
            &lock,
            &planned(&[(ENTRY, b"gen")]),
            &new,
            false,
            false,
        );
        assert!(items.is_empty());
        assert!(kept.is_empty());
    }
}
