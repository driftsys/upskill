//! Tests of an interrupted operation that run on every platform: the
//! `pending` record is written by hand into the lockfile of a completed
//! `add`, so no write has to fail.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

const NAME: &str = "tool";
const ADD: [&str; 5] = ["add", "./tools", "--claude", "--copilot", "--project"];

/// A project with the skill `tool` installed for Claude and Copilot.
fn installed() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let dir = tmp.path().join("tools").join(NAME);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nschema: 1\nname: {NAME}\ndescription: A skill\n---\nBody.\n"),
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(ADD)
        .assert()
        .success();
    (tmp, home)
}

fn lock_path(root: &Path) -> PathBuf {
    root.join(".upskill-lock.json")
}

/// Changes the entry of `tool` in the lockfile with `edit`.
fn edit_entry(root: &Path, edit: impl FnOnce(&mut serde_json::Value)) {
    let mut lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(lock_path(root)).unwrap()).unwrap();
    assert_eq!(lock["schema"], 2, "{lock}");
    let entry = lock["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["name"] == NAME)
        .unwrap();
    edit(entry);
    fs::write(
        lock_path(root),
        serde_json::to_string_pretty(&lock).unwrap(),
    )
    .unwrap();
}

/// Copies the `files` record of `tool` into its `pending` record.
fn mark_interrupted(root: &Path) {
    edit_entry(root, |entry| {
        entry["pending"] = entry["files"].clone();
    });
}

fn entry(root: &Path) -> Option<serde_json::Value> {
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(lock_path(root)).unwrap()).unwrap();
    lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == NAME)
        .cloned()
}

fn skill(root: &Path, client_root: &str) -> PathBuf {
    root.join(client_root)
        .join("skills")
        .join(NAME)
        .join("SKILL.md")
}

/// Every file is present and unchanged: the interrupted item is the only
/// thing that `doctor` finds, and it is enough for exit code 1.
#[test]
fn doctor_exits_1_for_an_interrupted_item_when_nothing_else_is_wrong() {
    let (tmp, home) = installed();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor", "--project"])
        .assert()
        .success();
    mark_interrupted(tmp.path());
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor", "--json", "--project"])
        .assert()
        .code(1);
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(
        v["interrupted"],
        serde_json::json!([{ "kind": "skill", "name": NAME }]),
        "{v}"
    );
    for key in ["missing_outputs", "modified_files", "stale_hashes"] {
        assert!(v[key].as_array().is_none_or(Vec::is_empty), "{key}: {v}");
    }
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor", "--project"])
        .assert()
        .code(1);
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("interrupted"), "{stdout}");
}

/// The files of an item recorded only in `pending` are deleted by `remove`.
#[test]
fn remove_deletes_the_pending_files_of_an_interrupted_item() {
    let (tmp, home) = installed();
    edit_entry(tmp.path(), |entry| {
        entry["pending"] = entry["files"].clone();
        entry["files"] = serde_json::json!([]);
    });
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["remove", NAME, "--project"])
        .assert()
        .success();
    assert!(!skill(tmp.path(), ".claude").exists());
    assert!(!skill(tmp.path(), ".github").exists());
    assert!(entry(tmp.path()).is_none());
}

/// The interrupted operation wrote the Claude file but not the Copilot
/// file: the same `add` run again writes it and clears `pending`.
#[test]
fn the_same_add_run_again_completes_an_interrupted_item() {
    let (tmp, home) = installed();
    mark_interrupted(tmp.path());
    let copilot = skill(tmp.path(), ".github");
    fs::remove_file(&copilot).unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(ADD)
        .assert()
        .success();
    assert!(copilot.is_file());
    let entry = entry(tmp.path()).unwrap();
    assert!(entry.get("pending").is_none(), "{entry}");
    assert_eq!(entry["files"].as_array().unwrap().len(), 2, "{entry}");
}

/// The source did not change, but the item has an interrupted operation:
/// `update --dry-run` reports it as one to complete, not as up to date.
#[test]
fn update_dry_run_reports_an_interrupted_item_to_complete() {
    let (tmp, home) = installed();
    mark_interrupted(tmp.path());
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--dry-run", "--project"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("would update 1 of 1"), "{stdout}");
    assert!(stdout.contains("would complete"), "{stdout}");
    assert!(!stdout.contains("up to date"), "{stdout}");
}

#[test]
fn update_completes_an_interrupted_item() {
    let (tmp, home) = installed();
    mark_interrupted(tmp.path());
    let copilot = skill(tmp.path(), ".github");
    fs::remove_file(&copilot).unwrap();
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("completed"), "{stdout}");
    assert!(!stdout.contains("up to date"), "{stdout}");
    assert!(copilot.is_file());
    assert!(entry(tmp.path()).unwrap().get("pending").is_none());
}

/// An interrupted item with a file edited by the user is skipped: the
/// conflict rule wins over the completion of the item.
#[test]
fn update_skips_an_interrupted_item_with_an_edited_file() {
    let (tmp, home) = installed();
    mark_interrupted(tmp.path());
    let claude = skill(tmp.path(), ".claude");
    fs::write(&claude, "my edit\n").unwrap();
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .env("NO_COLOR", "1")
        .assert()
        .code(1);
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("skipped"), "{stdout}");
    assert!(!stdout.contains("completed"), "{stdout}");
    assert_eq!(fs::read_to_string(&claude).unwrap(), "my edit\n");
}

/// Every file of the interrupted item is written (a stop between the last
/// rename and the final save): `update` still reports the item as
/// completed and clears `pending`.
#[test]
fn update_completes_an_interrupted_item_whose_files_are_all_written() {
    let (tmp, home) = installed();
    mark_interrupted(tmp.path());
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("completed"), "{stdout}");
    assert!(!stdout.contains("nothing to do"), "{stdout}");
    assert!(entry(tmp.path()).unwrap().get("pending").is_none());
}

/// Writes a new body into the source of `tool`.
fn change_source(root: &Path) {
    fs::write(
        root.join("tools").join(NAME).join("SKILL.md"),
        format!("---\nschema: 1\nname: {NAME}\ndescription: A skill\n---\nChanged.\n"),
    )
    .unwrap();
}

/// An interrupted item whose source also changed is reported as changed,
/// not as one to complete.
#[test]
fn an_interrupted_item_with_a_changed_source_is_reported_as_changed() {
    let (tmp, home) = installed();
    mark_interrupted(tmp.path());
    change_source(tmp.path());
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--dry-run", "--project"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("would change"), "{stdout}");
    assert!(!stdout.contains("would complete"), "{stdout}");
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("updated"), "{stdout}");
    assert!(!stdout.contains("completed"), "{stdout}");
}
