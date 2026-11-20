//! Per-client output path computation and the check that no source item
//! directory of an install run overlaps an output path of an item that the
//! run installs (its own or another item's). Nothing here deletes files: `add`,
//! `update` and `remove` delete only the files in an item's ownership
//! record.
//!
//! The install pipeline plans per-client output files at the paths that
//! [`output_path`] computes. The project-scope paths are those of format-spec
//! §7 / ADR-0003, and so are the global-scope paths of Claude Code and
//! OpenCode; all come from the frozen layout in `crate::legacy::layout`. The global scope writes Copilot
//! content under `~/.copilot`. Pipeline callers go through [`output_path`],
//! except where a path that a released version wrote is needed
//! ([`released_output_path`]).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::{ALL_CLIENTS, ItemKind};
use crate::generate::Client;
use crate::scope::{Scope, ScopeRoot};

/// The path that released versions wrote in both scopes, relative to the
/// scope root and as defined in format-spec §7. Used to adopt a `schema: 1`
/// lockfile and by `doctor` for an item that has no ownership record, and to
/// find the files of a global install at an earlier path, because a released
/// version wrote those paths in both scopes.
pub(super) fn released_output_path(kind: ItemKind, client: Client, name: &str) -> PathBuf {
    crate::legacy::layout::output_path(kind, client.into(), name)
}

/// Where the install pipeline writes the per-client output for a given
/// `(kind, name)` in `scope`. The path is relative to the scope root. It is
/// [`released_output_path`] except for Copilot in the global scope, which
/// uses the personal directories of Copilot CLI:
///
/// - skills: `~/.copilot/skills/<name>/SKILL.md`
/// - rules: `~/.copilot/instructions/<name>.instructions.md`
/// - agents: `~/.copilot/agents/<name>.agent.md`
///
/// Published documentation read on 2026-11-13:
///
/// - <https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference>
///   (`skills/` and `agents/` under `~/.copilot`)
/// - <https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions>
///   (`$HOME/.copilot/instructions/**/*.instructions.md`)
/// - <https://code.claude.com/docs/en/skills>,
///   <https://code.claude.com/docs/en/memory> and
///   <https://code.claude.com/docs/en/sub-agents> (`~/.claude/skills`,
///   `~/.claude/rules` and `~/.claude/agents`; the same paths as in the
///   project scope below `.claude/`)
///
/// `remove` does not use this function: it deletes the paths in the item's
/// record.
pub(super) fn output_path(scope: Scope, kind: ItemKind, client: Client, name: &str) -> PathBuf {
    match (scope, client) {
        (Scope::Global, Client::Copilot) => {
            let base = Path::new(".copilot");
            match kind {
                ItemKind::Skill => base.join("skills").join(name).join("SKILL.md"),
                ItemKind::Rule => base
                    .join("instructions")
                    .join(format!("{name}.instructions.md")),
                ItemKind::Agent => base.join("agents").join(format!("{name}.agent.md")),
            }
        }
        _ => released_output_path(kind, client, name),
    }
}

/// Whether resources live beside the entrypoint in the item's own directory.
pub(super) fn is_dir_backed(kind: ItemKind, client: Client) -> bool {
    crate::legacy::layout::is_dir_backed(kind, client.into())
}

/// The resource namespace that released versions used in both scopes,
/// relative to the scope root.
pub(super) fn released_resource_base_path(kind: ItemKind, client: Client, name: &str) -> PathBuf {
    crate::legacy::layout::resource_base_path(kind, client.into(), name)
}

/// Resource namespace in `scope`, relative to the scope root: the directory
/// of the entry file for a directory-backed kind, the sibling `<name>/`
/// directory otherwise.
pub(super) fn resource_base_path(
    scope: Scope,
    kind: ItemKind,
    client: Client,
    name: &str,
) -> PathBuf {
    if scope == Scope::Project || client != Client::Copilot {
        return released_resource_base_path(kind, client, name);
    }
    let entry = output_path(scope, kind, client, name);
    let parent = entry.parent().unwrap_or(Path::new(""));
    if is_dir_backed(kind, client) {
        parent.to_path_buf()
    } else {
        parent.join(name)
    }
}

