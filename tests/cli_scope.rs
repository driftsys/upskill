//! ATDD tests for scope reporting, both-scope `list` and `doctor`, and the
//! global output paths (re-steer design, section 3).
mod common;

use std::fs;
use std::path::{Path, PathBuf};

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

/// `repo` holds `.git`, `source` holds the fixture items, `outside` is not
/// inside a git repository.
struct Env {
    #[allow(dead_code)] // Keeps the temporary directory alive.
    tmp: tempfile::TempDir,
    home: PathBuf,
    repo: PathBuf,
    outside: PathBuf,
    source: PathBuf,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    let outside = tmp.path().join("outside");
    let source = tmp.path().join("source");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    stage_source(&source);
    Env {
        tmp,
        home,
        repo,
        outside,
        source,
    }
}

fn run(env: &Env, cwd: &Path, args: &[&str]) -> std::process::Output {
    common::upskill_cmd(&env.home)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[allow(dead_code)] // Used by later tests of this file.
fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn add_outside_a_git_repository_without_a_flag_asks_for_one() {
    let e = env();
    let out = run(&e, &e.outside, &["add", e.source.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    let err = stderr(&out);
    assert!(err.contains("not inside a git repository"), "{err}");
    assert!(
        err.contains("--project") && err.contains("--global"),
        "{err}"
    );
    assert!(!e.home.join(".upskill-lock.json").exists());
    assert!(!e.outside.join(".upskill-lock.json").exists());
}

#[test]
fn add_with_an_invalid_source_outside_a_git_repository_reports_the_source() {
    let e = env();
    let out = run(&e, &e.outside, &["add", "not-a-valid-source"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(stderr(&out), "error: source must be in owner/repo format\n");
}

#[test]
fn add_update_and_remove_state_the_scope_and_its_root() {
    let e = env();
    let src = e.source.to_str().unwrap();
    let project = format!(
        "scope: project ({})",
        e.repo.canonicalize().unwrap().display()
    );
    for args in [
        vec!["add", src],
        vec!["update", "--yes"],
        vec!["remove", "api-conventions"],
    ] {
        let out = run(&e, &e.repo, &args);
        assert!(out.status.success(), "{args:?}: {}", stderr(&out));
        assert!(
            stderr(&out).contains(&project),
            "{args:?}: {}",
            stderr(&out)
        );
    }
    let global = format!("scope: global ({})", e.home.display());
    for args in [
        vec!["add", "--global", src],
        vec!["update", "--yes"],
        vec!["remove", "api-conventions"],
    ] {
        // Outside a git repository, `update` and `remove` use the global scope.
        let out = run(&e, &e.outside, &args);
        assert!(out.status.success(), "{args:?}: {}", stderr(&out));
        assert!(stderr(&out).contains(&global), "{args:?}: {}", stderr(&out));
    }
}

#[test]
fn quiet_hides_the_scope_line() {
    let e = env();
    let out = run(&e, &e.repo, &["--quiet", "add", e.source.to_str().unwrap()]);
    assert!(!stderr(&out).contains("scope:"));
}
