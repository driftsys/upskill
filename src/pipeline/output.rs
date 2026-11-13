//! Per-client output path computation and the check that a source item does
//! not overlap its own output paths. Nothing here deletes files: `add`,
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
/// registry rooted at `~/.claude`), and the error says so.
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
    let rels = |client| {
        let mut rels = vec![
            output_path(at.scope, kind, client, name),
            resource_base_path(at.scope, kind, client, name),
        ];
        if at.scope == Scope::Global {
            rels.push(released_output_path(kind, client, name));
            rels.push(released_resource_base_path(kind, client, name));
        }
        rels
    };
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
                if let Some((owned, client_dir)) = output_inside_root(&target, &source, &root, rels)
                {
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

/// Find an output path that contains `source` (or is `source`) and is inside
/// `root`. Returns that output path and its client directory (the first
/// component of the output path under `target`).
fn output_inside_root(
    target: &Path,
    source: &Path,
    root: &Path,
    rels: impl Fn(Client) -> Vec<PathBuf>,
) -> Option<(PathBuf, PathBuf)> {
    for client in ALL_CLIENTS {
        for rel in rels(client) {
            let out = fs::canonicalize(target.join(&rel)).unwrap_or_else(|_| target.join(&rel));
            if source.starts_with(&out) && out.starts_with(root) {
                let first = rel.components().next()?;
                return Some((out, target.join(first)));
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
}
