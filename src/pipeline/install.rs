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
use super::{ALL_CLIENTS, InstallReport, InstalledItem, ItemKind, PluginResult};
use crate::generate::{self, Client};
use crate::model::{Agent, Audience, RequireRef, Rule, Skill};
use crate::parse::frontmatter;

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
    if is_bundle_file(source) {
        return install_bundle_file(source, target);
    }
    let mut report = InstallReport::default();
    for kind in [ItemKind::Skill, ItemKind::Rule, ItemKind::Agent] {
        install_items_of_kind(kind, source, target, &mut report, filter)?;
    }
    Ok(report)
}

pub(super) fn install_bundle_file(bundle_path: &Path, target: &Path) -> Result<InstallReport> {
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
        let bundle_report = install_bundle_file(bp, target)?;
        report.items.extend(bundle_report.items);
        report.bundles.extend(bundle_report.bundles);
    }

    if !item_names.is_empty() {
        let filter = crate::bundle::ResolvedItems {
            rules: item_names.clone(),
            skills: item_names.clone(),
            agents: item_names.clone(),
        };
        let item_report = install_from_local_path(local_source, target, Some(&filter))?;
        report.items.extend(item_report.items);
    }

    Ok(report)
}

// ---------------------------------------------------------------------------
// Same-source `requires` transitive closure
// ---------------------------------------------------------------------------

/// The set of items reached by expanding a requested item set along
/// same-source `requires` edges, plus per-item provenance recording which
/// installed item(s) pulled each dependency in.
#[derive(Debug)]
pub(super) struct DependencyClosure {
    /// Every item to install: the requested set plus its transitive
    /// same-source dependencies, deduplicated by `(kind, name)`.
    pub items: crate::bundle::ResolvedItems,
    /// `(kind, name) -> { "<requirer-kind>:<requirer-name>", .. }`. Empty
    /// for directly-requested items; populated for items pulled in as a
    /// dependency. Drives the lockfile's `required_by` provenance.
    pub required_by: BTreeMap<(ItemKind, String), BTreeSet<String>>,
}

/// Index every item in `source` by its effective `(kind, name)` identity,
/// mapping to the item directory that holds its entrypoint.
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
/// `requires.skills` (a preloaded skill is implicitly required).
///
/// A preloaded skill that is absent from the source index is treated as a
/// runtime hint, not a hard SSOT dependency, and is NOT folded in: agents
/// may preload built-in or separately-installed skills that this registry
/// does not vend. Explicit `requires` entries remain strict and are never
/// dropped — a missing one bails later in `visit_requires`.
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

/// Expand `initial` along same-source `requires` edges into the full set of
/// items to install, recording dependency provenance.
///
/// Resolution identity is the effective `(kind, name)`. Cross-source
/// `{ name, source }` requires entries are NOT resolved in this release —
/// they bail with a clear error. Dependency cycles bail with a "circular"
/// error. A required item missing from the source bails with a "not found"
/// error.
pub(super) fn resolve_requires_closure(
    source: &Path,
    initial: &[(ItemKind, String)],
) -> Result<DependencyClosure> {
    let index = index_source(source)?;

    let mut required_by: BTreeMap<(ItemKind, String), BTreeSet<String>> = BTreeMap::new();
    let mut resolved: BTreeSet<(ItemKind, String)> = BTreeSet::new();
    let mut order: Vec<(ItemKind, String)> = Vec::new();
    let mut on_path: BTreeSet<(ItemKind, String)> = BTreeSet::new();

    for node in initial {
        visit_requires(
            &index,
            node,
            None,
            &mut required_by,
            &mut resolved,
            &mut order,
            &mut on_path,
        )?;
    }

    let mut items = crate::bundle::ResolvedItems::default();
    for (kind, name) in order {
        match kind {
            ItemKind::Rule => items.rules.push(name),
            ItemKind::Skill => items.skills.push(name),
            ItemKind::Agent => items.agents.push(name),
        }
    }

    Ok(DependencyClosure { items, required_by })
}

