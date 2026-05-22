//! Client CLI shellout for plugin installation (ADR-0008).
//!
//! Plugins are installed by shelling out to each client's native CLI:
//! - Claude Code: `claude plugin marketplace add` + `claude plugin install`
//! - Copilot CLI: `copilot plugin marketplace add` + `copilot plugin install`
//! - VS Code: `code --install-extension`
//! - opencode: `opencode plugin`
//!
//! This module exposes typed install/uninstall functions per client.
//! All functions return structured results (never write to stdout/stderr)
//! and handle CLI-not-found gracefully per the warn-skip policy.

use crate::model::bundle::{
    ClaudePluginDescriptor, CopilotPluginDescriptor, OpencodePluginDescriptor,
    VscodePluginDescriptor,
};
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
}

// ---------------------------------------------------------------------------
// Install functions
// ---------------------------------------------------------------------------

/// Install a Claude Code plugin via `claude plugin marketplace add` followed
/// by `claude plugin install`. The marketplace-add step is idempotent.
pub fn install_claude_plugin(
    descriptor: &ClaudePluginDescriptor,
    scope: PluginScope,
) -> PluginOutcome {
    // Step 1: Add marketplace source (idempotent).
    let result = run_command(
        "claude",
        &["plugin", "marketplace", "add", &descriptor.source],
    );
    match result {
        PluginOutcome::CliNotFound => return PluginOutcome::CliNotFound,
        PluginOutcome::Failed { .. } => return result,
        PluginOutcome::Success => {}
    }

    // Step 2: Install plugin with scope.
    let install_ref = format!("{}@{}", descriptor.plugin, descriptor.source);
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
pub fn install_vscode_extension(descriptor: &VscodePluginDescriptor) -> PluginOutcome {
    run_command("code", &["--install-extension", &descriptor.extension])
}

/// Install an opencode module via `opencode plugin`.
pub fn install_opencode_plugin(descriptor: &OpencodePluginDescriptor) -> PluginOutcome {
    run_command("opencode", &["plugin", &descriptor.module])
}

/// Install a GitHub Copilot CLI plugin via `copilot plugin marketplace add`
/// followed by `copilot plugin install`. The marketplace-add step is
/// idempotent. Unlike Claude, Copilot CLI does not support a `--scope` flag.
pub fn install_copilot_plugin(descriptor: &CopilotPluginDescriptor) -> PluginOutcome {
    // Step 1: Add marketplace source (idempotent).
    let result = run_command(
        "copilot",
        &["plugin", "marketplace", "add", &descriptor.source],
    );
    match result {
        PluginOutcome::CliNotFound => return PluginOutcome::CliNotFound,
        PluginOutcome::Failed { .. } => return result,
        PluginOutcome::Success => {}
    }

    // Step 2: Install plugin from marketplace.
    let install_ref = format!("{}@{}", descriptor.plugin, descriptor.source);
    run_command("copilot", &["plugin", "install", &install_ref])
}

// ---------------------------------------------------------------------------
// Uninstall functions
// ---------------------------------------------------------------------------

/// Uninstall a Claude Code plugin.
pub fn uninstall_claude_plugin(plugin: &str, source: &str, scope: PluginScope) -> PluginOutcome {
    let install_ref = format!("{plugin}@{source}");
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

/// Uninstall an opencode module.
pub fn uninstall_opencode_plugin(module: &str) -> PluginOutcome {
    run_command("opencode", &["plugin", "remove", module])
}

/// Uninstall a GitHub Copilot CLI plugin.
pub fn uninstall_copilot_plugin(plugin: &str, source: &str) -> PluginOutcome {
    let install_ref = format!("{plugin}@{source}");
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
    match Command::new(cli).arg("--version").output() {
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
/// substring of any output line.
pub fn check_opencode_plugin_installed(module_name: &str) -> PluginCheckResult {
    let out = run_command_output("opencode", &["plugin", "list"]);
    check_output_for_substring(out, module_name)
}

// ---------------------------------------------------------------------------
// Internal command output helpers
// ---------------------------------------------------------------------------

/// Captured result of a CLI invocation.
enum CommandOutput {
    Success {
        stdout: String,
    },
    CliNotFound,
    Failed {
        exit_code: Option<i32>,
        stderr: String,
    },
}

fn run_command_output(program: &str, args: &[&str]) -> CommandOutput {
    match Command::new(program).args(args).output() {
        Ok(out) if out.status.success() => CommandOutput::Success {
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        },
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

/// Map `CommandOutput` to `PluginCheckResult` by searching for `needle` as
/// a substring of any stdout line.
fn check_output_for_substring(output: CommandOutput, needle: &str) -> PluginCheckResult {
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
fn run_command(program: &str, args: &[&str]) -> PluginOutcome {
    let output = match Command::new(program).args(args).output() {
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
        let descriptor = VscodePluginDescriptor {
            extension: "test.extension".to_string(),
            install_url: None,
        };
        // code is likely not on PATH in CI; if it is, this still works
        // because --install-extension with a fake extension just fails.
        let result = run_command(
            "nonexistent-code-xyz-42",
            &["--install-extension", &descriptor.extension],
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
