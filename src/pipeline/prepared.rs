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
use super::discovery::{self, scan_source_items, scan_source_items_strict};
use super::install::{
    self, DependencyClosure, ResolvedBundle, install_from_local_path_selective,
    install_mcps_from_bundles, install_plugins_from_bundles,
    install_with_name_resolution_from_local, resolve_bundle_closure, resolve_bundle_request,
    resolve_requires_closure,
};
use super::plan::{self, PlannedItem};
use super::{
    AddOptions, AdoptedBeforeFailure, InstallReport, ItemKind, PreviewAction, PreviewItem,
    bundle_item_names, fetch_ssot, owned,
};
use crate::lockfile::{LockedItem, Lockfile, SourceStamp};
use crate::model::{PackOnlyKind, pack_only_items};
use crate::plugin::PluginScope;
use crate::source::InstallSource;

/// One source after fetch, resolution, the conflict check, the overlap
/// check and planning. Owns the fetch guards. Nothing was written.
pub(super) struct PreparedInstall {
    /// The [`InstallSource`] `Display` form.
    label: String,
    /// The pinned ref of the entry source; `None` for a local path.
    git_ref: Option<String>,
    /// The commit that the fetch of the entry source resolved; `None` for a
    /// local path.
    commit: Option<String>,
    options: AddOptions,
    /// The item names given to this run, before any alias. Empty for a
    /// whole-source or bundle-file install.
    requested: Vec<String>,
    /// The plan, after `plan::merge_duplicates`.
    report: InstallReport,
    /// Holds the clone guards of the cross-source fetches.
    closure: Option<DependencyClosure>,
    resolved_bundles: Vec<ResolvedBundle>,
    /// Recorded bundles that the fetched source no longer holds; filled only
    /// when [`AddOptions::allow_vanished`] is set.
    vanished_bundles: Vec<String>,
    /// True when the fetched entry source is a directory that holds at least
    /// one item or one bundle file.
    holds_content: bool,
    /// Every item directory of every source that this install reads, as given
    /// to the overlap check. `update` checks the items of all its sources
    /// together with [`PreparedInstall::source_items`].
    source_items: Vec<super::output::SourceItem>,
    /// The fetched root of the entry source: a directory or a bundle file.
    local_source: PathBuf,
    /// Deletes the clone of the entry source when dropped.
    _tmp_guard: Option<tempfile::TempDir>,
}

/// The `requires` provenance of one run; see
/// [`PreparedInstall::requires_provenance`].
#[derive(Debug, Default)]
struct RequiresProvenance {
    evaluated: BTreeSet<String>,
    required_by: BTreeMap<(ItemKind, String), Vec<String>>,
    /// A kind-restricted install refreshes only entries that it plans.
    restriction: Option<BTreeSet<(ItemKind, String)>>,
}

/// What [`PreparedInstall::commit`] would do now. Computed without any
/// change on disk or in the lockfile.
pub(super) struct InstallPreview {
    pub items: Vec<PreviewItem>,
    /// Notes, not changes: the modified destinations of items with
    /// `Disposition::KeepEdits`, and the kept modified stale files of
    /// written items that have no other change.
    pub kept_notes: Vec<PathBuf>,
    pub handshake: Vec<PathBuf>,
}

