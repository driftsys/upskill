//! Exit-code contract per ADR-0004: 0 success, 1 general error, 2 usage,
//! 130 SIGINT. These tests pin the codes the CLI must produce for each
//! bucket so the contract holds across implementation changes.

use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn version_long_flag_prints_to_stdout_and_exits_zero() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    let expected = format!("upskill {}\n", env!("CARGO_PKG_VERSION"));
    cmd.current_dir(cwd.path())
        .args(["--version"])
        .assert()
        .code(0)
        .stdout(expected);
}

#[test]
fn version_short_flag_prints_to_stdout_and_exits_zero() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    let expected = format!("upskill {}\n", env!("CARGO_PKG_VERSION"));
    cmd.current_dir(cwd.path())
        .args(["-V"])
        .assert()
        .code(0)
        .stdout(expected);
}

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
    // `add --global` resolves the install target from `$HOME`. With HOME
    // unset, the target lookup fails — a general error (exit 1), not a
    // usage error.
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .env_remove("HOME")
        .args(["add", "--global", "owner/repo"])
        .assert()
        .code(1)
        .stderr("error: HOME is not set\n");
}
