//! Integration tests for item conflict detection and resolution flags.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Set up a temp dir with a lockfile containing an item from source A,
/// plus a local source directory with the same-named skill.
fn setup_conflict(tmp: &std::path::Path) {
    // Existing lockfile with item from a different source
    let lockfile = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "skill",
            "name": "test-skill",
            "source": "github:org-a/repo-a",
            "hash": "sha256:aaa"
        }],
        "bundles": [],
        "plugins": []
    });
    fs::write(
        tmp.join(".upskill-lock.json"),
        serde_json::to_string_pretty(&lockfile).unwrap(),
    )
    .unwrap();

    // Local source with a skill that will conflict
    let skill_dir = tmp.join("source/test-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nschema: 1\nname: test-skill\ndescription: A test skill\n---\nContent here.\n",
    )
    .unwrap();

    // Init git repo so upskill uses project scope
    Command::new("git")
        .args(["init"])
        .current_dir(tmp)
        .assert()
        .success();
}

#[test]
fn add_from_different_source_errors_without_force() {
    let tmp = TempDir::new().unwrap();
    setup_conflict(tmp.path());

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "./source"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already installed from"));
}

#[test]
fn add_from_different_source_succeeds_with_force() {
    let tmp = TempDir::new().unwrap();
    setup_conflict(tmp.path());

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "./source", "--force"])
        .assert()
        .success();
}

#[test]
fn add_from_same_source_succeeds_without_force() {
    let tmp = TempDir::new().unwrap();

    // Lockfile with source matching what we'll install from (local:source)
    let lockfile = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "skill",
            "name": "test-skill",
            "source": "local:./source",
            "hash": "sha256:aaa"
        }],
        "bundles": [],
        "plugins": []
    });
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        serde_json::to_string_pretty(&lockfile).unwrap(),
    )
    .unwrap();

    let skill_dir = tmp.path().join("source/test-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nschema: 1\nname: test-skill\ndescription: A test skill\n---\nContent here.\n",
    )
    .unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "./source"])
        .assert()
        .success();
}

#[test]
fn add_with_exclude_skips_named_item() {
    let tmp = TempDir::new().unwrap();

    let skill_a = tmp.path().join("source/skill-a");
    let skill_b = tmp.path().join("source/skill-b");
    fs::create_dir_all(&skill_a).unwrap();
    fs::create_dir_all(&skill_b).unwrap();
    fs::write(
        skill_a.join("SKILL.md"),
        "---\nschema: 1\nname: skill-a\ndescription: Skill A\n---\nA\n",
    )
    .unwrap();
    fs::write(
        skill_b.join("SKILL.md"),
        "---\nschema: 1\nname: skill-b\ndescription: Skill B\n---\nB\n",
    )
    .unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "./source", "--exclude", "skill-b"])
        .assert()
        .success();

    let lock_content = fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(
        lock_content.contains("skill-a"),
        "expected skill-a: {lock_content}"
    );
    assert!(
        !lock_content.contains("skill-b"),
        "expected no skill-b: {lock_content}"
    );
}
