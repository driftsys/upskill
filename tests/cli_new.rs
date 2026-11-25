//! ATDD tests for `upskill new <kind> <name>`.
//!
//! Scaffolds a starter SSOT item under `<cwd>/<kind>s/<name>/<KIND>.md`
//! with the minimum frontmatter the format spec requires, plus
//! kind-specific defaults (e.g. `mode: subagent` / `model: sonnet` for
//! agents). Author command — refuses to run inside a consumer project
//! (`.upskill-lock.json`) per ADR-0004.

mod common;

use std::fs;

#[test]
fn new_skill_creates_skill_md_with_minimum_frontmatter() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    common::upskill_cmd(&home)
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
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    common::upskill_cmd(&home)
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
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    common::upskill_cmd(&home)
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
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join("dup")).unwrap();
    fs::write(tmp.path().join("dup/SKILL.md"), "old").unwrap();

    let assert = common::upskill_cmd(&home)
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
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let assert = common::upskill_cmd(&home)
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
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        r#"{"schema":2,"items":[]}"#,
    )
    .unwrap();
    let assert = common::upskill_cmd(&home)
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
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    for (kind, name) in [
        ("skill", "starter-skill"),
        ("rule", "starter-rule"),
        ("agent", "starter-agent"),
    ] {
        common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(["new", kind, name])
            .assert()
            .success();
    }

    // Lint must accept its own scaffolding.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict"])
        .assert()
        .success();

    // Fmt must report nothing changed (the scaffold is already canonical).
    let assert = common::upskill_cmd(&home)
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

#[test]
fn new_command_creates_command_md() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["new", "command", "release-notes"])
        .assert()
        .success();
    let path = tmp.path().join("release-notes/COMMAND.md");
    let before = fs::read_to_string(&path).unwrap();
    assert!(before.contains("\nschema: 1\n"), "{before}");
    assert!(before.contains("\nname: release-notes\n"), "{before}");
    assert!(before.contains("description:"), "{before}");
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict", "release-notes"])
        .assert()
        .success();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["fmt", "release-notes"])
        .assert()
        .success();
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn new_command_colocates_with_a_rule() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    for kind in ["rule", "command"] {
        common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(["new", kind, "x"])
            .assert()
            .success();
    }
    assert!(tmp.path().join("x/RULE.md").is_file());
    assert!(tmp.path().join("x/COMMAND.md").is_file());
}

#[test]
fn new_command_refuses_a_directory_with_a_skill() {
    assert_skill_command_colocation_refused("skill", "command", "COMMAND.md");
}

#[test]
fn new_skill_refuses_a_directory_with_a_command() {
    assert_skill_command_colocation_refused("command", "skill", "SKILL.md");
}

fn assert_skill_command_colocation_refused(first: &str, second: &str, refused: &str) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["new", first, "x"])
        .assert()
        .success();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["new", second, "x"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "a COMMAND.md and a SKILL.md cannot share one item directory",
        ));
    assert!(!tmp.path().join("x").join(refused).exists());
}

#[test]
fn new_refuses_a_directory_whose_command_declares_another_name() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join("foo")).unwrap();
    fs::write(
        tmp.path().join("foo/COMMAND.md"),
        "---\nschema: 1\nname: bar\ndescription: Write the notes\n---\n\nWrite the notes.\n",
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["new", "rule", "foo"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "existing command entrypoint declares `name: bar` \u{2014} a command must have the name of its directory `foo`",
        ));
    assert!(!tmp.path().join("foo/RULE.md").exists());
}

#[test]
fn new_rejects_an_unknown_kind_and_lists_five_kinds() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["new", "bundle", "x"])
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "expected one of: rule, skill, agent, command, hook",
        ));
}

#[test]
fn new_hook_creates_hook_md() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["new", "hook", "team-reminder"])
        .assert()
        .success();
    let path = tmp.path().join("team-reminder/HOOK.md");
    let before = fs::read_to_string(&path).unwrap();
    for text in [
        "\nschema: 1\n",
        "\nname: team-reminder\n",
        "\nevents:\n  session-start:\n",
        "\n## session-start\n",
    ] {
        assert!(before.contains(text), "{before}");
    }
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict", "team-reminder"])
        .assert()
        .success();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["fmt", "team-reminder"])
        .assert()
        .success();
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn new_hook_colocates_with_a_skill() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    for kind in ["skill", "hook"] {
        common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(["new", kind, "x"])
            .assert()
            .success();
    }
    assert!(tmp.path().join("x/SKILL.md").is_file());
    assert!(tmp.path().join("x/HOOK.md").is_file());
}

#[test]
fn new_hook_refuses_the_reserved_name_context() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["new", "hook", "context"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("hook name `context` is reserved"));
    assert!(!tmp.path().join("context").exists());
}
