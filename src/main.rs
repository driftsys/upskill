use clap::{Parser, Subcommand, error::ErrorKind};
use std::sync::atomic::{AtomicBool, Ordering};

use upskill::agent;
use upskill::install;
use upskill::source::{InstallSource, parse_install_source};
use upskill::ui;

const EXIT_SUCCESS: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_USAGE: i32 = 2;
const EXIT_INTERRUPTED: i32 = 130;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

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
        /// Copy skills to agent directories instead of creating symlinks
        #[arg(long)]
        copy: bool,
        /// Use user-level global installation target
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
    /// List installed skills
    List {
        /// Read from user-level global installation target
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
    /// Remove an installed skill
    Remove {
        /// Skill name to remove
        skill: String,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
        /// Remove from user-level global installation target
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
}

fn main() {
    if let Err(err) = install_signal_handlers() {
        eprintln!("error: {}", err);
        std::process::exit(EXIT_ERROR);
    }

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let code = map_clap_error(&err);
            let _ = err.print();
            std::process::exit(code);
        }
    };

    let mut exit_code = match cli.command {
        Commands::Add {
            source,
            skills,
            claude,
            copilot,
            all,
            copy,
            global,
        } => run_add(&source, &skills, claude, copilot, all, copy, global),
        Commands::List { global } => run_list(global),
        Commands::Remove { skill, yes, global } => run_remove(&skill, yes, global),
    };

    if was_interrupted() {
        exit_code = EXIT_INTERRUPTED;
    }

    std::process::exit(exit_code);
}

fn install_signal_handlers() -> Result<(), String> {
    ctrlc::set_handler(|| {
        INTERRUPTED.store(true, Ordering::SeqCst);
    })
    .map_err(|err| format!("failed to install signal handler: {}", err))
}

fn was_interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

fn map_clap_error(err: &clap::Error) -> i32 {
    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => EXIT_SUCCESS,
        _ => EXIT_USAGE,
    }
}

fn run_add(
    source: &str,
    skills: &[String],
    claude: bool,
    copilot: bool,
    all: bool,
    copy: bool,
    global: bool,
) -> i32 {
    let canonical_target = match install::canonical_target(global) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("error: {}", err);
            return EXIT_ERROR;
        }
    };

    if let Err(err) = install::ensure_canonical_target(&canonical_target) {
        eprintln!("error: {}", err);
        return EXIT_ERROR;
    }

    match parse_install_source(source) {
        Ok(InstallSource::Github(repo)) => {
            let mut source_label = format!("github:{}/{}", repo.owner, repo.name);
            if let Some(git_ref) = &repo.git_ref {
                source_label.push_str(&format!("@{}", git_ref));
            }
            if let Some(subfolder) = &repo.subfolder {
                source_label.push_str(&format!(":{}", subfolder));
            }
            let resolved_skills = match install::resolve_requested_skills(skills, &repo.name) {
                Ok(skills) => skills,
                Err(err) => {
                    eprintln!("error: {}", err);
                    return EXIT_ERROR;
                }
            };

            if let Err(err) = install::persist_installed_skills(
                &canonical_target,
                &resolved_skills,
                &source_label,
            ) {
                eprintln!("error: {}", err);
                return EXIT_ERROR;
            }

            if !global
                && let Err(err) =
                    agent::ensure_agent_targets(claude, copilot, all, copy, &canonical_target)
            {
                eprintln!("error: {}", err);
                return EXIT_ERROR;
            }

            println!("install source: github");
            println!("owner: {}", repo.owner);
            println!("repo: {}", repo.name);
            if let Some(git_ref) = repo.git_ref {
                println!("ref: {}", git_ref);
            }
            if let Some(subfolder) = repo.subfolder {
                println!("subfolder: {}", subfolder);
            }
            ui::print_selected_skills(&resolved_skills, skills.is_empty());
            EXIT_SUCCESS
        }
        Ok(InstallSource::LocalPath(path)) => {
            if !path.exists() {
                eprintln!("error: local path does not exist: {}", path.display());
                return EXIT_USAGE;
            }

            let default_skill = path
                .file_name()
                .and_then(|v| v.to_str())
                .filter(|v| !v.is_empty())
                .unwrap_or("local-skill");
            let resolved_skills = match install::resolve_requested_skills(skills, default_skill) {
                Ok(skills) => skills,
                Err(err) => {
                    eprintln!("error: {}", err);
                    return EXIT_ERROR;
                }
            };

            let source_label = format!("local:{}", path.display());
            if let Err(err) = install::persist_installed_skills(
                &canonical_target,
                &resolved_skills,
                &source_label,
            ) {
                eprintln!("error: {}", err);
                return EXIT_ERROR;
            }

            if !global
                && let Err(err) =
                    agent::ensure_agent_targets(claude, copilot, all, copy, &canonical_target)
            {
                eprintln!("error: {}", err);
                return EXIT_ERROR;
            }

            println!("install source: local");
            println!("path: {}", path.display());
            ui::print_selected_skills(&resolved_skills, skills.is_empty());
            EXIT_SUCCESS
        }
        Err(err) => {
            eprintln!("error: {}", err);
            EXIT_USAGE
        }
    }
}

fn run_list(global: bool) -> i32 {
    let canonical = match install::canonical_target(global) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("error: {}", err);
            return EXIT_ERROR;
        }
    };

    if !canonical.exists() {
        println!("no skills installed");
        return EXIT_SUCCESS;
    }

    let mut skills = Vec::new();
    let entries = match std::fs::read_dir(&canonical) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("error: failed to read {}: {}", canonical.display(), err);
            return EXIT_ERROR;
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
        return EXIT_SUCCESS;
    }

    let active_agents = agent::detect_active_agents();
    let symlink_text = if active_agents.is_empty() {
        "none".to_string()
    } else {
        active_agents.join(",")
    };

    for (name, source) in skills {
        println!("{}\tsource={}\tsymlinks={}", name, source, symlink_text);
    }

    EXIT_SUCCESS
}

fn run_remove(skill: &str, yes: bool, global: bool) -> i32 {
    let canonical = match install::canonical_target(global) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("error: {}", err);
            return EXIT_ERROR;
        }
    };

    let skill_path = canonical.join(skill);
    if !skill_path.is_dir() {
        eprintln!("error: skill not installed: {}", skill);
        return EXIT_USAGE;
    }

    if ui::should_prompt_for_confirmation(yes) && !ui::confirm_removal(skill) {
        eprintln!("error: removal cancelled");
        return EXIT_ERROR;
    }

    if let Err(err) = std::fs::remove_dir_all(&skill_path) {
        eprintln!("error: failed to remove {}: {}", skill_path.display(), err);
        return EXIT_ERROR;
    }

    if !global && let Err(err) = agent::cleanup_agent_symlinks_if_empty(&canonical) {
        eprintln!("error: {}", err);
        return EXIT_ERROR;
    }

    println!("removed skill: {}", skill);
    EXIT_SUCCESS
}
