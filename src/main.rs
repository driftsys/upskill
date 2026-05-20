use anyhow::Context;
use clap::{Parser, error::ErrorKind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use upskill::cli::{Cli, Commands};
use upskill::fmt::{FmtReport, fmt};
use upskill::lint::{LintReport, lint};
use upskill::pipeline::{
    DoctorReport, InstallReport, ItemKind, ListReport, ListedBundle, ListedItem, OrphanReason,
    RemoveFilter, RemoveReport, UpdateMode, UpdateReport, UpdateStatus, doctor,
    install_with_lockfile, list, remove, update,
};
use upskill::scaffold::{NewKind, ScaffoldReport, scaffold};
use upskill::search;
use upskill::source::{InstallSource, parse_install_source};
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
        } => run_add(&source, &items, global, project),
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
        } => run_update(&names, dry_run, global, project),
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
        Commands::Search { query, limit } => run_search(&query, limit),
        Commands::Lint { paths, strict } => run_lint(&paths, strict),
        Commands::Fmt { paths } => run_fmt(&paths),
        Commands::New { kind, name } => run_new(&kind, &name),
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
        // clig.dev §"Responsiveness": say something before cleanup so
        // the user knows their Ctrl-C registered. Stderr only — never
        // touches stdout-as-data.
        eprintln!("\n{} cleaning up", style::warn("interrupted:"));
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

fn run_add(source: &str, items: &[String], global: bool, project: bool) -> i32 {
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

    print_install_progress(&parsed);
    match install_with_lockfile(&parsed, &target, items) {
        Ok(report) => {
            print_install_report(&report, source);
            EXIT_SUCCESS
        }
        Err(err) => {
            print_error_chain(&err);
            EXIT_ERROR
        }
    }
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
        Scope::Global => std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("HOME is not set")),
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

fn run_update(names: &[String], dry_run: bool, global: bool, project: bool) -> i32 {
    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
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
            print_error_chain(&err);
            EXIT_ERROR
        }
    }
}

fn print_update_report(report: &UpdateReport, dry_run: bool) {
    if style::is_quiet() {
        return;
    }
    if report.items.is_empty() {
        println!("nothing to update — lockfile is empty");
        return;
    }
    let header: colored::ColoredString = if dry_run {
        style::warn("Dry-run: would update")
    } else {
        style::success("Updated")
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
        let status: colored::ColoredString = match &item.status {
            UpdateStatus::UpToDate => style::dim("up to date"),
            UpdateStatus::Updated { new_hash, .. } => {
                style::success(&format!("updated → {}", short(new_hash)))
            }
            UpdateStatus::WouldChange { new_hash, .. } => {
                style::warn(&format!("would change → {}", short(new_hash)))
            }
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
        return;
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
        println!("no items installed");
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

fn run_search(query: &str, limit: usize) -> i32 {
    match search::search(query, limit) {
        Err(err) => {
            print_error(&err);
            EXIT_ERROR
        }
        Ok(results) if results.is_empty() => {
            if !style::is_quiet() {
                println!("no skills found for '{}'", query);
            }
            EXIT_SUCCESS
        }
        Ok(results) => {
            if !style::is_quiet() {
                for skill in &results {
                    let repo = skill
                        .source
                        .trim_start_matches("github/")
                        .trim_start_matches("gitlab/");
                    println!(
                        "{}\t{}\t{}",
                        style::name(&skill.name),
                        style::dim(&format!("{} installs", skill.installs)),
                        style::dim(&format!("upskill add {repo} --skill {}", skill.name))
                    );
                }
            }
            EXIT_SUCCESS
        }
    }
}
