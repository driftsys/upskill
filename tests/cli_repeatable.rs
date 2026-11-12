//! ATDD tests for repeatable operations: when `add` or `update` fails
//! between its first and its last write, running the same command again,
//! without `--force`, completes it.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

const NAME: &str = "tool";

/// Writes the skill `tool` under `<root>/tools` with `body`, and the
/// resource `scripts/run.sh` (mode 0755 on Unix).
fn write_skill(root: &Path, body: &str) {
    let dir = root.join("tools").join(NAME);
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!(
            "---\nschema: 1\nname: {NAME}\ndescription: A skill with a script\n---\n{body}\n\
             Run [the script](./scripts/run.sh).\n"
        ),
    )
    .unwrap();
    let script = dir.join("scripts").join("run.sh");
    fs::write(&script, format!("#!/bin/sh\necho {body}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn project(body: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    write_skill(tmp.path(), body);
    (tmp, home)
}

const ADD: [&str; 5] = ["add", "./tools", "--claude", "--copilot", "--project"];

fn claude_skill(root: &Path) -> PathBuf {
    root.join(".claude")
        .join("skills")
        .join(NAME)
        .join("SKILL.md")
}

fn copilot_skill(root: &Path) -> PathBuf {
    root.join(".github")
        .join("skills")
        .join(NAME)
        .join("SKILL.md")
}

fn item(root: &Path) -> serde_json::Value {
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join(".upskill-lock.json")).unwrap())
            .unwrap();
    lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == NAME)
        .unwrap_or_else(|| panic!("no item {NAME} in {lock}"))
        .clone()
}

fn list_len(item: &serde_json::Value, key: &str) -> usize {
    item[key].as_array().map_or(0, Vec::len)
}

/// Sets the mode of a directory and restores 0o755 when dropped, also
/// when an assertion fails, so that the temporary directory can be deleted.
#[cfg(unix)]
struct Mode(PathBuf);

#[cfg(unix)]
impl Mode {
    fn set(path: PathBuf, mode: u32) -> Self {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        Mode(path)
    }
}

#[cfg(unix)]
impl Drop for Mode {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
    }
}

/// True when the process can write into a directory with mode 0o555
/// (running as root): the failure cannot be produced.
#[cfg(unix)]
fn permissions_are_ignored(dir: &Path) -> bool {
    let probe = dir.join(".probe");
    let ok = fs::write(&probe, b"x").is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

/// Runs `add` with `.github/skills` read-only: the Claude file is written,
/// the Copilot file fails. Returns `None` when running as root.
#[cfg(unix)]
fn failed_add(root: &Path, home: &Path) -> Option<Mode> {
    let skills = root.join(".github").join("skills");
    fs::create_dir_all(&skills).unwrap();
    let guard = Mode::set(skills.clone(), 0o555);
    if permissions_are_ignored(&skills) {
        return None;
    }
    common::upskill_cmd(home)
        .current_dir(root)
        .args(ADD)
        .assert()
        .failure();
    Some(guard)
}

#[cfg(unix)]
#[test]
fn add_completes_on_a_second_run_after_a_failed_write() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    let record = item(tmp.path());
    assert!(list_len(&record, "pending") > 0, "{record}");
    drop(guard);

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(ADD)
        .assert()
        .success();

    assert!(claude_skill(tmp.path()).is_file());
    assert!(copilot_skill(tmp.path()).is_file());
    let record = item(tmp.path());
    assert!(record.get("pending").is_none(), "{record}");
    // SKILL.md and scripts/run.sh for each of the two clients.
    assert_eq!(list_len(&record, "files"), 4, "{record}");
}

#[cfg(unix)]
#[test]
fn update_completes_on_a_second_run_after_a_failed_write() {
    let (tmp, home) = project("v1");
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(ADD)
        .assert()
        .success();
    write_skill(tmp.path(), "v2");
    let item_dir = tmp.path().join(".github").join("skills").join(NAME);
    let guard = Mode::set(item_dir.clone(), 0o555);
    if permissions_are_ignored(&item_dir) {
        return;
    }
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .failure();
    assert!(
        fs::read_to_string(claude_skill(tmp.path()))
            .unwrap()
            .contains("v2")
    );
    assert!(
        fs::read_to_string(copilot_skill(tmp.path()))
            .unwrap()
            .contains("v1")
    );
    drop(guard);

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(!stdout.contains("skipped"), "{stdout}");
    assert!(
        fs::read_to_string(claude_skill(tmp.path()))
            .unwrap()
            .contains("v2")
    );
    assert!(
        fs::read_to_string(copilot_skill(tmp.path()))
            .unwrap()
            .contains("v2")
    );
    assert!(item(tmp.path()).get("pending").is_none());
}

fn stage_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            stage_files(&path, out);
        } else if entry
            .file_name()
            .to_string_lossy()
            .starts_with(".upskill-stage-")
        {
            out.push(path);
        }
    }
}

#[cfg(unix)]
#[test]
fn a_failed_run_leaves_no_stage_file() {
    let (tmp, home) = project("v1");
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(ADD)
        .assert()
        .success();
    write_skill(tmp.path(), "v2");
    // The item directory stays writable, its `scripts` directory does not:
    // SKILL.md is staged and renamed, the script cannot be staged.
    let scripts = tmp
        .path()
        .join(".github")
        .join("skills")
        .join(NAME)
        .join("scripts");
    let guard = Mode::set(scripts.clone(), 0o555);
    if permissions_are_ignored(&scripts) {
        return;
    }
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .failure();
    drop(guard);
    let mut found = Vec::new();
    stage_files(tmp.path(), &mut found);
    assert!(found.is_empty(), "{found:?}");
}

