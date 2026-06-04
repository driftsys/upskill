//! Integration tests for item conflict detection and resolution flags.

mod common;

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

    // Fake git repo so upskill uses project scope
    fs::create_dir_all(tmp.join(".git")).unwrap();
}

#[test]
fn add_from_different_source_errors_without_force() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    setup_conflict(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already installed from"));
}

#[test]
fn add_from_different_source_succeeds_with_force() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    setup_conflict(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "--force"])
        .assert()
        .success();
}

#[test]
fn add_from_same_source_succeeds_without_force() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let skill_dir = tmp.path().join("source/test-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nschema: 1\nname: test-skill\ndescription: A test skill\n---\nContent here.\n",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    // First install records the canonical (absolutized) `local:` source label.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source"])
        .assert()
        .success();

    // Re-adding from the SAME source must succeed without --force: the
    // recomputed source label matches the recorded one (issue #212 ensures
    // both spellings canonicalize identically).
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source"])
        .assert()
        .success();
}

#[test]
fn add_with_exclude_skips_named_item() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

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

    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    common::upskill_cmd(&home)
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

#[test]
fn add_with_alias_installs_under_alternate_name() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    // Existing lockfile with item from different source (creates conflict)
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
        tmp.path().join(".upskill-lock.json"),
        serde_json::to_string_pretty(&lockfile).unwrap(),
    )
    .unwrap();

    // Local source with the conflicting skill
    let skill_dir = tmp.path().join("source/test-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nschema: 1\nname: test-skill\ndescription: A test skill\n---\nContent here.\n",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    // Install with alias to avoid conflict
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "--as", "test-skill-v2"])
        .assert()
        .success();

    // Verify lockfile has the alias with source_name
    let lock_content = fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(
        lock_content.contains("test-skill-v2"),
        "expected alias in lockfile: {lock_content}"
    );
    assert!(
        lock_content.contains("source_name") && lock_content.contains("test-skill"),
        "expected source_name tracking: {lock_content}"
    );
}

#[test]
fn update_handles_aliased_items() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    // Create source with a skill
    let skill_dir = tmp.path().join("source/original-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nschema: 1\nname: original-skill\ndescription: Original skill\n---\nUpdated content v2.\n",
    )
    .unwrap();

    // Lockfile with an aliased item (as if previously installed with --as)
    let lockfile = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "skill",
            "name": "my-alias",
            "source": "local:./source",
            "source_name": "original-skill",
            "hash": "sha256:oldoldold"
        }],
        "bundles": [],
        "plugins": []
    });
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        serde_json::to_string_pretty(&lockfile).unwrap(),
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    // Update should succeed (finds original-skill in source, installs as my-alias)
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update"])
        .assert()
        .success();

    // Verify the alias is preserved in lockfile
    let lock_content = fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(
        lock_content.contains("my-alias"),
        "alias preserved: {lock_content}"
    );
    assert!(
        lock_content.contains("source_name"),
        "source_name preserved: {lock_content}"
    );
}

#[test]
fn update_dry_run_handles_aliased_items() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let skill_dir = tmp.path().join("source/original-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nschema: 1\nname: original-skill\ndescription: Changed\n---\nNew content.\n",
    )
    .unwrap();

    let lockfile = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "skill",
            "name": "my-alias",
            "source": "local:./source",
            "source_name": "original-skill",
            "hash": "sha256:oldoldold"
        }],
        "bundles": [],
        "plugins": []
    });
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        serde_json::to_string_pretty(&lockfile).unwrap(),
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    // Dry-run should detect changes (hash differs)
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--dry-run"])
        .assert()
        .success();
}
