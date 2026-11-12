//! ATDD tests for repeatable operations: when `add` or `update` fails
//! between its first and its last write, running the same command again,
//! without `--force`, completes it.
//!
//! The tests make a write fail with a read-only directory or with a file
//! size limit set by `sh`, or use a Unix terminal or file mode, so the file
//! is built on Unix only. `tests/cli_interrupted.rs` holds the tests that
//! run on every platform.
#![cfg(unix)]

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

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The hash recorded in `key` (`files` or `pending`) for `path`. Fails
/// when the record does not hold `path` exactly once.
fn hash_of(item: &serde_json::Value, key: &str, path: &str) -> String {
    let hashes: Vec<&str> = item[key]
        .as_array()
        .unwrap_or_else(|| panic!("no {key} in {item}"))
        .iter()
        .filter(|f| f["path"] == path)
        .map(|f| f["sha256"].as_str().unwrap())
        .collect();
    assert_eq!(hashes.len(), 1, "{path} in {key}: {item}");
    hashes[0].to_string()
}

/// Every hash recorded in `key` for `path`.
fn hashes_of(item: &serde_json::Value, key: &str, path: &str) -> Vec<String> {
    item[key]
        .as_array()
        .map(|records| {
            records
                .iter()
                .filter(|f| f["path"] == path)
                .map(|f| f["sha256"].as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
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
    let files_before = item(tmp.path())["files"].clone();
    write_skill(tmp.path(), "v2");
    let item_dir = tmp.path().join(".github").join("skills").join(NAME);
    let guard = Mode::set(item_dir.clone(), 0o555);
    if permissions_are_ignored(&item_dir) {
        return;
    }
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .failure();
    // The stage file of the Copilot SKILL.md cannot be created: the error
    // names the write of the destination.
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("error: write "), "{stderr}");
    // `files` still holds v1; `pending` holds the v2 hash of every file,
    // the Claude SKILL.md (written with v2 bytes) included.
    let record = item(tmp.path());
    assert_eq!(record["files"], files_before, "{record}");
    assert_eq!(list_len(&record, "pending"), 4, "{record}");
    let claude_hash = sha256(&fs::read(claude_skill(tmp.path())).unwrap());
    assert_eq!(
        hash_of(&record, "pending", ".claude/skills/tool/SKILL.md"),
        claude_hash,
        "{record}"
    );
    assert_ne!(
        hash_of(&record, "files", ".claude/skills/tool/SKILL.md"),
        claude_hash,
        "{record}"
    );
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

/// Runs `cmd` through `sh` with `ulimit -f 8` (8 blocks; the block size
/// depends on the shell: 512 bytes in a POSIX `sh`, so 4096 bytes, and 1024
/// bytes in plain bash) and the
/// signal SIGXFSZ ignored, so that a write beyond the limit fails with an
/// error instead of stopping the process.
fn run_with_file_size_limit(cmd: &assert_cmd::Command) -> std::process::Output {
    let mut sh = std::process::Command::new("sh");
    sh.arg("-c")
        .arg("ulimit -f 8; trap '' XFSZ; exec \"$@\"")
        .arg("sh")
        .arg(cmd.get_program())
        .args(cmd.get_args());
    if let Some(dir) = cmd.get_current_dir() {
        sh.current_dir(dir);
    }
    for (key, value) in cmd.get_envs() {
        match value {
            Some(value) => sh.env(key, value),
            None => sh.env_remove(key),
        };
    }
    sh.output().unwrap()
}

/// The stage file is created and the write into it fails: the stage file
/// is deleted.
#[test]
fn a_failed_run_leaves_no_stage_file() {
    let (tmp, home) = project("v1");
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(ADD)
        .assert()
        .success();
    // A body larger than the file size limit; the lockfile stays below it.
    write_skill(tmp.path(), &"x".repeat(64 * 1024));
    let mut update = common::upskill_cmd(&home);
    update
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"]);
    let out = run_with_file_size_limit(&update);
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("error: write "), "{stderr}");
    // The run failed in a write, after its pending save.
    assert!(list_len(&item(tmp.path()), "pending") > 0);
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
    // A new plan: a `pending` save for the skipped item would change the
    // lockfile bytes.
    write_skill(tmp.path(), "v2");
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

fn stdout_of(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&out.get_output().stdout).to_string()
}

/// After a failed `add`, the entry holds the source hash of the item:
/// `update --dry-run` must still report the item as one to complete.
#[test]
fn update_dry_run_after_a_failed_add_reports_the_item_to_complete() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    drop(guard);
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--dry-run", "--project"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let stdout = stdout_of(&out);
    assert!(stdout.contains("would update 1 of 1"), "{stdout}");
    assert!(stdout.contains("would complete"), "{stdout}");
    assert!(!stdout.contains("up to date"), "{stdout}");
    assert!(!copilot_skill(tmp.path()).exists());
}

#[test]
fn update_after_a_failed_add_completes_the_item() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    drop(guard);
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let stdout = stdout_of(&out);
    assert!(stdout.contains("completed"), "{stdout}");
    assert!(!stdout.contains("up to date"), "{stdout}");
    assert!(copilot_skill(tmp.path()).is_file());
    assert!(item(tmp.path()).get("pending").is_none());
}

