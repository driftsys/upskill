//! ATDD test for `upskill add --pipeline <local-path>`.
//!
//! Drives the CLI surface end-to-end. The `--pipeline` flag (hidden in
//! `--help`) routes the v0.1 `add` subcommand to `pipeline::install_from_source`
//! instead of the v0.1 fetch-and-symlink flow, allowing v0.2 install to be
//! exercised via the binary while v0.1 behaviour stays the default.
//!
//! Local-path source only — no network. GitHub-source coverage is in
//! `tests/pipeline_source.rs` at the library level.

use assert_cmd::Command;
use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn stage_source(source: &Path) {
    for kind in ["skills", "rules", "agents"] {
        let from = format!("{FIXTURES}/{kind}");
        let to = source.join(kind);
        copy_dir_all(Path::new(&from), &to).unwrap();
    }
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

#[test]
fn pipeline_flag_installs_local_ssot_to_cwd() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["add", "--pipeline", source.to_str().unwrap()])
        .assert()
        .success();

    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    // Report header includes the source label and item count (12 outputs from
    // 1 skill + 2 rules + 1 agent × 3 clients).
    assert!(out.contains("Installed 12"), "stdout missing report: {out}");

    // Spot-check one output per client at the expected per-client paths.
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
    assert!(
        target
            .join(".github/instructions/api-conventions.instructions.md")
            .exists()
    );
    assert!(
        target
            .join(".opencode/agents/security-reviewer.md")
            .exists()
    );
}

#[test]
fn pipeline_flag_invalid_source_returns_usage_error() {
    let tmp = tempfile::tempdir().unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "--pipeline", "not a valid source"])
        .assert()
        .failure()
        .code(2); // EXIT_USAGE per CLI spec.
}

#[test]
fn pipeline_flag_is_hidden_from_help() {
    // The flag is intentionally hidden — it's a transitional escape hatch
    // until the v0.2 surface replaces v0.1 add. Pin the hide(true) attribute
    // so a future maintainer doesn't surface it accidentally before that
    // migration lands.
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .args(["add", "--help"])
        .assert()
        .success();

    let help = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        !help.contains("--pipeline"),
        "expected --pipeline hidden from help, got:\n{help}"
    );
}
