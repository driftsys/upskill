//! Ancillary file management for the v0.2 install pipeline.
//!
//! Per [ADR-0003](../../docs/adr/0003-generation-pipeline.md) §"Ancillary
//! file handling" and format-spec §7.4. The pipeline writes per-client item
//! output (rules / skills / agents) to the paths in §7; some clients also
//! need a single one-time hand-shake file at the consumer-project root for
//! discovery to work. Those files are managed here, separately from the
//! per-item generation pipeline:
//!
//! - **`CLAUDE.md`** — created once with `@AGENTS.md` content if absent.
//!   Bridges Claude Code (which does not natively read `AGENTS.md`) to the
//!   project-level instructions Claude users expect to live in `AGENTS.md`.
//!   Never overwritten — protects user customisations.
//!
//! - **`opencode.json`** — when an install includes any rule item, the glob
//!   `.agents/rules/**/RULE.md` is added to the `instructions[]` array.
//!   Idempotent (existing entry → no-op); unknown keys preserved. opencode
//!   discovers rules through this glob; without it, rules under
//!   `.agents/rules/` are invisible.
//!
//! - **`.vscode/settings.json`** — when an install includes any rule item,
//!   the entry `".github/instructions": true` is added to the
//!   `chat.instructionsFilesLocations` object. VS Code Copilot uses this map
//!   (`Record<string, boolean>`) to discover instruction files. The default
//!   already includes `.github/instructions`, but writing it explicitly
//!   documents the project dependency. Idempotent and respectful of an
//!   existing user value (true *or* false) for the same path. Unknown keys
//!   preserved.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::path::Path;

use crate::model::bundle::{McpLocal, McpRemote};
use crate::plugin::PluginOutcome;

/// Filename written at the consumer-project root.
const CLAUDE_MD: &str = "CLAUDE.md";

/// Content the bridge file ships with. The single `@AGENTS.md` line is the
/// Claude Code "load this file" directive; everything Claude Code needs at
/// the project level is then expected to live in `AGENTS.md`.
const CLAUDE_MD_BRIDGE: &str = "@AGENTS.md\n";

/// opencode config filename at the consumer-project root.
const OPENCODE_JSON: &str = "opencode.json";

/// Glob entry added to opencode's `instructions[]` so rules under
/// `.agents/rules/` are discovered (opencode walks the glob, which dot-aware
/// matches the hidden `.agents/` directory per ADR-0003 implementation note).
const OPENCODE_RULES_GLOB: &str = ".agents/rules/**/RULE.md";

/// VS Code workspace settings file at the consumer-project root.
const VSCODE_SETTINGS: &str = ".vscode/settings.json";

/// VS Code Copilot setting key whose value is a `Record<string, boolean>`
/// mapping path globs to whether VS Code searches them for instruction
/// files.
const VSCODE_INSTRUCTIONS_KEY: &str = "chat.instructionsFilesLocations";

/// Path entry inserted into `chat.instructionsFilesLocations` so VS Code
/// Copilot picks up generated rule files at `.github/instructions/`. Matches
/// the per-client output path for Copilot rules in format-spec §7.
const VSCODE_INSTRUCTIONS_PATH: &str = ".github/instructions";

/// Outcome of a single ancillary write, surfaced for callers that want to
/// log or report what happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AncillaryAction {
    /// File did not exist; we created it.
    Created,
    /// File already existed; we updated it (e.g., appended an entry).
    Updated,
    /// File already existed and required no change.
    Preserved,
}

/// Ensure `<target>/CLAUDE.md` exists with the bridge content.
///
/// - Absent → created with `CLAUDE_MD_BRIDGE`.
/// - Present → never modified, regardless of content. A user (or a previous
///   `upskill` run) may have customised the file; preserving it is part of
///   the contract per ADR-0003.
pub fn ensure_claude_bridge(target: &Path) -> Result<AncillaryAction> {
    let path = target.join(CLAUDE_MD);
    if path.exists() {
        return Ok(AncillaryAction::Preserved);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }
    std::fs::write(&path, CLAUDE_MD_BRIDGE).with_context(|| format!("write {}", path.display()))?;
    Ok(AncillaryAction::Created)
}

/// Ensure `<target>/opencode.json` lists `.agents/rules/**/RULE.md` in
/// `instructions[]` so opencode can discover rule files. No-op when the
/// install contains no rules.
///
/// Behaviour:
/// - `has_rules == false` → `Preserved`, file untouched.
/// - File absent + has_rules → `Created` with
///   `{"instructions": [".agents/rules/**/RULE.md"]}`.
/// - File present and entry already in `instructions[]` → `Preserved`.
/// - File present without `instructions` → `Updated`, array added; other
///   keys preserved.
/// - File present with `instructions` lacking the entry → `Updated`,
///   entry appended; existing entries preserved.
///
/// `instructions` MUST be an array; if the existing file has it set to a
/// non-array value, this function returns an error rather than clobbering
/// user data.
pub fn ensure_opencode_rules_registered(target: &Path, has_rules: bool) -> Result<AncillaryAction> {
    if !has_rules {
        return Ok(AncillaryAction::Preserved);
    }

    let path = target.join(OPENCODE_JSON);
    match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let new_doc = json!({ "instructions": [OPENCODE_RULES_GLOB] });
            write_pretty_json(&path, &new_doc)?;
            Ok(AncillaryAction::Created)
        }
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
        Ok(raw) => {
            let mut doc: Value =
                serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
            if !doc.is_object() {
                anyhow::bail!("{}: top-level value must be an object", path.display());
            }

            let obj = doc.as_object_mut().expect("checked is_object");
            let entry = match obj.get_mut("instructions") {
                None => {
                    obj.insert("instructions".to_string(), json!([OPENCODE_RULES_GLOB]));
                    AncillaryAction::Updated
                }
                Some(existing) => {
                    let arr = existing.as_array_mut().with_context(|| {
                        format!("{}: `instructions` must be an array", path.display())
                    })?;
                    if arr.iter().any(|v| v.as_str() == Some(OPENCODE_RULES_GLOB)) {
                        return Ok(AncillaryAction::Preserved);
                    }
                    arr.push(json!(OPENCODE_RULES_GLOB));
                    AncillaryAction::Updated
                }
            };

            write_pretty_json(&path, &doc)?;
            Ok(entry)
        }
    }
}

