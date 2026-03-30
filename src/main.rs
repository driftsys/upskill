use clap::{Parser, Subcommand};

use upskill::{InstallSource, parse_install_source};

const CANONICAL_TARGET: &str = ".agents/skills";
const AGENT_SKILL_LINKS: [&str; 7] = [
    ".claude/skills",
    ".github/skills",
    ".codex/skills",
    ".cursor/skills",
    ".kiro/skills",
    ".windsurf/skills",
    ".opencode/skills",
];

#[derive(Parser, Debug)]
#[command(name = "upskill")]
#[command(about = "Upskill your coding agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install skills from a source
    Add {
        /// GitHub source in owner/repo format
        source: String,
        /// Install only a specific skill (repeatable)
        #[arg(long = "skill", short = 's')]
        skills: Vec<String>,
        /// Symlink to Claude Code skills directory
        #[arg(long)]
        claude: bool,
        /// Symlink to Copilot skills directory
        #[arg(long)]
        copilot: bool,
        /// Symlink to every supported agent skills directory
        #[arg(long)]
        all: bool,
    },
    /// List installed skills
    List,
    /// Remove an installed skill
    Remove {
        /// Skill name to remove
        skill: String,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Add {
            source,
            skills,
            claude,
            copilot,
            all,
        } => run_add(&source, &skills, claude, copilot, all),
        Commands::List => run_list(),
        Commands::Remove { skill, yes } => run_remove(&skill, yes),
    };

    std::process::exit(exit_code);
}

fn run_add(source: &str, skills: &[String], claude: bool, copilot: bool, all: bool) -> i32 {
    if let Err(err) = ensure_canonical_target() {
        eprintln!("error: {}", err);
        return 1;
    }

    match parse_install_source(source) {
        Ok(InstallSource::Github(repo)) => {
            let source_label = if let Some(subfolder) = &repo.subfolder {
                format!("github:{}/{}:{}", repo.owner, repo.name, subfolder)
            } else {
                format!("github:{}/{}", repo.owner, repo.name)
            };
            let resolved_skills = resolve_requested_skills(skills, &repo.name);

            if let Err(err) = ensure_agent_symlinks(claude, copilot, all) {
                eprintln!("error: {}", err);
                return 1;
            }

            if let Err(err) = persist_installed_skills(&resolved_skills, &source_label) {
                eprintln!("error: {}", err);
                return 1;
            }

            println!("install source: github");
            println!("owner: {}", repo.owner);
            println!("repo: {}", repo.name);
            if let Some(subfolder) = repo.subfolder {
                println!("subfolder: {}", subfolder);
            }
            print_selected_skills(skills);
            0
        }
        Ok(InstallSource::LocalPath(path)) => {
            if !std::path::Path::new(&path).exists() {
                eprintln!("error: local path does not exist: {}", path);
                return 2;
            }

            let default_skill = std::path::Path::new(&path)
                .file_name()
                .and_then(|v| v.to_str())
                .filter(|v| !v.is_empty())
                .unwrap_or("local-skill");
            let resolved_skills = resolve_requested_skills(skills, default_skill);

            if let Err(err) = ensure_agent_symlinks(claude, copilot, all) {
                eprintln!("error: {}", err);
                return 1;
            }

            let source_label = format!("local:{}", path);
            if let Err(err) = persist_installed_skills(&resolved_skills, &source_label) {
                eprintln!("error: {}", err);
                return 1;
            }

            println!("install source: local");
            println!("path: {}", path);
            print_selected_skills(skills);
            0
        }
        Err(err) => {
            eprintln!("error: {}", err);
            2
        }
    }
}

fn run_list() -> i32 {
    let canonical = std::path::Path::new(CANONICAL_TARGET);
    if !canonical.exists() {
        println!("no skills installed");
        return 0;
    }

    let mut skills = Vec::new();
    let entries = match std::fs::read_dir(canonical) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("error: failed to read {}: {}", CANONICAL_TARGET, err);
            return 1;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };

        let source = std::fs::read_to_string(path.join(".upskill-source"))
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string();
        skills.push((name.to_string(), source));
    }

    skills.sort_by(|a, b| a.0.cmp(&b.0));
    if skills.is_empty() {
        println!("no skills installed");
        return 0;
    }

    let active_agents = detect_active_agents();
    let symlink_text = if active_agents.is_empty() {
        "none".to_string()
    } else {
        active_agents.join(",")
    };

    for (name, source) in skills {
        println!("{}\tsource={}\tsymlinks={}", name, source, symlink_text);
    }

    0
}

fn run_remove(skill: &str, yes: bool) -> i32 {
    let skill_path = std::path::Path::new(CANONICAL_TARGET).join(skill);
    if !skill_path.is_dir() {
        eprintln!("error: skill not installed: {}", skill);
        return 2;
    }

    if should_prompt_for_confirmation(yes) && !confirm_removal(skill) {
        eprintln!("error: removal cancelled");
        return 1;
    }

    if let Err(err) = std::fs::remove_dir_all(&skill_path) {
        eprintln!("error: failed to remove {}: {}", skill_path.display(), err);
        return 1;
    }

    if let Err(err) = cleanup_agent_symlinks_if_empty() {
        eprintln!("error: {}", err);
        return 1;
    }

    println!("removed skill: {}", skill);
    0
}

