//! Claude Code generation (§7.1).

use crate::model::{Agent, Rule, Skill, ToolCap};
use anyhow::{Context, Result};
use serde_yaml_ng::{Mapping, Value};

/// Skill frontmatter per §7.1: `name`, `description`, Agent Skills
/// extended fields pass through.
pub fn skill_frontmatter(skill: &Skill, name: &str) -> Result<String> {
    super::build_skill_frontmatter(skill, name, skill.claude.as_ref())
}

/// Rule frontmatter per §7.1: `name`, `description`, `paths:` (array)
/// from `scope.paths` when present.
pub fn rule_frontmatter(rule: &Rule, name: &str) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(name.to_string()));
    map.insert(
        Value::from("description"),
        Value::from(rule.description.clone()),
    );

    if let Some(scope) = &rule.scope
        && !scope.paths.is_empty()
    {
        let paths: Vec<Value> = scope.paths.iter().cloned().map(Value::from).collect();
        map.insert(Value::from("paths"), Value::Sequence(paths));
    }

    if let Some(Value::Mapping(pt)) = rule.claude.as_ref() {
        for (k, v) in pt {
            map.insert(k.clone(), v.clone());
        }
    }

    serde_yaml_ng::to_string(&map).context("serializing claude rule frontmatter")
}

/// Agent frontmatter per §7.1 / Appendix B: `name`, `description`,
/// `model:` (literal alias), `tools:` (capitalized), `skills:` (from
/// `preload-skills`). `mode` is implicit by file location and not
/// emitted.
pub fn agent_frontmatter(agent: &Agent, name: &str) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(name.to_string()));
    map.insert(
        Value::from("description"),
        Value::from(agent.description.clone()),
    );

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

    if !agent.preload_skills.is_empty() {
        let skills: Vec<Value> = agent
            .preload_skills
            .iter()
            .cloned()
            .map(Value::from)
            .collect();
        map.insert(Value::from("skills"), Value::Sequence(skills));
    }

    if let Some(Value::Mapping(pt)) = agent.claude.as_ref() {
        for (k, v) in pt {
            map.insert(k.clone(), v.clone());
        }
    }

    serde_yaml_ng::to_string(&map).context("serializing claude agent frontmatter")
}

/// Map a capability-level tool name to Claude's capitalized form.
fn map_tool(t: ToolCap) -> &'static str {
    match t {
        ToolCap::Read => "Read",
        ToolCap::Write => "Write",
        ToolCap::Edit => "Edit",
        ToolCap::Bash => "Bash",
        ToolCap::Grep => "Grep",
        ToolCap::Glob => "Glob",
        ToolCap::WebFetch => "WebFetch",
        ToolCap::WebSearch => "WebSearch",
    }
}
