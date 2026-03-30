use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn remove_deletes_installed_skill_with_yes() {
    let cwd = tempdir().expect("must create temp dir");

    let mut add = Command::cargo_bin("upskill").expect("binary exists");
    add.current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "rust-lint"])
        .assert()
        .success();

    let mut remove = Command::cargo_bin("upskill").expect("binary exists");
    remove
        .current_dir(cwd.path())
        .args(["remove", "rust-lint", "--yes"])
        .assert()
        .success()
        .stdout("removed skill: rust-lint\n");

    assert!(!cwd.path().join(".agents/skills/rust-lint").exists());
}

#[test]
fn remove_without_yes_skips_prompt_in_non_tty() {
    let cwd = tempdir().expect("must create temp dir");

    let mut add = Command::cargo_bin("upskill").expect("binary exists");
    add.current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "rust-lint"])
        .assert()
        .success();

    let mut remove = Command::cargo_bin("upskill").expect("binary exists");
    remove
        .current_dir(cwd.path())
        .args(["remove", "rust-lint"])
        .assert()
        .success()
        .stdout("removed skill: rust-lint\n");

    assert!(!cwd.path().join(".agents/skills/rust-lint").exists());
}

#[test]
fn remove_deletes_agent_symlinks_when_no_skills_remain() {
    let cwd = tempdir().expect("must create temp dir");
    std::fs::create_dir_all(cwd.path().join(".claude")).expect("must create .claude");
    std::fs::create_dir_all(cwd.path().join(".github")).expect("must create .github");

    let mut add = Command::cargo_bin("upskill").expect("binary exists");
    add.current_dir(cwd.path())
        .args([
            "add",
            "microsoft/skills",
            "--skill",
            "rust-lint",
            "--claude",
            "--copilot",
        ])
        .assert()
        .success();

    let mut remove = Command::cargo_bin("upskill").expect("binary exists");
    remove
        .current_dir(cwd.path())
        .args(["remove", "rust-lint", "--yes"])
        .assert()
        .success();

    assert!(std::fs::symlink_metadata(cwd.path().join(".claude/skills")).is_err());
    assert!(std::fs::symlink_metadata(cwd.path().join(".github/skills")).is_err());
}

#[test]
fn remove_keeps_symlinks_when_other_skills_exist() {
    let cwd = tempdir().expect("must create temp dir");
    std::fs::create_dir_all(cwd.path().join(".claude")).expect("must create .claude");

    let mut add = Command::cargo_bin("upskill").expect("binary exists");
    add.current_dir(cwd.path())
        .args([
            "add",
            "microsoft/skills",
            "--skill",
            "rust-lint",
            "--skill",
            "release-check",
            "--claude",
        ])
        .assert()
        .success();

    let mut remove = Command::cargo_bin("upskill").expect("binary exists");
    remove
        .current_dir(cwd.path())
        .args(["remove", "rust-lint", "--yes"])
        .assert()
        .success();

    let link_meta = std::fs::symlink_metadata(cwd.path().join(".claude/skills")).expect("metadata");
    assert!(link_meta.file_type().is_symlink());
    assert!(cwd.path().join(".agents/skills/release-check").is_dir());
}

#[test]
fn remove_fails_for_missing_skill() {
    let cwd = tempdir().expect("must create temp dir");

    let mut remove = Command::cargo_bin("upskill").expect("binary exists");
    remove
        .current_dir(cwd.path())
        .args(["remove", "missing", "--yes"])
        .assert()
        .code(2)
        .stderr("error: skill not installed: missing\n");
}
