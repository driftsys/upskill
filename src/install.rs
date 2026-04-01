use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::ui;

const PROJECT_CANONICAL_TARGET: &str = ".agents/skills";
const GLOBAL_CANONICAL_TARGET: &str = ".agents/skills";

pub fn canonical_target(global: bool) -> Result<PathBuf> {
    if global {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
        return Ok(home.join(GLOBAL_CANONICAL_TARGET));
    }

    Ok(PathBuf::from(PROJECT_CANONICAL_TARGET))
}

pub fn ensure_canonical_target(canonical_target: &Path) -> Result<()> {
    std::fs::create_dir_all(canonical_target).with_context(|| {
        format!(
            "failed to create canonical target {}",
            canonical_target.display(),
        )
    })
}

pub fn persist_installed_skills(
    canonical_target: &Path,
    skills: &[String],
    source: &str,
) -> Result<()> {
    for skill in skills {
        let skill_dir = canonical_target.join(skill);
        std::fs::create_dir_all(&skill_dir)
            .with_context(|| format!("failed to create {}", skill_dir.display()))?;
        std::fs::write(skill_dir.join(".upskill-source"), source)
            .with_context(|| format!("failed to write source metadata for {}", skill))?;
    }

    Ok(())
}

pub fn resolve_requested_skills(skills: &[String], default_skill: &str) -> Result<Vec<String>> {
    if !skills.is_empty() {
        return Ok(skills.to_vec());
    }

    if !ui::interactive_skill_selection_enabled() {
        return Ok(vec![default_skill.to_string()]);
    }

    ui::prompt_for_skill_selection(default_skill)
}
