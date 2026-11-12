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
    PluginResult, PreviewAction, RemoveFilter, RemoveReport, SourceDecision, SourcePreview,
    SourceResult, UpdateObserver, UpdateReport, UpdateStatus, doctor, install_with_lockfile, list,
    remove, update,
};
use upskill::plugin::PluginScope;
use upskill::scaffold::{NewKind, ScaffoldReport, scaffold};
use upskill::search;
use upskill::select::{ClientSelection, SelectedClient};
use upskill::source::{InstallSource, home_dir, parse_install_source};
use upskill::style;

const EXIT_SUCCESS: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_USAGE: i32 = 2;
const EXIT_INTERRUPTED: i32 = 130;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
/// True only while `update` in a terminal waits for the answer to
/// `Apply? [y/N/q]`. No write is in progress then, so SIGINT exits at once.
static WAITING_FOR_ANSWER: AtomicBool = AtomicBool::new(false);

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
            claude,
            copilot,
            vscode,
            opencode,
        } => run_add(AddArgs {
            source: &source,
            items: &items,
            global,
            project,
            force,
            aliases: &alias,
            excludes: &exclude,
            flag_clients: client_flags(claude, copilot, vscode, opencode),
        }),
        Commands::Remove {
            names,
            source,
            global,
            project,
            yes,
            force,
        } => run_remove(&names, source.as_deref(), global, project, yes, force),
        Commands::Update {
            names,
            dry_run,
            global,
            project,
            yes,
            force,
            claude,
            copilot,
            vscode,
            opencode,
        } => run_update(UpdateArgs {
            names: &names,
            dry_run,
            yes,
            global,
            project,
            force,
            flag_clients: client_flags(claude, copilot, vscode, opencode),
        }),
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
        Commands::RemoveMcp {
            name,
            global,
            project,
        } => run_remove_mcp(&name, global, project),
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
        if WAITING_FOR_ANSWER.load(Ordering::SeqCst) {
            // End the question line; nothing is written, so exit now.
            eprintln!();
            std::process::exit(EXIT_INTERRUPTED);
        }
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
/// anyhow context chain (root cause last). The adoption context that
/// `AdoptedBeforeFailure` adds is left out: `print_adoption_after_error`
/// prints the adoption notice instead.
fn print_error_chain(err: &anyhow::Error) {
    if err
        .downcast_ref::<upskill::pipeline::AdoptedBeforeFailure>()
        .is_some()
    {
        let rest: Vec<String> = err.chain().skip(1).map(ToString::to_string).collect();
        eprintln!("{} {}", style::error_label("error:"), rest.join(": "));
    } else {
        eprintln!("{} {err:#}", style::error_label("error:"));
    }
}

fn map_clap_error(err: &clap::Error) -> i32 {
    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => EXIT_SUCCESS,
        _ => EXIT_USAGE,
    }
}

/// Positional-argument-heavy `add` inputs grouped into one struct to keep the
/// resolver call under clippy's `too_many_arguments` bar (AGENTS.md).
struct AddArgs<'a> {
    source: &'a str,
    items: &'a [String],
    global: bool,
    project: bool,
    force: bool,
    aliases: &'a [String],
    excludes: &'a [String],
    /// Clients named by `--claude`/`--copilot`/`--vscode`/`--opencode`.
    /// Empty means no client flag was passed (fall back to config / all).
    flag_clients: Vec<SelectedClient>,
}

/// Collect the per-client boolean flags into an ordered selection list.
fn client_flags(claude: bool, copilot: bool, vscode: bool, opencode: bool) -> Vec<SelectedClient> {
    let mut out = Vec::new();
    if claude {
        out.push(SelectedClient::Claude);
    }
    if copilot {
        out.push(SelectedClient::Copilot);
    }
    if vscode {
        out.push(SelectedClient::VsCode);
    }
    if opencode {
        out.push(SelectedClient::OpenCode);
    }
    out
}

/// Resolve the effective client selection from per-invocation flags and the
/// merged (global + project) config. Precedence: flags > config > all.
///
/// `project_root` is the directory whose `.upskill/config.yaml` is read as the
/// project layer — `None` for a global-scoped operation, so a `--global`
/// install is not steered by the unrelated project the shell happens to be in.
fn resolve_selection(
    flag_clients: &[SelectedClient],
    project_root: Option<&std::path::Path>,
) -> ClientSelection {
    let config = config::load_config(home_dir().as_deref(), project_root).unwrap_or_default();
    config::resolve_client_selection(flag_clients, &config)
}

/// The project-config root for a scope: the current directory unless the
/// operation resolved to global scope.
fn project_config_root(is_global: bool) -> Option<&'static std::path::Path> {
    if is_global {
        None
    } else {
        Some(std::path::Path::new("."))
    }
}

