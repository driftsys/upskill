//! Client CLI shellout for plugin installation (ADR-0008).
//!
//! Plugins are installed by shelling out to each client's native CLI:
//! - Claude Code: `claude plugin marketplace add` + `claude plugin install`
//! - Copilot CLI: `copilot plugin marketplace add` + `copilot plugin install`
//! - VS Code: `code --install-extension`
//! - opencode: `opencode plugin add` (opencode 2 or later; the major
//!   version is read with `opencode --version` before any `opencode plugin`
//!   command)
//!
//! This module exposes typed install/uninstall functions per client.
//! All functions return structured results (never write to stdout/stderr)
//! and handle CLI-not-found gracefully per the warn-skip policy.

use std::io::ErrorKind;
use std::process::Command;

/// Scope for Claude plugin installation, derived from upskill's
/// project/global flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginScope {
    /// Project-scoped install (`claude plugin install --scope project`).
    Project,
    /// User-scoped install (`claude plugin install --scope user`).
    User,
}

impl PluginScope {
    /// The plugin scope of an install scope: the scope that `resolve_scope`
    /// returned, so the flags, the git-repo detection and the home-directory
    /// refusal are decided in one place. It also selects the opencode config
    /// file, for the scope whose lockfile a run works on.
    pub fn of(scope: crate::scope::Scope) -> Self {
        match scope {
            crate::scope::Scope::Project => Self::Project,
            crate::scope::Scope::Global => Self::User,
        }
    }

    /// Returns the CLI flag value for `claude plugin install --scope`.
    pub fn as_claude_flag(&self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::User => "user",
        }
    }
}

/// Outcome of a plugin install or uninstall attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginOutcome {
    /// Command executed successfully (exit code 0).
    Success,
    /// Client CLI not found on PATH — plugin skipped.
    CliNotFound,
    /// Client CLI found but command returned non-zero.
    Failed {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// Plugin requires manual installation via instructions URL.
    ManualInstructions,
    /// Client CLI found, but its version is below the version that upskill
    /// needs, or `--version` did not print a version. No plugin command was
    /// run — plugin skipped.
    UnsupportedVersion {
        /// The version that upskill needs, for example `opencode 2`.
        required: &'static str,
        /// The first line that `--version` printed; `None` when it printed
        /// nothing or failed.
        found: Option<String>,
    },
}

impl PluginOutcome {
    /// True when the plugin was successfully installed/uninstalled.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// True when the CLI was not found (warn-skip scenario).
    pub fn is_cli_not_found(&self) -> bool {
        matches!(self, Self::CliNotFound)
    }

    /// True when the plugin has manual installation instructions.
    pub fn is_manual_instructions(&self) -> bool {
        matches!(self, Self::ManualInstructions)
    }

    /// True when the CLI version is below the needed version.
    pub fn is_unsupported_version(&self) -> bool {
        matches!(self, Self::UnsupportedVersion { .. })
    }
}

// ---------------------------------------------------------------------------
// Install functions
// ---------------------------------------------------------------------------

/// Install a Claude Code plugin via `claude plugin marketplace add` followed
/// by `claude plugin install`. The marketplace-add step is idempotent.
///
/// `source` is the `owner/repo`/URL/path passed to `marketplace add`;
/// `marketplace` is the marketplace **name** the CLI requires in the install
/// ref `<plugin>@<marketplace>` (the CLI derives the name from the marketplace
/// manifest, so it differs from `source`).
pub fn install_claude_plugin(
    source: &str,
    marketplace: &str,
    plugin: &str,
    scope: PluginScope,
) -> PluginOutcome {
    // Step 1: Add marketplace source (idempotent).
    let result = run_command("claude", &["plugin", "marketplace", "add", source]);
    match result {
        PluginOutcome::CliNotFound => return PluginOutcome::CliNotFound,
        PluginOutcome::Failed { .. } => return result,
        PluginOutcome::Success => {}
        PluginOutcome::ManualInstructions | PluginOutcome::UnsupportedVersion { .. } => {
            unreachable!()
        }
    }

    // Step 2: Install plugin with scope. The install ref uses the marketplace
    // NAME, not the source — `claude plugin install` resolves `<plugin>@<name>`.
    let install_ref = format!("{plugin}@{marketplace}");
    run_command(
        "claude",
        &[
            "plugin",
            "install",
            &install_ref,
            "--scope",
            scope.as_claude_flag(),
        ],
    )
}

