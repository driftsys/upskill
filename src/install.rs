use std::path::{Path, PathBuf};

use crate::ui;

const PROJECT_CANONICAL_TARGET: &str = ".agents/skills";
const GLOBAL_CANONICAL_TARGET: &str = ".agents/skills";

pub fn canonical_target(global: bool) -> Result<PathBuf, String> {
    if global {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set".to_string())?;
        return Ok(home.join(GLOBAL_CANONICAL_TARGET));
    }

    Ok(PathBuf::from(PROJECT_CANONICAL_TARGET))
}

pub fn ensure_canonical_target(canonical_target: &Path) -> Result<(), String> {
    std::fs::create_dir_all(canonical_target).map_err(|err| {
        format!(
            "failed to create canonical target {}: {}",
            canonical_target.display(),
            err
        )
    })
}

pub fn persist_installed_skills(
    canonical_target: &Path,
    skills: &[String],
    source: &str,
) -> Result<(), String> {
    for skill in skills {
        let skill_dir = canonical_target.join(skill);
        std::fs::create_dir_all(&skill_dir)
            .map_err(|err| format!("failed to create {}: {}", skill_dir.display(), err))?;
        std::fs::write(skill_dir.join(".upskill-source"), source)
            .map_err(|err| format!("failed to write source metadata for {}: {}", skill, err))?;
    }

    Ok(())
}

pub fn resolve_requested_skills(
    skills: &[String],
    default_skill: &str,
) -> Result<Vec<String>, String> {
    if !skills.is_empty() {
        return Ok(skills.to_vec());
    }

    if !ui::interactive_skill_selection_enabled() {
        return Ok(vec![default_skill.to_string()]);
    }

    ui::prompt_for_skill_selection(default_skill)
}
