//! ATDD coverage for the UX polish bundle (#118):
//!
//! - short flags (`-n` for `--dry-run`, `-s` for `--strict` and
//!   `--source`, `-l` for `--limit`)
//! - bulk-remove confirmation (skipped under `--yes` / non-TTY)
//! - "Cloning ..." progress line on git-backed installs

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
fn update_short_dry_run_flag_is_recognised() {
    // `-n` mirrors `make -n` / `rsync -n` — write nothing, report what
    // would change.
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

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update", "-n"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("Dry-run"), "expected dry-run header: {out}");
}

#[test]
fn lint_short_strict_flag_is_recognised() {
    let tmp = tempfile::tempdir().unwrap();
    let item = tmp.path().join("skills/strict-h1/SKILL.md");
    fs::create_dir_all(item.parent().unwrap()).unwrap();
    fs::write(
        &item,
        concat!(
            "---\n",
            "schema: 1\n",
            "name: strict-h1\n",
            "description: Body H1 promoted to error in -s.\n",
            "---\n",
            "\n",
            "# Body H1 here\n",
        ),
    )
    .unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["lint", "-s"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn search_short_limit_flag_is_recognised() {
    // Hit a closed loopback port so the request fails fast — we only
    // care that `-l` parses, not that the registry is reachable.
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .env("UPSKILL_REGISTRY_URL", "http://127.0.0.1:1")
        .args(["search", "anything", "-l", "3"])
        .assert()
        .failure();
}

#[test]
fn remove_short_source_flag_is_recognised() {
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

    let label = format!("local:{}", source.display());
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["remove", "-s", &label, "-y"])
        .assert()
        .success();
}

#[test]
fn remove_by_source_under_yes_flag_skips_prompt_and_proceeds() {
    // No TTY in assert_cmd, so the prompt would already be skipped — but
    // the explicit -y is the user-visible contract: never prompt, always
    // proceed. Worth pinning down.
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

    let label = format!("local:{}", source.display());
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["remove", "--source", &label, "--yes"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("Removed"),
        "expected removal report on stdout, got: {stdout}"
    );
}

#[test]
fn remove_by_source_in_non_tty_does_not_prompt_and_proceeds() {
    // assert_cmd captures stdin via a pipe — never a TTY. The CLI must
    // proceed silently without prompting (the alternative would deadlock
    // CI on stdin read).
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

    let label = format!("local:{}", source.display());
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["remove", "--source", &label])
        .assert()
        .success();
}

#[test]
fn add_against_unreachable_github_emits_progress_line_to_stderr() {
    // We can't really clone in CI, but we can check that the "Cloning"
    // progress line is emitted to stderr before the failure. The clone
    // fails fast (DNS / port-1) and we just inspect captured stderr.
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        // Bogus repo on a private TLD. git resolution fails instantly.
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["add", "no-such-owner-12345/no-such-repo-12345"])
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("Cloning") && stderr.contains("github:"),
        "expected `Cloning github:...` line on stderr, got: {stderr}"
    );
}
