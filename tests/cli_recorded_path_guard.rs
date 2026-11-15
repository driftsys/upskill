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

/// Appends a record for `path` with `sha256` to the list `list` (`files` or
/// `pending`) of the item `name` in the lockfile under `root`.
fn add_record_to(root: &Path, name: &str, list: &str, client: &str, path: &str, sha256: &str) {
    let lock_path = root.join(".upskill-lock.json");
    let mut lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    let item = lock["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["name"] == name)
        .unwrap();
    if item.get(list).is_none() {
        item[list] = serde_json::json!([]);
    }
    item[list]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"client": client, "path": path, "sha256": sha256}));
    fs::write(&lock_path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();
}

fn add_record(root: &Path, path: &str, sha256: &str) {
    add_record_to(root, ITEM, "files", "claude", path, sha256);
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

fn stderr_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).to_string()
}

#[test]
fn a_multi_source_update_refuses_before_it_writes_anything() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let items = Path::new(FIXTURES).join("items");
    copy_dir_all(&items.join(ITEM), &tmp.path().join("a").join(ITEM));
    copy_dir_all(
        &items.join("security-reviewer"),
        &tmp.path().join("b").join("security-reviewer"),
    );
    for source in ["./a", "./b"] {
        common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(["add", source, "--claude", "--project"])
            .assert()
            .success();
    }
    // A changes in its source; B gets a record outside its layout.
    let skill = tmp.path().join("a").join(ITEM).join("SKILL.md");
    let mut text = fs::read_to_string(&skill).unwrap();
    text.push_str("\nA new line.\n");
    fs::write(&skill, text).unwrap();
    fs::create_dir_all(tmp.path().join(".github")).unwrap();
    fs::write(tmp.path().join(".github/x.yml"), b"x\n").unwrap();
    add_record_to(
        tmp.path(),
        "security-reviewer",
        "files",
        "claude",
        ".github/x.yml",
        &sha256_hex(b"x\n"),
    );
    let before = snapshot(tmp.path());

    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1);
    assert!(stderr_of(&assert).contains(".github/x.yml"));
    assert_eq!(snapshot(tmp.path()), before, "nothing may change");
}

#[test]
fn update_refuses_a_pending_record_outside_the_item_layout() {
    let (tmp, home) = installed();
    fs::create_dir_all(tmp.path().join(".github/workflows")).unwrap();
    fs::write(tmp.path().join(".github/workflows/ci.yml"), b"ci\n").unwrap();
    add_record_to(
        tmp.path(),
        ITEM,
        "pending",
        "claude",
        ".github/workflows/ci.yml",
        &sha256_hex(b"ci\n"),
    );
    let before = snapshot(tmp.path());
    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1);
    let stderr = stderr_of(&assert);
    assert!(stderr.contains(".github/workflows/ci.yml"), "{stderr}");
    assert!(stderr.contains("pending"), "{stderr}");
    assert_eq!(snapshot(tmp.path()), before);
}

#[test]
fn update_refuses_a_record_of_another_client() {
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
        .args(["add", "./source", ITEM, "--copilot", "--project"])
        .assert()
        .success();
    fs::create_dir_all(tmp.path().join(".github/workflows")).unwrap();
    fs::write(tmp.path().join(".github/workflows/ci.yml"), b"ci\n").unwrap();
    add_record_to(
        tmp.path(),
        ITEM,
        "files",
        "copilot",
        ".github/workflows/ci.yml",
        &sha256_hex(b"ci\n"),
    );
    let before = snapshot(tmp.path());
    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1);
    assert!(stderr_of(&assert).contains(".github/workflows/ci.yml"));
    assert_eq!(snapshot(tmp.path()), before);
}

#[test]
fn update_refuses_a_record_in_the_global_scope() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    copy_dir_all(
        &Path::new(FIXTURES).join("items"),
        &tmp.path().join("source"),
    );
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", ITEM, "--claude", "--global"])
        .assert()
        .success();
    fs::create_dir_all(home.join(".github/workflows")).unwrap();
    fs::write(home.join(".github/workflows/ci.yml"), b"ci\n").unwrap();
    add_record_to(
        &home,
        ITEM,
        "files",
        "claude",
        ".github/workflows/ci.yml",
        &sha256_hex(b"ci\n"),
    );
    let before = snapshot(&home);
    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--global"])
        .assert()
        .code(1);
    let stderr = stderr_of(&assert);
    assert!(stderr.contains(".github/workflows/ci.yml"), "{stderr}");
    assert!(
        stderr.contains(&home.join(".upskill-lock.json").display().to_string()),
        "{stderr}"
    );
    assert_eq!(snapshot(&home), before);
}
