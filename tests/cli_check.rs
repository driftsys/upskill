use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn check_no_lockfile_reports_no_skills() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["check"])
        .assert()
        .success()
        .stdout("no skills installed\n");
}

#[test]
fn check_with_installed_skills_reports_current() {
    let cwd = tempdir().expect("must create temp dir");

    // Install a skill first
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills@v1.0", "--skill", "lint"])
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");
    cmd.current_dir(cwd.path())
        .args(["check"])
        .assert()
        .success()
        .stdout(predicates::str::contains("lint"))
        .stdout(predicates::str::contains("github:microsoft/skills@v1.0"));
}

#[test]
fn check_with_multiple_skills() {
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills@v1.0", "--skill", "lint"])
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills@v2.0", "--skill", "format"])
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");
    cmd.current_dir(cwd.path())
        .args(["check"])
        .assert()
        .success()
        .stdout(predicates::str::contains("format"))
        .stdout(predicates::str::contains("lint"));
}
