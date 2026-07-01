//! ATDD tests for consumer-side client filtering (issue #238, ADR-0012).
//!
//! A consumer restricts which clients an install targets, either
//! per-invocation (`--claude` / `--copilot` / `--vscode` / `--opencode`) or
//! persistently via `clients:` in config. No selection preserves the default
//! (emit for all clients). Precedence: flag > project config > global config
//! > default (all).

mod common;

use predicates::prelude::*;
use std::fs;
use std::path::Path;

/// Stage a one-skill SSOT registry at `root` so `upskill add <root>` installs
/// a single skill named `skill`.
fn stage_skill_registry(root: &Path, skill: &str) {
    let dir = root.join(skill);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!(
            "---\nschema: 1\nname: {skill}\n\
             description: Test skill exercising consumer-side client filtering.\n---\n\n\
             # {skill}\n\nBody.\n"
        ),
    )
    .unwrap();
}

/// A project dir marked as a git repo so upskill selects project scope.
fn stage_project(root: &Path) -> std::path::PathBuf {
    let project = root.join("project");
    fs::create_dir_all(project.join(".git")).unwrap();
    project
}

fn skill_claude(project: &Path, skill: &str) -> std::path::PathBuf {
    project.join(format!(".claude/skills/{skill}/SKILL.md"))
}
fn skill_copilot(project: &Path, skill: &str) -> std::path::PathBuf {
    project.join(format!(".github/skills/{skill}/SKILL.md"))
}
fn skill_opencode(project: &Path, skill: &str) -> std::path::PathBuf {
    project.join(format!(".agents/skills/{skill}/SKILL.md"))
}

/// AC: with no selection, `add` still emits for all clients (default
/// unchanged) — regression guard.
#[test]
fn default_add_writes_all_three_client_trees() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    stage_skill_registry(&registry, "code-review");
    let project = stage_project(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap()])
        .assert()
        .success();

    assert!(skill_claude(&project, "code-review").exists());
    assert!(skill_copilot(&project, "code-review").exists());
    assert!(skill_opencode(&project, "code-review").exists());
}

/// AC: `add --claude` writes only `.claude/**` and no `.github/**` /
/// `.agents/**`.
#[test]
fn claude_flag_writes_only_claude_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    stage_skill_registry(&registry, "code-review");
    let project = stage_project(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap(), "--claude"])
        .assert()
        .success();

    assert!(skill_claude(&project, "code-review").exists());
    assert!(!skill_copilot(&project, "code-review").exists());
    assert!(!skill_opencode(&project, "code-review").exists());
}

/// AC: `--copilot --vscode` writes the shared `.github/**` generation once
/// (VS Code reads Copilot's tree) and nothing under `.claude/**` /
/// `.agents/**`.
#[test]
fn copilot_and_vscode_share_github_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    stage_skill_registry(&registry, "code-review");
    let project = stage_project(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap(), "--copilot", "--vscode"])
        .assert()
        .success();

    assert!(skill_copilot(&project, "code-review").exists());
    assert!(!skill_claude(&project, "code-review").exists());
    assert!(!skill_opencode(&project, "code-review").exists());
}

/// AC: a persistent project `clients:` config restricts an `add` without
/// flags.
#[test]
fn project_config_clients_restricts_without_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    stage_skill_registry(&registry, "code-review");
    let project = stage_project(tmp.path());
    fs::create_dir_all(project.join(".upskill")).unwrap();
    fs::write(project.join(".upskill/config.yaml"), "clients: [claude]\n").unwrap();

    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap()])
        .assert()
        .success();

    assert!(skill_claude(&project, "code-review").exists());
    assert!(!skill_copilot(&project, "code-review").exists());
    assert!(!skill_opencode(&project, "code-review").exists());
}

/// AC: a per-invocation flag overrides the persistent config selection.
#[test]
fn invocation_flag_overrides_config() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    stage_skill_registry(&registry, "code-review");
    let project = stage_project(tmp.path());
    fs::create_dir_all(project.join(".upskill")).unwrap();
    fs::write(project.join(".upskill/config.yaml"), "clients: [claude]\n").unwrap();

    // Config says claude-only, but the flag selects opencode only.
    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap(), "--opencode"])
        .assert()
        .success();

    assert!(skill_opencode(&project, "code-review").exists());
    assert!(!skill_claude(&project, "code-review").exists());
    assert!(!skill_copilot(&project, "code-review").exists());
}

/// AC: precedence is project > global — a project `clients:` wins over a
/// broader global `clients:`.
#[test]
fn project_config_overrides_global_config() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(home.join(".config/upskill")).unwrap();
    // Global selects all three; project narrows to claude.
    fs::write(
        home.join(".config/upskill/config.yaml"),
        "clients: [claude, copilot, opencode]\n",
    )
    .unwrap();
    let registry = tmp.path().join("registry");
    stage_skill_registry(&registry, "code-review");
    let project = stage_project(tmp.path());
    fs::create_dir_all(project.join(".upskill")).unwrap();
    fs::write(project.join(".upskill/config.yaml"), "clients: [claude]\n").unwrap();

    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap()])
        .assert()
        .success();

    assert!(skill_claude(&project, "code-review").exists());
    assert!(!skill_copilot(&project, "code-review").exists());
    assert!(!skill_opencode(&project, "code-review").exists());
}

