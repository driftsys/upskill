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
    /// One entry per item and client in the plan. It also includes the
    /// items listed in `conflicts`, which were not written; `update` relies
    /// on this so that a skipped item is not removed as no longer in the
    /// source.
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
    /// Every output file of every item, computed before anything is
    /// written. `PreparedInstall::commit` writes it with one call.
    pub(crate) planned: Vec<super::plan::PlannedItem>,
    /// Paths recorded when this install adopted a `schema: 1` lockfile;
    /// `None` when no adoption happened. `main.rs` prints the adoption
    /// notice when it is `Some` and not empty.
    pub adopted: Option<Vec<String>>,
    /// Destinations that stopped the write of an item. Every item listed
    /// here was not written, and its lockfile entry is unchanged.
    pub conflicts: Vec<super::FileConflict>,
    /// With `--force`: modified or unmanaged files that were replaced, and
    /// modified files that the new version no longer contains and that were
    /// deleted.
    pub forced: Vec<PathBuf>,
    /// Every file that was written with new content (it was absent or held
    /// other bytes), that got a different planned mode, or that was deleted,
    /// with or without `--force`.
    pub changed: Vec<PathBuf>,
    /// Modified files that the new version of an item no longer contains.
    /// They are kept on disk and stay in the item's record. With
    /// [`super::AddOptions::keep_unwithheld_edits`], also the modified
    /// destinations of the items that were not written because nothing is
    /// withheld from them; their lockfile entries are unchanged.
    pub kept_modified: Vec<PathBuf>,
}

/// What a commit would do to one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewAction {
    Update,
    Complete,
    Remove,
    Skip,
}

/// One item of a preview that is not up to date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewItem {
    pub kind: ItemKind,
    pub name: String,
    pub action: PreviewAction,
    /// Files that would be written with new content, or that would get a
    /// different planned mode.
    pub write: Vec<PathBuf>,
    /// Files that would be deleted.
    pub delete: Vec<PathBuf>,
    /// Modified files that are kept (stale, or of a removed item).
    pub kept_modified: Vec<PathBuf>,
    /// With `--force`: modified or unmanaged files that would be lost.
    pub forced: Vec<PathBuf>,
    /// For `Skip`: the files that cause the skip.
    pub blocking: Vec<PathBuf>,
    /// True when only the lockfile record of the item changes.
    pub record_only: bool,
}

/// Error context attached when a command fails after it saved the
/// adoption of a `schema: 1` lockfile. `paths` are the adopted paths, for
/// the adoption notice that `main.rs` prints before the error.
#[derive(Debug, Clone)]
pub struct AdoptedBeforeFailure {
    pub paths: Vec<String>,
}

impl std::fmt::Display for AdoptedBeforeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the lockfile was adopted before this error")
    }
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
    /// string verbatim. When the lockfile contains no entry from the named
    /// source, nothing is deleted; a `schema: 1` lockfile is still adopted
    /// and saved.
    BySource(String),
}

