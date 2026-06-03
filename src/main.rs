use anyhow::Context;
use clap::{Parser, error::ErrorKind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use upskill::cli::{Cli, Commands};
use upskill::config;
use upskill::fmt::{FmtReport, fmt};
use upskill::index;
use upskill::lint::{LintReport, lint};
use upskill::pipeline::{
    DoctorReport, InstallReport, ItemKind, ListReport, ListedBundle, ListedItem, OrphanReason,
    PluginResult, RemoveFilter, RemoveReport, UpdateMode, UpdateReport, UpdateStatus, doctor,
    install_with_lockfile, list, remove, update,
};
use upskill::plugin::PluginScope;
use upskill::scaffold::{NewKind, ScaffoldReport, scaffold};
use upskill::search;
use upskill::source::{InstallSource, home_dir, parse_install_source};
use upskill::style;

const EXIT_SUCCESS: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_USAGE: i32 = 2;
const EXIT_INTERRUPTED: i32 = 130;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let code = map_clap_error(&err);
            let _ = err.print();
            std::process::exit(code);
        }
    };

    style::init(cli.no_color);
    style::set_quiet(cli.quiet);

    if let Err(err) = install_signal_handlers() {
        print_error(&err);
        std::process::exit(EXIT_ERROR);
    }

    let mut exit_code = match cli.command {
        Commands::Add {
            source,
            items,
            global,
            project,
            force,
            alias,
            exclude,
        } => run_add(&source, &items, global, project, force, &alias, &exclude),
        Commands::Remove {
            names,
            source,
            global,
            project,
            yes,
        } => run_remove(&names, source.as_deref(), global, project, yes),
        Commands::Update {
            names,
            dry_run,
            global,
            project,
            yes,
        } => run_update(&names, dry_run, yes, global, project),
        Commands::List {
            global,
            project,
            json,
        } => run_list(global, project, json),
        Commands::Doctor {
            global,
            project,
            json,
        } => run_doctor(global, project, json),
        Commands::Search {
            query,
            limit,
            registry,
            kind,
        } => run_search(&query, limit, registry.as_deref(), kind.as_deref()),
        Commands::Lint { paths, strict } => run_lint(&paths, strict),
        Commands::Fmt { paths } => run_fmt(&paths),
        Commands::New { kind, name } => run_new(&kind, &name),
        Commands::Index { registry, clear } => run_index(registry.as_deref(), clear),
    };

    if was_interrupted() {
        exit_code = EXIT_INTERRUPTED;
    }

    std::process::exit(exit_code);
}

fn install_signal_handlers() -> anyhow::Result<()> {
    ctrlc::set_handler(|| {
        // Flag first, then notify. Setting INTERRUPTED before the print
        // ensures the `was_interrupted()` check in main() sees true even
        // if the eprintln races with process teardown.
        INTERRUPTED.store(true, Ordering::SeqCst);
        eprintln!("\n{} cleaning up", style::warn("interrupted:"));
        eprintln!("  hint: run 'upskill doctor' to check for partial installs");
    })
    .context("failed to install signal handler")
}

fn was_interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Print `error: <message>` to stderr, with the label colored when allowed
/// by the disable chain. Centralises the convention so every error site
/// uses the same shape.
fn print_error(err: impl std::fmt::Display) {
    eprintln!("{} {err}", style::error_label("error:"));
}

/// Like [`print_error`] but uses the `:#` formatter to print the full
/// anyhow context chain (root cause last).
fn print_error_chain(err: &anyhow::Error) {
    eprintln!("{} {err:#}", style::error_label("error:"));
}

fn map_clap_error(err: &clap::Error) -> i32 {
    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => EXIT_SUCCESS,
        _ => EXIT_USAGE,
    }
}

