//! Core install pipeline: walk SSOT layout → parse → render → write.
//!
//! These functions own the "for each item: parse its frontmatter,
//! render per-client, write the result, prune stale outputs" loop. The
//! consumer-facing entry point [`super::install_with_lockfile`]
//! coordinates this module with the lockfile and ancillary writes.
//!
//! Plugin orchestration ([`install_plugins_from_bundles`], ADR-0008)
//! also lives here because it is logically part of "what the install
//! pipeline does for a bundle".

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::discovery::{
    detect_item_entrypoint, find_bundle_by_name, find_registry_root, has_matching_items,
    is_bundle_file, iter_item_dirs, iter_item_resources,
};
use super::hash::hash_item_dir;
use super::output::{
    copy_item_resources, is_dir_backed, output_path, remove_item_outputs, write_output,
};
use super::{ALL_CLIENTS, InstallReport, InstalledItem, ItemKind, McpResult, PluginResult};
use crate::generate::{self, Client};
use crate::model::{Agent, Audience, RequireRef, Rule, Skill};
use crate::parse::frontmatter;
use crate::select::ClientSelection;
use crate::source::{InstallSource, parse_install_source};

/// Install every item under `source` into `target`, generating per-client
/// output for each client unless filtered by the item's `audience` field.
///
/// Bundle dispatch: when `source` is a `*.bundle.yaml` file (not a
/// directory), discovers sibling bundles in the registry root walked up
/// from the file, resolves transitively (per [`crate::bundle::resolve`]),
/// and installs only the resolved items. The reached bundles are
/// surfaced via [`InstallReport::bundles`] so the lockfile slice can
/// record them.
pub fn install_from_local_path(
    source: &Path,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    install_from_local_path_selective(source, target, filter, &ClientSelection::all())
}

/// [`install_from_local_path`] with an explicit consumer-side client
/// selection (ADR-0012). The public entry defaults to all clients; the
/// lockfile `add` path threads the resolved selection here.
pub(super) fn install_from_local_path_selective(
    source: &Path,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
    selection: &ClientSelection,
) -> Result<InstallReport> {
    if is_bundle_file(source) {
        return install_bundle_file(source, target, selection);
    }
    let mut report = InstallReport::default();
    for kind in [ItemKind::Skill, ItemKind::Rule, ItemKind::Agent] {
        install_items_of_kind(kind, source, target, &mut report, filter, selection)?;
    }
    Ok(report)
}

pub(super) fn install_bundle_file(
    bundle_path: &Path,
    target: &Path,
    selection: &ClientSelection,
) -> Result<InstallReport> {
    let registry_root = find_registry_root(bundle_path).with_context(|| {
        format!(
            "find SSOT registry root containing skills/, rules/, agents/, or bundles/ \
             above {}",
            bundle_path.display()
        )
    })?;

    let entry = crate::parse::bundle::load(bundle_path)
        .with_context(|| format!("load entry bundle {}", bundle_path.display()))?;

    let available: Vec<crate::model::Bundle> = crate::parse::bundle::discover(&registry_root)
        .with_context(|| {
            format!(
                "discover sibling bundles under registry root {}",
                registry_root.display()
            )
        })?
        .into_iter()
        .map(|(_, b)| b)
        .collect();

    let resolved = crate::bundle::resolve(&entry, &available)?;

    let mut report = InstallReport {
        bundles: resolved.bundles.clone(),
        ..InstallReport::default()
    };
    for kind in [ItemKind::Skill, ItemKind::Rule, ItemKind::Agent] {
        install_items_of_kind(
            kind,
            &registry_root,
            target,
            &mut report,
            Some(&resolved.items),
            selection,
        )?;
    }
    Ok(report)
}

/// Like [`super::install_with_lockfile`]'s name-resolution branch but
/// operates on an already-fetched local source path (avoids double-fetch
/// when `install_with_lockfile` has already called `fetch_ssot`).
pub(super) fn install_with_name_resolution_from_local(
    local_source: &Path,
    target: &Path,
    names: &[String],
    selection: &ClientSelection,
) -> Result<InstallReport> {
    let mut bundle_paths: Vec<std::path::PathBuf> = Vec::new();
    let mut item_names: Vec<String> = Vec::new();

    for name in names {
        let has_items = has_matching_items(local_source, name);
        let bundle_path = find_bundle_by_name(local_source, name);

        match (has_items, bundle_path) {
            (true, Some(bp)) => {
                let rel = bp
                    .strip_prefix(local_source)
                    .unwrap_or(&bp)
                    .display()
                    .to_string();
                anyhow::bail!(
                    "'{}' matches both an item and a bundle\n\n  \
                     item:   {}/{}\n  \
                     bundle: {}\n\n\
                     Disambiguate by using the full path to the bundle:\n  \
                     upskill add <source>:{}",
                    name,
                    name,
                    detect_item_entrypoint(local_source, name),
                    rel,
                    rel,
                );
            }
            (false, Some(bp)) => bundle_paths.push(bp),
            (true, None) => item_names.push(name.clone()),
            (false, None) => {
                anyhow::bail!("no matching items or bundles in source for: {}", name);
            }
        }
    }

    let mut report = InstallReport::default();

    for bp in &bundle_paths {
        let bundle_report = install_bundle_file(bp, target, selection)?;
        report.items.extend(bundle_report.items);
        report.bundles.extend(bundle_report.bundles);
    }

    if !item_names.is_empty() {
        let filter = crate::bundle::ResolvedItems {
            rules: item_names.clone(),
            skills: item_names.clone(),
            agents: item_names.clone(),
        };
        let item_report =
            install_from_local_path_selective(local_source, target, Some(&filter), selection)?;
        report.items.extend(item_report.items);
    }

    Ok(report)
}

// ---------------------------------------------------------------------------
// Same-source `requires` transitive closure
// ---------------------------------------------------------------------------