#[derive(Debug, Default, Clone)]
pub struct RemoveReport {
    pub items: Vec<RemovedItem>,
    /// Paths recorded when this command adopted a `schema: 1` lockfile;
    /// `None` when no adoption happened.
    pub adopted: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct RemovedItem {
    pub kind: ItemKind,
    pub name: String,
    /// Recorded files actually deleted from disk (paths relative to
    /// `target`), sorted. Empty when the record is empty or every recorded
    /// file was already gone or kept.
    pub deleted_files: Vec<PathBuf>,
    /// Recorded files kept because they were modified after upskill wrote
    /// them (paths relative to `target`), sorted. With `--force` this holds
    /// only recorded paths that are not regular files.
    pub kept_modified: Vec<PathBuf>,
}

/// One per-client output file the lockfile said should exist but doesn't.
#[derive(Debug, Clone, Serialize)]
pub struct MissingOutput {
    pub kind: ItemKind,
    pub name: String,
    /// Paths relative to the install target.
    pub missing_files: Vec<PathBuf>,
}

/// Recorded files of one item whose content does not match the recorded
/// hash, as it is or with every CRLF replaced by LF, or that are no longer
/// regular files.
#[derive(Debug, Clone, Serialize)]
pub struct ModifiedFile {
    pub kind: ItemKind,
    pub name: String,
    /// Paths relative to the install target.
    pub files: Vec<PathBuf>,
}

/// Recorded files of one item that could not be inspected (for example
/// because of a permission error).
#[derive(Debug, Clone, Serialize)]
pub struct UnreadableFile {
    pub kind: ItemKind,
    pub name: String,
    /// Paths relative to the install target.
    pub files: Vec<PathBuf>,
    /// The error text for `files[0]`, the first path in sorted order. The
    /// errors of the other paths are not kept.
    pub error: String,
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

/// An item with a non-empty `pending` record: an `add` or `update` failed
/// or was stopped between its first and its last write. Running the same
/// command again completes it.
#[derive(Debug, Clone, Serialize)]
pub struct InterruptedItem {
    pub kind: ItemKind,
    pub name: String,
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
    /// Recorded files modified after upskill wrote them. Non-empty →
    /// `is_clean()` returns false.
    pub modified_files: Vec<ModifiedFile>,
    /// Recorded files that could not be inspected. Non-empty →
    /// `is_clean()` returns false.
    pub unreadable_files: Vec<UnreadableFile>,
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
    /// false or trigger exit 1. Run `upskill update --yes` after installing the
    /// missing CLI to install them.
    pub skipped_plugins: Vec<SkippedPlugin>,
    /// MCP server reconciliation results. Entries are present for every MCP
    /// recorded in the lockfile. `McpDoctorStatus::Ok` entries are included
    /// so callers can distinguish "checked and fine" from "not checked".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_entries: Vec<McpDoctorEntry>,
    /// Items whose last `add` or `update` did not complete. Non-empty →
    /// `is_clean()` returns false.
    pub interrupted: Vec<InterruptedItem>,
}

impl DoctorReport {
    /// True when nothing is wrong — every per-client output is on disk,
    /// every recorded file still has its recorded content (as it is, or with
    /// every CRLF replaced by LF) and can be read,
    /// every locally-sourced item still hashes the same, every lockfile
    /// entry has a recoverable source, no installed plugin is missing
    /// from its client, and no item has an interrupted operation.
    ///
    /// `skipped_plugins` (warn-skip outcomes) are informational: the user
    /// never had the CLI at install time, so this is the expected state.
    /// They are reported but do not affect cleanness.
    pub fn is_clean(&self) -> bool {
        self.missing_outputs.is_empty()
            && self.modified_files.is_empty()
            && self.unreadable_files.is_empty()
            && self.stale_hashes.is_empty()
            && self.orphan_entries.is_empty()
            && self.missing_plugins.is_empty()
            && self.interrupted.is_empty()
            && !self
                .mcp_entries
                .iter()
                .any(|e| matches!(e.status, McpDoctorStatus::NotRegistered))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// The source of the item was applied, and the source hash of the item
    /// equals the hash recorded in the lockfile entry; the item has no
    /// `pending` record. The status comes from this hash comparison only:
    /// an item reported as up to date can have had files written or
    /// deleted (for example a missing output written again, or the files
    /// of a client that the selection no longer targets deleted) or its
    /// lockfile record changed (for example a historical source label
    /// rewritten). Also the status of an item of a source that was not
    /// applied and that has no preview item.
    UpToDate,
    /// The source of the item was applied, and the source hash of the item
    /// differs from the hash recorded in the lockfile entry (or no hash was
    /// recorded).
    Updated {
        old_hash: Option<String>,
        new_hash: Option<String>,
    },
    /// The item had an interrupted operation (a non-empty `pending`
    /// record) and its source hash did not change. The operation was
    /// completed: its planned files were written and its `pending` record
    /// was cleared.
    CompletedInterrupted,
    /// The item no longer exists in the source. Its unchanged recorded
    /// files were deleted (modified ones are kept and listed in
    /// [`UpdateReport::kept_modified`], unless `--force`), and its lockfile
    /// entry was removed.
    Removed,
    /// The item was not written because these destinations (relative to
    /// the install target root) are modified or not managed by upskill.
    /// The files and the lockfile entry are unchanged. Reported for an
    /// applied source, and for a source that was not applied because it
    /// has no changes (see [`SourcePreview::has_changes`]).
    SkippedModified { files: Vec<PathBuf> },
    /// The source of the item has changes, and the source was not applied:
    /// the observer did not return [`SourceDecision::Apply`], or the run is
    /// a dry run. `action` is what applying the source would do to the
    /// item; a [`PreviewAction::Skip`] item of such a source is reported
    /// here and does not cause exit code 1.
    NotApplied { action: PreviewAction },
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
    /// Paths recorded when `update` adopted a `schema: 1` lockfile, sorted
    /// and without duplicates; `None` when no adoption happened.
    pub adopted: Option<Vec<String>>,
    /// Modified files that `update` kept: files that the new version of an
    /// item no longer contains, files of an item that its source no longer
    /// contains, and modified destinations of an item whose source did not
    /// change and from which nothing is withheld (the item is reported as
    /// [`UpdateStatus::UpToDate`]).
    pub kept_modified: Vec<PathBuf>,
    /// With `--force`: modified or unmanaged files that were replaced or
    /// deleted, including those of items that the source no longer contains.
    pub forced: Vec<PathBuf>,
    /// Every file that was written with new content (it was absent or held
    /// other bytes), that got a different planned mode, or that was deleted,
    /// with or without `--force`.
    pub changed: Vec<PathBuf>,
    /// Sources that were previewed.
    pub sources_considered: usize,
    /// Sources that were applied.
    pub sources_applied: usize,
    /// Sources with changes that were not applied (answer no, quit, dry run).
    pub sources_not_applied: usize,
    /// True when the observer returned [`SourceDecision::Stop`].
    pub stopped: bool,
    /// Sources of the run that were not previewed because the observer
    /// returned [`SourceDecision::Stop`] for an earlier source; 0 when the
    /// run did not stop.
    pub sources_remaining: usize,
}

/// What applying one source of `update` would do now, computed after the
/// fetch of the source and before any write of it.
#[derive(Debug, Clone)]
pub struct SourcePreview {
    /// The source label, as recorded in the lockfile.
    pub source: String,
    /// Items that are not up to date. An up-to-date item has no entry.
    pub items: Vec<PreviewItem>,
    /// `kept modified file` notes, not changes: the modified destinations of
    /// items kept by the rule for edited files, and the kept modified stale
    /// files of items that have no other change.
    pub kept_notes: Vec<PathBuf>,
    /// Hand-shake files that applying the source would create or change.
    pub handshake: Vec<PathBuf>,
    /// Paths adopted from a `schema: 1` lockfile just before this preview.
    pub adopted: Option<Vec<String>>,
}

impl SourcePreview {
    /// True exactly when a commit would write or delete a file or change an
    /// item record: an item has a non-empty `write`, `delete` or `forced`,
    /// or its action is `Complete` or `Remove`, or it is `record_only`; or a
    /// hand-shake file would be created or changed. A `Skip` item and a
    /// note in `kept_notes` are not changes.
    pub fn has_changes(&self) -> bool {
        self.items.iter().any(|item| {
            !item.write.is_empty()
                || !item.delete.is_empty()
                || !item.forced.is_empty()
                || matches!(item.action, PreviewAction::Complete | PreviewAction::Remove)
                || item.record_only
        }) || !self.handshake.is_empty()
    }
}

/// What `update` does with one previewed source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceDecision {
    /// Apply the source, then continue with the next source.
    Apply,
    /// Do not apply the source; continue with the next source.
    Skip,
    /// Do not apply the source and stop the run.
    Stop,
}

/// What the apply of one source did, counted per item status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceResult {
    pub source: String,
    pub updated: usize,
    pub completed: usize,
    pub removed: usize,
    /// Modified files that were kept, counted for this source before the
    /// report removes the duplicates across sources: the counts of all
    /// sources can add up to more than [`UpdateReport::kept_modified`]
    /// holds.
    pub kept_modified: usize,
    pub skipped: usize,
    /// Files that the commit and the removals of this source wrote with new
    /// content or deleted: the number of paths that this source added to
    /// [`UpdateReport::changed`].
    pub changed_files: usize,
}

/// The caller of `update`: it receives each source preview and decides,
/// and it receives the result of each applied source. `update` never reads
/// input and never prints; the observer does.
pub trait UpdateObserver {
    /// Called once per source, after its preview and before any write of
    /// it. With a dry run, the decision is ignored.
    fn preview(&mut self, preview: &SourcePreview) -> SourceDecision;
    /// Called after a source was applied.
    fn applied(&mut self, result: &SourceResult);
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
        report.modified_files.push(ModifiedFile {
            kind: ItemKind::Skill,
            name: "x".into(),
            files: vec![PathBuf::from("a")],
        });
        assert!(!report.is_clean());

        let mut report = DoctorReport::default();
        report.unreadable_files.push(UnreadableFile {
            kind: ItemKind::Skill,
            name: "x".into(),
            files: vec![PathBuf::from("a")],
            error: "denied".into(),
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
