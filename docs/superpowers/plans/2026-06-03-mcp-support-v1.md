# MCP Server Support (v1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let an upskill bundle declare MCP servers (`mcps:`) that `upskill add` configures into each targeted client — CLI-first, config-write fallback — with a lockfile + `doctor` lifecycle and `${VAR}` secret indirection.

**Architecture:** A near-exact parallel of the existing plugin mechanism (ADR-0008). A new `mcps:` map on `Bundle` carries per-server descriptors (remote `http`/`sse` URL, or local stdio `command`). A new `src/mcp.rs` module shells out to each client's native MCP verb (`claude mcp add`), falling back to writing the client config file when the CLI is absent. Outcomes are recorded as `LockedMcp` entries; `remove`/`doctor` reconcile via the inverse verb. upskill never expands `${VAR}` — values pass through verbatim, so it holds no secret.

**Tech Stack:** Rust 2024 (MSRV 1.85), `serde`/`serde_yaml_ng`/`serde_json`, `std::process::Command` (no new deps), `assert_cmd` + `tempfile` for integration tests.

**Spec:** [docs/superpowers/specs/2026-06-03-mcp-support-design.md](../specs/2026-06-03-mcp-support-design.md)

**Out of scope (v2):** `requires.mcps` skill dependency edge, generated skill-body preflight, the install agent for bare binaries. This plan is v1 (bundle-level, eager config-write) only.

---

## File Structure

| File                                | Responsibility                                                                                                                | New/Modify |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ---------- |
| `src/model/bundle.rs`               | `mcps: BTreeMap<String, McpEntry>` on `Bundle`; `McpEntry`, `McpTransport`, `McpRemote`, `McpLocal` types + `validate_mcps()` | Modify     |
| `src/parse/bundle.rs`               | Call `bundle.validate_mcps()` from `load`/`load_if_bundle`; parse tests                                                       | Modify     |
| `src/mcp.rs`                        | Client shellout (`claude mcp add`/`remove`/`list`) + outcome types; reuses `plugin.rs` command helpers                        | New        |
| `src/plugin.rs`                     | Make `run_command`, `run_command_output`, `CommandOutput`, `check_output_for_substring` `pub(crate)` for reuse                | Modify     |
| `src/ancillary.rs`                  | `write_claude_mcp_json`, `write_opencode_mcp`, `write_vscode_mcp_json` config-write fallbacks                                 | Modify     |
| `src/pipeline/report.rs`            | `McpResult` struct; `mcp_results` field on `InstallReport`                                                                    | Modify     |
| `src/pipeline/install.rs`           | `install_mcps_from_bundles()`                                                                                                 | Modify     |
| `src/pipeline/mod.rs`               | Call it; record `LockedMcp` entries                                                                                           | Modify     |
| `src/lockfile.rs`                   | `LockedMcp`, `McpInstallStatus`, `upsert_mcp`, `remove_mcps_by_name`; `mcps` field                                            | Modify     |
| `src/main.rs`                       | `print_mcp_results`; `remove mcp <name>`; doctor reconciliation + `requires-env` warn                                         | Modify     |
| `src/lint.rs`                       | Lint `mcps:` shape (delegates to `validate_mcps`)                                                                             | Modify     |
| `tests/cli_mcp.rs`                  | CLI integration: config-write + warn-skip + remove                                                                            | New        |
| `tests/pipeline_mcp.rs`             | Pipeline + lockfile recording                                                                                                 | New        |
| `docs/adr/0010-mcp-config-write.md` | ADR for this decision                                                                                                         | New        |
| `docs/format-spec.md`               | `mcps:` sub-shape + lockfile schema note                                                                                      | Modify     |

> **Worktree note:** all paths are relative to the worktree root
> `.claude/worktrees/feat-mcp-support/`. Use the full worktree-prefixed
> absolute path for every Write/Edit.

---

## Task 1: Model — `mcps:` descriptor types

**Files:**

