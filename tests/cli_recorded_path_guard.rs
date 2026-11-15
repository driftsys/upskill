//! ATDD tests for the recorded-path guard: a `files` record whose path is
//! outside the layout that upskill writes for the item is refused by
//! `update` and `remove`, and nothing is deleted.

mod common;

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
const ITEM: &str = "create-api-endpoint";

fn copy_dir_all(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let to_path = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir_all(&entry.path(), &to_path);
        } else {
            fs::copy(entry.path(), &to_path).unwrap();
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A project with the item installed for Claude Code from `./source`.
fn installed() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    copy_dir_all(
        &Path::new(FIXTURES).join("items"),
        &tmp.path().join("source"),
    );
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", ITEM, "--claude", "--project"])
        .assert()
        .success();
    (tmp, home)
}

/// Appends a `files` record for `path` with `sha256` to the item.
fn add_record(root: &Path, path: &str, sha256: &str) -> Vec<u8> {
    let lock_path = root.join(".upskill-lock.json");
    let mut lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    let item = lock["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["name"] == ITEM)
        .unwrap();
    item["files"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"client": "claude", "path": path, "sha256": sha256}));
    fs::write(&lock_path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();
    fs::read(&lock_path).unwrap()
}

fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "home" || n == ".git") {
                    continue;
                }
                walk(&path, root, out);
            } else {
                let rel = path.strip_prefix(root).unwrap().display().to_string();
                out.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn assert_refused(command: &[&str], record_path: &str, outside: Option<&str>) {
    let (tmp, home) = installed();
    let bytes = b"name: ci\n";
    if let Some(outside) = outside {
        let full = tmp.path().join(outside);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, bytes).unwrap();
    }
    add_record(tmp.path(), record_path, &sha256_hex(bytes));
    let before = snapshot(tmp.path());

    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(command)
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains(record_path), "{stderr}");
    assert!(stderr.contains(".upskill-lock.json"), "{stderr}");
    assert!(stderr.contains(ITEM), "{stderr}");
    assert_eq!(snapshot(tmp.path()), before, "nothing may change");
}

#[test]
fn update_refuses_a_record_outside_the_item_layout() {
    assert_refused(
        &["update", "--yes", "--project"],
        ".github/workflows/ci.yml",
        Some(".github/workflows/ci.yml"),
    );
}

#[test]
fn remove_refuses_a_record_outside_the_item_layout() {
    assert_refused(
        &["remove", ITEM, "--project"],
        ".github/workflows/ci.yml",
        Some(".github/workflows/ci.yml"),
    );
}

#[test]
fn update_refuses_a_record_of_another_item() {
    assert_refused(
        &["update", "--yes", "--project"],
        ".claude/skills/other/SKILL.md",
        Some(".claude/skills/other/SKILL.md"),
    );
}

#[test]
fn update_refuses_a_record_with_a_parent_component() {
    assert_refused(
        &["update", "--yes", "--project"],
        ".claude/skills/../../outside.txt",
        None,
    );
}

#[test]
fn remove_refuses_a_record_with_a_parent_component() {
    assert_refused(
        &["remove", ITEM, "--project"],
        ".claude/skills/../../outside.txt",
        None,
    );
}

#[test]
fn update_refuses_an_absolute_record() {
    assert_refused(&["update", "--yes", "--project"], "/etc/hosts", None);
}

#[test]
fn remove_refuses_an_absolute_record() {
    assert_refused(&["remove", ITEM, "--project"], "/etc/hosts", None);
}

#[test]
fn a_record_of_a_resource_file_in_the_item_directory_is_still_deleted() {
    let (tmp, home) = installed();
    let stale = tmp.path().join(".claude/skills").join(ITEM).join("old.md");
    fs::write(&stale, b"old\n").unwrap();
    add_record(
        tmp.path(),
        &format!(".claude/skills/{ITEM}/old.md"),
        &sha256_hex(b"old\n"),
    );
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(!stale.exists());
}
