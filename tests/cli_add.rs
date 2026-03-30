use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn add_accepts_owner_repo_source() {
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.args(["add", "microsoft/skills"])
        .assert()
        .success()
        .stdout("install source: github\nowner: microsoft\nrepo: skills\n");
}

#[test]
fn add_rejects_invalid_source_format() {
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.args(["add", "microsoft-skills"])
        .assert()
        .code(2)
        .stderr("error: source must be in owner/repo format\n");
}

#[test]
fn add_accepts_local_path_source() {
    let tmp = tempdir().expect("must create temp dir");
    let path = tmp.path().display().to_string();

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.args(["add", path.as_str()])
        .assert()
        .success()
        .stdout(format!("install source: local\npath: {}\n", path));
}

#[test]
fn add_rejects_missing_local_path() {
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.args(["add", "./definitely-not-present-upskill-skill-path"])
        .assert()
        .code(2)
        .stderr("error: local path does not exist: ./definitely-not-present-upskill-skill-path\n");
}