- Modify: `src/model/bundle.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `src/parse/bundle.rs` (it already exercises `Bundle` parsing — the model has no own test module):

```rust
#[test]
fn load_parses_mcp_remote_and_local() {
    use crate::model::bundle::McpTransport;

    let content = "schema: 1
name: with-mcp
description: Bundle with MCP servers
items:
  skills: []
mcps:
  drawio:
    remote:
      type: http
      url: https://mcp.draw.io/mcp
  local-server:
    local:
      command: npx
      args: [\"-y\", \"drawio-mcp-server\"]
      env:
        DRAWIO_TOKEN: \"${DRAWIO_TOKEN}\"
    requires-env: [DRAWIO_TOKEN]
";
    let tmp = tempfile::tempdir().unwrap();
    let path = write_file(tmp.path(), "with-mcp.bundle.yaml", content);

    let bundle = load(&path).expect("load");
    assert_eq!(bundle.mcps.len(), 2);

    match &bundle.mcps["drawio"].transport {
        McpTransport::Remote(r) => {
            assert_eq!(r.transport_type, "http");
            assert_eq!(r.url, "https://mcp.draw.io/mcp");
        }
        McpTransport::Local(_) => panic!("expected remote"),
    }

    let local = &bundle.mcps["local-server"];
    match &local.transport {
        McpTransport::Local(l) => {
            assert_eq!(l.command, "npx");
            assert_eq!(l.args, vec!["-y", "drawio-mcp-server"]);
            assert_eq!(l.env["DRAWIO_TOKEN"], "${DRAWIO_TOKEN}");
        }
        McpTransport::Remote(_) => panic!("expected local"),
    }
    assert_eq!(local.requires_env, vec!["DRAWIO_TOKEN"]);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib parse::bundle::tests::load_parses_mcp_remote_and_local`
Expected: FAIL — `no field mcps on type Bundle` / `unresolved import McpTransport`.

- [ ] **Step 3: Add the model types**

In `src/model/bundle.rs`, add the field to `Bundle` (next to `plugins`):

```rust
/// MCP servers configured into each targeted client (ADR-0010, §3.8).
/// Map key is the upskill-level MCP name; value carries the transport
/// descriptor and declared required env vars.
#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
pub mcps: BTreeMap<String, McpEntry>,
```

Append these types to the end of the file (before `#[cfg(test)]` if any, else end):

```rust
/// One entry in the bundle `mcps:` map. Exactly one transport (`remote`
/// or `local`) is present; `validate_mcps` enforces this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpEntry {
    /// Transport descriptor — flattened so YAML carries either a
    /// `remote:` or a `local:` key directly under the server name.
    #[serde(flatten)]
    pub transport: McpTransport,

    /// Environment variables the server requires. Declared (not valued) so
    /// `upskill doctor` can warn when one is unset. upskill never reads the
    /// values — secret custody stays with the user's environment.
    #[serde(default, rename = "requires-env", skip_serializing_if = "Vec::is_empty")]
    pub requires_env: Vec<String>,
}

/// MCP transport: a hosted server reached by URL, or a local process the
/// client spawns and speaks to over stdio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum McpTransport {
    #[serde(rename = "remote")]
    Remote(McpRemote),
    #[serde(rename = "local")]
    Local(McpLocal),
}

/// Remote (hosted) MCP server descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpRemote {
    /// `http` or `sse`.
    #[serde(rename = "type")]
    pub transport_type: String,
    /// Endpoint URL the client connects to.
    pub url: String,
    /// Optional headers. Values pass through verbatim (use `${VAR}`
    /// references for secrets — upskill never expands them).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
}

/// Local (stdio) MCP server descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpLocal {
    /// Launcher command (e.g. `npx`, `uvx`, `docker`, or a bare binary).
    pub command: String,
    /// Arguments passed to the command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Environment variables. Values pass through verbatim (use `${VAR}`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib parse::bundle::tests::load_parses_mcp_remote_and_local`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/model/bundle.rs src/parse/bundle.rs
git commit -m "feat(model): add mcps: descriptor types to Bundle"
```

---

## Task 2: Model — validation (`exactly-one-of`, remote/local required fields)

`#[serde(flatten)]` on an untagged-style enum already rejects "neither"
(deserialization fails when neither `remote` nor `local` is present) and
"both" is rejected because `McpTransport` deserializes the first matching
variant — so we add an explicit `validate_mcps` for the cases serde does not
catch: an empty `type`/`url`/`command`, and an unknown remote `type`.

**Files:**

- Modify: `src/model/bundle.rs`
- Modify: `src/parse/bundle.rs`

- [ ] **Step 1: Write the failing test**

Add to `src/parse/bundle.rs` tests:

```rust
#[test]
fn load_rejects_mcp_remote_with_bad_type() {
    let content = "schema: 1
name: bad-type
description: bad transport type
items:
  skills: []
mcps:
  x:
    remote:
      type: websocket
      url: https://example.com
";
    let tmp = tempfile::tempdir().unwrap();
    let path = write_file(tmp.path(), "bad-type.bundle.yaml", content);
    let err = load(&path).expect_err("must reject unknown transport type");
    let msg = format!("{:#}", err);
    assert!(msg.contains("transport type") && msg.contains("websocket"), "got: {msg}");
}

#[test]
fn load_rejects_mcp_local_empty_command() {
    let content = "schema: 1
name: empty-cmd
description: empty command
items:
  skills: []
mcps:
  x:
    local:
      command: \"\"
";
    let tmp = tempfile::tempdir().unwrap();
    let path = write_file(tmp.path(), "empty-cmd.bundle.yaml", content);
    let err = load(&path).expect_err("must reject empty command");
    let msg = format!("{:#}", err);
    assert!(msg.contains("command") && msg.contains("empty"), "got: {msg}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib parse::bundle::tests::load_rejects_mcp`
Expected: FAIL — `load` currently accepts these (no validation).

- [ ] **Step 3: Add `validate_mcps` and call it**

In `src/model/bundle.rs`, add an `impl Bundle` block (or extend an existing one):

```rust
impl Bundle {
    /// Validate the `mcps:` map beyond what serde enforces. Returns the
    /// first offending `(server-name, reason)` as an error.
    pub fn validate_mcps(&self) -> Result<(), String> {
        for (name, entry) in &self.mcps {
            match &entry.transport {
                McpTransport::Remote(r) => {
                    if r.transport_type != "http" && r.transport_type != "sse" {
                        return Err(format!(
                            "mcp `{name}`: transport type `{}` is not one of http, sse",
                            r.transport_type
                        ));
                    }
                    if r.url.trim().is_empty() {
                        return Err(format!("mcp `{name}`: remote url must not be empty"));
                    }
                }
                McpTransport::Local(l) => {
                    if l.command.trim().is_empty() {
                        return Err(format!("mcp `{name}`: local command must not be empty"));
                    }
                }
            }
        }
        Ok(())
    }
}
```

In `src/parse/bundle.rs`, in **both** `load` and `load_if_bundle`, after the
filename-stem check and before `Ok(bundle)` / `Ok(Some(bundle))`, add:

```rust
bundle
    .validate_mcps()
    .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib parse::bundle::tests::load_rejects_mcp`
Expected: PASS (both).

- [ ] **Step 5: Commit**

```bash
git add src/model/bundle.rs src/parse/bundle.rs
git commit -m "feat(model): validate mcps transport fields"
```

---

## Task 3: Expose `plugin.rs` command helpers for reuse

`src/mcp.rs` needs the same spawn/CLI-not-found/run helpers `plugin.rs`
already has. Promote them to `pub(crate)` rather than duplicating.

**Files:**

- Modify: `src/plugin.rs`

- [ ] **Step 1: Promote visibility**

In `src/plugin.rs`, change these signatures (bodies unchanged):

```rust
pub(crate) enum CommandOutput {
    Success { stdout: String },
    CliNotFound,
    Failed { exit_code: Option<i32>, stderr: String },
}

pub(crate) fn run_command_output(program: &str, args: &[&str]) -> CommandOutput { /* unchanged */ }

pub(crate) fn run_command(program: &str, args: &[&str]) -> PluginOutcome { /* unchanged */ }

pub(crate) fn check_output_for_substring(output: CommandOutput, needle: &str) -> PluginCheckResult { /* unchanged */ }
```

(`is_cli_available` and `PluginOutcome`/`PluginCheckResult` are already `pub`.)

- [ ] **Step 2: Verify the crate still builds**

Run: `cargo build`
Expected: builds with zero warnings (the items are now used cross-module in later tasks; until then `pub(crate)` on an in-crate-unused item does not warn).

- [ ] **Step 3: Commit**

```bash
git add src/plugin.rs
git commit -m "refactor(plugin): expose command helpers as pub(crate)"
```

---

## Task 4: `src/mcp.rs` — Claude CLI shellout (add)

**Files:**

- Create: `src/mcp.rs`
- Modify: `src/lib.rs` (add `pub mod mcp;`)

- [ ] **Step 1: Write the failing test**

Create `src/mcp.rs` with only this test module to start:

```rust
//! Client-native MCP server configuration (ADR-0010).
//!
//! CLI-first: shells out to each client's MCP verb (`claude mcp add`),
//! falling back to writing the client config file when the CLI is absent.
//! Reuses the command helpers in [`crate::plugin`]. Never expands `${VAR}`
//! values — they pass through verbatim, so upskill holds no secret.

use crate::model::bundle::{McpLocal, McpRemote};
use crate::plugin::PluginScope;

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
                "mcp", "add", "drawio", "--scope", "project",
                "-e", "DRAWIO_TOKEN=${DRAWIO_TOKEN}",
                "--", "npx", "-y", "drawio-mcp-server",
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
                "mcp", "add", "--transport", "http",
                "drawio", "https://mcp.draw.io/mcp",
                "--scope", "user",
            ]
        );
    }
}
```

Add to `src/lib.rs` alphabetically near `pub mod plugin;`:

```rust
pub mod mcp;
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test --lib mcp::tests`
Expected: PASS (these are pure arg-builders — they pass as soon as the code compiles; the value is locking the exact CLI contract).

- [ ] **Step 3: Add the install/remove/check shellout functions**

Append to `src/mcp.rs` (before the test module):

```rust
use crate::plugin::{PluginCheckResult, PluginOutcome};

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
```

- [ ] **Step 4: Run the full module + build**

Run: `cargo test --lib mcp:: && cargo build`
Expected: PASS, zero warnings.

- [ ] **Step 5: Commit**

```bash
git add src/mcp.rs src/lib.rs
git commit -m "feat(mcp): claude mcp add/remove/list shellout"
```

---

## Task 5: `src/ancillary.rs` — config-write fallbacks

Write the MCP entry into a client config file when its CLI is absent.
`.mcp.json` (Claude project-scope fallback) and `opencode.json` (`mcp` key)
are the v1 targets. Both merge without clobbering existing entries, mirroring
`write_opencode_plugin_uri`.

**Files:**

- Modify: `src/ancillary.rs`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` in `src/ancillary.rs`:

```rust
#[test]
fn write_claude_mcp_json_adds_local_server() {
    use crate::model::bundle::McpLocal;
    use std::collections::BTreeMap;

    let tmp = tempfile::tempdir().unwrap();
    let mut env = BTreeMap::new();
    env.insert("TOK".to_string(), "${TOK}".to_string());
    let local = McpLocal { command: "npx".into(), args: vec!["-y".into(), "srv".into()], env };

    let outcome = write_claude_mcp_local(tmp.path(), "drawio", &local);
    assert!(outcome.is_success());

    let raw = std::fs::read_to_string(tmp.path().join(".mcp.json")).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(doc["mcpServers"]["drawio"]["command"], "npx");
    assert_eq!(doc["mcpServers"]["drawio"]["args"][1], "srv");
    assert_eq!(doc["mcpServers"]["drawio"]["env"]["TOK"], "${TOK}");
}

#[test]
fn write_claude_mcp_json_preserves_existing_servers() {
    use crate::model::bundle::McpLocal;
    use std::collections::BTreeMap;

    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join(".mcp.json"),
        r#"{"mcpServers":{"existing":{"command":"foo"}}}"#,
    )
    .unwrap();

    let local = McpLocal { command: "npx".into(), args: vec![], env: BTreeMap::new() };
    write_claude_mcp_local(tmp.path(), "drawio", &local);

    let raw = std::fs::read_to_string(tmp.path().join(".mcp.json")).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(doc["mcpServers"]["existing"]["command"], "foo");
    assert_eq!(doc["mcpServers"]["drawio"]["command"], "npx");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib ancillary::tests::write_claude_mcp`
Expected: FAIL — `write_claude_mcp_local` not found.

- [ ] **Step 3: Implement the config-write helpers**

Add to `src/ancillary.rs` (near `write_opencode_plugin_uri`). The
`.mcp.json` shape Claude Code reads is `{ "mcpServers": { "<name>": {…} } }`:

```rust
use crate::model::bundle::{McpLocal, McpRemote};

/// Filename Claude Code reads for project-scoped MCP servers.
const CLAUDE_MCP_JSON: &str = ".mcp.json";

/// Write a local (stdio) MCP server into `<target>/.mcp.json` under
/// `mcpServers.<name>`. Merge-preserving; creates the file if absent.
/// Used as the fallback when the `claude` CLI is not on PATH.
pub fn write_claude_mcp_local(target: &Path, name: &str, local: &McpLocal) -> PluginOutcome {
    let mut server = serde_json::Map::new();
    server.insert("command".into(), json!(local.command));
    if !local.args.is_empty() {
        server.insert("args".into(), json!(local.args));
    }
    if !local.env.is_empty() {
        let env: serde_json::Map<String, Value> =
            local.env.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        server.insert("env".into(), Value::Object(env));
    }
    upsert_mcp_server(target, name, Value::Object(server))
}

/// Write a remote MCP server into `<target>/.mcp.json` under
/// `mcpServers.<name>`. Merge-preserving; creates the file if absent.
pub fn write_claude_mcp_remote(target: &Path, name: &str, remote: &McpRemote) -> PluginOutcome {
    let mut server = serde_json::Map::new();
    server.insert("type".into(), json!(remote.transport_type));
    server.insert("url".into(), json!(remote.url));
    if !remote.headers.is_empty() {
        let headers: serde_json::Map<String, Value> =
            remote.headers.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        server.insert("headers".into(), Value::Object(headers));
    }
    upsert_mcp_server(target, name, Value::Object(server))
}

/// Shared merge: insert `server` at `mcpServers.<name>` in `.mcp.json`,
/// preserving any other top-level keys and other servers.
fn upsert_mcp_server(target: &Path, name: &str, server: Value) -> PluginOutcome {
    let path = target.join(CLAUDE_MCP_JSON);
    let mut doc: Value = match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => {
            return PluginOutcome::Failed {
                exit_code: None,
                stderr: format!("read {}: {e}", path.display()),
            };
        }
        Ok(raw) => match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                return PluginOutcome::Failed {
                    exit_code: None,
                    stderr: format!("parse {}: {e}", path.display()),
                };
            }
        },
    };

    if !doc.is_object() {
        return PluginOutcome::Failed {
            exit_code: None,
            stderr: format!("{}: top-level value must be an object", path.display()),
        };
    }
    let obj = doc.as_object_mut().expect("checked is_object");
    let servers = obj
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let Some(servers) = servers.as_object_mut() else {
        return PluginOutcome::Failed {
            exit_code: None,
            stderr: format!("{}: `mcpServers` must be an object", path.display()),
        };
    };
    servers.insert(name.to_string(), server);

    if let Err(e) = write_pretty_json(&path, &doc) {
        return PluginOutcome::Failed { exit_code: None, stderr: e.to_string() };
    }
    PluginOutcome::Success
}

