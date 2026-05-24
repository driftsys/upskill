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

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::fetch;
use crate::generate::{self, Client};
use crate::model::{Agent, Audience, Rule, Skill};
use crate::parse::frontmatter;
use crate::source::{GithubRepo, GitlabRepo, InstallSource};

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

#[derive(Debug, Clone)]
pub struct InstalledItem {
    pub kind: ItemKind,
    pub name: String,
    pub client: Client,
    /// Path relative to the install target root.
    pub output_path: PathBuf,
    /// SHA-256 of the SSOT item directory at install time. Used by the
    /// lockfile for drift detection. Repeated across the per-
    /// client entries for the same item — they share one SSOT input.
    pub source_hash: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct InstallReport {
    pub items: Vec<InstalledItem>,
    /// When the install resolved a bundle (entry `source` was a
    /// `.bundle.yaml` file), every reached bundle in dependency order. The
    /// last entry is the bundle the user named. Empty for non-bundle
    /// installs.
    pub bundles: Vec<crate::model::Bundle>,
    /// Results of plugin install attempts (ADR-0008). One entry per
    /// (plugin-name, client) pair attempted. Empty when no bundles with
    /// plugins were resolved.
    pub plugin_results: Vec<PluginResult>,
}

/// Result of a single plugin install attempt, for reporting to the user.
#[derive(Debug, Clone)]
pub struct PluginResult {
    /// Upskill-level plugin name (key in the bundle's `plugins:` map).
    pub name: String,
    /// Client identifier: `"claude"`, `"vscode"`, or `"opencode"`.
    pub client: String,
    /// What happened.
    pub outcome: crate::plugin::PluginOutcome,
    /// Client-specific identifier (for lockfile recording and uninstall).
    pub identifier: String,
    /// Bundle that declared this plugin.
    pub bundle: String,
    /// URL shown in warn-skip message (if available from the descriptor).
    pub install_url: Option<String>,
    /// URL with manual installation instructions (Instructions variant).
    pub instructions_url: Option<String>,
    /// Human-readable summary for manual instructions.
    pub summary: Option<String>,
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
    install_skills(source, target, &mut report, filter)?;
    install_rules(source, target, &mut report, filter)?;
    install_agents(source, target, &mut report, filter)?;
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
    install_skills(&registry_root, target, &mut report, Some(&resolved.items))?;
    install_rules(&registry_root, target, &mut report, Some(&resolved.items))?;
    install_agents(&registry_root, target, &mut report, Some(&resolved.items))?;
    Ok(report)
}

fn is_bundle_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(crate::parse::bundle::BUNDLE_SUFFIX))
}

/// Search `root` recursively for a file named `<name>.bundle.yaml`.
/// Skips hidden directories. Returns the first match or `None`.
fn find_bundle_by_name(root: &Path, name: &str) -> Option<PathBuf> {
    let target_filename = format!("{}{}", name, crate::parse::bundle::BUNDLE_SUFFIX);
    find_bundle_recursive(root, &target_filename)
}

fn find_bundle_recursive(dir: &Path, target: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let entry_name = entry.file_name();
        let name_str = entry_name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_file() && name_str == target {
            return Some(path);
        }
        if path.is_dir()
            && let Some(found) = find_bundle_recursive(&path, target)
        {
            return Some(found);
        }
    }
    None
}

/// Check whether a source directory contains any item (skill, rule, or
/// agent) with the given name. An item exists when a subdirectory named
/// `name` contains at least one of `SKILL.md`, `RULE.md`, or `AGENT.md`.
fn has_matching_items(source: &Path, name: &str) -> bool {
    let item_dir = source.join(name);
    if !item_dir.is_dir() {
        return false;
    }
    item_dir.join("SKILL.md").is_file()
        || item_dir.join("RULE.md").is_file()
        || item_dir.join("AGENT.md").is_file()
}

/// Scan a source directory for item (kind, name) pairs without generating output.
fn scan_source_items(source: &Path) -> Vec<(ItemKind, String)> {
    let mut items = Vec::new();
    let Ok(entries) = fs::read_dir(source) else {
        return items;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if is_item_dir(&path) {
            if path.join("SKILL.md").is_file() {
                items.push((ItemKind::Skill, name.to_string()));
            }
            if path.join("RULE.md").is_file() {
                items.push((ItemKind::Rule, name.to_string()));
            }
            if path.join("AGENT.md").is_file() {
                items.push((ItemKind::Agent, name.to_string()));
            }
        } else {
            // Category subdir: source/<category>/<item>/ENTRY.md
            if let Ok(sub_entries) = fs::read_dir(&path) {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    if !sub_path.is_dir() {
                        continue;
                    }
                    let Some(sub_name) = sub_path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if sub_name.starts_with('.') {
                        continue;
                    }
                    if sub_path.join("SKILL.md").is_file() {
                        items.push((ItemKind::Skill, sub_name.to_string()));
                    }
                    if sub_path.join("RULE.md").is_file() {
                        items.push((ItemKind::Rule, sub_name.to_string()));
                    }
                    if sub_path.join("AGENT.md").is_file() {
                        items.push((ItemKind::Agent, sub_name.to_string()));
                    }
                }
            }
        }
    }
    items
}

