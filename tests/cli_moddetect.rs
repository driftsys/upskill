use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn update_detects_local_modifications_and_skips() {
    let cwd = tempdir().expect("must create temp dir");

    // Install a skill
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    // Modify the installed skill locally
    let skill_file = cwd
        .path()
        .join(".agents")
        .join("skills")
        .join("lint")
        .join("modified.txt");
    std::fs::write(&skill_file, "local change").expect("write local change");

    // Update should warn about modifications and skip
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["update"])
        .assert()
        .success()
        .stderr(predicates::str::contains("local modifications"));
}

#[test]
fn update_force_overwrites_modified_skills() {
    let cwd = tempdir().expect("must create temp dir");

    // Install a skill
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    // Modify the installed skill locally
    let skill_file = cwd
        .path()
        .join(".agents")
        .join("skills")
        .join("lint")
        .join("modified.txt");
    std::fs::write(&skill_file, "local change").expect("write local change");

    // Force update should proceed
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["update", "--force"])
        .assert()
        .success()
        .stdout(predicates::str::contains("updated: lint"));
}

#[test]
fn lockfile_stores_hash_on_add() {
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    let lockfile =
        std::fs::read_to_string(cwd.path().join(".upskill-lock.json")).expect("lockfile exists");
    assert!(
        lockfile.contains("\"hash\""),
        "lockfile should contain hash field"
    );
}

#[test]
fn update_unmodified_skill_succeeds() {
    let cwd = tempdir().expect("must create temp dir");

    // Install a skill
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    // Update without modifications should succeed normally
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["update"])
        .assert()
        .success()
        .stdout(predicates::str::contains("updated: lint"))
        .stderr(predicates::str::contains("local modifications").not());
}