fn run_add(args: AddArgs) -> i32 {
    let AddArgs {
        source,
        items,
        global,
        project,
        force,
        aliases,
        excludes,
        flag_clients,
    } = args;
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

    let is_global = global || (!project && !is_inside_git_repo());

    let options = upskill::pipeline::AddOptions {
        force,
        overwrite: force,
        aliases: parse_alias_args(aliases),
        excludes: excludes.to_vec(),
        selection: resolve_selection(&flag_clients, project_config_root(is_global)),
        keep_unwithheld_edits: false,
    };

    let scope_label = if is_global { "global" } else { "project" };
    if !style::is_quiet() {
        eprintln!("scope: {} ({})", scope_label, target.display());
    }

    print_install_progress(&parsed);
    match install_with_lockfile(&parsed, &target, items, plugin_scope, &options) {
        Ok(report) => {
            print_install_report(&report, source);
            print_adoption_notice(report.adopted.as_deref());
            print_selection_skipped(&report);
            print_plugin_results(&report);
            print_mcp_results(&report);
            print_kept_modified(&report.kept_modified);
            print_forced(&report.forced);
            if report.conflicts.is_empty() {
                EXIT_SUCCESS
            } else {
                print_conflicts(&report.conflicts);
                EXIT_ERROR
            }
        }
        Err(err) => {
            print_adoption_after_error(&err);
            print_error_chain(&err);
            eprintln!();
            eprintln!("  hint: run 'upskill doctor' to check for inconsistencies");
            EXIT_ERROR
        }
    }
}

/// One line per destination that stopped the write of an item, then the
/// hint. Printed to stderr, so `-q/--quiet` does not hide it.
fn print_conflicts(conflicts: &[upskill::pipeline::FileConflict]) {
    for conflict in conflicts {
        let reason = match conflict.state {
            upskill::pipeline::ConflictState::Modified => "was modified",
            upskill::pipeline::ConflictState::Unowned => "exists and is not managed by upskill",
        };
        eprintln!(
            "skipped {} {}: {} {reason}",
            conflict.kind,
            conflict.name,
            conflict.path.display()
        );
    }
    eprintln!("Use --force to replace.");
}