/// Run `upskill update <args>` with a terminal on stdin and write `input`
/// to it. Returns the JSON result of `tests/fixtures/terminal_stdin.py`.
/// The same helper as in `tests/cli_update.rs`.
fn update_in_terminal(root: &Path, home: &Path, args: &[&str], input: &str) -> serde_json::Value {
    let mut update = common::upskill_cmd(home);
    update
        .current_dir(root)
        .arg("update")
        .args(args)
        .env("NO_COLOR", "1");
    let spec = serde_json::json!({
        "program": update.get_program().to_str().unwrap(),
        "args": update.get_args().map(|arg| arg.to_str().unwrap()).collect::<Vec<_>>(),
        "cwd": update.get_current_dir().unwrap(),
        "home": home,
        "env": update.get_envs().map(|(key, value)| {
            (key.to_str().unwrap(), value.map(|value| value.to_str().unwrap()))
        }).collect::<Vec<_>>(),
        "input": input,
    });
    let output = std::process::Command::new("python3")
        .env("HOME", home)
        .env("USERPROFILE", home)
        .args([
            "-c",
            include_str!("fixtures/terminal_stdin.py"),
            &spec.to_string(),
        ])
        .output()
        .expect("Python 3 with stdlib pty is required for the Unix terminal test");
    assert!(
        output.status.success(),
        "terminal fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        result["stderr"]
            .as_str()
            .unwrap()
            .contains("terminal stdin verified\n"),
        "child did not verify terminal stdin: {result}"
    );
    assert_eq!(result["timed_out"], false, "{result}");
    result
}

/// In a terminal without `--yes`, an interrupted item is a change: `update`
/// shows the plan, asks, and completes the item on "y".
#[test]
#[ignore = "rewritten in the next task"]
fn update_in_a_terminal_after_a_failed_add_asks_and_completes_on_yes() {
    let (tmp, home) = project("v1");
    let Some(guard) = failed_add(tmp.path(), &home) else {
        return;
    };
    drop(guard);
    let result = update_in_terminal(tmp.path(), &home, &["--project"], "y\n");
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        result["stderr"].as_str().unwrap().contains("[y/N]"),
        "{result}"
    );
    let stdout = result["stdout"].as_str().unwrap();
    assert!(stdout.contains("complete:  1 item(s)"), "{result}");
    assert!(!stdout.contains("nothing to do"), "{result}");
    assert!(copilot_skill(tmp.path()).is_file());
    assert!(item(tmp.path()).get("pending").is_none());
}

/// Writes a skill without resources under `<root>/tools/<name>`.
fn write_plain_skill(root: &Path, name: &str) {
    let dir = root.join("tools").join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nschema: 1\nname: {name}\ndescription: d\n---\n{name}\n"),
    )
    .unwrap();
}

/// Three skills; only `tool` changes, and its Copilot write fails. Only
/// `tool` gets a `pending` record, so `doctor` lists only `tool`.
#[test]
fn only_the_item_being_changed_is_interrupted() {
    let (tmp, home) = project("v1");
    write_plain_skill(tmp.path(), "aaa");
    write_plain_skill(tmp.path(), "zzz");
    if failed_update(tmp.path(), &home).is_none() {
        return;
    }
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

/// `doctor` reports an `update` that failed on an item that already has a
/// `files` record.
#[test]
fn doctor_reports_an_interrupted_update() {
    let (tmp, home) = project("v1");
    if failed_update(tmp.path(), &home).is_none() {
        return;
    }
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

/// The entry that a failed `add --as` inserts carries the name of the item
/// in its source.
#[test]
fn an_interrupted_aliased_item_keeps_its_source_name() {
    let (tmp, home) = project("v1");
    let skills = tmp.path().join(".github").join("skills");
    fs::create_dir_all(&skills).unwrap();
    let _guard = Mode::set(skills.clone(), 0o555);
    if permissions_are_ignored(&skills) {
        return;
    }
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./tools",
            "--as",
            "alias",
            "--claude",
            "--copilot",
            "--project",
        ])
        .assert()
        .failure();
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap())
            .unwrap();
    let entry = lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "alias")
        .unwrap_or_else(|| panic!("{lock}"));
    assert_eq!(entry["source_name"], NAME, "{entry}");
    assert!(list_len(entry, "pending") > 0, "{entry}");
}

/// R2: an `update` to v2 fails after it wrote the Claude files; the source
/// becomes v3 and the next `update` fails on its first write; a third
/// `update` completes without `--force`, because the v2 hashes stay in
/// `pending` next to the v3 hashes.
#[test]
fn update_completes_without_force_after_two_failed_runs() {
    let (tmp, home) = project("v1");
    if failed_update(tmp.path(), &home).is_none() {
        return;
    }
    let path = ".claude/skills/tool/SKILL.md";
    let v2 = sha256(&fs::read(claude_skill(tmp.path())).unwrap());
    let record = item(tmp.path());
    assert_eq!(hashes_of(&record, "pending", path), vec![v2.clone()]);

    write_skill(tmp.path(), "v3");
    let claude_item = tmp.path().join(".claude").join("skills").join(NAME);
    {
        let _item = Mode::set(claude_item.clone(), 0o555);
        let _scripts = Mode::set(claude_item.join("scripts"), 0o555);
        common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(["update", "--project"])
            .assert()
            .failure();
    }
    // Nothing of v3 was written to the Claude files: they hold v2.
    assert_eq!(sha256(&fs::read(claude_skill(tmp.path())).unwrap()), v2);
    let record = item(tmp.path());
    let pending = hashes_of(&record, "pending", path);
    assert_eq!(pending.len(), 2, "{record}");
    assert!(pending.contains(&v2), "{record}");

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .success();
    let stdout = stdout_of(&out);
    assert!(!stdout.contains("skipped"), "{stdout}");
    assert!(!stdout.contains("modified"), "{stdout}");
    assert!(
        fs::read_to_string(claude_skill(tmp.path()))
            .unwrap()
            .contains("v3")
    );
    let record = item(tmp.path());
    assert!(record.get("pending").is_none(), "{record}");
    let v3 = sha256(&fs::read(claude_skill(tmp.path())).unwrap());
    assert_eq!(hashes_of(&record, "files", path), vec![v3]);
}