/// The transitive closure of items reached by expanding a requested set
/// along `requires` edges — possibly spanning multiple sources.
#[derive(Debug, Default)]
pub(super) struct DependencyClosure {
    /// Per canonical source label: the fetched root, its pinned ref, and
    /// the items to install from it (in dependency order).
    pub by_source: BTreeMap<String, SourceInstall>,
    /// Canonical source label each resolved `(kind, name)` came from.
    /// `(kind, name)` is globally unique within a valid closure (a clash
    /// across sources is a conflict error), so this maps cleanly.
    pub item_source: BTreeMap<(ItemKind, String), String>,
    /// `(kind, name) -> { "<requirer-kind>:<requirer-name>", .. }`. Empty
    /// for directly-requested items. Drives the lockfile's `required_by`.
    pub required_by: BTreeMap<(ItemKind, String), BTreeSet<String>>,
    /// Tempdir guards for cross-source fetches. Held here so the clones
    /// outlive the per-source install calls in `install_with_lockfile`.
    #[allow(dead_code)]
    pub guards: Vec<tempfile::TempDir>,
}

/// One source's contribution to a [`DependencyClosure`].
#[derive(Debug)]
pub(super) struct SourceInstall {
    pub root: PathBuf,
    pub git_ref: Option<String>,
    pub items: crate::bundle::ResolvedItems,
}

/// A fetched-and-indexed source, cached by canonical label during DFS.
struct FetchedSource {
    root: PathBuf,
    git_ref: Option<String>,
    index: BTreeMap<(ItemKind, String), PathBuf>,
}

/// Index every item in `source` by its effective `(kind, name)` identity.
fn index_source(source: &Path) -> Result<BTreeMap<(ItemKind, String), PathBuf>> {
    let mut idx = BTreeMap::new();
    for (folder, dir) in iter_item_dirs(source)? {
        for kind in [ItemKind::Skill, ItemKind::Rule, ItemKind::Agent] {
            let entry = dir.join(kind.entrypoint_filename());
            if entry.is_file() {
                let name = super::discovery::probe_effective_name(&entry, kind, &folder);
                idx.insert((kind, name), dir.clone());
            }
        }
    }
    Ok(idx)
}

/// Parse an item's `requires` block. For agents, fold `preload_skills` into
/// `requires.skills` softly (a preloaded skill present in the SAME source is
/// implicitly required; an absent one is a runtime hint, skipped).
fn read_item_requires(
    dir: &Path,
    kind: ItemKind,
    index: &BTreeMap<(ItemKind, String), PathBuf>,
) -> Result<crate::model::ItemRequires> {
    let entry = dir.join(kind.entrypoint_filename());
    let raw = fs::read_to_string(&entry).with_context(|| format!("read {}", entry.display()))?;
    let req = match kind {
        ItemKind::Skill => frontmatter::parse::<Skill>(&raw)?.0.requires,
        ItemKind::Rule => frontmatter::parse::<Rule>(&raw)?.0.requires,
        ItemKind::Agent => {
            let agent = frontmatter::parse::<Agent>(&raw)?.0;
            let mut req = agent.requires;
            for s in &agent.preload_skills {
                let present = index.contains_key(&(ItemKind::Skill, s.clone()));
                if present && !req.skills.iter().any(|r| r.name() == s) {
                    req.skills.push(RequireRef::Name(s.clone()));
                }
            }
            req
        }
    };
    Ok(req)
}

/// DFS state for [`resolve_requires_closure`]. Carries the fetch cache so a
/// source pulled by two edges is fetched once, and the visited/on-path sets
/// keyed for cross-source cycle and conflict detection.
struct Resolver {
    /// Canonical label -> fetched-and-indexed source.
    sources: BTreeMap<String, FetchedSource>,
    /// Tempdir guards for cross-source clones (drained into the closure).
    guards: Vec<tempfile::TempDir>,
    /// Fully-resolved identities (dedup).
    resolved: BTreeSet<(ItemKind, String)>,
    /// The label each identity resolved under — second visit under a
    /// different label is a cross-source conflict.
    item_label: BTreeMap<(ItemKind, String), String>,
    /// Dependency-order accumulation.
    order: Vec<(ItemKind, String)>,
    /// On-path set keyed by `(label, kind, name)` for cycle detection (§2.5).
    on_path: BTreeSet<(String, ItemKind, String)>,
    /// Dependency provenance keyed by identity.
    required_by: BTreeMap<(ItemKind, String), BTreeSet<String>>,
}

impl Resolver {
    /// Ensure `src_dsl` (an `add`-DSL locator) is fetched and indexed;
    /// return its canonical label.
    fn ensure_source(&mut self, src_dsl: &str) -> Result<String> {
        let install_source = parse_install_source(src_dsl)
            .map_err(|e| anyhow::anyhow!("parse requires source `{src_dsl}`: {e}"))?;
        let label = install_source.to_string();
        if !self.sources.contains_key(&label) {
            let (root, guard) = super::git::fetch_ssot(&install_source)
                .with_context(|| format!("fetch requires source `{label}`"))?;
            let index = index_source(&root)?;
            if let Some(g) = guard {
                self.guards.push(g);
            }
            self.sources.insert(
                label.clone(),
                FetchedSource {
                    root,
                    git_ref: source_git_ref(&install_source),
                    index,
                },
            );
        }
        Ok(label)
    }

    fn visit(
        &mut self,
        label: &str,
        node: (ItemKind, String),
        requirer: Option<&(ItemKind, String)>,
    ) -> Result<()> {
        let (kind, name) = node.clone();

        // Cross-source identity conflict: the same (kind, name) reached from
        // two different sources (format-spec §3.7).
        if let Some(prev) = self.item_label.get(&node)
            && prev != label
        {
            anyhow::bail!(
                "{kind} `{name}` is required from two different sources: `{prev}` and `{label}`"
            );
        }

        let dir = self
            .sources
            .get(label)
            .expect("source fetched before visit")
            .index
            .get(&node)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!("{kind} `{name}` is required but not found in source `{label}`")
            })?;

        if let Some((rk, rn)) = requirer {
            self.required_by
                .entry(node.clone())
                .or_default()
                .insert(format!("{rk}:{rn}"));
        }

        if self.resolved.contains(&node) {
            return Ok(());
        }
        let path_key = (label.to_string(), kind, name.clone());
        if self.on_path.contains(&path_key) {
            anyhow::bail!("circular dependency detected including {kind} `{name}`");
        }
        self.on_path.insert(path_key.clone());
        self.item_label.insert(node.clone(), label.to_string());

        let requires = {
            let index = &self.sources.get(label).expect("source present").index;
            read_item_requires(&dir, kind, index)?
        };
        for (dep_kind, refs) in [
            (ItemKind::Rule, &requires.rules),
            (ItemKind::Skill, &requires.skills),
            (ItemKind::Agent, &requires.agents),
        ] {
            for r in refs {
                let dep = (dep_kind, r.name().to_string());
                match r.source() {
                    None => self.visit(label, dep, Some(&node))?,
                    Some(src_dsl) => {
                        let dep_label = self.ensure_source(src_dsl)?;
                        self.visit(&dep_label, dep, Some(&node))?;
                    }
                }
            }
        }

        self.on_path.remove(&path_key);
        self.resolved.insert(node.clone());
        self.order.push(node);
        Ok(())
    }
}

