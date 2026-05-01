//! Claude Code generation (§7.1).

use crate::model::{Rule, Skill};
use anyhow::{Context, Result};
use serde_yaml_ng::{Mapping, Value};

/// Skill frontmatter per §7.1: `name`, `description`, Agent Skills
/// extended fields pass through.
pub fn skill_frontmatter(skill: &Skill) -> Result<String> {
    super::build_skill_frontmatter(skill, skill.claude.as_ref())
}

/// Rule frontmatter per §7.1: `name`, `description`, `paths:` (array)
/// from `scope.paths` when present.
pub fn rule_frontmatter(rule: &Rule) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(rule.name.clone()));
    map.insert(
        Value::from("description"),
        Value::from(rule.description.clone()),
    );

    if let Some(scope) = &rule.scope {
        if !scope.paths.is_empty() {
            let paths: Vec<Value> = scope.paths.iter().cloned().map(Value::from).collect();
            map.insert(Value::from("paths"), Value::Sequence(paths));
        }
    }

    if let Some(Value::Mapping(pt)) = rule.claude.as_ref() {
        for (k, v) in pt {
            map.insert(k.clone(), v.clone());
        }
    }

    serde_yaml_ng::to_string(&map).context("serializing claude rule frontmatter")
}
