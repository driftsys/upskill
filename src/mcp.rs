//! Client-native MCP server configuration (ADR-0010).
//!
//! CLI-first: shells out to each client's MCP verb (`claude mcp add`),
//! falling back to writing the client config file when the CLI is absent.
//! Reuses the command helpers in [`crate::plugin`]. Never expands `${VAR}`
//! values — they pass through verbatim, so upskill holds no secret.

use crate::model::bundle::{McpLocal, McpRemote};
use crate::plugin::{PluginCheckResult, PluginOutcome, PluginScope};

/// Build the argument vector for `claude mcp add` from a local (stdio)
/// descriptor. Returned as owned strings so the caller can borrow them.
pub fn claude_add_local_args(name: &str, local: &McpLocal, scope: PluginScope) -> Vec<String> {
    let mut args = vec![
        "mcp".to_string(),
        "add".to_string(),
        name.to_string(),
        "--scope".to_string(),
        scope.as_claude_flag().to_string(),
    ];
    for (k, v) in &local.env {
        args.push("-e".to_string());
        args.push(format!("{k}={v}"));
    }
    args.push("--".to_string());
    args.push(local.command.clone());
    args.extend(local.args.iter().cloned());
    args
}

/// Build the argument vector for `claude mcp add` from a remote descriptor.
pub fn claude_add_remote_args(name: &str, remote: &McpRemote, scope: PluginScope) -> Vec<String> {
    let mut args = vec![
        "mcp".to_string(),
        "add".to_string(),
        "--transport".to_string(),
        remote.transport_type.clone(),
        name.to_string(),
        remote.url.clone(),
        "--scope".to_string(),
        scope.as_claude_flag().to_string(),
    ];
    for (header, value) in &remote.headers {
        args.push("-H".to_string());
        args.push(format!("{header}: {value}"));
    }
    args
}

/// Install a local (stdio) MCP server into Claude Code via `claude mcp add`.
pub fn install_claude_local(name: &str, local: &McpLocal, scope: PluginScope) -> PluginOutcome {
    let args = claude_add_local_args(name, local, scope);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    crate::plugin::run_command("claude", &arg_refs)
}

/// Install a remote MCP server into Claude Code via `claude mcp add`.
pub fn install_claude_remote(name: &str, remote: &McpRemote, scope: PluginScope) -> PluginOutcome {
    let args = claude_add_remote_args(name, remote, scope);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    crate::plugin::run_command("claude", &arg_refs)
}

/// Remove an MCP server from Claude Code via `claude mcp remove`.
pub fn uninstall_claude(name: &str, scope: PluginScope) -> PluginOutcome {
    crate::plugin::run_command(
        "claude",
        &["mcp", "remove", name, "--scope", scope.as_claude_flag()],
    )
}

/// Check whether an MCP server is registered in Claude Code via
/// `claude mcp list` (substring match on the server name).
pub fn check_claude_installed(name: &str) -> PluginCheckResult {
    let out = crate::plugin::run_command_output("claude", &["mcp", "list"]);
    crate::plugin::check_output_for_substring(out, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn claude_local_args_include_scope_env_and_command() {
        let mut env = BTreeMap::new();
        env.insert("DRAWIO_TOKEN".to_string(), "${DRAWIO_TOKEN}".to_string());
        let local = McpLocal {
            command: "npx".into(),
            args: vec!["-y".into(), "drawio-mcp-server".into()],
            env,
        };
        let args = claude_add_local_args("drawio", &local, PluginScope::Project);
        assert_eq!(
            args,
            vec![
                "mcp",
                "add",
                "drawio",
                "--scope",
                "project",
                "-e",
                "DRAWIO_TOKEN=${DRAWIO_TOKEN}",
                "--",
                "npx",
                "-y",
                "drawio-mcp-server",
            ]
        );
    }

    #[test]
    fn claude_remote_args_include_transport_and_url() {
        let remote = McpRemote {
            transport_type: "http".into(),
            url: "https://mcp.draw.io/mcp".into(),
            headers: BTreeMap::new(),
        };
        let args = claude_add_remote_args("drawio", &remote, PluginScope::User);
        assert_eq!(
            args,
            vec![
                "mcp",
                "add",
                "--transport",
                "http",
                "drawio",
                "https://mcp.draw.io/mcp",
                "--scope",
                "user",
            ]
        );
    }
}
