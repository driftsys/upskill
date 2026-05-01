//! opencode generation (§7.3).

use crate::model::{Agent, Mode, Rule, Skill, ToolCap};
use anyhow::{Context, Result};
use serde_yaml_ng::{Mapping, Value};

/// Skill frontmatter per §7.3. opencode walks `.agents/skills/` natively;
/// for the rendering layer we still produce the canonical content with
/// the `opencode.*` passthrough merged.
pub fn skill_frontmatter(skill: &Skill) -> Result<String> {
    super::build_skill_frontmatter(skill, skill.opencode.as_ref())
}

/// Rule frontmatter per §7.3 / ADR-0003. opencode rules are referenced
/// via a path entry in `opencode.json` `instructions[]`; no dedicated
/// per-client file is generated.
///
/// For API symmetry with the other clients, this function still returns
/// a string. The install layer (Phase 3) decides whether to write it or
/// just register the SSOT path. `scope.paths` is silently dropped per
/// spec §3.2 (opencode does not support per-rule path-scoping).
pub fn rule_frontmatter(rule: &Rule) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(rule.name.clone()));
    map.insert(
        Value::from("description"),
        Value::from(rule.description.clone()),
    );

    if let Some(Value::Mapping(pt)) = rule.opencode.as_ref() {
        for (k, v) in pt {
            map.insert(k.clone(), v.clone());
        }
    }

    serde_yaml_ng::to_string(&map).context("serializing opencode rule frontmatter")
}

/// Agent frontmatter per §7.3 / Appendix B: `description`, `mode`,
/// `model`, `tools` (lowercase). `name` is in filename per Appendix B
/// and not emitted. `preload-skills` has no opencode equivalent and is
/// dropped. `mode` defaults to `subagent` per spec §3.4 when absent.
pub fn agent_frontmatter(agent: &Agent) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(
        Value::from("description"),
        Value::from(agent.description.clone()),
    );

    let mode = agent.mode.unwrap_or(Mode::Subagent);
    map.insert(Value::from("mode"), Value::from(mode_str(mode)));

    if let Some(model) = &agent.model {
        map.insert(Value::from("model"), Value::from(model.clone()));
    }

    if !agent.tools.is_empty() {
        let tools: Vec<Value> = agent
            .tools
            .iter()
            .map(|t| Value::from(map_tool(*t)))
            .collect();
        map.insert(Value::from("tools"), Value::Sequence(tools));
    }

    if let Some(Value::Mapping(pt)) = agent.opencode.as_ref() {
        for (k, v) in pt {
            map.insert(k.clone(), v.clone());
        }
    }

    serde_yaml_ng::to_string(&map).context("serializing opencode agent frontmatter")
}

fn mode_str(m: Mode) -> &'static str {
    match m {
        Mode::Primary => "primary",
        Mode::Subagent => "subagent",
        Mode::All => "all",
    }
}

/// Map a capability-level tool name to opencode's lowercase form.
///
/// Per spec §4: opencode has no documented mapping for `web-fetch` or
/// `web-search` (both `—`). Pass them through as kebab-case identifiers
/// rather than dropping them — opencode's tool registry may add support
/// later, and dropping silently would surprise authors.
fn map_tool(t: ToolCap) -> &'static str {
    match t {
        ToolCap::Read => "read",
        ToolCap::Write => "write",
        ToolCap::Edit => "edit",
        ToolCap::Bash => "bash",
        ToolCap::Grep => "grep",
        ToolCap::Glob => "glob",
        ToolCap::WebFetch => "web-fetch",
        ToolCap::WebSearch => "web-search",
    }
}