/// Record installed and warn-skipped plugins in `lock`. A plugin whose
/// client CLI is absent or has an unsupported version is recorded as
/// skipped. Plugins that failed (non-zero exit) are NOT recorded — they are
/// transient errors; the CLI is present but misbehaving.
pub(super) fn record_plugin_results(
    lock: &mut Lockfile,
    results: &[super::PluginResult],
    plugin_scope: PluginScope,
) {
    use crate::lockfile::PluginInstallStatus;
    use crate::plugin::PluginOutcome;

    for pr in results {
        let status = match &pr.outcome {
            PluginOutcome::Success => PluginInstallStatus::Installed,
            PluginOutcome::CliNotFound | PluginOutcome::UnsupportedVersion { .. } => {
                PluginInstallStatus::Skipped
            }
            PluginOutcome::ManualInstructions => PluginInstallStatus::Instructions,
            PluginOutcome::Failed { .. } => continue,
        };
        lock.upsert_plugin(crate::lockfile::LockedPlugin {
            config_write: pr.config_write,
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
    let fetched = fetch_ssot(source)?;
    let local_source = fetched.root;
    let tmp_guard = fetched.guard;
    let commit = fetched.commit;
    let scanned_items = if options.allow_vanished {
        // `update` removes an installed item that no plan provides, so the
        // scan is strict (see below for the strict walk of every planned
        // root): an unreadable directory stops the run instead of making its
        // items look absent.
        scan_source_items_strict(&local_source)?
    } else {
        scan_source_items(&local_source)?
    };
    let holds_content = !discovery::is_bundle_file(&local_source)
        && (!scanned_items.is_empty() || discovery::has_any_bundle_file(&local_source));
    // `update` drops a recorded bundle that the source no longer holds; the
    // run resolves the other bundles.
    let mut vanished_bundles = Vec::new();
    let pruned;
    let options = if options.allow_vanished {
        // A bundle is vanished only after a complete search finds it absent;
        // an unreadable directory is an error.
        let mut kept = Vec::new();
        for name in &options.bundles {
            if discovery::try_find_bundle_by_name(&local_source, name)?.is_some() {
                kept.push(name.clone());
            } else {
                vanished_bundles.push(name.clone());
            }
        }
        pruned = AddOptions {
            bundles: kept,
            ..options.clone()
        };
        &pruned
    } else {
        options
    };
    // `update`: the direct members of a vanished bundle that are still in the
    // source stay installed; a name that matches
    // nothing in the source is dropped from the request, so that the item is
    // removed as no longer in the source. `nothing_left` marks a request
    // that was not empty and has nothing left: the run installs nothing and
    // must not fall back to the whole source (an empty request means the
    // whole source).
    let present_items: Vec<String>;
    let mut nothing_left = false;
    // The `(kind, name)` pairs that only this recovery requests; for a name
    // that the caller did not request, only these kinds are installed.
    let recovered: BTreeSet<(ItemKind, String)> = vanished_bundles
        .iter()
        .filter_map(|bundle| options.bundle_members.get(bundle))
        .flatten()
        .chain(&options.direct_requests)
        .cloned()
        .collect();
    let original_names: BTreeSet<String> = items.iter().cloned().collect();
    let items: &[String] = if options.allow_vanished && !discovery::is_bundle_file(&local_source) {
        let mut seen = BTreeSet::new();
        let wanted: Vec<String> = items
            .iter()
            .chain(recovered.iter().map(|(_, name)| name))
            .filter(|name| seen.insert(name.as_str()))
            .cloned()
            .collect();
        present_items = wanted
            .iter()
            .filter(|name| {
                scanned_items.iter().any(|(_, scanned)| scanned == *name)
                    || discovery::has_matching_items(&local_source, name)
            })
            .cloned()
            .collect();
        nothing_left =
            present_items.is_empty() && (!wanted.is_empty() || !vanished_bundles.is_empty());
        &present_items
    } else {
        items
    };
    if !options.keep_recorded_excludes && !discovery::is_bundle_file(&local_source) {
        for name in items {
            if !scanned_items.iter().any(|(_, scanned)| scanned == name)
                && discovery::find_bundle_by_name(&local_source, name).is_none()
                && discovery::find_command_dir(&local_source, name)?.is_some()
            {
                anyhow::bail!(
                    "`{name}` is a command item; `upskill add` does not install command items. \
                     Use `upskill pack` to build a plugin that holds it"
                );
            }
            if !scanned_items.iter().any(|(_, scanned)| scanned == name)
                && discovery::find_bundle_by_name(&local_source, name).is_none()
                && discovery::find_hook_dir(&local_source, name)?.is_some()
            {
                anyhow::bail!(
                    "`{name}` is a hook item; `upskill add` does not install hook items. \
                     Use `upskill pack` to build a plugin that holds it"
                );
            }
        }
    }

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
            .filter(|(kind, _)| {
                let kinds = options.source_kinds.get(&label).unwrap_or(&options.kinds);
                kinds.is_empty() || kinds.contains(kind)
            })
            .filter(|(_, n)| items.is_empty() || items.contains(n))
            .filter(|(kind, n)| {
                original_names.contains(n)
                    || !recovered.iter().any(|(_, name)| name == n)
                    || recovered.contains(&(*kind, n.clone()))
            })
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
            // A name that the scan does not list but that names an item
            // directory (a hidden one, installed by name): only the name
            // resolution reads it.
            || items
                .iter()
                .any(|n| !scanned_items.iter().any(|(_, scanned)| scanned == n))
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
    let bundle_resolution = if nothing_left && options.bundles.is_empty() {
        Some((DependencyClosure::default(), Vec::new()))
    } else if options.bundles.is_empty() {
        resolve_bundle_request(
            &label,
            &local_source,
            git_ref,
            commit.as_deref(),
            items,
            &options.excludes,
        )?
    } else {
        let (mut closure, bundles) = resolve_bundle_closure(
            &label,
            &local_source,
            git_ref,
            commit.as_deref(),
            &options.bundles,
            &options.excludes,
        )?;
        if !items.is_empty() {
            closure.merge(resolve_requires_closure(
                &label,
                &local_source,
                git_ref,
                commit.as_deref(),
                &requested,
                &options.excludes,
            )?)?;
        }
        Some((closure, bundles))
    };
    let (mut closure, resolved_bundles): (Option<DependencyClosure>, Vec<ResolvedBundle>) =
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
                    commit.as_deref(),
                    &requested,
                    &options.excludes,
                )?),
                Vec::new(),
            )
        };

    if let Some(closure) = &mut closure {
        closure.retain_kinds(&options.kinds, &options.source_kinds);
        // `update` can resolve nothing from the entry source (every recorded
        // bundle vanished). Its item directories still take part in the
        // overlap checks, with an empty install selection.
        if options.allow_vanished && !discovery::is_bundle_file(&local_source) {
            closure
                .source_roots
                .entry(label.clone())
                .or_insert_with(|| local_source.clone());
        }
        // `update` removes an installed item that no plan provides, so every
        // root that the run plans from is walked strictly: each source root
        // of the closure (the entry source, the registry root of a
        // bundle-file source and every dependency source). The strict walk
        // reads the hidden directories that the scan skips, and an
        // unreadable directory stops the run instead of making its items
        // look absent.
        if options.allow_vanished {
            for root in closure.source_roots.values() {
                discovery::iter_item_dirs_strict(root)?;
            }
        }
    } else if options.allow_vanished && local_source.is_dir() {
        // The name resolution reads the entry source without a closure:
        // walk it strictly as well.
        discovery::iter_item_dirs_strict(&local_source)?;
    }

    // Installed names of entry-source items. The first mapping given for
    // a name wins; an exact `orig=alias` wins over a bare `--as`. The
    // conflict check below and the plan use the same names.
    let mut entry_names = plan::PlanNames {
        excludes: options.excludes.clone(),
        kinds: options.kinds.clone(),
        clients: options.item_clients.clone(),
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
    super::apply::check_lockfile_records(target, &lock)?;
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
            .filter(|(kind, _)| entry_names.takes(*kind))
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

    // -- Refuse a source item that overlaps an output path of the run --
    // Runs before the adoption and before any write, so a refused install
    // changes no file. Aliases apply to entry-source items only, as in the
    // plan below. The items of every source that this call reads are compared
    // with each other. `update` calls this function once for each lockfile
    // source, and then checks the items of all these calls together, before
    // its first write.
    let at = crate::scope::ScopeRoot {
        scope: options.scope,
        root: target.to_path_buf(),
    };
    let mut source_items = Vec::new();
    if let Some(closure) = &closure {
        let no_aliases = plan::PlanNames {
            excludes: options.excludes.clone(),
            kinds: options.kinds.clone(),
            clients: options.item_clients.clone(),
            ..plan::PlanNames::default()
        };
        // Every visited source, also one without items in the run: its item
        // directories are protected (an empty filter installs none of them).
        let no_items = crate::bundle::ResolvedItems::default();
        for (src_label, root) in &closure.source_roots {
            let names = if *src_label == label {
                &entry_names
            } else {
                &no_aliases
            };
            let filter = closure
                .by_source
                .get(src_label)
                .map_or(&no_items, |si| &si.items);
            install::collect_source_items(root, Some(filter), names, &mut source_items)?;
        }
    } else {
        // Without a closure, `items` is not empty: a bundle-file source always
        // gets a closure, and only a request with names defers to name
        // resolution.
        install::collect_name_resolution_items(
            &local_source,
            items,
            &entry_names,
            &mut source_items,
        )?;
    }
    super::output::check_run_outside_outputs(&at, &source_items)?;

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
        kinds: options.kinds.clone(),
        clients: options.item_clients.clone(),
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
                options.scope,
            )?;
            r.items.extend(part.items);
            r.planned.extend(part.planned);
            r.bundles.extend(part.bundles);
            r.selection_skipped.extend(part.selection_skipped);
        }
        r
    } else if items.is_empty() {
        install_from_local_path_selective(
            &local_source,
            None,
            &options.selection,
            &entry_names,
            options.scope,
        )?
    } else {
        install_with_name_resolution_from_local(
            &local_source,
            items,
            &options.selection,
            &entry_names,
            options.scope,
        )?
    };

    // `update` accepts an empty plan: the recorded items that the source no
    // longer provides are removed.
    if !options.allow_vanished
        && !options.kinds.is_empty()
        && !items.is_empty()
        && report.planned.is_empty()
    {
        anyhow::bail!(
            "no item of kind {} matches: {}",
            options
                .kinds
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            items.join(", ")
        );
    }

    // An item planned twice from the same source item (for example named
    // and also a member of a named bundle) is kept once. Two items with one
    // installed name fail here, before the adoption.
    plan::merge_duplicates(&mut report.planned);
    // The same item reaches `report.items` once for the bundle and once for
    // the name, so the summary would count it twice.
    let mut seen = std::collections::BTreeSet::new();
    report.items.retain(|i| {
        seen.insert((
            i.kind,
            i.name.clone(),
            i.client.name(),
            i.output_path.clone(),
        ))
    });
    apply::check_installed_names(&report.planned)?;

    let mut options = options.clone();
    options.kinds.sort();
    options.kinds.dedup();
    let mut excluded: BTreeSet<(ItemKind, String)> = closure
        .as_ref()
        .map(|c| c.skipped.clone())
        .unwrap_or_default();
    // A whole-source install takes every item of the source; the items
    // with an excluded name are the ones it does not take.
    if items.is_empty() {
        excluded.extend(
            scanned_items
                .iter()
                .filter(|(_, n)| options.excludes.contains(n))
                .filter(|(kind, _)| options.kinds.is_empty() || options.kinds.contains(kind))
                .map(|(kind, name)| (*kind, name.clone())),
        );
    }
    report.excluded = excluded.into_iter().collect();
    Ok(PreparedInstall {
        label,
        git_ref: git_ref.map(str::to_string),
        commit,
        options,
        requested: items.to_vec(),
        report,
        closure,
        resolved_bundles,
        vanished_bundles,
        holds_content,
        source_items,
        local_source,
        _tmp_guard: tmp_guard,
    })
}