fn source_git_ref(s: &InstallSource) -> Option<String> {
    match s {
        InstallSource::Github(r) => r.git_ref.clone(),
        InstallSource::Git(r) => r.git_ref.clone(),
        InstallSource::LocalPath(_) => None,
    }
}

/// Expand `initial` (identities in the already-fetched entry source) along
/// `requires` edges into the full cross-source closure.
///
/// `entry_label` is the entry source's canonical label, `entry_root` its
/// already-fetched root (seeded into the fetch cache so it is not re-fetched),
/// `entry_git_ref` its pinned ref. Cross-source `{ name, source }` entries
/// are fetched via [`fetch_ssot`], cached by canonical label. Cycles
/// (keyed `(label, kind, name)`) and same-`(kind,name)`-different-source
/// clashes are errors; a required item missing from its source is an error.
pub(super) fn resolve_requires_closure(
    entry_label: &str,
    entry_root: &Path,
    entry_git_ref: Option<&str>,
    initial: &[(ItemKind, String)],
) -> Result<DependencyClosure> {
    let mut resolver = Resolver {
        sources: BTreeMap::new(),
        guards: Vec::new(),
        resolved: BTreeSet::new(),
        item_label: BTreeMap::new(),
        order: Vec::new(),
        on_path: BTreeSet::new(),
        required_by: BTreeMap::new(),
    };
    resolver.sources.insert(
        entry_label.to_string(),
        FetchedSource {
            root: entry_root.to_path_buf(),
            git_ref: entry_git_ref.map(str::to_string),
            index: index_source(entry_root)?,
        },
    );

    for node in initial {
        resolver.visit(entry_label, node.clone(), None)?;
    }

    let mut by_source: BTreeMap<String, SourceInstall> = BTreeMap::new();
    let mut item_source: BTreeMap<(ItemKind, String), String> = BTreeMap::new();
    for node in &resolver.order {
        let label = resolver
            .item_label
            .get(node)
            .cloned()
            .expect("resolved item has a source");
        let group = by_source.entry(label.clone()).or_insert_with(|| {
            let fs = resolver.sources.get(&label).expect("source present");
            SourceInstall {
                root: fs.root.clone(),
                git_ref: fs.git_ref.clone(),
                items: crate::bundle::ResolvedItems::default(),
            }
        });
        match node.0 {
            ItemKind::Rule => group.items.rules.push(node.1.clone()),
            ItemKind::Skill => group.items.skills.push(node.1.clone()),
            ItemKind::Agent => group.items.agents.push(node.1.clone()),
        }
        item_source.insert(node.clone(), label);
    }

    Ok(DependencyClosure {
        by_source,
        item_source,
        required_by: resolver.required_by,
        guards: resolver.guards,
    })
}

// ---------------------------------------------------------------------------
// Bundle-level `requires` transitive closure (cross-source) — ADR-0009
// ---------------------------------------------------------------------------

/// One resolved bundle tagged with the source it was reached from. Drives the
/// per-bundle lockfile `bundles[]` recording and plugin/MCP installation.
#[derive(Debug, Clone)]
pub(super) struct ResolvedBundle {
    pub bundle: crate::model::Bundle,
    pub source_label: String,
    pub git_ref: Option<String>,
}

/// A fetched-and-discovered source, cached by canonical label during the
/// bundle DFS.
struct FetchedBundleSource {
    root: PathBuf,
    git_ref: Option<String>,
    bundles: BTreeMap<String, crate::model::Bundle>,
}

/// The `(kind, name)` items a single bundle directly declares.
fn bundle_item_identities(bundle: &crate::model::Bundle) -> Vec<(ItemKind, String)> {
    let mut out = Vec::new();
    out.extend(
        bundle
            .items
            .rules
            .iter()
            .map(|n| (ItemKind::Rule, n.clone())),
    );
    out.extend(
        bundle
            .items
            .skills
            .iter()
            .map(|n| (ItemKind::Skill, n.clone())),
    );
    out.extend(
        bundle
            .items
            .agents
            .iter()
            .map(|n| (ItemKind::Agent, n.clone())),
    );
    out
}

/// Discover every bundle in `root`, keyed by name. A later bundle with a
/// duplicate name wins (malformed registry; not expected in practice).
fn discover_bundles(root: &Path) -> Result<BTreeMap<String, crate::model::Bundle>> {
    Ok(crate::parse::bundle::discover(root)?
        .into_iter()
        .map(|(_, b)| (b.name.clone(), b))
        .collect())
}

/// DFS state for the bundle closure. Mirrors [`Resolver`] (item-level) but
/// walks bundle `requires` edges and accumulates per-source item sets.
struct BundleResolver {
    sources: BTreeMap<String, FetchedBundleSource>,
    guards: Vec<tempfile::TempDir>,
    /// Bundles fully resolved (dedup), keyed `(label, bundle-name)`.
    resolved: BTreeSet<(String, String)>,
    /// The label each bundle name resolved under; a second visit under a
    /// different label is a cross-source bundle conflict.
    bundle_label: BTreeMap<String, String>,
    /// On-path set keyed `(label, bundle-name)` for cycle detection.
    on_path: BTreeSet<(String, String)>,
    /// Post-order accumulation (dependencies before dependents).
    order: Vec<ResolvedBundle>,
    /// Per source: union of items across reached bundles, in dependency order.
    items_by_source: BTreeMap<String, crate::bundle::ResolvedItems>,
    /// Owning `(label, bundle)` of each item identity — a second owner is an
    /// item conflict (the same-source `union_items` rule, extended across
    /// sources).
    item_owner: BTreeMap<(ItemKind, String), (String, String)>,
    /// Canonical source label each resolved item identity came from.
    item_source: BTreeMap<(ItemKind, String), String>,
}

