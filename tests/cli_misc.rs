//! Coverage gaps from the audit (epic #105 / issue #111): documented
//! behavior that already worked but had no test asserting it.
//!
//! - Spec §2.6: `install` and `uninstall` are NOT aliases.
//! - Spec §3.2: outputs are file copies, never symlinks.
//! - Spec §3.4: `CLAUDE.md` bridge is `@AGENTS.md\n` literally.

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
fn install_is_not_an_alias_for_add() {
    // Spec §2.6: `add` does NOT alias `install`. clap rejects unknown
    // subcommands at parse time → exit 2.
    let cwd = tempfile::tempdir().unwrap();
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(cwd.path())
        .args(["install", "owner/repo"])
        .assert()
        .failure()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("unrecognized subcommand") && stderr.contains("install"),
        "expected unrecognized-subcommand error: {stderr}"
    );
}

#[test]
fn uninstall_is_not_an_alias_for_remove() {
    // Spec §2.6: `remove` does NOT alias `uninstall`.
    let cwd = tempfile::tempdir().unwrap();
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(cwd.path())
        .args(["uninstall", "code-review"])
        .assert()
        .failure()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("unrecognized subcommand") && stderr.contains("uninstall"),
        "expected unrecognized-subcommand error: {stderr}"
    );
}

#[test]
fn outputs_are_file_copies_not_symlinks() {
    // Spec §3.2: every per-client output is a file copy, never a symlink.
    // Windows portability without Developer Mode requires this — symlinks
    // would silently degrade.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    // Spot-check one output per client. `symlink_metadata` does NOT
    // follow symlinks; `is_symlink()` would be true for any link.
    for path in [
        ".claude/skills/create-api-endpoint/SKILL.md",
        ".github/instructions/api-conventions.instructions.md",
        ".opencode/agents/security-reviewer.md",
        ".agents/skills/create-api-endpoint/SKILL.md",
    ] {
        let full = target.join(path);
        let meta = fs::symlink_metadata(&full)
            .unwrap_or_else(|e| panic!("symlink_metadata({}): {e}", full.display()));
        assert!(
            !meta.file_type().is_symlink(),
            "{path} must be a regular file, not a symlink"
        );
        assert!(meta.file_type().is_file(), "{path} must be a regular file");
    }
}

#[test]
fn claude_md_bridge_content_is_at_agents_md() {
    // Spec §3.4: `CLAUDE.md` is created with the literal `@AGENTS.md`
    // bridge content if absent. The single line is what makes Claude
    // Code load `AGENTS.md` (which it does not read natively).
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    let claude_md =
        fs::read_to_string(target.join("CLAUDE.md")).expect("CLAUDE.md must be created");
    assert_eq!(
        claude_md, "@AGENTS.md\n",
        "CLAUDE.md bridge content must be exactly `@AGENTS.md\\n`"
    );
}
