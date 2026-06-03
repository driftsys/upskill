//! opencode generation (§7.3).

use crate::model::{Agent, Mode, Rule, Skill, ToolCap};
use anyhow::{Context, Result};
use serde_yaml_ng::{Mapping, Value};

/// Skill frontmatter per §7.3. opencode walks `.agents/skills/` natively;
/// for the rendering layer we still produce the canonical content with
/// the `opencode.*` passthrough merged.
pub fn skill_frontmatter(skill: &Skill, name: &str) -> Result<String> {
    super::build_skill_frontmatter(skill, name, skill.opencode.as_ref())
}

/// Rule frontmatter per §7.3 / ADR-0003. opencode rules are referenced
/// via a path entry in `opencode.json` `instructions[]`; no dedicated
/// per-client file is generated.
///
/// For API symmetry with the other clients, this function still returns
/// a string. The install layer (Phase 3) decides whether to write it or
/// just register the SSOT path. `scope.paths` is silently dropped per
/// spec §3.2 (opencode does not support per-rule path-scoping).
pub fn rule_frontmatter(rule: &Rule, name: &str) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(name.to_string()));
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

/// Agent frontmatter per §7.3 — emits `name`, `description`, `mode`,
/// `model`, `permission` map. `name` is emitted explicitly for cross-kind
/// consistency with skills/rules and to support bookkeeping
/// (`upskill list`/`doctor`, human readability), even though Appendix B
/// originally suggested filename-only; opencode tolerates the redundant
/// field. `preload-skills` has no opencode equivalent and is dropped.
/// `mode` defaults to `subagent` per spec §3.4 when absent.
pub fn agent_frontmatter(agent: &Agent, name: &str) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(name.to_string()));
    map.insert(
        Value::from("description"),
        Value::from(agent.description.clone()),
    );

    let mode = agent.mode.unwrap_or(Mode::Subagent);
    map.insert(Value::from("mode"), Value::from(mode_str(mode)));

    if let Some(model) = &agent.model {
        map.insert(Value::from("model"), Value::from(model.clone()));
    }

    let mut perms = Mapping::new();
    for tool in &agent.tools {
        let key = Value::from(permission_key(*tool));
        if !perms.contains_key(&key) {
            perms.insert(key, Value::from("allow"));
        }
    }
    if !perms.is_empty() {
        map.insert(Value::from("permission"), Value::Mapping(perms));
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

/// Map a capability-level tool name to opencode's `permission:` map key.
///
/// Per opencode docs (https://opencode.ai/docs/agents), agents declare
/// allowed capabilities via a `permission:` map (allow|ask|deny per key)
/// rather than a `tools:` array. `write` rolls into `edit` since opencode
/// has no separate write key. Other opencode-only permission keys
/// (`task`, `external_directory`, `todowrite`, `lsp`, `skill`, `list`)
/// have no neutral capability — authors who need them use the
/// `opencode:` passthrough block.
fn permission_key(t: ToolCap) -> &'static str {
    match t {
        ToolCap::Read => "read",
        ToolCap::Write => "edit",
        ToolCap::Edit => "edit",
        ToolCap::Bash => "bash",
        ToolCap::Grep => "grep",
        ToolCap::Glob => "glob",
        ToolCap::WebFetch => "webfetch",
        ToolCap::WebSearch => "websearch",
    }
}
