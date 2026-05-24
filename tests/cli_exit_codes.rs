//! Exit-code contract per ADR-0004: 0 success, 1 general error, 2 usage,
//! 130 SIGINT. These tests pin the codes the CLI must produce for each
//! bucket so the contract holds across implementation changes.

mod common;

use std::fs;

use tempfile::tempdir;

#[test]
fn version_long_flag_prints_to_stdout_and_exits_zero() {
    let cwd = tempdir().expect("must create temp dir");
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let mut cmd = common::upskill_cmd(&home);

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
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let mut cmd = common::upskill_cmd(&home);

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
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let mut cmd = common::upskill_cmd(&home);

    cmd.current_dir(cwd.path())
        .args(["--help"])
        .assert()
        .code(0);
}

#[test]
fn usage_errors_exit_two() {
    let cwd = tempdir().expect("must create temp dir");
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let mut cmd = common::upskill_cmd(&home);

    cmd.current_dir(cwd.path())
        .args(["add", "invalid-source"])
        .assert()
        .code(2);
}

#[test]
fn general_errors_exit_one() {
    // `add --global` resolves the install target from `$HOME`. With HOME
    // (and USERPROFILE on Windows) unset, the target lookup fails — a
    // general error (exit 1), not a usage error.
    let cwd = tempdir().expect("must create temp dir");
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let mut cmd = common::upskill_cmd(&home);

    cmd.current_dir(cwd.path())
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .args(["add", "--global", "owner/repo"])
        .assert()
        .code(1)
        .stderr("error: HOME (or USERPROFILE on Windows) is not set\n");
}
