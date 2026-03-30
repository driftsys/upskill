use clap::{Parser, Subcommand};

use upskill::{InstallSource, parse_install_source};

const CANONICAL_TARGET: &str = ".agents/skills";

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
        /// Symlink to Claude Code skills directory
        #[arg(long)]
        claude: bool,
        /// Symlink to Copilot skills directory
        #[arg(long)]
        copilot: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Add {
            source,
            claude,
            copilot,
        } => run_add(&source, claude, copilot),
    };

    std::process::exit(exit_code);
}

fn run_add(source: &str, claude: bool, copilot: bool) -> i32 {
    if let Err(err) = ensure_canonical_target() {
        eprintln!("error: {}", err);
        return 1;
    }

    match parse_install_source(source) {
        Ok(InstallSource::Github(repo)) => {
            if let Err(err) = ensure_agent_symlinks(claude, copilot) {
                eprintln!("error: {}", err);
                return 1;
            }

            println!("install source: github");
            println!("owner: {}", repo.owner);
            println!("repo: {}", repo.name);
            0
        }
        Ok(InstallSource::LocalPath(path)) => {
            if !std::path::Path::new(&path).exists() {
                eprintln!("error: local path does not exist: {}", path);
                return 2;
            }

            if let Err(err) = ensure_agent_symlinks(claude, copilot) {
                eprintln!("error: {}", err);
                return 1;
            }

            println!("install source: local");
            println!("path: {}", path);
            0
        }
        Err(err) => {
            eprintln!("error: {}", err);
            2
        }
    }
}

fn ensure_canonical_target() -> Result<(), String> {
    std::fs::create_dir_all(CANONICAL_TARGET).map_err(|err| {
        format!(
            "failed to create canonical target {}: {}",
            CANONICAL_TARGET, err
        )
    })
}

fn ensure_agent_symlinks(claude: bool, copilot: bool) -> Result<(), String> {
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