/// Whether the recorded path `rel` is inside the layout that upskill can
/// write for the item `(kind, name)`: it equals or lies below, for any
/// client and either scope, the entry file, the resource base, or the
/// released path of both. A path with a root, a prefix or a `..` component
/// is never inside the layout. A recorded path is compared as written; no
/// file system access happens here.
pub(super) fn in_item_layout(kind: ItemKind, name: &str, rel: &str) -> bool {
    use std::path::Component;
    let path = Path::new(rel);
    if !path.components().all(|c| matches!(c, Component::Normal(_))) {
        return false;
    }
    ALL_CLIENTS.iter().any(|&client| {
        let mut bases = vec![
            released_output_path(kind, client, name),
            released_resource_base_path(kind, client, name),
        ];
        for scope in [Scope::Project, Scope::Global] {
            bases.push(output_path(scope, kind, client, name));
            bases.push(resource_base_path(scope, kind, client, name));
        }
        bases.iter().any(|base| path.starts_with(base))
    })
}

/// Refuse an install when the source item directory `source_dir` is the
/// same directory as, is inside, or contains a path that the install
/// deletes or writes for `(kind, name)` under the root of `at`. These are
/// the per-item paths (the entrypoint and the resource base), not a whole
/// client output directory. The paths of `at.scope` are checked, and for
/// [`Scope::Global`] the paths of released versions as well, because an
/// update deletes the owned files at those paths. Without this check, an item directory such as
/// `.claude/skills/<name>` inside a source such as `.claude/skills` could be
/// deleted or overwritten by its own install. A path that
/// exists is compared in canonical form; a path that does not exist is
/// compared as given. Every client is checked because the install also
/// deletes files of clients that it does not write: the recorded files that
/// the plan no longer contains, for example those of a client that is no
/// longer selected (see [`super::apply::apply_plan`]).
///
/// When `source_dir` is inside an output path that is itself inside
/// `source_root`, the source root holds upskill's own output (for example a
/// registry rooted at `~/.claude`), and the error says so. When the client
/// directory, or a path below it down to that output path, is a symbolic
/// link, the error asks to remove the link instead. This includes a client
/// directory that is itself a link to a directory that contains the source
/// root (a dotfiles layout).
pub(super) fn check_source_outside_outputs(
    at: &ScopeRoot,
    kind: ItemKind,
    name: &str,
    source_dir: &Path,
    source_root: &Path,
) -> Result<()> {
    let canonical = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let source = canonical(source_dir);
    let root = canonical(source_root);
    let target = canonical(&at.root);
    let rels = |client| checked_output_paths(at.scope, kind, client, name);
    for client in ALL_CLIENTS {
        for rel in rels(client) {
            let out = canonical(&target.join(&rel));
            if source.starts_with(&out) || out.starts_with(&source) {
                let prefix = format!(
                    "{kind} '{name}': source directory {} overlaps the output path {}; \
                     installing would delete or overwrite the source.",
                    source_dir.display(),
                    out.display()
                );
                if let Some((owned, client_dir, link)) =
                    output_inside_root(&target, &source, &root, rels)
                {
                    // A symbolic link on the output path (the client
                    // directory or a directory or file below it) points into
                    // the source root. A client directory that is itself a
                    // link (a dotfiles layout) also gets this message.
                    if has_link_between(&client_dir, &link) {
                        anyhow::bail!(
                            "{prefix} The output path {} under the client directory {} is or \
                             passes through a symbolic link into the source root {}. Remove the \
                             symbolic link, then add the source again",
                            link.display(),
                            client_dir.display(),
                            root.display()
                        );
                    }
                    anyhow::bail!(
                        "{prefix} The source root {} contains the output path {} that upskill \
                         writes under the client directory {}. A source must not be rooted in a \
                         client directory or contain one. Move the source items to a directory \
                         outside the client directories and add them from there",
                        root.display(),
                        owned.display(),
                        client_dir.display()
                    );
                }
                anyhow::bail!(
                    "{prefix} Copy the item to a directory outside the client output directories \
                     and add it from there"
                );
            }
        }
    }
    Ok(())
}

/// True when `dir` is the same as, or is inside, a path that an install in
/// `at` writes for `(kind, name)` for any client. Paths are compared in
/// canonical form when they exist.
pub(super) fn is_inside_output_path(
    at: &ScopeRoot,
    kind: ItemKind,
    name: &str,
    dir: &Path,
) -> bool {
    let canonical = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let dir = canonical(dir);
    let target = canonical(&at.root);
    ALL_CLIENTS.into_iter().any(|client| {
        checked_output_paths(at.scope, kind, client, name)
            .iter()
            .any(|rel| dir.starts_with(canonical(&target.join(rel))))
    })
}

/// The per-item paths, relative to the scope root, that an install in
/// `scope` deletes or writes for `(kind, name)` and `client`: the entrypoint
/// and the resource base, and for [`Scope::Global`] the same paths of
/// released versions.
fn checked_output_paths(scope: Scope, kind: ItemKind, client: Client, name: &str) -> Vec<PathBuf> {
    let mut rels = vec![
        output_path(scope, kind, client, name),
        resource_base_path(scope, kind, client, name),
    ];
    if scope == Scope::Global {
        rels.push(released_output_path(kind, client, name));
        rels.push(released_resource_base_path(kind, client, name));
    }
    rels
}

