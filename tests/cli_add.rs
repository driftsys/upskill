//! ATDD tests for `upskill add <source>`.
//!
//! Drives the v0.2 install pipeline end-to-end via the CLI: parses an
//! `InstallSource`, runs `pipeline::install_with_lockfile`, and writes
//! per-client output for rules / skills / agents (format-spec §7).
//!
//! Local-path source only — no network. Git-source coverage is
//! in `tests/pipeline_source.rs` at the library level.

mod common;

use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn stage_source(source: &Path) {
    let from = format!("{FIXTURES}/items");
    copy_dir_all(Path::new(&from), source).unwrap();
}

fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let to_path = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &to_path)?;
        } else {
            fs::copy(entry.path(), &to_path)?;
        }
    }
    Ok(())
}

#[test]
fn add_installs_local_ssot_to_cwd() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    // Mark `target` as a project directory so auto-fallback picks
    // project scope (cwd) instead of global ($HOME).
    fs::create_dir_all(target.join(".git")).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    // Report header includes the item count (12 outputs from
    // 1 skill + 2 rules + 1 agent × 3 clients).
    assert!(out.contains("Installed 12"), "stdout missing report: {out}");

    // Spot-check one output per client at the expected per-client paths.
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
    assert!(
        target
            .join(".github/instructions/api-conventions.instructions.md")
            .exists()
    );
    assert!(
        target
            .join(".opencode/agents/security-reviewer.md")
            .exists()
    );
}

#[test]
fn add_invalid_source_returns_usage_error() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "not a valid source"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn add_with_items_subset_installs_only_named_items() {
    // Spec §2.1: `upskill add <source> [items...]` filters the install
    // to a subset. Pick the skill (`create-api-endpoint`) and one rule
    // (`license-awareness`); leave the agent and the second rule out.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "create-api-endpoint",
            "license-awareness",
        ])
        .assert()
        .success();

    // Selected items are present.
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
    assert!(target.join(".claude/rules/license-awareness.md").exists());
    // Non-selected items are absent (skipped during the filtered walk).
    assert!(
        !target.join(".claude/agents/security-reviewer.md").exists(),
        "agent should not have been installed when not in the subset"
    );
    assert!(
        !target.join(".claude/rules/api-conventions.md").exists(),
        "second rule should not have been installed when not in the subset"
    );
}

#[test]
fn add_with_items_no_match_is_general_error() {
    // No name in the list matches anything in the source — the install
    // should refuse rather than silently emitting nothing.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "nope-not-real",
            "also-fictional",
        ])
        .assert()
        .failure()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("nope-not-real") || stderr.contains("no matching"),
        "stderr should name the missing items or say no match: {stderr}"
    );
}