/// Walk up from `bundle_path`'s parent until a directory is found that
/// looks like an SSOT root — a directory whose direct children include
/// at least one item directory (containing `RULE.md`, `SKILL.md`, or
/// `AGENT.md`) or another bundle file. Falls back to the bundle's
/// parent directory if no such ancestor exists, so a flat layout
/// (bundle and items in the same dir) still works.
fn find_registry_root(bundle_path: &Path) -> Result<PathBuf> {
    let parent = bundle_path
        .parent()
        .ok_or_else(|| anyhow!("bundle path {} has no parent", bundle_path.display()))?;
    let mut cursor = parent;
    loop {
        if has_ssot_layout(cursor) {
            return Ok(cursor.to_path_buf());
        }
        match cursor.parent() {
            Some(p) => cursor = p,
            None => break,
        }
    }
    Ok(parent.to_path_buf())
}

fn has_ssot_layout(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Direct item check: dir/<item>/ENTRY.md
        if is_item_dir(&path) {
            return true;
        }
        // Grandchild check: dir/<category>/<item>/ENTRY.md
        // Handles sibling layouts where items live in subdirectories
        // (e.g. `skills/<item>/RULE.md` alongside `bundles/`).
        if let Ok(sub_entries) = fs::read_dir(&path) {
            for sub_entry in sub_entries.flatten() {
                let sub_path = sub_entry.path();
                if sub_path.is_dir() && is_item_dir(&sub_path) {
                    return true;
                }
            }
        }
    }
    false
}

/// Returns true when the directory contains at least one SSOT entrypoint
/// file (`RULE.md`, `SKILL.md`, or `AGENT.md`).
fn is_item_dir(path: &Path) -> bool {
    path.join("RULE.md").is_file()
        || path.join("SKILL.md").is_file()
        || path.join("AGENT.md").is_file()
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

/// Detect which entrypoint file (SKILL.md, RULE.md, AGENT.md) exists for
/// an item, for use in error messages.
fn detect_item_entrypoint(source: &Path, name: &str) -> &'static str {
    let dir = source.join(name);
    if dir.join("SKILL.md").is_file() {
        "SKILL.md"
    } else if dir.join("RULE.md").is_file() {
        "RULE.md"
    } else if dir.join("AGENT.md").is_file() {
        "AGENT.md"
    } else {
        "SKILL.md" // fallback for error message
    }
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

/// What to remove. Per ADR-0004 the user must be explicit — bare
/// `upskill remove` is not allowed; either name items or pass
/// `--source <label>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveFilter {
    /// Remove every lockfile entry whose `name` matches one of these
    /// values, regardless of kind. An item name listed here that does not
    /// match any entry is an error (the caller asked to remove a thing
    /// that is not installed).
    ByNames(Vec<String>),
    /// Remove every lockfile entry whose `source` label matches this
    /// string verbatim. No-op when the lockfile contains no entry from
    /// the named source.
    BySource(String),
}

#[derive(Debug, Default, Clone)]
pub struct RemoveReport {
    pub items: Vec<RemovedItem>,
}