impl BundleResolver {
    /// Ensure `src_dsl` is fetched and its bundles discovered; return its label.
    fn ensure_source(&mut self, src_dsl: &str) -> Result<String> {
        let install_source = parse_install_source(src_dsl)
            .map_err(|e| anyhow::anyhow!("parse requires source `{src_dsl}`: {e}"))?;
        let label = install_source.to_string();
        if !self.sources.contains_key(&label) {
            let (root, guard) = super::git::fetch_ssot(&install_source)
                .with_context(|| format!("fetch requires source `{label}`"))?;
            if let Some(g) = guard {
                self.guards.push(g);
            }
            let bundles = discover_bundles(&root)?;
            self.sources.insert(
                label.clone(),
                FetchedBundleSource {
                    root,
                    git_ref: source_git_ref(&install_source),
                    bundles,
                },
            );
        }
        Ok(label)
    }

    fn visit(&mut self, label: &str, name: &str) -> Result<()> {
        // Cross-source bundle conflict: same bundle name from two sources.
        if let Some(prev) = self.bundle_label.get(name)
            && prev != label
        {
            anyhow::bail!(
                "bundle `{name}` is required from two different sources: `{prev}` and `{label}`"
            );
        }

        let bundle = self
            .sources
            .get(label)
            .expect("source fetched before visit")
            .bundles
            .get(name)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!("bundle `{name}` is required but not found in source `{label}`")
            })?;

        let path_key = (label.to_string(), name.to_string());
        if self.resolved.contains(&path_key) {
            return Ok(());
        }
        if self.on_path.contains(&path_key) {
            anyhow::bail!("bundle dependency cycle detected including `{name}`");
        }
        self.on_path.insert(path_key.clone());
        self.bundle_label
            .insert(name.to_string(), label.to_string());

        // Accumulate this bundle's items into its source, detecting conflicts.
        for (kind, item_name) in bundle_item_identities(&bundle) {
            let id = (kind, item_name.clone());
            match self.item_owner.get(&id) {
                Some((ol, ob)) if (ol.as_str(), ob.as_str()) != (label, name) => {
                    anyhow::bail!(
                        "item conflict: {kind} `{item_name}` is provided by bundles `{ob}` (`{ol}`) and `{name}` (`{label}`)"
                    );
                }
                Some(_) => {}
                None => {
                    self.item_owner
                        .insert(id.clone(), (label.to_string(), name.to_string()));
                    self.item_source.insert(id.clone(), label.to_string());
                    let items = self.items_by_source.entry(label.to_string()).or_default();
                    match kind {
                        ItemKind::Rule => items.rules.push(item_name),
                        ItemKind::Skill => items.skills.push(item_name),
                        ItemKind::Agent => items.agents.push(item_name),
                    }
                }
            }
        }

        // Walk requires: bare → same source; `{ name, source }` → cross-source.
        for req in &bundle.requires {
            match &req.source {
                None => self.visit(label, &req.name)?,
                Some(src_dsl) => {
                    let dep_label = self.ensure_source(src_dsl)?;
                    self.visit(&dep_label, &req.name)?;
                }
            }
        }

        self.on_path.remove(&path_key);
        self.resolved.insert(path_key);
        let git_ref = self.sources.get(label).and_then(|s| s.git_ref.clone());
        self.order.push(ResolvedBundle {
            bundle,
            source_label: label.to_string(),
            git_ref,
        });
        Ok(())
    }
}

/// Resolve the cross-source bundle closure for `entry_bundles` (names present
/// in the already-fetched entry source). Returns the item-level
/// [`DependencyClosure`] — so the existing per-source install and item lockfile
/// machinery is reused verbatim — plus the ordered resolved bundles for
/// plugin/MCP install and per-bundle lockfile recording.
pub(super) fn resolve_bundle_closure(
    entry_label: &str,
    entry_root: &Path,
    entry_git_ref: Option<&str>,
    entry_bundles: &[String],
) -> Result<(DependencyClosure, Vec<ResolvedBundle>)> {
    let mut resolver = BundleResolver {
        sources: BTreeMap::new(),
        guards: Vec::new(),
        resolved: BTreeSet::new(),
        bundle_label: BTreeMap::new(),
        on_path: BTreeSet::new(),
        order: Vec::new(),
        items_by_source: BTreeMap::new(),
        item_owner: BTreeMap::new(),
        item_source: BTreeMap::new(),
    };
    resolver.sources.insert(
        entry_label.to_string(),
        FetchedBundleSource {
            root: entry_root.to_path_buf(),
            git_ref: entry_git_ref.map(str::to_string),
            bundles: discover_bundles(entry_root)?,
        },
    );

    for name in entry_bundles {
        resolver.visit(entry_label, name)?;
    }

    let mut by_source: BTreeMap<String, SourceInstall> = BTreeMap::new();
    for (lbl, items) in &resolver.items_by_source {
        let fs = resolver.sources.get(lbl).expect("source present");
        by_source.insert(
            lbl.clone(),
            SourceInstall {
                root: fs.root.clone(),
                git_ref: fs.git_ref.clone(),
                items: items.clone(),
            },
        );
    }
    let closure = DependencyClosure {
        by_source,
        item_source: resolver.item_source,
        // Bundle items carry no item-level `required_by`; each bundle's own
        // lockfile entry records which items it provides.
        required_by: BTreeMap::new(),
        guards: resolver.guards,
    };
    Ok((closure, resolver.order))
}

/// Detect a bundle-shaped `add` — a `*.bundle.yaml` source, or positional
/// `names` that all resolve to bundles — and resolve its cross-source closure.
/// Returns `None` when the request is not bundle-shaped (the caller keeps the
/// item-closure / direct-install path), including ambiguous names that match
/// both an item and a bundle (left to the name-resolution path to report).
pub(super) fn resolve_bundle_request(
    label: &str,
    local_source: &Path,
    git_ref: Option<&str>,
    names: &[String],
) -> Result<Option<(DependencyClosure, Vec<ResolvedBundle>)>> {
    if is_bundle_file(local_source) {
        let registry_root = find_registry_root(local_source)?;
        let entry = crate::parse::bundle::load(local_source)?;
        return resolve_bundle_closure(
            label,
            &registry_root,
            git_ref,
            std::slice::from_ref(&entry.name),
        )
        .map(Some);
    }
    if names.is_empty() {
        return Ok(None);
    }
    let all_bundles = names
        .iter()
        .all(|n| find_bundle_by_name(local_source, n).is_some());
    let any_item = names.iter().any(|n| has_matching_items(local_source, n));
    if !all_bundles || any_item {
        return Ok(None);
    }
    resolve_bundle_closure(label, local_source, git_ref, names).map(Some)
}