/// Install a VS Code extension via `code --install-extension`.
pub fn install_vscode_extension(extension: &str) -> PluginOutcome {
    run_command("code", &["--install-extension", extension])
}

/// Install an opencode package via `opencode plugin add <package>`.
///
/// Runs `opencode --version` first and runs no `opencode plugin` command
/// when the major version is below 2 or cannot be read (see
/// `opencode_version`).
pub fn install_opencode_plugin(module: &str) -> PluginOutcome {
    match opencode_version() {
        OpencodeVersion::NotFound => PluginOutcome::CliNotFound,
        OpencodeVersion::Unsupported { found } => PluginOutcome::UnsupportedVersion {
            required: OPENCODE_REQUIRED,
            found,
        },
        OpencodeVersion::Supported => run_command("opencode", &["plugin", "add", module]),
    }
}

/// The opencode version that upskill needs for its `opencode plugin`
/// commands, as shown to the user.
pub const OPENCODE_REQUIRED: &str = "opencode 2";

/// The lowest opencode major version whose plugin commands upskill uses
/// (`opencode plugin add`, `opencode plugin list`). upskill runs no
/// `opencode plugin remove`: `remove` only edits `opencode.json`.
const OPENCODE_MIN_MAJOR: u64 = 2;

/// Result of `opencode --version`, read before any `opencode plugin` command.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OpencodeVersion {
    /// `opencode` is not on PATH.
    NotFound,
    /// The major version is [`OPENCODE_MIN_MAJOR`] or higher.
    Supported,
    /// The major version is lower, or no version could be read.
    Unsupported { found: Option<String> },
}

/// Run `opencode --version` and compare its major version with
/// [`OPENCODE_MIN_MAJOR`]. opencode 1.x reads `opencode plugin <word>` as an
/// install of the module `<word>`, so no `opencode plugin` command is run
/// unless the version is known to be supported.
fn opencode_version() -> OpencodeVersion {
    match run_command_output("opencode", &["--version"]) {
        CommandOutput::CliNotFound => OpencodeVersion::NotFound,
        CommandOutput::Failed { .. } => OpencodeVersion::Unsupported { found: None },
        CommandOutput::Success { stdout } => {
            let first = stdout.lines().next().unwrap_or("").trim().to_string();
            match parse_major_version(&first) {
                Some(major) if major >= OPENCODE_MIN_MAJOR => OpencodeVersion::Supported,
                _ => OpencodeVersion::Unsupported {
                    found: (!first.is_empty()).then_some(first),
                },
            }
        }
    }
}

/// The major version in a `--version` line such as `opencode v2.1.4` or
/// `2.1.4`: the leading digits of the first word that starts with a digit,
/// after an optional `v`.
fn parse_major_version(line: &str) -> Option<u64> {
    line.split_whitespace().find_map(|word| {
        let word = word.strip_prefix('v').unwrap_or(word);
        let digits: String = word.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            None
        } else {
            digits.parse().ok()
        }
    })
}

/// Install a GitHub Copilot CLI plugin via `copilot plugin marketplace add`
/// followed by `copilot plugin install`. The marketplace-add step is
/// idempotent. Unlike Claude, Copilot CLI does not support a `--scope` flag.
pub fn install_copilot_plugin(source: &str, marketplace: &str, plugin: &str) -> PluginOutcome {
    // Step 1: Add marketplace source (idempotent).
    let result = run_command("copilot", &["plugin", "marketplace", "add", source]);
    match result {
        PluginOutcome::CliNotFound => return PluginOutcome::CliNotFound,
        PluginOutcome::Failed { .. } => return result,
        PluginOutcome::Success => {}
        PluginOutcome::ManualInstructions | PluginOutcome::UnsupportedVersion { .. } => {
            unreachable!()
        }
    }

    // Step 2: Install plugin from marketplace. The install ref uses the
    // marketplace NAME, not the source.
    let install_ref = format!("{plugin}@{marketplace}");
    run_command("copilot", &["plugin", "install", &install_ref])
}