#[derive(Debug, Clone)]
pub struct RemovedItem {
    pub kind: ItemKind,
    pub name: String,
    /// Files actually deleted from disk (paths relative to `target`).
    /// May be empty if the lockfile knew about the item but its outputs
    /// were already gone.
    pub deleted_files: Vec<PathBuf>,
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

/// One per-client output file the lockfile said should exist but doesn't.
#[derive(Debug, Clone, Serialize)]
pub struct MissingOutput {
    pub kind: ItemKind,
    pub name: String,
    /// Paths relative to the install target.
    pub missing_files: Vec<PathBuf>,
}

/// SSOT content hash differs from what the lockfile recorded at install
/// time. Only computed for `local:` sources still on disk —
/// remote-source drift is the job of `update --dry-run`, which fetches.
#[derive(Debug, Clone, Serialize)]
pub struct StaleHash {
    pub kind: ItemKind,
    pub name: String,
    pub source: String,
    pub stored_hash: Option<String>,
    pub current_hash: Option<String>,
}

/// Lockfile entry whose source can no longer be reached: the local
/// path is gone or the named item has been removed from the SSOT
/// directory. The user has to `remove` it explicitly to clear the
/// lockfile, since `update` would just fail trying to fetch.
#[derive(Debug, Clone, Serialize)]
pub struct OrphanEntry {
    pub kind: ItemKind,
    pub name: String,
    pub source: String,
    pub reason: OrphanReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrphanReason {
    /// `local:<path>` source no longer resolves to a directory on disk.
    LocalPathGone,
    /// Source still exists but no longer contains the item with this
    /// `(kind, name)` (e.g., it was renamed or removed in the SSOT).
    ItemMissingInSource,
}

/// Plugin in lockfile (status: installed) but not found when querying the
/// client CLI.  Likely uninstalled out-of-band.
#[derive(Debug, Clone, Serialize)]
pub struct MissingPlugin {
    pub name: String,
    pub client: String,
    pub identifier: String,
    pub bundle: String,
}

/// Plugin in lockfile (status: skipped) because the client CLI was not on
/// PATH at install time.  The plugin has never been installed.
#[derive(Debug, Clone, Serialize)]
pub struct SkippedPlugin {
    pub name: String,
    pub client: String,
    pub identifier: String,
    pub bundle: String,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct DoctorReport {
    pub missing_outputs: Vec<MissingOutput>,
    pub stale_hashes: Vec<StaleHash>,
    pub orphan_entries: Vec<OrphanEntry>,
    /// Plugins recorded in the lockfile as `installed` but absent from the
    /// client's installed list (uninstalled out-of-band).  Non-empty →
    /// `is_clean()` returns false → exit 1.
    pub missing_plugins: Vec<MissingPlugin>,
    /// Plugins recorded in the lockfile as `skipped` (warn-skip at install
    /// time). Informational only — does NOT cause `is_clean()` to return
    /// false or trigger exit 1. Run `upskill update` after installing the
    /// missing CLI to install them.
    pub skipped_plugins: Vec<SkippedPlugin>,
}

impl DoctorReport {
    /// True when nothing is wrong — every per-client output is on disk,
    /// every locally-sourced item still hashes the same, every lockfile
    /// entry has a recoverable source, and no installed plugin is missing
    /// from its client.
    ///
    /// `skipped_plugins` (warn-skip outcomes) are informational: the user
    /// never had the CLI at install time, so this is the expected state.
    /// They are reported but do not affect cleanness.
    pub fn is_clean(&self) -> bool {
        self.missing_outputs.is_empty()
            && self.stale_hashes.is_empty()
            && self.orphan_entries.is_empty()
            && self.missing_plugins.is_empty()
    }
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

/// Whether `update` writes or just reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateMode {
    Apply,
    DryRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// SSOT hash matches the lockfile — nothing to do.
    UpToDate,
    /// `Apply` mode: the lockfile hash changed (or was previously unset
    /// and now resolved). Outputs were rewritten.
    Updated {
        old_hash: Option<String>,
        new_hash: Option<String>,
    },
    /// `DryRun` mode: SSOT hash differs from the lockfile entry; an
    /// `update` (without `--dry-run`) would rewrite outputs.
    WouldChange {
        old_hash: Option<String>,
        new_hash: Option<String>,
    },
    /// `Apply` mode: item no longer exists in the source. Outputs deleted
    /// and lockfile entry removed.
    Removed,
    /// `DryRun` mode: item no longer exists in the source; an `update`
    /// (without `--dry-run`) would remove it.
    WouldRemove,
}

#[derive(Debug, Clone)]
pub struct UpdatedItem {
    pub kind: ItemKind,
    pub name: String,
    pub source: String,
    pub status: UpdateStatus,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateReport {
    pub items: Vec<UpdatedItem>,
}

/// Delete all per-client output files for an item (best-effort).
fn remove_item_outputs(target: &Path, kind: ItemKind, name: &str) {
    for client in ALL_CLIENTS {
        let rel = output_path(kind, client, name);
        let full = target.join(&rel);
        if full.exists() {
            let _ = fs::remove_file(&full);
        }
        // If the item has its own directory (e.g. `.claude/skills/<name>/`),
        // remove it entirely so stale sibling files are cleaned up.
        // Only remove the parent if it's item-specific (contains the name).
        if let Some(parent) = full.parent()
            && parent
                .file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|f| f == name)
            && parent.is_dir()
        {
            let _ = fs::remove_dir_all(parent);
        }
    }
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

/// Hash every item directory under a SSOT root, keyed by `(kind, name)`.
/// Used by `update --dry-run` to compute would-be hashes without
/// installing. Mirrors the discovery of `install_from_local_path`: an
/// item directory `<source_root>/<name>/` contributes one entry per
/// kind for which it holds an entrypoint (so co-located multi-kind
/// items contribute multiple entries, one per kind).
fn hash_source_items(
    source_root: &Path,
) -> std::collections::BTreeMap<(ItemKind, String), Option<String>> {
    let mut out = std::collections::BTreeMap::new();
    let Ok(entries) = fs::read_dir(source_root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let hash = hash_item_dir(&path);
        for (entrypoint, kind) in [
            ("RULE.md", ItemKind::Rule),
            ("SKILL.md", ItemKind::Skill),
            ("AGENT.md", ItemKind::Agent),
        ] {
            if path.join(entrypoint).is_file() {
                out.insert((kind, name.to_string()), hash.clone());
            }
        }
    }
    out
}

/// One entry in a [`ListReport`] — a single installed item as recorded
/// in the lockfile. Mirrors the lockfile shape; no per-client expansion.
#[derive(Debug, Clone, Serialize)]
pub struct ListedItem {
    pub kind: ItemKind,
    pub name: String,
    pub source: String,
    pub git_ref: Option<String>,
}

/// One installed bundle as recorded in the lockfile (the per-bundle
/// breakdown — see [`crate::lockfile::LockedBundle`]).
#[derive(Debug, Clone, Serialize)]
pub struct ListedBundle {
    pub name: String,
    pub source: String,
    pub git_ref: Option<String>,
    pub items: Vec<String>,
}

/// What `upskill list` reports: every item the lockfile records, plus
/// any installed bundles. Items are grouped by kind; the per-kind
/// vectors are sorted by name for deterministic output.
#[derive(Debug, Default, Clone, Serialize)]
pub struct ListReport {
    pub rules: Vec<ListedItem>,
    pub skills: Vec<ListedItem>,
    pub agents: Vec<ListedItem>,
    pub bundles: Vec<ListedBundle>,
}

impl ListReport {
    /// True when the lockfile contains no items and no bundles.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
            && self.skills.is_empty()
            && self.agents.is_empty()
            && self.bundles.is_empty()
    }
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

/// Install items from any supported source into `target`.
///
/// Dispatches on the source variant. All git-backed variants funnel
/// through [`install_from_git_url`]; the only difference is URL
/// construction.
///
/// - `LocalPath` — installs directly from the path on disk.
/// - `Github` — `https://github.com/<owner>/<repo>.git`.
/// - `Gitlab` — `https://<host>/<owner>/<repo>.git`. Self-hosted GitLab
///   works through the `host` field on `GitlabRepo`.
///
/// Authentication: when a token is resolved via [`crate::auth`]
/// (`GITHUB_TOKEN` / `GH_TOKEN` / `gh auth token` for GitHub;
/// `GITLAB_TOKEN` / `GL_TOKEN` / `glab auth token` for GitLab), it is
/// URL-encoded and injected into the clone URL as
/// `https://<user>:<token>@<host>/...`. With no token, the clone falls
/// back to git's own credential helpers (keychain, manager, etc.) so the
/// previous behaviour is unchanged for users who rely on those.
pub fn install_from_source(
    source: &InstallSource,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    match source {
        InstallSource::LocalPath(path) => install_from_local_path(path, target, filter),
        InstallSource::Github(repo) => install_from_github(repo, target, filter),
        InstallSource::Gitlab(repo) => install_from_gitlab(repo, target, filter),
    }
}

fn install_from_github(
    repo: &GithubRepo,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    install_from_git_url(
        &github_authenticated_url(repo)?,
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
        target,
        filter,
    )
}

fn install_from_gitlab(
    repo: &GitlabRepo,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    install_from_git_url(
        &gitlab_authenticated_url(repo)?,
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
        target,
        filter,
    )
}

fn github_clone_url(repo: &GithubRepo) -> String {
    format!("https://github.com/{}/{}.git", repo.owner, repo.name)
}

fn gitlab_clone_url(repo: &GitlabRepo) -> String {
    format!("https://{}/{}/{}.git", repo.host, repo.owner, repo.name)
}

fn github_authenticated_url(repo: &GithubRepo) -> Result<String> {
    Ok(match crate::auth::resolve_github_token().token() {
        Some(token) => inject_basic_auth(&github_clone_url(repo), "x-access-token", token)?,
        None => github_clone_url(repo),
    })
}

fn gitlab_authenticated_url(repo: &GitlabRepo) -> Result<String> {
    Ok(match crate::auth::resolve_gitlab_token().token() {
        Some(token) => inject_basic_auth(&gitlab_clone_url(repo), "oauth2", token)?,
        None => gitlab_clone_url(repo),
    })
}

/// Resolve the SSOT root for a source, fetching when remote.
///
/// Returns `(path, guard)`:
/// - `path` is the on-disk SSOT root that callers walk for `skills/`,
///   `rules/`, `agents/` subdirectories.
/// - `guard` is `Some(TempDir)` for git sources (drop cleans up the
///   clone) and `None` for local-path sources.
///
/// Used by `update` (especially `--dry-run`) where we want the SSOT on
/// disk without committing to an install. For `install_*` the same
/// fetch happens internally inside `install_from_git_url` — we keep
/// that path independent so install can stay one tempdir-scoped pass.
pub fn fetch_ssot(source: &InstallSource) -> Result<(PathBuf, Option<tempfile::TempDir>)> {
    match source {
        InstallSource::LocalPath(p) => Ok((p.clone(), None)),
        InstallSource::Github(repo) => clone_to_tempdir(
            &github_authenticated_url(repo)?,
            repo.git_ref.as_deref(),
            repo.subfolder.as_deref(),
            &repo.owner,
            &repo.name,
        ),
        InstallSource::Gitlab(repo) => clone_to_tempdir(
            &gitlab_authenticated_url(repo)?,
            repo.git_ref.as_deref(),
            repo.subfolder.as_deref(),
            &repo.owner,
            &repo.name,
        ),
    }
}

fn clone_to_tempdir(
    url: &str,
    git_ref: Option<&str>,
    subfolder: Option<&str>,
    owner: &str,
    name: &str,
) -> Result<(PathBuf, Option<tempfile::TempDir>)> {
    let tmp = tempfile::tempdir().context("create temp dir for clone")?;
    fetch::shallow_clone(url, git_ref, "clone", tmp.path(), subfolder)
        .map_err(|e| anyhow!("git clone {}: {}", url, e))?;
    let source = fetch::resolve_subfolder(&tmp.path().join("clone"), subfolder, owner, name)
        .map_err(|e| anyhow!("{}", e))?;
    Ok((source, Some(tmp)))
}

/// Inject HTTP Basic credentials into an `https://` URL so `git clone`
/// can authenticate without depending on a credential helper. The token
/// is percent-encoded against the RFC 3986 unreserved set; the user
/// segment is encoded the same way (over-aggressive but safe — typical
/// values are `oauth2` / `x-access-token`, both unreserved-only).
///
/// Returns an error if `url` does not start with `https://` or if `token`
/// is empty (callers should not invoke with an empty token).
fn inject_basic_auth(url: &str, user: &str, token: &str) -> Result<String> {
    if token.is_empty() {
        anyhow::bail!("refusing to inject empty token into URL");
    }
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| anyhow!("expected https:// URL for token injection, got: {url}"))?;
    Ok(format!(
        "https://{}:{}@{}",
        percent_encode_userinfo(user),
        percent_encode_userinfo(token),
        rest
    ))
}

/// Percent-encode `s` keeping only RFC 3986 unreserved characters
/// (`A-Z`, `a-z`, `0-9`, `-`, `_`, `.`, `~`). Used for the userinfo
/// segment of an HTTPS clone URL — over-aggressive but always safe; the
/// character set covers every realistic token format
/// (`ghp_...`, `glpat-...`, etc.) without escaping.
fn percent_encode_userinfo(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

/// Shallow-clone `url` into a tempdir, resolve `subfolder` inside the clone,
/// and run the local install pipeline against the result. The tempdir is
/// removed on return regardless of outcome (RAII via `tempfile::TempDir`).
///
/// Public so callers can install from arbitrary git URLs (mirrors, local
/// `file://` clones, future GitLab self-hosted) without going through
/// [`InstallSource`]. The high-level [`install_from_source`] is preferred
/// when an `InstallSource` already exists.
pub fn install_from_git_url(
    url: &str,
    git_ref: Option<&str>,
    subfolder: Option<&str>,
    owner: &str,
    name: &str,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    let tmp = tempfile::tempdir().context("create temp dir for clone")?;
    fetch::shallow_clone(url, git_ref, "clone", tmp.path(), subfolder)
        .map_err(|e| anyhow!("git clone {}: {}", url, e))?;
    let source = fetch::resolve_subfolder(&tmp.path().join("clone"), subfolder, owner, name)
        .map_err(|e| anyhow!("{}", e))?;
    install_from_local_path(&source, target, filter)
}

fn install_skills(
    source: &Path,
    target: &Path,
    report: &mut InstallReport,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<()> {
    for (name, dir) in iter_item_dirs(source)? {
        if let Some(items) = filter
            && !items.contains(ItemKind::Skill, &name)
        {
            continue;
        }
        let entry_path = dir.join("SKILL.md");
        if !entry_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&entry_path)
            .with_context(|| format!("read {}", entry_path.display()))?;
        let (skill, body) = frontmatter::parse::<Skill>(&raw)
            .with_context(|| format!("parse {}", entry_path.display()))?;
        let audience = skill.audience.as_deref();
        let source_hash = hash_item_dir(&dir);

        // Clean existing output directories for this specific item before
        // writing new outputs. This removes stale sibling files (e.g. renamed
        // resources) while keeping other items' outputs intact if generation
        // fails later.
        remove_item_outputs(target, ItemKind::Skill, &name);
        for client in ALL_CLIENTS {
            if !targets(client, audience) {
                continue;
            }
            let rendered = generate::render_skill(&skill, body, client)
                .with_context(|| format!("render skill {} for {:?}", name, client))?;
            let rel = skill_output_path(client, &name);
            write_output(target, &rel, &rendered)?;
            report.items.push(InstalledItem {
                kind: ItemKind::Skill,
                name: name.clone(),
                client,
                output_path: rel,
                source_hash: source_hash.clone(),
            });
        }
    }
    Ok(())
}

fn install_rules(
    source: &Path,
    target: &Path,
    report: &mut InstallReport,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<()> {
    for (name, dir) in iter_item_dirs(source)? {
        if let Some(items) = filter
            && !items.contains(ItemKind::Rule, &name)
        {
            continue;
        }
        let entry_path = dir.join("RULE.md");
        if !entry_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&entry_path)
            .with_context(|| format!("read {}", entry_path.display()))?;
        let (rule, body) = frontmatter::parse::<Rule>(&raw)
            .with_context(|| format!("parse {}", entry_path.display()))?;
        let audience = rule.audience.as_deref();
        let source_hash = hash_item_dir(&dir);

        // Clean existing output directories for this specific item before
        // writing new outputs. This removes stale sibling files while keeping
        // other items' outputs intact if generation fails later.
        remove_item_outputs(target, ItemKind::Rule, &name);
        for client in ALL_CLIENTS {
            if !targets(client, audience) {
                continue;
            }
            let rendered = generate::render_rule(&rule, body, client)
                .with_context(|| format!("render rule {} for {:?}", name, client))?;
            let rel = rule_output_path(client, &name);
            write_output(target, &rel, &rendered)?;
            report.items.push(InstalledItem {
                kind: ItemKind::Rule,
                name: name.clone(),
                client,
                output_path: rel,
                source_hash: source_hash.clone(),
            });
        }
    }
    Ok(())
}

fn install_agents(
    source: &Path,
    target: &Path,
    report: &mut InstallReport,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<()> {
    for (name, dir) in iter_item_dirs(source)? {
        if let Some(items) = filter
            && !items.contains(ItemKind::Agent, &name)
        {
            continue;
        }
        let entry_path = dir.join("AGENT.md");
        if !entry_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&entry_path)
            .with_context(|| format!("read {}", entry_path.display()))?;
        let (agent, body) = frontmatter::parse::<Agent>(&raw)
            .with_context(|| format!("parse {}", entry_path.display()))?;
        let audience = agent.audience.as_deref();
        let source_hash = hash_item_dir(&dir);

        // Clean existing output directories for this specific item before
        // writing new outputs. This removes stale sibling files while keeping
        // other items' outputs intact if generation fails later.
        remove_item_outputs(target, ItemKind::Agent, &name);
        for client in ALL_CLIENTS {
            if !targets(client, audience) {
                continue;
            }
            let rendered = generate::render_agent(&agent, body, client)
                .with_context(|| format!("render agent {} for {:?}", name, client))?;
            let rel = agent_output_path(client, &name);
            write_output(target, &rel, &rendered)?;
            report.items.push(InstalledItem {
                kind: ItemKind::Agent,
                name: name.clone(),
                client,
                output_path: rel,
                source_hash: source_hash.clone(),
            });
        }
        // Clean up outputs for clients no longer targeted.
        for client in ALL_CLIENTS {
            if targets(client, audience) {
                continue;
            }
            let rel = agent_output_path(client, &name);
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

/// Where the install pipeline writes — and `remove` deletes — the per-
/// client output for a given `(kind, name)`. Path is relative to the
/// install target root and matches format-spec §7. Used by both
/// `install_*` and [`remove`] so the two stay in lockstep.
pub(crate) fn output_path(kind: ItemKind, client: Client, name: &str) -> PathBuf {
    match kind {
        ItemKind::Skill => skill_output_path(client, name),
        ItemKind::Rule => rule_output_path(client, name),
        ItemKind::Agent => agent_output_path(client, name),
    }
}

/// Per format-spec §7 / ADR-0003. opencode `.agents/skills/<n>/SKILL.md`
/// is the canonical-store path; opencode walks it natively.
fn skill_output_path(client: Client, name: &str) -> PathBuf {
    match client {
        Client::Claude => PathBuf::from(format!(".claude/skills/{name}/SKILL.md")),
        Client::Copilot => PathBuf::from(format!(".github/skills/{name}/SKILL.md")),
        Client::OpenCode => PathBuf::from(format!(".agents/skills/{name}/SKILL.md")),
    }
}

/// Per format-spec §7 / ADR-0003. Copilot uses
/// `<name>.instructions.md`; opencode uses a per-rule directory under
/// `.agents/rules/`.
fn rule_output_path(client: Client, name: &str) -> PathBuf {
    match client {
        Client::Claude => PathBuf::from(format!(".claude/rules/{name}.md")),
        Client::Copilot => PathBuf::from(format!(".github/instructions/{name}.instructions.md")),
        Client::OpenCode => PathBuf::from(format!(".agents/rules/{name}/RULE.md")),
    }
}

/// Per format-spec §7 / ADR-0003. Copilot uses `<name>.agent.md`.
fn agent_output_path(client: Client, name: &str) -> PathBuf {
    match client {
        Client::Claude => PathBuf::from(format!(".claude/agents/{name}.md")),
        Client::Copilot => PathBuf::from(format!(".github/agents/{name}.agent.md")),
        Client::OpenCode => PathBuf::from(format!(".opencode/agents/{name}.md")),
    }
}

fn write_output(target: &Path, rel: &Path, content: &str) -> Result<()> {
    let full = target.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create dir {}", parent.display()))?;
    }
    fs::write(&full, content).with_context(|| format!("write {}", full.display()))?;
    Ok(())
}

/// SHA-256 hash of every file under `dir`, with each file's path-relative
/// name folded into the hash so renames register as drift. Recursive,
/// deterministic (sorted file list), and `None` when `dir` is empty or
/// unreadable. Used by the pipeline to populate `LockedItem.hash` and by
/// `doctor` (Phase B3) to detect SSOT drift.
pub(crate) fn hash_item_dir(dir: &Path) -> Option<String> {
    let mut files = Vec::new();
    collect_files(dir, &mut files);
    if files.is_empty() {
        return None;
    }
    files.sort();
    let mut hasher = Sha256::new();
    for file in &files {
        let relative = file.strip_prefix(dir).unwrap_or(file);
        hasher.update(relative.to_string_lossy().as_bytes());
        if let Ok(content) = fs::read(file) {
            hasher.update(&content);
        }
    }
    Some(
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

/// Iterate `(name, dir)` for every immediate subdirectory of `kind_root`.
/// Returns an empty iterator when the kind root does not exist (treating
/// "no items of this kind" as a non-error).
fn iter_item_dirs(kind_root: &Path) -> Result<Vec<(String, PathBuf)>> {
    if !kind_root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in
        fs::read_dir(kind_root).with_context(|| format!("read_dir {}", kind_root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if is_item_dir(&path) {
            // Direct item: kind_root/<item>/ENTRY.md
            let name = entry
                .file_name()
                .to_str()
                .map(str::to_owned)
                .with_context(|| format!("non-UTF8 name in {}", kind_root.display()))?;
            out.push((name, path));
        } else {
            // Category subdir: kind_root/<category>/<item>/ENTRY.md
            // Descend one level to find items in subdirectories (format-spec §2.2).
            if let Ok(sub_entries) = fs::read_dir(&path) {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    if sub_path.is_dir() && is_item_dir(&sub_path) {
                        let name = sub_entry
                            .file_name()
                            .to_str()
                            .map(str::to_owned)
                            .with_context(|| format!("non-UTF8 name in {}", path.display()))?;
                        out.push((name, sub_path));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
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
    fn output_paths_match_spec() {
        assert_eq!(
            skill_output_path(Client::Claude, "x"),
            PathBuf::from(".claude/skills/x/SKILL.md")
        );
        assert_eq!(
            skill_output_path(Client::OpenCode, "x"),
            PathBuf::from(".agents/skills/x/SKILL.md")
        );
        assert_eq!(
            rule_output_path(Client::Copilot, "x"),
            PathBuf::from(".github/instructions/x.instructions.md")
        );
        assert_eq!(
            rule_output_path(Client::OpenCode, "x"),
            PathBuf::from(".agents/rules/x/RULE.md")
        );
        assert_eq!(
            agent_output_path(Client::Copilot, "x"),
            PathBuf::from(".github/agents/x.agent.md")
        );
        assert_eq!(
            agent_output_path(Client::OpenCode, "x"),
            PathBuf::from(".opencode/agents/x.md")
        );
    }

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
    fn github_clone_url_is_https_dot_git() {
        let repo = GithubRepo {
            owner: "driftsys".into(),
            name: "skills".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            github_clone_url(&repo),
            "https://github.com/driftsys/skills.git"
        );
    }

    #[test]
    fn gitlab_clone_url_uses_repo_host() {
        // gitlab.com
        let repo = GitlabRepo {
            host: "gitlab.com".into(),
            owner: "driftsys".into(),
            name: "skills".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            gitlab_clone_url(&repo),
            "https://gitlab.com/driftsys/skills.git"
        );

        // self-hosted GitLab
        let self_hosted = GitlabRepo {
            host: "gitlab.example.com".into(),
            owner: "team".into(),
            name: "rules".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            gitlab_clone_url(&self_hosted),
            "https://gitlab.example.com/team/rules.git"
        );
    }

    #[test]
    fn inject_basic_auth_github_oauth_user() {
        // Mirrors the call install_from_github makes when GITHUB_TOKEN is set.
        let url = inject_basic_auth(
            "https://github.com/driftsys/skills.git",
            "x-access-token",
            "ghp_AbCdEf1234567890",
        )
        .expect("inject");
        assert_eq!(
            url,
            "https://x-access-token:ghp_AbCdEf1234567890@github.com/driftsys/skills.git"
        );
    }

    #[test]
    fn inject_basic_auth_gitlab_oauth_user() {
        // Mirrors the call install_from_gitlab makes when GITLAB_TOKEN is set,
        // and exercises self-hosted GitLab (per the plan: gitlab.example.com).
        let url = inject_basic_auth(
            "https://gitlab.example.com/team/rules.git",
            "oauth2",
            "glpat-XYZ_abc-123",
        )
        .expect("inject");
        assert_eq!(
            url,
            "https://oauth2:glpat-XYZ_abc-123@gitlab.example.com/team/rules.git"
        );
    }

    #[test]
    fn inject_basic_auth_percent_encodes_special_chars() {
        // Tokens containing `:`, `@`, `/`, `%` would otherwise corrupt the URL
        // parse on the git side. Verify they're percent-encoded.
        let url = inject_basic_auth(
            "https://gitlab.com/o/r.git",
            "oauth2",
            "tok:en@with/special%chars",
        )
        .expect("inject");
        assert_eq!(
            url,
            "https://oauth2:tok%3Aen%40with%2Fspecial%25chars@gitlab.com/o/r.git"
        );
    }

    #[test]
    fn inject_basic_auth_rejects_empty_token() {
        let err = inject_basic_auth("https://github.com/o/r.git", "x-access-token", "")
            .expect_err("must reject");
        assert!(err.to_string().contains("empty token"));
    }

    #[test]
    fn inject_basic_auth_rejects_non_https() {
        let err = inject_basic_auth("http://github.com/o/r.git", "x-access-token", "tok")
            .expect_err("must reject");
        assert!(err.to_string().contains("https://"));

        let err = inject_basic_auth("git@github.com:o/r.git", "x-access-token", "tok")
            .expect_err("must reject ssh form");
        assert!(err.to_string().contains("https://"));
    }

    #[test]
    fn percent_encode_userinfo_passes_unreserved_unchanged() {
        // RFC 3986 unreserved set: A-Z a-z 0-9 - _ . ~
        assert_eq!(
            percent_encode_userinfo("Abc-_.~123"),
            "Abc-_.~123",
            "unreserved chars unchanged"
        );
    }

    #[test]
    fn percent_encode_userinfo_escapes_userinfo_separators() {
        // The chars that would actually break a URL parse if unescaped.
        assert_eq!(percent_encode_userinfo(":"), "%3A");
        assert_eq!(percent_encode_userinfo("@"), "%40");
        assert_eq!(percent_encode_userinfo("/"), "%2F");
        assert_eq!(percent_encode_userinfo("%"), "%25");
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

    #[test]
    fn output_path_dispatches_to_per_kind_helper() {
        // The dispatcher must produce the same path as the per-kind
        // function for the same `(kind, client, name)` tuple, otherwise
        // `install` would write to one place and `remove` would look in
        // another.
        for client in ALL_CLIENTS {
            assert_eq!(
                output_path(ItemKind::Skill, client, "x"),
                skill_output_path(client, "x")
            );
            assert_eq!(
                output_path(ItemKind::Rule, client, "x"),
                rule_output_path(client, "x")
            );
            assert_eq!(
                output_path(ItemKind::Agent, client, "x"),
                agent_output_path(client, "x")
            );
        }
    }

    #[test]
    fn find_bundle_by_name_finds_nested_bundle_file() {
        let tmp = tempfile::tempdir().unwrap();
        let bundles_dir = tmp.path().join("bundles");
        std::fs::create_dir_all(&bundles_dir).unwrap();
        std::fs::write(
            bundles_dir.join("baseline.bundle.yaml"),
            "schema: 1\nname: baseline\ndescription: test\nitems:\n  rules: []\n",
        )
        .unwrap();

        let result = find_bundle_by_name(tmp.path(), "baseline");
        assert_eq!(result, Some(bundles_dir.join("baseline.bundle.yaml")));
    }

    #[test]
    fn find_bundle_by_name_returns_none_when_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("skills/foo")).unwrap();
        std::fs::write(
            tmp.path().join("skills/foo/SKILL.md"),
            "---\nschema: 1\nname: foo\n---\n# body\n",
        )
        .unwrap();

        let result = find_bundle_by_name(tmp.path(), "foo");
        assert!(result.is_none());
    }

    #[test]
    fn find_bundle_by_name_skips_hidden_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let hidden = tmp.path().join(".hidden");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(
            hidden.join("secret.bundle.yaml"),
            "schema: 1\nname: secret\ndescription: x\nitems:\n  rules: []\n",
        )
        .unwrap();

        let result = find_bundle_by_name(tmp.path(), "secret");
        assert!(result.is_none());
    }

    #[test]
    fn has_matching_items_true_when_skill_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("code-review");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nschema: 1\nname: code-review\n---\n# body\n",
        )
        .unwrap();

        assert!(has_matching_items(tmp.path(), "code-review"));
    }

    #[test]
    fn has_matching_items_false_when_no_item_exists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("other")).unwrap();

        assert!(!has_matching_items(tmp.path(), "nonexistent"));
    }

    #[test]
    fn has_matching_items_true_for_rules_and_agents() {
        let tmp = tempfile::tempdir().unwrap();
        let rule_dir = tmp.path().join("my-rule");
        std::fs::create_dir_all(&rule_dir).unwrap();
        std::fs::write(
            rule_dir.join("RULE.md"),
            "---\nschema: 1\nname: my-rule\n---\n# body\n",
        )
        .unwrap();

        assert!(has_matching_items(tmp.path(), "my-rule"));
    }

    #[test]
    fn has_ssot_layout_detects_direct_children() {
        // Flat layout: root/<item>/RULE.md
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("my-rule");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("RULE.md"), "").unwrap();

        assert!(has_ssot_layout(tmp.path()));
    }

    #[test]
    fn has_ssot_layout_detects_grandchild_entrypoints() {
        // Sibling layout: root/skills/<item>/RULE.md
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("skills/my-rule");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("RULE.md"), "").unwrap();

        assert!(
            has_ssot_layout(tmp.path()),
            "has_ssot_layout must detect items nested one level deeper"
        );
    }

    #[test]
    fn has_ssot_layout_returns_false_for_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!has_ssot_layout(tmp.path()));
    }

    #[test]
    fn find_registry_root_returns_parent_for_sibling_layout() {
        // registry/bundles/x.bundle.yaml + registry/skills/<item>/RULE.md
        // → find_registry_root must return registry/
        let tmp = tempfile::tempdir().unwrap();
        let registry = tmp.path().join("registry");
        std::fs::create_dir_all(registry.join("bundles")).unwrap();
        std::fs::create_dir_all(registry.join("skills/my-rule")).unwrap();
        std::fs::write(registry.join("skills/my-rule/RULE.md"), "").unwrap();
        let bundle = registry.join("bundles/test.bundle.yaml");
        std::fs::write(&bundle, "").unwrap();

        let root = find_registry_root(&bundle).unwrap();
        assert_eq!(root, registry);
    }

    #[test]
    fn iter_item_dirs_finds_items_in_category_subdirs() {
        // registry/skills/<item>/SKILL.md should be discovered
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("skills/my-skill");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("SKILL.md"), "").unwrap();

        let dirs = iter_item_dirs(tmp.path()).unwrap();
        let names: Vec<&str> = dirs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.contains(&"my-skill"),
            "iter_item_dirs must find items in category subdirectories: {names:?}"
        );
    }

    #[test]
    fn iter_item_dirs_still_finds_direct_children() {
        // Flat layout: root/<item>/RULE.md must still work
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("my-rule");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("RULE.md"), "").unwrap();

        let dirs = iter_item_dirs(tmp.path()).unwrap();
        let names: Vec<&str> = dirs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.contains(&"my-rule"),
            "iter_item_dirs must still find direct item children: {names:?}"
        );
    }
}
