//! Client-native MCP server configuration (ADR-0010).
//!
//! CLI-first: shells out to each client's MCP verb (`claude mcp add`),
//! falling back to writing the client config file when the CLI is absent.
//! Reuses the command helpers in [`crate::plugin`]. Never expands `${VAR}`
//! values — they pass through verbatim, so upskill holds no secret.

use crate::model::bundle::{McpLocal, McpRemote};
use crate::plugin::{PluginCheckResult, PluginOutcome, PluginScope};

/// A client whose MCP configuration upskill can write. Distinct from
/// [`crate::generate::Client`]: VS Code shares `.github/**` rules/skills/agents
/// output with Copilot and forks **only** on MCP, so it is an MCP target but
/// not a generation `Client` (ADR-0010, issue #237).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum McpTarget {
    Claude,
    Copilot,
    VsCode,
    OpenCode,
}

impl McpTarget {
    /// Every MCP target, in declaration order. Single source of truth for the
    /// install fan-out and doctor reconciliation loops.
    pub const ALL: [McpTarget; 4] = [Self::Claude, Self::Copilot, Self::VsCode, Self::OpenCode];

    /// Stable identifier used in the lockfile `client` field, CLI output, and
    /// [`McpResult`](crate::pipeline::report::McpResult)/[`LockedMcp`](crate::lockfile::LockedMcp).
    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Copilot => "copilot",
            Self::VsCode => "vscode",
            Self::OpenCode => "opencode",
        }
    }
}

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

// ---------------------------------------------------------------------------
// VS Code — `code --add-mcp '<json>'`
// ---------------------------------------------------------------------------

/// Build the single JSON argument `code --add-mcp` expects from a local
/// (stdio) descriptor. VS Code keys the server by an inline `name` field and
/// types stdio transports as `"stdio"`.
pub fn vscode_add_local_json(name: &str, local: &McpLocal) -> String {
    let mut server = serde_json::Map::new();
    server.insert("name".into(), serde_json::json!(name));
    server.insert("type".into(), serde_json::json!("stdio"));
    server.insert("command".into(), serde_json::json!(local.command));
    if !local.args.is_empty() {
        server.insert("args".into(), serde_json::json!(local.args));
    }
    if !local.env.is_empty() {
        server.insert("env".into(), serde_json::json!(local.env));
    }
    serde_json::Value::Object(server).to_string()
}

/// Build the single JSON argument `code --add-mcp` expects from a remote
/// descriptor. VS Code types remote transports by their `http`/`sse` value.
pub fn vscode_add_remote_json(name: &str, remote: &McpRemote) -> String {
    let mut server = serde_json::Map::new();
    server.insert("name".into(), serde_json::json!(name));
    server.insert("type".into(), serde_json::json!(remote.transport_type));
    server.insert("url".into(), serde_json::json!(remote.url));
    if !remote.headers.is_empty() {
        server.insert("headers".into(), serde_json::json!(remote.headers));
    }
    serde_json::Value::Object(server).to_string()
}

/// Install a local (stdio) MCP server into VS Code via `code --add-mcp`.
pub fn install_vscode_local(name: &str, local: &McpLocal) -> PluginOutcome {
    crate::plugin::run_command("code", &["--add-mcp", &vscode_add_local_json(name, local)])
}

/// Install a remote MCP server into VS Code via `code --add-mcp`.
pub fn install_vscode_remote(name: &str, remote: &McpRemote) -> PluginOutcome {
    crate::plugin::run_command(
        "code",
        &["--add-mcp", &vscode_add_remote_json(name, remote)],
    )
}

// ---------------------------------------------------------------------------
// GitHub Copilot CLI — `copilot mcp add` / `copilot mcp remove`
// ---------------------------------------------------------------------------

/// Build the argument vector for `copilot mcp add` from a local (stdio)
/// descriptor. Mirrors the `claude mcp add` shape; Copilot has no `--scope`.
pub fn copilot_add_local_args(name: &str, local: &McpLocal) -> Vec<String> {
    let mut args = vec!["mcp".to_string(), "add".to_string(), name.to_string()];
    for (k, v) in &local.env {
        args.push("-e".to_string());
        args.push(format!("{k}={v}"));
    }
    args.push("--".to_string());
    args.push(local.command.clone());
    args.extend(local.args.iter().cloned());
    args
}

