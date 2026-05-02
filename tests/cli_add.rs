//! ATDD tests for `upskill add <source>`.
//!
//! Drives the v0.2 install pipeline end-to-end via the CLI: parses an
//! `InstallSource`, runs `pipeline::install_with_lockfile`, and writes
//! per-client output for rules / skills / agents (format-spec §7).
//!
//! Local-path source only — no network. GitHub/GitLab-source coverage is
//! in `tests/pipeline_source.rs` at the library level.

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
fn add_installs_local_ssot_to_cwd() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    // Report header includes the item count (12 outputs from
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
fn add_invalid_source_returns_usage_error() {
    let tmp = tempfile::tempdir().unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "not a valid source"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn unimplemented_commands_emit_phase_message() {
    // The CLI surface lists every command from ADR-0004, but the
    // implementations land progressively. Verify the stub message points
    // the user at the phase that ships each command.
    let tmp = tempfile::tempdir().unwrap();
    for (cmd, phase) in [
        ("update", "Phase B2"),
        ("list", "Phase B"),
        ("doctor", "Phase B3"),
    ] {
        let assert = Command::cargo_bin("upskill")
            .unwrap()
            .current_dir(tmp.path())
            .args([cmd])
            .assert()
            .failure()
            .code(1);
        let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
        assert!(
            stderr.contains("not yet implemented") && stderr.contains(phase),
            "{cmd} stub message wrong: {stderr}"
        );
    }
}
