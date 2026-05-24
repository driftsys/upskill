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
//! Authentication: when a token is resolved via [`crate::auth`], it is
//! URL-injected into the clone URL. With no token, clones fall back to
//! git's own credential helpers.
//!
//! Audience filter: the top-level `audience` field (per format-spec §3.1)
//! restricts emission to listed clients; absence means all clients.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::generate::{self, Client};
use crate::model::{Agent, Audience, Rule, Skill};
use crate::parse::frontmatter;
use crate::source::InstallSource;

mod discovery;
mod git;
mod hash;
mod output;
mod report;

use discovery::{
    detect_item_entrypoint, find_bundle_by_name, find_registry_root, has_matching_items,
    is_bundle_file, iter_item_dirs, scan_source_items,
};
pub use git::{fetch_ssot, install_from_git_url, install_from_source};
pub(crate) use hash::hash_item_dir;
use hash::hash_source_items;
use output::{output_path, remove_item_outputs, write_output};
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

const ALL_CLIENTS: [Client; 3] = [Client::Claude, Client::Copilot, Client::OpenCode];

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

fn install_bundle_file(bundle_path: &Path, target: &Path) -> Result<InstallReport> {
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

/// Replace the item name component in an output path.
/// E.g., `.claude/skills/foo/SKILL.md` → `.claude/skills/bar/SKILL.md`
fn rename_output_path(path: &Path, old_name: &str, new_name: &str) -> PathBuf {
    path.iter()
        .map(|component| {
            if component == old_name {
                std::ffi::OsStr::new(new_name)
            } else {
                component
            }
        })
        .collect()
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

    // -- Conflict detection (before any files are written) --
    let mut lock = crate::lockfile::Lockfile::load(target)?;
    let incoming: Vec<(ItemKind, String)> = {
        let mut seen = std::collections::BTreeSet::new();
        scanned_items
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
            .collect()
    };
    let conflicts = crate::conflict::detect_conflicts(&incoming, &lock, &label);
    if !conflicts.is_empty() && !options.force {
        anyhow::bail!("{}", crate::conflict::format_conflict_error(&conflicts));
    }

    // -- Now proceed with generation (files are written here) --
    let mut report = if items.is_empty() {
        install_from_local_path(&local_source, target, None)?
    } else {
        install_with_name_resolution_from_local(&local_source, target, items)?
    };

    // -- Plugin installation (ADR-0008) --
    let plugin_results = install_plugins_from_bundles(&report.bundles, plugin_scope, target);
    report.plugin_results = plugin_results;

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
        // Rename output files on disk and update report items.
        for item in &mut report.items {
            let alias = options
                .aliases
                .iter()
                .find(|(from, _)| from.is_empty() || *from == item.name);
            if let Some((_, alias_name)) = alias {
                let old_path = target.join(&item.output_path);
                let new_output = rename_output_path(&item.output_path, &item.name, alias_name);
                let new_path = target.join(&new_output);

                if old_path.exists() {
                    if let Some(parent) = new_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::rename(&old_path, &new_path).with_context(|| {
                        format!("rename {} to {}", old_path.display(), new_path.display())
                    })?;
                    // Clean up empty parent directory
                    if let Some(parent) = old_path.parent() {
                        let _ = fs::remove_dir(parent);
                    }
                }

                aliased_names
                    .entry(alias_name.clone())
                    .or_insert_with(|| item.name.clone());
                item.output_path = new_output;
                item.name = alias_name.clone();
            }
        }
    }

    let git_ref = match source {
        InstallSource::Github(r) => r.git_ref.as_deref(),
        InstallSource::Gitlab(r) => r.git_ref.as_deref(),
        InstallSource::LocalPath(_) => None,
    };
    let hashes: std::collections::BTreeMap<(ItemKind, String), Option<String>> = report
        .items
        .iter()
        .map(|it| ((it.kind, it.name.clone()), it.source_hash.clone()))
        .collect();
    let new_items = crate::lockfile::items_from_report(&report, &label, git_ref, |k, n| {
        hashes.get(&(k, n.to_string())).cloned().flatten()
    });

    for mut item in new_items {
        if let Some(original) = aliased_names.get(&item.name) {
            item.source_name = Some(original.clone());
        }
        lock.upsert(item);
    }
    for bundle in &report.bundles {
        lock.upsert_bundle(crate::lockfile::LockedBundle {
            name: bundle.name.clone(),
            source: label.clone(),
            git_ref: git_ref.map(str::to_string),
            items: bundle_item_names(bundle),
        });
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

/// Like [`install_with_name_resolution_from_local`] but operates on an already-fetched
/// local source path (avoids double-fetch when `install_with_lockfile` has
/// already called `fetch_ssot`).
fn install_with_name_resolution_from_local(
    local_source: &Path,
    target: &Path,
    names: &[String],
) -> Result<InstallReport> {
    let mut bundle_paths: Vec<PathBuf> = Vec::new();
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
fn install_plugins_from_bundles(
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

    let to_remove: Vec<crate::lockfile::LockedItem> = match &filter {
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

    let mut report = RemoveReport::default();
    for entry in &to_remove {
        let kind = entry.kind;
        let mut deleted_files = Vec::new();
        for client in ALL_CLIENTS {
            let rel = output_path(kind, client, &entry.name);
            let full = target.join(&rel);
            if full.exists() {
                fs::remove_file(&full).with_context(|| format!("delete {}", full.display()))?;
                deleted_files.push(rel);
                if let Some(parent) = full.parent() {
                    let _ = fs::remove_dir(parent);
                }
            }
        }
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
            let item_dir = ssot_root.join(&entry.name);
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
/// `update` always fetches per ADR-0004 — there is no `--offline`.
pub fn update(
    target: &Path,
    names: &[String],
    mode: UpdateMode,
    plugin_scope: crate::plugin::PluginScope,
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
                let options = AddOptions {
                    force: true,
                    aliases,
                    excludes: vec![],
                };
                let install_report =
                    install_with_lockfile(&source, target, &[], plugin_scope, &options)?;
                let mut new_hashes: std::collections::BTreeMap<(ItemKind, String), Option<String>> =
                    std::collections::BTreeMap::new();
                for it in &install_report.items {
                    new_hashes.insert((it.kind, it.name.clone()), it.source_hash.clone());
                }
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
                let new_hashes = hash_source_items(&root);
                for entry in &source_entries {
                    let kind = entry.kind;
                    let lookup_name = entry.source_name.as_deref().unwrap_or(&entry.name);
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

    #[test]
    fn doctor_report_is_clean_when_all_buckets_empty() {
        let report = DoctorReport::default();
        assert!(report.is_clean());
    }

    #[test]
    fn doctor_report_not_clean_with_any_drift() {
        let mut report = DoctorReport::default();
        report.missing_outputs.push(MissingOutput {
            kind: ItemKind::Skill,
            name: "x".into(),
            missing_files: vec![PathBuf::from("a")],
        });
        assert!(!report.is_clean());

        let mut report = DoctorReport::default();
        report.stale_hashes.push(StaleHash {
            kind: ItemKind::Skill,
            name: "x".into(),
            source: "local:/p".into(),
            stored_hash: None,
            current_hash: Some("abc".into()),
        });
        assert!(!report.is_clean());

        let mut report = DoctorReport::default();
        report.orphan_entries.push(OrphanEntry {
            kind: ItemKind::Skill,
            name: "x".into(),
            source: "local:/p".into(),
            reason: OrphanReason::LocalPathGone,
        });
        assert!(!report.is_clean());
    }
}