fn run_add(
    source: &str,
    items: &[String],
    global: bool,
    project: bool,
    force: bool,
    aliases: &[String],
    excludes: &[String],
) -> i32 {
    let parsed = match parse_install_source(source) {
        Ok(s) => s,
        Err(err) => {
            print_error(&err);
            return EXIT_USAGE;
        }
    };

    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    let plugin_scope = scope_to_plugin_scope(global, project);

    let options = upskill::pipeline::AddOptions {
        force,
        aliases: parse_alias_args(aliases),
        excludes: excludes.to_vec(),
    };

    let is_global = global || (!project && !is_inside_git_repo());
    let scope_label = if is_global { "global" } else { "project" };
    if !style::is_quiet() {
        eprintln!("scope: {} ({})", scope_label, target.display());
    }

    print_install_progress(&parsed);
    match install_with_lockfile(&parsed, &target, items, plugin_scope, &options) {
        Ok(report) => {
            print_install_report(&report, source);
            print_plugin_results(&report);
            EXIT_SUCCESS
        }
        Err(err) => {
            print_error_chain(&err);
            eprintln!();
            eprintln!("  hint: run 'upskill doctor' to check for inconsistencies");
            EXIT_ERROR
        }
    }
}

/// Parse `--as` arguments. Direct: `"alt-name"`. Bundle: `"original=alias"`.
fn parse_alias_args(args: &[String]) -> Vec<(String, String)> {
    args.iter()
        .map(|a| {
            if let Some((from, to)) = a.split_once('=') {
                (from.to_string(), to.to_string())
            } else {
                (String::new(), a.to_string())
            }
        })
        .collect()
}