#[allow(clippy::too_many_arguments)]
fn visit_requires(
    index: &BTreeMap<(ItemKind, String), PathBuf>,
    node: &(ItemKind, String),
    requirer: Option<&(ItemKind, String)>,
    required_by: &mut BTreeMap<(ItemKind, String), BTreeSet<String>>,
    resolved: &mut BTreeSet<(ItemKind, String)>,
    order: &mut Vec<(ItemKind, String)>,
    on_path: &mut BTreeSet<(ItemKind, String)>,
) -> Result<()> {
    let (kind, name) = node;

    let dir = index.get(node).ok_or_else(|| {
        anyhow::anyhow!("{kind} `{name}` is required but not found in the source")
    })?;

    // Record provenance when reached via a requirer (even if already
    // resolved — multiple installed items may require the same dependency).
    // Directly-requested items (no requirer) get no entry here; they appear
    // in the map only if also pulled in elsewhere.
    if let Some((rk, rn)) = requirer {
        required_by
            .entry(node.clone())
            .or_default()
            .insert(format!("{rk}:{rn}"));
    }

    if resolved.contains(node) {
        return Ok(());
    }
    if on_path.contains(node) {
        anyhow::bail!("circular dependency detected including {kind} `{name}`");
    }
    on_path.insert(node.clone());

    let requires = read_item_requires(dir, *kind, index)?;
    for (dep_kind, refs) in [
        (ItemKind::Rule, &requires.rules),
        (ItemKind::Skill, &requires.skills),
        (ItemKind::Agent, &requires.agents),
    ] {
        for r in refs {
            if let Some(src) = r.source() {
                let rname = r.name();
                anyhow::bail!(
                    "{kind} `{name}` requires {dep_kind} `{rname}` from cross-source `{src}`: \
                     cross-source dependency resolution is not yet available in this release"
                );
            }
            let dep = (dep_kind, r.name().to_string());
            visit_requires(
                index,
                &dep,
                Some(node),
                required_by,
                resolved,
                order,
                on_path,
            )?;
        }
    }

    on_path.remove(node);
    resolved.insert(node.clone());
    order.push(node.clone());
    Ok(())
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
                        plugin,
                        install_url,
                    } => {
                        let outcome = crate::plugin::install_claude_plugin(source, plugin, scope);
                        let identifier = format!("{plugin}@{source}");
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
                        plugin,
                        install_url,
                    } => {
                        let outcome = crate::plugin::install_copilot_plugin(source, plugin);
                        let identifier = format!("{plugin}@{source}");
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
                    if !targets(client, aud.as_deref()) {
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
                    if !targets(client, aud.as_deref()) {
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
                    if !targets(client, aud.as_deref()) {
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

        // Clean up outputs for clients no longer targeted.
        for client in ALL_CLIENTS {
            if targets(client, audience.as_deref()) {
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

        let closure =
            resolve_requires_closure(src, &[(ItemKind::Agent, "code-review".to_string())])
                .expect("closure");

        assert!(closure.items.contains(ItemKind::Agent, "code-review"));
        assert!(closure.items.contains(ItemKind::Skill, "sarif"));
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

        let err =
            resolve_requires_closure(src, &[(ItemKind::Rule, "a".to_string())]).expect_err("cycle");
        assert!(err.to_string().contains("circular"), "{err}");
    }

    #[test]
    fn closure_errors_on_cross_source_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path();
        write_item(
            src,
            ItemKind::Rule,
            "a",
            "requires:\n  skills: [{ name: x, source: org/repo }]\n",
        );

        let err = resolve_requires_closure(src, &[(ItemKind::Rule, "a".to_string())])
            .expect_err("cross-source");
        assert!(err.to_string().contains("cross-source"), "{err}");
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

        let closure =
            resolve_requires_closure(src, &[(ItemKind::Agent, "code-review".to_string())])
                .expect("closure");

        assert!(closure.items.contains(ItemKind::Skill, "sarif"));
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

        let closure = resolve_requires_closure(src, &[(ItemKind::Agent, "reviewer".to_string())])
            .expect("closure");

        assert!(closure.items.contains(ItemKind::Agent, "reviewer"));
        assert!(!closure.items.contains(ItemKind::Skill, "missing-skill"));
        assert!(
            !closure
                .required_by
                .contains_key(&(ItemKind::Skill, "missing-skill".to_string()))
        );
    }
}