// ---------------------------------------------------------------------------
// Uninstall functions
// ---------------------------------------------------------------------------

/// Uninstall a Claude Code plugin. `marketplace` is the marketplace name used
/// in the install ref `<plugin>@<marketplace>` (see [`install_claude_plugin`]).
pub fn uninstall_claude_plugin(
    plugin: &str,
    marketplace: &str,
    scope: PluginScope,
) -> PluginOutcome {
    let install_ref = format!("{plugin}@{marketplace}");
    run_command(
        "claude",
        &[
            "plugin",
            "uninstall",
            &install_ref,
            "--scope",
            scope.as_claude_flag(),
        ],
    )
}

/// Uninstall a VS Code extension.
pub fn uninstall_vscode_extension(extension: &str) -> PluginOutcome {
    run_command("code", &["--uninstall-extension", extension])
}

/// Uninstall a GitHub Copilot CLI plugin. `marketplace` is the marketplace
/// name used in the install ref `<plugin>@<marketplace>`.
pub fn uninstall_copilot_plugin(plugin: &str, marketplace: &str) -> PluginOutcome {
    let install_ref = format!("{plugin}@{marketplace}");
    run_command("copilot", &["plugin", "uninstall", &install_ref])
}

// ---------------------------------------------------------------------------
// CLI availability check
// ---------------------------------------------------------------------------

/// Returns `true` if the named CLI binary is available on PATH.
///
/// Uses `Command::new(cli).arg("--version")` and checks for
/// `ErrorKind::NotFound`. Does not validate the command succeeds — only
/// that the binary can be spawned.
pub fn is_cli_available(cli: &str) -> bool {
    match spawn_command(cli, &["--version"]) {
        Ok(out) if is_command_not_found(&out) => false,
        Ok(_) => true,
        Err(e) if e.kind() == ErrorKind::NotFound => false,
        // Other errors (e.g., permission denied) — the binary exists but
        // can't be run. Treat as "available but broken" so the install
        // attempt surfaces the real error.
        Err(_) => true,
    }
}

// ---------------------------------------------------------------------------
// Plugin list / reconciliation (ADR-0008 / issue #151)
// ---------------------------------------------------------------------------

/// Result of checking whether a specific plugin is currently installed in
/// the client.  Used by `doctor` for plugin reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginCheckResult {
    /// Plugin found in the client's installed list.
    Installed,
    /// Plugin NOT found — present in lockfile but absent from client.
    NotInstalled,
    /// Client CLI not found on PATH — install state cannot be determined.
    CliNotFound,
    /// CLI present but the list command returned non-zero.
    QueryFailed {
        exit_code: Option<i32>,
        stderr: String,
    },
}

impl PluginCheckResult {
    /// True when the plugin is confirmed installed.
    pub fn is_installed(&self) -> bool {
        matches!(self, Self::Installed)
    }

    /// True when the plugin is confirmed NOT installed.
    pub fn is_not_installed(&self) -> bool {
        matches!(self, Self::NotInstalled)
    }

    /// True when the CLI was not found on PATH.
    pub fn is_cli_not_found(&self) -> bool {
        matches!(self, Self::CliNotFound)
    }
}

/// Check whether a Claude Code plugin is installed for the given scope.
///
/// Runs `claude plugin list --scope <scope>` and searches for `plugin_name`
/// as a substring of any output line.
pub fn check_claude_plugin_installed(plugin_name: &str, scope: PluginScope) -> PluginCheckResult {
    let out = run_command_output(
        "claude",
        &["plugin", "list", "--scope", scope.as_claude_flag()],
    );
    check_output_for_substring(out, plugin_name)
}

/// Check whether a VS Code extension is installed.
///
/// Runs `code --list-extensions` and checks for the extension ID as an
/// exact (case-insensitive) line match.
pub fn check_vscode_extension_installed(extension_id: &str) -> PluginCheckResult {
    let out = run_command_output("code", &["--list-extensions"]);
    check_output_for_exact_line(out, extension_id)
}

