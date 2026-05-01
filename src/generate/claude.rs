//! Claude Code generation (§7.1).

use crate::model::Skill;
use anyhow::Result;

/// Skill frontmatter per §7.1: `name`, `description`, Agent Skills
/// extended fields pass through.
pub fn skill_frontmatter(skill: &Skill) -> Result<String> {
    super::build_skill_frontmatter(skill, skill.claude.as_ref())
}
