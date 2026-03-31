use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn update_no_lockfile_reports_nothing() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["update"])
        .assert()
        .success()
        .stdout("no skills installed\n");
}

#[test]
fn update_reports_installed_skills() {
    let cwd = tempdir().expect("must create temp dir");

    // Install a skill
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");
    cmd.current_dir(cwd.path())
        .args(["update"])
        .assert()
        .success()
        .stdout(predicates::str::contains("lint"));
}

#[test]
fn update_specific_skill() {
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "format"])
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");
    cmd.current_dir(cwd.path())
        .args(["update", "lint"])
        .assert()
        .success()
        .stdout(predicates::str::contains("lint"))
        .stdout(predicates::str::contains("format").not());
}

#[test]
fn update_unknown_skill_fails() {
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");
    cmd.current_dir(cwd.path())
        .args(["update", "nonexistent"])
        .assert()
        .code(2)
        .stderr("error: skill not in lockfile: nonexistent\n");
}