// ---------------------------------------------------------------------------
// Plugin installation orchestration (ADR-0008)
// ---------------------------------------------------------------------------

/// Iterate all resolved bundles, attempt to install each declared plugin for
/// its target client(s), and return structured results. Follows the
/// warn-skip policy: if a client CLI is not on PATH, records
/// `PluginOutcome::CliNotFound` but does not fail the overall install.
pub(super) fn install_plugins_from_bundles(
    bundles: &[crate::model::Bundle],
    scope: crate::plugin::PluginScope,
    target: &Path,
) -> Vec<PluginResult> {
    use crate::model::bundle::{
        ClaudePluginDescriptor, CopilotPluginDescriptor, OpencodePluginDescriptor,
        VscodePluginDescriptor,
    };

    let mut results = Vec::new();

    for bundle in bundles {
        for (plugin_name, entry) in &bundle.plugins {
            // Claude
            if let Some(claude) = &entry.claude {
                match claude {
                    ClaudePluginDescriptor::Install {
                        source,
                        marketplace,
                        plugin,
                        install_url,
                    } => {
                        let outcome = crate::plugin::install_claude_plugin(
                            source,
                            marketplace,
                            plugin,
                            scope,
                        );
                        let identifier = format!("{plugin}@{marketplace}");
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "claude".into(),
                            outcome,
                            identifier,
                            bundle: bundle.name.clone(),
                            install_url: install_url.clone(),
                            instructions_url: None,
                            summary: None,
                        });
                    }
                    ClaudePluginDescriptor::Instructions {
                        instructions_url,
                        summary,
                    } => {
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "claude".into(),
                            outcome: crate::plugin::PluginOutcome::ManualInstructions,
                            identifier: String::new(),
                            bundle: bundle.name.clone(),
                            install_url: None,
                            instructions_url: Some(instructions_url.clone()),
                            summary: summary.clone(),
                        });
                    }
                }
            }

            // VS Code
            if let Some(vscode) = &entry.vscode {
                match vscode {
                    VscodePluginDescriptor::Install {
                        extension,
                        install_url,
                    } => {
                        let outcome = crate::plugin::install_vscode_extension(extension);
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "vscode".into(),
                            outcome,
                            identifier: extension.clone(),
                            bundle: bundle.name.clone(),
                            install_url: install_url.clone(),
                            instructions_url: None,
                            summary: None,
                        });
                    }
                    VscodePluginDescriptor::Instructions {
                        instructions_url,
                        summary,
                    } => {
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "vscode".into(),
                            outcome: crate::plugin::PluginOutcome::ManualInstructions,
                            identifier: String::new(),
                            bundle: bundle.name.clone(),
                            install_url: None,
                            instructions_url: Some(instructions_url.clone()),
                            summary: summary.clone(),
                        });
                    }
                }
            }

            // opencode
            if let Some(opencode) = &entry.opencode {
                match opencode {
                    OpencodePluginDescriptor::Install {
                        module,
                        install_url,
                    } => {
                        let outcome = crate::plugin::install_opencode_plugin(module);
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "opencode".into(),
                            outcome,
                            identifier: module.clone(),
                            bundle: bundle.name.clone(),
                            install_url: install_url.clone(),
                            instructions_url: None,
                            summary: None,
                        });
                    }
                    OpencodePluginDescriptor::Instructions {
                        instructions_url,
                        summary,
                    } => {
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "opencode".into(),
                            outcome: crate::plugin::PluginOutcome::ManualInstructions,
                            identifier: String::new(),
                            bundle: bundle.name.clone(),
                            install_url: None,
                            instructions_url: Some(instructions_url.clone()),
                            summary: summary.clone(),
                        });
                    }
                    OpencodePluginDescriptor::ConfigWrite {
                        plugin_uri,
                        install_url,
                    } => {
                        let outcome =
                            crate::ancillary::write_opencode_plugin_uri(target, plugin_uri);
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "opencode".into(),
                            outcome,
                            identifier: plugin_uri.clone(),
                            bundle: bundle.name.clone(),
                            install_url: install_url.clone(),
                            instructions_url: None,
                            summary: None,
                        });
                    }
                }
            }

            // Copilot CLI
            if let Some(copilot) = &entry.copilot {
                match copilot {
                    CopilotPluginDescriptor::Install {
                        source,
                        marketplace,
                        plugin,
                        install_url,
                    } => {
                        let outcome =
                            crate::plugin::install_copilot_plugin(source, marketplace, plugin);
                        let identifier = format!("{plugin}@{marketplace}");
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "copilot".into(),
                            outcome,
                            identifier,
                            bundle: bundle.name.clone(),
                            install_url: install_url.clone(),
                            instructions_url: None,
                            summary: None,
                        });
                    }
                    CopilotPluginDescriptor::Instructions {
                        instructions_url,
                        summary,
                    } => {
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "copilot".into(),
                            outcome: crate::plugin::PluginOutcome::ManualInstructions,
                            identifier: String::new(),
                            bundle: bundle.name.clone(),
                            install_url: None,
                            instructions_url: Some(instructions_url.clone()),
                            summary: summary.clone(),
                        });
                    }
                }
            }
        }
    }

    results
}

// ---------------------------------------------------------------------------
// MCP server configuration orchestration (ADR-0010)
// ---------------------------------------------------------------------------

