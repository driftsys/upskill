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
use std::fs;
use std::path::Path;

use super::discovery::{
    detect_item_entrypoint, find_bundle_by_name, find_registry_root, has_matching_items,
    is_bundle_file, iter_item_dirs,
};
use super::hash::hash_item_dir;
use super::output::{output_path, remove_item_outputs, write_output};
use super::{ALL_CLIENTS, InstallReport, InstalledItem, ItemKind, PluginResult};
use crate::generate::{self, Client};
use crate::model::{Agent, Audience, Rule, Skill};
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

fn install_items_of_kind(
    kind: ItemKind,
    source: &Path,
    target: &Path,
    report: &mut InstallReport,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<()> {
    let entrypoint = kind.entrypoint_filename();
    for (name, dir) in iter_item_dirs(source)? {
        if let Some(items) = filter
            && !items.contains(kind, &name)
        {
            continue;
        }
        let entry_path = dir.join(entrypoint);
        if !entry_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&entry_path)
            .with_context(|| format!("read {}", entry_path.display()))?;

        // Parse frontmatter, extract audience, and define a render helper.
        // Each kind has its own model type, so we dispatch here.
        let (audience, renders): (Option<Vec<Audience>>, Vec<(Client, String)>) = match kind {
            ItemKind::Skill => {
                let (skill, body) = frontmatter::parse::<Skill>(&raw)
                    .with_context(|| format!("parse {}", entry_path.display()))?;
                let aud = skill.audience.clone();
                let mut out = Vec::new();
                for client in ALL_CLIENTS {
                    if !targets(client, aud.as_deref()) {
                        continue;
                    }
                    let rendered = generate::render_skill(&skill, body, client)
                        .with_context(|| format!("render skill {} for {:?}", name, client))?;
                    out.push((client, rendered));
                }
                (aud, out)
            }
            ItemKind::Rule => {
                let (rule, body) = frontmatter::parse::<Rule>(&raw)
                    .with_context(|| format!("parse {}", entry_path.display()))?;
                let aud = rule.audience.clone();
                let mut out = Vec::new();
                for client in ALL_CLIENTS {
                    if !targets(client, aud.as_deref()) {
                        continue;
                    }
                    let rendered = generate::render_rule(&rule, body, client)
                        .with_context(|| format!("render rule {} for {:?}", name, client))?;
                    out.push((client, rendered));
                }
                (aud, out)
            }
            ItemKind::Agent => {
                let (agent, body) = frontmatter::parse::<Agent>(&raw)
                    .with_context(|| format!("parse {}", entry_path.display()))?;
                let aud = agent.audience.clone();
                let mut out = Vec::new();
                for client in ALL_CLIENTS {
                    if !targets(client, aud.as_deref()) {
                        continue;
                    }
                    let rendered = generate::render_agent(&agent, body, client)
                        .with_context(|| format!("render agent {} for {:?}", name, client))?;
                    out.push((client, rendered));
                }
                (aud, out)
            }
        };

        let source_hash = hash_item_dir(&dir);

        // Clean existing output directories for this specific item before
        // writing new outputs. This removes stale sibling files while keeping
        // other items' outputs intact if generation fails later.
        remove_item_outputs(target, kind, &name);

        for (client, rendered) in &renders {
            let rel = output_path(kind, *client, &name);
            write_output(target, &rel, rendered)?;
            report.items.push(InstalledItem {
                kind,
                name: name.clone(),
                client: *client,
                output_path: rel,
                source_hash: source_hash.clone(),
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
}