/// Remove an MCP server from `<target>/.mcp.json`. No-op if absent.
pub fn remove_claude_mcp(target: &Path, name: &str) -> Result<()> {
    let path = target.join(CLAUDE_MCP_JSON);
    let raw = match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
        Ok(r) => r,
    };
    let mut doc: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse {}", path.display()))?;
    if let Some(servers) = doc.get_mut("mcpServers").and_then(Value::as_object_mut) {
        servers.remove(name);
    }
    write_pretty_json(&path, &doc)
}
```

> If `json!`, `Value`, `write_pretty_json`, `PluginOutcome`, `Path`,
> `Context`/`Result` are not already imported at the top of `ancillary.rs`,
> add the missing `use` lines (the file already uses all of these for
> `write_opencode_plugin_uri`).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib ancillary::tests::write_claude_mcp`
Expected: PASS (both).

- [ ] **Step 5: Commit**

```bash
git add src/ancillary.rs
git commit -m "feat(ancillary): .mcp.json config-write fallback for MCP servers"
```

---

## Task 6: Lockfile — `LockedMcp` + lifecycle methods

**Files:**

- Modify: `src/lockfile.rs`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` in `src/lockfile.rs`:

```rust
#[test]
fn upsert_mcp_replaces_by_name_and_client() {
    let mut lock = Lockfile::new();
    lock.upsert_mcp(LockedMcp {
        name: "drawio".into(),
        client: "claude".into(),
        scope: Some("project".into()),
        bundle: "with-mcp".into(),
        status: McpInstallStatus::Installed,
    });
    lock.upsert_mcp(LockedMcp {
        name: "drawio".into(),
        client: "claude".into(),
        scope: Some("project".into()),
        bundle: "with-mcp".into(),
        status: McpInstallStatus::Skipped,
    });
    assert_eq!(lock.mcps.len(), 1);
    assert_eq!(lock.mcps[0].status, McpInstallStatus::Skipped);
}

