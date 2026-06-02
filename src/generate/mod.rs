//! Generation pipeline: SSOT model → per-client markdown string.
//!
//! Pure rendering. No filesystem writes — those land in Phase 3.

use crate::model::{Agent, Rule, Skill};
use anyhow::{Context, Result};
use serde_yaml_ng::{Mapping, Value};

pub mod claude;
pub mod copilot;
pub mod directives;
pub mod format;
pub mod link_rewrite;
pub mod opencode;

/// Target AI coding client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Client {
    Claude,
    Copilot,
    OpenCode,
}

impl Client {
    pub fn name(self) -> &'static str {
        match self {
            Client::Claude => "claude",
            Client::Copilot => "copilot",
            Client::OpenCode => "opencode",
        }
    }
}

impl std::str::FromStr for Client {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "claude" => Ok(Client::Claude),
            "copilot" => Ok(Client::Copilot),
            "opencode" => Ok(Client::OpenCode),
            other => anyhow::bail!("unknown client identifier: {}", other),
        }
    }
}

/// Render a skill to its client-specific SKILL.md string.
///
/// Process directives in `body` for `client`, generate client-specific
/// frontmatter from `skill`, concatenate, and run the result through the
/// dprint formatter for §7.5 idempotence.
pub fn render_skill(skill: &Skill, body: &str, client: Client) -> Result<String> {
    let processed_body = directives::process(body, client)?;
    let frontmatter = match client {
        Client::Claude => claude::skill_frontmatter(skill)?,
        Client::Copilot => copilot::skill_frontmatter(skill)?,
        Client::OpenCode => opencode::skill_frontmatter(skill)?,
    };
    let combined = assemble(&frontmatter, &processed_body);
    format::format_markdown(&combined)
}

/// Render a rule to its client-specific markdown string.
///
/// Per spec §7 / ADR-0003:
/// - Claude: `paths:` array from `scope.paths`.
/// - Copilot: `applyTo:` comma-joined string (default `"**"` if no scope).
/// - opencode: scope is dropped (opencode does not support per-rule
///   path-scoping per spec §3.2). The renderer still produces a string
///   for API symmetry; the install layer (Phase 3) decides whether to
///   write it or just register the SSOT path in `opencode.json`.
pub fn render_rule(rule: &Rule, body: &str, client: Client) -> Result<String> {
    let processed_body = directives::process(body, client)?;
    let frontmatter = match client {
        Client::Claude => claude::rule_frontmatter(rule)?,
        Client::Copilot => copilot::rule_frontmatter(rule)?,
        Client::OpenCode => opencode::rule_frontmatter(rule)?,
    };
    let combined = assemble(&frontmatter, &processed_body);
    format::format_markdown(&combined)
}

/// Render an agent to its client-specific markdown string.
///
/// Per spec §7 / Appendix B:
/// - Claude: `name`, `description`, `model`, `tools` (capitalized),
///   `skills:` (renamed from `preload-skills`). `mode` is implicit by
///   file location and not emitted.
/// - Copilot: `name`, `description`, `model`, `tools` (per §4 mapping
///   where defined, else passthrough). `mode` not emitted.
///   `preload-skills` dropped (no equivalent).
/// - opencode: `description`, `mode` (defaults to `subagent`), `model`,
///   `tools` (lowercase). `name` is in filename per Appendix B and not
///   emitted. `preload-skills` dropped (no equivalent).
pub fn render_agent(agent: &Agent, body: &str, client: Client) -> Result<String> {
    let processed_body = directives::process(body, client)?;
    let frontmatter = match client {
        Client::Claude => claude::agent_frontmatter(agent)?,
        Client::Copilot => copilot::agent_frontmatter(agent)?,
        Client::OpenCode => opencode::agent_frontmatter(agent)?,
    };
    let combined = assemble(&frontmatter, &processed_body);
    format::format_markdown(&combined)
}

/// Combine YAML frontmatter and body into a single markdown string.
fn assemble(frontmatter: &str, body: &str) -> String {
    let body = body.trim_start_matches('\n');
    if body.is_empty() {
        format!("---\n{}---\n", frontmatter)
    } else {
        format!("---\n{}---\n\n{}", frontmatter, body)
    }
}

/// Build the client-shared skill frontmatter mapping.
///
/// Output field order: `name`, `description`, then `Skill.extra` keys
/// (alphabetical via `BTreeMap`), then keys from the matching client
/// passthrough block.
///
/// Stripped per §7 / Appendix B: `schema`, `metadata`, non-matching
/// passthrough blocks, `license`. Only `name`, `description`, Agent
/// Skills extended fields, and the matching passthrough survive.
pub(crate) fn build_skill_frontmatter(
    skill: &Skill,
    passthrough: Option<&Value>,
) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(skill.name.clone()));
    map.insert(
        Value::from("description"),
        Value::from(skill.description.clone()),
    );

    for (k, v) in &skill.extra {
        map.insert(Value::from(k.clone()), v.clone());
    }

    if let Some(Value::Mapping(m)) = passthrough {
        for (k, v) in m {
            map.insert(k.clone(), v.clone());
        }
    }

    serde_yaml_ng::to_string(&map).context("serializing skill frontmatter")
}
