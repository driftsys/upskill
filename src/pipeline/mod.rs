//! v0.2 install pipeline: SSOT (local path or git source) → per-client
//! output on disk.
//!
//! Walks a source directory laid out per format-spec §2.1, parses each
//! item's frontmatter into the model (§3), renders per-client output via
//! `crate::generate`, and writes the result to the per-client paths
//! defined in format-spec §7 / ADR-0003.
//!
//! Entry points:
//! - [`install_with_lockfile`] — consumer-facing; install + record state +
//!   create ancillary files. Backs `upskill add`.
//! - [`install_from_source`] / [`install_from_local_path`] /
//!   [`install_from_git_url`] — library-only variants without lockfile or
//!   ancillary handling.
//!
//! Authentication: clones use the bare URL and rely on git's own
//! configuration (credential helpers, `insteadOf` rewrites, SSH);
//! upskill never injects credentials.
//!
//! Audience filter: the top-level `audience` field (per format-spec §3.1)
//! restricts emission to listed clients; absence means all clients.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::generate::Client;
use crate::source::InstallSource;

mod discovery;
mod git;
mod hash;
mod ignore;
mod install;
mod lifecycle;
mod output;
mod report;

use discovery::scan_source_items;
pub use git::{fetch_ssot, install_from_git_url, install_from_source};
pub(crate) use hash::hash_item_dir;
pub use install::install_from_local_path;
use install::{
    ResolvedBundle, install_mcps_from_bundles, install_plugins_from_bundles,
    install_with_name_resolution_from_local, resolve_bundle_request, resolve_requires_closure,
};
pub use lifecycle::{doctor, list, remove, update};
pub use report::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Rule,
    Skill,
    Agent,
}

impl std::fmt::Display for ItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rule => write!(f, "rule"),
            Self::Skill => write!(f, "skill"),
            Self::Agent => write!(f, "agent"),
        }
    }
}

impl std::str::FromStr for ItemKind {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rule" => Ok(Self::Rule),
            "skill" => Ok(Self::Skill),
            "agent" => Ok(Self::Agent),
            other => anyhow::bail!("unknown item kind: {other}"),
        }
    }
}

impl ItemKind {
    /// Every item kind, in declaration order. Single source of truth for
    /// code that must enumerate kinds (e.g. override-file detection).
    pub const ALL: [ItemKind; 3] = [Self::Rule, Self::Skill, Self::Agent];

    pub fn entrypoint_filename(self) -> &'static str {
        match self {
            Self::Rule => "RULE.md",
            Self::Skill => "SKILL.md",
            Self::Agent => "AGENT.md",
        }
    }
}

/// Options that control conflict resolution during install.
#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    /// When true, replace items from different sources without error.
    pub force: bool,
    /// Alias mappings. For direct `--as alt-name`: vec contains `("", "alt-name")`.
    /// For bundle `--as original=alias`: vec contains `("original", "alias")`.
    pub aliases: Vec<(String, String)>,
    /// Item names to skip during install.
    pub excludes: Vec<String>,
}

const ALL_CLIENTS: [Client; 3] = Client::ALL;

/// Flat list of every item name a single bundle declares (in
/// rule/skill/agent order). Used by `install_with_lockfile` to populate
/// `LockedBundle.items` — the per-bundle view, not the transitive
/// closure.
fn bundle_item_names(bundle: &crate::model::Bundle) -> Vec<String> {
    let mut out = Vec::with_capacity(
        bundle.items.rules.len() + bundle.items.skills.len() + bundle.items.agents.len(),
    );
    out.extend(bundle.items.rules.iter().cloned());
    out.extend(bundle.items.skills.iter().cloned());
    out.extend(bundle.items.agents.iter().cloned());
    out
}

/// Move a file or directory from `from` to `to`, creating `to`'s parent
/// directory first and replacing any stale destination. A no-op when `from`
/// does not exist, or when `from` and `to` are the same path.
///
/// Replacing the destination matters on `update` / re-add: the fresh install
/// writes under the item's original name and then relocates to the alias, so
/// a prior aliased install can still be sitting at `to`. A plain `fs::rename`
/// of a directory onto a non-empty directory fails ("Directory not empty"),
/// so the stale destination is removed first.
fn relocate(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() || from == to {
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create dir {}", parent.display()))?;
    }
    if to.is_dir() {
        let _ = fs::remove_dir_all(to);
    } else if to.exists() {
        let _ = fs::remove_file(to);
    }
    fs::rename(from, to).with_context(|| format!("rename {} to {}", from.display(), to.display()))
}

