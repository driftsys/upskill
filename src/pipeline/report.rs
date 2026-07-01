//! Report types returned by the pipeline's top-level operations.
//!
//! Every public command (`install`, `remove`, `update`, `doctor`, `list`)
//! returns a structured report rather than printing — presentation lives
//! in `main.rs`. These types are also exposed via `pub use` from the
//! parent module so consumers `use upskill::pipeline::InstallReport;`
//! works unchanged.

use serde::Serialize;
use std::path::PathBuf;

use super::ItemKind;
use crate::generate::Client;

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
    /// Source folder this item was discovered in — the co-location
    /// grouping key (§2.1). Threaded into the lockfile so `remove <name>`
    /// can act on the whole `(source, group)` unit. This is the folder
    /// LEAF name (not a source-root-relative path), so two items with the
    /// same leaf folder name under different category directories within
    /// one source would share a group.
    pub group: Option<String>,
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
    /// Results of MCP server configuration (ADR-0010). Empty when no
    /// bundles with `mcps:` were resolved.
    pub mcp_results: Vec<McpResult>,
    /// Names of items warn-skipped because a restrictive consumer selection
    /// (ADR-0012) left no client in common with the item's author `audience:`.
    /// Surfaced to the user by `main.rs`; never an error.
    pub selection_skipped: Vec<String>,
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

/// Result of a single MCP server configuration attempt, for reporting.
#[derive(Debug, Clone)]
pub struct McpResult {
    /// Upskill-level MCP name.
    pub name: String,
    /// Client identifier.
    pub client: String,
    /// Configuration outcome.
    pub outcome: crate::plugin::PluginOutcome,
    /// Bundle that declared this MCP server.
    pub bundle: String,
    /// Env vars declared as required (for the doctor/warn surface).
    pub requires_env: Vec<String>,
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

/// Reconciliation outcome for a single MCP server during `doctor`.
#[derive(Debug, Clone, Serialize)]
pub struct McpDoctorEntry {
    pub name: String,
    pub client: String,
    pub bundle: String,
    pub status: McpDoctorStatus,
}

/// The specific condition detected during doctor MCP reconciliation.
///
/// Only `NotRegistered` is drift that fails [`DoctorReport::is_clean`]. The
/// other non-`Ok` variants are informational "cannot verify" states — the
/// server may well be configured, upskill just cannot confirm it — so they are
/// surfaced without flipping the exit code (issue #241).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpDoctorStatus {
    /// Registered in the client — no action needed.
    Ok,
    /// In the lockfile but absent from the client's registered list.
    NotRegistered,
    /// Client CLI was not found; cannot verify.
    CliNotFound,
    /// CLI returned non-zero when listing MCP servers.
    QueryFailed { stderr: String },
    /// Config-write target: the config file does not exist, so state is
    /// undetermined (the server may be configured via the client's own CLI).
    ConfigMissing,
    /// Config-write target: the config file exists but could not be read or
    /// parsed as JSON.
    ConfigUnreadable { detail: String },
}

/// A dependency-pulled item whose every requirer is no longer installed.
/// Advisory only — upskill never auto-removes (#196); the user decides.
#[derive(Debug, Clone, Serialize)]
pub struct OrphanedDependency {
    pub kind: ItemKind,
    pub name: String,
    /// The now-absent requirers (`"kind:name"`) recorded at install time.
    pub former_requirers: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct DoctorReport {
    pub missing_outputs: Vec<MissingOutput>,
    pub stale_hashes: Vec<StaleHash>,
    pub orphan_entries: Vec<OrphanEntry>,
    /// Items present only as a dependency of an item that is no longer
    /// installed. Advisory — does NOT affect `is_clean()`.
    pub orphaned_dependencies: Vec<OrphanedDependency>,
    /// Plugins recorded in the lockfile as `installed` but absent from the
    /// client's installed list (uninstalled out-of-band).  Non-empty →
    /// `is_clean()` returns false → exit 1.
    pub missing_plugins: Vec<MissingPlugin>,
    /// Plugins recorded in the lockfile as `skipped` (warn-skip at install
    /// time). Informational only — does NOT cause `is_clean()` to return
    /// false or trigger exit 1. Run `upskill update` after installing the
    /// missing CLI to install them.
    pub skipped_plugins: Vec<SkippedPlugin>,
    /// MCP server reconciliation results. Entries are present for every MCP
    /// recorded in the lockfile. `McpDoctorStatus::Ok` entries are included
    /// so callers can distinguish "checked and fine" from "not checked".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_entries: Vec<McpDoctorEntry>,
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
            && !self
                .mcp_entries
                .iter()
                .any(|e| matches!(e.status, McpDoctorStatus::NotRegistered))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn doctor_report_not_clean_with_not_registered_mcp() {
        let mut report = DoctorReport::default();
        report.mcp_entries.push(McpDoctorEntry {
            name: "my-mcp".into(),
            client: "claude".into(),
            bundle: "my-bundle".into(),
            status: McpDoctorStatus::NotRegistered,
        });
        assert!(!report.is_clean());
    }

    #[test]
    fn doctor_report_clean_with_mcp_cli_not_found_or_query_failed() {
        let mut report = DoctorReport::default();
        report.mcp_entries.push(McpDoctorEntry {
            name: "my-mcp".into(),
            client: "claude".into(),
            bundle: "my-bundle".into(),
            status: McpDoctorStatus::CliNotFound,
        });
        assert!(report.is_clean());

        let mut report = DoctorReport::default();
        report.mcp_entries.push(McpDoctorEntry {
            name: "my-mcp".into(),
            client: "claude".into(),
            bundle: "my-bundle".into(),
            status: McpDoctorStatus::QueryFailed {
                stderr: "some error".into(),
            },
        });
        assert!(report.is_clean());
    }

    #[test]
    fn doctor_report_clean_with_config_missing_or_unreadable_mcp() {
        // Config-write targets whose config file is absent or unreadable are
        // informational "cannot verify" states — they must NOT flip is_clean
        // (issue #241, no false-positive exit 1).
        let mut report = DoctorReport::default();
        report.mcp_entries.push(McpDoctorEntry {
            name: "my-mcp".into(),
            client: "vscode".into(),
            bundle: "my-bundle".into(),
            status: McpDoctorStatus::ConfigMissing,
        });
        report.mcp_entries.push(McpDoctorEntry {
            name: "my-mcp".into(),
            client: "opencode".into(),
            bundle: "my-bundle".into(),
            status: McpDoctorStatus::ConfigUnreadable {
                detail: "expected value at line 1".into(),
            },
        });
        assert!(report.is_clean());
    }
}
