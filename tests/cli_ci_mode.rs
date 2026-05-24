//! End-to-end tests for the color disable chain (clig.dev §Output).
//!
//! Five disable signals + one re-enable signal are wired in
//! `src/style.rs`. The unit tests there cover the env-var precedence
//! exhaustively. These integration tests pin the contract through the
//! actual binary so a regression in `style::init` can't sneak past.

mod common;

use std::fs;

use tempfile::tempdir;

const ANSI_ESC: char = '\x1b';

fn assert_no_ansi(output: &str) {
    assert!(
        !output.contains(ANSI_ESC),
        "expected plain output, got ANSI: {output:?}"
    );
}

fn assert_has_ansi(output: &str) {
    assert!(
        output.contains(ANSI_ESC),
        "expected ANSI codes, got plain: {output:?}"
    );
}

#[test]
fn no_color_env_strips_ansi() {
    let cwd = tempdir().unwrap();
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let assert = common::upskill_cmd(&home)
        .current_dir(cwd.path())
        .env("NO_COLOR", "1")
        .env_remove("FORCE_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .args(["add", "not-a-valid-source"])
        .assert()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert_eq!(stderr, "error: source must be in owner/repo format\n");
}

#[test]
fn no_color_flag_strips_ansi() {
    let cwd = tempdir().unwrap();
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let assert = common::upskill_cmd(&home)
        .current_dir(cwd.path())
        .env_remove("NO_COLOR")
        .env("FORCE_COLOR", "1") // would force color, but --no-color takes precedence
        .args(["--no-color", "add", "not-a-valid-source"])
        .assert()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert_no_ansi(&stderr);
}

#[test]
fn upskill_no_color_strips_ansi() {
    let cwd = tempdir().unwrap();
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let assert = common::upskill_cmd(&home)
        .current_dir(cwd.path())
        .env_remove("NO_COLOR")
        .env("UPSKILL_NO_COLOR", "1")
        .env_remove("FORCE_COLOR")
        .args(["add", "not-a-valid-source"])
        .assert()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert_no_ansi(&stderr);
}

#[test]
fn term_dumb_strips_ansi() {
    let cwd = tempdir().unwrap();
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let assert = common::upskill_cmd(&home)
        .current_dir(cwd.path())
        .env("TERM", "dumb")
        .env_remove("NO_COLOR")
        .env_remove("FORCE_COLOR")
        .args(["add", "not-a-valid-source"])
        .assert()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert_no_ansi(&stderr);
}

#[test]
fn piped_output_strips_ansi_by_default() {
    // assert_cmd runs the binary with stderr/stdout NOT a TTY. With no
    // FORCE_COLOR override, `colored` should auto-disable.
    let cwd = tempdir().unwrap();
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let assert = common::upskill_cmd(&home)
        .current_dir(cwd.path())
        .env_remove("NO_COLOR")
        .env_remove("UPSKILL_NO_COLOR")
        .env_remove("FORCE_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .env("TERM", "xterm")
        .args(["add", "not-a-valid-source"])
        .assert()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert_no_ansi(&stderr);
}

#[test]
fn force_color_re_enables_when_piped() {
    // Even though stderr is piped (assert_cmd subprocess), FORCE_COLOR
    // tells `colored` to emit ANSI anyway. clig.dev §Environment Variables.
    let cwd = tempdir().unwrap();
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let assert = common::upskill_cmd(&home)
        .current_dir(cwd.path())
        .env_remove("NO_COLOR")
        .env_remove("UPSKILL_NO_COLOR")
        .env("FORCE_COLOR", "1")
        .env("TERM", "xterm")
        .args(["add", "not-a-valid-source"])
        .assert()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert_has_ansi(&stderr);
    // The "error:" label should be styled (red bold = sequence containing 31).
    assert!(
        stderr.contains("\x1b[1;31m") || stderr.contains("\x1b[31;1m"),
        "expected red+bold on label: {stderr:?}"
    );
}
