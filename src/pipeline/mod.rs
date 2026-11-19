//! v0.2 install pipeline: SSOT (local path or git source) → per-client
//! output on disk.
//!
//! Walks a source directory laid out per format-spec §2.1, parses each
//! item's frontmatter into the model (§3), renders per-client output via
//! `crate::generate`, plans the result, and writes the plan to the
//! per-client paths computed by `output::output_path`. The project-scope
//! paths are those of format-spec §7 / ADR-0003; the global Copilot paths
//! differ and are listed in the doc comment of `output_path`.
//!
//! Entry points:
//! - [`install_with_lockfile`] — consumer-facing; install + record state +
//!   create ancillary files. Backs `upskill add`.
//! - [`install_from_source`] / [`install_from_local_path`] /
//!   [`install_from_git_url`] — library-only variants without lockfile or
//!   ancillary handling.
//! - [`check_scopes`] — reports the sources of the lockfiles that have a
//!   newer version, without a fetch and without changing a file.
//!
//! Authentication: clones use the bare URL and rely on git's own
//! configuration (credential helpers, `insteadOf` rewrites, SSH);
//! upskill never injects credentials.
//!
//! Audience filter: the top-level `audience` field (per format-spec §3.1)
//! restricts emission to listed clients; absence means all clients.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::generate::Client;
use crate::source::InstallSource;

mod apply;
mod bundle_plan;
mod check;
mod discovery;
mod git;
mod hash;
mod ignore;
mod install;
mod lifecycle;
mod output;
mod owned;
pub(crate) mod plan;
mod prepared;
mod report;

pub use apply::{ConflictState, FileConflict};
pub use bundle_plan::{BundleFile, BundleOutput, plan_bundle_output};
pub use check::{
    CheckedSource, GitRemote, LookupError, RemoteLookup, ScopedCheck, SourceStatus, check_scopes,
};
pub use git::{FetchedSource, fetch_ssot, install_from_git_url, install_from_source};
pub(crate) use hash::hash_item_dir;
pub use install::install_from_local_path;
pub use lifecycle::{doctor, doctor_scopes, installed_names, list, list_scopes, remove, update};
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
    /// Replace modified and unowned files. Set by `--force` on `add` and
    /// `update`. When false, an item with such a file at one of its
    /// destinations is not written and is reported in
    /// [`InstallReport::conflicts`], except as described for
    /// `keep_unwithheld_edits`.
    pub overwrite: bool,
    /// Alias mappings. For direct `--as alt-name`: vec contains `("", "alt-name")`.
    /// For bundle `--as original=alias`: vec contains `("original", "alias")`.
    pub aliases: Vec<(String, String)>,
    /// Item names to skip during install.
    pub excludes: Vec<String>,
    /// Item kinds to install. Empty means every kind. When it is not empty, the
    /// install runs no plugin step and no MCP step.
    pub kinds: Vec<ItemKind>,
    /// Bundle names that the run resolves as bundles only; set by `update`;
    /// `add` leaves it empty.
    pub bundles: Vec<String>,
    /// Consumer-side client selection (ADR-0012). Default targets all
    /// clients, preserving the emit-for-everyone behaviour.
    pub selection: crate::select::ClientSelection,
    /// Set by `update`. When true, an item whose only conflicts are
    /// modified destinations, and from which nothing is withheld (see
    /// `decide_plan`), is not written and is not reported in
    /// [`InstallReport::conflicts`]: its modified destinations are listed in
    /// [`InstallReport::kept_modified`], and its lockfile entry is unchanged
    /// except its commit and its `required_by`.
    pub keep_unwithheld_edits: bool,
    /// The scope of the install. It selects the output paths: the global
    /// scope writes Copilot content under `~/.copilot`. The default is the
    /// project scope.
    pub scope: crate::scope::Scope,
}

const ALL_CLIENTS: [Client; 3] = Client::ALL;

/// Flat list of every item name a single bundle declares (in
/// rule/skill/agent order). Used by `PreparedInstall::commit` to populate
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

/// Install + write lockfile. Consumer-facing entry point:
/// `prepared::prepare_install` followed by
/// `prepared::PreparedInstall::commit`.
///
/// Fetches the source, plans every output file, adopts a `schema: 1`
/// lockfile, writes the plan with one call, then merges the resulting
/// [`InstallReport`] into `<target>/.upskill-lock.json` via
/// [`crate::lockfile`], with each item's written files in `files`. The
/// write saves the lockfile with a `pending` record for each item that it
/// changes on disk before its first change on disk (when there is such an
/// item), and again after its last one, so that
/// running the same command again completes a failed install (see
/// `apply::execute_plan`); the bundles, plugins and MCP servers are saved
/// after that. Existing lockfile entries for the same `(kind, name)` are
/// replaced, except that their `required_by` is refreshed
/// (`prepared::refresh_required_by`), as is `required_by` of every other
/// entry; entries installed from a different source are otherwise left in
/// place.
///
/// Unless `options.overwrite` is true, an item with a modified or unowned
/// file at one of its destinations is not written, its lockfile entry is
/// unchanged except its `required_by`, and its conflicts are in
/// [`InstallReport::conflicts`]; the other items are written (see
/// `apply::decide_plan`). An installed name that is empty, `.` or `..`, or
/// that contains a path separator, fails before any change.
///
/// Before the adoption and before any write, the overlap check refuses
/// a source item directory that is, is inside, or contains an output path
/// of an item that the run installs: its own, or that of another item. The
/// protected directories are those of every item of every source that the
/// run reads, also of an item that is not installed. The item directories of
/// the run must not contain a file that the lockfile records (`files` or
/// `pending`) for an item from another source. A refused install changes no
/// file.
///
/// `git_ref` recorded per item is taken from the source variant when one
/// is pinned (Github/Git `git_ref`); local-path sources record `None`.
/// `source` label is the [`InstallSource`] `Display` form.
pub fn install_with_lockfile(
    source: &InstallSource,
    target: &Path,
    items: &[String],
    plugin_scope: crate::plugin::PluginScope,
    options: &AddOptions,
) -> Result<InstallReport> {
    if let InstallSource::LocalPath(path) = source {
        discovery::refuse_item_dir_source(path)?;
    }
    let prepared = prepared::prepare_install(source, target, items, options)?;
    // A source item directory must not contain a file that the lockfile
    // records for an item from another source (issue #550). Checked before
    // the adoption and before any write. The records are read after an
    // adoption in memory, which writes nothing: a `schema: 1` lockfile has
    // no `files` record before it.
    // Two spellings of one local path (a trailing slash, a `..` component,
    // a symbolic link) are one source: local labels are compared by their
    // canonical paths, any other label by its text.
    let same_source_key = |label: &str| match crate::source::parse_install_source_label(label) {
        Ok(InstallSource::LocalPath(path)) => std::fs::canonicalize(&path)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| label.to_string()),
        _ => label.to_string(),
    };
    let label = same_source_key(&source.to_string());
    let mut lock = crate::lockfile::Lockfile::load(target)?;
    owned::adopt_schema1_in_memory(target, &mut lock)?;
    let recorded: Vec<output::RecordedPath> = lock
        .items
        .iter()
        .filter(|entry| same_source_key(&entry.source) != label)
        .flat_map(output::recorded_paths)
        .collect();
    output::check_outside_recorded_paths(
        target,
        &recorded,
        prepared.source_items(),
        output::RecordedUse::AddReads,
    )?;
    prepared.commit(target, plugin_scope)
}
