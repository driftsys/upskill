//! ATDD tests for the MCP server install + remove lifecycle across all four
//! MCP targets (Claude, Copilot, VS Code, opencode — ADR-0010, issue #237).
//!
//! `claude`, `copilot`, and `code` are kept off PATH throughout by setting
//! PATH to an empty directory, so each target's config-write fallback runs
//! deterministically into the test's tempdir (and the test HOME for Copilot's
//! user-scope file). opencode is config-only and has no CLI step.

mod common;

use std::fs;
use std::path::Path;

/// Minimal SKILL.md for a skill named `s`.
const SKILL_MD: &str = "\
---
schema: 1
name: s
description: A minimal skill
---

# Skill

Minimal body.
";

/// Bundle with a remote HTTP MCP server.
const BUNDLE_REMOTE_YAML: &str = "\
schema: 1
name: b
description: Bundle with remote MCP

items:
  skills:
    - s

mcps:
  remote-srv:
    remote:
      type: http
      url: https://example.com/mcp
";

/// Bundle with a local (stdio) MCP server that references a secret via
/// `${TOKEN}` indirection — upskill must write it verbatim, never expand it.
const BUNDLE_LOCAL_YAML: &str = "\
schema: 1
name: b
description: Bundle with local MCP

items:
  skills:
    - s

mcps:
  local-srv:
    local:
      command: npx
      args: [\"-y\", \"my-tool-mcp\"]
      env:
        TOKEN: \"${TOKEN}\"
";

/// Build a minimal SSOT registry with a single skill and a bundle whose
/// `.bundle.yaml` contents are `bundle_yaml`.
fn stage_registry(root: &Path, bundle_yaml: &str) {
    let skill_dir = root.join("s");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), SKILL_MD).unwrap();

    let bundles_dir = root.join("bundles");
    fs::create_dir_all(&bundles_dir).unwrap();
    fs::write(bundles_dir.join("b.bundle.yaml"), bundle_yaml).unwrap();
}

/// PATH that contains only an empty directory — every client CLI is absent,
/// so config-write fallbacks run. Local-path `add` needs no external binary.
fn empty_bin_path(tmp: &Path) -> String {
    let bin = tmp.join("empty-bin");
    fs::create_dir_all(&bin).unwrap();
    bin.to_str().unwrap().to_string()
}

fn read_json(path: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap()
}

#[test]
fn add_then_remove_mcp_updates_lockfile() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let registry = tmp.path().join("registry");
    let project = tmp.path().join("project");
    stage_registry(&registry, BUNDLE_REMOTE_YAML);
    fs::create_dir_all(&project).unwrap();
    // Mark as a git repo so project scope is selected.
    fs::create_dir_all(project.join(".git")).unwrap();

    let bundle_path = registry.join("bundles/b.bundle.yaml");
    let path_val = empty_bin_path(tmp.path());

    // Step 1 — add the bundle; the lockfile must contain the MCP entry.
    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &path_val)
        .args(["add", bundle_path.to_str().unwrap()])
        .assert()
        .success();

    let lock_raw = fs::read_to_string(project.join(".upskill-lock.json")).unwrap();
    assert!(
        lock_raw.contains("\"remote-srv\""),
        "lockfile must record 'remote-srv' after add: {lock_raw}"
    );
    assert!(
        lock_raw.contains("\"mcps\""),
        "lockfile must have a 'mcps' key after add: {lock_raw}"
    );

    // Step 2 — remove the MCP server; the lockfile must no longer contain it.
    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &path_val)
        .args(["remove-mcp", "remote-srv"])
        .assert()
        .success();

    let lock_raw_after = fs::read_to_string(project.join(".upskill-lock.json")).unwrap();
    assert!(
        !lock_raw_after.contains("\"remote-srv\""),
        "lockfile must NOT contain 'remote-srv' after remove-mcp: {lock_raw_after}"
    );
}

