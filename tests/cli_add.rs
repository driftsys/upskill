use assert_cmd::Command;

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
