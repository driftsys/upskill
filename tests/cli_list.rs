use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn list_shows_no_skills_when_empty() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["list"])
        .assert()
        .success()
        .stdout("no skills installed\n");
}

#[test]
fn list_shows_installed_skill_with_source_and_symlinks() {
    let cwd = tempdir().expect("must create temp dir");
    std::fs::create_dir_all(cwd.path().join(".claude")).expect("must create .claude");

    let mut add = Command::cargo_bin("upskill").expect("binary exists");
    add.current_dir(cwd.path())
        .args([
            "add",
            "microsoft/skills",
            "--skill",
            "rust-lint",
            "--claude",
        ])
        .assert()
        .success();

    let mut list = Command::cargo_bin("upskill").expect("binary exists");
    list.current_dir(cwd.path())
        .args(["list"])
        .assert()
        .success()
        .stdout("rust-lint\tsource=github:microsoft/skills\tsymlinks=claude\n");
}

#[test]
fn list_shows_multiple_skills_sorted() {
    let cwd = tempdir().expect("must create temp dir");

    let mut add = Command::cargo_bin("upskill").expect("binary exists");
    add.current_dir(cwd.path())
        .args([
            "add",
            "microsoft/skills",
            "--skill",
            "zeta",
            "--skill",
            "alpha",
        ])
        .assert()
        .success();

    let mut list = Command::cargo_bin("upskill").expect("binary exists");
    list.current_dir(cwd.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(
            "alpha\tsource=github:microsoft/skills\tsymlinks=none\nzeta\tsource=github:microsoft/skills\tsymlinks=none\n",
        );
}