/// Build the argument vector for `copilot mcp add` from a remote descriptor.
pub fn copilot_add_remote_args(name: &str, remote: &McpRemote) -> Vec<String> {
    let mut args = vec![
        "mcp".to_string(),
        "add".to_string(),
        "--transport".to_string(),
        remote.transport_type.clone(),
        name.to_string(),
        remote.url.clone(),
    ];
    for (header, value) in &remote.headers {
        args.push("-H".to_string());
        args.push(format!("{header}: {value}"));
    }
    args
}

/// Install a local (stdio) MCP server into Copilot CLI via `copilot mcp add`.
pub fn install_copilot_local(name: &str, local: &McpLocal) -> PluginOutcome {
    let args = copilot_add_local_args(name, local);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    crate::plugin::run_command("copilot", &arg_refs)
}

/// Install a remote MCP server into Copilot CLI via `copilot mcp add`.
pub fn install_copilot_remote(name: &str, remote: &McpRemote) -> PluginOutcome {
    let args = copilot_add_remote_args(name, remote);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    crate::plugin::run_command("copilot", &arg_refs)
}

/// Remove an MCP server from Copilot CLI via `copilot mcp remove`.
pub fn uninstall_copilot(name: &str) -> PluginOutcome {
    crate::plugin::run_command("copilot", &["mcp", "remove", name])
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

    #[test]
    fn mcp_target_names_are_stable() {
        assert_eq!(McpTarget::Claude.name(), "claude");
        assert_eq!(McpTarget::Copilot.name(), "copilot");
        assert_eq!(McpTarget::VsCode.name(), "vscode");
        assert_eq!(McpTarget::OpenCode.name(), "opencode");
        assert_eq!(McpTarget::ALL.len(), 4);
    }

    #[test]
    fn vscode_local_json_uses_stdio_type_and_inline_name() {
        let mut env = BTreeMap::new();
        env.insert("DRAWIO_TOKEN".to_string(), "${DRAWIO_TOKEN}".to_string());
        let local = McpLocal {
            command: "npx".into(),
            args: vec!["-y".into(), "drawio-mcp-server".into()],
            env,
        };
        let json: serde_json::Value =
            serde_json::from_str(&vscode_add_local_json("drawio", &local)).unwrap();
        assert_eq!(json["name"], "drawio");
        assert_eq!(json["type"], "stdio");
        assert_eq!(json["command"], "npx");
        assert_eq!(json["args"], serde_json::json!(["-y", "drawio-mcp-server"]));
        // Secret passes through verbatim — never expanded.
        assert_eq!(json["env"]["DRAWIO_TOKEN"], "${DRAWIO_TOKEN}");
    }

    #[test]
    fn vscode_remote_json_carries_transport_type_and_url() {
        let mut headers = BTreeMap::new();
        headers.insert("Authorization".to_string(), "Bearer ${TOK}".to_string());
        let remote = McpRemote {
            transport_type: "sse".into(),
            url: "https://mcp.draw.io/mcp".into(),
            headers,
        };
        let json: serde_json::Value =
            serde_json::from_str(&vscode_add_remote_json("drawio", &remote)).unwrap();
        assert_eq!(json["name"], "drawio");
        assert_eq!(json["type"], "sse");
        assert_eq!(json["url"], "https://mcp.draw.io/mcp");
        assert_eq!(json["headers"]["Authorization"], "Bearer ${TOK}");
    }

    #[test]
    fn copilot_local_args_have_no_scope_flag() {
        let mut env = BTreeMap::new();
        env.insert("DRAWIO_TOKEN".to_string(), "${DRAWIO_TOKEN}".to_string());
        let local = McpLocal {
            command: "npx".into(),
            args: vec!["-y".into(), "drawio-mcp-server".into()],
            env,
        };
        let args = copilot_add_local_args("drawio", &local);
        assert_eq!(
            args,
            vec![
                "mcp",
                "add",
                "drawio",
                "-e",
                "DRAWIO_TOKEN=${DRAWIO_TOKEN}",
                "--",
                "npx",
                "-y",
                "drawio-mcp-server",
            ]
        );
        assert!(!args.iter().any(|a| a == "--scope"));
    }

    #[test]
    fn copilot_remote_args_include_transport_and_url() {
        let remote = McpRemote {
            transport_type: "http".into(),
            url: "https://mcp.draw.io/mcp".into(),
            headers: BTreeMap::new(),
        };
        let args = copilot_add_remote_args("drawio", &remote);
        assert_eq!(
            args,
            vec![
                "mcp",
                "add",
                "--transport",
                "http",
                "drawio",
                "https://mcp.draw.io/mcp",
            ]
        );
    }
}
