use anyhow::{Context, Result};
use std::path::Path;

struct AgentDef {
    name: &'static str,
    skill_link: &'static str,
}

const AGENT_DEFS: [AgentDef; 7] = [
    AgentDef {
        name: "claude",
        skill_link: ".claude/skills",
    },
    AgentDef {
        name: "copilot",
        skill_link: ".github/skills",
    },
    AgentDef {
        name: "codex",
        skill_link: ".codex/skills",
    },
    AgentDef {
        name: "cursor",
        skill_link: ".cursor/skills",
    },
    AgentDef {
        name: "kiro",
        skill_link: ".kiro/skills",
    },
    AgentDef {
        name: "windsurf",
        skill_link: ".windsurf/skills",
    },
    AgentDef {
        name: "opencode",
        skill_link: ".opencode/skills",
    },
];

pub fn all_skill_links() -> impl Iterator<Item = &'static str> {
    AGENT_DEFS.iter().map(|a| a.skill_link)
}

pub fn detect_active_agents() -> Vec<String> {
    AGENT_DEFS
        .iter()
        .filter(|a| std::fs::symlink_metadata(a.skill_link).is_ok())
        .map(|a| a.name.to_string())
        .collect()
}

pub fn ensure_agent_targets(
    claude: bool,
    copilot: bool,
    all: bool,
    copy: bool,
    canonical_target: &Path,
) -> Result<()> {
    if all {
        for link in all_skill_links() {
            create_agent_target(link, copy, canonical_target)?;
        }
        return Ok(());
    }

    let auto_detect = !claude && !copilot;

    if auto_detect {
        for agent in &AGENT_DEFS {
            let parent = Path::new(agent.skill_link)
                .parent()
                .unwrap_or(Path::new("."));
            if parent.exists() {
                create_agent_target(agent.skill_link, copy, canonical_target)?;
            }
        }
    } else {
        if claude {
            create_agent_target(".claude/skills", copy, canonical_target)?;
        }
        if copilot {
            create_agent_target(".github/skills", copy, canonical_target)?;
        }
    }

    Ok(())
}

pub fn cleanup_agent_symlinks_if_empty(canonical: &Path) -> Result<()> {
    if !canonical_has_skills(canonical)? {
        for link in all_skill_links() {
            remove_symlink_if_exists(link)?;
        }
    }

    Ok(())
}

fn canonical_has_skills(canonical: &Path) -> Result<bool> {
    if !canonical.exists() {
        return Ok(false);
    }

    let entries = std::fs::read_dir(canonical).with_context(|| {
        format!(
            "failed to inspect installed skills in {}",
            canonical.display()
        )
    })?;

    for entry in entries {
        let entry = entry.context("failed to inspect installed skills")?;
        if entry.path().is_dir() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn remove_symlink_if_exists(link_path: &str) -> Result<()> {
    let link = Path::new(link_path);
    match std::fs::symlink_metadata(link) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                std::fs::remove_file(link)
                    .with_context(|| format!("failed to remove symlink {}", link.display()))?;
            }
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("failed to inspect {}", link.display())),
    }
}

fn create_agent_target(link_path: &str, copy: bool, canonical_target: &Path) -> Result<()> {
    let link = Path::new(link_path);

    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    if link.exists() || std::fs::symlink_metadata(link).is_ok() {
        if link.is_dir() && !link.is_symlink() {
            std::fs::remove_dir_all(link)
                .with_context(|| format!("failed to reset {}", link.display()))?;
        } else {
            std::fs::remove_file(link)
                .with_context(|| format!("failed to reset {}", link.display()))?;
        }
    }

    if copy {
        copy_dir_all(canonical_target, link)?;
        return Ok(());
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(canonical_target, link).with_context(|| {
            format!(
                "failed to create symlink {} -> {}",
                link.display(),
                canonical_target.display(),
            )
        })?;
    }

    #[cfg(not(unix))]
    {
        let _ = link;
        anyhow::bail!("symlink creation is currently supported on unix platforms");
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("failed to create {}", dst.display()))?;

    let entries =
        std::fs::read_dir(src).with_context(|| format!("failed to read {}", src.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("failed to read {}", src.display()))?;
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_all(&entry_path, &target_path)?;
        } else {
            std::fs::copy(&entry_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    entry_path.display(),
                    target_path.display(),
                )
            })?;
        }
    }

    Ok(())
}