/// Interactive y/n prompt for bulk removal by source label. Returns
/// `true` to proceed. When stdin is not a TTY (CI / pipes), returns
/// `true` without prompting — bulk removal in non-interactive contexts
/// is the script author's responsibility, and blocking forever on a
/// non-existent stdin would be worse than the footgun we're guarding.
/// `--yes` short-circuits this entirely (callers check it first).
fn confirm_bulk_remove(label: &str) -> bool {
    use std::io::{BufRead, IsTerminal, Write};

    if !std::io::stdin().is_terminal() {
        return true;
    }

    eprint!(
        "{} remove every item from {}? [y/N] ",
        style::warn("warning:"),
        style::name(label)
    );
    let _ = std::io::stderr().flush();

    let stdin = std::io::stdin();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Stderr progress line for git-backed installs. The clone happens deep
/// in `pipeline.rs`, which is silent by contract — without a marker the
/// CLI looks frozen for the seconds a clone takes. Local-path installs
/// stay silent (sub-second latency, nothing to wait on).
fn print_install_progress(source: &InstallSource) {
    if style::is_quiet() {
        return;
    }
    match source {
        InstallSource::LocalPath(_) => {}
        InstallSource::Github(repo) => {
            eprintln!(
                "{} {}",
                style::dim("Cloning"),
                style::name(&format!("github:{}/{}", repo.owner, repo.name))
            );
        }
        InstallSource::Gitlab(repo) => {
            eprintln!(
                "{} {}",
                style::dim("Cloning"),
                style::name(&format!("{}:{}/{}", repo.host, repo.owner, repo.name))
            );
        }
    }
}

/// Resolve the install target from the `-g/--global` and `-p/--project` flags.
///
/// Precedence:
/// - `--project` (explicit) → cwd, regardless of git-repo state.
/// - `--global` (explicit) → `$HOME`.
/// - Neither → cwd if inside a git repo, else `$HOME` (per spec §2.1
///   auto-fallback). The two flags are mutually exclusive at the clap layer.
fn install_target(global: bool, project: bool) -> anyhow::Result<PathBuf> {
    let scope = if project {
        Scope::Project
    } else if global {
        Scope::Global
    } else if is_inside_git_repo() {
        Scope::Project
    } else {
        Scope::Global
    };

    match scope {
        Scope::Project => std::env::current_dir().context("failed to get current directory"),
        Scope::Global => {
            home_dir().ok_or_else(|| anyhow::anyhow!("HOME (or USERPROFILE on Windows) is not set"))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    Project,
    Global,
}

/// Walk up from `cwd` looking for a `.git` entry (file or directory). Returns
/// `false` on any I/O error so the caller falls back to global scope rather
/// than crashing — that matches user expectation when `cwd` is unreadable.
fn is_inside_git_repo() -> bool {
    let mut dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => return false,
    };
    loop {
        if dir.join(".git").exists() {
            return true;
        }
        if !dir.pop() {
            return false;
        }
    }
}

/// Map the `--global` / `--project` CLI flags to a `PluginScope` for
/// plugin installation (used by clients that support scoped installs,
/// e.g. Claude Code).
fn scope_to_plugin_scope(global: bool, _project: bool) -> PluginScope {
    if global {
        PluginScope::User
    } else if is_inside_git_repo() {
        // `--project` or auto-detected git repo → project scope.
        PluginScope::Project
    } else {
        PluginScope::User
    }
}

fn print_install_report(report: &InstallReport, source: &str) {
    if style::is_quiet() {
        return;
    }
    use std::collections::BTreeMap;

    println!(
        "{} {} files from {}",
        style::success("Installed"),
        report.items.len(),
        style::dim(source)
    );

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
        println!(
            "  {} {:<32} → {}",
            style::dim(kind_label),
            style::name(&name),
            clients.join(", ")
        );
    }
}

/// Print plugin install results (success, skipped, failed) after an install.
/// Follows the warn-skip policy: CLI-not-found prints a warning with an
/// optional install URL; failures print the stderr output.
fn print_plugin_results(report: &InstallReport) {
    if style::is_quiet() || report.plugin_results.is_empty() {
        return;
    }

    use upskill::plugin::PluginOutcome;

    let successes: Vec<&PluginResult> = report
        .plugin_results
        .iter()
        .filter(|r| r.outcome.is_success())
        .collect();
    let skipped: Vec<&PluginResult> = report
        .plugin_results
        .iter()
        .filter(|r| r.outcome.is_cli_not_found())
        .collect();
    let instructions: Vec<&PluginResult> = report
        .plugin_results
        .iter()
        .filter(|r| r.outcome.is_manual_instructions())
        .collect();
    let failures: Vec<&PluginResult> = report
        .plugin_results
        .iter()
        .filter(|r| {
            !r.outcome.is_success()
                && !r.outcome.is_cli_not_found()
                && !r.outcome.is_manual_instructions()
        })
        .collect();

    if !successes.is_empty() {
        println!(
            "{} {} plugin(s) installed",
            style::success("Plugins:"),
            successes.len()
        );
        for r in &successes {
            println!(
                "  {} {} ({})",
                style::dim("plugin"),
                style::name(&r.name),
                r.client
            );
        }
    }

    for r in &instructions {
        let url = r.instructions_url.as_deref().unwrap_or(&r.identifier);
        let summary_lines = r
            .summary
            .as_deref()
            .map(|s| format!("\n        {}", s.trim().replace('\n', "\n        ")))
            .unwrap_or_default();
        eprintln!(
            "{} plugin {} ({}) \u{2014} manual step required{}",
            style::info("info:"),
            style::name(&r.name),
            r.client,
            summary_lines
        );
        eprintln!("        Instructions: {url}");
    }

    for r in &skipped {
        let url_hint = r
            .install_url
            .as_deref()
            .map(|u| format!(" — install manually: {u}"))
            .unwrap_or_default();
        eprintln!(
            "{} plugin {} skipped — {} CLI not found{}",
            style::warn("warning:"),
            style::name(&r.name),
            r.client,
            url_hint
        );
    }

    for r in &failures {
        if let PluginOutcome::Failed { exit_code, stderr } = &r.outcome {
            eprintln!(
                "{} plugin {} ({}) failed (exit {:?}): {}",
                style::warn("warning:"),
                style::name(&r.name),
                r.client,
                exit_code,
                stderr.trim()
            );
        }
    }
}

fn run_remove(
    names: &[String],
    source: Option<&str>,
    global: bool,
    project: bool,
    yes: bool,
) -> i32 {
    let filter = match (names.is_empty(), source) {
        (true, Some(s)) => RemoveFilter::BySource(s.to_string()),
        (false, None) => RemoveFilter::ByNames(names.to_vec()),
        (true, None) => {
            print_error("nothing to remove — pass one or more item names, or `--source <label>`");
            return EXIT_USAGE;
        }
        (false, Some(_)) => {
            print_error("`--source` and item names are mutually exclusive");
            return EXIT_USAGE;
        }
    };

    if let RemoveFilter::BySource(label) = &filter
        && !yes
        && !confirm_bulk_remove(label)
    {
        eprintln!("aborted");
        return EXIT_SUCCESS;
    }

    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    match remove(&target, filter) {
        Ok(report) => {
            print_remove_report(&report);
            EXIT_SUCCESS
        }
        Err(err) => {
            print_error_chain(&err);
            eprintln!();
            eprintln!("  hint: run 'upskill doctor' to check for inconsistencies");
            EXIT_ERROR
        }
    }
}

fn print_remove_report(report: &RemoveReport) {
    if style::is_quiet() {
        return;
    }
    if report.items.is_empty() {
        println!("no matching items in lockfile");
        return;
    }
    println!(
        "{} {} item(s)",
        style::success("Removed"),
        report.items.len()
    );
    for item in &report.items {
        let kind_label = match item.kind {
            ItemKind::Rule => "rule ",
            ItemKind::Skill => "skill",
            ItemKind::Agent => "agent",
        };
        println!(
            "  {} {:<32} {}",
            style::dim(kind_label),
            style::name(&item.name),
            style::dim(&format!("({} file(s))", item.deleted_files.len())),
        );
    }
}

fn run_update(names: &[String], dry_run: bool, yes: bool, global: bool, project: bool) -> i32 {
    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    let plugin_scope = scope_to_plugin_scope(global, project);

    if dry_run {
        // Explicit --dry-run: compute and report without applying.
        match update(&target, names, UpdateMode::DryRun, plugin_scope) {
            Ok(report) => {
                print_update_report(&report, true);
                EXIT_SUCCESS
            }
            Err(err) => {
                print_error_chain(&err);
                eprintln!();
                eprintln!("  hint: run 'upskill doctor' to check for inconsistencies");
                EXIT_ERROR
            }
        }
    } else if yes || !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        // No confirmation needed — apply directly (single fetch).
        match update(&target, names, UpdateMode::Apply, plugin_scope) {
            Ok(report) => {
                print_update_report(&report, false);
                EXIT_SUCCESS
            }
            Err(err) => {
                print_error_chain(&err);
                eprintln!();
                eprintln!("  hint: run 'upskill doctor' to check for inconsistencies");
                EXIT_ERROR
            }
        }
    } else {
        // Interactive: dry-run first to show plan, then apply if confirmed.
        let plan = match update(&target, names, UpdateMode::DryRun, plugin_scope) {
            Ok(r) => r,
            Err(err) => {
                print_error_chain(&err);
                eprintln!();
                eprintln!("  hint: run 'upskill doctor' to check for inconsistencies");
                return EXIT_ERROR;
            }
        };

        let has_changes = plan
            .items
            .iter()
            .any(|i| !matches!(i.status, UpdateStatus::UpToDate));

        if !has_changes {
            if !style::is_quiet() {
                println!("everything is up to date — nothing to do");
            }
            return EXIT_SUCCESS;
        }

        if !style::is_quiet() {
            print_update_plan(&plan);
        }

        if !confirm_update() {
            eprintln!("aborted.");
            return EXIT_SUCCESS;
        }

        match update(&target, names, UpdateMode::Apply, plugin_scope) {
            Ok(report) => {
                print_update_report(&report, false);
                EXIT_SUCCESS
            }
            Err(err) => {
                print_error_chain(&err);
                eprintln!();
                eprintln!("  hint: run 'upskill doctor' to check for inconsistencies");
                EXIT_ERROR
            }
        }
    }
}