fn should_prompt_for_confirmation(yes: bool) -> bool {
    use std::io::IsTerminal;

    !yes && std::io::stdin().is_terminal()
}

fn confirm_removal(skill: &str) -> bool {
    use std::io::{self, IsTerminal, Write};

    if io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() {
        print!("\u{1b}[33mremove skill '{}' ? [y/N]:\u{1b}[0m ", skill);
    } else {
        print!("remove skill '{}' ? [y/N]: ", skill);
    }
    if io::stdout().flush().is_err() {
        return false;
    }

    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).is_err() {
        return false;
    }

    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn cleanup_agent_symlinks_if_empty() -> Result<(), String> {
    if !canonical_has_skills()? {
        for link in AGENT_SKILL_LINKS {
            remove_symlink_if_exists(link)?;
        }
    }

    Ok(())
}

fn canonical_has_skills() -> Result<bool, String> {
    let canonical = std::path::Path::new(CANONICAL_TARGET);
    if !canonical.exists() {
        return Ok(false);
    }

    let entries = std::fs::read_dir(canonical)
        .map_err(|err| format!("failed to inspect installed skills: {}", err))?;

    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to inspect installed skills: {}", err))?;
        if entry.path().is_dir() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn remove_symlink_if_exists(link_path: &str) -> Result<(), String> {
    let link = std::path::Path::new(link_path);
    match std::fs::symlink_metadata(link) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                std::fs::remove_file(link).map_err(|err| {
                    format!("failed to remove symlink {}: {}", link.display(), err)
                })?;
            }
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to inspect {}: {}", link.display(), err)),
    }
}

fn resolve_requested_skills(skills: &[String], default_skill: &str) -> Vec<String> {
    if skills.is_empty() {
        return vec![default_skill.to_string()];
    }

    skills.to_vec()
}

fn persist_installed_skills(skills: &[String], source: &str) -> Result<(), String> {
    for skill in skills {
        let skill_dir = std::path::Path::new(CANONICAL_TARGET).join(skill);
        std::fs::create_dir_all(&skill_dir)
            .map_err(|err| format!("failed to create {}: {}", skill_dir.display(), err))?;
        std::fs::write(skill_dir.join(".upskill-source"), source)
            .map_err(|err| format!("failed to write source metadata for {}: {}", skill, err))?;
    }

    Ok(())
}

fn detect_active_agents() -> Vec<String> {
    let mut agents = Vec::new();

    if std::fs::symlink_metadata(".claude/skills").is_ok() {
        agents.push("claude".to_string());
    }
    if std::fs::symlink_metadata(".github/skills").is_ok() {
        agents.push("copilot".to_string());
    }
    if std::fs::symlink_metadata(".codex/skills").is_ok() {
        agents.push("codex".to_string());
    }
    if std::fs::symlink_metadata(".cursor/skills").is_ok() {
        agents.push("cursor".to_string());
    }
    if std::fs::symlink_metadata(".kiro/skills").is_ok() {
        agents.push("kiro".to_string());
    }
    if std::fs::symlink_metadata(".windsurf/skills").is_ok() {
        agents.push("windsurf".to_string());
    }
    if std::fs::symlink_metadata(".opencode/skills").is_ok() {
        agents.push("opencode".to_string());
    }

    agents
}

fn print_selected_skills(skills: &[String]) {
    if skills.is_empty() {
        return;
    }

    println!("skills: {}", skills.join(","));
}

fn ensure_canonical_target() -> Result<(), String> {
    std::fs::create_dir_all(CANONICAL_TARGET).map_err(|err| {
        format!(
            "failed to create canonical target {}: {}",
            CANONICAL_TARGET, err
        )
    })
}

fn ensure_agent_symlinks(claude: bool, copilot: bool, all: bool) -> Result<(), String> {
    if all {
        for link in AGENT_SKILL_LINKS {
            create_symlink(link)?;
        }
        return Ok(());
    }

    let auto_detect = !claude && !copilot;

    let link_claude = claude || (auto_detect && std::path::Path::new(".claude").exists());
    let link_copilot = copilot || (auto_detect && std::path::Path::new(".github").exists());

    if link_claude {
        create_symlink(".claude/skills")?;
    }

    if link_copilot {
        create_symlink(".github/skills")?;
    }

    Ok(())
}

fn create_symlink(link_path: &str) -> Result<(), String> {
    let link = std::path::Path::new(link_path);
    let target = std::env::current_dir()
        .map_err(|err| format!("failed to resolve current dir: {}", err))?
        .join(CANONICAL_TARGET);

    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {}", parent.display(), err))?;
    }

    if link.exists() || std::fs::symlink_metadata(link).is_ok() {
        if link.is_dir() && !link.is_symlink() {
            std::fs::remove_dir_all(link)
                .map_err(|err| format!("failed to reset {}: {}", link.display(), err))?;
        } else {
            std::fs::remove_file(link)
                .map_err(|err| format!("failed to reset {}: {}", link.display(), err))?;
        }
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, link).map_err(|err| {
            format!(
                "failed to create symlink {} -> {}: {}",
                link.display(),
                target.display(),
                err
            )
        })?;
    }

    #[cfg(not(unix))]
    {
        let _ = link;
        return Err("symlink creation is currently supported on unix platforms".to_string());
    }

    Ok(())
}