/// Check whether an opencode plugin is installed.
///
/// Runs `opencode plugin list` and searches for `module_name` as a
/// substring of any output line. Runs `opencode --version` first; when the
/// major version is below 2 or cannot be read, runs no `opencode plugin`
/// command and returns [`PluginCheckResult::QueryFailed`] (the state cannot
/// be checked).
pub fn check_opencode_plugin_installed(module_name: &str) -> PluginCheckResult {
    match opencode_version() {
        OpencodeVersion::NotFound => PluginCheckResult::CliNotFound,
        OpencodeVersion::Unsupported { found } => PluginCheckResult::QueryFailed {
            exit_code: None,
            stderr: format!(
                "{OPENCODE_REQUIRED} or later is needed to list plugins; found: {}",
                found.as_deref().unwrap_or("no version")
            ),
        },
        OpencodeVersion::Supported => {
            let out = run_command_output("opencode", &["plugin", "list"]);
            check_output_for_substring(out, module_name)
        }
    }
}

// ---------------------------------------------------------------------------
// Internal command output helpers
// ---------------------------------------------------------------------------

/// Captured result of a CLI invocation.
pub(crate) enum CommandOutput {
    Success {
        stdout: String,
    },
    CliNotFound,
    Failed {
        exit_code: Option<i32>,
        stderr: String,
    },
}

pub(crate) fn run_command_output(program: &str, args: &[&str]) -> CommandOutput {
    let result = spawn_command(program, args);
    match result {
        Ok(out) if out.status.success() => CommandOutput::Success {
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        },
        Ok(out) if is_command_not_found(&out) => CommandOutput::CliNotFound,
        Ok(out) => CommandOutput::Failed {
            exit_code: out.status.code(),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        },
        Err(e) if e.kind() == ErrorKind::NotFound => CommandOutput::CliNotFound,
        Err(e) => CommandOutput::Failed {
            exit_code: None,
            stderr: format!("failed to spawn {program}: {e}"),
        },
    }
}

/// Spawn a command, using `cmd /c` on Windows so that PATHEXT resolution
/// finds `.cmd` and `.bat` wrappers (e.g. `code.cmd` for VS Code).
fn spawn_command(program: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    {
        let mut cmd_args = vec!["/c", program];
        cmd_args.extend(args);
        Command::new("cmd").args(&cmd_args).output()
    }
    #[cfg(not(windows))]
    {
        Command::new(program).args(args).output()
    }
}

/// On Windows, `cmd /c nonexistent` prints "is not recognized" to stderr.
/// The exit code varies by Windows version (1 or 9009), so we detect
/// the stderr message instead.
fn is_command_not_found(output: &std::process::Output) -> bool {
    #[cfg(windows)]
    {
        let stderr = String::from_utf8_lossy(&output.stderr);
        stderr.contains("is not recognized")
    }
    #[cfg(not(windows))]
    {
        let _ = output;
        false
    }
}

/// Map `CommandOutput` to `PluginCheckResult` by searching for `needle` as
/// a substring of any stdout line.
pub(crate) fn check_output_for_substring(output: CommandOutput, needle: &str) -> PluginCheckResult {
    match output {
        CommandOutput::CliNotFound => PluginCheckResult::CliNotFound,
        CommandOutput::Failed { exit_code, stderr } => {
            PluginCheckResult::QueryFailed { exit_code, stderr }
        }
        CommandOutput::Success { stdout } => {
            if stdout.lines().any(|line| line.contains(needle)) {
                PluginCheckResult::Installed
            } else {
                PluginCheckResult::NotInstalled
            }
        }
    }
}

/// Map `CommandOutput` to `PluginCheckResult` by checking for `needle` as an
/// exact (case-insensitive, trimmed) line in stdout.
fn check_output_for_exact_line(output: CommandOutput, needle: &str) -> PluginCheckResult {
    match output {
        CommandOutput::CliNotFound => PluginCheckResult::CliNotFound,
        CommandOutput::Failed { exit_code, stderr } => {
            PluginCheckResult::QueryFailed { exit_code, stderr }
        }
        CommandOutput::Success { stdout } => {
            if stdout
                .lines()
                .any(|line| line.trim().eq_ignore_ascii_case(needle))
            {
                PluginCheckResult::Installed
            } else {
                PluginCheckResult::NotInstalled
            }
        }
    }
}

