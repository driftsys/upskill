//! opencode generation (§7.3).

use crate::model::Skill;
use anyhow::Result;

/// Skill frontmatter per §7.3. opencode walks `.agents/skills/` natively;
/// for the rendering layer we still produce the canonical content with
/// the `opencode.*` passthrough merged.
pub fn skill_frontmatter(skill: &Skill) -> Result<String> {
    super::build_skill_frontmatter(skill, skill.opencode.as_ref())
}
