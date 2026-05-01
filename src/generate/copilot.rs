//! GitHub Copilot generation (§7.2).

use crate::model::Skill;
use anyhow::Result;

/// Skill frontmatter per §7.2: `name`, `description`, follows Agent
/// Skills open standard.
pub fn skill_frontmatter(skill: &Skill) -> Result<String> {
    super::build_skill_frontmatter(skill, skill.copilot.as_ref())
}
