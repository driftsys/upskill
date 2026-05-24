//! ATDD tests for global vs project scope and the auto-fallback rule.
//!
//! Spec §1.2 / §2.1: project (`<cwd>/`) is the default; global (`$HOME/`)
//! kicks in when `-g/--global` is passed OR when `cwd` is not inside a git
//! repo. `-p/--project` forces project regardless. `-g` and `-p` are
//! mutually exclusive at the clap layer.

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

#[test]
fn add_global_writes_under_home_not_cwd() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let home = tmp.path().join("fakehome");
    let cwd = tmp.path().join("cwd");
    stage_source(&source);
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    // Mark cwd as a project so without `-g` we'd go project — proves the
    // flag is what flips the scope, not the missing `.git`.
    fs::create_dir_all(cwd.join(".git")).unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env("HOME", &home)
        .args(["add", "--global", source.to_str().unwrap()])
        .assert()
        .success();

    // Outputs land under HOME, not cwd.
    assert!(
        home.join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "global skill output under HOME"
    );
    assert!(
        home.join(".upskill-lock.json").exists(),
        "global lockfile under HOME"
    );
    assert!(
        !cwd.join(".upskill-lock.json").exists(),
        "no project lockfile under cwd"
    );
    assert!(
        !cwd.join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "no project output under cwd"
    );
}

#[test]
fn add_short_g_flag_works_too() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let home = tmp.path().join("fakehome");
    let cwd = tmp.path().join("cwd");
    stage_source(&source);
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(cwd.join(".git")).unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env("HOME", &home)
        .args(["add", "-g", source.to_str().unwrap()])
        .assert()
        .success();

    assert!(home.join(".upskill-lock.json").exists());
}

#[test]
fn auto_fallback_when_not_in_git_repo_uses_global() {
    // No `.git` anywhere up the tree — auto-fallback to global.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let home = tmp.path().join("fakehome");
    let cwd = tmp.path().join("cwd");
    stage_source(&source);
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    // No `.git` marker — cwd is NOT a project.

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env("HOME", &home)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    assert!(
        home.join(".upskill-lock.json").exists(),
        "auto-fallback wrote to HOME"
    );
    assert!(
        !cwd.join(".upskill-lock.json").exists(),
        "auto-fallback did NOT write to cwd"
    );
}

#[test]
fn project_flag_overrides_auto_fallback() {
    // `cwd` is NOT in a git repo, so auto-fallback would pick global.
    // `-p/--project` forces project anyway.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let home = tmp.path().join("fakehome");
    let cwd = tmp.path().join("cwd");
    stage_source(&source);
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    // No `.git` marker.

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env("HOME", &home)
        .args(["add", "--project", source.to_str().unwrap()])
        .assert()
        .success();

    assert!(
        cwd.join(".upskill-lock.json").exists(),
        "--project forced lockfile under cwd despite no .git"
    );
    assert!(
        !home.join(".upskill-lock.json").exists(),
        "--project did NOT write to HOME"
    );
}

#[test]
fn global_and_project_flags_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "--global", "--project", "owner/repo"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn list_global_reads_home_lockfile() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let home = tmp.path().join("fakehome");
    let cwd = tmp.path().join("cwd");
    stage_source(&source);
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(cwd.join(".git")).unwrap();

    // Install globally.
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env("HOME", &home)
        .args(["add", "--global", source.to_str().unwrap()])
        .assert()
        .success();

    // `list` without `-g` from project cwd: reports nothing (project lockfile is absent).
    let project_assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env("HOME", &home)
        .args(["list"])
        .assert()
        .success();
    let project_out = String::from_utf8(project_assert.get_output().stdout.clone()).unwrap();
    assert!(
        project_out.contains("No items installed"),
        "project list empty: {project_out}"
    );

    // `list -g` reads the global lockfile.
    let global_assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env("HOME", &home)
        .args(["list", "-g"])
        .assert()
        .success();
    let global_out = String::from_utf8(global_assert.get_output().stdout.clone()).unwrap();
    assert!(
        global_out.contains("create-api-endpoint"),
        "global list shows installed skill: {global_out}"
    );
}

#[test]
fn add_global_uses_userprofile_when_home_unset() {
    // Windows compatibility: when HOME is absent, USERPROFILE should be
    // used as the global install target.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let home = tmp.path().join("fakehome");
    let cwd = tmp.path().join("cwd");
    stage_source(&source);
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(cwd.join(".git")).unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env_remove("HOME")
        .env("USERPROFILE", &home)
        .args(["add", "--global", source.to_str().unwrap()])
        .assert()
        .success();

    assert!(
        home.join(".upskill-lock.json").exists(),
        "global lockfile written under USERPROFILE"
    );
    assert!(
        home.join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "global skill output under USERPROFILE"
    );
}

#[test]
fn add_global_errors_clearly_when_neither_home_nor_userprofile_set() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path().join("cwd");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(cwd.join(".git")).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .args(["add", "--global", "./nonexistent"])
        .assert()
        .failure()
        .code(1);

    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("USERPROFILE"),
        "error mentions USERPROFILE: {stderr}"
    );
}
