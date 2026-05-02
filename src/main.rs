use anyhow::Context;
use clap::{Parser, Subcommand, error::ErrorKind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use upskill::pipeline::{
    InstallReport, ItemKind, RemoveFilter, RemoveReport, UpdateMode, UpdateReport, UpdateStatus,
    install_with_lockfile, remove, update,
};
use upskill::search;
use upskill::source::parse_install_source;

const EXIT_SUCCESS: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_USAGE: i32 = 2;
const EXIT_INTERRUPTED: i32 = 130;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Parser, Debug)]
#[command(name = "upskill")]
#[command(about = "Author and distribute AI-assistance content across coding agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install rules / skills / agents from a source.
    ///
    /// Runs the v0.2 SSOT generation pipeline: parses each item from the
    /// source, renders per-client output, and records the install in
    /// `.upskill-lock.json`. Per format-spec §7 / ADR-0003.
    Add {
        /// Source: `owner/repo[@ref][:subfolder]`, full https URL, or local path.
        source: String,
        /// Install into `$HOME` instead of the current directory.
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
    /// Remove installed content.
    ///
    /// Either name one or more items, or pass `--source <label>` to
    /// remove every item that came from a single source. Bare
    /// `upskill remove` is rejected — be explicit per ADR-0004. Ancillary
    /// files (`CLAUDE.md`, `opencode.json`, `.vscode/settings.json`) are
    /// not touched.
    Remove {
        /// Item names to remove. Mutually exclusive with `--source`.
        names: Vec<String>,
        /// Remove every item whose lockfile `source` label matches this
        /// string. Use the value reported by the install or shown in
        /// `.upskill-lock.json` (e.g. `local:/path` or
        /// `github:owner/repo`).
        #[arg(long = "source")]
        source: Option<String>,
        /// Operate on `$HOME` instead of the current directory.
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
    /// Pull latest sources and regenerate changed items.
    ///
    /// Re-fetches the source for every (or just the named) lockfile
    /// entries and reinstalls those sources. With `--dry-run`, hashes
    /// the new SSOT and reports what would change without writing.
    /// `update` always fetches per ADR-0004.
    Update {
        /// Item names to update (omit to update everything).
        names: Vec<String>,
        /// Report what would change without writing.
        #[arg(long = "dry-run")]
        dry_run: bool,
        /// Operate on `$HOME` instead of the current directory.
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
    /// List installed content. (Phase B-and-beyond.)
    List,
    /// Verify installed-state consistency. (Phase B3.)
    Doctor,
    /// Search the public skills registry.
    Search {
        /// Search query.
        query: String,
        /// Maximum number of results.
        #[arg(long, default_value = "10")]
        limit: usize,
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
        Commands::Add { source, global } => run_add(&source, global),
        Commands::Remove {
            names,
            source,
            global,
        } => run_remove(&names, source.as_deref(), global),
        Commands::Update {
            names,
            dry_run,
            global,
        } => run_update(&names, dry_run, global),
        Commands::List => unimplemented_stub("list", "B"),
        Commands::Doctor => unimplemented_stub("doctor", "B3"),
        Commands::Search { query, limit } => run_search(&query, limit),
    };

    if was_interrupted() {
        exit_code = EXIT_INTERRUPTED;
    }

    std::process::exit(exit_code);
}

fn install_signal_handlers() -> anyhow::Result<()> {
    ctrlc::set_handler(|| {
        INTERRUPTED.store(true, Ordering::SeqCst);
    })
    .context("failed to install signal handler")
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

fn run_add(source: &str, global: bool) -> i32 {
    let parsed = match parse_install_source(source) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("error: {}", err);
            return EXIT_USAGE;
        }
    };

    let target = match install_target(global) {
        Ok(t) => t,
        Err(err) => {
            eprintln!("error: {}", err);
            return EXIT_ERROR;
        }
    };

    match install_with_lockfile(&parsed, &target) {
        Ok(report) => {
            print_install_report(&report, source);
            EXIT_SUCCESS
        }
        Err(err) => {
            eprintln!("error: {:#}", err);
            EXIT_ERROR
        }
    }
}

fn install_target(global: bool) -> anyhow::Result<PathBuf> {
    if global {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("HOME is not set"))
    } else {
        std::env::current_dir().context("failed to get current directory")
    }
}

fn print_install_report(report: &InstallReport, source: &str) {
    use std::collections::BTreeMap;

    println!("Installed {} files from {}", report.items.len(), source);

    let mut grouped: BTreeMap<(ItemKind, String), Vec<&'static str>> = BTreeMap::new();
    for item in &report.items {
        grouped
            .entry((item.kind, item.name.clone()))
            .or_default()
            .push(item.client.name());
    }
    for ((kind, name), clients) in grouped {
        let kind_label = match kind {
            ItemKind::Rule => "rule ",
            ItemKind::Skill => "skill",
            ItemKind::Agent => "agent",
        };
        println!("  {} {:<32} → {}", kind_label, name, clients.join(", "));
    }
}

