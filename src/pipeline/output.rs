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
}