/// Iterate all resolved bundles and configure each declared MCP server for
/// every [`McpTarget`](crate::mcp::McpTarget): Claude, Copilot, VS Code, and
/// opencode (ADR-0010, issue #237). Per target, CLI-first; on `CliNotFound`,
/// fall back to the target's config-write file. Warn-skip preserved: a single
/// target's failure or skip never aborts the overall install. One `McpResult`
/// is recorded per `(name, target)`.
///
/// Covered by the `tests/pipeline_mcp.rs` integration test, which clears
/// PATH so the config-write fallback runs deterministically into a tempdir
/// (an in-process unit test cannot safely force the client CLIs off PATH).
pub(super) fn install_mcps_from_bundles(
    bundles: &[crate::model::Bundle],
    scope: crate::plugin::PluginScope,
    target: &Path,
    selection: &ClientSelection,
) -> Vec<McpResult> {
    use crate::mcp::McpTarget;

    let mut results = Vec::new();
    for bundle in bundles {
        for (name, entry) in &bundle.mcps {
            for mcp_target in McpTarget::ALL {
                if !selection.targets_mcp(mcp_target) {
                    continue;
                }
                let outcome = configure_mcp_for_target(mcp_target, name, entry, scope, target);
                results.push(McpResult {
                    name: name.clone(),
                    client: mcp_target.name().into(),
                    outcome,
                    bundle: bundle.name.clone(),
                    requires_env: entry.requires_env.clone(),
                });
            }
        }
    }
    results
}

/// Configure one MCP server into one target: CLI-first, then the target's
/// config-write fallback when the CLI is absent. opencode has no `mcp add`
/// verb, so it goes straight to config-write.
fn configure_mcp_for_target(
    mcp_target: crate::mcp::McpTarget,
    name: &str,
    entry: &crate::model::bundle::McpEntry,
    scope: crate::plugin::PluginScope,
    target: &Path,
) -> crate::plugin::PluginOutcome {
    use crate::mcp::McpTarget;
    use crate::model::bundle::McpTransport;
    use crate::plugin::PluginOutcome;

    // CLI-first. opencode is config-only — treat as CliNotFound so the match
    // below routes it to config-write.
    let cli_outcome = match (mcp_target, &entry.transport) {
        (McpTarget::Claude, McpTransport::Local(l)) => {
            crate::mcp::install_claude_local(name, l, scope)
        }
        (McpTarget::Claude, McpTransport::Remote(r)) => {
            crate::mcp::install_claude_remote(name, r, scope)
        }
        (McpTarget::Copilot, McpTransport::Local(l)) => crate::mcp::install_copilot_local(name, l),
        (McpTarget::Copilot, McpTransport::Remote(r)) => {
            crate::mcp::install_copilot_remote(name, r)
        }
        (McpTarget::VsCode, McpTransport::Local(l)) => crate::mcp::install_vscode_local(name, l),
        (McpTarget::VsCode, McpTransport::Remote(r)) => crate::mcp::install_vscode_remote(name, r),
        (McpTarget::OpenCode, _) => PluginOutcome::CliNotFound,
    };

    match cli_outcome {
        PluginOutcome::CliNotFound => config_write_mcp_for_target(mcp_target, name, entry, target),
        other => other,
    }
}

/// Config-write fallback dispatch — one writer per target/transport. Copilot
/// writes its user-scope `~/.copilot/mcp-config.json` (no project file).
fn config_write_mcp_for_target(
    mcp_target: crate::mcp::McpTarget,
    name: &str,
    entry: &crate::model::bundle::McpEntry,
    target: &Path,
) -> crate::plugin::PluginOutcome {
    use crate::ancillary;
    use crate::mcp::McpTarget;
    use crate::model::bundle::McpTransport;

    match (mcp_target, &entry.transport) {
        (McpTarget::Claude, McpTransport::Local(l)) => {
            ancillary::write_claude_mcp_local(target, name, l)
        }
        (McpTarget::Claude, McpTransport::Remote(r)) => {
            ancillary::write_claude_mcp_remote(target, name, r)
        }
        (McpTarget::Copilot, McpTransport::Local(l)) => ancillary::write_copilot_mcp_local(name, l),
        (McpTarget::Copilot, McpTransport::Remote(r)) => {
            ancillary::write_copilot_mcp_remote(name, r)
        }
        (McpTarget::VsCode, McpTransport::Local(l)) => {
            ancillary::write_vscode_mcp_local(target, name, l)
        }
        (McpTarget::VsCode, McpTransport::Remote(r)) => {
            ancillary::write_vscode_mcp_remote(target, name, r)
        }
        (McpTarget::OpenCode, McpTransport::Local(l)) => {
            ancillary::write_opencode_mcp_local(target, name, l)
        }
        (McpTarget::OpenCode, McpTransport::Remote(r)) => {
            ancillary::write_opencode_mcp_remote(target, name, r)
        }
    }
}

/// The per-kind frontmatter fields the install loop needs after parsing,
/// plus the rendered per-client output. Groups what would otherwise be a
/// `clippy::type_complexity`-flagged tuple out of the `match kind` arms.
struct ParsedItem {
    /// Effective identity name (§2.1): the folder for skills, the
    /// frontmatter `name` (else folder) for rules/agents. Drives output
    /// paths, link-rewrite namespace, and lockfile identity.
    name: String,
    audience: Option<Vec<Audience>>,
    ignore: Vec<String>,
    renders: Vec<(Client, String)>,
}

