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
pub(crate) use ignore::filter_ignored;
mod install;
mod lifecycle;
mod output;
mod owned;
pub(crate) mod plan;
mod prepared;
mod report;

pub use apply::{ConflictState, FileConflict};
pub use bundle_plan::{
    BundleCommand, BundleFile, BundleHook, BundleOutput, CommandResource, CommandSource,
    HookSource, plan_bundle_output, read_command, read_hook,
};
pub use check::{
    CheckedSource, GitRemote, LookupError, RemoteLookup, ScopedCheck, SourceStatus, check_scopes,
};
pub(crate) use discovery::normalize_lexically;
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
    /// Set by `update`. When true, the run never removes a name from the
    /// excludes recorded for the scope. Only the positional names of `add`
    /// remove names from the record.
    pub keep_recorded_excludes: bool,
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

/// The excludes of a run: the recorded names that `named` does not name,
/// plus the `--exclude` names of the command (`own`).
pub(crate) fn effective_excludes(
    recorded: &[String],
    own: &[String],
    named: &[String],
) -> Vec<String> {
    let mut names: std::collections::BTreeSet<String> = recorded
        .iter()
        .filter(|name| !named.contains(name))
        .cloned()
        .collect();
    names.extend(own.iter().cloned());
    names.into_iter().collect()
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
/// entry unless a kind restriction limits the refresh to planned entries.
/// Entries installed from a different source are otherwise left in place.
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
/// A source directory that is itself one item directory, with no item in
/// its child or grandchild directories, no `*.bundle.yaml` file and no item
/// name that matches a bundle in it, is refused before any write (see
/// `discovery::refuse_item_dir_source`): a local path before the fetch, a
/// git subfolder or repository root after it. The git check runs after
/// `prepare_install`, so with item names given that match nothing there,
/// `add` fails first with the "no matching items or bundles" error.
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
        let at = crate::scope::ScopeRoot {
            scope: options.scope,
            root: target.to_path_buf(),
        };
        let inside_output =
            |kind: ItemKind, name: &str| output::is_inside_output_path(&at, kind, name, path);
        discovery::refuse_item_dir_source(
            path,
            items,
            discovery::ItemDirSource::Local {
                inside_output: &inside_output,
            },
        )?;
    }
    // `add` also applies the excludes recorded for the scope (issue #570).
    // An item named on the command line is not excluded unless the same
    // command also excludes it.
    let recorded = crate::lockfile::Lockfile::load(target)?.excludes;
    let own_excludes = options.excludes.clone();
    let mut effective = options.clone();
    effective.excludes = effective_excludes(&recorded, &options.excludes, items);
    let options = &effective;
    let prepared = prepared::prepare_install(source, target, items, options)?;
    // A name of the command's own `--exclude` that matches nothing is most
    // likely a typo. The recorded names are not checked: an item can leave
    // its source. Nothing is written before this point.
    let unknown = prepared.unknown_excludes(&own_excludes)?;
    if !unknown.is_empty() {
        return Err(UnknownExclude { names: unknown }.into());
    }
    // A git subfolder or repository root that is one item directory
    // installs nothing (#577). `prepare_install` writes nothing, so the
    // refusal still comes before any write. `update` does not run this check.
    if let Some(git) = discovery::GitSourceForm::of(source) {
        discovery::refuse_item_dir_source(
            prepared.entry_root(),
            items,
            discovery::ItemDirSource::Git(git),
        )?;
    }
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
    let pack_only_skipped = prepared.pack_only_skipped();
    let mut report = prepared.commit(target, plugin_scope)?;
    report.pack_only_skipped = pack_only_skipped;
    // An item of this source that the command excludes and that is already
    // installed is removed, as `remove` removes it, but without the other
    // members of its group (issue #570).
    let lock = crate::lockfile::Lockfile::load(target)?;
    let excluded_installed: Vec<(ItemKind, String)> = lock
        .items
        .iter()
        .filter(|i| same_source_key(&i.source) == label)
        .filter(|i| options.kinds.is_empty() || options.kinds.contains(&i.kind))
        .filter(|i| {
            let in_source = i.source_name.as_ref().unwrap_or(&i.name);
            own_excludes.contains(in_source) && !items.contains(in_source)
        })
        .map(|i| (i.kind, i.name.clone()))
        .collect();
    if !excluded_installed.is_empty() {
        let at = crate::scope::ScopeRoot {
            scope: options.scope,
            root: target.to_path_buf(),
        };
        // An item with a modified file is kept, files and lockfile entry,
        // and reported in `kept_excluded`.
        let (removal, kept) = lifecycle::remove_exact(&at, &excluded_installed)?;
        let removed_items: Vec<(ItemKind, String)> = removal
            .items
            .iter()
            .map(|item| (item.kind, item.name.clone()))
            .collect();
        // `required_by` links to a removed item stay, as with `remove`:
        // `doctor` reports a dependency left without a requirer (ADR-0009).
        report.removed_excluded = removed_items;
        report.kept_excluded = kept;
    }
    let removed = report.removed_excluded.clone();
    let kept: Vec<(ItemKind, String)> = report
        .kept_excluded
        .iter()
        .map(|item| (item.kind, item.name.clone()))
        .collect();
    report
        .excluded
        .retain(|item| !removed.contains(item) && !kept.contains(item));
    Ok(report)
}