/// One item directory in a source of an install run, for
/// [`check_run_outside_outputs`].
#[derive(Debug, Clone)]
pub(super) struct SourceItem {
    pub kind: ItemKind,
    /// The name of the item in its source.
    pub name: String,
    /// The installed name when the run installs the item, `None` when the
    /// run does not select it.
    pub installed: Option<String>,
    /// Whether `--exclude` names the item. The plan writes nothing for an
    /// excluded item, so it has no paths in the check; its directory is
    /// still protected.
    pub excluded: bool,
    /// The item directory.
    pub dir: PathBuf,
    /// The root of the source that holds the item.
    pub root: PathBuf,
}

/// Refuse an install run when a source item directory overlaps a path that
/// the run deletes or writes. `items` holds every item directory of every
/// source that the run reads, also the items that the run does not install.
/// A run is one `prepare_install` call, or for `update` all its sources
/// together.
///
/// Only a selected item that is not excluded has output paths here; an
/// excluded item writes nothing. First, each such item is checked against
/// its own paths with [`check_source_outside_outputs`]. Then the directory
/// of every item, selected, excluded or not selected, is compared with the
/// paths of every such item, its own included, which gives the result of
/// the first step again (all clients, as in
/// [`check_source_outside_outputs`]): the item directory must not be inside
/// such a path or contain one. Called before the first deletion or write of
/// the run. `update` also passes the same item directories to
/// [`check_outside_recorded_paths`] with the recorded files of every item that
/// it removes because its source no longer contains it, and `add` passes its
/// own item directories with the recorded files of every lockfile item from
/// another source.
pub(super) fn check_run_outside_outputs(at: &ScopeRoot, items: &[SourceItem]) -> Result<()> {
    let written = |item: &SourceItem| item.installed.clone().filter(|_| !item.excluded);
    for item in items {
        if let Some(installed) = written(item) {
            check_source_outside_outputs(at, item.kind, &installed, &item.dir, &item.root)?;
        }
    }
    let canonical = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let target = canonical(&at.root);
    let dirs: Vec<PathBuf> = items.iter().map(|item| canonical(&item.dir)).collect();
    for outer in items {
        let Some(outer_name) = written(outer) else {
            continue;
        };
        for client in ALL_CLIENTS {
            for rel in checked_output_paths(at.scope, outer.kind, client, &outer_name) {
                let out = canonical(&target.join(&rel));
                for (inner, inner_dir) in items.iter().zip(&dirs) {
                    let relation = if inner_dir.starts_with(&out) {
                        "is inside"
                    } else if out.starts_with(inner_dir) {
                        "contains"
                    } else {
                        continue;
                    };
                    return Err(cross_overlap_error(
                        inner,
                        relation,
                        &out,
                        outer.kind,
                        &outer_name,
                    ));
                }
            }
        }
    }
    Ok(())
}

/// The error of [`check_run_outside_outputs`]: the directory of `inner` is inside, or
/// contains (`relation`), the path `out` of the item `(outer_kind,
/// outer_name)`.
fn cross_overlap_error(
    inner: &SourceItem,
    relation: &str,
    out: &Path,
    outer_kind: ItemKind,
    outer_name: &str,
) -> anyhow::Error {
    let inner_name = inner.installed.as_ref().unwrap_or(&inner.name);
    anyhow::anyhow!(
        "the source directory of {} '{inner_name}' ({}) {relation} the output path {} of {} \
         '{outer_name}'; installing would delete or overwrite the source. Copy the items to a \
         directory outside the client output directories and add them from there",
        inner.kind,
        inner.dir.display(),
        out.display(),
        outer_kind
    )
}

/// One recorded file of an installed item, relative to the scope root: a
/// path of its `files` or `pending` record that `apply::deletable` accepts.
#[derive(Debug, Clone)]
pub(super) struct RecordedPath {
    pub kind: ItemKind,
    pub name: String,
    pub path: PathBuf,
}

/// The recorded files of `entry` (its `files` and `pending` records), as
/// `remove` plans them for deletion.
pub(super) fn recorded_paths(entry: &crate::lockfile::LockedItem) -> Vec<RecordedPath> {
    entry
        .files
        .iter()
        .chain(&entry.pending)
        .filter_map(|file| super::apply::deletable(&file.path))
        .map(|path| RecordedPath {
            kind: entry.kind,
            name: entry.name.clone(),
            path,
        })
        .collect()
}