#[test]
fn add_remote_mcp_configures_every_target_with_correct_root_key() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let registry = tmp.path().join("registry");
    let project = tmp.path().join("project");
    stage_registry(&registry, BUNDLE_REMOTE_YAML);
    fs::create_dir_all(project.join(".git")).unwrap();

    let bundle_path = registry.join("bundles/b.bundle.yaml");
    let path_val = empty_bin_path(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &path_val)
        .args(["add", bundle_path.to_str().unwrap()])
        .assert()
        .success();

    // Claude — `.mcp.json`, root key `mcpServers`.
    let claude = read_json(&project.join(".mcp.json"));
    assert_eq!(claude["mcpServers"]["remote-srv"]["type"], "http");
    assert_eq!(
        claude["mcpServers"]["remote-srv"]["url"],
        "https://example.com/mcp"
    );

    // VS Code — `.vscode/mcp.json`, root key `servers` (NOT mcpServers).
    let vscode = read_json(&project.join(".vscode/mcp.json"));
    assert!(
        vscode.get("mcpServers").is_none(),
        "VS Code config must use `servers`, not `mcpServers`: {vscode}"
    );
    assert_eq!(vscode["servers"]["remote-srv"]["type"], "http");
    assert_eq!(
        vscode["servers"]["remote-srv"]["url"],
        "https://example.com/mcp"
    );

    // opencode — `opencode.json`, key `mcp.<name>`, transport `remote`.
    let opencode = read_json(&project.join("opencode.json"));
    assert_eq!(opencode["mcp"]["remote-srv"]["type"], "remote");
    assert_eq!(
        opencode["mcp"]["remote-srv"]["url"],
        "https://example.com/mcp"
    );

    // Copilot — user-scope `~/.copilot/mcp-config.json`, root key `mcpServers`.
    let copilot = read_json(&home.join(".copilot/mcp-config.json"));
    assert_eq!(
        copilot["mcpServers"]["remote-srv"]["url"],
        "https://example.com/mcp"
    );
}

#[test]
fn add_local_mcp_uses_per_target_shapes_and_keeps_secret_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let registry = tmp.path().join("registry");
    let project = tmp.path().join("project");
    stage_registry(&registry, BUNDLE_LOCAL_YAML);
    fs::create_dir_all(project.join(".git")).unwrap();

    let bundle_path = registry.join("bundles/b.bundle.yaml");
    let path_val = empty_bin_path(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &path_val)
        // Ensure TOKEN is unset so any accidental expansion would be visible.
        .env_remove("TOKEN")
        .args(["add", bundle_path.to_str().unwrap()])
        .assert()
        .success();

    // opencode — command folded into one array, env named `environment`.
    let opencode = read_json(&project.join("opencode.json"));
    assert_eq!(opencode["mcp"]["local-srv"]["type"], "local");
    assert_eq!(
        opencode["mcp"]["local-srv"]["command"],
        serde_json::json!(["npx", "-y", "my-tool-mcp"])
    );
    assert_eq!(
        opencode["mcp"]["local-srv"]["environment"]["TOKEN"],
        "${TOKEN}"
    );
    assert!(opencode["mcp"]["local-srv"].get("env").is_none());

    // VS Code — `type: stdio`, separate command + args + env.
    let vscode = read_json(&project.join(".vscode/mcp.json"));
    assert_eq!(vscode["servers"]["local-srv"]["type"], "stdio");
    assert_eq!(vscode["servers"]["local-srv"]["command"], "npx");
    assert_eq!(vscode["servers"]["local-srv"]["env"]["TOKEN"], "${TOKEN}");

    // Claude / Copilot — `mcpServers` shape with `env`.
    let claude = read_json(&project.join(".mcp.json"));
    assert_eq!(claude["mcpServers"]["local-srv"]["command"], "npx");
    assert_eq!(
        claude["mcpServers"]["local-srv"]["env"]["TOKEN"],
        "${TOKEN}"
    );
    let copilot = read_json(&home.join(".copilot/mcp-config.json"));
    assert_eq!(
        copilot["mcpServers"]["local-srv"]["env"]["TOKEN"],
        "${TOKEN}"
    );

    // No literal secret leaks into any config file — `${TOKEN}` is verbatim.
    for path in [
        project.join(".mcp.json"),
        project.join(".vscode/mcp.json"),
        project.join("opencode.json"),
        home.join(".copilot/mcp-config.json"),
    ] {
        let raw = fs::read_to_string(&path).unwrap();
        assert!(
            raw.contains("${TOKEN}"),
            "{} must keep the ${{TOKEN}} reference verbatim",
            path.display()
        );
    }
}