impl PreparedInstall {
    /// Each skipped command, hook or LSP server once, unless kinds restrict the run.
    pub fn pack_only_skipped(&self) -> Vec<(PackOnlyKind, String)> {
        if !self.options.kinds.is_empty() {
            return Vec::new();
        }
        if self.resolved_bundles.is_empty() {
            pack_only_items(&self.report.bundles)
        } else {
            let bundles: Vec<_> = self
                .resolved_bundles
                .iter()
                .map(|resolved| resolved.bundle.clone())
                .collect();
            pack_only_items(&bundles)
        }
    }

    /// The items that an exclude kept out of this run, each once.
    pub fn excluded(&self) -> &[(ItemKind, String)] {
        &self.report.excluded
    }

    /// The names of `own` that match no item of the entry source and no
    /// item that the resolver skipped. Such a name is most likely a typo.
    pub fn unknown_excludes(&self, own: &[String]) -> Result<Vec<String>> {
        let known: BTreeSet<String> = scan_source_items(&self.local_source)?
            .into_iter()
            .map(|(_, name)| name)
            .chain(self.report.excluded.iter().map(|(_, name)| name.clone()))
            .collect();
        Ok(own
            .iter()
            .filter(|name| !known.contains(*name))
            .cloned()
            .collect())
    }

    /// The names of the bundles that this install resolved, through the
    /// `requires` of the requested bundles included.
    pub fn resolved_bundle_names(&self) -> BTreeSet<String> {
        self.resolved_bundles
            .iter()
            .map(|rb| rb.bundle.name.clone())
            .collect()
    }

    /// Each bundle that this install resolved, with the bundles of this
    /// resolution whose `requires` reached it (`bundle:<name>`; empty for a
    /// requested bundle).
    pub fn resolved_requirers(&self) -> BTreeMap<String, Vec<String>> {
        self.resolved_bundles
            .iter()
            .map(|rb| (rb.bundle.name.clone(), rb.required_by.clone()))
            .collect()
    }

    /// The recorded bundles that the fetched source no longer holds.
    pub fn vanished_bundles(&self) -> &[String] {
        &self.vanished_bundles
    }

    /// True when the fetched entry source holds at least one item or bundle.
    /// Always false for a bundle-file source.
    pub fn holds_content(&self) -> bool {
        self.holds_content
    }

    /// The bundle `name` as the fetched entry source declares it now: a
    /// bundle that this install resolved (always the case for a bundle-file
    /// source), else the bundle file of that name in a directory source.
    /// `None` when the source has no such bundle or its manifest cannot be
    /// read.
    pub fn source_bundle(&self, name: &str) -> Option<crate::model::Bundle> {
        if let Some(rb) = self
            .resolved_bundles
            .iter()
            .find(|rb| rb.bundle.name == name)
        {
            return Some(rb.bundle.clone());
        }
        // A bundle-file source always resolves its bundles above, so only a
        // directory source is searched here.
        let path = discovery::find_bundle_by_name(&self.local_source, name)?;
        crate::parse::bundle::load(&path)
            .ok()
            .filter(|bundle| bundle.name == name)
    }

    /// The fetched root of the entry source: a directory or a bundle file.
    pub fn entry_root(&self) -> &Path {
        &self.local_source
    }

    /// Every item directory of every source that this install reads, as given
    /// to the overlap check of [`prepare_install`].
    pub fn source_items(&self) -> &[super::output::SourceItem] {
        &self.source_items
    }

    /// `(kind, installed name) -> source hash` of every planned item.
    pub fn planned_hashes(&self) -> BTreeMap<(ItemKind, String), Option<String>> {
        self.report
            .items
            .iter()
            .map(|it| ((it.kind, it.name.clone()), it.source_hash.clone()))
            .collect()
    }

