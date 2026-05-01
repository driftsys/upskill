//! opencode generation (§7.3).

use crate::model::{Rule, Skill};
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