fn install_items_of_kind(
    kind: ItemKind,
    source: &Path,
    target: &Path,
    report: &mut InstallReport,
    filter: Option<&crate::bundle::ResolvedItems>,
    selection: &ClientSelection,
) -> Result<()> {
    let entrypoint = kind.entrypoint_filename();
    for (folder, dir) in iter_item_dirs(source)? {
        let entry_path = dir.join(entrypoint);
        if !entry_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&entry_path)
            .with_context(|| format!("read {}", entry_path.display()))?;

        // Parse frontmatter, resolve the effective identity name, extract
        // audience + ignore, and render per client. Each kind has its own
        // model type, so we dispatch here.
        let ParsedItem {
            name,
            audience,
            ignore,
            renders,
        } = match kind {
            ItemKind::Skill => {
                let (skill, body) = frontmatter::parse::<Skill>(&raw)
                    .with_context(|| format!("parse {}", entry_path.display()))?;
                let name = super::discovery::effective_name(kind, skill.name.as_deref(), &folder);
                let aud = skill.audience.clone();
                let ignore = skill.ignore.clone();
                let mut out = Vec::new();
                for client in ALL_CLIENTS {
                    if !targets(client, aud.as_deref()) || !selection.targets_generation(client) {
                        continue;
                    }
                    let rendered = generate::render_skill(&skill, &name, body, client)
                        .with_context(|| format!("render skill {} for {:?}", name, client))?;
                    out.push((client, rendered));
                }
                ParsedItem {
                    name,
                    audience: aud,
                    ignore,
                    renders: out,
                }
            }
            ItemKind::Rule => {
                let (rule, body) = frontmatter::parse::<Rule>(&raw)
                    .with_context(|| format!("parse {}", entry_path.display()))?;
                let name = super::discovery::effective_name(kind, rule.name.as_deref(), &folder);
                let aud = rule.audience.clone();
                let ignore = rule.ignore.clone();
                let mut out = Vec::new();
                for client in ALL_CLIENTS {
                    if !targets(client, aud.as_deref()) || !selection.targets_generation(client) {
                        continue;
                    }
                    let rendered = generate::render_rule(&rule, &name, body, client)
                        .with_context(|| format!("render rule {} for {:?}", name, client))?;
                    out.push((client, rendered));
                }
                ParsedItem {
                    name,
                    audience: aud,
                    ignore,
                    renders: out,
                }
            }
            ItemKind::Agent => {
                let (agent, body) = frontmatter::parse::<Agent>(&raw)
                    .with_context(|| format!("parse {}", entry_path.display()))?;
                let name = super::discovery::effective_name(kind, agent.name.as_deref(), &folder);
                let aud = agent.audience.clone();
                let ignore = agent.ignore.clone();
                let mut out = Vec::new();
                for client in ALL_CLIENTS {
                    if !targets(client, aud.as_deref()) || !selection.targets_generation(client) {
                        continue;
                    }
                    let rendered = generate::render_agent(&agent, &name, body, client)
                        .with_context(|| format!("render agent {} for {:?}", name, client))?;
                    out.push((client, rendered));
                }
                ParsedItem {
                    name,
                    audience: aud,
                    ignore,
                    renders: out,
                }
            }
        };

        // Audience/identity filtering keys on the effective name, not the
        // folder, so a filter referencing a divergent rule/agent name
        // resolves correctly.
        if let Some(items) = filter
            && !items.contains(kind, &name)
        {
            continue;
        }

        let source_hash = hash_item_dir(&dir);
        let resources = super::ignore::filter_ignored(iter_item_resources(&dir), &ignore);
        let copied: std::collections::HashSet<std::path::PathBuf> =
            resources.iter().cloned().collect();

        // Clean existing output directories for this specific item before
        // writing new outputs. This removes stale sibling files while keeping
        // other items' outputs intact if generation fails later.
        remove_item_outputs(target, kind, &name);

        for (client, rendered) in &renders {
            // Flat-kind entrypoints (Claude/Copilot rules, all agents) live
            // beside a `<name>/` resource directory; rewrite their relative
            // resource links to point into it. Directory-backed kinds need
            // no rewrite (entrypoint and resources share the directory).
            let body = if !copied.is_empty() && !is_dir_backed(kind, *client) {
                crate::generate::link_rewrite::rewrite_resource_links(rendered, &name, &copied)
                    .with_context(|| format!("rewrite resource links for {name} ({client:?})"))?
            } else {
                rendered.clone()
            };
            let rel = output_path(kind, *client, &name);
            write_output(target, &rel, &body)?;
            copy_item_resources(target, &dir, kind, *client, &name, &resources)?;
            report.items.push(InstalledItem {
                kind,
                name: name.clone(),
                client: *client,
                output_path: rel,
                source_hash: source_hash.clone(),
                group: Some(folder.clone()),
            });
        }

        // Clean up outputs for clients no longer targeted — by author
        // audience or by the consumer selection (ADR-0012).
        for client in ALL_CLIENTS {
            if targets(client, audience.as_deref()) && selection.targets_generation(client) {
                continue;
            }
            let rel = output_path(kind, client, &name);
            let full = target.join(&rel);
            if full.exists() {
                let _ = fs::remove_file(&full);
            }
            if let Some(parent) = full.parent()
                && parent
                    .file_name()
                    .and_then(|f| f.to_str())
                    .is_some_and(|f| f == name)
                && parent.is_dir()
            {
                let _ = fs::remove_dir(parent);
            }
        }
    }
    Ok(())
}

fn targets(client: Client, audience: Option<&[Audience]>) -> bool {
    match audience {
        None => true,
        Some(list) => list.iter().any(|a| audience_matches(client, *a)),
    }
}