/// Relocate one already-installed per-client output from its original name to
/// `alias`, including any supporting resources (format-spec §2.4).
///
/// - **Directory-backed** kinds (all skills, opencode rules) keep the
///   entrypoint and resources together in `resource_base_path`, so the whole
///   directory is moved. Their links are not namespaced — no rewrite.
/// - **Flat** kinds (Claude/Copilot rules, all agents) keep the entrypoint
///   file beside a sibling `<name>/` resource namespace dir. The entrypoint
///   file and the namespace dir are moved, then the entrypoint's namespaced
///   links are re-prefixed from `<orig>/` to `<alias>/`.
///
/// Returns the new output path (relative to `target`) for the entrypoint.
fn relocate_aliased_output(
    target: &Path,
    item: &report::InstalledItem,
    alias: &str,
) -> Result<PathBuf> {
    let (kind, client, orig) = (item.kind, item.client, item.name.as_str());
    let new_output = output::output_path(kind, client, alias);

    if output::is_dir_backed(kind, client) {
        // Entrypoint + resources share one directory; move it wholesale.
        let old_base = target.join(output::resource_base_path(kind, client, orig));
        let new_base = target.join(output::resource_base_path(kind, client, alias));
        relocate(&old_base, &new_base)?;
        return Ok(new_output);
    }

    // Flat kind: move the entrypoint file, then its resource namespace dir.
    relocate(&target.join(&item.output_path), &target.join(&new_output))?;
    let old_base = target.join(output::resource_base_path(kind, client, orig));
    if old_base.is_dir() {
        let new_base = target.join(output::resource_base_path(kind, client, alias));
        relocate(&old_base, &new_base)?;
        // Re-prefix the entrypoint's namespaced links to the moved dir.
        let copied: std::collections::HashSet<PathBuf> = discovery::iter_item_resources(&new_base)
            .into_iter()
            .collect();
        let entry = target.join(&new_output);
        let body =
            fs::read_to_string(&entry).with_context(|| format!("read {}", entry.display()))?;
        let rewritten =
            crate::generate::link_rewrite::reprefix_resource_links(&body, orig, alias, &copied)
                .with_context(|| format!("re-prefix resource links for {alias} ({client:?})"))?;
        if rewritten != body {
            fs::write(&entry, rewritten).with_context(|| format!("write {}", entry.display()))?;
        }
    }
    Ok(new_output)
}