/// AC: `doctor` does not report unselected-client outputs as missing after a
/// narrowed install.
#[test]
fn doctor_clean_after_narrowed_install() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    stage_skill_registry(&registry, "code-review");
    let project = stage_project(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap(), "--claude"])
        .assert()
        .success();

    // doctor must exit 0 — the unwritten .github/** and .agents/** outputs
    // are a deliberate selection, not drift.
    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["doctor"])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// MCP fan-out respects the selection (ADR-0010 targets ∩ ADR-0012 selection)
// ---------------------------------------------------------------------------

const DRAWIO_SKILL_MD: &str = "\
---
schema: 1
name: drawio-diagrams
description: Create diagrams with Draw.io
---

# Draw.io diagrams

Use diagrams to communicate architecture.
";

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
";

/// AC: an item whose selection excludes a client is not MCP-configured for
/// it — `add --claude` records exactly one MCP entry (claude), not four.
#[test]
fn mcp_fanout_respects_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let registry = tmp.path().join("registry");
    let skill_dir = registry.join("drawio-diagrams");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), DRAWIO_SKILL_MD).unwrap();
    let bundles_dir = registry.join("bundles");
    fs::create_dir_all(&bundles_dir).unwrap();
    fs::write(bundles_dir.join("drawio.bundle.yaml"), DRAWIO_BUNDLE_YAML).unwrap();

    let project = stage_project(tmp.path());
    // Force the config-write fallback: no client CLI on PATH.
    let empty_bin = tmp.path().join("empty-bin");
    fs::create_dir_all(&empty_bin).unwrap();

    let bundle_path = bundles_dir.join("drawio.bundle.yaml");
    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", empty_bin.to_str().unwrap())
        .args(["add", bundle_path.to_str().unwrap(), "--claude"])
        .assert()
        .success();

    let lock_raw = fs::read_to_string(project.join(".upskill-lock.json")).unwrap();
    let lock: serde_json::Value = serde_json::from_str(&lock_raw).unwrap();
    let mcps = lock["mcps"].as_array().expect("mcps array");
    assert_eq!(
        mcps.len(),
        1,
        "--claude must configure only the claude MCP target; got {mcps:?}"
    );
    assert_eq!(mcps[0]["client"], "claude");
}

/// Finding 1 regression: a bare `update` (no flags, no config) must NOT
/// re-expand a flag-narrowed install, nor overwrite the recorded `clients`.
#[test]
fn update_preserves_narrowed_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    stage_skill_registry(&registry, "code-review");
    let project = stage_project(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap(), "--claude"])
        .assert()
        .success();
    assert!(skill_claude(&project, "code-review").exists());
    assert!(!skill_copilot(&project, "code-review").exists());

    // Bare update — must stay claude-only.
    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["update", "--yes"])
        .assert()
        .success();

    assert!(skill_claude(&project, "code-review").exists());
    assert!(
        !skill_copilot(&project, "code-review").exists(),
        "bare update must not re-expand a --claude install to Copilot"
    );
    assert!(!skill_opencode(&project, "code-review").exists());

    // The lockfile record must still be claude-only.
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(project.join(".upskill-lock.json")).unwrap())
            .unwrap();
    let item = &lock["items"][0];
    assert_eq!(
        item["clients"],
        serde_json::json!(["claude"]),
        "update must preserve the recorded clients selection; got {}",
        item["clients"]
    );
}

/// Finding 2 regression: a `--global` install must ignore the current
/// directory's project `clients:` config (it belongs to an unrelated project).
#[test]
fn global_install_ignores_project_config() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    stage_skill_registry(&registry, "code-review");
    // cwd is a project whose config narrows to claude...
    let project = stage_project(tmp.path());
    fs::create_dir_all(project.join(".upskill")).unwrap();
    fs::write(project.join(".upskill/config.yaml"), "clients: [claude]\n").unwrap();

    // ...but --global targets $HOME and must not read that project config.
    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap(), "--global"])
        .assert()
        .success();

    assert!(home.join(".claude/skills/code-review/SKILL.md").exists());
    assert!(
        home.join(".github/skills/code-review/SKILL.md").exists(),
        "global install must emit for all clients, ignoring the project's clients: config"
    );
    assert!(home.join(".agents/skills/code-review/SKILL.md").exists());
}

/// Finding 3 regression: an item whose `audience:` shares no client with a
/// restrictive selection is warn-skipped (stderr warning, exit 0), not
/// silently dropped.
#[test]
fn empty_audience_selection_warns_and_skips() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    let dir = registry.join("claude-rule");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nschema: 1\nname: claude-rule\n\
         description: A Claude-only skill for testing audience-selection warn-skip.\n\
         audience:\n  - claude\n---\n\n# claude-rule\n\nBody.\n",
    )
    .unwrap();
    let project = stage_project(tmp.path());

    // Select opencode only — disjoint from the item's claude-only audience.
    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap(), "--opencode"])
        .assert()
        .success()
        .stderr(predicates::str::contains("warning:").and(predicates::str::contains("skipped")));

    assert!(!skill_claude(&project, "claude-rule").exists());
    assert!(!skill_opencode(&project, "claude-rule").exists());
}
