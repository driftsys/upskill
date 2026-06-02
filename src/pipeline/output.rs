//! Per-client output path computation and file I/O.
//!
//! The install pipeline writes (and `remove` deletes) per-client output
//! files at the paths defined in format-spec §7 / ADR-0003. Centralising
//! the path mapping here keeps install and remove in lockstep — they
//! both go through [`output_path`].

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
    match kind {
        ItemKind::Skill => skill_output_path(client, name),
        ItemKind::Rule => rule_output_path(client, name),
        ItemKind::Agent => agent_output_path(client, name),
    }
}

/// True when an item's entrypoint for `client` lives inside its own
/// `<name>/` directory (all skills; opencode rules). Such items hold
/// resources beside the entrypoint and need no link rewrite. Flat kinds
/// (Claude/Copilot rules, all agents) return false.
pub(super) fn is_dir_backed(kind: ItemKind, client: Client) -> bool {
    matches!(
        (kind, client),
        (ItemKind::Skill, _) | (ItemKind::Rule, Client::OpenCode)
    )
}

/// Directory (relative to the install target) under which an item's
/// supporting resources are copied for `client`. Directory-backed items
/// use the entrypoint's own `<name>/` directory; flat items use a sibling
/// `<name>/` namespace directory next to the flat entrypoint file.
pub(super) fn resource_base_path(kind: ItemKind, client: Client, name: &str) -> PathBuf {
    let entry = output_path(kind, client, name);
    let parent = entry
        .parent()
        .expect("output path always has a parent directory");
    if is_dir_backed(kind, client) {
        parent.to_path_buf()
    } else {
        parent.join(name)
    }
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

pub(super) fn write_output(target: &Path, rel: &Path, content: &str) -> Result<()> {
    let full = target.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create dir {}", parent.display()))?;
    }
    fs::write(&full, content).with_context(|| format!("write {}", full.display()))?;
    Ok(())
}

/// Delete all per-client output files for an item (best-effort).
pub(super) fn remove_item_outputs(target: &Path, kind: ItemKind, name: &str) {
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
}