    /// Sets a fetched commit on the lockfile entries in `selected` (the
    /// `(kind, installed name)` of each entry that the run selected and
    /// whose update is not withheld): an entry of this source gets the
    /// commit of this source, and an entry of another source (a dependency
    /// entry that joined the group, #648) gets the commit of its own source,
    /// or none when this install did not fetch that source with a commit. It refreshes `required_by` of every entry
    /// with [`refresh_required_by`], and saves the lockfile only when a
    /// value differs. An entry that is not in `selected` keeps its recorded
    /// commit. A source without a commit (a local path) sets no commit.
    /// Used by `update` for a source that is not applied because it has no
    /// changes, so that the new commit alone does not leave the recorded
    /// commit behind.
    pub fn record_commit(
        &self,
        target: &Path,
        selected: &BTreeSet<(ItemKind, String)>,
    ) -> Result<()> {
        let mut lock = Lockfile::load(target)?;
        let mut changed = refresh_required_by(&mut lock, &self.requires_provenance());
        for item in lock
            .items
            .iter_mut()
            .filter(|i| selected.contains(&(i.kind, i.name.clone())))
        {
            // An entry of another source (a dependency that this install
            // reaches) gets the commit of its own source.
            let commit = if item.source == self.label {
                self.commit.as_ref()
            } else {
                self.closure
                    .as_ref()
                    .and_then(|c| c.by_source.get(&item.source))
                    .and_then(|s| s.commit.as_ref())
            };
            if commit.is_some() && item.commit.as_ref() != commit {
                item.commit = commit.cloned();
                changed = true;
            }
        }
        if changed {
            lock.save(target)?;
        }
        Ok(())
    }

    /// Whether the install includes a rule: the `has_rules` argument of the
    /// hand-shake steps.
    fn has_rules(&self) -> bool {
        self.report.items.iter().any(|i| i.kind == ItemKind::Rule)
    }