/// Ensure `<target>/.vscode/settings.json` lists `.github/instructions` in
/// `chat.instructionsFilesLocations` so VS Code Copilot discovers generated
/// rule files. No-op when the install contains no rules.
///
/// Behaviour:
/// - `has_rules == false` → `Preserved`, file untouched.
/// - File absent + has_rules → `Created` with
///   `{"chat.instructionsFilesLocations": {".github/instructions": true}}`.
/// - File present without `chat.instructionsFilesLocations` → `Updated`,
///   key added; other keys preserved.
/// - File present with the path entry already (any boolean value) →
///   `Preserved`. We respect `false` because the user explicitly turned the
///   default location off; flipping it back would override their choice.
/// - File present with the key but the path absent → `Updated`, entry
///   appended with `true`; existing entries preserved.
///
/// `chat.instructionsFilesLocations` MUST be an object; if the existing
/// file has it set to a non-object value, this function returns an error
/// rather than clobbering user data. The same applies to the top level of
/// `settings.json`.
///
/// Limitation: `serde_json` parses strict JSON. VS Code natively writes
/// JSONC (line comments, trailing commas) and a user-edited
/// `settings.json` may legally contain either. This function returns an
/// error on those, matching the `opencode.json` handler. Tracked as a
/// follow-up if it bites in practice.
pub fn ensure_vscode_instructions_registered(
    target: &Path,
    has_rules: bool,
) -> Result<AncillaryAction> {
    if !has_rules {
        return Ok(AncillaryAction::Preserved);
    }

    let path = target.join(VSCODE_SETTINGS);
    match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let new_doc = json!({
                VSCODE_INSTRUCTIONS_KEY: { VSCODE_INSTRUCTIONS_PATH: true },
            });
            write_pretty_json(&path, &new_doc)?;
            Ok(AncillaryAction::Created)
        }
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
        Ok(raw) => {
            let mut doc: Value =
                serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
            if !doc.is_object() {
                anyhow::bail!("{}: top-level value must be an object", path.display());
            }

            let obj = doc.as_object_mut().expect("checked is_object");
            let action = match obj.get_mut(VSCODE_INSTRUCTIONS_KEY) {
                None => {
                    obj.insert(
                        VSCODE_INSTRUCTIONS_KEY.to_string(),
                        json!({ VSCODE_INSTRUCTIONS_PATH: true }),
                    );
                    AncillaryAction::Updated
                }
                Some(existing) => {
                    let map = existing.as_object_mut().with_context(|| {
                        format!(
                            "{}: `{}` must be an object",
                            path.display(),
                            VSCODE_INSTRUCTIONS_KEY
                        )
                    })?;
                    if map.contains_key(VSCODE_INSTRUCTIONS_PATH) {
                        return Ok(AncillaryAction::Preserved);
                    }
                    map.insert(VSCODE_INSTRUCTIONS_PATH.to_string(), json!(true));
                    AncillaryAction::Updated
                }
            };

            write_pretty_json(&path, &doc)?;
            Ok(action)
        }
    }
}

/// Write a plugin URI to the `plugin[]` array in `<target>/opencode.json`.
///
/// Idempotent: if the URI is already present, no change is made.
/// Creates the file if absent. Returns `PluginOutcome::Failed` if the file
/// exists but is not a JSON object or `plugin` is not an array.
pub fn write_opencode_plugin_uri(target: &Path, plugin_uri: &str) -> PluginOutcome {
    let path = target.join(OPENCODE_JSON);
    match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let new_doc = json!({ "plugin": [plugin_uri] });
            if let Err(e) = write_pretty_json(&path, &new_doc) {
                return PluginOutcome::Failed {
                    exit_code: None,
                    stderr: e.to_string(),
                };
            }
            PluginOutcome::Success
        }
        Err(e) => PluginOutcome::Failed {
            exit_code: None,
            stderr: format!("read {}: {e}", path.display()),
        },
        Ok(raw) => {
            let mut doc: Value = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(e) => {
                    return PluginOutcome::Failed {
                        exit_code: None,
                        stderr: format!("parse {}: {e}", path.display()),
                    };
                }
            };
            if !doc.is_object() {
                return PluginOutcome::Failed {
                    exit_code: None,
                    stderr: format!("{}: top-level value must be an object", path.display()),
                };
            }

            let obj = doc.as_object_mut().expect("checked is_object");
            match obj.get_mut("plugin") {
                None => {
                    obj.insert("plugin".to_string(), json!([plugin_uri]));
                }
                Some(existing) => {
                    let Some(arr) = existing.as_array_mut() else {
                        return PluginOutcome::Failed {
                            exit_code: None,
                            stderr: format!("{}: `plugin` must be an array", path.display()),
                        };
                    };
                    if arr.iter().any(|v| v.as_str() == Some(plugin_uri)) {
                        return PluginOutcome::Success;
                    }
                    arr.push(json!(plugin_uri));
                }
            }

            if let Err(e) = write_pretty_json(&path, &doc) {
                return PluginOutcome::Failed {
                    exit_code: None,
                    stderr: e.to_string(),
                };
            }
            PluginOutcome::Success
        }
    }
}

