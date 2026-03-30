use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn help_exits_zero() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["--help"])
        .assert()
        .code(0);
}

#[test]
fn usage_errors_exit_two() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "invalid-source"])
        .assert()
        .code(2);
}

#[test]
fn general_errors_exit_one() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .env_remove("HOME")
        .args(["list", "-g"])
        .assert()
        .code(1)
        .stderr("error: HOME is not set\n");
}
