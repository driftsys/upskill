//! GitHub Copilot generation (§7.2).

use crate::model::{Rule, Skill};
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
