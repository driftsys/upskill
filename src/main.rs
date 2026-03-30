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
    },
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Add { source } => run_add(&source),
    };

    std::process::exit(exit_code);
}

fn run_add(source: &str) -> i32 {
    if let Err(err) = ensure_canonical_target() {
        eprintln!("error: {}", err);
        return 1;
    }

    match parse_install_source(source) {
        Ok(InstallSource::Github(repo)) => {
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
