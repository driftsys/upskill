//! GitHub Copilot generation (§7.2).

use crate::model::{Agent, Rule, Skill, ToolCap};
use anyhow::{Context, Result};
use serde_yaml_ng::{Mapping, Value};

/// Skill frontmatter per §7.2: `name`, `description`, follows Agent
/// Skills open standard.
pub fn skill_frontmatter(skill: &Skill) -> Result<String> {
    super::build_skill_frontmatter(skill, skill.copilot.as_ref())
}

/// Rule frontmatter per §7.2: `name`, `description`, `applyTo:`
/// (comma-joined from `scope.paths`, default `"**"`).
pub fn rule_frontmatter(rule: &Rule) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(rule.name.clone()));
    map.insert(
        Value::from("description"),
        Value::from(rule.description.clone()),
    );

    let apply_to = match &rule.scope {
        Some(scope) if !scope.paths.is_empty() => scope.paths.join(", "),
        _ => "**".to_string(),
    };
    map.insert(Value::from("applyTo"), Value::from(apply_to));

    if let Some(Value::Mapping(pt)) = rule.copilot.as_ref() {
        for (k, v) in pt {
            map.insert(k.clone(), v.clone());
        }
    }

    serde_yaml_ng::to_string(&map).context("serializing copilot rule frontmatter")
}

/// Agent frontmatter per §7.2: `name`, `description`, `model`, `tools`.
/// `preload-skills` has no Copilot equivalent and is dropped. `mode` is
/// not emitted (not in the §7.2 field list).
pub fn agent_frontmatter(agent: &Agent) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(agent.name.clone()));
    map.insert(
        Value::from("description"),
        Value::from(agent.description.clone()),
    );

    if let Some(model) = &agent.model {
        map.insert(Value::from("model"), Value::from(model.clone()));
    }

    let tools: Vec<Value> = agent
        .tools
        .iter()
        .filter_map(|t| map_tool(*t))
        .map(Value::from)
        .collect();
    if !tools.is_empty() {
        map.insert(Value::from("tools"), Value::Sequence(tools));
    }

    if let Some(Value::Mapping(pt)) = agent.copilot.as_ref() {
        for (k, v) in pt {
            map.insert(k.clone(), v.clone());
        }
    }

    serde_yaml_ng::to_string(&map).context("serializing copilot agent frontmatter")
}

/// Map a capability-level tool name to Copilot's form.
///
/// Per spec §4, Copilot only has documented mappings for `bash → shell`,
/// `web-fetch → fetch`, and `web-search → web_search`. Capabilities
/// without a documented mapping (`read`, `write`, `edit`, `grep`, `glob`)
/// return `None` and are dropped silently — emitting them as kebab-case
/// would produce fields Copilot does not consume. Authors needing
/// Copilot-specific tool names can use the `copilot:` passthrough block.
fn map_tool(t: ToolCap) -> Option<&'static str> {
    match t {
        ToolCap::Bash => Some("shell"),
        ToolCap::WebFetch => Some("fetch"),
        ToolCap::WebSearch => Some("web_search"),
        ToolCap::Read | ToolCap::Write | ToolCap::Edit | ToolCap::Grep | ToolCap::Glob => None,
    }
}
