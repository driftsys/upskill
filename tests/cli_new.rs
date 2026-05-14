//! ATDD tests for `upskill new <kind> <name>`.
//!
//! Scaffolds a starter SSOT item under `<cwd>/<kind>s/<name>/<KIND>.md`
//! with the minimum frontmatter the format spec requires, plus
//! kind-specific defaults (e.g. `mode: subagent` / `model: sonnet` for
//! agents). Author command — refuses to run inside a consumer project
//! (`.upskill-lock.json`) per ADR-0004.

use assert_cmd::Command;
use std::fs;

#[test]
fn new_skill_creates_skill_md_with_minimum_frontmatter() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["new", "skill", "code-review"])
        .assert()
        .success();

    let path = tmp.path().join("code-review/SKILL.md");
    assert!(path.is_file(), "{path:?} should exist");
    let body = fs::read_to_string(&path).unwrap();
    assert!(body.starts_with("---\n"), "{body}");
    assert!(body.contains("\nschema: 1\n"), "{body}");
    assert!(body.contains("\nname: code-review\n"), "{body}");
    assert!(body.contains("description:"), "{body}");
}

#[test]
fn new_rule_creates_rule_md() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["new", "rule", "license-awareness"])
        .assert()
        .success();
    let path = tmp.path().join("license-awareness/RULE.md");
    assert!(path.is_file());
    let body = fs::read_to_string(&path).unwrap();
    assert!(body.contains("\nname: license-awareness\n"), "{body}");
}

#[test]
fn new_agent_emits_default_mode_and_model() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["new", "agent", "security-reviewer"])
        .assert()
        .success();
    let body = fs::read_to_string(tmp.path().join("security-reviewer/AGENT.md")).unwrap();
    assert!(body.contains("\nmode: subagent\n"), "{body}");
    assert!(body.contains("\nmodel: sonnet\n"), "{body}");
}

#[test]
fn new_refuses_existing_item_directory() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("dup")).unwrap();
    fs::write(tmp.path().join("dup/SKILL.md"), "old").unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["new", "skill", "dup"])
        .assert()
        .failure()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("already exists"), "{stderr}");
    // Existing file untouched.
    assert_eq!(
        fs::read_to_string(tmp.path().join("dup/SKILL.md")).unwrap(),
        "old"
    );
}

#[test]
fn new_rejects_invalid_name() {
    let tmp = tempfile::tempdir().unwrap();
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["new", "skill", "Invalid_Name"])
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("name") || stderr.contains("invalid"),
        "{stderr}"
    );
}

#[test]
fn new_refuses_to_run_inside_consumer_project() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        r#"{"schema":2,"items":[]}"#,
    )
    .unwrap();
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["new", "skill", "anything"])
        .assert()
        .failure()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("consumer project") || stderr.contains(".upskill-lock.json"),
        "{stderr}"
    );
}

#[test]
fn new_output_passes_lint_and_fmt() {
    let tmp = tempfile::tempdir().unwrap();
    for (kind, name) in [
        ("skill", "starter-skill"),
        ("rule", "starter-rule"),
        ("agent", "starter-agent"),
    ] {
        Command::cargo_bin("upskill")
            .unwrap()
            .current_dir(tmp.path())
            .args(["new", kind, name])
            .assert()
            .success();
    }

    // Lint must accept its own scaffolding.
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["lint", "--strict"])
        .assert()
        .success();

    // Fmt must report nothing changed (the scaffold is already canonical).
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["fmt"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("all formatted"),
        "scaffold not canonical: {out}"
    );
}