/// The `status` string `doctor --json` reports for `name` under `client`.
fn doctor_mcp_status(report: &serde_json::Value, client: &str, name: &str) -> String {
    report["mcp_entries"]
        .as_array()
        .expect("mcp_entries array")
        .iter()
        .find(|e| e["client"] == client && e["name"] == name)
        .unwrap_or_else(|| panic!("no mcp_entry for {client}/{name}: {report}"))["status"]
        .as_str()
        .expect("status string")
        .to_string()
}

fn run_doctor_json(home: &Path, project: &Path, path_val: &str) -> serde_json::Value {
    let assert = common::upskill_cmd(home)
        .current_dir(project)
        .env("PATH", path_val)
        .args(["doctor", "--json"])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("doctor --json not JSON: {e}\n{stdout}"))
}

#[test]
fn doctor_reconciles_config_targets_and_flags_drift_without_false_positives() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let registry = tmp.path().join("registry");
    let project = tmp.path().join("project");
    stage_registry(&registry, BUNDLE_REMOTE_YAML);
    fs::create_dir_all(project.join(".git")).unwrap();

    let bundle_path = registry.join("bundles/b.bundle.yaml");
    let path_val = empty_bin_path(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &path_val)
        .args(["add", bundle_path.to_str().unwrap()])
        .assert()
        .success();

    // Right after add, every config-write target carries the server (`ok`),
    // and claude — its CLI absent (PATH cleared) — is `cli-not-found`, NOT a
    // false `not-registered`.
    let report = run_doctor_json(&home, &project, &path_val);
    assert_eq!(doctor_mcp_status(&report, "vscode", "remote-srv"), "ok");
    assert_eq!(doctor_mcp_status(&report, "opencode", "remote-srv"), "ok");
    assert_eq!(doctor_mcp_status(&report, "copilot", "remote-srv"), "ok");
    assert_eq!(
        doctor_mcp_status(&report, "claude", "remote-srv"),
        "cli-not-found"
    );

    // Drop the server from VS Code's config only — doctor flags exactly that
    // target as drifted, leaving the others untouched.
    fs::write(project.join(".vscode/mcp.json"), "{\"servers\":{}}\n").unwrap();
    let report = run_doctor_json(&home, &project, &path_val);
    assert_eq!(
        doctor_mcp_status(&report, "vscode", "remote-srv"),
        "not-registered"
    );
    assert_eq!(doctor_mcp_status(&report, "opencode", "remote-srv"), "ok");

    // The human-readable message must name the drifted target (vscode), not a
    // hardcoded "claude".
    let assert = common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &path_val)
        .args(["doctor"])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(
        stdout.contains("not registered in vscode"),
        "doctor must attribute the drift to vscode, not claude: {stdout}"
    );
}

#[test]
fn remove_mcp_clears_every_target_config_file() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let registry = tmp.path().join("registry");
    let project = tmp.path().join("project");
    stage_registry(&registry, BUNDLE_REMOTE_YAML);
    fs::create_dir_all(project.join(".git")).unwrap();

    let bundle_path = registry.join("bundles/b.bundle.yaml");
    let path_val = empty_bin_path(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &path_val)
        .args(["add", bundle_path.to_str().unwrap()])
        .assert()
        .success();

    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &path_val)
        .args(["remove-mcp", "remote-srv"])
        .assert()
        .success();

    // Each target's config file must no longer carry the server.
    let claude = read_json(&project.join(".mcp.json"));
    assert!(claude["mcpServers"]["remote-srv"].is_null());
    let vscode = read_json(&project.join(".vscode/mcp.json"));
    assert!(vscode["servers"]["remote-srv"].is_null());
    let opencode = read_json(&project.join("opencode.json"));
    assert!(opencode["mcp"]["remote-srv"].is_null());
    let copilot = read_json(&home.join(".copilot/mcp-config.json"));
    assert!(copilot["mcpServers"]["remote-srv"].is_null());
}
