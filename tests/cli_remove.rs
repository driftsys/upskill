//! ATDD tests for `upskill remove`.
//!
//! Drives `add` then `remove` end-to-end via the CLI: verifies the
//! per-client output files come back gone and the lockfile drops the
//! entries. Ancillary files (`CLAUDE.md`, `opencode.json`,
//! `.vscode/settings.json`) must remain — they are user-owned after
//! creation per ADR-0003.

use assert_cmd::Command;
use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn stage_source(source: &Path) {
    let from = format!("{FIXTURES}/items");
    copy_dir_all(Path::new(&from), source).unwrap();
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

fn install(target: &Path, source: &Path) {
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn remove_by_name_deletes_all_per_client_outputs_and_lockfile_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    // Sanity check: install put the skill files in place.
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
    assert!(
        target
            .join(".github/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
    assert!(
        target
            .join(".agents/skills/create-api-endpoint/SKILL.md")
            .exists()
    );

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["remove", "create-api-endpoint"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("Removed 1 item"),
        "remove report missing: {stdout}"
    );

    for path in [
        ".claude/skills/create-api-endpoint/SKILL.md",
        ".github/skills/create-api-endpoint/SKILL.md",
        ".agents/skills/create-api-endpoint/SKILL.md",
    ] {
        assert!(
            !target.join(path).exists(),
            "still on disk after remove: {path}"
        );
    }

    let lock = fs::read_to_string(target.join(".upskill-lock.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&lock).unwrap();
    let names: Vec<&str> = parsed["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert!(
        !names.contains(&"create-api-endpoint"),
        "lockfile still has the entry: {names:?}"
    );
}

#[test]
fn remove_by_source_drops_every_entry_from_that_source() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    let source_label = format!("local:{}", source.display());

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["remove", "--source", &source_label])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    // Fixture corpus = 1 skill + 2 rules + 1 agent = 4 items.
    assert!(
        stdout.contains("Removed 4 item"),
        "remove report wrong: {stdout}"
    );

    let lock = fs::read_to_string(target.join(".upskill-lock.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&lock).unwrap();
    assert!(
        parsed["items"].as_array().unwrap().is_empty(),
        "lockfile not emptied: {parsed}"
    );
}

#[test]
fn remove_preserves_ancillary_files() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    let source_label = format!("local:{}", source.display());

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["remove", "--source", &source_label])
        .assert()
        .success();

    // ADR-0003: ancillary files are user-owned after creation. `remove`
    // must not delete them even when every item from a source goes away.
    assert!(
        target.join("CLAUDE.md").exists(),
        "CLAUDE.md must survive remove"
    );
    assert!(
        target.join("opencode.json").exists(),
        "opencode.json must survive remove"
    );
    assert!(
        target.join(".vscode/settings.json").exists(),
        ".vscode/settings.json must survive remove"
    );
}

#[test]
fn remove_bare_invocation_is_usage_error() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["remove"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn remove_names_and_source_together_is_usage_error() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["remove", "foo", "--source", "github:o/r"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn remove_unknown_name_is_general_error() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["remove", "this-was-never-installed"])
        .assert()
        .failure()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("this-was-never-installed"),
        "stderr should name the missing item: {stderr}"
    );
}