/// The first source item of `items` whose directory contains a path of
/// `recorded` under the scope root `target`. Returns the item, the canonical
/// path and the recorded path. Paths are compared in canonical form, as in
/// [`check_run_outside_outputs`]. A recorded path names a file, so an item
/// directory cannot be inside it; when a directory now stands at a recorded
/// path, that path is not a regular file and no deletion removes it.
pub(super) fn find_recorded_overlap<'a>(
    target: &Path,
    recorded: &'a [RecordedPath],
    items: &'a [SourceItem],
) -> Option<(&'a SourceItem, PathBuf, &'a RecordedPath)> {
    let canonical = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let target = canonical(target);
    let dirs: Vec<PathBuf> = items.iter().map(|item| canonical(&item.dir)).collect();
    for rec in recorded {
        let out = canonical(&target.join(&rec.path));
        for (item, dir) in items.iter().zip(&dirs) {
            if out.starts_with(dir) {
                return Some((item, out, rec));
            }
        }
    }
    None
}

/// Which command reads the recorded paths in [`check_outside_recorded_paths`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecordedUse {
    /// `update` deletes the recorded files of an item that its source no
    /// longer contains.
    UpdateDeletes,
    /// `add` installs from a source that holds recorded files of installed
    /// items of another source.
    AddReads,
}

/// Refuse a run when a source item directory of `items` contains a recorded
/// path of `recorded`. For [`RecordedUse::UpdateDeletes`], `recorded` holds
/// every recorded file (`files` and `pending`) of every item that the run
/// removes because its source no longer contains it, also a modified file
/// that the removal would keep. For [`RecordedUse::AddReads`], `recorded`
/// holds every recorded file of every lockfile item from another source.
/// Called before the first deletion or write of the run.
pub(super) fn check_outside_recorded_paths(
    target: &Path,
    recorded: &[RecordedPath],
    items: &[SourceItem],
    usage: RecordedUse,
) -> Result<()> {
    let Some((item, out, rec)) = find_recorded_overlap(target, recorded, items) else {
        return Ok(());
    };
    let inner = item.installed.as_ref().unwrap_or(&item.name);
    match usage {
        RecordedUse::UpdateDeletes => anyhow::bail!(
            "the source directory of {} '{inner}' ({}) contains the recorded file {} of {} '{}'; \
             the update would delete this file because the source of {} '{}' no longer \
             contains the item. Remove {} '{inner}' first, or restore {} '{}' in its source",
            item.kind,
            item.dir.display(),
            out.display(),
            rec.kind,
            rec.name,
            rec.kind,
            rec.name,
            item.kind,
            rec.kind,
            rec.name
        ),
        RecordedUse::AddReads => anyhow::bail!(
            "the source directory of {} '{inner}' ({}) contains the file {}, which upskill \
             recorded for the installed {} '{}'; an update or a remove of {} '{}' would delete \
             it. Copy the item to a directory outside the client output directories and add it \
             from there",
            item.kind,
            item.dir.display(),
            out.display(),
            rec.kind,
            rec.name,
            rec.kind,
            rec.name
        ),
    }
}

/// True when `top`, or a path from `top` down to `path` (included), is a
/// symbolic link. `path` must be below `top`.
fn has_link_between(top: &Path, path: &Path) -> bool {
    let is_link = |p: &Path| fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink());
    let Ok(rest) = path.strip_prefix(top) else {
        return false;
    };
    let mut cursor = top.to_path_buf();
    if is_link(&cursor) {
        return true;
    }
    for component in rest.components() {
        cursor.push(component);
        if is_link(&cursor) {
            return true;
        }
    }
    false
}

