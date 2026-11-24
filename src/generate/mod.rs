//! Generation pipeline: SSOT model → per-client markdown string.
//!
//! Pure rendering. No filesystem writes — those land in Phase 3.

use crate::model::{Agent, Command, Rule, Skill};
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
    /// Every client, in declaration order. Single source of truth for code
    /// that must enumerate clients (e.g. `ALL_CLIENTS`).
    pub const ALL: [Client; 3] = [Self::Claude, Self::Copilot, Self::OpenCode];

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
pub fn render_skill(skill: &Skill, name: &str, body: &str, client: Client) -> Result<String> {
    let processed_body = directives::process(body, client)?;
    let frontmatter = match client {
        Client::Claude => claude::skill_frontmatter(skill, name)?,
        Client::Copilot => copilot::skill_frontmatter(skill, name)?,
        Client::OpenCode => opencode::skill_frontmatter(skill, name)?,
    };
    let combined = assemble(&frontmatter, &processed_body);
    format::format_markdown(&combined)
}

/// Render a command as a skill that only the user can invoke.
/// Process client directives and format the complete Markdown output.
pub fn render_command(command: &Command, body: &str, client: Client) -> Result<String> {
    let frontmatter = match client {
        Client::Claude => claude::command_frontmatter(command)?,
        Client::Copilot => copilot::command_frontmatter(command)?,
        Client::OpenCode => anyhow::bail!("command items have no opencode output"),
    };
    let processed_body = directives::process(body, client)?;
    format::format_markdown(&assemble(&frontmatter, &processed_body))
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
pub fn render_rule(rule: &Rule, name: &str, body: &str, client: Client) -> Result<String> {
    let processed_body = directives::process(body, client)?;
    let frontmatter = match client {
        Client::Claude => claude::rule_frontmatter(rule, name)?,
        Client::Copilot => copilot::rule_frontmatter(rule, name)?,
        Client::OpenCode => opencode::rule_frontmatter(rule, name)?,
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
///   `preload-skills` rendered as a prose `## Skills` section in the body.
/// - opencode: `description`, `mode` (defaults to `subagent`), `model`,
///   `tools` (lowercase). `name` is in filename per Appendix B and not
///   emitted. `preload-skills` rendered as a prose `## Skills` section in
///   the body.
pub fn render_agent(agent: &Agent, name: &str, body: &str, client: Client) -> Result<String> {
    let processed_body = directives::process(body, client)?;
    let processed_body = match client {
        Client::Claude => processed_body,
        Client::Copilot | Client::OpenCode => {
            append_preload_skills_section(&processed_body, &agent.preload_skills)
        }
    };
    let frontmatter = match client {
        Client::Claude => claude::agent_frontmatter(agent, name)?,
        Client::Copilot => copilot::agent_frontmatter(agent, name)?,
        Client::OpenCode => opencode::agent_frontmatter(agent, name)?,
    };
    let combined = assemble(&frontmatter, &processed_body);
    format::format_markdown(&combined)
}

/// For clients without a native skill-preload mechanism (Copilot,
/// opencode), surface an agent's `preload-skills` as a prose section in
/// the body so the agent is told which skills it relies on. Claude Code
/// emits the native `skills:` frontmatter field instead (see
/// `claude::agent_frontmatter`) and does not get this section. Returns the
/// body unchanged when there are no preload skills.
fn append_preload_skills_section(body: &str, preload_skills: &[String]) -> String {
    if preload_skills.is_empty() {
        return body.to_string();
    }
    let mut out = body.trim_end().to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str("## Skills\n\nThis agent relies on the following skills, installed alongside it. Use them when relevant:\n\n");
    for s in preload_skills {
        out.push_str(&format!("- {s}\n"));
    }
    out
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
    name: &str,
    passthrough: Option<&Value>,
) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(name.to_string()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Agent;
    use crate::model::common::SchemaVersion;
    use std::collections::BTreeMap;

    fn command() -> crate::model::Command {
        serde_yaml_ng::from_str("name: release-notes\ndescription: Write notes\nargument-hint: '[version]'\nlicense: MIT\nmetadata: {version: '1'}\nignore: [draft.md]\n").unwrap()
    }

    #[test]
    fn render_command_writes_the_claude_keys_in_order() {
        let out = render_command(&command(), "Write $ARGUMENTS.\n", Client::Claude).unwrap();
        let (yaml, body) = crate::parse::frontmatter::split(&out).unwrap();
        let map: Mapping = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(
            map.keys().map(|k| k.as_str().unwrap()).collect::<Vec<_>>(),
            [
                "name",
                "description",
                "argument-hint",
                "disable-model-invocation"
            ]
        );
        assert_eq!(map["argument-hint"].as_str(), Some("[version]"));
        assert_eq!(map["disable-model-invocation"].as_bool(), Some(true));
        assert!(body.contains("$ARGUMENTS"));
        let mut command = command();
        command.argument_hint = None;
        let out = render_command(&command, "Write notes.\n", Client::Claude).unwrap();
        assert!(!out.contains("argument-hint:"));
    }

    #[test]
    fn render_command_omits_argument_hint_for_copilot() {
        let out = render_command(&command(), "Write $ARGUMENTS.\n", Client::Copilot).unwrap();
        let (yaml, _) = crate::parse::frontmatter::split(&out).unwrap();
        let map: Mapping = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(
            map.keys().map(|k| k.as_str().unwrap()).collect::<Vec<_>>(),
            ["name", "description", "disable-model-invocation"]
        );
        assert_eq!(map["disable-model-invocation"].as_bool(), Some(true));
    }

    #[test]
    fn render_command_has_no_opencode_output() {
        assert_eq!(
            render_command(&command(), "Write notes.\n", Client::OpenCode)
                .unwrap_err()
                .to_string(),
            "command items have no opencode output"
        );
    }

    #[test]
    fn render_rule_uses_passed_name_not_frontmatter() {
        let rule = Rule {
            schema: SchemaVersion::new(1).unwrap(),
            name: None,
            description: "A baseline security ruleset.".to_string(),
            audience: None,
            license: None,
            scope: None,
            metadata: None,
            requires: Default::default(),
            ignore: vec![],
            claude: None,
            copilot: None,
            opencode: None,
            extra: BTreeMap::new(),
        };
        let out = render_rule(&rule, "security-baseline", "## Body\n", Client::Claude).unwrap();
        assert!(
            out.contains("name: security-baseline"),
            "expected passed name in output, got:\n{out}"
        );
    }

    fn make_agent_with_preload(preload_skills: Vec<String>) -> Agent {
        Agent {
            schema: SchemaVersion::new(1).unwrap(),
            name: Some("test-agent".to_string()),
            description: "Test agent.".to_string(),
            audience: None,
            license: None,
            mode: None,
            model: None,
            tools: vec![],
            preload_skills,
            metadata: None,
            requires: Default::default(),
            ignore: vec![],
            claude: None,
            copilot: None,
            opencode: None,
            extra: BTreeMap::new(),
        }
    }

    #[test]
    fn preload_skills_rendered_as_prose_for_copilot_and_opencode() {
        let agent = make_agent_with_preload(vec!["a-skill".to_string()]);
        let body = "## Instructions\n\nDo stuff.\n";

        for client in [Client::Copilot, Client::OpenCode] {
            let out = render_agent(&agent, "test-agent", body, client).unwrap();
            assert!(
                out.contains("## Skills"),
                "{client:?}: expected '## Skills' section, got:\n{out}"
            );
            assert!(
                out.contains("- a-skill"),
                "{client:?}: expected '- a-skill' in output, got:\n{out}"
            );
        }
    }

    #[test]
    fn preload_skills_not_in_body_for_claude() {
        let agent = make_agent_with_preload(vec!["a-skill".to_string()]);
        let body = "## Instructions\n\nDo stuff.\n";
        let out = render_agent(&agent, "test-agent", body, Client::Claude).unwrap();
        assert!(
            !out.contains("## Skills"),
            "Claude: expected NO '## Skills' body section (uses native skills: field), got:\n{out}"
        );
        assert!(
            out.contains("skills:"),
            "Claude: expected native 'skills:' frontmatter field, got:\n{out}"
        );
    }

    #[test]
    fn empty_preload_skills_produces_no_skills_section() {
        let agent = make_agent_with_preload(vec![]);
        let body = "## Instructions\n\nDo stuff.\n";
        for client in [Client::Claude, Client::Copilot, Client::OpenCode] {
            let out = render_agent(&agent, "test-agent", body, client).unwrap();
            assert!(
                !out.contains("## Skills"),
                "{client:?}: expected NO '## Skills' section for empty preload, got:\n{out}"
            );
        }
    }
}
