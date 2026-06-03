//! ATDD tests for the MCP server install + remove lifecycle.
//!
//! Exercises `upskill add` → lockfile records MCP server, then
//! `upskill remove-mcp <name>` → lockfile drops MCP server.
//!
//! `claude` is kept off PATH throughout by setting PATH to an empty
//! directory, so the config-write fallback runs deterministically into the
//! test's tempdir.

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
const BUNDLE_YAML: &str = "\
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

/// Build a minimal SSOT registry with a single skill and a bundle that
/// declares a remote MCP server.
fn stage_remote_mcp_registry(root: &Path) {
    let skill_dir = root.join("s");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), SKILL_MD).unwrap();

    let bundles_dir = root.join("bundles");
    fs::create_dir_all(&bundles_dir).unwrap();
    fs::write(bundles_dir.join("b.bundle.yaml"), BUNDLE_YAML).unwrap();
}

/// PATH that contains only an empty directory — `claude` absent, local-path
/// add still works (no external binary needed for local sources).
fn empty_bin_path(tmp: &Path) -> String {
    let bin = tmp.join("empty-bin");
    fs::create_dir_all(&bin).unwrap();
    bin.to_str().unwrap().to_string()
}

#[test]
fn add_then_remove_mcp_updates_lockfile() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let registry = tmp.path().join("registry");
    let project = tmp.path().join("project");
    stage_remote_mcp_registry(&registry);
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