/// Show a summary plan of what an update would do.
fn print_update_plan(report: &UpdateReport) {
    let update_count = report
        .items
        .iter()
        .filter(|i| matches!(i.status, UpdateStatus::WouldChange { .. }))
        .count();
    let remove_count = report
        .items
        .iter()
        .filter(|i| matches!(i.status, UpdateStatus::WouldRemove))
        .count();
    let unchanged_count = report
        .items
        .iter()
        .filter(|i| matches!(i.status, UpdateStatus::UpToDate))
        .count();

    if update_count > 0 {
        println!("  {}  {} item(s)", style::success("update:"), update_count);
    }
    if remove_count > 0 {
        println!(
            "  {}  {} item(s) (no longer in source)",
            style::error_label("remove:"),
            remove_count
        );
    }
    if unchanged_count > 0 {
        println!(
            "  {}  {} item(s)",
            style::dim("unchanged:"),
            unchanged_count
        );
    }
}

/// Interactive confirmation for update. Auto-proceeds in non-TTY (CI/pipes).
fn confirm_update() -> bool {
    use std::io::{BufRead, IsTerminal, Write};

    if !std::io::stdin().is_terminal() {
        return true;
    }

    eprint!("Apply? [y/N] ");
    let _ = std::io::stderr().flush();

    let stdin = std::io::stdin();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

fn print_update_report(report: &UpdateReport, dry_run: bool) {
    if style::is_quiet() {
        return;
    }
    if report.items.is_empty() {
        println!("nothing to update — lockfile is empty");
        return;
    }
    let changes = report
        .items
        .iter()
        .filter(|i| !matches!(i.status, UpdateStatus::UpToDate))
        .count();
    if changes == 0 && !dry_run {
        println!("everything is up to date — nothing to do");
        return;
    }
    let header: colored::ColoredString = if dry_run {
        style::warn("Dry-run: would update")
    } else {
        style::success("Updated")
    };
    println!("{header} {} of {} item(s)", changes, report.items.len());
    for item in &report.items {
        let kind_label = match item.kind {
            ItemKind::Rule => "rule ",
            ItemKind::Skill => "skill",
            ItemKind::Agent => "agent",
        };
        let status: colored::ColoredString = match &item.status {
            UpdateStatus::UpToDate => style::dim("up to date"),
            UpdateStatus::Updated { new_hash, .. } => {
                style::success(&format!("updated → {}", short(new_hash)))
            }
            UpdateStatus::WouldChange { new_hash, .. } => {
                style::warn(&format!("would change → {}", short(new_hash)))
            }
            UpdateStatus::Removed => style::error_label("removed — no longer in source"),
            UpdateStatus::WouldRemove => style::warn("would remove — no longer in source"),
        };
        println!(
            "  {} {:<32} {}",
            style::dim(kind_label),
            style::name(&item.name),
            status
        );
    }
}

fn short(h: &Option<String>) -> String {
    match h {
        Some(s) if s.len() >= 8 => s[..8].to_string(),
        Some(s) => s.clone(),
        None => "<no hash>".to_string(),
    }
}

fn run_doctor(global: bool, project: bool, json: bool) -> i32 {
    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    match doctor(&target) {
        Ok(report) => {
            if json {
                print_doctor_json(&report);
            } else {
                print_doctor_report(&report);
            }
            if report.is_clean() {
                EXIT_SUCCESS
            } else {
                EXIT_ERROR
            }
        }
        Err(err) => {
            print_error_chain(&err);
            EXIT_ERROR
        }
    }
}

fn print_doctor_json(report: &DoctorReport) {
    if style::is_quiet() {
        return;
    }
    // serde_json::to_string_pretty cannot fail for owned data — every
    // field on DoctorReport derives Serialize and contains no maps with
    // non-string keys. unwrap is safe.
    println!("{}", serde_json::to_string_pretty(report).unwrap());
}

fn print_doctor_report(report: &DoctorReport) {
    if style::is_quiet() {
        return;
    }
    if report.is_clean() {
        println!("{} clean", style::success("doctor:"));
        // Fall through: still print skipped_plugins warnings if any.
    }

    if !report.missing_outputs.is_empty() {
        println!(
            "{} missing per-client outputs ({} item(s)) — reinstall to fix",
            style::error_label("doctor:"),
            report.missing_outputs.len()
        );
        for m in &report.missing_outputs {
            println!(
                "  {} {}",
                style::dim(kind_label(m.kind)),
                style::name(&m.name)
            );
            for path in &m.missing_files {
                println!("    - {}", style::dim(&path.display().to_string()));
            }
        }
    }

    if !report.stale_hashes.is_empty() {
        println!(
            "{} SSOT hash drift on local sources ({} item(s)) — `upskill update` to fix",
            style::warn("doctor:"),
            report.stale_hashes.len()
        );
        for s in &report.stale_hashes {
            println!(
                "  {} {} {}",
                style::dim(kind_label(s.kind)),
                style::name(&s.name),
                style::dim(&format!("({})", s.source))
            );
        }
    }

    if !report.orphan_entries.is_empty() {
        println!(
            "{} lockfile entries with no recoverable source ({} item(s)) — `upskill remove` to clear",
            style::dim("doctor:"),
            report.orphan_entries.len()
        );
        for o in &report.orphan_entries {
            let reason = match o.reason {
                OrphanReason::LocalPathGone => "local path gone",
                OrphanReason::ItemMissingInSource => "item not in source",
            };
            println!(
                "  {} {} {} — {}",
                style::dim(kind_label(o.kind)),
                style::name(&o.name),
                style::dim(&format!("({})", o.source)),
                style::dim(reason),
            );
        }
    }

    if !report.missing_plugins.is_empty() {
        println!(
            "{} {} plugin(s) recorded as installed but missing from client — `upskill update` to fix",
            style::error_label("doctor:"),
            report.missing_plugins.len()
        );
        for p in &report.missing_plugins {
            println!(
                "  {} {} ({})",
                style::dim("plugin"),
                style::name(&p.name),
                p.client,
            );
        }
    }

    print_doctor_orphaned_dependencies(report);
    print_doctor_skipped_plugins(report);
}

fn print_doctor_orphaned_dependencies(report: &DoctorReport) {
    if report.orphaned_dependencies.is_empty() {
        return;
    }
    println!(
        "{} {} orphaned dependency(ies) — every item that required them is no longer installed",
        style::warn("doctor:"),
        report.orphaned_dependencies.len()
    );
    for d in &report.orphaned_dependencies {
        println!(
            "  orphaned dependency: {} {} (was required by {}, now absent)",
            style::dim(kind_label(d.kind)),
            style::name(&d.name),
            d.former_requirers.join(", "),
        );
        println!(
            "    {}",
            style::dim(&format!(
                "remove with `upskill remove {}` if no longer needed",
                d.name
            ))
        );
    }
}

fn print_doctor_skipped_plugins(report: &DoctorReport) {
    if report.skipped_plugins.is_empty() {
        return;
    }
    println!(
        "{} {} plugin(s) never installed — client CLI was missing at install time",
        style::warn("doctor:"),
        report.skipped_plugins.len()
    );
    for p in &report.skipped_plugins {
        println!(
            "  {} {} ({})",
            style::dim("plugin"),
            style::name(&p.name),
            p.client,
        );
    }
    println!(
        "  {}",
        style::dim("install the missing CLI then run `upskill update` to install them")
    );
}

fn kind_label(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Rule => "rule ",
        ItemKind::Skill => "skill",
        ItemKind::Agent => "agent",
    }
}