#[test]
fn remove_mcps_by_name_drops_all_clients() {
    let mut lock = Lockfile::new();
    for client in ["claude", "opencode"] {
        lock.upsert_mcp(LockedMcp {
            name: "drawio".into(),
            client: client.into(),
            scope: None,
            bundle: "with-mcp".into(),
            status: McpInstallStatus::Installed,
        });
    }
    lock.remove_mcps_by_name("drawio");
    assert!(lock.mcps.is_empty());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib lockfile::tests::upsert_mcp lockfile::tests::remove_mcps_by_name`
Expected: FAIL — `mcps` field / `LockedMcp` / `upsert_mcp` not found.

- [ ] **Step 3: Add the field, types, and methods**

Add the field to `Lockfile` (after `plugins`):

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub mcps: Vec<LockedMcp>,
```

Initialize it in `Lockfile::new()`:

```rust
mcps: Vec::new(),
```

Add the types (near `LockedPlugin` / `PluginInstallStatus`):

```rust
/// MCP server entry recorded when a bundle's `mcps:` map is configured
/// into a client (ADR-0010). One entry per (mcp-name, client) pair so
/// `remove`/`doctor` can invoke the inverse CLI command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockedMcp {
    /// Upskill-level MCP name (key in the bundle's `mcps:` map).
    pub name: String,
    /// Client identifier: `"claude"`, `"opencode"`, etc.
    pub client: String,
    /// Install scope (only meaningful for claude: `"project"` or `"user"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Bundle that declared this MCP server.
    pub bundle: String,
    /// Configuration outcome recorded at `upskill add` time.
    #[serde(default)]
    pub status: McpInstallStatus,
}

