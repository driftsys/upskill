//! Per-client output path computation and file I/O.
//!
//! The install pipeline writes (and `remove` deletes) per-client output
//! files at the paths defined in format-spec §7 / ADR-0003. The frozen
//! legacy layout keeps install, remove and legacy candidates in lockstep;
//! pipeline callers go through [`output_path`].

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::{ALL_CLIENTS, ItemKind};
use crate::generate::Client;

/// Where the install pipeline writes — and `remove` deletes — the per-
/// client output for a given `(kind, name)`. Path is relative to the
/// install target root and matches format-spec §7. Used by both
/// `install_*` and `remove` so the two stay in lockstep.
pub(super) fn output_path(kind: ItemKind, client: Client, name: &str) -> PathBuf {
    crate::legacy::layout::output_path(kind, client.into(), name)
}

/// Whether resources live beside the entrypoint in the item's own directory.
pub(super) fn is_dir_backed(kind: ItemKind, client: Client) -> bool {
    crate::legacy::layout::is_dir_backed(kind, client.into())
}

/// Historical resource namespace, relative to the installation root.
pub(super) fn resource_base_path(kind: ItemKind, client: Client, name: &str) -> PathBuf {
    crate::legacy::layout::resource_base_path(kind, client.into(), name)
}

/// Copy each resource (a path relative to the SSOT item directory
/// `source_dir`) into the client's [`resource_base_path`], preserving
/// sub-structure. `fs::copy` preserves the file mode on Unix, so an
/// executable script stays executable.
pub(super) fn copy_item_resources(
    target: &Path,
    source_dir: &Path,
    kind: ItemKind,
    client: Client,
    name: &str,
    resources: &[PathBuf],
) -> Result<()> {
    if resources.is_empty() {
        return Ok(());
    }
    let base = resource_base_path(kind, client, name);
    for rel in resources {
        let from = source_dir.join(rel);
        let to = target.join(&base).join(rel);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create dir {}", parent.display()))?;
        }
        fs::copy(&from, &to)
            .with_context(|| format!("copy {} to {}", from.display(), to.display()))?;
    }
    Ok(())
}

pub(super) fn write_output(target: &Path, rel: &Path, content: &str) -> Result<()> {
    let full = target.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create dir {}", parent.display()))?;
    }
    fs::write(&full, content).with_context(|| format!("write {}", full.display()))?;
    Ok(())
}