fn run_list(global: bool, project: bool, json: bool) -> i32 {
    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    match list(&target) {
        Ok(report) => {
            if json {
                print_list_json(&report);
            } else {
                print_list_report(&report);
            }
            EXIT_SUCCESS
        }
        Err(err) => {
            print_error_chain(&err);
            EXIT_ERROR
        }
    }
}

fn print_list_json(report: &ListReport) {
    if style::is_quiet() {
        return;
    }
    // serde_json::to_string_pretty cannot fail for owned data — every
    // field on ListReport derives Serialize and contains no maps with
    // non-string keys. unwrap is safe.
    println!("{}", serde_json::to_string_pretty(report).unwrap());
}

fn print_list_report(report: &ListReport) {
    if style::is_quiet() {
        return;
    }
    if report.is_empty() {
        println!("No items installed.");
        println!();
        println!("  Get started: upskill add owner/repo");
        println!("  Browse:      upskill search <query>");
        return;
    }

    print_list_section("rules", &report.rules);
    print_list_section("skills", &report.skills);
    print_list_section("agents", &report.agents);
    print_list_bundles(&report.bundles);
}

fn print_list_section(label: &str, items: &[ListedItem]) {
    if items.is_empty() {
        return;
    }
    println!("{} ({})", style::name(label), items.len());
    for item in items {
        let pinned = match &item.git_ref {
            Some(r) => format!("@{r}"),
            None => String::new(),
        };
        println!(
            "  {:<32} {}",
            style::name(&item.name),
            style::dim(&format!("{}{pinned}", item.source))
        );
    }
}

