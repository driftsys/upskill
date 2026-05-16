//! Clap derive definition for the `upskill` CLI.
//!
//! Lives in the library crate so the man-page generator
//! (`examples/mangen.rs`) and integration tests can reach the same
//! `Command` tree the binary parses against. `main.rs` matches on
//! `Commands` and dispatches.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "upskill")]
#[command(version)]
#[command(about = "Author and distribute AI-assistance content across coding agents")]
#[command(
    after_help = "DOCUMENTATION:\n  https://driftsys.github.io/upskill/\n\n\
        REPORT BUGS:\n  https://github.com/driftsys/upskill/issues"
)]
pub struct Cli {
    /// Disable colored output. Honored alongside `NO_COLOR`,
    /// `UPSKILL_NO_COLOR`, `TERM=dumb`, and TTY auto-detection.
    #[arg(long = "no-color", global = true)]
    pub no_color: bool,
    /// Suppress informational stdout. Errors on stderr and exit codes
    /// are unaffected. Useful for CI use of `lint`, `update --dry-run`,
    /// etc.
    #[arg(short = 'q', long = "quiet", global = true)]
    pub quiet: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Install rules / skills / agents from a source.
    ///
    /// Parses each item from the source, renders per-client output, and
    /// records the install in `.upskill-lock.json`. Default scope is the
    /// current project; falls back to global (`$HOME`) when `cwd` is not
    /// inside a git repo.
    #[command(after_help = "EXAMPLES:\n  \
            upskill add owner/repo\n  \
            upskill add owner/repo@v1.2\n  \
            upskill add owner/repo:skills/code-review\n  \
            upskill add owner/repo code-review secret-scanner\n  \
            upskill add gitlab:team/repo\n  \
            upskill add ./local-source\n  \
            upskill add owner/repo --global")]
    Add {
        /// Source: `owner/repo[@ref][:subfolder]`, full https URL, or local path.
        source: String,
        /// Optional subset filter — only items whose name matches one of
        /// these is installed. Empty means install everything in the
        /// source (the default).
        items: Vec<String>,
        /// Install into `$HOME` instead of the current directory.
        #[arg(short = 'g', long = "global", conflicts_with = "project")]
        global: bool,
        /// Force project scope (current directory). Overrides the auto-detect
        /// fallback to global when `cwd` is not inside a git repo.
        #[arg(short = 'p', long = "project")]
        project: bool,
    },
    /// Remove installed content.
    ///
    /// Either name one or more items, or pass `--source <label>` to
    /// remove every item that came from a single source. Bare
    /// `upskill remove` is rejected — be explicit. Ancillary files
    /// (`CLAUDE.md`, `opencode.json`, `.vscode/settings.json`) are not
    /// touched.
    #[command(after_help = "EXAMPLES:\n  \
            upskill remove code-review\n  \
            upskill remove rule-a skill-b agent-c\n  \
            upskill remove --source github:owner/repo\n  \
            upskill remove --global code-review")]
    Remove {
        /// Item names to remove. Mutually exclusive with `--source`.
        names: Vec<String>,
        /// Remove every item whose lockfile `source` label matches this
        /// string. Use the value reported by the install or shown in
        /// `.upskill-lock.json` (e.g. `local:/path` or
        /// `github:owner/repo`).
        #[arg(short = 's', long = "source")]
        source: Option<String>,
        /// Operate on `$HOME` instead of the current directory.
        #[arg(short = 'g', long = "global", conflicts_with = "project")]
        global: bool,
        /// Force project scope (current directory). Overrides the auto-detect
        /// fallback to global when `cwd` is not inside a git repo.
        #[arg(short = 'p', long = "project")]
        project: bool,
        /// Skip the confirmation prompt that `--source` shows on a TTY.
        /// Already implicit when stdin is not a terminal (CI / pipes).
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// Pull latest sources and regenerate changed items.
    ///
    /// Re-fetches the source for every (or just the named) lockfile
    /// entries and reinstalls those sources. With `--dry-run`, hashes
    /// the new SSOT and reports what would change without writing.
    /// `update` always fetches.
    #[command(after_help = "EXAMPLES:\n  \
            upskill update\n  \
            upskill update code-review\n  \
            upskill update --dry-run\n  \
            upskill update --global")]
    Update {
        /// Item names to update (omit to update everything).
        names: Vec<String>,
        /// Report what would change without writing.
        #[arg(short = 'n', long = "dry-run")]
        dry_run: bool,
        /// Operate on `$HOME` instead of the current directory.
        #[arg(short = 'g', long = "global", conflicts_with = "project")]
        global: bool,
        /// Force project scope (current directory). Overrides the auto-detect
        /// fallback to global when `cwd` is not inside a git repo.
        #[arg(short = 'p', long = "project")]
        project: bool,
    },
    /// List installed content recorded in `.upskill-lock.json`.
    ///
    /// Items are grouped by kind (rules, skills, agents). Bundles, when
    /// present, are surfaced as a separate section. The command never
    /// fetches and never inspects per-client output files — for that, run
    /// `upskill doctor`.
    #[command(after_help = "EXAMPLES:\n  \
            upskill list\n  \
            upskill list --global")]
    List {
        /// Read `$HOME/.upskill-lock.json` instead of the current directory.
        #[arg(short = 'g', long = "global", conflicts_with = "project")]
        global: bool,
        /// Force project scope (current directory). Overrides the auto-detect
        /// fallback to global when `cwd` is not inside a git repo.
        #[arg(short = 'p', long = "project")]
        project: bool,
        /// Emit a stable JSON document instead of the human-readable
        /// grouping. See `docs/commands.md` for the schema.
        #[arg(long = "json")]
        json: bool,
    },
    /// Verify installed-state consistency.
    ///
    /// Three independent drift buckets:
    /// - missing per-client output files (reinstall fixes)
    /// - SSOT hash drift on `local:` sources (`update` fixes)
    /// - lockfile entries with no recoverable source (manual `remove`)
    ///
    /// Doctor never fetches; remote-source drift detection is
    /// `update --dry-run`. Exit 0 when clean, 1 when any drift is found.
    #[command(after_help = "EXAMPLES:\n  \
            upskill doctor\n  \
            upskill doctor --global")]
    Doctor {
        /// Operate on `$HOME` instead of the current directory.
        #[arg(short = 'g', long = "global", conflicts_with = "project")]
        global: bool,
        /// Force project scope (current directory). Overrides the auto-detect
        /// fallback to global when `cwd` is not inside a git repo.
        #[arg(short = 'p', long = "project")]
        project: bool,
        /// Emit the three drift buckets as a stable JSON document. Exit
        /// code is unchanged (0 clean, 1 drifted). See `docs/commands.md`
        /// for the schema.
        #[arg(long = "json")]
        json: bool,
    },
    /// Search the public skills registry.
    #[command(after_help = "EXAMPLES:\n  \
            upskill search code-review\n  \
            upskill search api --limit 5")]
    Search {
        /// Search query.
        query: String,
        /// Maximum number of results.
        #[arg(short = 'l', long, default_value = "10")]
        limit: usize,
    },
    /// Validate SSOT files against the format spec.
    ///
    /// Author command — runs only inside a source registry. Refuses to
    /// run inside a consumer project (detected by `.upskill-lock.json`).
    /// Default mode emits warnings and exits 0 unless an error rule
    /// fires; `--strict` promotes warnings to errors (CI mode). With no
    /// paths, lints the current directory.
    #[command(after_help = "EXAMPLES:\n  \
            upskill lint\n  \
            upskill lint rules/\n  \
            upskill lint --strict")]
    Lint {
        /// Files or directories to lint. Empty = current directory.
        paths: Vec<PathBuf>,
        /// Promote warnings to errors. Use in CI.
        #[arg(short = 's', long)]
        strict: bool,
    },
    /// Canonicalise YAML frontmatter in SSOT files.
    ///
    /// Author command — runs only inside a source registry. Body
    /// markdown is left untouched (dprint's job). Refuses to run inside
    /// a consumer project (detected by `.upskill-lock.json`). With no
    /// paths, formats the current directory.
    #[command(after_help = "EXAMPLES:\n  \
            upskill fmt\n  \
            upskill fmt rules/")]
    Fmt {
        /// Files or directories to format. Empty = current directory.
        paths: Vec<PathBuf>,
    },
    /// Scaffold a new rule, skill, or agent.
    ///
    /// Writes the minimum frontmatter required plus kind-specific
    /// defaults (e.g. `mode: subagent` / `model: sonnet` for agents)
    /// into `<cwd>/<kind>s/<name>/<KIND>.md`. Author command — refuses
    /// to run inside a consumer project.
    #[command(after_help = "EXAMPLES:\n  \
            upskill new rule no-direct-database-access\n  \
            upskill new skill code-review\n  \
            upskill new agent security-reviewer")]
    New {
        /// One of `rule`, `skill`, `agent`.
        kind: String,
        /// Item name. Lowercase letters, digits, hyphens; max 64 chars.
        name: String,
    },
    /// Author a registry: generate `.upskill-registry.json`.
    ///
    /// Author command — refuses to run inside a consumer project
    /// (detected by `.upskill-lock.json`). Use `build` to write the
    /// manifest; run `build --check` in CI to verify it is current.
    Registry {
        #[command(subcommand)]
        command: RegistryCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum RegistryCommands {
    /// Scan the registry tree and write `.upskill-registry.json`.
    ///
    /// Reads `REGISTRY.md` identity, enumerates `RULE/SKILL/AGENT.md`
    /// items and `*.bundle.md` bundles, writes a deterministic manifest.
    /// Author command — refuses to run inside a consumer project.
    #[command(after_help = "EXAMPLES:\n  \
            upskill registry build\n  \
            upskill registry build --check")]
    Build {
        /// Verify the on-disk manifest is up to date without writing.
        /// Exit 1 if stale. Use in CI.
        #[arg(long = "check")]
        check: bool,
    },
}
