use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn add_global_installs_skill_in_home_scope() {
    let cwd = tempdir().expect("must create cwd");
    let home = tempdir().expect("must create home");

    let mut add = Command::cargo_bin("upskill").expect("binary exists");
    add.current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["add", "microsoft/skills", "--skill", "rust-lint", "-g"])
        .assert()
        .success();

    assert!(home.path().join(".agents/skills/rust-lint").is_dir());
    assert!(!cwd.path().join(".agents/skills/rust-lint").exists());
}

#[test]
fn list_global_reads_home_scope() {
    let cwd = tempdir().expect("must create cwd");
    let home = tempdir().expect("must create home");

    let mut add = Command::cargo_bin("upskill").expect("binary exists");
    add.current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["add", "microsoft/skills", "--skill", "rust-lint", "-g"])
        .assert()
        .success();

    let mut list = Command::cargo_bin("upskill").expect("binary exists");
    list.current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["list", "-g"])
        .assert()
        .success()
        .stdout("rust-lint\tsource=github:microsoft/skills\tsymlinks=none\n");
}

#[test]
fn remove_global_deletes_home_scope_skill() {
    let cwd = tempdir().expect("must create cwd");
    let home = tempdir().expect("must create home");

    let mut add = Command::cargo_bin("upskill").expect("binary exists");
    add.current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["add", "microsoft/skills", "--skill", "rust-lint", "-g"])
        .assert()
        .success();

    let mut remove = Command::cargo_bin("upskill").expect("binary exists");
    remove
        .current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["remove", "rust-lint", "--yes", "-g"])
        .assert()
        .success()
        .stdout("removed skill: rust-lint\n");

    assert!(!home.path().join(".agents/skills/rust-lint").exists());
}