fn print_list_bundles(bundles: &[ListedBundle]) {
    if bundles.is_empty() {
        return;
    }
    println!("{} ({})", style::name("bundles"), bundles.len());
    for bundle in bundles {
        let pinned = match &bundle.git_ref {
            Some(r) => format!("@{r}"),
            None => String::new(),
        };
        println!(
            "  {:<32} {}",
            style::name(&bundle.name),
            style::dim(&format!("{}{pinned}", bundle.source))
        );
    }
}

fn run_lint(paths: &[PathBuf], strict: bool) -> i32 {
    match lint(paths, strict) {
        Ok(report) => {
            print_lint_report(&report);
            if report.has_errors() {
                EXIT_ERROR
            } else {
                EXIT_SUCCESS
            }
        }
        Err(err) => {
            print_error_chain(&err);
            // Author-command misuse (consumer-project / unreadable
            // path) maps to usage error. The lint module surfaces
            // those as Err(_); per-rule findings travel inside Ok.
            EXIT_USAGE
        }
    }
}

fn print_lint_report(report: &LintReport) {
    if style::is_quiet() {
        return;
    }
    for f in &report.findings {
        let line = f.line.map(|n| format!(":{n}")).unwrap_or_default();
        let severity_label = match f.severity {
            upskill::lint::Severity::Error => style::error_label(f.severity.label()),
            upskill::lint::Severity::Warning => style::warn(f.severity.label()),
        };
        println!(
            "{}: {}{} {} {}",
            severity_label,
            style::name(&f.path.display().to_string()),
            line,
            style::dim(&format!("[{}]", f.rule_id)),
            f.message
        );
    }
    println!(
        "{} file(s) checked, {} findings",
        report.files_checked,
        report.findings.len()
    );
}