fn run_remove(names: &[String], source: Option<&str>, global: bool) -> i32 {
    let filter = match (names.is_empty(), source) {
        (true, Some(s)) => RemoveFilter::BySource(s.to_string()),
        (false, None) => RemoveFilter::ByNames(names.to_vec()),
        (true, None) => {
            eprintln!(
                "error: nothing to remove — pass one or more item names, or `--source <label>`"
            );
            return EXIT_USAGE;
        }
        (false, Some(_)) => {
            eprintln!("error: `--source` and item names are mutually exclusive");
            return EXIT_USAGE;
        }
    };

    let target = match install_target(global) {
        Ok(t) => t,
        Err(err) => {
            eprintln!("error: {}", err);
            return EXIT_ERROR;
        }
    };

    match remove(&target, filter) {
        Ok(report) => {
            print_remove_report(&report);
            EXIT_SUCCESS
        }
        Err(err) => {
            eprintln!("error: {:#}", err);
            EXIT_ERROR
        }
    }
}

fn print_remove_report(report: &RemoveReport) {
    if report.items.is_empty() {
        println!("no matching items in lockfile");
        return;
    }
    println!("Removed {} item(s)", report.items.len());
    for item in &report.items {
        let kind_label = match item.kind {
            ItemKind::Rule => "rule ",
            ItemKind::Skill => "skill",
            ItemKind::Agent => "agent",
        };
        println!(
            "  {} {:<32} ({} file(s))",
            kind_label,
            item.name,
            item.deleted_files.len()
        );
    }
}

fn run_update(names: &[String], dry_run: bool, global: bool) -> i32 {
    let target = match install_target(global) {
        Ok(t) => t,
        Err(err) => {
            eprintln!("error: {}", err);
            return EXIT_ERROR;
        }
    };
    let mode = if dry_run {
        UpdateMode::DryRun
    } else {
        UpdateMode::Apply
    };

    match update(&target, names, mode) {
        Ok(report) => {
            print_update_report(&report, dry_run);
            EXIT_SUCCESS
        }
        Err(err) => {
            eprintln!("error: {:#}", err);
            EXIT_ERROR
        }
    }
}

fn print_update_report(report: &UpdateReport, dry_run: bool) {
    if report.items.is_empty() {
        println!("nothing to update — lockfile is empty");
        return;
    }
    let header = if dry_run {
        "Dry-run: would update"
    } else {
        "Updated"
    };
    let changes = report
        .items
        .iter()
        .filter(|i| !matches!(i.status, UpdateStatus::UpToDate))
        .count();
    println!("{header} {} of {} item(s)", changes, report.items.len());
    for item in &report.items {
        let kind_label = match item.kind {
            ItemKind::Rule => "rule ",
            ItemKind::Skill => "skill",
            ItemKind::Agent => "agent",
        };
        let status = match &item.status {
            UpdateStatus::UpToDate => "up to date".to_string(),
            UpdateStatus::Updated { new_hash, .. } => format!("updated → {}", short(new_hash)),
            UpdateStatus::WouldChange { new_hash, .. } => {
                format!("would change → {}", short(new_hash))
            }
        };
        println!("  {} {:<32} {}", kind_label, item.name, status);
    }
}

fn short(h: &Option<String>) -> String {
    match h {
        Some(s) if s.len() >= 8 => s[..8].to_string(),
        Some(s) => s.clone(),
        None => "<no hash>".to_string(),
    }
}

fn run_search(query: &str, limit: usize) -> i32 {
    match search::search(query, limit) {
        Err(err) => {
            eprintln!("error: {}", err);
            EXIT_ERROR
        }
        Ok(results) if results.is_empty() => {
            println!("no skills found for '{}'", query);
            EXIT_SUCCESS
        }
        Ok(results) => {
            for skill in &results {
                let repo = skill
                    .source
                    .trim_start_matches("github/")
                    .trim_start_matches("gitlab/");
                println!(
                    "{}\t{} installs\tupskill add {} --skill {}",
                    skill.name, skill.installs, repo, skill.name
                );
            }
            EXIT_SUCCESS
        }
    }
}

/// Print a friendly "this command lands in <phase>" message and exit
/// with `EXIT_ERROR`. The CLI surface is final; the implementations land
/// over the rest of Phase B.
fn unimplemented_stub(cmd: &str, phase: &str) -> i32 {
    eprintln!("error: `upskill {cmd}` is not yet implemented (lands in Phase {phase})");
    EXIT_ERROR
}