/// Remove a plugin URI from the `plugin[]` array in `<target>/opencode.json`.
///
/// No-op if the file is absent or the URI is not in the array.
pub fn remove_opencode_plugin_uri(target: &Path, plugin_uri: &str) -> Result<()> {
    let path = target.join(OPENCODE_JSON);
    let raw = match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
        Ok(r) => r,
    };

    let mut doc: Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;

    let Some(obj) = doc.as_object_mut() else {
        anyhow::bail!("{}: top-level value must be an object", path.display());
    };

    if let Some(existing) = obj.get_mut("plugin") {
        let arr = existing
            .as_array_mut()
            .with_context(|| format!("{}: `plugin` must be an array", path.display()))?;
        let before = arr.len();
        arr.retain(|v| v.as_str() != Some(plugin_uri));
        if arr.len() < before {
            write_pretty_json(&path, &doc)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// MCP config-write fallbacks (ADR-0010, issue #237)
//
// Each MCP target has its own config file and root key; the CLI is preferred
// and these writers run only when the target's CLI is absent. All writes are
// merge-preserving via `upsert_mcp_server` and never expand `${VAR}` values.
// ---------------------------------------------------------------------------

/// Filename Claude Code reads for project-scoped MCP servers (root key
/// `mcpServers`).
const CLAUDE_MCP_JSON: &str = ".mcp.json";
/// Filename VS Code reads for workspace MCP servers (root key `servers`).
const VSCODE_MCP_JSON: &str = ".vscode/mcp.json";
/// User-scope file the GitHub Copilot CLI reads (root key `mcpServers`).
/// Copilot has no documented project-scope config file, so the fallback is
/// always user-scope (issue #237 caveat).
const COPILOT_MCP_CONFIG: &str = ".copilot/mcp-config.json";

/// Resolve `~/.copilot/mcp-config.json`. `None` when neither `HOME` nor
/// `USERPROFILE` is set.
fn copilot_mcp_config_path() -> Option<std::path::PathBuf> {
    crate::source::home_dir().map(|home| home.join(COPILOT_MCP_CONFIG))
}

// -- Claude / Copilot share the `{ "<root>": { "<name>": { … } } }` shape --

/// `mcpServers`-style server object for a local (stdio) descriptor:
/// `{ command, args?, env? }`.
fn mcp_servers_local_value(local: &McpLocal) -> Value {
    let mut server = serde_json::Map::new();
    server.insert("command".into(), json!(local.command));
    if !local.args.is_empty() {
        server.insert("args".into(), json!(local.args));
    }
    if !local.env.is_empty() {
        server.insert("env".into(), json!(local.env));
    }
    Value::Object(server)
}

/// `mcpServers`-style server object for a remote descriptor:
/// `{ type, url, headers? }`.
fn mcp_servers_remote_value(remote: &McpRemote) -> Value {
    let mut server = serde_json::Map::new();
    server.insert("type".into(), json!(remote.transport_type));
    server.insert("url".into(), json!(remote.url));
    if !remote.headers.is_empty() {
        server.insert("headers".into(), json!(remote.headers));
    }
    Value::Object(server)
}

/// Write a local (stdio) MCP server into `<target>/.mcp.json` under
/// `mcpServers.<name>`. Merge-preserving; creates the file if absent.
/// Used as the fallback when the `claude` CLI is not on PATH.
pub fn write_claude_mcp_local(target: &Path, name: &str, local: &McpLocal) -> PluginOutcome {
    upsert_mcp_server(
        &target.join(CLAUDE_MCP_JSON),
        "mcpServers",
        name,
        mcp_servers_local_value(local),
    )
}

/// Write a remote MCP server into `<target>/.mcp.json` under
/// `mcpServers.<name>`. Merge-preserving; creates the file if absent.
pub fn write_claude_mcp_remote(target: &Path, name: &str, remote: &McpRemote) -> PluginOutcome {
    upsert_mcp_server(
        &target.join(CLAUDE_MCP_JSON),
        "mcpServers",
        name,
        mcp_servers_remote_value(remote),
    )
}

/// Write a local (stdio) MCP server into `~/.copilot/mcp-config.json` under
/// `mcpServers.<name>` (Copilot's user-scope file). Fallback when the
/// `copilot` CLI is not on PATH.
pub fn write_copilot_mcp_local(name: &str, local: &McpLocal) -> PluginOutcome {
    let Some(path) = copilot_mcp_config_path() else {
        return PluginOutcome::Failed {
            exit_code: None,
            stderr: "could not determine HOME for ~/.copilot/mcp-config.json".into(),
        };
    };
    upsert_mcp_server(&path, "mcpServers", name, mcp_servers_local_value(local))
}

/// Write a remote MCP server into `~/.copilot/mcp-config.json` under
/// `mcpServers.<name>`.
pub fn write_copilot_mcp_remote(name: &str, remote: &McpRemote) -> PluginOutcome {
    let Some(path) = copilot_mcp_config_path() else {
        return PluginOutcome::Failed {
            exit_code: None,
            stderr: "could not determine HOME for ~/.copilot/mcp-config.json".into(),
        };
    };
    upsert_mcp_server(&path, "mcpServers", name, mcp_servers_remote_value(remote))
}

// -- VS Code: `.vscode/mcp.json`, root key `servers`, stdio typed `"stdio"` --

/// VS Code server object for a local (stdio) descriptor:
/// `{ type: "stdio", command, args?, env? }`.
fn vscode_local_value(local: &McpLocal) -> Value {
    let mut server = serde_json::Map::new();
    server.insert("type".into(), json!("stdio"));
    server.insert("command".into(), json!(local.command));
    if !local.args.is_empty() {
        server.insert("args".into(), json!(local.args));
    }
    if !local.env.is_empty() {
        server.insert("env".into(), json!(local.env));
    }
    Value::Object(server)
}

/// VS Code server object for a remote descriptor: `{ type, url, headers? }`
/// where `type` is the `http`/`sse` transport value.
fn vscode_remote_value(remote: &McpRemote) -> Value {
    let mut server = serde_json::Map::new();
    server.insert("type".into(), json!(remote.transport_type));
    server.insert("url".into(), json!(remote.url));
    if !remote.headers.is_empty() {
        server.insert("headers".into(), json!(remote.headers));
    }
    Value::Object(server)
}

/// Write a local (stdio) MCP server into `<target>/.vscode/mcp.json` under
/// `servers.<name>`. Merge-preserving; fallback when `code` is not on PATH.
pub fn write_vscode_mcp_local(target: &Path, name: &str, local: &McpLocal) -> PluginOutcome {
    upsert_mcp_server(
        &target.join(VSCODE_MCP_JSON),
        "servers",
        name,
        vscode_local_value(local),
    )
}

/// Write a remote MCP server into `<target>/.vscode/mcp.json` under
/// `servers.<name>`.
pub fn write_vscode_mcp_remote(target: &Path, name: &str, remote: &McpRemote) -> PluginOutcome {
    upsert_mcp_server(
        &target.join(VSCODE_MCP_JSON),
        "servers",
        name,
        vscode_remote_value(remote),
    )
}

// -- opencode: `opencode.json`, root key `mcp`, distinct field shape --

/// opencode server object for a local descriptor: `{ type: "local",
/// command: [command, args…], environment? }`. opencode folds command+args
/// into one array and names the env map `environment`.
fn opencode_local_value(local: &McpLocal) -> Value {
    let mut command = Vec::with_capacity(local.args.len() + 1);
    command.push(local.command.clone());
    command.extend(local.args.iter().cloned());

    let mut server = serde_json::Map::new();
    server.insert("type".into(), json!("local"));
    server.insert("command".into(), json!(command));
    if !local.env.is_empty() {
        server.insert("environment".into(), json!(local.env));
    }
    Value::Object(server)
}

/// opencode server object for a remote descriptor: `{ type: "remote", url,
/// headers? }`.
fn opencode_remote_value(remote: &McpRemote) -> Value {
    let mut server = serde_json::Map::new();
    server.insert("type".into(), json!("remote"));
    server.insert("url".into(), json!(remote.url));
    if !remote.headers.is_empty() {
        server.insert("headers".into(), json!(remote.headers));
    }
    Value::Object(server)
}

/// Write a local MCP server into `<target>/opencode.json` under `mcp.<name>`.
/// Merge-preserving (opencode.json also holds `instructions`/`plugin`); this
/// is opencode's only configuration path — it has no `mcp add` CLI verb.
pub fn write_opencode_mcp_local(target: &Path, name: &str, local: &McpLocal) -> PluginOutcome {
    upsert_mcp_server(
        &target.join(OPENCODE_JSON),
        "mcp",
        name,
        opencode_local_value(local),
    )
}

/// Write a remote MCP server into `<target>/opencode.json` under `mcp.<name>`.
pub fn write_opencode_mcp_remote(target: &Path, name: &str, remote: &McpRemote) -> PluginOutcome {
    upsert_mcp_server(
        &target.join(OPENCODE_JSON),
        "mcp",
        name,
        opencode_remote_value(remote),
    )
}

/// Shared merge: insert `server` at `<root_key>.<name>` in `path`,
/// preserving any other top-level keys and any sibling servers. Creates the
/// file (and parent dirs) when absent.
fn upsert_mcp_server(path: &Path, root_key: &str, name: &str, server: Value) -> PluginOutcome {
    let mut doc: Value = match std::fs::read_to_string(path) {
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
        .entry(root_key)
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let Some(servers) = servers.as_object_mut() else {
        return PluginOutcome::Failed {
            exit_code: None,
            stderr: format!("{}: `{root_key}` must be an object", path.display()),
        };
    };
    servers.insert(name.to_string(), server);

    if let Err(e) = write_pretty_json(path, &doc) {
        return PluginOutcome::Failed {
            exit_code: None,
            stderr: e.to_string(),
        };
    }
    PluginOutcome::Success
}

/// Remove an MCP server from `<target>/.mcp.json`. No-op if absent.
pub fn remove_claude_mcp(target: &Path, name: &str) -> Result<()> {
    remove_mcp_server(&target.join(CLAUDE_MCP_JSON), "mcpServers", name)
}

/// Remove an MCP server from `<target>/.vscode/mcp.json`. No-op if absent.
pub fn remove_vscode_mcp(target: &Path, name: &str) -> Result<()> {
    remove_mcp_server(&target.join(VSCODE_MCP_JSON), "servers", name)
}

/// Remove an MCP server from `<target>/opencode.json` `mcp` map. No-op if
/// absent.
pub fn remove_opencode_mcp(target: &Path, name: &str) -> Result<()> {
    remove_mcp_server(&target.join(OPENCODE_JSON), "mcp", name)
}

/// Remove an MCP server from `~/.copilot/mcp-config.json`. No-op if absent or
/// if HOME is unset.
pub fn remove_copilot_mcp(name: &str) -> Result<()> {
    match copilot_mcp_config_path() {
        Some(path) => remove_mcp_server(&path, "mcpServers", name),
        None => Ok(()),
    }
}

/// Shared removal: drop `<root_key>.<name>` from `path`. No-op if the file is
/// absent. Preserves all other keys and servers.
fn remove_mcp_server(path: &Path, root_key: &str, name: &str) -> Result<()> {
    let raw = match std::fs::read_to_string(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
        Ok(r) => r,
    };
    let mut doc: Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    if let Some(servers) = doc.get_mut(root_key).and_then(Value::as_object_mut) {
        servers.remove(name);
    }
    write_pretty_json(path, &doc)
}

// ---------------------------------------------------------------------------
// MCP doctor reconciliation for config-write targets (issue #237, #241)
// ---------------------------------------------------------------------------

/// Result of probing a config-write target's file for a named MCP server.
/// Lets `doctor` distinguish "configured", "drifted", "file absent", and
/// "file unreadable" instead of collapsing the last three into a silent skip
/// (issue #241).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpConfigState {
    /// The server is present in the target's config file.
    Present,
    /// The config file exists but the server entry is gone (genuine drift).
    Missing,
    /// The config file does not exist — state undetermined (the server may be
    /// configured via the client's own CLI in a store upskill cannot read).
    FileAbsent,
    /// The config file exists but could not be read or parsed as JSON.
    Unreadable(String),
}

/// Doctor query: is `<root_key>.<name>` present in the JSON config at `path`?
/// Distinguishes absent from unreadable so `doctor` can surface a broken or
/// missing config instead of silently skipping it.
fn mcp_config_state(path: &Path, root_key: &str, name: &str) -> McpConfigState {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return McpConfigState::FileAbsent,
        Err(e) => return McpConfigState::Unreadable(e.to_string()),
    };
    let doc: Value = match serde_json::from_str(&raw) {
        Ok(doc) => doc,
        Err(e) => return McpConfigState::Unreadable(e.to_string()),
    };
    if doc
        .get(root_key)
        .and_then(Value::as_object)
        .is_some_and(|m| m.contains_key(name))
    {
        McpConfigState::Present
    } else {
        McpConfigState::Missing
    }
}

/// Doctor state for a VS Code MCP entry (`.vscode/mcp.json` `servers.<name>`).
pub fn vscode_mcp_state(target: &Path, name: &str) -> McpConfigState {
    mcp_config_state(&target.join(VSCODE_MCP_JSON), "servers", name)
}

/// Doctor state for an opencode MCP entry (`opencode.json` `mcp.<name>`).
pub fn opencode_mcp_state(target: &Path, name: &str) -> McpConfigState {
    mcp_config_state(&target.join(OPENCODE_JSON), "mcp", name)
}

/// Doctor state for a Copilot MCP entry (`~/.copilot/mcp-config.json`
/// `mcpServers.<name>`). Treated as `FileAbsent` when HOME is unset.
pub fn copilot_mcp_state(name: &str) -> McpConfigState {
    match copilot_mcp_config_path() {
        Some(path) => mcp_config_state(&path, "mcpServers", name),
        None => McpConfigState::FileAbsent,
    }
}

fn write_pretty_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value).context("serialize JSON")?;
    std::fs::write(path, format!("{json}\n")).with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_claude_md_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let action = ensure_claude_bridge(tmp.path()).expect("ensure");

        assert_eq!(action, AncillaryAction::Created);
        let content = std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
        assert_eq!(content, CLAUDE_MD_BRIDGE);
    }

    #[test]
    fn preserves_existing_claude_md_verbatim() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        let user_content = "# My CLAUDE.md\n\nUser customisations here.\n";
        std::fs::write(&path, user_content).unwrap();

        let action = ensure_claude_bridge(tmp.path()).expect("ensure");

        assert_eq!(action, AncillaryAction::Preserved);
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, user_content, "must not overwrite user content");
    }

    #[test]
    fn second_call_after_create_is_preserve() {
        let tmp = tempfile::tempdir().unwrap();
        let first = ensure_claude_bridge(tmp.path()).expect("ensure 1");
        let second = ensure_claude_bridge(tmp.path()).expect("ensure 2");
        assert_eq!(first, AncillaryAction::Created);
        assert_eq!(second, AncillaryAction::Preserved);
    }

    fn read_opencode(target: &Path) -> Value {
        let raw = std::fs::read_to_string(target.join(OPENCODE_JSON)).unwrap();
        serde_json::from_str(&raw).unwrap()
    }

    #[test]
    fn opencode_no_rules_is_no_op() {
        let tmp = tempfile::tempdir().unwrap();
        let action = ensure_opencode_rules_registered(tmp.path(), false).expect("ensure");
        assert_eq!(action, AncillaryAction::Preserved);
        assert!(!tmp.path().join(OPENCODE_JSON).exists(), "no file created");
    }

    #[test]
    fn opencode_creates_file_when_absent_and_has_rules() {
        let tmp = tempfile::tempdir().unwrap();
        let action = ensure_opencode_rules_registered(tmp.path(), true).expect("ensure");
        assert_eq!(action, AncillaryAction::Created);
        let doc = read_opencode(tmp.path());
        assert_eq!(doc["instructions"][0], OPENCODE_RULES_GLOB);
    }

    #[test]
    fn opencode_preserves_existing_entry() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(OPENCODE_JSON),
            format!(
                "{{\"instructions\": [\"{}\"], \"theme\": \"dark\"}}",
                OPENCODE_RULES_GLOB
            ),
        )
        .unwrap();

        let action = ensure_opencode_rules_registered(tmp.path(), true).expect("ensure");
        assert_eq!(action, AncillaryAction::Preserved);
        let doc = read_opencode(tmp.path());
        // Other keys preserved.
        assert_eq!(doc["theme"], "dark");
        assert_eq!(doc["instructions"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn opencode_appends_when_instructions_present_without_entry() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(OPENCODE_JSON),
            r#"{"instructions": ["other.md"], "theme": "dark"}"#,
        )
        .unwrap();

        let action = ensure_opencode_rules_registered(tmp.path(), true).expect("ensure");
        assert_eq!(action, AncillaryAction::Updated);
        let doc = read_opencode(tmp.path());
        let arr = doc["instructions"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0], "other.md");
        assert_eq!(arr[1], OPENCODE_RULES_GLOB);
        assert_eq!(doc["theme"], "dark", "other keys preserved");
    }

    #[test]
    fn opencode_adds_instructions_when_field_absent() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(OPENCODE_JSON),
            r#"{"theme": "dark", "model": "sonnet"}"#,
        )
        .unwrap();

        let action = ensure_opencode_rules_registered(tmp.path(), true).expect("ensure");
        assert_eq!(action, AncillaryAction::Updated);
        let doc = read_opencode(tmp.path());
        assert_eq!(doc["instructions"][0], OPENCODE_RULES_GLOB);
        assert_eq!(doc["theme"], "dark");
        assert_eq!(doc["model"], "sonnet");
    }

    #[test]
    fn opencode_errors_on_non_array_instructions() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(OPENCODE_JSON),
            r#"{"instructions": "not-an-array"}"#,
        )
        .unwrap();

        let err =
            ensure_opencode_rules_registered(tmp.path(), true).expect_err("must reject non-array");
        assert!(err.to_string().contains("instructions"));
    }

    #[test]
    fn opencode_errors_on_non_object_top_level() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(OPENCODE_JSON), r#"["not", "an", "object"]"#).unwrap();
        let err = ensure_opencode_rules_registered(tmp.path(), true).expect_err("must reject");
        assert!(err.to_string().contains("object"));
    }

    fn read_vscode(target: &Path) -> Value {
        let raw = std::fs::read_to_string(target.join(VSCODE_SETTINGS)).unwrap();
        serde_json::from_str(&raw).unwrap()
    }

    #[test]
    fn vscode_no_rules_is_no_op() {
        let tmp = tempfile::tempdir().unwrap();
        let action = ensure_vscode_instructions_registered(tmp.path(), false).expect("ensure");
        assert_eq!(action, AncillaryAction::Preserved);
        assert!(
            !tmp.path().join(VSCODE_SETTINGS).exists(),
            "no file created"
        );
    }

    #[test]
    fn vscode_creates_file_when_absent_and_has_rules() {
        let tmp = tempfile::tempdir().unwrap();
        let action = ensure_vscode_instructions_registered(tmp.path(), true).expect("ensure");
        assert_eq!(action, AncillaryAction::Created);
        let doc = read_vscode(tmp.path());
        assert_eq!(doc[VSCODE_INSTRUCTIONS_KEY][VSCODE_INSTRUCTIONS_PATH], true);
    }

    #[test]
    fn vscode_adds_key_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vscode")).unwrap();
        std::fs::write(
            tmp.path().join(VSCODE_SETTINGS),
            r#"{"editor.tabSize": 2, "files.eol": "\n"}"#,
        )
        .unwrap();

        let action = ensure_vscode_instructions_registered(tmp.path(), true).expect("ensure");
        assert_eq!(action, AncillaryAction::Updated);
        let doc = read_vscode(tmp.path());
        assert_eq!(doc[VSCODE_INSTRUCTIONS_KEY][VSCODE_INSTRUCTIONS_PATH], true);
        assert_eq!(doc["editor.tabSize"], 2, "other keys preserved");
        assert_eq!(doc["files.eol"], "\n");
    }

    #[test]
    fn vscode_appends_when_key_present_without_path() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vscode")).unwrap();
        std::fs::write(
            tmp.path().join(VSCODE_SETTINGS),
            r#"{"chat.instructionsFilesLocations": {".cursor/rules": true}}"#,
        )
        .unwrap();

        let action = ensure_vscode_instructions_registered(tmp.path(), true).expect("ensure");
        assert_eq!(action, AncillaryAction::Updated);
        let doc = read_vscode(tmp.path());
        let map = doc[VSCODE_INSTRUCTIONS_KEY].as_object().unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map[".cursor/rules"], true, "existing entry preserved");
        assert_eq!(map[VSCODE_INSTRUCTIONS_PATH], true);
    }

    #[test]
    fn vscode_preserves_when_path_already_true() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vscode")).unwrap();
        let original = r#"{"chat.instructionsFilesLocations": {".github/instructions": true}}"#;
        std::fs::write(tmp.path().join(VSCODE_SETTINGS), original).unwrap();

        let action = ensure_vscode_instructions_registered(tmp.path(), true).expect("ensure");
        assert_eq!(action, AncillaryAction::Preserved);
    }

    #[test]
    fn vscode_preserves_when_path_explicitly_false() {
        // User said "don't search here". We don't override the choice — only
        // insert if the path key is absent altogether.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vscode")).unwrap();
        std::fs::write(
            tmp.path().join(VSCODE_SETTINGS),
            r#"{"chat.instructionsFilesLocations": {".github/instructions": false}}"#,
        )
        .unwrap();

        let action = ensure_vscode_instructions_registered(tmp.path(), true).expect("ensure");
        assert_eq!(action, AncillaryAction::Preserved);
        let doc = read_vscode(tmp.path());
        assert_eq!(
            doc[VSCODE_INSTRUCTIONS_KEY][VSCODE_INSTRUCTIONS_PATH], false,
            "user-set false is preserved"
        );
    }

    #[test]
    fn vscode_errors_on_non_object_instructions_value() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vscode")).unwrap();
        std::fs::write(
            tmp.path().join(VSCODE_SETTINGS),
            r#"{"chat.instructionsFilesLocations": ["not", "an", "object"]}"#,
        )
        .unwrap();

        let err = ensure_vscode_instructions_registered(tmp.path(), true)
            .expect_err("must reject non-object value");
        assert!(err.to_string().contains(VSCODE_INSTRUCTIONS_KEY));
    }

    #[test]
    fn vscode_errors_on_non_object_top_level() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vscode")).unwrap();
        std::fs::write(tmp.path().join(VSCODE_SETTINGS), r#"["array", "top"]"#).unwrap();
        let err = ensure_vscode_instructions_registered(tmp.path(), true).expect_err("must reject");
        assert!(err.to_string().contains("object"));
    }

    #[test]
    fn write_opencode_plugin_uri_creates_file_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let outcome = write_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com");
        assert_eq!(outcome, crate::plugin::PluginOutcome::Success);
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap(),
        )
        .unwrap();
        let plugins = content["plugin"].as_array().unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0], "sp@git+https://example.com");
    }

    #[test]
    fn write_opencode_plugin_uri_appends_to_existing() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("opencode.json"),
            r#"{"instructions": [".agents/rules/**/RULE.md"], "plugin": ["existing@foo"]}"#,
        )
        .unwrap();
        let outcome = write_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com");
        assert_eq!(outcome, crate::plugin::PluginOutcome::Success);
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap(),
        )
        .unwrap();
        let plugins = content["plugin"].as_array().unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0], "existing@foo");
        assert_eq!(plugins[1], "sp@git+https://example.com");
        assert!(content["instructions"].is_array());
    }

    #[test]
    fn write_opencode_plugin_uri_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("opencode.json"),
            r#"{"plugin": ["sp@git+https://example.com"]}"#,
        )
        .unwrap();
        let outcome = write_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com");
        assert_eq!(outcome, crate::plugin::PluginOutcome::Success);
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap(),
        )
        .unwrap();
        let plugins = content["plugin"].as_array().unwrap();
        assert_eq!(plugins.len(), 1);
    }

    #[test]
    fn write_opencode_plugin_uri_fails_on_non_array_plugin_key() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("opencode.json"),
            r#"{"plugin": "not-an-array"}"#,
        )
        .unwrap();
        let outcome = write_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com");
        assert!(matches!(
            outcome,
            crate::plugin::PluginOutcome::Failed { .. }
        ));
    }

    #[test]
    fn remove_opencode_plugin_uri_removes_entry() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("opencode.json"),
            r#"{"plugin": ["sp@git+https://example.com", "other@foo"]}"#,
        )
        .unwrap();
        remove_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com").unwrap();
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap(),
        )
        .unwrap();
        let plugins = content["plugin"].as_array().unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0], "other@foo");
    }

    #[test]
    fn remove_opencode_plugin_uri_noop_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("opencode.json"),
            r#"{"plugin": ["other@foo"]}"#,
        )
        .unwrap();
        remove_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com").unwrap();
        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap(),
        )
        .unwrap();
        let plugins = content["plugin"].as_array().unwrap();
        assert_eq!(plugins.len(), 1);
    }

    #[test]
    fn remove_opencode_plugin_uri_noop_when_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        remove_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com").unwrap();
        assert!(!tmp.path().join("opencode.json").exists());
    }

    #[test]
    fn write_claude_mcp_json_adds_local_server() {
        use crate::model::bundle::McpLocal;
        use std::collections::BTreeMap;

        let tmp = tempfile::tempdir().unwrap();
        let mut env = BTreeMap::new();
        env.insert("TOK".to_string(), "${TOK}".to_string());
        let local = McpLocal {
            command: "npx".into(),
            args: vec!["-y".into(), "srv".into()],
            env,
        };

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

        let local = McpLocal {
            command: "npx".into(),
            args: vec![],
            env: BTreeMap::new(),
        };
        write_claude_mcp_local(tmp.path(), "drawio", &local);

        let raw = std::fs::read_to_string(tmp.path().join(".mcp.json")).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(doc["mcpServers"]["existing"]["command"], "foo");
        assert_eq!(doc["mcpServers"]["drawio"]["command"], "npx");
    }

    #[test]
    fn write_claude_mcp_json_adds_remote_server() {
        use crate::model::bundle::McpRemote;
        use std::collections::BTreeMap;

        let tmp = tempfile::tempdir().unwrap();
        let mut headers = BTreeMap::new();
        headers.insert("Authorization".to_string(), "Bearer ${TOKEN}".to_string());
        let remote = McpRemote {
            transport_type: "http".into(),
            url: "https://example.com/mcp".into(),
            headers,
        };

        let outcome = write_claude_mcp_remote(tmp.path(), "example", &remote);
        assert!(outcome.is_success());

        let raw = std::fs::read_to_string(tmp.path().join(".mcp.json")).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(doc["mcpServers"]["example"]["type"], "http");
        assert_eq!(
            doc["mcpServers"]["example"]["url"],
            "https://example.com/mcp"
        );
        assert_eq!(
            doc["mcpServers"]["example"]["headers"]["Authorization"],
            "Bearer ${TOKEN}"
        );
    }

    #[test]
    fn remove_claude_mcp_removes_server() {
        use crate::model::bundle::McpLocal;
        use std::collections::BTreeMap;

        let tmp = tempfile::tempdir().unwrap();
        let local = McpLocal {
            command: "npx".into(),
            args: vec![],
            env: BTreeMap::new(),
        };
        write_claude_mcp_local(tmp.path(), "drawio", &local);
        write_claude_mcp_local(tmp.path(), "other", &local);

        remove_claude_mcp(tmp.path(), "drawio").unwrap();

        let raw = std::fs::read_to_string(tmp.path().join(".mcp.json")).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(doc["mcpServers"]["drawio"].is_null());
        assert_eq!(doc["mcpServers"]["other"]["command"], "npx");
    }

    #[test]
    fn remove_claude_mcp_noop_when_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        remove_claude_mcp(tmp.path(), "drawio").unwrap();
        assert!(!tmp.path().join(".mcp.json").exists());
    }

    #[test]
    fn write_vscode_mcp_uses_servers_root_and_stdio_type() {
        use crate::model::bundle::McpLocal;
        use std::collections::BTreeMap;

        let tmp = tempfile::tempdir().unwrap();
        let mut env = BTreeMap::new();
        env.insert("TOK".to_string(), "${TOK}".to_string());
        let local = McpLocal {
            command: "npx".into(),
            args: vec!["-y".into(), "srv".into()],
            env,
        };

        assert!(write_vscode_mcp_local(tmp.path(), "drawio", &local).is_success());

        let raw = std::fs::read_to_string(tmp.path().join(".vscode/mcp.json")).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        // Root key MUST be `servers`, not `mcpServers` (AC for issue #237).
        assert!(doc.get("mcpServers").is_none());
        assert_eq!(doc["servers"]["drawio"]["type"], "stdio");
        assert_eq!(doc["servers"]["drawio"]["command"], "npx");
        assert_eq!(doc["servers"]["drawio"]["env"]["TOK"], "${TOK}");
    }

    #[test]
    fn write_vscode_mcp_remote_types_by_transport() {
        use crate::model::bundle::McpRemote;
        use std::collections::BTreeMap;

        let tmp = tempfile::tempdir().unwrap();
        let remote = McpRemote {
            transport_type: "sse".into(),
            url: "https://example.com/mcp".into(),
            headers: BTreeMap::new(),
        };
        assert!(write_vscode_mcp_remote(tmp.path(), "ex", &remote).is_success());

        let raw = std::fs::read_to_string(tmp.path().join(".vscode/mcp.json")).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(doc["servers"]["ex"]["type"], "sse");
        assert_eq!(doc["servers"]["ex"]["url"], "https://example.com/mcp");
    }

    #[test]
    fn write_opencode_mcp_uses_array_command_and_environment() {
        use crate::model::bundle::McpLocal;
        use std::collections::BTreeMap;

        let tmp = tempfile::tempdir().unwrap();
        let mut env = BTreeMap::new();
        env.insert("TOK".to_string(), "${TOK}".to_string());
        let local = McpLocal {
            command: "npx".into(),
            args: vec!["-y".into(), "srv".into()],
            env,
        };

        assert!(write_opencode_mcp_local(tmp.path(), "drawio", &local).is_success());

        let raw = std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(doc["mcp"]["drawio"]["type"], "local");
        // command is a single array combining command + args.
        assert_eq!(
            doc["mcp"]["drawio"]["command"],
            serde_json::json!(["npx", "-y", "srv"])
        );
        // env is named `environment`; values pass through verbatim.
        assert_eq!(doc["mcp"]["drawio"]["environment"]["TOK"], "${TOK}");
        assert!(doc["mcp"]["drawio"].get("env").is_none());
    }

    #[test]
    fn write_opencode_mcp_preserves_existing_keys() {
        use crate::model::bundle::McpRemote;
        use std::collections::BTreeMap;

        let tmp = tempfile::tempdir().unwrap();
        // opencode.json commonly already carries instructions / plugin keys.
        std::fs::write(
            tmp.path().join("opencode.json"),
            r#"{"instructions":[".agents/rules/**/RULE.md"],"plugin":["p@git"]}"#,
        )
        .unwrap();

        let remote = McpRemote {
            transport_type: "http".into(),
            url: "https://example.com/mcp".into(),
            headers: BTreeMap::new(),
        };
        assert!(write_opencode_mcp_remote(tmp.path(), "ex", &remote).is_success());

        let raw = std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(doc["instructions"][0], ".agents/rules/**/RULE.md");
        assert_eq!(doc["plugin"][0], "p@git");
        assert_eq!(doc["mcp"]["ex"]["type"], "remote");
        assert_eq!(doc["mcp"]["ex"]["url"], "https://example.com/mcp");
    }

    #[test]
    fn remove_vscode_and_opencode_mcp_drop_only_the_named_server() {
        use crate::model::bundle::McpLocal;
        use std::collections::BTreeMap;

        let tmp = tempfile::tempdir().unwrap();
        let local = McpLocal {
            command: "npx".into(),
            args: vec![],
            env: BTreeMap::new(),
        };
        write_vscode_mcp_local(tmp.path(), "a", &local);
        write_vscode_mcp_local(tmp.path(), "b", &local);
        write_opencode_mcp_local(tmp.path(), "a", &local);
        write_opencode_mcp_local(tmp.path(), "b", &local);

        remove_vscode_mcp(tmp.path(), "a").unwrap();
        remove_opencode_mcp(tmp.path(), "a").unwrap();

        let vscode: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join(".vscode/mcp.json")).unwrap(),
        )
        .unwrap();
        assert!(vscode["servers"]["a"].is_null());
        assert_eq!(vscode["servers"]["b"]["command"], "npx");

        let opencode: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap(),
        )
        .unwrap();
        assert!(opencode["mcp"]["a"].is_null());
        assert_eq!(opencode["mcp"]["b"]["type"], "local");
    }
}