fn run_fmt(paths: &[PathBuf]) -> i32 {
    match fmt(paths) {
        Ok(report) => {
            print_fmt_report(&report);
            EXIT_SUCCESS
        }
        Err(err) => {
            print_error_chain(&err);
            EXIT_USAGE
        }
    }
}

fn print_fmt_report(report: &FmtReport) {
    if style::is_quiet() {
        return;
    }
    if report.files_changed.is_empty() {
        println!("{} file(s) checked, all formatted", report.files_checked);
        return;
    }
    for path in &report.files_changed {
        println!(
            "{} {}",
            style::warn("formatted:"),
            style::name(&path.display().to_string())
        );
    }
    println!(
        "{} file(s) checked, {} file(s) changed",
        report.files_checked,
        report.files_changed.len()
    );
}

fn run_new(kind: &str, name: &str) -> i32 {
    let parsed_kind = match NewKind::parse(kind) {
        Ok(k) => k,
        Err(err) => {
            print_error(&err);
            return EXIT_USAGE;
        }
    };
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(err) => {
            print_error(format!("get current directory: {err}"));
            return EXIT_ERROR;
        }
    };
    match scaffold(&cwd, parsed_kind, name) {
        Ok(report) => {
            print_scaffold_report(&report);
            EXIT_SUCCESS
        }
        Err(err) => {
            let msg = format!("{:#}", err);
            print_error(&msg);
            // Author-command misuse (consumer-project) → usage error;
            // every other failure → 1.
            if msg.contains("consumer project") {
                EXIT_USAGE
            } else {
                EXIT_ERROR
            }
        }
    }
}

fn print_scaffold_report(report: &ScaffoldReport) {
    if style::is_quiet() {
        return;
    }
    let kind_label = match report.kind {
        NewKind::Rule => "rule",
        NewKind::Skill => "skill",
        NewKind::Agent => "agent",
    };
    println!(
        "{} {kind_label} `{}` at {}",
        style::success("scaffolded"),
        style::name(&report.name),
        style::name(&report.written.display().to_string())
    );
    println!("edit the file and replace the TODO body before publishing.");
}

