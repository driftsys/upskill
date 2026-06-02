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

use crate::generate::Client;
use crate::source::InstallSource;

mod discovery;
mod git;
mod hash;
mod install;
mod lifecycle;
mod output;
mod report;

use discovery::scan_source_items;
pub use git::{fetch_ssot, install_from_git_url, install_from_source};
pub(crate) use hash::hash_item_dir;
pub use install::install_from_local_path;
use install::{install_plugins_from_bundles, install_with_name_resolution_from_local};
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

    // -- Guard: aliasing an item that ships supporting resources is not yet
    // supported (the resource namespace dir and rewritten `<name>/` link
    // prefix would not be relocated to the alias). Tracked as debt; abort
    // before writing rather than emit broken output.
    // Skip for bundle-file sources: `local_source` is then a file, not a
    // directory, so `iter_item_dirs` would error. Bundle-sourced aliasing of
    // resource-bearing items is covered by the same debt follow-up (#200).
    if !options.aliases.is_empty() && !discovery::is_bundle_file(&local_source) {
        for (name, dir) in discovery::iter_item_dirs(&local_source)? {
            let aliased = options
                .aliases
                .iter()
                .any(|(from, _)| from.is_empty() || *from == name);
            if aliased && !discovery::iter_item_resources(&dir).is_empty() {
                anyhow::bail!(
                    "aliasing items with supporting resources is not yet supported \
                     ('{name}' ships resource files). Install it without --as. \
                     (See format-spec §2.4; tracked in #200.)"
                );
            }
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
