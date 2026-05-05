//! ATDD coverage for spec §6.1 / §6.2: stdout = data, stderr = errors,
//! and ANSI auto-disabled when stdout is not a TTY.
//!
//! SIGINT exit-130 (spec §6.1) is intentionally not covered here — the
//! CLI has no long-running command that's deterministic in CI without
//! the network. Tracked in #112 as deferred.

use assert_cmd::Command;
use std::fs;

fn ansi_present(s: &str) -> bool {
    s.contains('\x1b')
}

#[test]
fn list_data_goes_to_stdout_stderr_is_empty() {
    // Empty lockfile in a faux project (no .git is fine — list short-
    // circuits on missing lockfile to "no items installed").
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir(tmp.path().join(".git")).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["list", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    assert!(
        stdout.contains("no items installed"),
        "expected data on stdout, got: {stdout:?}"
    );
    assert!(
        stderr.is_empty(),
        "expected stderr empty for success path, got: {stderr:?}"
    );
}

#[test]
fn add_invalid_source_writes_error_to_stderr_not_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "not a valid source"])
        .assert()
        .failure()
        .code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    assert!(
        stdout.is_empty(),
        "expected stdout empty for usage error, got: {stdout:?}"
    );
    assert!(
        stderr.contains("error:"),
        "expected `error:` prefix on stderr, got: {stderr:?}"
    );
}

#[test]
fn list_stdout_has_no_ansi_when_piped() {
    // assert_cmd captures via pipes — never a TTY. The `colored` disable
    // chain (`init` in src/style.rs) auto-disables in that case, so
    // captured bytes must contain no `\x1b` escape sequences. This is the
    // user-facing guarantee behind spec §6.2 "no color in pipes/CI".
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir(tmp.path().join(".git")).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["list", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        !ansi_present(&stdout),
        "stdout must not contain ANSI escapes when piped, got: {stdout:?}"
    );
}

#[test]
fn add_invalid_source_stderr_has_no_ansi_under_no_color_env() {
    // `NO_COLOR` is the spec-blessed kill switch; even on a TTY it must
    // suppress ANSI. Belt-and-braces against a future regression where
    // styled `error:` slips through with a hard-coded escape.
    let tmp = tempfile::tempdir().unwrap();
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .env("NO_COLOR", "1")
        .args(["add", "not a valid source"])
        .assert()
        .failure()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        !ansi_present(&stderr),
        "NO_COLOR=1 must strip ANSI from stderr, got: {stderr:?}"
    );
}