/// Execute a CLI command and map the result to a `PluginOutcome`.
pub(crate) fn run_command(program: &str, args: &[&str]) -> PluginOutcome {
    let output = match spawn_command(program, args) {
        Ok(output) if is_command_not_found(&output) => {
            return PluginOutcome::CliNotFound;
        }
        Ok(output) => output,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return PluginOutcome::CliNotFound;
        }
        Err(e) => {
            return PluginOutcome::Failed {
                exit_code: None,
                stderr: format!("failed to spawn {program}: {e}"),
            };
        }
    };

    if output.status.success() {
        PluginOutcome::Success
    } else {
        PluginOutcome::Failed {
            exit_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_major_version_reads_the_forms_of_opencode() {
        assert_eq!(parse_major_version("opencode v2.1.4"), Some(2));
        assert_eq!(parse_major_version("opencode v1.15.13"), Some(1));
        assert_eq!(parse_major_version("2.1.4"), Some(2));
        assert_eq!(parse_major_version("v10.0.0"), Some(10));
        assert_eq!(parse_major_version("opencode"), None);
        assert_eq!(parse_major_version(""), None);
    }

    #[test]
    fn plugin_scope_as_claude_flag() {
        assert_eq!(PluginScope::Project.as_claude_flag(), "project");
        assert_eq!(PluginScope::User.as_claude_flag(), "user");
    }

    #[test]
    fn is_cli_available_returns_true_for_known_binary() {
        // `sh` is always available on Unix.
        assert!(is_cli_available("sh"));
    }

    #[test]
    fn is_cli_available_returns_false_for_nonexistent_binary() {
        assert!(!is_cli_available(
            "nonexistent-binary-that-does-not-exist-xyz-42"
        ));
    }

    #[test]
    fn install_claude_returns_cli_not_found_when_binary_missing() {
        // Using a descriptor that references a nonexistent binary would
        // require overriding the program name. Instead, we test via
        // run_command directly.
        let result = run_command(
            "nonexistent-claude-xyz-42",
            &["plugin", "marketplace", "add", "test-source"],
        );
        assert_eq!(result, PluginOutcome::CliNotFound);
    }

    #[test]
    fn install_vscode_returns_cli_not_found_when_binary_missing() {
        // code is likely not on PATH in CI; if it is, this still works
        // because --install-extension with a fake extension just fails.
        let result = run_command(
            "nonexistent-code-xyz-42",
            &["--install-extension", "test.extension"],
        );
        assert_eq!(result, PluginOutcome::CliNotFound);
    }

    #[test]
    fn install_opencode_returns_cli_not_found_when_binary_missing() {
        let result = run_command("nonexistent-opencode-xyz-42", &["plugin", "test-module"]);
        assert_eq!(result, PluginOutcome::CliNotFound);
    }

    #[test]
    fn run_command_returns_success_on_zero_exit() {
        // `true` always exits 0 on Unix.
        let result = run_command("true", &[]);
        assert_eq!(result, PluginOutcome::Success);
    }

    #[test]
    fn run_command_returns_failed_on_nonzero_exit() {
        // `false` always exits 1 on Unix.
        let result = run_command("false", &[]);
        assert!(matches!(
            result,
            PluginOutcome::Failed {
                exit_code: Some(1),
                ..
            }
        ));
    }

    #[test]
    fn outcome_is_success_predicate() {
        assert!(PluginOutcome::Success.is_success());
        assert!(!PluginOutcome::CliNotFound.is_success());
        assert!(
            !PluginOutcome::Failed {
                exit_code: Some(1),
                stderr: String::new(),
            }
            .is_success()
        );
    }

    #[test]
    fn outcome_is_cli_not_found_predicate() {
        assert!(PluginOutcome::CliNotFound.is_cli_not_found());
        assert!(!PluginOutcome::Success.is_cli_not_found());
    }

    #[test]
    fn install_copilot_returns_cli_not_found_when_binary_missing() {
        // Using a descriptor that references a nonexistent binary would
        // require overriding the program name. Instead, we test via
        // run_command directly (same pattern as the Claude tests above).
        let result = run_command(
            "nonexistent-copilot-xyz-42",
            &["plugin", "marketplace", "add", "test-source"],
        );
        assert_eq!(result, PluginOutcome::CliNotFound);
    }

    #[test]
    fn uninstall_copilot_returns_cli_not_found_when_binary_missing() {
        let result = run_command(
            "nonexistent-copilot-xyz-42",
            &["plugin", "uninstall", "test@source"],
        );
        assert_eq!(result, PluginOutcome::CliNotFound);
    }

    // -----------------------------------------------------------------------
    // PluginCheckResult tests (check_output_for_* helpers)
    // -----------------------------------------------------------------------

    #[test]
    fn check_output_for_exact_line_finds_matching_extension() {
        let output = CommandOutput::Success {
            stdout: "ms-python.python\nanthropic.superpowers\n".to_string(),
        };
        assert_eq!(
            check_output_for_exact_line(output, "anthropic.superpowers"),
            PluginCheckResult::Installed
        );
    }

    #[test]
    fn check_output_for_exact_line_case_insensitive() {
        let output = CommandOutput::Success {
            stdout: "Anthropic.SuperPowers\n".to_string(),
        };
        assert_eq!(
            check_output_for_exact_line(output, "anthropic.superpowers"),
            PluginCheckResult::Installed
        );
    }

    #[test]
    fn check_output_for_exact_line_returns_not_installed_when_absent() {
        let output = CommandOutput::Success {
            stdout: "ms-python.python\n".to_string(),
        };
        assert_eq!(
            check_output_for_exact_line(output, "anthropic.superpowers"),
            PluginCheckResult::NotInstalled
        );
    }

    #[test]
    fn check_output_for_exact_line_cli_not_found() {
        assert_eq!(
            check_output_for_exact_line(CommandOutput::CliNotFound, "anything"),
            PluginCheckResult::CliNotFound
        );
    }

    #[test]
    fn check_output_for_substring_finds_plugin_name() {
        let output = CommandOutput::Success {
            stdout: "superpowers@anthropics/claude-plugins\nother-plugin\n".to_string(),
        };
        assert_eq!(
            check_output_for_substring(output, "superpowers"),
            PluginCheckResult::Installed
        );
    }

    #[test]
    fn check_output_for_substring_returns_not_installed_when_absent() {
        let output = CommandOutput::Success {
            stdout: "other-plugin\n".to_string(),
        };
        assert_eq!(
            check_output_for_substring(output, "superpowers"),
            PluginCheckResult::NotInstalled
        );
    }

    #[test]
    fn check_output_for_substring_cli_not_found() {
        assert_eq!(
            check_output_for_substring(CommandOutput::CliNotFound, "anything"),
            PluginCheckResult::CliNotFound
        );
    }

    #[test]
    fn check_output_query_failed_maps_correctly() {
        let output = CommandOutput::Failed {
            exit_code: Some(1),
            stderr: "some error".to_string(),
        };
        assert!(matches!(
            check_output_for_substring(output, "anything"),
            PluginCheckResult::QueryFailed {
                exit_code: Some(1),
                ..
            }
        ));
    }

    #[test]
    fn check_vscode_extension_installed_returns_cli_not_found_when_no_binary() {
        // `nonexistent-code-xyz-42` is never on PATH.
        let result =
            check_output_for_exact_line(CommandOutput::CliNotFound, "anthropic.superpowers");
        assert_eq!(result, PluginCheckResult::CliNotFound);
    }

    #[test]
    fn plugin_check_result_predicates() {
        assert!(PluginCheckResult::Installed.is_installed());
        assert!(!PluginCheckResult::NotInstalled.is_installed());

        assert!(PluginCheckResult::NotInstalled.is_not_installed());
        assert!(!PluginCheckResult::Installed.is_not_installed());

        assert!(PluginCheckResult::CliNotFound.is_cli_not_found());
        assert!(!PluginCheckResult::Installed.is_cli_not_found());
    }
}