/// Refuse an install when the source item directory `source_dir` is the
/// same directory as, is inside, or contains a path that
/// [`remove_item_outputs`] deletes or the install writes for `(kind, name)`
/// under `target`. These are the per-item paths (the entrypoint and the
/// resource base), not a whole client output directory. Without this check,
/// an item directory such as `.claude/skills/<name>` inside a source such as
/// `.claude/skills` would be deleted before its files are copied. A path that
/// exists is compared in canonical form; a path that does not exist is
/// compared as given. Every client is checked because [`remove_item_outputs`]
/// touches every client.
///
/// When `source_dir` is inside an output path that is itself inside
/// `source_root`, the source root holds upskill's own output (for example a
/// registry rooted at `~/.claude`), and the error says so.
pub(super) fn check_source_outside_outputs(
    target: &Path,
    kind: ItemKind,
    name: &str,
    source_dir: &Path,
    source_root: &Path,
) -> Result<()> {
    let canonical = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let source = canonical(source_dir);
    let root = canonical(source_root);
    let target = canonical(target);
    let rels = |client| {
        [
            output_path(kind, client, name),
            resource_base_path(kind, client, name),
        ]
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
    rels: impl Fn(Client) -> [PathBuf; 2],
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

/// Delete all per-client output files for an item (best-effort).
pub(super) fn remove_item_outputs(target: &Path, kind: ItemKind, name: &str) {
    for client in ALL_CLIENTS {
        // Remove the entrypoint file. For directory-backed kinds this also
        // gets swept by the resource-base removal below; doing it first is
        // harmless (the second remove is a no-op).
        let _ = fs::remove_file(target.join(output_path(kind, client, name)));
        // Remove the item's resource base directory. For directory-backed
        // kinds this is the item's own `<name>/` dir (entrypoint + resources);
        // for flat kinds it is the sibling `<name>/` namespace dir. Both are
        // item-specific — `resource_base_path` never returns the shared kind
        // directory — so removing an item named like its kind dir (e.g. a
        // rule named `rules`) cannot wipe sibling items.
        let base = target.join(resource_base_path(kind, client, name));
        if base.is_dir() {
            let _ = fs::remove_dir_all(&base);
        }
    }
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

    fn assert_refused(target: &Path, kind: ItemKind, name: &str, source: &Path) {
        let err = check_source_outside_outputs(target, kind, name, source, source)
            .expect_err("overlap must be refused");
        assert!(
            err.to_string().contains("overlaps the output path"),
            "{err}"
        );
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
        check_source_outside_outputs(&target, ItemKind::Skill, "x", &source, &source).unwrap();
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
            assert_eq!(output_path(kind, client, "x"), PathBuf::from(entrypoint));
            assert_eq!(
                resource_base_path(kind, client, "x"),
                PathBuf::from(resources)
            );
        }
    }

    #[test]
    fn raw_separator_names_retain_native_pipeline_layout() {
        assert_eq!(
            output_path(ItemKind::Rule, Client::Claude, "a/b"),
            PathBuf::from(".claude/rules/a/b.md")
        );
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Claude, "a/b"),
            PathBuf::from(".claude/rules/a/a/b")
        );
        #[cfg(unix)]
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Claude, "a\\b"),
            PathBuf::from(".claude/rules/a\\b")
        );
        #[cfg(windows)]
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Claude, "a\\b"),
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
            resource_base_path(ItemKind::Skill, Client::Claude, "x"),
            PathBuf::from(".claude/skills/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::OpenCode, "x"),
            PathBuf::from(".agents/rules/x")
        );
    }

    #[test]
    fn resource_base_flat_is_sibling_namespace_dir() {
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Claude, "x"),
            PathBuf::from(".claude/rules/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Copilot, "x"),
            PathBuf::from(".github/instructions/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Agent, Client::Claude, "x"),
            PathBuf::from(".claude/agents/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Agent, Client::OpenCode, "x"),
            PathBuf::from(".opencode/agents/x")
        );
    }

    #[test]
    fn copy_item_resources_preserves_tree() {
        let src = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(src.path().join("scripts")).unwrap();
        std::fs::write(src.path().join("scripts/gate.sh"), b"#!/bin/sh\n").unwrap();
        let target = tempfile::tempdir().unwrap();

        copy_item_resources(
            target.path(),
            src.path(),
            ItemKind::Rule,
            Client::Claude,
            "demo",
            &[PathBuf::from("scripts/gate.sh")],
        )
        .unwrap();

        let dest = target.path().join(".claude/rules/demo/scripts/gate.sh");
        assert!(dest.is_file());
        assert_eq!(std::fs::read(&dest).unwrap(), b"#!/bin/sh\n");
    }

    #[test]
    fn remove_item_outputs_deletes_flat_kind_resource_dir() {
        let target = tempfile::tempdir().unwrap();
        // Flat-kind (Claude rule) entrypoint + sibling resource namespace dir.
        std::fs::create_dir_all(target.path().join(".claude/rules/demo/scripts")).unwrap();
        std::fs::write(target.path().join(".claude/rules/demo.md"), "x").unwrap();
        std::fs::write(
            target.path().join(".claude/rules/demo/scripts/gate.sh"),
            "x",
        )
        .unwrap();

        remove_item_outputs(target.path(), ItemKind::Rule, "demo");

        assert!(!target.path().join(".claude/rules/demo.md").exists());
        assert!(
            !target.path().join(".claude/rules/demo").exists(),
            "the sibling resource namespace dir must be removed too"
        );
    }

    #[test]
    fn remove_item_outputs_never_wipes_sibling_items_in_shared_dir() {
        let target = tempfile::tempdir().unwrap();
        // Two flat-kind (Claude) rules share `.claude/rules/`. One is named
        // `rules`, matching the shared kind-directory name — the case that
        // previously caused `remove_dir_all(.claude/rules)` to wipe everything.
        fs::create_dir_all(target.path().join(".claude/rules/rules")).unwrap();
        fs::write(target.path().join(".claude/rules/other.md"), "keep").unwrap();
        fs::write(target.path().join(".claude/rules/rules.md"), "x").unwrap();
        fs::write(target.path().join(".claude/rules/rules/g.sh"), "x").unwrap();

        remove_item_outputs(target.path(), ItemKind::Rule, "rules");

        assert!(!target.path().join(".claude/rules/rules.md").exists());
        assert!(!target.path().join(".claude/rules/rules").exists());
        assert!(
            target.path().join(".claude/rules/other.md").exists(),
            "removing the 'rules'-named item must not wipe sibling rules"
        );
    }
}