/// Find an output path that contains `source` (or is `source`) and is inside
/// `root`. Returns that output path in canonical form, its client directory
/// (the first component of the output path under `target`) and the output
/// path as given (before symbolic links are resolved).
fn output_inside_root(
    target: &Path,
    source: &Path,
    root: &Path,
    rels: impl Fn(Client) -> Vec<PathBuf>,
) -> Option<(PathBuf, PathBuf, PathBuf)> {
    for client in ALL_CLIENTS {
        for rel in rels(client) {
            let given = target.join(&rel);
            let out = fs::canonicalize(&given).unwrap_or_else(|_| given.clone());
            if source.starts_with(&out) && out.starts_with(root) {
                let first = rel.components().next()?;
                return Some((out, target.join(first), given));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A project target with `rel` created as a directory. Returns
    /// `(tmp, target)`.
    fn target_with_dir(rel: &str) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("proj");
        fs::create_dir_all(target.join(rel)).unwrap();
        (tmp, target)
    }

    fn at(scope: Scope, root: &Path) -> ScopeRoot {
        ScopeRoot {
            scope,
            root: root.to_path_buf(),
        }
    }

    fn assert_refused_in(scope: Scope, target: &Path, kind: ItemKind, name: &str, source: &Path) {
        let err = check_source_outside_outputs(&at(scope, target), kind, name, source, source)
            .expect_err("overlap must be refused");
        assert!(
            err.to_string().contains("overlaps the output path"),
            "{err}"
        );
    }

    fn assert_refused(target: &Path, kind: ItemKind, name: &str, source: &Path) {
        assert_refused_in(Scope::Project, target, kind, name, source);
    }

    #[test]
    fn overlap_check_refuses_source_inside_an_output_path() {
        let (_tmp, target) = target_with_dir(".claude/skills/x/nested");
        let source = target.join(".claude/skills/x/nested");
        assert_refused(&target, ItemKind::Skill, "x", &source);
    }

    #[test]
    fn overlap_check_refuses_source_that_contains_an_output_path() {
        let (_tmp, target) = target_with_dir(".git");
        assert_refused(&target, ItemKind::Skill, "x", &target);
    }

    #[cfg(unix)]
    #[test]
    fn overlap_check_compares_canonical_paths() {
        let (tmp, target) = target_with_dir(".claude/skills/x");
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(target.join(".claude/skills/x"), &link).unwrap();
        assert_refused(&target, ItemKind::Skill, "x", &link);
        let dotted = target.join(".claude/skills/../skills/x");
        assert_refused(&target, ItemKind::Skill, "x", &dotted);
    }

    #[test]
    fn overlap_check_covers_the_resource_base_path() {
        // A Claude rule renders to `.claude/rules/r.md`; only its resource
        // base `.claude/rules/r` overlaps this source.
        let (_tmp, target) = target_with_dir(".claude/rules/r");
        let source = target.join(".claude/rules/r");
        assert_refused(&target, ItemKind::Rule, "r", &source);
    }

    #[test]
    fn overlap_check_accepts_a_separate_source() {
        let (tmp, target) = target_with_dir(".claude/skills/x");
        let source = tmp.path().join("source/x");
        fs::create_dir_all(&source).unwrap();
        check_source_outside_outputs(
            &at(Scope::Project, &target),
            ItemKind::Skill,
            "x",
            &source,
            &source,
        )
        .unwrap();
    }

    fn source_item(name: &str, installed: Option<&str>, dir: &Path) -> SourceItem {
        SourceItem {
            kind: ItemKind::Skill,
            name: name.to_string(),
            installed: installed.map(str::to_string),
            excluded: false,
            dir: dir.to_path_buf(),
            root: dir.parent().unwrap().to_path_buf(),
        }
    }

    fn run_check(target: &Path, items: &[SourceItem]) -> Result<()> {
        check_run_outside_outputs(&at(Scope::Project, target), items)
    }

    #[test]
    fn run_check_refuses_a_source_item_inside_the_output_path_of_another_item() {
        let (tmp, target) = target_with_dir(".claude/skills/q/y");
        let q = tmp.path().join("source/q");
        fs::create_dir_all(&q).unwrap();
        let y = target.join(".claude/skills/q/y");
        let err = run_check(
            &target,
            &[
                source_item("q", Some("q"), &q),
                source_item("y", Some("y"), &y),
            ],
        )
        .expect_err("cross-item overlap must be refused");
        let msg = err.to_string();
        assert!(msg.contains("source directory of skill 'y'"), "{msg}");
        assert!(msg.contains(") is inside the output path"), "{msg}");
        assert!(msg.contains("of skill 'q';"), "{msg}");
    }

    #[test]
    fn run_check_refuses_a_source_item_that_contains_the_output_path_of_another_item() {
        let (tmp, target) = target_with_dir(".claude/skills/q");
        let q = tmp.path().join("source/q");
        fs::create_dir_all(&q).unwrap();
        let skills = target.join(".claude/skills");
        let err = run_check(
            &target,
            &[
                source_item("q", Some("q"), &q),
                source_item("all", None, &skills),
            ],
        )
        .expect_err("cross-item overlap must be refused");
        assert!(
            err.to_string().contains(") contains the output path"),
            "{err}"
        );
    }

    #[test]
    fn run_check_protects_an_item_that_is_not_installed() {
        let (tmp, target) = target_with_dir(".claude/skills/q/y");
        let q = tmp.path().join("source/q");
        fs::create_dir_all(&q).unwrap();
        let y = target.join(".claude/skills/q/y");
        let err = run_check(
            &target,
            &[source_item("q", Some("q"), &q), source_item("y", None, &y)],
        )
        .expect_err("a source item that is not installed is still protected");
        assert!(err.to_string().contains("skill 'y'"), "{err}");
    }

    #[test]
    fn run_check_uses_the_installed_name_of_the_outer_item() {
        let (tmp, target) = target_with_dir(".claude/skills/q/y");
        let q = tmp.path().join("source/q");
        fs::create_dir_all(&q).unwrap();
        let y = target.join(".claude/skills/q/y");
        // `q` installs as `bar`, so nothing is written at `.claude/skills/q`.
        run_check(
            &target,
            &[
                source_item("q", Some("bar"), &q),
                source_item("y", Some("y"), &y),
            ],
        )
        .unwrap();
    }

    #[test]
    fn run_check_ignores_the_paths_of_an_item_that_is_not_installed() {
        let (tmp, target) = target_with_dir(".claude/skills/q/y");
        let q = tmp.path().join("source/q");
        fs::create_dir_all(&q).unwrap();
        let y = target.join(".claude/skills/q/y");
        run_check(
            &target,
            &[source_item("q", None, &q), source_item("y", Some("y"), &y)],
        )
        .unwrap();
    }

    #[test]
    fn run_check_ignores_the_paths_of_an_excluded_item() {
        let (tmp, target) = target_with_dir(".claude/skills/q/y");
        let q = tmp.path().join("source/q");
        fs::create_dir_all(&q).unwrap();
        let y = target.join(".claude/skills/q/y");
        let mut excluded = source_item("q", Some("q"), &q);
        excluded.excluded = true;
        run_check(&target, &[excluded, source_item("y", Some("y"), &y)]).unwrap();
    }

    #[test]
    fn run_check_does_not_check_an_excluded_item_against_its_own_paths() {
        let (_tmp, target) = target_with_dir(".claude/skills/x");
        let x = target.join(".claude/skills/x");
        let mut excluded = source_item("x", Some("x"), &x);
        excluded.excluded = true;
        run_check(&target, &[excluded]).unwrap();
    }

    #[test]
    fn run_check_compares_the_paths_of_every_client() {
        // `.agents/skills/q` is the OpenCode output path of `q`.
        let (tmp, target) = target_with_dir(".agents/skills/q/y");
        let q = tmp.path().join("source/q");
        fs::create_dir_all(&q).unwrap();
        let y = target.join(".agents/skills/q/y");
        let err = run_check(
            &target,
            &[source_item("q", Some("q"), &q), source_item("y", None, &y)],
        )
        .expect_err("an overlap with an OpenCode path must be refused");
        assert!(err.to_string().contains("of skill 'q';"), "{err}");
    }

    #[test]
    fn run_check_compares_the_released_paths_in_the_global_scope() {
        // In the global scope, Copilot writes under `.copilot`; released
        // versions wrote `.github/skills/q`, which an update deletes.
        let (tmp, target) = target_with_dir(".github/skills/q/y");
        let q = tmp.path().join("source/q");
        fs::create_dir_all(&q).unwrap();
        let y = target.join(".github/skills/q/y");
        let err = check_run_outside_outputs(
            &at(Scope::Global, &target),
            &[source_item("q", Some("q"), &q), source_item("y", None, &y)],
        )
        .expect_err("an overlap with a released path must be refused");
        assert!(err.to_string().contains("of skill 'q';"), "{err}");
    }

    #[test]
    fn run_check_compares_the_entrypoint_of_a_rule() {
        // A Claude rule `r` renders to `.claude/rules/r.md`, a sibling of its
        // resource base `.claude/rules/r`. A directory named `r.md` overlaps
        // only the entrypoint.
        let (tmp, target) = target_with_dir(".claude/rules/r.md/y");
        let r = tmp.path().join("source/r");
        fs::create_dir_all(&r).unwrap();
        let y = target.join(".claude/rules/r.md/y");
        let mut rule = source_item("r", Some("r"), &r);
        rule.kind = ItemKind::Rule;
        let err = run_check(&target, &[rule, source_item("y", None, &y)])
            .expect_err("an overlap with a rule entrypoint must be refused");
        assert!(err.to_string().contains("of rule 'r';"), "{err}");
    }

    #[test]
    fn run_check_names_the_inner_item_by_its_installed_name() {
        let (tmp, target) = target_with_dir(".claude/skills/q/y");
        let q = tmp.path().join("source/q");
        fs::create_dir_all(&q).unwrap();
        let y = target.join(".claude/skills/q/y");
        let err = run_check(
            &target,
            &[
                source_item("q", Some("q"), &q),
                source_item("y", Some("z"), &y),
            ],
        )
        .expect_err("cross-item overlap must be refused");
        assert!(
            err.to_string().contains("source directory of skill 'z' ("),
            "{err}"
        );
    }

    #[test]
    fn run_check_keeps_the_same_item_refusal() {
        let (_tmp, target) = target_with_dir(".claude/skills/x/nested");
        let source = target.join(".claude/skills/x/nested");
        let err = run_check(&target, &[source_item("x", Some("x"), &source)])
            .expect_err("same-item overlap must be refused");
        assert!(
            err.to_string().starts_with("skill 'x': source directory "),
            "{err}"
        );
    }

    #[test]
    fn output_paths_and_resource_bases_match_historical_spec() {
        let cases = [
            (
                ItemKind::Skill,
                Client::Claude,
                ".claude/skills/x/SKILL.md",
                ".claude/skills/x",
            ),
            (
                ItemKind::Skill,
                Client::Copilot,
                ".github/skills/x/SKILL.md",
                ".github/skills/x",
            ),
            (
                ItemKind::Skill,
                Client::OpenCode,
                ".agents/skills/x/SKILL.md",
                ".agents/skills/x",
            ),
            (
                ItemKind::Rule,
                Client::Claude,
                ".claude/rules/x.md",
                ".claude/rules/x",
            ),
            (
                ItemKind::Rule,
                Client::Copilot,
                ".github/instructions/x.instructions.md",
                ".github/instructions/x",
            ),
            (
                ItemKind::Rule,
                Client::OpenCode,
                ".agents/rules/x/RULE.md",
                ".agents/rules/x",
            ),
            (
                ItemKind::Agent,
                Client::Claude,
                ".claude/agents/x.md",
                ".claude/agents/x",
            ),
            (
                ItemKind::Agent,
                Client::Copilot,
                ".github/agents/x.agent.md",
                ".github/agents/x",
            ),
            (
                ItemKind::Agent,
                Client::OpenCode,
                ".opencode/agents/x.md",
                ".opencode/agents/x",
            ),
        ];
        for (kind, client, entrypoint, resources) in cases {
            assert_eq!(
                output_path(Scope::Project, kind, client, "x"),
                PathBuf::from(entrypoint)
            );
            assert_eq!(
                resource_base_path(Scope::Project, kind, client, "x"),
                PathBuf::from(resources)
            );
        }
    }

    #[test]
    fn raw_separator_names_retain_native_pipeline_layout() {
        assert_eq!(
            output_path(Scope::Project, ItemKind::Rule, Client::Claude, "a/b"),
            PathBuf::from(".claude/rules/a/b.md")
        );
        assert_eq!(
            resource_base_path(Scope::Project, ItemKind::Rule, Client::Claude, "a/b"),
            PathBuf::from(".claude/rules/a/a/b")
        );
        #[cfg(unix)]
        assert_eq!(
            resource_base_path(Scope::Project, ItemKind::Rule, Client::Claude, "a\\b"),
            PathBuf::from(".claude/rules/a\\b")
        );
        #[cfg(windows)]
        assert_eq!(
            resource_base_path(Scope::Project, ItemKind::Rule, Client::Claude, "a\\b"),
            PathBuf::from(".claude/rules/a/a/b")
        );
    }

    #[test]
    fn is_dir_backed_matches_layout() {
        // Skills are directory-backed on every client.
        for c in ALL_CLIENTS {
            assert!(is_dir_backed(ItemKind::Skill, c));
        }
        // Rules: only opencode is directory-backed.
        assert!(is_dir_backed(ItemKind::Rule, Client::OpenCode));
        assert!(!is_dir_backed(ItemKind::Rule, Client::Claude));
        assert!(!is_dir_backed(ItemKind::Rule, Client::Copilot));
        // Agents are flat on every client.
        for c in ALL_CLIENTS {
            assert!(!is_dir_backed(ItemKind::Agent, c));
        }
    }

    #[test]
    fn resource_base_dir_backed_is_entrypoint_dir() {
        assert_eq!(
            resource_base_path(Scope::Project, ItemKind::Skill, Client::Claude, "x"),
            PathBuf::from(".claude/skills/x")
        );
        assert_eq!(
            resource_base_path(Scope::Project, ItemKind::Rule, Client::OpenCode, "x"),
            PathBuf::from(".agents/rules/x")
        );
    }

    #[test]
    fn resource_base_flat_is_sibling_namespace_dir() {
        assert_eq!(
            resource_base_path(Scope::Project, ItemKind::Rule, Client::Claude, "x"),
            PathBuf::from(".claude/rules/x")
        );
        assert_eq!(
            resource_base_path(Scope::Project, ItemKind::Rule, Client::Copilot, "x"),
            PathBuf::from(".github/instructions/x")
        );
        assert_eq!(
            resource_base_path(Scope::Project, ItemKind::Agent, Client::Claude, "x"),
            PathBuf::from(".claude/agents/x")
        );
        assert_eq!(
            resource_base_path(Scope::Project, ItemKind::Agent, Client::OpenCode, "x"),
            PathBuf::from(".opencode/agents/x")
        );
    }

    #[test]
    fn global_copilot_paths_use_the_personal_directories() {
        let cases = [
            (
                ItemKind::Skill,
                ".copilot/skills/x/SKILL.md",
                ".copilot/skills/x",
            ),
            (
                ItemKind::Rule,
                ".copilot/instructions/x.instructions.md",
                ".copilot/instructions/x",
            ),
            (
                ItemKind::Agent,
                ".copilot/agents/x.agent.md",
                ".copilot/agents/x",
            ),
        ];
        for (kind, entrypoint, resources) in cases {
            assert_eq!(
                output_path(Scope::Global, kind, Client::Copilot, "x"),
                PathBuf::from(entrypoint)
            );
            assert_eq!(
                resource_base_path(Scope::Global, kind, Client::Copilot, "x"),
                PathBuf::from(resources)
            );
        }
    }

    #[test]
    fn only_global_copilot_differs_from_the_released_paths() {
        for kind in [ItemKind::Skill, ItemKind::Rule, ItemKind::Agent] {
            for client in ALL_CLIENTS {
                let released = released_output_path(kind, client, "x");
                let released_base = released_resource_base_path(kind, client, "x");
                assert_eq!(output_path(Scope::Project, kind, client, "x"), released);
                assert_eq!(
                    resource_base_path(Scope::Project, kind, client, "x"),
                    released_base
                );
                if client != Client::Copilot {
                    assert_eq!(output_path(Scope::Global, kind, client, "x"), released);
                    assert_eq!(
                        resource_base_path(Scope::Global, kind, client, "x"),
                        released_base
                    );
                }
            }
        }
    }

    #[test]
    fn global_overlap_check_covers_the_released_copilot_paths() {
        // The global scope writes `.copilot/skills/x`, but an update also
        // deletes the owned files at the released `.github/skills/x`.
        let (_tmp, target) = target_with_dir(".github/skills/x");
        let source = target.join(".github/skills/x");
        assert_refused_in(Scope::Global, &target, ItemKind::Skill, "x", &source);
        let source = target.join(".copilot/skills");
        fs::create_dir_all(&source).unwrap();
        assert_refused_in(Scope::Global, &target, ItemKind::Skill, "x", &source);
    }

    #[test]
    fn global_overlap_check_covers_the_released_resource_base_of_a_rule() {
        // A rule writes `.copilot/instructions/x.instructions.md` and its
        // resource base `.copilot/instructions/x` in the global scope. Only
        // the released resource base `.github/instructions/x` overlaps this
        // source.
        let (_tmp, target) = target_with_dir(".github/instructions/x");
        let source = target.join(".github/instructions/x");
        assert_refused_in(Scope::Global, &target, ItemKind::Rule, "x", &source);
    }

    #[test]
    fn in_item_layout_accepts_the_paths_of_the_item() {
        for rel in [
            ".claude/skills/s/SKILL.md",
            ".claude/skills/s/notes/a.md",
            ".github/skills/s/SKILL.md",
            ".copilot/skills/s/SKILL.md",
        ] {
            assert!(in_item_layout(ItemKind::Skill, "s", rel), "{rel}");
        }
        assert!(in_item_layout(
            ItemKind::Agent,
            "a",
            ".copilot/agents/a.agent.md"
        ));
        assert!(in_item_layout(
            ItemKind::Rule,
            "r",
            ".copilot/instructions/r.instructions.md"
        ));
    }

    #[test]
    fn in_item_layout_refuses_every_other_path() {
        for rel in [
            ".github/workflows/ci.yml",
            ".claude/skills/other/SKILL.md",
            ".claude/skills/sx/SKILL.md",
            ".claude/skills/../../outside.txt",
            ".claude/skills/s/../other/SKILL.md",
            "/etc/hosts",
            "/.claude/skills/s/SKILL.md",
            "src/main.rs",
            "",
        ] {
            assert!(!in_item_layout(ItemKind::Skill, "s", rel), "{rel}");
        }
        assert!(!in_item_layout(ItemKind::Skill, "s", ".claude/rules/s.md"));
    }
}