/// Configuration status of an MCP entry in the lockfile (ADR-0010).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "lowercase")]
pub enum McpInstallStatus {
    /// Configured successfully (CLI shellout or config-write).
    #[default]
    Installed,
    /// Skipped — the client CLI was not on PATH and no config-write target
    /// applied (warn-skip outcome). `doctor` surfaces these.
    Skipped,
}
```

Add the methods to `impl Lockfile` (near `upsert_plugin`):

```rust
    /// Add or replace an MCP entry by `(name, client)`. Sorted for
    /// deterministic on-disk output.
    pub fn upsert_mcp(&mut self, mcp: LockedMcp) {
        self.mcps
            .retain(|existing| !(existing.name == mcp.name && existing.client == mcp.client));
        self.mcps.push(mcp);
        self.mcps.sort();
    }

    /// Remove all MCP entries matching `name` (across all clients).
    pub fn remove_mcps_by_name(&mut self, name: &str) {
        self.mcps.retain(|existing| existing.name != name);
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib lockfile::tests::upsert_mcp lockfile::tests::remove_mcps_by_name`
Expected: PASS (both).

- [ ] **Step 5: Commit**

```bash
git add src/lockfile.rs
git commit -m "feat(lockfile): record MCP servers (LockedMcp + lifecycle)"
```

---

## Task 7: Pipeline — `McpResult` + `install_mcps_from_bundles`

**Files:**

- Modify: `src/pipeline/report.rs`
- Modify: `src/pipeline/install.rs`
- Modify: `src/pipeline/mod.rs`

- [ ] **Step 1: Write the failing test**

Add to `tests/pipeline_mcp.rs` (new file — full file shown in Task 10; for
now add this unit-level assertion to `src/pipeline/install.rs` tests, creating
a `#[cfg(test)] mod tests` if none exists):

```rust
#[cfg(test)]
mod mcp_tests {
    use crate::model::bundle::{Bundle, McpEntry, McpLocal, McpRemote, McpTransport};
    use crate::model::common::SchemaVersion;
    use crate::plugin::PluginScope;
    use std::collections::BTreeMap;

    fn bundle_with_mcp(name: &str, entry: McpEntry) -> Bundle {
        let mut mcps = BTreeMap::new();
        mcps.insert(name.to_string(), entry);
        Bundle {
            schema: SchemaVersion::new(1),
            name: "with-mcp".into(),
            description: "test".into(),
            license: None,
            items: Default::default(),
            requires: vec![],
            plugins: BTreeMap::new(),
            mcps,
            metadata: None,
            extra: BTreeMap::new(),
        }
    }

    #[test]
    fn install_mcps_records_result_per_server() {
        let tmp = tempfile::tempdir().unwrap();
        let entry = McpEntry {
            transport: McpTransport::Remote(McpRemote {
                transport_type: "http".into(),
                url: "https://example.com/mcp".into(),
                headers: BTreeMap::new(),
            }),
            requires_env: vec![],
        };
        let bundles = vec![bundle_with_mcp("drawio", entry)];
        let results =
            super::install_mcps_from_bundles(&bundles, PluginScope::Project, tmp.path());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "drawio");
        assert_eq!(results[0].client, "claude");
        // No `claude` CLI on PATH in CI → CliNotFound, then config-write
        // fallback writes .mcp.json and the result is Success.
        assert!(results[0].outcome.is_success() || results[0].outcome.is_cli_not_found());
    }
}
```

> Adjust the `Bundle { … }` literal field list to match the current struct
> exactly (run `cargo build` once and fix any field mismatch — e.g. if
> `license`/`extra` differ). The `McpRemote`/`McpEntry`/`McpLocal` fields are
> fixed by Task 1.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib pipeline::install::mcp_tests`
Expected: FAIL — `install_mcps_from_bundles` not found; `mcp_results` not on report.

- [ ] **Step 3: Add `McpResult` to the report**

In `src/pipeline/report.rs`, add the struct (mirror `PluginResult`):

```rust
/// Result of a single MCP server configuration attempt, for reporting.
#[derive(Debug, Clone)]
pub struct McpResult {
    /// Upskill-level MCP name.
    pub name: String,
    /// Client identifier.
    pub client: String,
    /// Configuration outcome.
    pub outcome: crate::plugin::PluginOutcome,
    /// Bundle that declared this MCP server.
    pub bundle: String,
    /// Env vars declared as required (for the `doctor`/warn surface).
    pub requires_env: Vec<String>,
}
```

Add the field to `InstallReport` (next to `plugin_results`):

```rust
/// Results of MCP server configuration (ADR-0010). Empty when no
/// bundles with `mcps:` were resolved.
pub mcp_results: Vec<McpResult>,
```

Initialize `mcp_results: Vec::new()` everywhere `InstallReport` is constructed
(search: `grep -rn "plugin_results: Vec::new()" src/`) and add the sibling
line. Also update the `lockfile.rs:391` test constructor similarly.

- [ ] **Step 4: Add `install_mcps_from_bundles`**

In `src/pipeline/install.rs`, after `install_plugins_from_bundles`:

```rust
// ---------------------------------------------------------------------------
// MCP server configuration orchestration (ADR-0010)
// ---------------------------------------------------------------------------

/// Iterate all resolved bundles and configure each declared MCP server for
/// Claude Code (v1's only target client). CLI-first: `claude mcp add`; on
/// CliNotFound, fall back to writing `.mcp.json`. Warn-skip preserved: a
/// failure never aborts the overall install.
pub(super) fn install_mcps_from_bundles(
    bundles: &[crate::model::Bundle],
    scope: crate::plugin::PluginScope,
    target: &Path,
) -> Vec<McpResult> {
    use crate::model::bundle::McpTransport;
    use crate::plugin::PluginOutcome;

    let mut results = Vec::new();
    for bundle in bundles {
        for (name, entry) in &bundle.mcps {
            let cli_outcome = match &entry.transport {
                McpTransport::Local(local) => {
                    crate::mcp::install_claude_local(name, local, scope)
                }
                McpTransport::Remote(remote) => {
                    crate::mcp::install_claude_remote(name, remote, scope)
                }
            };

            // CLI absent → config-write fallback into .mcp.json.
            let outcome = match cli_outcome {
                PluginOutcome::CliNotFound => match &entry.transport {
                    McpTransport::Local(local) => {
                        crate::ancillary::write_claude_mcp_local(target, name, local)
                    }
                    McpTransport::Remote(remote) => {
                        crate::ancillary::write_claude_mcp_remote(target, name, remote)
                    }
                },
                other => other,
            };

            results.push(McpResult {
                name: name.clone(),
                client: "claude".into(),
                outcome,
                bundle: bundle.name.clone(),
                requires_env: entry.requires_env.clone(),
            });
        }
    }
    results
}
```

Ensure `McpResult` is imported at the top of `install.rs` (it likely already
`use`s `PluginResult` from `super::report` — add `McpResult` to that list).

- [ ] **Step 5: Wire into orchestration + lockfile recording**

In `src/pipeline/mod.rs`, after the plugin block (`report.plugin_results = …`):

```rust
// -- MCP server configuration (ADR-0010) --
let mcp_results = install_mcps_from_bundles(&report.bundles, plugin_scope, target);
report.mcp_results = mcp_results;
```

And after the plugin lockfile-recording loop:

```rust
    // Record configured and warn-skipped MCP servers in the lockfile.
    for mr in &report.mcp_results {
        use crate::lockfile::McpInstallStatus;
        use crate::plugin::PluginOutcome;

        let status = match &mr.outcome {
            PluginOutcome::Success => McpInstallStatus::Installed,
            PluginOutcome::CliNotFound => McpInstallStatus::Skipped,
            // ManualInstructions is unused for MCP; Failed is transient.
            PluginOutcome::ManualInstructions | PluginOutcome::Failed { .. } => continue,
        };
        lock.upsert_mcp(crate::lockfile::LockedMcp {
            name: mr.name.clone(),
            client: mr.client.clone(),
            scope: match plugin_scope {
                crate::plugin::PluginScope::Project => Some("project".into()),
                crate::plugin::PluginScope::User => Some("user".into()),
            },
            bundle: mr.bundle.clone(),
            status,
        });
    }
```

Add `install_mcps_from_bundles` to the `use super::install::{…}` import in
`mod.rs` (next to `install_plugins_from_bundles`).

- [ ] **Step 6: Run the tests + build**

Run: `cargo test --lib pipeline:: && cargo build`
Expected: PASS, zero warnings.

- [ ] **Step 7: Commit**

```bash
git add src/pipeline/report.rs src/pipeline/install.rs src/pipeline/mod.rs src/lockfile.rs
git commit -m "feat(pipeline): configure MCP servers from bundles + record in lockfile"
```

---

## Task 8: main.rs — reporting, `remove mcp`, doctor

**Files:**

- Modify: `src/main.rs`

- [ ] **Step 1: Add MCP result printing**

Mirror `print_plugin_results`. Find it (`src/main.rs:361`) and add alongside:

```rust
fn print_mcp_results(report: &InstallReport) {
    if style::is_quiet() || report.mcp_results.is_empty() {
        return;
    }
    for mr in &report.mcp_results {
        use crate::plugin::PluginOutcome;
        match &mr.outcome {
            PluginOutcome::Success => {
                println!("  configured MCP server '{}' for {}", mr.name, mr.client);
            }
            PluginOutcome::CliNotFound => {
                eprintln!(
                    "warn: {} CLI not found and no config target; skipped MCP '{}'.",
                    mr.client, mr.name
                );
            }
            PluginOutcome::Failed { stderr, .. } => {
                eprintln!("warn: failed to configure MCP '{}' for {}: {stderr}", mr.name, mr.client);
            }
            PluginOutcome::ManualInstructions => {}
        }
        // Warn about declared-but-unset secret env vars.
        for var in &mr.requires_env {
            if std::env::var_os(var).is_none() {
                eprintln!(
                    "warn: MCP '{}' needs env var '{var}'; it is not set in your environment.",
                    mr.name
                );
            }
        }
    }
}
```

Call it where `print_plugin_results(&report)` is called (`src/main.rs:179`):

```rust
print_plugin_results(&report);
print_mcp_results(&report);
```

- [ ] **Step 2: Wire `upskill remove mcp <name>`**

Find the `remove` command dispatch and the plugin-removal branch (search:
`remove_plugins_by_name`). Add a parallel MCP branch. For each
`LockedMcp` matching `name`, call the inverse:

```rust
// MCP removal: inverse of `claude mcp add`, then drop from lockfile.
let mcp_scope = if global {
    crate::plugin::PluginScope::User
} else {
    crate::plugin::PluginScope::Project
};
for mcp in lock.mcps.iter().filter(|m| m.name == name) {
    if mcp.client == "claude" {
        let outcome = crate::mcp::uninstall_claude(&mcp.name, mcp_scope);
        if outcome.is_cli_not_found() {
            // CLI gone → config-write removal fallback.
            crate::ancillary::remove_claude_mcp(target, &mcp.name)?;
        }
    }
}
lock.remove_mcps_by_name(&name);
```

> Match the surrounding code's variable names (`global`, `target`, `lock`,
> `name`) to whatever the existing remove branch uses; the plugin branch
> immediately above is the template.

- [ ] **Step 3: Add doctor reconciliation**

Find the doctor plugin-reconciliation loop (search: `check_claude_plugin_installed`).
Add a parallel MCP loop:

```rust
for mcp in &lock.mcps {
    if mcp.client == "claude" {
        match crate::mcp::check_claude_installed(&mcp.name) {
            crate::plugin::PluginCheckResult::Installed => {}
            crate::plugin::PluginCheckResult::NotInstalled => {
                println!(
                    "  MCP '{}' is in the lockfile but not registered in claude; re-run `upskill update`.",
                    mcp.name
                );
            }
            crate::plugin::PluginCheckResult::CliNotFound => {
                println!("  claude CLI not found; cannot verify MCP '{}'.", mcp.name);
            }
            crate::plugin::PluginCheckResult::QueryFailed { stderr, .. } => {
                println!("  could not query claude MCP list for '{}': {stderr}", mcp.name);
            }
        }
    }
}
```

- [ ] **Step 4: Build + clippy**

Run: `cargo build && cargo clippy --all-targets -- -D warnings`
Expected: zero warnings.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): report, remove, and doctor-reconcile MCP servers"
```

---

## Task 9: lint — surface `mcps` validation

**Files:**

- Modify: `src/lint.rs`

- [ ] **Step 1: Write the failing test**

Add to `src/lint.rs` tests (or `tests/cli_*`): a bundle whose `mcps` has a bad
transport type should make `upskill lint` fail. If `lint.rs` already loads
bundles via `parse::bundle::load`, `validate_mcps` (Task 2) already runs and
this is covered — verify by test:

```rust
#[test]
fn lint_flags_invalid_mcp_transport() {
    // Construct a bundle with an invalid remote type and assert lint reports it.
    use crate::model::bundle::{Bundle, McpEntry, McpRemote, McpTransport};
    use crate::model::common::SchemaVersion;
    use std::collections::BTreeMap;

    let mut mcps = BTreeMap::new();
    mcps.insert(
        "x".to_string(),
        McpEntry {
            transport: McpTransport::Remote(McpRemote {
                transport_type: "ftp".into(),
                url: "https://x".into(),
                headers: BTreeMap::new(),
            }),
            requires_env: vec![],
        },
    );
    let bundle = Bundle {
        schema: SchemaVersion::new(1),
        name: "b".into(),
        description: "d".into(),
        license: None,
        items: Default::default(),
        requires: vec![],
        plugins: BTreeMap::new(),
        mcps,
        metadata: None,
        extra: BTreeMap::new(),
    };
    assert!(bundle.validate_mcps().is_err());
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test --lib lint::tests::lint_flags_invalid_mcp_transport`
Expected: PASS if `validate_mcps` is reachable (it is — Task 2). If `lint.rs`
has its own bundle-walk that bypasses `parse::bundle::load`, add a
`bundle.validate_mcps()` call into that walk and surface the error string.

- [ ] **Step 3: Commit**

```bash
git add src/lint.rs
git commit -m "test(lint): cover mcps transport validation"
```

---

## Task 10: Integration tests — CLI add/remove + warn-skip

**Files:**

- Create: `tests/cli_mcp.rs`
- Create: `tests/pipeline_mcp.rs`

- [ ] **Step 1: Write `tests/cli_mcp.rs`**

```rust
//! Integration: `upskill add` of a local bundle declaring an MCP server.
//! In CI no `claude` CLI is on PATH, so the config-write fallback fires and
//! `.mcp.json` is written.

use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

/// Build a minimal SSOT registry with one skill and a bundle that declares
/// a local MCP server, then `upskill add` it from the local path.
#[test]
fn add_bundle_with_mcp_writes_claude_mcp_json() {
    let registry = tempdir().unwrap();
    // A skill the bundle ships.
    let skill_dir = registry.path().join("drawio-diagrams");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: drawio-diagrams\ndescription: draw.io diagrams\n---\nbody\n",
    )
    .unwrap();
    // The bundle.
    fs::write(
        registry.path().join("drawio.bundle.yaml"),
        "schema: 1\n\
         name: drawio\n\
         description: draw.io skill and MCP\n\
         items:\n  skills:\n    - drawio-diagrams\n\
         mcps:\n  drawio:\n    local:\n      command: npx\n      args: [\"-y\", \"drawio-mcp-server\"]\n      env:\n        DRAWIO_TOKEN: \"${DRAWIO_TOKEN}\"\n    requires-env: [DRAWIO_TOKEN]\n",
    )
    .unwrap();

    let project = tempdir().unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(project.path())
        .env_remove("PATH") // ensure no `claude` CLI → config-write fallback
        .env("PATH", "/nonexistent")
        .args(["add", registry.path().to_str().unwrap(), "--claude"])
        .assert()
        .success();

    let mcp_json = fs::read_to_string(project.path().join(".mcp.json")).unwrap();
    assert!(mcp_json.contains("\"drawio\""), "got: {mcp_json}");
    assert!(mcp_json.contains("npx"), "got: {mcp_json}");
    assert!(mcp_json.contains("${DRAWIO_TOKEN}"), "secret reference preserved verbatim");

    // Lockfile records the MCP entry.
    let lock = fs::read_to_string(project.path().join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("\"drawio\""));
    assert!(lock.contains("\"mcps\""));
}
```

> If `env_remove("PATH")` then setting `PATH=/nonexistent` is awkward on the
> CI runner (`upskill` itself was already located by `cargo_bin`, which
> returns an absolute path, so it still runs), keep both lines. If the test
> harness needs `git` on PATH for the local-path source, instead point `PATH`
> at a temp dir containing only a `git` symlink. Verify by running the test.

- [ ] **Step 2: Run it to verify it fails, then passes**

Run: `cargo test --test cli_mcp`
Expected: After Tasks 1–8, PASS. If it fails on PATH handling, adjust per the
note above and re-run.

- [ ] **Step 3: Write `tests/pipeline_mcp.rs`**

```rust
//! Integration: lockfile records MCP servers and `remove` drops them.

use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

fn make_registry() -> tempfile::TempDir {
    let registry = tempdir().unwrap();
    let skill_dir = registry.path().join("s");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "---\nname: s\ndescription: s\n---\nb\n").unwrap();
    fs::write(
        registry.path().join("b.bundle.yaml"),
        "schema: 1\nname: b\ndescription: d\nitems:\n  skills:\n    - s\n\
         mcps:\n  remote-srv:\n    remote:\n      type: http\n      url: https://example.com/mcp\n",
    )
    .unwrap();
    registry
}

#[test]
fn add_then_remove_mcp_updates_lockfile() {
    let registry = make_registry();
    let project = tempdir().unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(project.path())
        .env("PATH", "/nonexistent")
        .args(["add", registry.path().to_str().unwrap(), "--claude"])
        .assert()
        .success();

    let lock = fs::read_to_string(project.path().join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("remote-srv"));

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(project.path())
        .env("PATH", "/nonexistent")
        .args(["remove", "mcp", "remote-srv"])
        .assert()
        .success();

    let lock = fs::read_to_string(project.path().join(".upskill-lock.json")).unwrap();
    assert!(!lock.contains("remote-srv"), "MCP entry removed from lockfile");
}
```

> Confirm the exact `remove` sub-command shape against the existing CLI
> (`upskill remove plugin <name>` is the template — match its noun position
> for `mcp`). Adjust the args array if the CLI uses `remove --mcp` instead.

- [ ] **Step 4: Run it**

Run: `cargo test --test pipeline_mcp`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add tests/cli_mcp.rs tests/pipeline_mcp.rs
git commit -m "test(mcp): CLI add config-write + lockfile record + remove"
```

---

## Task 11: Docs — ADR-0010 + format-spec

**Files:**

- Create: `docs/adr/0010-mcp-config-write.md`
- Modify: `docs/format-spec.md`
- Modify: `docs/adr/README.md` (index, if it lists ADRs)

- [ ] **Step 1: Write ADR-0010**

Create `docs/adr/0010-mcp-config-write.md` capturing: context (no MCP support;
skills need their server), decision (bundle-level `mcps:`, CLI-first with
config-write fallback, `${VAR}` indirection + `requires-env` declare-and-check,
lockfile + doctor lifecycle), the trust invariant (upskill never expands
`${VAR}`, never runs server code), consequences, and the alternatives rejected
(verbatim from the spec's _Alternatives considered_). Model it on
`docs/adr/0008-plugin-install-shellout.md`.

- [ ] **Step 2: Document the `mcps:` sub-shape in format-spec**

Add an `mcps:` subsection to `docs/format-spec.md` §3 (next to the `plugins:`
sub-shape) with the YAML example from the spec, the secret-indirection rule,
and the lockfile `mcps[]` entry shape (note the `schema` stays `1` — additive).

- [ ] **Step 3: Run docs lint + book build**

Run: `just fmt && just book`
Expected: builds clean.

- [ ] **Step 4: Commit**

```bash
git add docs/
git commit -m "docs(mcp): ADR-0010 and format-spec mcps: sub-shape"
```

---

## Task 12: Final verification + open issue/PR

- [ ] **Step 1: Full check**

Run: `just fmt && just verify`
Expected: all green (test + lint + build).

- [ ] **Step 2: Open the epic-linked story and PR**

Per AGENTS.md issue model, ensure a story issue exists (`Epic: #N` first body
line, `story` + `epic:<name>` labels). Then push the branch and open one PR
carrying code + tests + docs. Use a Conventional Commit title:
`feat: configure MCP servers from bundles (v1)`.