#[cfg(unix)]
#[test]
fn resource_keeps_the_executable_bit() {
    use std::os::unix::fs::PermissionsExt;
    let (tmp, home) = project("v1");
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(ADD)
        .assert()
        .success();
    let script = tmp
        .path()
        .join(".claude")
        .join("skills")
        .join(NAME)
        .join("scripts")
        .join("run.sh");
    let mode = fs::metadata(&script).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
}

#[cfg(unix)]
#[test]
fn doctor_reports_an_interrupted_item() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    drop(guard);
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor", "--project"])
        .assert()
        .code(1);
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("interrupted"), "{stdout}");
    assert!(
        stdout.contains("run the same command again to complete it"),
        "{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn remove_cleans_an_interrupted_install() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    drop(guard);
    assert!(claude_skill(tmp.path()).is_file());
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["remove", NAME, "--project"])
        .assert()
        .success();
    assert!(!claude_skill(tmp.path()).exists());
    let lock = fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(!lock.contains(&format!("\"{NAME}\"")), "{lock}");
}

/// The Claude file written by the failed run is recorded only in
/// `pending`. When the source changes before the second run, that file
/// holds other bytes than the plan: it is replaced only because the
/// `pending` hash makes it owned.
#[cfg(unix)]
#[test]
fn a_file_written_by_a_failed_run_is_owned_through_pending() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    drop(guard);
    write_skill(tmp.path(), "v2");
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(ADD)
        .assert()
        .success();
    assert!(
        fs::read_to_string(claude_skill(tmp.path()))
            .unwrap()
            .contains("v2")
    );
}

#[cfg(unix)]
#[test]
fn a_file_edited_after_a_failed_run_is_a_conflict() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    drop(guard);
    fs::write(claude_skill(tmp.path()), "my edit\n").unwrap();
    let lock_path = tmp.path().join(".upskill-lock.json");
    let before = fs::read(&lock_path).unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(ADD)
        .assert()
        .failure();
    assert_eq!(
        fs::read_to_string(claude_skill(tmp.path())).unwrap(),
        "my edit\n"
    );
    // The skipped item keeps its record, `pending` included.
    assert_eq!(fs::read(&lock_path).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn a_refused_command_after_a_failed_run_does_not_change_the_lockfile() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    drop(guard);
    let lock_path = tmp.path().join(".upskill-lock.json");
    let before = fs::read(&lock_path).unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./tools", "--as", "a/b", "--claude", "--project"])
        .assert()
        .failure();
    assert_eq!(fs::read(&lock_path).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn doctor_json_lists_an_interrupted_item() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    drop(guard);
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
}

/// Runs `add`, changes the source to v2, and runs an `update` that fails
/// on the Copilot item directory. Returns `None` when running as root.
#[cfg(unix)]
fn failed_update(root: &Path, home: &Path) -> Option<()> {
    common::upskill_cmd(home)
        .current_dir(root)
        .args(ADD)
        .assert()
        .success();
    write_skill(root, "v2");
    let item_dir = root.join(".github").join("skills").join(NAME);
    let guard = Mode::set(item_dir.clone(), 0o555);
    if permissions_are_ignored(&item_dir) {
        return None;
    }
    common::upskill_cmd(home)
        .current_dir(root)
        .args(["update", "--project"])
        .assert()
        .failure();
    drop(guard);
    Some(())
}

#[cfg(unix)]
#[test]
fn remove_cleans_an_interrupted_update() {
    let (tmp, home) = project("v1");
    if failed_update(tmp.path(), &home).is_none() {
        return;
    }
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["remove", NAME, "--project"])
        .assert()
        .success();
    assert!(!claude_skill(tmp.path()).exists());
    assert!(!copilot_skill(tmp.path()).exists());
    let lock = fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(!lock.contains(&format!("\"{NAME}\"")), "{lock}");
}

#[cfg(unix)]
#[test]
fn update_force_lists_a_forced_file_of_an_interrupted_item_once() {
    let (tmp, home) = project("v1");
    // A second item keeps the source from being empty.
    let other = tmp.path().join("tools").join("other");
    fs::create_dir_all(&other).unwrap();
    fs::write(
        other.join("SKILL.md"),
        "---\nschema: 1\nname: other\ndescription: d\n---\nother\n",
    )
    .unwrap();
    if failed_update(tmp.path(), &home).is_none() {
        return;
    }
    fs::write(claude_skill(tmp.path()), "my edit\n").unwrap();
    fs::remove_dir_all(tmp.path().join("tools").join(NAME)).unwrap();
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--force", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let line = format!("  .claude/skills/{NAME}/SKILL.md\n");
    assert_eq!(stdout.matches(&line).count(), 1, "{stdout}");
    assert!(!claude_skill(tmp.path()).exists());
}

#[cfg(unix)]
#[test]
fn a_file_of_a_failed_run_is_deleted_when_the_next_run_no_longer_plans_it() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    drop(guard);
    assert!(claude_skill(tmp.path()).is_file());
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./tools", "--copilot", "--project"])
        .assert()
        .success();
    assert!(!claude_skill(tmp.path()).exists());
    assert!(copilot_skill(tmp.path()).is_file());
    let lock = fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(!lock.contains(".claude/skills"), "{lock}");
    assert!(!lock.contains("\"pending\""), "{lock}");
}