    /// Names of the rules that this install writes for opencode.
    fn opencode_rule_names(&self) -> BTreeSet<String> {
        self.report
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Rule && i.client == crate::generate::Client::OpenCode)
            .map(|i| i.name.clone())
            .collect()
    }

    /// `alias -> source name` of every planned item installed under an
    /// `--as` alias.
    fn aliased_names(&self) -> HashMap<String, String> {
        let mut aliased_names: HashMap<String, String> = HashMap::new();
        for planned in &self.report.planned {
            if let Some(original) = &planned.source_name {
                aliased_names
                    .entry(planned.name.clone())
                    .or_insert_with(|| original.clone());
            }
        }
        aliased_names
    }

    /// The `requires` provenance of this run, with installed (post-alias)
    /// names.
    ///
    /// `evaluated` holds the label `"kind:name"` of every item in the
    /// retained closure. An item install evaluates its item closure. A bundle
    /// or name-resolution install evaluates no requirer. An update of a
    /// directory source with recorded bundles evaluates the merged bundle and
    /// direct-item closure, including bundle members even without direct items.
    /// The bundle closure contributes no item-level `required_by` edges.
    /// `required_by` maps each dependency reached by the item closure to its
    /// requirers; an excluded dependency is not reached and has no entry. A kind-restricted install refreshes only
    /// planned entries, including entries from other sources.
    fn requires_provenance(&self) -> RequiresProvenance {
        // Translate `requires` provenance through any applied `--as` aliases so
        // the recorded `required_by` is consistent with the post-alias names that
        // `doctor` matches against (#205). The closure keys dependencies and labels
        // requirers by ORIGINAL effective name; the plan already installed the
        // items under their alias. `aliased_names` maps `alias ->
        // original`; invert it to `original -> alias` and rewrite (a) each
        // dependency key's name and (b) each `"{kind}:{name}"` requirer label.
        let aliased_names = self.aliased_names();
        let original_to_alias: HashMap<&str, &str> = aliased_names
            .iter()
            .map(|(alias, original)| (original.as_str(), alias.as_str()))
            .collect();
        let alias_name = |name: &str| -> String {
            original_to_alias
                .get(name)
                .map(|a| a.to_string())
                .unwrap_or_else(|| name.to_string())
        };
        let alias_label = |label: &str| -> String {
            match label.split_once(':') {
                Some((kind, name)) => format!("{kind}:{}", alias_name(name)),
                None => label.to_string(),
            }
        };
        let mut provenance = RequiresProvenance {
            restriction: (!self.options.kinds.is_empty()).then(|| {
                self.report
                    .planned
                    .iter()
                    .map(|item| (item.kind, item.name.clone()))
                    .collect()
            }),
            ..RequiresProvenance::default()
        };
        // Update evaluates its recorded bundle closure even without direct items.
        let Some(closure) = self
            .closure
            .as_ref()
            .filter(|_| self.resolved_bundles.is_empty() || !self.options.bundles.is_empty())
        else {
            return provenance;
        };
        provenance.evaluated = closure
            .item_source
            .keys()
            .map(|(kind, name)| format!("{kind}:{}", alias_name(name)))
            .collect();
        provenance.required_by = closure
            .required_by
            .iter()
            .map(|((kind, name), requirers)| {
                let aliased_requirers =
                    requirers.iter().map(|r| alias_label(r)).collect::<Vec<_>>();
                ((*kind, alias_name(name)), aliased_requirers)
            })
            .collect();
        provenance
    }

    /// The lockfile entry of each planned item, with an empty `files`
    /// record. Used by `preview` and `commit`.
    ///
    /// `direct` is true for an item that this `add` installs because the user
    /// named it, or through a whole directory source without names, and for
    /// an item that `lock` already records as direct. `update` never sets it.
    fn new_items(&self, lock: &Lockfile) -> Vec<LockedItem> {
        // Aliases were applied at plan time; map each alias to its source
        // name for the lockfile `source_name`.
        let aliased_names = self.aliased_names();
        let hashes = self.planned_hashes();
        let provenance = self.requires_provenance().required_by;
        let item_source_lookup = |kind: ItemKind, name: &str| -> SourceStamp {
            if let Some(closure) = &self.closure {
                if let Some(src_label) = closure.item_source.get(&(kind, name.to_string())) {
                    let source = closure.by_source.get(src_label);
                    return SourceStamp {
                        label: src_label.clone(),
                        git_ref: source.and_then(|si| si.git_ref.clone()),
                        commit: source.and_then(|si| si.commit.clone()),
                    };
                }
            }
            SourceStamp {
                label: self.label.clone(),
                git_ref: self.git_ref.clone(),
                commit: self.commit.clone(),
            }
        };
        let mut new_items = crate::lockfile::items_from_report(
            &self.report,
            item_source_lookup,
            &provenance,
            |k, n| hashes.get(&(k, n.to_string())).cloned().flatten(),
        );
        let bundle_names: BTreeSet<&str> = self
            .resolved_bundles
            .iter()
            .map(|rb| rb.bundle.name.as_str())
            .collect();
        let whole_directory_source = self.requested.is_empty()
            && self.resolved_bundles.is_empty()
            && !discovery::is_bundle_file(&self.local_source);
        for item in &mut new_items {
            if let Some(original) = aliased_names.get(&item.name) {
                item.source_name = Some(original.clone());
            }
            let source_name = item.source_name.as_ref().unwrap_or(&item.name);
            let named = self
                .requested
                .iter()
                .any(|n| n == source_name && !bundle_names.contains(n.as_str()));
            let added_here = !self.options.allow_vanished
                && item.source == self.label
                && (named || whole_directory_source);
            // The lockfile keys an item by `(kind, name)`; the recorded source
            // can still have a legacy spelling that `update` rewrites, so the
            // source is not compared.
            let recorded = lock
                .items
                .iter()
                .any(|i| i.kind == item.kind && i.name == item.name && i.direct);
            item.direct = recorded || added_here;
        }
        new_items
    }

    /// What `commit` would do now, against `lock`. Writes nothing.
    ///
    /// `lock` is the lockfile that `commit` would use: for a `schema: 1`
    /// lockfile, the caller passes it after `owned::adopt_schema1_in_memory`.
    /// The preview is computed by [`apply::decide_plan`], as `commit` does;
    /// see [`preview_items`] for how each item is reported.
    ///
    /// `orphan_rules` holds the names of the rules that an `update` removes
    /// because their source no longer has them; their entries leave the
    /// global opencode config.
    pub fn preview(
        &self,
        target: &Path,
        plugin_scope: PluginScope,
        lock: &Lockfile,
        orphan_rules: &BTreeSet<String>,
    ) -> Result<InstallPreview> {
        let new_items = self.new_items(lock);
        let decision = apply::decide_plan(
            target,
            lock,
            &self.report.planned,
            self.options.overwrite,
            Some(&new_items),
            self.options.keep_unwithheld_edits,
        )?;
        let (items, kept_notes) =
            preview_items(target, lock, &self.report.planned, decision, &new_items);
        // The rules that this install would write for opencode, whose entries
        // the global opencode config would get, and the rules that would stop
        // targeting opencode, whose entries it would lose (also the removed
        // rules in `orphan_rules`). A skipped item is neither written nor
        // recorded; a skip is any `Disposition::Skip`, also a conflict with
        // an unowned file or a modified file of an owned item.
        let skipped: BTreeSet<&str> = items
            .iter()
            .filter(|i| i.action == PreviewAction::Skip && i.kind == ItemKind::Rule)
            .map(|i| i.name.as_str())
            .collect();
        let opencode_rules: BTreeSet<String> = self
            .opencode_rule_names()
            .into_iter()
            .filter(|name| !skipped.contains(name.as_str()))
            .collect();
        let mut after = lock.clone();
        for name in orphan_rules {
            after.remove(ItemKind::Rule, name);
        }
        for item in new_items {
            if !(item.kind == ItemKind::Rule && skipped.contains(item.name.as_str())) {
                after.upsert(item);
            }
        }
        let stale_root = (plugin_scope == PluginScope::User).then_some(target);
        let dropped = dropped_opencode_rules(lock, &after, stale_root);
        let handshake = crate::ancillary::pending_handshake_changes(
            target,
            plugin_scope,
            self.has_rules(),
            &opencode_rules,
            &dropped,
        )?;
        Ok(InstallPreview {
            items,
            kept_notes,
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
        let new_items = self.new_items(&lock);
        let provenance = self.requires_provenance();
        let has_rules = self.has_rules();
        let opencode_rules = self.opencode_rule_names();
        // The lockfile before this install changes it.
        let lock_before = lock.clone();
        let PreparedInstall {
            label,
            git_ref,
            commit: _,
            options,
            requested,
            mut report,
            closure: _closure,
            resolved_bundles,
            vanished_bundles: _,
            holds_content: _,
            source_items: _,
            local_source: _,
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
        // A skipped item keeps its lockfile entry, or gets none. An item kept
        // for its modified files keeps its lockfile entry, except for its
        // commit. In both, only `required_by` can change, by the refresh below.
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

        if options.kinds.is_empty() {
            // -- Plugin installation (ADR-0008) --
            report.plugin_results =
                install_plugins_from_bundles(&report.bundles, plugin_scope, target);

            // -- MCP server configuration (ADR-0010), restricted to the selection --
            report.mcp_results = install_mcps_from_bundles(
                &report.bundles,
                plugin_scope,
                target,
                &options.selection,
            );
        }

        // `required_by` of every entry, also of a skipped or kept item, gets
        // the provenance of this run (#504).
        refresh_required_by(&mut lock, &provenance);
        // An item that keeps its modified files has an equal record
        // (`apply::same_record`), so only its commit is brought up to date.
        let kept_edits: BTreeSet<(ItemKind, String)> = outcome
            .kept_edits
            .iter()
            .map(|c| (c.kind, c.name.clone()))
            .collect();
        for mut item in new_items {
            if kept_edits.contains(&(item.kind, item.name.clone())) {
                if let Some(entry) = lock
                    .items
                    .iter_mut()
                    .find(|i| i.kind == item.kind && i.name == item.name)
                {
                    entry.commit = item.commit.take();
                    entry.direct |= item.direct;
                }
                continue;
            }
            if skipped.contains(&(item.kind, item.name.clone())) {
                continue;
            }
            item.files = recorded
                .remove(&(item.kind, item.name.clone()))
                .unwrap_or_default();
            // The refreshed `required_by` of an existing entry contains the
            // requirers of this run and keeps the requirers of other runs.
            if let Some(existing) = lock
                .items
                .iter()
                .find(|i| i.kind == item.kind && i.name == item.name)
            {
                item.required_by = existing.required_by.clone();
            }
            lock.upsert(item);
        }
        // A requested name that this run also excludes stays in the record.
        // `update` never removes a name from the record, also when the item
        // is still installed and its name is in the item filter: only the
        // positional names of `add` do.
        let named: Vec<String> = if options.keep_recorded_excludes {
            Vec::new()
        } else {
            requested
                .iter()
                .filter(|name| !options.excludes.contains(name))
                .cloned()
                .collect()
        };
        lock.record_excludes(&options.excludes, &named);
        if resolved_bundles.is_empty() {
            // Same-source bundle install (no cross-source provenance): every bundle
            // came from the entry source.
            for bundle in &report.bundles {
                lock.upsert_bundle(crate::lockfile::LockedBundle {
                    kinds: options.kinds.clone(),
                    name: bundle.name.clone(),
                    source: label.clone(),
                    git_ref: git_ref.clone(),
                    items: bundle_item_names(bundle),
                    required_by: Vec::new(),
                });
            }
        } else {
            // Bundle closure: record each bundle under its OWN source and ref.
            for rb in &resolved_bundles {
                // A recorded root stays a root. Otherwise the requirers that
                // this run resolved are replaced by the ones it found, and the
                // requirers of other runs are kept.
                let existing = lock.bundles.iter().find(|b| b.name == rb.bundle.name);
                let required_by = match existing {
                    Some(existing)
                        if rb.required_by.is_empty() && options.keep_recorded_requirers =>
                    {
                        existing.required_by.clone()
                    }
                    _ if rb.required_by.is_empty() => Vec::new(),
                    Some(existing) if existing.required_by.is_empty() => Vec::new(),
                    Some(existing) => {
                        let mut merged: BTreeSet<String> = existing
                            .required_by
                            .iter()
                            .filter(|r| {
                                !resolved_bundles
                                    .iter()
                                    .any(|o| format!("bundle:{}", o.bundle.name) == **r)
                            })
                            .cloned()
                            .collect();
                        merged.extend(rb.required_by.iter().cloned());
                        merged.into_iter().collect()
                    }
                    None => rb.required_by.clone(),
                };
                lock.upsert_bundle(crate::lockfile::LockedBundle {
                    required_by,
                    kinds: options.kinds.clone(),
                    name: rb.bundle.name.clone(),
                    source: rb.source_label.clone(),
                    git_ref: rb.git_ref.clone(),
                    items: bundle_item_names(&rb.bundle),
                });
            }
        }
        // Record installed and warn-skipped plugins in the lockfile so that
        // `doctor` can surface skipped ones and verify installed ones.
        record_plugin_results(&mut lock, &report.plugin_results, plugin_scope);
        // Record configured and warn-skipped MCP servers in the lockfile.
        for mr in &report.mcp_results {
            use crate::lockfile::McpInstallStatus;
            use crate::plugin::PluginOutcome;

            let status = match &mr.outcome {
                PluginOutcome::Success => McpInstallStatus::Installed,
                PluginOutcome::CliNotFound => McpInstallStatus::Skipped,
                // ManualInstructions and UnsupportedVersion are unused for
                // MCP; Failed is transient.
                PluginOutcome::ManualInstructions
                | PluginOutcome::UnsupportedVersion { .. }
                | PluginOutcome::Failed { .. } => continue,
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
        // In the global scope the glob does not work: opencode expands no `**`
        // there, and resolves a relative path against the project directory.
        // The global opencode config gets one absolute path per installed rule
        // instead (ADR-0008, issue #581).
        match plugin_scope {
            PluginScope::Project => {
                crate::ancillary::ensure_opencode_rules_registered(target, has_rules)?;
            }
            PluginScope::User => {
                // A rule that was skipped (any `Disposition::Skip`: a conflict
                // with an unowned file, or a modified file of an owned item)
                // is not written, so it gets no entry.
                let written: BTreeSet<String> = opencode_rules
                    .iter()
                    .filter(|name| {
                        !report
                            .conflicts
                            .iter()
                            .any(|c| c.kind == ItemKind::Rule && &c.name == *name)
                    })
                    .cloned()
                    .collect();
                let dropped = dropped_opencode_rules(&lock_before, &lock, Some(target));
                crate::ancillary::sync_opencode_global_rules(target, &written, &dropped)?;
            }
        }

        // Per ADR-0003: when the install includes any rule, register
        // `.github/instructions` in `.vscode/settings.json`'s
        // `chat.instructionsFilesLocations`. That directory is the Copilot
        // rule path of the project scope, where VS Code Copilot picks up the
        // generated `<name>.instructions.md` files. In the global scope the
        // rule files are written to `.copilot/instructions`, which this
        // entry does not point at.
        crate::ancillary::ensure_vscode_instructions_registered(target, has_rules)?;

        Ok(report)
    }
}

/// The rules that `before` records for opencode and `after` does not: a rule
/// that an install records for fewer clients no longer targets opencode, and
/// its entry in the global opencode config is deleted. A rule that this install
/// writes for opencode is in `after` as an opencode rule, so it is not here.
///
/// With `stale_root`, the global root, the result also holds each rule that
/// `after` records for other clients than opencode and whose entry the global
/// opencode config still lists. An earlier removal of that entry can have
/// failed after the lockfile was saved; `doctor` reports such an entry, and
/// this deletes it. The config is read for this; a file that cannot be read as
/// JSON adds no name.
fn dropped_opencode_rules(
    before: &Lockfile,
    after: &Lockfile,
    stale_root: Option<&Path>,
) -> BTreeSet<String> {
    let remaining = after.opencode_rule_names();
    let mut dropped: BTreeSet<String> = before
        .opencode_rule_names()
        .into_iter()
        .filter(|name| !remaining.contains(name))
        .collect();
    if let Some(root) = stale_root {
        let listed = crate::ancillary::opencode_global_rule_names(root);
        dropped.extend(
            after
                .other_client_rule_names()
                .into_iter()
                .filter(|name| listed.contains(name)),
        );
    }
    dropped
}

/// Refreshes `required_by` of every entry of `lock` with the provenance of
/// one run, and returns whether a value changed (#504).
///
/// A requirer label (`"kind:name"`) in `provenance.evaluated` is removed
/// from every entry and added back where `provenance.required_by` records
/// it. A kind-restricted install leaves entries it does not plan untouched, also
/// in other sources. Every other requirer is kept, whether or not it is
/// installed. The result is sorted and has no duplicates.
fn refresh_required_by(lock: &mut Lockfile, provenance: &RequiresProvenance) -> bool {
    let mut changed = false;
    for entry in &mut lock.items {
        if provenance
            .restriction
            .as_ref()
            .is_some_and(|planned| !planned.contains(&(entry.kind, entry.name.clone())))
        {
            continue;
        }
        let mut merged: BTreeSet<String> = entry
            .required_by
            .iter()
            .filter(|r| !provenance.evaluated.contains(*r))
            .cloned()
            .collect();
        if let Some(new) = provenance
            .required_by
            .get(&(entry.kind, entry.name.clone()))
        {
            merged.extend(new.iter().cloned());
        }
        let merged: Vec<String> = merged.into_iter().collect();
        if merged != entry.required_by {
            entry.required_by = merged;
            changed = true;
        }
    }
    changed
}

/// Maps each item of `decision` to a [`PreviewItem`], and returns the
/// notes: the modified destinations of the items with
/// [`Disposition::KeepEdits`], and the kept modified stale files of the
/// `Write` items that have no other change.
///
/// - `KeepEdits`: no `PreviewItem`; its conflict paths are notes.
/// - `Skip`: action `Skip`, `blocking` is the conflict paths.
/// - `Write`: `write` and `delete` are the paths that `execute_plan` would
///   count in `changed` (see [`apply::changes_file`] and
///   [`apply::deletes_existing`]); `kept_modified` is the kept stale files;
///   `forced` is the conflict paths (there are some only with `overwrite`).
///   The action is `Complete` when the lockfile entry has a `pending` record
///   and the planned source hash equals the recorded hash, and `Update`
///   otherwise. `record_only` is true when nothing is written or deleted,
///   `new_items` holds an entry for the item, and the lockfile record of
///   the item would change: the lockfile has no entry for the item yet, a
///   field other than `files`, `pending`, `required_by` and `commit`
///   differs (see
///   [`apply::same_record`]), or the `files` record that `execute_plan`
///   would save ([`apply::ItemChanges::recorded_files`]: the planned files
///   and the records of the kept stale files) differs from the `files`
///   record of the entry, compared as a set of (client, path, hash). An
///   item whose `write`, `delete` and `forced` are empty, that is not
///   `Complete` and not `record_only`, gets no `PreviewItem`: its kept
///   stale files, if any, are notes.
fn preview_items(
    target: &Path,
    lock: &Lockfile,
    planned: &[PlannedItem],
    decision: PlanDecision,
    new_items: &[LockedItem],
) -> (Vec<PreviewItem>, Vec<PathBuf>) {
    let mut items = Vec::new();
    let mut kept_notes = Vec::new();
    for (item, changes) in planned.iter().zip(decision.items) {
        let recorded = changes.recorded_files();
        let conflict_paths: Vec<PathBuf> = changes.conflicts.into_iter().map(|c| c.path).collect();
        match changes.disposition {
            Disposition::KeepEdits => {
                kept_notes.extend(conflict_paths);
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
            .filter(|file| apply::changes_file(target, file, &changes.unchanged))
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
        // A kept modified stale file of an item with no other change is a
        // note, not a change.
        if write.is_empty()
            && delete.is_empty()
            && conflict_paths.is_empty()
            && !complete
            && !record_only
        {
            kept_notes.extend(kept_modified);
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
    (items, kept_notes)
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
            .preview(
                &target,
                PluginScope::Project,
                &Lockfile::load(&target).unwrap(),
                &BTreeSet::new(),
            )
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

    /// A source that only adds the executable bit to a resource: the
    /// bytes and the source hash are unchanged, and commit sets the
    /// executable bits, so the preview lists the file as a change.
    #[cfg(unix)]
    #[test]
    fn a_mode_only_change_is_in_the_preview_and_in_the_commit() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "same");
        write(&src, "a/run.sh", b"echo run\n");
        let script = src.join("a/run.sh");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o644)).unwrap();
        let source = InstallSource::LocalPath(src.clone());
        prepare_install(&source, &target, &[], &AddOptions::default())
            .unwrap()
            .commit(&target, PluginScope::Project)
            .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

        let prepared = prepare_install(&source, &target, &[], &AddOptions::default()).unwrap();
        let preview = prepared
            .preview(
                &target,
                PluginScope::Project,
                &Lockfile::load(&target).unwrap(),
                &BTreeSet::new(),
            )
            .unwrap();
        assert_eq!(preview.items.len(), 1, "{:?}", preview.items);
        let previewed: BTreeSet<PathBuf> = preview
            .items
            .iter()
            .flat_map(|i| i.write.iter().chain(&i.delete).cloned())
            .collect();
        assert!(!previewed.is_empty());
        assert!(
            previewed.iter().all(|p| p.ends_with("skills/a/run.sh")),
            "{previewed:?}"
        );
        let report = prepared.commit(&target, PluginScope::Project).unwrap();
        let changed: BTreeSet<PathBuf> = report.changed.into_iter().collect();
        assert_eq!(previewed, changed);
        let mode = fs::metadata(target.join(".claude/skills/a/run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
    }

    /// The user removed the group and other read bits of an installed
    /// resource: only the executable bits count, so the preview has no
    /// change, commit changes nothing, and the mode stays as the user set it.
    #[cfg(unix)]
    #[test]
    fn a_difference_in_the_other_permission_bits_is_not_in_the_preview() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "same");
        write(&src, "a/notes.md", b"notes\n");
        let notes = src.join("a/notes.md");
        fs::set_permissions(&notes, fs::Permissions::from_mode(0o644)).unwrap();
        let source = InstallSource::LocalPath(src.clone());
        prepare_install(&source, &target, &[], &AddOptions::default())
            .unwrap()
            .commit(&target, PluginScope::Project)
            .unwrap();
        let dest = target.join(".claude/skills/a/notes.md");
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o600)).unwrap();

        let prepared = prepare_install(&source, &target, &[], &AddOptions::default()).unwrap();
        let preview = prepared
            .preview(
                &target,
                PluginScope::Project,
                &Lockfile::load(&target).unwrap(),
                &BTreeSet::new(),
            )
            .unwrap();
        assert!(
            preview
                .items
                .iter()
                .all(|i| i.write.is_empty() && i.delete.is_empty()),
            "{:?}",
            preview.items
        );
        let report = prepared.commit(&target, PluginScope::Project).unwrap();
        assert!(report.changed.is_empty(), "{:?}", report.changed);
        let mode = fs::metadata(&dest).unwrap().permissions().mode();
        assert_eq!(mode & 0o7777, 0o600);
    }

    /// The source adds the executable bits to a resource that the user made
    /// private: only the owner gets the executable bit, and a second preview
    /// of the same plan has no change.
    #[cfg(unix)]
    #[test]
    fn executable_bits_limited_by_the_read_bits_are_previewed_once() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let target = tmp.path().join("t");
        fs::create_dir_all(&target).unwrap();
        skill(&src, "a", "same");
        write(&src, "a/run.sh", b"echo run\n");
        let script = src.join("a/run.sh");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o644)).unwrap();
        let source = InstallSource::LocalPath(src.clone());
        prepare_install(&source, &target, &[], &AddOptions::default())
            .unwrap()
            .commit(&target, PluginScope::Project)
            .unwrap();
        let dest = target.join(".claude/skills/a/run.sh");
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let written = |preview: &InstallPreview| -> Vec<PathBuf> {
            preview
                .items
                .iter()
                .flat_map(|i| i.write.iter().chain(&i.delete).cloned())
                .collect()
        };

        let prepared = prepare_install(&source, &target, &[], &AddOptions::default()).unwrap();
        let preview = prepared
            .preview(
                &target,
                PluginScope::Project,
                &Lockfile::load(&target).unwrap(),
                &BTreeSet::new(),
            )
            .unwrap();
        let first = written(&preview);
        // Each client copy of the resource is listed once. The `.claude` copy
        // (0o600) gets only the owner executable bit; the `.github` and
        // `.agents` copies still have the installed mode 0o644, so they get
        // all three executable bits, and are listed too.
        assert_eq!(
            first,
            vec![
                PathBuf::from(".claude/skills/a/run.sh"),
                PathBuf::from(".github/skills/a/run.sh"),
                PathBuf::from(".agents/skills/a/run.sh"),
            ]
        );
        prepared.commit(&target, PluginScope::Project).unwrap();
        let mode = fs::metadata(&dest).unwrap().permissions().mode();
        assert_eq!(mode & 0o7777, 0o700);

        let again = prepare_install(&source, &target, &[], &AddOptions::default()).unwrap();
        let preview = again
            .preview(
                &target,
                PluginScope::Project,
                &Lockfile::load(&target).unwrap(),
                &BTreeSet::new(),
            )
            .unwrap();
        assert!(written(&preview).is_empty(), "{:?}", preview.items);
    }

    #[test]
    fn prepare_and_preview_write_nothing() {
        let (tmp, target, source) = installed_then_changed();
        let before = snapshot(tmp.path());
        let prepared = prepare_install(&source, &target, &[], &AddOptions::default()).unwrap();
        prepared
            .preview(
                &target,
                PluginScope::Project,
                &Lockfile::load(&target).unwrap(),
                &BTreeSet::new(),
            )
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
        let preview = prepared
            .preview(
                &target,
                PluginScope::Project,
                &Lockfile::new(),
                &BTreeSet::new(),
            )
            .unwrap();
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
            direct: false,
            kind: ItemKind::Skill,
            name: "s".to_string(),
            source: "./src".to_string(),
            git_ref: None,
            commit: None,
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

    /// `required_by` is provenance: a change of it only is not a change,
    /// so the source asks nothing.
    #[test]
    fn a_change_of_required_by_only_is_not_a_change() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), ENTRY, b"gen");
        let lock = lock_of(vec![entry(vec![owned_file(ENTRY, b"gen")])]);
        let mut changed = entry(vec![owned_file(ENTRY, b"gen")]);
        changed.required_by = vec!["other".to_string()];
        for keep in [false, true] {
            let (items, kept) = map(
                tmp.path(),
                &lock,
                &planned(&[(ENTRY, b"gen")]),
                std::slice::from_ref(&changed),
                false,
                keep,
            );
            assert!(items.is_empty(), "keep {keep}: {items:?}");
            assert!(kept.is_empty());
            let preview = crate::pipeline::report::SourcePreview {
                source: "./src".to_string(),
                items,
                kept_notes: kept,
                handshake: Vec::new(),
                adopted: None,
                vanished_bundles: Vec::new(),
            };
            assert!(!preview.has_changes());
        }
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

    /// A lockfile entry `kind:name` with `required_by`.
    fn requirement(kind: ItemKind, name: &str, required_by: &[&str]) -> LockedItem {
        let mut item = entry(vec![]);
        item.kind = kind;
        item.name = name.to_string();
        item.required_by = required_by.iter().map(|r| r.to_string()).collect();
        item
    }

    /// The provenance of a run that resolved the `requires` of each item of
    /// `items`, with the `required_by` of each item.
    fn provenance_of(items: &[LockedItem]) -> RequiresProvenance {
        RequiresProvenance {
            restriction: None,
            evaluated: items
                .iter()
                .map(|i| format!("{}:{}", i.kind, i.name))
                .collect(),
            required_by: items
                .iter()
                .filter(|i| !i.required_by.is_empty())
                .map(|i| ((i.kind, i.name.clone()), i.required_by.clone()))
                .collect(),
        }
    }

    #[test]
    fn refresh_required_by_keeps_every_requirer_when_the_run_resolved_no_requires() {
        // A bundle install: `agent:a` is installed again, its `requires`
        // were not read.
        let mut lock = lock_of(vec![requirement(ItemKind::Skill, "x", &["agent:a"])]);
        assert!(!refresh_required_by(
            &mut lock,
            &RequiresProvenance::default()
        ));
        assert_eq!(lock.items[0].required_by, vec!["agent:a"]);
    }

    #[test]
    fn refresh_required_by_sets_the_new_requirers_on_an_empty_entry() {
        let mut lock = lock_of(vec![requirement(ItemKind::Skill, "x", &[])]);
        let new = vec![
            requirement(ItemKind::Agent, "a", &[]),
            requirement(ItemKind::Skill, "x", &["agent:a"]),
        ];
        assert!(refresh_required_by(&mut lock, &provenance_of(&new)));
        assert_eq!(lock.items[0].required_by, vec!["agent:a"]);
    }

    #[test]
    fn refresh_required_by_keeps_a_requirer_that_is_not_in_the_run() {
        // `agent:a` of another source recorded the requirement; this run
        // installs `x` alone.
        let mut lock = lock_of(vec![requirement(ItemKind::Skill, "x", &["agent:a"])]);
        let new = vec![requirement(ItemKind::Skill, "x", &[])];
        assert!(!refresh_required_by(&mut lock, &provenance_of(&new)));
        assert_eq!(lock.items[0].required_by, vec!["agent:a"]);
    }

    #[test]
    fn refresh_required_by_removes_a_requirer_of_the_run_that_no_longer_requires() {
        // `x` is not in the run: only its requirer `agent:a` is.
        let mut lock = lock_of(vec![
            requirement(ItemKind::Agent, "a", &[]),
            requirement(ItemKind::Skill, "x", &["agent:a", "agent:other"]),
        ]);
        let new = vec![requirement(ItemKind::Agent, "a", &[])];
        assert!(refresh_required_by(&mut lock, &provenance_of(&new)));
        assert_eq!(lock.items[1].required_by, vec!["agent:other"]);
    }

    #[test]
    fn refresh_required_by_merges_sorted_without_duplicates() {
        let mut lock = lock_of(vec![requirement(
            ItemKind::Skill,
            "x",
            &["skill:z", "agent:a"],
        )]);
        let new = vec![
            requirement(ItemKind::Agent, "a", &[]),
            requirement(ItemKind::Skill, "x", &["agent:a", "agent:b"]),
        ];
        refresh_required_by(&mut lock, &provenance_of(&new));
        assert_eq!(
            lock.items[0].required_by,
            vec!["agent:a", "agent:b", "skill:z"]
        );
    }
}