- [ ] **Step 3: After CI**

Fix CI failures first, then address review comments. Track v2 (skill
`requires.mcps` + install agent) as a follow-up story.

---

## Self-Review

**Spec coverage:**

- Bundle-level `mcps:` descriptor (remote/local) → Tasks 1–2. ✓
- `${VAR}` indirection, no expansion → upskill never reads env values; passes verbatim (Tasks 4–5). ✓
- `requires-env` declare-and-check → model (Task 1) + doctor/add warn (Task 8). ✓
- CLI-first, config-write fallback (Claude) → Tasks 4, 5, 7. ✓
- Lockfile + remove + doctor lifecycle → Tasks 6, 8. ✓
- Warn-skip policy preserved → Task 7 (CliNotFound → fallback; Failed → continue) + Task 8 messaging. ✓
- Trust invariant (no code execution, no secret custody) → no expansion anywhere; only `claude mcp add` shellout + JSON write. ✓
- Tests (unit + integration) → Tasks 1–10. ✓
- ADR-0010 + format-spec → Task 11. ✓

**Scope notes / deliberate v1 limits (log, don't silently drop):**

- v1 targets **Claude Code only** for active configuration. opencode/VS Code/Copilot MCP writers are deferred — `install_mcps_from_bundles` records a `claude` result only. If a bundle author expects opencode MCP config in v1, that is **not** delivered; note this in the PR description and the ADR's consequences. (Extending to opencode is a small follow-up: add `write_opencode_mcp` + a branch in `install_mcps_from_bundles`.)
- v1 has **no `requires.mcps`** skill dependency edge — MCP travels with a skill only by shared bundle membership (v2).

**Placeholder scan:** No "TBD"/"handle appropriately" steps; every code step shows code. The two `> Adjust …` notes (Bundle struct literal field list; CLI remove noun) are real seams against current code, with the exact verification command given — not placeholders.

**Type consistency:** `McpEntry { transport, requires_env }`, `McpTransport::{Remote,Local}`, `McpRemote { transport_type, url, headers }`, `McpLocal { command, args, env }`, `LockedMcp { name, client, scope, bundle, status }`, `McpInstallStatus::{Installed,Skipped}`, `McpResult { name, client, outcome, bundle, requires_env }` — names used identically across Tasks 1, 6, 7, 8, 9, 10. `install_mcps_from_bundles` / `install_claude_local` / `install_claude_remote` / `uninstall_claude` / `check_claude_installed` / `write_claude_mcp_local` / `write_claude_mcp_remote` / `remove_claude_mcp` — consistent across Tasks 4, 5, 7, 8.
