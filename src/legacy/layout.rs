//! Frozen historical generation vocabulary and native path computation.
//!
//! Native path operations preserve the current pipeline's raw-name behavior.
//! Projection validates names before presenting paths as portable strings.

use std::path::PathBuf;

use crate::pipeline::ItemKind;

/// The original generation clients, independent of future adapter defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyClient {
    Claude,
    Copilot,
    OpenCode,
}

impl LegacyClient {
    pub const ALL: [Self; 3] = [Self::Claude, Self::Copilot, Self::OpenCode];

    pub(super) fn from_label(label: &str) -> Option<Self> {
        match label {
            "claude" => Some(Self::Claude),
            "copilot" => Some(Self::Copilot),
            "opencode" => Some(Self::OpenCode),
            _ => None,
        }
    }
}

impl From<crate::generate::Client> for LegacyClient {
    fn from(client: crate::generate::Client) -> Self {
        match client {
            crate::generate::Client::Claude => Self::Claude,
            crate::generate::Client::Copilot => Self::Copilot,
            crate::generate::Client::OpenCode => Self::OpenCode,
        }
    }
}

pub(crate) fn output_path(kind: ItemKind, client: LegacyClient, name: &str) -> PathBuf {
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
pub(crate) fn is_dir_backed(kind: ItemKind, client: LegacyClient) -> bool {
    matches!(
        (kind, client),
        (ItemKind::Skill, _) | (ItemKind::Rule, LegacyClient::OpenCode)
    )
}

/// Directory (relative to the install target) under which an item's
/// supporting resources are copied for `client`. Directory-backed items
/// use the entrypoint's own `<name>/` directory; flat items use a sibling
/// `<name>/` namespace directory next to the flat entrypoint file.
pub(crate) fn resource_base_path(kind: ItemKind, client: LegacyClient, name: &str) -> PathBuf {
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

/// Per format-spec §7 / ADR-0003. opencode `.agents/skills/<n>/SKILL.md`
/// is the canonical-store path; opencode walks it natively.
fn skill_output_path(client: LegacyClient, name: &str) -> PathBuf {
    match client {
        LegacyClient::Claude => PathBuf::from(format!(".claude/skills/{name}/SKILL.md")),
        LegacyClient::Copilot => PathBuf::from(format!(".github/skills/{name}/SKILL.md")),
        LegacyClient::OpenCode => PathBuf::from(format!(".agents/skills/{name}/SKILL.md")),
    }
}

/// Per format-spec §7 / ADR-0003. Copilot uses
/// `<name>.instructions.md`; opencode uses a per-rule directory under
/// `.agents/rules/`.
fn rule_output_path(client: LegacyClient, name: &str) -> PathBuf {
    match client {
        LegacyClient::Claude => PathBuf::from(format!(".claude/rules/{name}.md")),
        LegacyClient::Copilot => {
            PathBuf::from(format!(".github/instructions/{name}.instructions.md"))
        }
        LegacyClient::OpenCode => PathBuf::from(format!(".agents/rules/{name}/RULE.md")),
    }
}

/// Per format-spec §7 / ADR-0003. Copilot uses `<name>.agent.md`.
fn agent_output_path(client: LegacyClient, name: &str) -> PathBuf {
    match client {
        LegacyClient::Claude => PathBuf::from(format!(".claude/agents/{name}.md")),
        LegacyClient::Copilot => PathBuf::from(format!(".github/agents/{name}.agent.md")),
        LegacyClient::OpenCode => PathBuf::from(format!(".opencode/agents/{name}.md")),
    }
}
