//! ATDD tests for MCP server config-write install via `upskill add`.
//!
//! Drives the full install pipeline with `claude` absent from PATH so the
//! config-write fallback runs deterministically into the test's tempdir
//! (never the developer's real home or the worktree root).
//!
//! Covered scenario: a bundle declares a local (stdio) MCP server; with
//! `claude` absent, upskill falls back to writing `.mcp.json` and records
//! the server in `.upskill-lock.json`.

mod common;

use std::fs;
use std::path::Path;

/// Minimal SKILL.md for a skill named `drawio-diagrams`.
const DRAWIO_SKILL_MD: &str = "\
---
schema: 1
name: drawio-diagrams
description: Create diagrams with Draw.io
---

# Draw.io diagrams

Use diagrams to communicate architecture.
";

/// Bundle that declares `drawio-diagrams` and a local MCP server.
const DRAWIO_BUNDLE_YAML: &str = "\
schema: 1
name: drawio
description: Draw.io bundle with MCP server

items:
  skills:
    - drawio-diagrams

mcps:
  drawio:
    local:
      command: npx
      args:
        - \"-y\"
        - drawio-mcp-server
      env:
        DRAWIO_TOKEN: \"${DRAWIO_TOKEN}\"
    requires-env:
      - DRAWIO_TOKEN
";

/// Build a minimal SSOT registry in `root`:
/// - `drawio-diagrams/SKILL.md` — a single skill so `items.skills` resolves.
/// - `bundles/drawio.bundle.yaml` — declares the skill and an MCP server.
fn stage_mcp_registry(root: &Path) {
    let skill_dir = root.join("drawio-diagrams");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), DRAWIO_SKILL_MD).unwrap();

    let bundles_dir = root.join("bundles");
    fs::create_dir_all(&bundles_dir).unwrap();
    fs::write(bundles_dir.join("drawio.bundle.yaml"), DRAWIO_BUNDLE_YAML).unwrap();
}

/// Return a PATH value containing only an empty temp `bin/` directory so
/// that `claude` is not found (config-write fallback triggers) while
/// `upskill` itself — resolved by absolute path via `cargo_bin` — still
/// runs.
fn empty_bin_path(tmp: &Path) -> String {
    let bin = tmp.join("empty-bin");
    fs::create_dir_all(&bin).unwrap();
    bin.to_str().unwrap().to_string()
}

#[test]
fn add_bundle_with_mcp_writes_claude_mcp_json() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let registry = tmp.path().join("registry");
    let project = tmp.path().join("project");
    stage_mcp_registry(&registry);
    fs::create_dir_all(&project).unwrap();
    // Mark project as a git repo so upskill selects project scope.
    fs::create_dir_all(project.join(".git")).unwrap();

    let bundle_path = registry.join("bundles/drawio.bundle.yaml");
    let path_val = empty_bin_path(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &path_val)
        .args(["add", bundle_path.to_str().unwrap()])
        .assert()
        .success();

    // .mcp.json must exist in the project dir (config-write fallback ran).
    let mcp_json_path = project.join(".mcp.json");
    assert!(
        mcp_json_path.exists(),
        ".mcp.json must be written by the config-write fallback"
    );

    let raw = fs::read_to_string(&mcp_json_path).unwrap();

    // Server name is present.
    assert!(
        raw.contains("\"drawio\""),
        ".mcp.json must contain the server name 'drawio': {raw}"
    );

    // Command is present.
    assert!(
        raw.contains("npx"),
        ".mcp.json must contain the command 'npx': {raw}"
    );

    // Secret reference is preserved verbatim — upskill must NOT expand it.
    assert!(
        raw.contains("${DRAWIO_TOKEN}"),
        ".mcp.json must carry the literal `${{DRAWIO_TOKEN}}` reference verbatim \
         (secret custody stays with the user's environment): {raw}"
    );

    // Lockfile records the MCP entry.
    let lock_path = project.join(".upskill-lock.json");
    assert!(lock_path.exists(), ".upskill-lock.json must exist");
    let lock_raw = fs::read_to_string(&lock_path).unwrap();
    assert!(
        lock_raw.contains("\"mcps\""),
        "lockfile must have a 'mcps' array: {lock_raw}"
    );
    assert!(
        lock_raw.contains("\"drawio\""),
        "lockfile must record the 'drawio' MCP server: {lock_raw}"
    );
}

#[test]
fn add_bundle_with_mcp_is_deterministic_second_run() {
    // Running the same add twice must not accumulate duplicate entries
    // in .mcp.json or .upskill-lock.json (idempotency check).
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let registry = tmp.path().join("registry");
    let project = tmp.path().join("project");
    stage_mcp_registry(&registry);
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(project.join(".git")).unwrap();

    let bundle_path = registry.join("bundles/drawio.bundle.yaml");
    let path_val = empty_bin_path(tmp.path());

    for _ in 0..2 {
        common::upskill_cmd(&home)
            .current_dir(&project)
            .env("PATH", &path_val)
            .args(["add", bundle_path.to_str().unwrap()])
            .assert()
            .success();
    }

    let raw = fs::read_to_string(project.join(".mcp.json")).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let servers = doc["mcpServers"].as_object().expect("mcpServers object");
    assert_eq!(
        servers.len(),
        1,
        "second add must not duplicate the server entry; got {servers:?}"
    );

    let lock_raw = fs::read_to_string(project.join(".upskill-lock.json")).unwrap();
    let lock: serde_json::Value = serde_json::from_str(&lock_raw).unwrap();
    let mcps = lock["mcps"].as_array().expect("mcps array");
    assert_eq!(
        mcps.len(),
        1,
        "second add must not duplicate the lockfile mcp entry; got {mcps:?}"
    );
}