/// One line per modified file that was kept instead of deleted. Printed to
/// stderr, so `-q/--quiet` does not hide it.
fn print_kept_modified(paths: &[PathBuf]) {
    for path in paths {
        eprintln!("kept modified file: {}", path.display());
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
        InstallSource::Git(repo) => {
            eprintln!("{} {}", style::dim("Cloning"), style::name(&repo.url));
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

    // Items skipped because of a conflict were not written.
    let written: Vec<_> = report
        .items
        .iter()
        .filter(|item| {
            !report
                .conflicts
                .iter()
                .any(|c| c.kind == item.kind && c.name == item.name)
        })
        .collect();
    println!(
        "{} {} files from {}",
        style::success("Installed"),
        written.len(),
        style::dim(source)
    );

    let mut grouped: BTreeMap<(ItemKind, String), Vec<&'static str>> = BTreeMap::new();
    for item in written {
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

/// Tell the user which files the adoption of a `schema: 1` lockfile
/// recorded as unchanged. Adoption cannot tell an edited file from one an
/// earlier version wrote, so manual edits to these files are not protected.
/// Printed to stderr, so `-q/--quiet` does not hide it.
fn print_adoption_notice(adopted: Option<&[String]>) {
    let Some(paths) = adopted.filter(|p| !p.is_empty()) else {
        return;
    };
    eprintln!(
        "note: adopted {} files installed by an earlier upskill version as unchanged.",
        paths.len()
    );
    eprintln!("Manual edits made to these files before now are not protected:");
    for path in paths {
        eprintln!("  {path}");
    }
    eprintln!(
        "The schema 1 lockfile is copied to {}, unless that file already exists.",
        upskill::lockfile::SCHEMA1_BACKUP_NAME
    );
}

/// Print the adoption notice when `err` carries the paths of an adoption
/// that was saved before the error.
fn print_adoption_after_error(err: &anyhow::Error) {
    if let Some(adopted) = err.downcast_ref::<upskill::pipeline::AdoptedBeforeFailure>() {
        print_adoption_notice(Some(&adopted.paths));
    }
}

/// Warn about items skipped because the consumer's client selection
/// (ADR-0012) shares no client with the item's author `audience:`. Printed to
/// stderr so `-q/--quiet` (which suppresses informational stdout only) does
/// not hide it.
fn print_selection_skipped(report: &InstallReport) {
    for name in &report.selection_skipped {
        eprintln!(
            "{} skipped {} — its audience shares no client with the selected client(s)",
            style::warn("warning:"),
            style::name(name)
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

/// Print MCP server configuration results after an install.
/// Success entries go to stdout; warnings (CLI-not-found, failure) go to stderr.
/// Env-var warnings are emitted for any required vars not set in the environment.
fn print_mcp_results(report: &InstallReport) {
    if style::is_quiet() || report.mcp_results.is_empty() {
        return;
    }

    use upskill::plugin::PluginOutcome;

    // A single MCP server fans out to one result per target, so its
    // `requires-env` warnings would otherwise repeat once per target. Warn
    // once per (server, var) — the env var is set in the shell, not per client.
    let mut warned_env: std::collections::HashSet<(&str, &str)> = std::collections::HashSet::new();

    for mr in &report.mcp_results {
        match &mr.outcome {
            PluginOutcome::Success => {
                println!(
                    "  {} MCP server '{}' for {}",
                    style::success("configured"),
                    mr.name,
                    mr.client
                );
            }
            PluginOutcome::CliNotFound => {
                eprintln!(
                    "{} {} CLI not found and no config target; skipped MCP '{}'.",
                    style::warn("warning:"),
                    mr.client,
                    mr.name
                );
            }
            PluginOutcome::Failed { stderr, exit_code } => {
                eprintln!(
                    "{} failed to configure MCP '{}' for {} (exit {:?}): {}",
                    style::warn("warning:"),
                    mr.name,
                    mr.client,
                    exit_code,
                    stderr.trim()
                );
            }
            PluginOutcome::ManualInstructions => {}
        }
        for var in &mr.requires_env {
            if std::env::var_os(var).is_none() && warned_env.insert((&mr.name, var)) {
                eprintln!(
                    "{} MCP '{}' needs env var '{}'; it is not set in your environment.",
                    style::warn("warning:"),
                    mr.name,
                    var
                );
            }
        }
    }
}

fn run_remove(
    names: &[String],
    source: Option<&str>,
    global: bool,
    project: bool,
    yes: bool,
    force: bool,
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

    if let RemoveFilter::BySource(label) = &filter {
        if !yes && !confirm_bulk_remove(label) {
            eprintln!("aborted");
            return EXIT_SUCCESS;
        }
    }

    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    match remove(&target, filter, force) {
        Ok(report) => {
            print_adoption_notice(report.adopted.as_deref());
            print_remove_report(&report);
            for item in &report.items {
                print_kept_modified(&item.kept_modified);
            }
            EXIT_SUCCESS
        }
        Err(err) => {
            print_adoption_after_error(&err);
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

/// `update` inputs grouped into one struct to keep the call under
/// clippy's `too_many_arguments` bar (AGENTS.md).
struct UpdateArgs<'a> {
    names: &'a [String],
    dry_run: bool,
    yes: bool,
    global: bool,
    project: bool,
    /// `--force`: replace modified and unowned files.
    force: bool,
    flag_clients: Vec<SelectedClient>,
}

/// Exit code of an applied update: 1 when an item was skipped because of
/// modified or unowned files, 0 otherwise. Prints the kept files and the
/// skipped files to stderr.
fn finish_update(report: &UpdateReport) -> i32 {
    print_kept_modified(&report.kept_modified);
    if print_skipped(report) {
        eprintln!("Use --force to replace.");
        EXIT_ERROR
    } else {
        EXIT_SUCCESS
    }
}

/// Print to stderr one `skipped` line per file of each item with status
/// [`UpdateStatus::SkippedModified`]. Returns true when there is such an
/// item.
fn print_skipped(report: &UpdateReport) -> bool {
    let mut skipped = false;
    for item in &report.items {
        if let UpdateStatus::SkippedModified { files } = &item.status {
            skipped = true;
            for path in files {
                eprintln!(
                    "skipped {} {}: {} was modified or is not managed by upskill",
                    item.kind,
                    item.name,
                    path.display()
                );
            }
        }
    }
    skipped
}

/// Print `Changed <n> file(s) written or deleted`.
fn print_changed_files(count: usize) {
    println!(
        "{} {} file(s) written or deleted",
        style::success("Changed"),
        count
    );
}

fn run_update(args: UpdateArgs) -> i32 {
    let UpdateArgs {
        names,
        dry_run,
        yes,
        global,
        project,
        force,
        flag_clients,
    } = args;
    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    let plugin_scope = scope_to_plugin_scope(global, project);
    let is_global = global || (!project && !is_inside_git_repo());
    let selection = resolve_selection(&flag_clients, project_config_root(is_global));

    let terminal = !dry_run && !yes && std::io::IsTerminal::is_terminal(&std::io::stdin());
    let mut apply_all = ApplyAll;
    let mut dry = DryRunObserver::default();
    let mut ask = TerminalObserver::default();
    let observer: &mut dyn UpdateObserver = if dry_run {
        &mut dry
    } else if terminal {
        &mut ask
    } else {
        &mut apply_all
    };
    let result = update(
        &target,
        names,
        plugin_scope,
        &selection,
        force,
        dry_run,
        observer,
    );
    match result {
        Ok(report) if dry_run => {
            print_update_report(&report, true);
            EXIT_SUCCESS
        }
        Ok(report) if terminal => finish_terminal_update(&report),
        Ok(report) => {
            print_update_report(&report, false);
            print_adoption_notice(report.adopted.as_deref());
            finish_update(&report)
        }
        Err(err) => {
            // A terminal run prints the adoption notice before the block of
            // the source that adopted; it is not printed a second time.
            if !ask.notice_printed {
                print_adoption_after_error(&err);
            }
            print_error_chain(&err);
            eprintln!();
            eprintln!("  hint: run 'upskill doctor' to check for inconsistencies");
            EXIT_ERROR
        }
    }
}

/// The last line of `update --dry-run`.
const DRY_RUN_LAST_LINE: &str = "Each block is computed for the current state; a later block does not include the effect of an earlier one.";

/// The `update` observer for `--yes` and for a run without a terminal:
/// applies every source, also one without a change, so that its plugin and
/// MCP steps run. After an interrupt it stops before the next source.
/// Prints nothing.
struct ApplyAll;

impl UpdateObserver for ApplyAll {
    fn preview(&mut self, _preview: &SourcePreview) -> SourceDecision {
        // After an interrupt during the apply of a source, apply no other
        // source.
        if was_interrupted() {
            SourceDecision::Stop
        } else {
            SourceDecision::Apply
        }
    }

    fn applied(&mut self, _result: &SourceResult) {}
}

/// The `update` observer for `--dry-run`: prints the block of each source
/// and applies nothing. The totals are printed after the run.
#[derive(Default)]
struct DryRunObserver {
    blocks: usize,
}

impl UpdateObserver for DryRunObserver {
    fn preview(&mut self, preview: &SourcePreview) -> SourceDecision {
        print_block(preview, &mut self.blocks);
        SourceDecision::Skip
    }

    fn applied(&mut self, _result: &SourceResult) {}
}

/// The `update` observer for a terminal without `--yes`. For each source it
/// prints the adoption notice when the source adopted a `schema: 1`
/// lockfile, then the block of the source. For a source with changes it
/// asks `Apply? [y/N/q]` on stderr and reads one line; see [`parse_answer`].
/// A source without changes is not applied and is not asked about. After
/// an applied source it prints the result line ([`render_result`]).
///
/// Ctrl-C while the question waits exits with code 130 at once. After an
/// interrupt during the apply of a source, it returns
/// [`SourceDecision::Stop`] before the next block; the run exits with
/// code 130.
///
/// With `--quiet`, the blocks and the result lines are not printed, and the
/// question names the source: `Apply changes from <source>? [y/N/q]`. The
/// adoption notice is printed in both cases.
#[derive(Default)]
struct TerminalObserver {
    blocks: usize,
    /// True after the adoption notice was printed.
    notice_printed: bool,
}

impl UpdateObserver for TerminalObserver {
    fn preview(&mut self, preview: &SourcePreview) -> SourceDecision {
        use std::io::{BufRead, Write};

        // After an interrupt during the apply of a source, show and apply no
        // other source.
        if was_interrupted() {
            return SourceDecision::Stop;
        }
        if let Some(paths) = preview.adopted.as_deref().filter(|p| !p.is_empty()) {
            print_adoption_notice(Some(paths));
            self.notice_printed = true;
        }
        print_block(preview, &mut self.blocks);
        if !preview.has_changes() {
            return SourceDecision::Skip;
        }
        if style::is_quiet() {
            eprint!("Apply changes from {}? [y/N/q] ", preview.source);
        } else {
            eprint!("Apply? [y/N/q] ");
        }
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        WAITING_FOR_ANSWER.store(true, Ordering::SeqCst);
        // An interrupt received before the flag was set did not exit.
        if was_interrupted() {
            WAITING_FOR_ANSWER.store(false, Ordering::SeqCst);
            return SourceDecision::Stop;
        }
        let read = std::io::stdin().lock().read_line(&mut line);
        WAITING_FOR_ANSWER.store(false, Ordering::SeqCst);
        if was_interrupted() {
            return SourceDecision::Stop;
        }
        let answer = match read {
            Ok(0) | Err(_) => {
                // End the question line, which the user did not end.
                eprintln!();
                None
            }
            Ok(_) => Some(line.as_str()),
        };
        parse_answer(answer)
    }

    fn applied(&mut self, result: &SourceResult) {
        if !style::is_quiet() {
            println!("{}", render_result(result));
        }
    }
}

/// The decision for one answer to `Apply? [y/N/q]`. `None` is end of
/// input or a read error. Letter case and surrounding white space are
/// ignored.
fn parse_answer(line: Option<&str>) -> SourceDecision {
    let Some(line) = line else {
        return SourceDecision::Stop;
    };
    match line.trim().to_lowercase().as_str() {
        "y" | "yes" => SourceDecision::Apply,
        "q" | "quit" => SourceDecision::Stop,
        _ => SourceDecision::Skip,
    }
}

/// Print the block of one source to stdout, after an empty line when it is
/// not the first block. Nothing is printed with `--quiet`.
fn print_block(preview: &SourcePreview, blocks: &mut usize) {
    if style::is_quiet() {
        return;
    }
    if *blocks > 0 {
        println!();
    }
    *blocks += 1;
    for (i, line) in render_block(preview).iter().enumerate() {
        if i == 0 {
            println!("{}", style::name(line));
        } else if line.starts_with("  skip ") {
            println!("{}", style::warn(line));
        } else if line == "  up to date" {
            println!("{}", style::dim(line));
        } else {
            println!("{line}");
        }
    }
}

/// The lines of the block of one source, without colour: the source label,
/// one line per item that is not up to date, one `write` line per
/// hand-shake file, `up to date` when there is no such line, then one
/// `kept modified file` line per note.
///
/// An item line names each modified file that is kept, each file that
/// `--force` replaces or deletes, and for `skip` each file that causes the
/// skip. Other files that are written or deleted are counted, not named.
fn render_block(preview: &SourcePreview) -> Vec<String> {
    let mut lines = vec![preview.source.clone()];
    lines.extend(preview.items.iter().map(render_item));
    for path in &preview.handshake {
        lines.push(format!("  {:<8} {}", "write", path.display()));
    }
    if preview.items.is_empty() && preview.handshake.is_empty() {
        lines.push("  up to date".to_string());
    }
    for path in &preview.kept_notes {
        lines.push(format!("  kept modified file: {}", path.display()));
    }
    lines
}

/// One item line of a block; see [`render_block`].
fn render_item(item: &upskill::pipeline::PreviewItem) -> String {
    let word = match item.action {
        PreviewAction::Update => "update",
        PreviewAction::Complete => "complete",
        PreviewAction::Remove => "remove",
        PreviewAction::Skip => "skip",
    };
    let mut details: Vec<String> = Vec::new();
    match item.action {
        PreviewAction::Remove => details.push("no longer in source".to_string()),
        PreviewAction::Skip => details.push(format!(
            "modified or not managed by upskill: {}",
            item.blocking
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        PreviewAction::Update | PreviewAction::Complete => {}
    }
    let counted: std::collections::BTreeSet<&PathBuf> = item
        .write
        .iter()
        .chain(&item.delete)
        .filter(|path| !item.forced.contains(path))
        .collect();
    match counted.len() {
        0 => {}
        1 => details.push("1 file".to_string()),
        n => details.push(format!("{n} files")),
    }
    for path in &item.kept_modified {
        if item.action == PreviewAction::Remove {
            details.push(format!("keeps modified {}", path.display()));
        } else {
            details.push(format!(
                "keeps modified {} (no longer produced)",
                path.display()
            ));
        }
    }
    for path in &item.forced {
        // A forced path of a removed item, or one that the update deletes,
        // is deleted; any other forced path is replaced.
        let lost = if item.action == PreviewAction::Remove || item.delete.contains(path) {
            "deletes"
        } else {
            "replaces"
        };
        details.push(format!("{lost} {} (--force)", path.display()));
    }
    if item.record_only {
        details.push("record only".to_string());
    }
    let line = format!(
        "  {word:<8} {} {}   {}",
        item.kind,
        item.name,
        details.join(", ")
    );
    line.trim_end().to_string()
}

/// The result line printed after a source was applied: the non-zero counts
/// of `<n> updated`, `<n> completed`, `<n> removed`, `<n> skipped`,
/// `<n> modified file(s) kept` and `<n> file(s) changed`, separated by
/// `, `, or `applied` when every count is 0.
fn render_result(result: &SourceResult) -> String {
    let mut parts = Vec::new();
    for (n, what) in [
        (result.updated, "updated"),
        (result.completed, "completed"),
        (result.removed, "removed"),
        (result.skipped, "skipped"),
    ] {
        if n > 0 {
            parts.push(format!("{n} {what}"));
        }
    }
    if result.kept_modified > 0 {
        parts.push(format!("{} modified file(s) kept", result.kept_modified));
    }
    if result.changed_files > 0 {
        parts.push(format!("{} file(s) changed", result.changed_files));
    }
    if parts.is_empty() {
        "  applied".to_string()
    } else {
        format!("  {}", parts.join(", "))
    }
}

/// The closing summary of a terminal run: the items updated, completed or
/// removed out of the items considered, and the sources applied out of the
/// sources considered, then ` <k> source(s) not applied.` when a source with
/// changes was not applied, then ` Stopped; <r> source(s) not examined.`
/// (or ` Stopped.` when no source remained) after `q` or end of input.
fn closing_summary(report: &UpdateReport) -> String {
    let changed = report
        .items
        .iter()
        .filter(|i| {
            matches!(
                i.status,
                UpdateStatus::Updated { .. }
                    | UpdateStatus::CompletedInterrupted
                    | UpdateStatus::Removed
            )
        })
        .count();
    let mut line = format!(
        "Updated {changed} of {} item(s) from {} of {} source(s).",
        report.items.len(),
        report.sources_applied,
        report.sources_considered
    );
    if report.sources_not_applied > 0 {
        line.push_str(&format!(
            " {} source(s) not applied.",
            report.sources_not_applied
        ));
    }
    if report.stopped {
        if report.sources_remaining > 0 {
            line.push_str(&format!(
                " Stopped; {} source(s) not examined.",
                report.sources_remaining
            ));
        } else {
            line.push_str(" Stopped.");
        }
    }
    line
}

/// Exit code of a terminal run: 1 when an item was skipped because of
/// modified or unowned files, 0 otherwise.
///
/// Without `--quiet` it prints to stdout, after an empty line, one of:
/// `nothing to update — lockfile is empty` when the run has no item;
/// `everything is up to date — nothing to do` when no source was applied,
/// no source with changes was declined, no file was kept as a note, no item
/// was skipped and no lockfile was adopted; otherwise the closing summary
/// ([`closing_summary`]), followed by `Changed <n> file(s) written or
/// deleted` when files changed but no item was updated, completed or
/// removed. Without `--quiet` the blocks named the kept and the skipped
/// files; with `--quiet` it prints the `skipped` lines of
/// [`print_skipped`] to stderr. With a skipped item it then prints
/// `Use --force to replace.` to stderr.
fn finish_terminal_update(report: &UpdateReport) -> i32 {
    let skipped = report
        .items
        .iter()
        .any(|i| matches!(i.status, UpdateStatus::SkippedModified { .. }));
    if style::is_quiet() {
        print_skipped(report);
    } else if report.items.is_empty() {
        println!("nothing to update — lockfile is empty");
    } else if report.sources_applied == 0
        && report.sources_not_applied == 0
        && report.kept_modified.is_empty()
        && !skipped
        && report.adopted.is_none()
    {
        println!();
        println!("everything is up to date — nothing to do");
    } else {
        println!();
        println!("{}", closing_summary(report));
        let changed_items = report.items.iter().any(|i| {
            matches!(
                i.status,
                UpdateStatus::Updated { .. }
                    | UpdateStatus::CompletedInterrupted
                    | UpdateStatus::Removed
            )
        });
        if !changed_items && !report.changed.is_empty() {
            print_changed_files(report.changed.len());
        }
    }
    if skipped {
        eprintln!("Use --force to replace.");
        EXIT_ERROR
    } else {
        EXIT_SUCCESS
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
    if dry_run {
        // The blocks named the items; count the items that would change.
        let changes = report
            .items
            .iter()
            .filter(|i| match i.status {
                UpdateStatus::NotApplied { action } => action != PreviewAction::Skip,
                _ => false,
            })
            .count();
        println!();
        println!(
            "{} {} of {} item(s)",
            style::warn("Dry-run: would update"),
            changes,
            report.items.len()
        );
        println!("{DRY_RUN_LAST_LINE}");
        return;
    }
    // An applied run counts the items it changed.
    let changes = report
        .items
        .iter()
        .filter(|i| {
            !matches!(
                i.status,
                UpdateStatus::UpToDate
                    | UpdateStatus::SkippedModified { .. }
                    | UpdateStatus::NotApplied { .. }
            )
        })
        .count();
    let skipped = report
        .items
        .iter()
        .any(|i| matches!(i.status, UpdateStatus::SkippedModified { .. }));
    if changes == 0
        && !skipped
        && report.changed.is_empty()
        && report.adopted.is_none()
        && report.sources_not_applied == 0
    {
        // The `kept modified file` notes are printed by `finish_update`; a
        // run that prints them does not say that there was nothing to do.
        if report.kept_modified.is_empty() {
            println!("everything is up to date — nothing to do");
        }
        return;
    }
    println!(
        "{} {} of {} item(s)",
        style::success("Updated"),
        changes,
        report.items.len()
    );
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
            UpdateStatus::Removed => style::error_label("removed — no longer in source"),
            UpdateStatus::CompletedInterrupted => style::success("completed — interrupted before"),
            UpdateStatus::SkippedModified { .. } => {
                style::warn("skipped — modified or unmanaged files kept")
            }
            UpdateStatus::NotApplied { .. } => style::dim("not applied"),
        };
        println!(
            "  {} {:<32} {}",
            style::dim(kind_label),
            style::name(&item.name),
            status
        );
    }
    if !report.changed.is_empty() && changes == 0 {
        print_changed_files(report.changed.len());
    }
    print_forced(&report.forced);
}

/// The modified or unmanaged files that `--force` replaced or deleted.
fn print_forced(forced: &[PathBuf]) {
    if style::is_quiet() || forced.is_empty() {
        return;
    }
    println!(
        "{} {} modified or unmanaged file(s) (--force)",
        style::success("Replaced or deleted"),
        forced.len()
    );
    for path in forced {
        println!("  {}", path.display());
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

    if !report.modified_files.is_empty() {
        println!(
            "{} recorded files modified after install ({} item(s))",
            style::warn("doctor:"),
            report.modified_files.len()
        );
        for m in &report.modified_files {
            println!(
                "  {} {}",
                style::dim(kind_label(m.kind)),
                style::name(&m.name)
            );
            for path in &m.files {
                println!("    - {}", style::dim(&path.display().to_string()));
            }
        }
    }

    if !report.unreadable_files.is_empty() {
        println!(
            "{} recorded files that could not be inspected ({} item(s))",
            style::error_label("doctor:"),
            report.unreadable_files.len()
        );
        for u in &report.unreadable_files {
            println!(
                "  {} {} {}",
                style::dim(kind_label(u.kind)),
                style::name(&u.name),
                style::dim(&format!("({})", u.error))
            );
            for path in &u.files {
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
            "{} {} plugin(s) recorded as installed but missing from client — `upskill update --yes` to fix",
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

    if !report.interrupted.is_empty() {
        println!(
            "{} interrupted add or update ({} item(s)) — run the same command again to complete it",
            style::error_label("doctor:"),
            report.interrupted.len()
        );
        for i in &report.interrupted {
            println!(
                "  {} {}",
                style::dim(kind_label(i.kind)),
                style::name(&i.name)
            );
        }
    }

    print_doctor_orphaned_dependencies(report);
    print_doctor_skipped_plugins(report);
    print_doctor_mcps(report);
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
        style::dim("install the missing CLI then run `upskill update --yes` to install them")
    );
}

fn print_doctor_mcps(report: &DoctorReport) {
    use upskill::pipeline::McpDoctorStatus;

    // Split genuine drift (fails is_clean → exit 1) from "cannot verify"
    // states (informational; the server may be configured via the client's
    // own CLI). Only the former is framed as "needs attention" so a normal
    // doctor run over a CLI-installed target does not look alarming.
    let drifted: Vec<&upskill::pipeline::McpDoctorEntry> = report
        .mcp_entries
        .iter()
        .filter(|e| matches!(e.status, McpDoctorStatus::NotRegistered))
        .collect();
    let unverifiable: Vec<&upskill::pipeline::McpDoctorEntry> = report
        .mcp_entries
        .iter()
        .filter(|e| {
            !matches!(
                e.status,
                McpDoctorStatus::Ok | McpDoctorStatus::NotRegistered
            )
        })
        .collect();

    if !drifted.is_empty() {
        println!(
            "{} {} MCP server(s) need attention",
            style::warn("doctor:"),
            drifted.len()
        );
        for entry in &drifted {
            println!(
                "  MCP '{}' is in the lockfile but not registered in {}; re-run `upskill update --yes`.",
                entry.name, entry.client
            );
        }
    }

    if !unverifiable.is_empty() {
        println!(
            "{} {} MCP server(s) could not be verified",
            style::dim("doctor:"),
            unverifiable.len()
        );
        for entry in &unverifiable {
            match &entry.status {
                McpDoctorStatus::CliNotFound => {
                    println!(
                        "  {} CLI not found; cannot verify MCP '{}'.",
                        entry.client, entry.name
                    );
                }
                McpDoctorStatus::QueryFailed { stderr } => {
                    println!(
                        "  could not query {} MCP list for '{}': {}",
                        entry.client, entry.name, stderr
                    );
                }
                McpDoctorStatus::ConfigMissing => {
                    println!(
                        "  MCP '{}' for {}: config file not found; cannot verify (it may be configured via the {} CLI).",
                        entry.name, entry.client, entry.client
                    );
                }
                McpDoctorStatus::ConfigUnreadable { detail } => {
                    println!(
                        "  MCP '{}' for {}: config file could not be read ({}); fix or re-run `upskill update --yes`.",
                        entry.name, entry.client, detail
                    );
                }
                McpDoctorStatus::Ok | McpDoctorStatus::NotRegistered => {}
            }
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
        if let Some(w) = fresh.warning.as_ref().filter(|_| !style::is_quiet()) {
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
            if let Some(w) = fresh.warning.as_ref().filter(|_| !style::is_quiet()) {
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

/// Unconfigure one MCP entry from its target during `remove-mcp`. CLI-first
/// for targets with a native remove verb (claude, copilot); falls back to
/// deleting the entry from the target's config file when the CLI is absent.
/// VS Code and opencode are config-only. CLI failures warn (never abort);
/// config-file errors propagate to the caller.
fn unconfigure_mcp(
    client: &str,
    name: &str,
    scope: upskill::plugin::PluginScope,
    target: &std::path::Path,
) -> anyhow::Result<()> {
    use upskill::plugin::PluginOutcome;

    // CLI-first for targets with a remove verb; `None` = config-only target.
    let cli_outcome = match client {
        "claude" => Some(upskill::mcp::uninstall_claude(name, scope)),
        "copilot" => Some(upskill::mcp::uninstall_copilot(name)),
        _ => None,
    };

    match cli_outcome {
        // CLI absent, or a config-only target → remove from the config file.
        Some(PluginOutcome::CliNotFound) | None => match client {
            "claude" => upskill::ancillary::remove_claude_mcp(target, name)?,
            "copilot" => upskill::ancillary::remove_copilot_mcp(name)?,
            "vscode" => upskill::ancillary::remove_vscode_mcp(target, name)?,
            "opencode" => upskill::ancillary::remove_opencode_mcp(target, name)?,
            _ => {}
        },
        Some(PluginOutcome::Failed { stderr, exit_code }) => {
            eprintln!(
                "{} failed to remove MCP '{}' from {} (exit {:?}): {}",
                style::warn("warning:"),
                name,
                client,
                exit_code,
                stderr.trim()
            );
        }
        Some(_) => {}
    }
    Ok(())
}

fn run_remove_mcp(name: &str, global: bool, project: bool) -> i32 {
    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    let mcp_scope = scope_to_plugin_scope(global, project);

    let mut lock = match upskill::lockfile::Lockfile::load(&target) {
        Ok(l) => l,
        Err(err) => {
            print_error_chain(&err);
            return EXIT_ERROR;
        }
    };

    let matching_mcps: Vec<upskill::lockfile::LockedMcp> = lock
        .mcps
        .iter()
        .filter(|m| m.name == name)
        .cloned()
        .collect();

    if matching_mcps.is_empty() {
        print_error(format!("MCP '{}' not found in lockfile", name));
        return EXIT_ERROR;
    }

    for mcp in &matching_mcps {
        if let Err(err) = unconfigure_mcp(&mcp.client, &mcp.name, mcp_scope, &target) {
            print_error_chain(&err);
            return EXIT_ERROR;
        }
    }

    lock.remove_mcps_by_name(name);
    if let Err(err) = lock.save(&target) {
        print_error_chain(&err);
        return EXIT_ERROR;
    }

    if !style::is_quiet() {
        println!(
            "{} MCP server '{}'",
            style::success("Removed"),
            style::name(name)
        );
    }

    EXIT_SUCCESS
}

fn run_index(registry: Option<&str>, clear: bool) -> i32 {
    if clear {
        let dir = index::cache_dir();
        if dir.exists() {
            if let Err(err) = std::fs::remove_dir_all(&dir) {
                eprintln!("error: failed to clear index cache: {}", err);
                return EXIT_ERROR;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use upskill::pipeline::PreviewItem;

    fn item(action: PreviewAction) -> PreviewItem {
        PreviewItem {
            kind: ItemKind::Skill,
            name: "tool".into(),
            action,
            write: Vec::new(),
            delete: Vec::new(),
            kept_modified: Vec::new(),
            forced: Vec::new(),
            blocking: Vec::new(),
            record_only: false,
        }
    }

    fn preview(items: Vec<PreviewItem>) -> SourcePreview {
        SourcePreview {
            source: "local:/src".into(),
            items,
            kept_notes: Vec::new(),
            handshake: Vec::new(),
            adopted: None,
        }
    }

    fn p(path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    #[test]
    fn parse_answer_follows_the_table() {
        for line in ["y", "yes", "Y", "YeS", "  y \n", "yes\r\n"] {
            assert_eq!(parse_answer(Some(line)), SourceDecision::Apply, "{line:?}");
        }
        for line in ["q", "quit", "Q", " QUIT \n"] {
            assert_eq!(parse_answer(Some(line)), SourceDecision::Stop, "{line:?}");
        }
        for line in ["n", "no", "N", "", "\n", "maybe", "yy"] {
            assert_eq!(parse_answer(Some(line)), SourceDecision::Skip, "{line:?}");
        }
        assert_eq!(parse_answer(None), SourceDecision::Stop);
    }

    #[test]
    fn render_block_of_a_source_without_changes_says_up_to_date() {
        assert_eq!(
            render_block(&preview(Vec::new())),
            vec!["local:/src", "  up to date"]
        );
    }

    #[test]
    fn render_block_counts_unchanged_files_and_names_kept_ones() {
        let mut update = item(PreviewAction::Update);
        update.write = vec![p("a.md"), p("b.md")];
        update.delete = vec![p("c.md")];
        update.kept_modified = vec![p("notes.md")];
        assert_eq!(
            render_block(&preview(vec![update]))[1],
            "  update   skill tool   3 files, keeps modified notes.md (no longer produced)"
        );
    }

    #[test]
    fn render_block_names_a_forced_file_and_does_not_count_it() {
        let mut update = item(PreviewAction::Update);
        update.write = vec![p("a.md"), p("b.md")];
        update.forced = vec![p("a.md")];
        assert_eq!(
            render_block(&preview(vec![update]))[1],
            "  update   skill tool   1 file, replaces a.md (--force)"
        );
    }

    #[test]
    fn render_item_says_deletes_for_a_forced_file_that_the_update_deletes() {
        let mut update = item(PreviewAction::Update);
        update.write = vec![p("a.md")];
        update.delete = vec![p("b.md")];
        update.forced = vec![p("b.md")];
        assert_eq!(
            render_item(&update),
            "  update   skill tool   1 file, deletes b.md (--force)"
        );
    }

    #[test]
    fn render_block_of_complete_remove_skip_and_record_only() {
        let mut complete = item(PreviewAction::Complete);
        complete.write = vec![p("a.md")];
        let mut remove = item(PreviewAction::Remove);
        remove.delete = vec![p("a.md")];
        remove.kept_modified = vec![p("b.md")];
        remove.forced = vec![p("c.md")];
        let mut skip = item(PreviewAction::Skip);
        skip.blocking = vec![p("a.md"), p("b.md")];
        let mut record = item(PreviewAction::Update);
        record.record_only = true;
        let lines = render_block(&preview(vec![complete, remove, skip, record]));
        assert_eq!(
            lines[1..],
            [
                "  complete skill tool   1 file",
                "  remove   skill tool   no longer in source, 1 file, keeps modified b.md, \
                 deletes c.md (--force)",
                "  skip     skill tool   modified or not managed by upskill: a.md, b.md",
                "  update   skill tool   record only",
            ]
        );
    }

    #[test]
    fn render_block_lists_handshake_files_then_notes() {
        let mut source = preview(Vec::new());
        source.handshake = vec![p("CLAUDE.md")];
        source.kept_notes = vec![p("style.md")];
        assert_eq!(
            render_block(&source),
            vec![
                "local:/src",
                "  write    CLAUDE.md",
                "  kept modified file: style.md",
            ]
        );
    }

    #[test]
    fn render_block_of_a_note_only_source_says_up_to_date_first() {
        let mut source = preview(Vec::new());
        source.kept_notes = vec![p("style.md")];
        assert_eq!(
            render_block(&source),
            vec![
                "local:/src",
                "  up to date",
                "  kept modified file: style.md"
            ]
        );
    }

    #[test]
    fn render_result_lists_the_non_zero_counts() {
        let result = SourceResult {
            source: "s".into(),
            updated: 2,
            removed: 1,
            kept_modified: 1,
            ..SourceResult::default()
        };
        assert_eq!(
            render_result(&result),
            "  2 updated, 1 removed, 1 modified file(s) kept"
        );
        assert_eq!(render_result(&SourceResult::default()), "  applied");
        let files_only = SourceResult {
            changed_files: 2,
            ..SourceResult::default()
        };
        assert_eq!(render_result(&files_only), "  2 file(s) changed");
        let both = SourceResult {
            updated: 1,
            changed_files: 1,
            ..SourceResult::default()
        };
        assert_eq!(render_result(&both), "  1 updated, 1 file(s) changed");
    }

    #[test]
    fn closing_summary_counts_items_and_sources() {
        let mut report = UpdateReport {
            sources_considered: 2,
            sources_applied: 1,
            ..UpdateReport::default()
        };
        for status in [
            UpdateStatus::Updated {
                old_hash: None,
                new_hash: None,
            },
            UpdateStatus::NotApplied {
                action: PreviewAction::Update,
            },
        ] {
            report.items.push(upskill::pipeline::UpdatedItem {
                kind: ItemKind::Skill,
                name: "x".into(),
                source: "s".into(),
                status,
            });
        }
        assert_eq!(
            closing_summary(&report),
            "Updated 1 of 2 item(s) from 1 of 2 source(s)."
        );
        report.sources_not_applied = 1;
        assert_eq!(
            closing_summary(&report),
            "Updated 1 of 2 item(s) from 1 of 2 source(s). 1 source(s) not applied."
        );
        report.stopped = true;
        assert_eq!(
            closing_summary(&report),
            "Updated 1 of 2 item(s) from 1 of 2 source(s). 1 source(s) not applied. Stopped."
        );
        report.sources_remaining = 3;
        assert_eq!(
            closing_summary(&report),
            "Updated 1 of 2 item(s) from 1 of 2 source(s). 1 source(s) not applied. \
             Stopped; 3 source(s) not examined."
        );
    }
}
