//! ATDD tests for `upskill update`.
//!
//! Drives `add` then `update` end-to-end via the CLI. The local-path
//! source path lets us mutate the SSOT in place between `add` and
//! `update` to exercise the change-detection branch (lockfile hash vs
//! recomputed hash) without hitting the network.

use assert_cmd::Command;
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

fn install(target: &Path, source: &Path) {
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();
}

/// Mutate one fixture skill so its content hash will change on the next
/// install. Appends a sentinel line to the SKILL body.
fn mutate_skill(source: &Path) {
    let path = source.join("create-api-endpoint/SKILL.md");
    let original = fs::read_to_string(&path).unwrap();
    fs::write(path, format!("{original}\n<!-- mutated by test -->\n")).unwrap();
}

fn lockfile_hash_for(target: &Path, name: &str) -> Option<String> {
    let raw = fs::read_to_string(target.join(".upskill-lock.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    parsed["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"].as_str() == Some(name))
        .and_then(|i| i["hash"].as_str().map(str::to_string))
}

#[test]
fn update_no_change_reports_up_to_date() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("Updated 0 of 4 item(s)"),
        "expected no changes, got:\n{out}"
    );
    assert!(
        out.contains("up to date"),
        "expected up-to-date status, got:\n{out}"
    );
}

#[test]
fn update_after_ssot_mutation_rewrites_outputs_and_lockfile_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    let original_hash = lockfile_hash_for(&target, "create-api-endpoint").unwrap();
    mutate_skill(&source);

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update", "create-api-endpoint"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("Updated 1 of"),
        "expected 1 change, got:\n{out}"
    );

    let new_hash = lockfile_hash_for(&target, "create-api-endpoint").unwrap();
    assert_ne!(
        original_hash, new_hash,
        "lockfile hash should change after SSOT mutation"
    );

    // The rendered Claude SKILL output should now contain the mutation marker.
    let claude_out =
        fs::read_to_string(target.join(".claude/skills/create-api-endpoint/SKILL.md")).unwrap();
    assert!(
        claude_out.contains("mutated by test"),
        "regenerated output missing mutation marker"
    );
}

#[test]
fn update_dry_run_reports_change_without_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    let original_hash = lockfile_hash_for(&target, "create-api-endpoint").unwrap();
    mutate_skill(&source);

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update", "--dry-run"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("Dry-run"), "expected dry-run header: {out}");
    assert!(
        out.contains("would change"),
        "expected `would change` status: {out}"
    );

    // Lockfile must not have been touched.
    let after_hash = lockfile_hash_for(&target, "create-api-endpoint").unwrap();
    assert_eq!(
        original_hash, after_hash,
        "dry-run must not modify the lockfile"
    );

    // And the rendered output stays at the pre-mutation content.
    let claude_out =
        fs::read_to_string(target.join(".claude/skills/create-api-endpoint/SKILL.md")).unwrap();
    assert!(
        !claude_out.contains("mutated by test"),
        "dry-run must not regenerate per-client output"
    );
}

#[test]
fn update_unknown_name_is_general_error() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update", "this-was-never-installed"])
        .assert()
        .failure()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("this-was-never-installed"),
        "stderr should name the missing item: {stderr}"
    );
}

#[test]
fn update_removes_orphaned_item_when_source_no_longer_contains_it() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    // Verify the skill is installed.
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "skill should be installed before removal"
    );
    assert!(lockfile_hash_for(&target, "create-api-endpoint").is_some());

    // Remove the item from the source so it becomes an orphan.
    fs::remove_dir_all(source.join("create-api-endpoint")).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update", "--yes"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("removed") || out.contains("Removed"),
        "expected removed status, got:\n{out}"
    );

    // Output files should be gone.
    assert!(
        !target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "skill output should be deleted after orphan removal"
    );
    // Lockfile entry should be gone.
    assert!(
        lockfile_hash_for(&target, "create-api-endpoint").is_none(),
        "lockfile entry should be removed for orphaned item"
    );
}

#[test]
fn update_dry_run_reports_would_remove_without_deleting() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    // Remove the item from the source so it becomes an orphan.
    fs::remove_dir_all(source.join("create-api-endpoint")).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update", "--dry-run"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("would remove"),
        "expected `would remove` status, got:\n{out}"
    );

    // Output files should still exist.
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "dry-run must not delete output files"
    );
    // Lockfile entry should still exist.
    assert!(
        lockfile_hash_for(&target, "create-api-endpoint").is_some(),
        "dry-run must not remove lockfile entry"
    );
}

#[test]
fn update_empty_lockfile_is_no_op() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["update"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("nothing to update"),
        "expected empty-lockfile message: {out}"
    );
}
