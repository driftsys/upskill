use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn update_dryrun_shows_preview_without_changes() {
    let cwd = tempdir().expect("must create temp dir");

    // Install a skill
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    // Dry-run update
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");
    cmd.current_dir(cwd.path())
        .args(["update", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains("lint"))
        .stdout(predicates::str::contains("dry-run"));
}

#[test]
fn update_dryrun_does_not_modify_files() {
    let cwd = tempdir().expect("must create temp dir");

    // Install a skill
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    // Record the skill dir modification time
    let skill_dir = cwd.path().join(".skills").join("lint");
    let before = std::fs::metadata(&skill_dir)
        .and_then(|m| m.modified())
        .ok();

    // Small delay to ensure timestamps differ if modified
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Dry-run update
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["update", "--dry-run"])
        .assert()
        .success();

    let after = std::fs::metadata(&skill_dir)
        .and_then(|m| m.modified())
        .ok();
    assert_eq!(before, after, "files should not be modified during dry-run");
}

#[test]
fn update_dryrun_no_lockfile() {
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["update", "--dry-run"])
        .assert()
        .success()
        .stdout("no skills installed\n");
}