fn run_search(query: &str, limit: usize, registry: Option<&str>, kind: Option<&str>) -> i32 {
    let cfg = match config::load_config(home_dir().as_deref(), Some(std::path::Path::new("."))) {
        Ok(c) => c,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    let mut found_any = false;
    let mut skills_sh_failed = false;

    // If --registry is specified, search only that registry.
    if let Some(reg_name) = registry {
        let entry = match cfg.registries.iter().find(|r| r.name == reg_name) {
            Some(e) => e,
            None => {
                eprintln!("error: registry '{}' not found in config", reg_name);
                return EXIT_ERROR;
            }
        };
        let fresh = match index::ensure_fresh(entry) {
            Ok(f) => f,
            Err(err) => {
                print_error(&err);
                return EXIT_ERROR;
            }
        };
        if let Some(w) = &fresh.warning
            && !style::is_quiet()
        {
            eprintln!("{}", style::warn(w));
        }
        let results = search::search_index(&fresh.index, query, kind);
        if !results.is_empty() {
            found_any = true;
            if !style::is_quiet() {
                println!("{}", style::dim(&format!("── {} ──", reg_name)));
                for r in &results {
                    println!(
                        "  {} [{}]\t{}",
                        style::name(&r.name),
                        r.kind,
                        style::dim(&format!("upskill add {}:{}", r.source, r.path))
                    );
                }
            }
        }
    } else {
        // Query skills.sh unless --kind filters to something other than "skill".
        let skip_skills_sh = kind.is_some() && kind != Some("skill");
        if !skip_skills_sh {
            match search::search(query, limit) {
                Err(err) => {
                    skills_sh_failed = true;
                    print_error(&err);
                }
                Ok(results) if !results.is_empty() => {
                    found_any = true;
                    if !style::is_quiet() {
                        println!("{}", style::dim("── skills.sh ──"));
                        for skill in &results {
                            let repo = skill
                                .source
                                .trim_start_matches("github/")
                                .trim_start_matches("gitlab/");
                            println!(
                                "  {}\t{}\t{}",
                                style::name(&skill.name),
                                style::dim(&format!("{} installs", skill.installs)),
                                style::dim(&format!("upskill add {repo} {}", skill.name))
                            );
                        }
                    }
                }
                Ok(_) => {}
            }
        }

        // Search all configured registries.
        for entry in &cfg.registries {
            let fresh = match index::ensure_fresh(entry) {
                Ok(f) => f,
                Err(err) => {
                    if !style::is_quiet() {
                        eprintln!(
                            "{}: {}",
                            style::warn(&format!("registry '{}'", entry.name)),
                            err
                        );
                    }
                    continue;
                }
            };
            if let Some(w) = &fresh.warning
                && !style::is_quiet()
            {
                eprintln!("{}", style::warn(w));
            }
            let results = search::search_index(&fresh.index, query, kind);
            if !results.is_empty() {
                found_any = true;
                if !style::is_quiet() {
                    println!("{}", style::dim(&format!("── {} ──", entry.name)));
                    for r in &results {
                        println!(
                            "  {} [{}]\t{}",
                            style::name(&r.name),
                            r.kind,
                            style::dim(&format!("upskill add {}:{}", r.source, r.path))
                        );
                    }
                }
            }
        }
    }

    if !found_any && !style::is_quiet() {
        println!("no skills found for '{}'", query);
    }
    // If skills.sh failed and there are no configured registries to fall
    // back on, propagate the error (backwards-compatible behavior).
    if skills_sh_failed && cfg.registries.is_empty() {
        return EXIT_ERROR;
    }
    EXIT_SUCCESS
}

fn run_index(registry: Option<&str>, clear: bool) -> i32 {
    if clear {
        let dir = index::cache_dir();
        if dir.exists()
            && let Err(err) = std::fs::remove_dir_all(&dir)
        {
            eprintln!("error: failed to clear index cache: {}", err);
            return EXIT_ERROR;
        }
        if !style::is_quiet() {
            println!("cleared index cache");
        }
        return EXIT_SUCCESS;
    }

    let cfg = match config::load_config(home_dir().as_deref(), Some(std::path::Path::new("."))) {
        Ok(c) => c,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    let entries: Vec<_> = if let Some(name) = registry {
        match cfg.registries.iter().find(|r| r.name == name) {
            Some(e) => vec![e.clone()],
            None => {
                eprintln!("error: registry '{}' not found in config", name);
                return EXIT_ERROR;
            }
        }
    } else {
        cfg.registries.clone()
    };

    if entries.is_empty() {
        if !style::is_quiet() {
            println!("no registries configured");
        }
        return EXIT_SUCCESS;
    }

    for entry in &entries {
        if !style::is_quiet() {
            println!("indexing {}...", entry.name);
        }
        match index::build_index(entry) {
            Ok(idx) => {
                if let Err(err) = index::write_index(&idx) {
                    eprintln!("error: failed to write index for '{}': {}", entry.name, err);
                    return EXIT_ERROR;
                }
                if !style::is_quiet() {
                    println!("  {} items indexed", idx.items.len());
                }
            }
            Err(err) => {
                eprintln!("error: failed to index '{}': {}", entry.name, err);
                return EXIT_ERROR;
            }
        }
    }
    EXIT_SUCCESS
}