fn audience_matches(client: Client, a: Audience) -> bool {
    matches!(
        (client, a),
        (Client::Claude, Audience::Claude)
            | (Client::Copilot, Audience::Copilot)
            | (Client::OpenCode, Audience::OpenCode)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audience_none_targets_all_clients() {
        for c in ALL_CLIENTS {
            assert!(targets(c, None));
        }
    }

    #[test]
    fn audience_subset_filters_other_clients() {
        let only_claude = vec![Audience::Claude];
        assert!(targets(Client::Claude, Some(&only_claude)));
        assert!(!targets(Client::Copilot, Some(&only_claude)));
        assert!(!targets(Client::OpenCode, Some(&only_claude)));
    }

    fn write_item(root: &Path, kind: ItemKind, name: &str, frontmatter_extra: &str) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        let body = format!(
            "---\nschema: 1\nname: {name}\ndescription: test {name}\n{frontmatter_extra}---\n# {name}\n"
        );
        fs::write(dir.join(kind.entrypoint_filename()), body).unwrap();
    }

    #[test]
    fn closure_pulls_same_source_dependency() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path();
        write_item(
            src,
            ItemKind::Agent,
            "code-review",
            "requires:\n  skills: [sarif]\n",
        );
        write_item(src, ItemKind::Skill, "sarif", "");

        let closure = resolve_requires_closure(
            "local:test",
            src,
            None,
            &[(ItemKind::Agent, "code-review".to_string())],
        )
        .expect("closure");

        assert!(
            closure
                .item_source
                .contains_key(&(ItemKind::Agent, "code-review".to_string()))
        );
        assert!(
            closure
                .item_source
                .contains_key(&(ItemKind::Skill, "sarif".to_string()))
        );
        let prov = closure
            .required_by
            .get(&(ItemKind::Skill, "sarif".to_string()))
            .expect("sarif provenance");
        assert!(prov.contains("agent:code-review"), "{prov:?}");
    }

    #[test]
    fn closure_pulls_cross_source_dependency() {
        let tmp = tempfile::tempdir().unwrap();
        let src_a = tmp.path().join("a");
        let src_b = tmp.path().join("b");
        std::fs::create_dir_all(&src_a).unwrap();
        std::fs::create_dir_all(&src_b).unwrap();
        write_item(&src_b, ItemKind::Skill, "sarif", "");
        write_item(
            &src_a,
            ItemKind::Agent,
            "code-review",
            &format!(
                "requires:\n  skills: [{{ name: sarif, source: {} }}]\n",
                src_b.display()
            ),
        );

        let a_label = format!("local:{}", src_a.display());
        let b_label = format!("local:{}", src_b.display());
        let closure = resolve_requires_closure(
            &a_label,
            &src_a,
            None,
            &[(ItemKind::Agent, "code-review".to_string())],
        )
        .expect("closure");

        assert_eq!(
            closure
                .item_source
                .get(&(ItemKind::Agent, "code-review".to_string())),
            Some(&a_label)
        );
        assert_eq!(
            closure
                .item_source
                .get(&(ItemKind::Skill, "sarif".to_string())),
            Some(&b_label)
        );
        assert!(
            closure.by_source[&b_label]
                .items
                .skills
                .contains(&"sarif".to_string())
        );
        let prov = closure
            .required_by
            .get(&(ItemKind::Skill, "sarif".to_string()))
            .expect("sarif provenance");
        assert!(prov.contains("agent:code-review"), "{prov:?}");
    }

    #[test]
    fn closure_rejects_cycle() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path();
        write_item(src, ItemKind::Rule, "a", "requires:\n  rules: [b]\n");
        write_item(src, ItemKind::Rule, "b", "requires:\n  rules: [a]\n");

        let err = resolve_requires_closure(
            "local:test",
            src,
            None,
            &[(ItemKind::Rule, "a".to_string())],
        )
        .expect_err("cycle");
        assert!(err.to_string().contains("circular"), "{err}");
    }

    #[test]
    fn closure_errors_on_missing_cross_source_item() {
        let tmp = tempfile::tempdir().unwrap();
        let src_a = tmp.path().join("a");
        let src_b = tmp.path().join("b");
        std::fs::create_dir_all(&src_a).unwrap();
        std::fs::create_dir_all(&src_b).unwrap(); // exists but holds no `x`
        write_item(
            &src_a,
            ItemKind::Rule,
            "a",
            &format!(
                "requires:\n  skills: [{{ name: x, source: {} }}]\n",
                src_b.display()
            ),
        );

        let err = resolve_requires_closure(
            &format!("local:{}", src_a.display()),
            &src_a,
            None,
            &[(ItemKind::Rule, "a".to_string())],
        )
        .expect_err("missing cross-source item");
        assert!(err.to_string().contains("not found in source"), "{err}");
    }

    #[test]
    fn closure_rejects_cross_source_cycle() {
        let tmp = tempfile::tempdir().unwrap();
        let src_a = tmp.path().join("a");
        let src_b = tmp.path().join("b");
        std::fs::create_dir_all(&src_a).unwrap();
        std::fs::create_dir_all(&src_b).unwrap();
        write_item(
            &src_a,
            ItemKind::Rule,
            "alpha",
            &format!(
                "requires:\n  rules: [{{ name: beta, source: {} }}]\n",
                src_b.display()
            ),
        );
        write_item(
            &src_b,
            ItemKind::Rule,
            "beta",
            &format!(
                "requires:\n  rules: [{{ name: alpha, source: {} }}]\n",
                src_a.display()
            ),
        );

        let err = resolve_requires_closure(
            &format!("local:{}", src_a.display()),
            &src_a,
            None,
            &[(ItemKind::Rule, "alpha".to_string())],
        )
        .expect_err("cross-source cycle");
        assert!(err.to_string().contains("circular"), "{err}");
    }

    #[test]
    fn closure_rejects_same_item_from_two_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let entry = tmp.path().join("entry");
        let src_b = tmp.path().join("b");
        let src_c = tmp.path().join("c");
        std::fs::create_dir_all(&entry).unwrap();
        std::fs::create_dir_all(&src_b).unwrap();
        std::fs::create_dir_all(&src_c).unwrap();
        write_item(&src_b, ItemKind::Skill, "sarif", "");
        write_item(&src_c, ItemKind::Skill, "sarif", "");
        write_item(
            &entry,
            ItemKind::Rule,
            "wants-b",
            &format!(
                "requires:\n  skills: [{{ name: sarif, source: {} }}]\n",
                src_b.display()
            ),
        );
        write_item(
            &entry,
            ItemKind::Rule,
            "wants-c",
            &format!(
                "requires:\n  skills: [{{ name: sarif, source: {} }}]\n",
                src_c.display()
            ),
        );

        let err = resolve_requires_closure(
            &format!("local:{}", entry.display()),
            &entry,
            None,
            &[
                (ItemKind::Rule, "wants-b".to_string()),
                (ItemKind::Rule, "wants-c".to_string()),
            ],
        )
        .expect_err("conflict");
        assert!(err.to_string().contains("two different sources"), "{err}");
    }

    #[test]
    fn closure_folds_preload_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path();
        write_item(
            src,
            ItemKind::Agent,
            "code-review",
            "preload-skills: [sarif]\n",
        );
        write_item(src, ItemKind::Skill, "sarif", "");

        let closure = resolve_requires_closure(
            "local:test",
            src,
            None,
            &[(ItemKind::Agent, "code-review".to_string())],
        )
        .expect("closure");

        assert!(
            closure
                .item_source
                .contains_key(&(ItemKind::Skill, "sarif".to_string()))
        );
        let prov = closure
            .required_by
            .get(&(ItemKind::Skill, "sarif".to_string()))
            .expect("sarif provenance");
        assert!(prov.contains("agent:code-review"), "{prov:?}");
    }

    #[test]
    fn closure_skips_absent_preload_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path();
        write_item(
            src,
            ItemKind::Agent,
            "reviewer",
            "preload-skills: [missing-skill]\n",
        );

        let closure = resolve_requires_closure(
            "local:test",
            src,
            None,
            &[(ItemKind::Agent, "reviewer".to_string())],
        )
        .expect("closure");

        assert!(
            closure
                .item_source
                .contains_key(&(ItemKind::Agent, "reviewer".to_string()))
        );
        assert!(
            !closure
                .item_source
                .contains_key(&(ItemKind::Skill, "missing-skill".to_string()))
        );
        assert!(
            !closure
                .required_by
                .contains_key(&(ItemKind::Skill, "missing-skill".to_string()))
        );
    }
}
