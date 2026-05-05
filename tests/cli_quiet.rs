//! ATDD tests for the global `-q` / `--quiet` flag.
//!
//! `--quiet` suppresses informational stdout (state-change reports,
//! "no items" messages, search results). Errors on stderr are unaffected,
//! exit codes are unaffected.

use assert_cmd::Command;
use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

#[test]
fn lint_quiet_on_clean_corpus_emits_no_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    for kind in ["skills", "rules", "agents"] {
        let from = format!("{FIXTURES}/{kind}");
        let to = source.join(kind);
        copy_dir_all(Path::new(&from), &to).unwrap();
    }

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&source)
        .args(["lint", "--quiet"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.is_empty(), "expected empty stdout, got: {stdout:?}");
}

#[test]
fn lint_short_q_on_clean_corpus_emits_no_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    for kind in ["skills", "rules", "agents"] {
        let from = format!("{FIXTURES}/{kind}");
        let to = source.join(kind);
        copy_dir_all(Path::new(&from), &to).unwrap();
    }

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&source)
        .args(["lint", "-q"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.is_empty(), "expected empty stdout, got: {stdout:?}");
}

#[test]
fn quiet_does_not_silence_errors_on_stderr() {
    // Invalid source string: parser rejects it, error goes to stderr,
    // exit code is 2 (usage error). --quiet must not swallow that.
    let tmp = tempfile::tempdir().unwrap();
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "not a valid source", "--quiet"])
        .assert()
        .failure()
        .code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stdout.is_empty(),
        "expected empty stdout under --quiet, got: {stdout:?}"
    );
    assert!(
        !stderr.is_empty(),
        "expected error on stderr even under --quiet"
    );
}

#[test]
fn list_quiet_on_empty_lockfile_emits_no_stdout() {
    // No lockfile in the temp dir, but --project forces project scope so
    // list short-circuits to "no items installed" — that informational
    // message must be silenced under --quiet.
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir(tmp.path().join(".git")).unwrap(); // pretend repo

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["list", "--project", "--quiet"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.is_empty(), "expected empty stdout, got: {stdout:?}");
}

fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let to_path = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &to_path)?;
        } else {
            fs::copy(entry.path(), &to_path)?;
        }
    }
    Ok(())
}