/// Install + write lockfile. Consumer-facing entry point.
///
/// Calls [`install_from_source`] then merges the resulting [`InstallReport`]
/// into `<target>/.upskill-lock.json` via [`crate::lockfile`]. Existing
/// lockfile entries for the same `(kind, name)` are replaced; entries
/// installed from a different source are left in place.
///
/// `git_ref` recorded per item is taken from the source variant when one
/// is pinned (Github/Gitlab `git_ref`); local-path sources record `None`.
/// `source` label is the [`InstallSource`] `Display` form.
pub fn install_with_lockfile(
    source: &InstallSource,
    target: &Path,
    items: &[String],
    plugin_scope: crate::plugin::PluginScope,
    options: &AddOptions,
) -> Result<InstallReport> {
    let label = source.to_string();

    // -- Pre-flight: fetch SSOT and scan for items before writing anything --
    let (local_source, _tmp_guard) = fetch_ssot(source)?;
    let scanned_items = scan_source_items(&local_source);

    // -- Validate bare --as with multi-item source --
    if options.aliases.iter().any(|(from, _)| from.is_empty()) {
        let unique_names: std::collections::BTreeSet<&str> = scanned_items
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
        let mut seen = std::collections::BTreeSet::new();
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
        InstallSource::Gitlab(r) => r.git_ref.as_deref(),
        InstallSource::LocalPath(_) => None,
    };
    // A bundle-shaped request (`*.bundle.yaml` source or positional names that
    // all resolve to bundles) resolves its own — possibly cross-source — bundle
    // closure (ADR-0009). It produces the same `DependencyClosure` the
    // item-level path uses, so the install + item-lockfile machinery is shared;
    // `resolved_bundles` carries each bundle's own source for the bundle
    // lockfile recording and plugin/MCP install.
    let bundle_resolution = resolve_bundle_request(&label, &local_source, git_ref, items)?;
    let (closure, resolved_bundles): (Option<install::DependencyClosure>, Vec<ResolvedBundle>) =
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

    // -- Conflict detection (before any files are written) --
    let mut lock = crate::lockfile::Lockfile::load(target)?;
    let mut conflicts = Vec::new();
    if let Some(closure) = &closure {
        // Group incoming items by their canonical source label; conflicts
        // are per (kind, name, source). Aliases apply only to entry-source
        // items — dependency-pulled items are never aliased.
        let mut grouped: std::collections::BTreeMap<String, Vec<(ItemKind, String)>> =
            std::collections::BTreeMap::new();
        for ((kind, name), src_label) in &closure.item_source {
            let effective = if *src_label == label {
                options
                    .aliases
                    .iter()
                    .find(|(from, _)| from.is_empty() || *from == *name)
                    .map(|(_, to)| to.clone())
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
        let mut seen = std::collections::BTreeSet::new();
        let incoming: Vec<(ItemKind, String)> = scanned_items
            .iter()
            .filter(|(_, n)| !options.excludes.contains(n))
            .filter(|(_, n)| items.is_empty() || items.contains(n))
            .filter_map(|(kind, name)| {
                let effective_name = options
                    .aliases
                    .iter()
                    .find(|(from, _)| from.is_empty() || *from == *name)
                    .map(|(_, to)| to.clone())
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

    // -- Now proceed with generation (files are written here) --
    // With a closure, install each source's resolved items from that
    // source's own fetched root. Without one (bundle file / name
    // resolution), keep the existing dispatch.
    let mut report = if let Some(closure) = &closure {
        // The cross-source clone guards in `closure.guards` are held alive by
        // the owned `closure` until after every source's items are installed;
        // dropping them early would delete the fetched roots.
        let mut r = InstallReport::default();
        for si in closure.by_source.values() {
            let part = install_from_local_path(&si.root, target, Some(&si.items))?;
            r.items.extend(part.items);
            r.bundles.extend(part.bundles);
        }
        r
    } else if items.is_empty() {
        install_from_local_path(&local_source, target, None)?
    } else {
        install_with_name_resolution_from_local(&local_source, target, items)?
    };

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
    let plugin_results = install_plugins_from_bundles(&report.bundles, plugin_scope, target);
    report.plugin_results = plugin_results;

    // -- MCP server configuration (ADR-0010) --
    let mcp_results = install_mcps_from_bundles(&report.bundles, plugin_scope, target);
    report.mcp_results = mcp_results;

    // -- Apply excludes --
    if !options.excludes.is_empty() {
        report
            .items
            .retain(|item| !options.excludes.contains(&item.name));
    }

    // -- Apply aliases --
    let mut aliased_names: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    if !options.aliases.is_empty() {
        // Relocate output files (and any supporting resources) on disk and
        // update report items.
        for item in &mut report.items {
            // Only entry-source items are aliasable; dependency-pulled
            // (cross-source) items are never renamed. When there is no
            // closure (bundle / name-resolution paths), every item is
            // treated as entry-source, preserving prior behavior.
            let is_entry_source = closure
                .as_ref()
                .and_then(|c| c.item_source.get(&(item.kind, item.name.clone())))
                .is_none_or(|l| *l == label);
            let alias = if is_entry_source {
                options
                    .aliases
                    .iter()
                    .find(|(from, _)| from.is_empty() || *from == item.name)
            } else {
                None
            };
            if let Some((_, alias_name)) = alias {
                let new_output = relocate_aliased_output(target, item, alias_name)?;
                aliased_names
                    .entry(alias_name.clone())
                    .or_insert_with(|| item.name.clone());
                item.output_path = new_output;
                item.name = alias_name.clone();
            }
        }
    }

    let hashes: std::collections::BTreeMap<(ItemKind, String), Option<String>> = report
        .items
        .iter()
        .map(|it| ((it.kind, it.name.clone()), it.source_hash.clone()))
        .collect();
    // Translate `requires` provenance through any applied `--as` aliases so
    // the recorded `required_by` is consistent with the post-alias names that
    // `doctor` matches against (#205). The closure keys dependencies and labels
    // requirers by ORIGINAL effective name; the rename loop above already moved
    // the installed items to their alias. `aliased_names` maps `alias ->
    // original`; invert it to `original -> alias` and rewrite (a) each
    // dependency key's name and (b) each `"{kind}:{name}"` requirer label.
    let original_to_alias: std::collections::HashMap<&str, &str> = aliased_names
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
    let provenance: std::collections::BTreeMap<(ItemKind, String), Vec<String>> =
        if let Some(closure) = &closure {
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
            std::collections::BTreeMap::new()
        };
    let item_source_lookup = |kind: ItemKind, name: &str| -> (String, Option<String>) {
        if let Some(closure) = &closure
            && let Some(src_label) = closure.item_source.get(&(kind, name.to_string()))
        {
            let gr = closure
                .by_source
                .get(src_label)
                .and_then(|si| si.git_ref.clone());
            return (src_label.clone(), gr);
        }
        (label.clone(), git_ref.map(str::to_string))
    };
    let new_items =
        crate::lockfile::items_from_report(&report, item_source_lookup, &provenance, |k, n| {
            hashes.get(&(k, n.to_string())).cloned().flatten()
        });

    for mut item in new_items {
        if let Some(original) = aliased_names.get(&item.name) {
            item.source_name = Some(original.clone());
        }
        lock.upsert(item);
    }
    if resolved_bundles.is_empty() {
        // Same-source bundle install (no cross-source provenance): every bundle
        // came from the entry source.
        for bundle in &report.bundles {
            lock.upsert_bundle(crate::lockfile::LockedBundle {
                name: bundle.name.clone(),
                source: label.clone(),
                git_ref: git_ref.map(str::to_string),
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
                crate::plugin::PluginScope::Project => Some("project".into()),
                crate::plugin::PluginScope::User => Some("user".into()),
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
                crate::plugin::PluginScope::Project => Some("project".into()),
                crate::plugin::PluginScope::User => Some("user".into()),
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
    let has_rules = report.items.iter().any(|i| i.kind == ItemKind::Rule);
    crate::ancillary::ensure_opencode_rules_registered(target, has_rules)?;

    // Per ADR-0003: when the install includes any rule, register
    // `.github/instructions` in `.vscode/settings.json`'s
    // `chat.instructionsFilesLocations` so VS Code Copilot picks up the
    // generated `<name>.instructions.md` files.
    crate::ancillary::ensure_vscode_instructions_registered(target, has_rules)?;

    Ok(report)
}
