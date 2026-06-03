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
        let env: serde_json::Map<String, Value> = local
            .env
            .iter()
            .map(|(k, v)| (k.clone(), json!(v)))
            .collect();
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
        let headers: serde_json::Map<String, Value> = remote
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), json!(v)))
            .collect();
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
        return PluginOutcome::Failed {
            exit_code: None,
            stderr: e.to_string(),
        };
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
    let mut doc: Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    if let Some(servers) = doc.get_mut("mcpServers").and_then(Value::as_object_mut) {
        servers.remove(name);
    }
    write_pretty_json(&path, &doc)
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
}
