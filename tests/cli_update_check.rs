//! ATDD tests for the recorded source commit and `update --check`
//! (re-steer design, section 4).
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REMOTE: &str = "https://example.invalid/team/skills";

/// `home`, `repo` (holds `.git`, the project scope), `outside` (not in a git
/// repository) and `upstream` (the local repository behind `REMOTE`).
struct Env {
    _tmp: tempfile::TempDir,
    home: PathBuf,
    repo: PathBuf,
    #[allow(dead_code)] // Used by the tests of later tasks (global scope, no repository).
    outside: PathBuf,
    upstream: PathBuf,
}

fn git_cmd(e_home: &Path, dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.current_dir(dir)
        .env("HOME", e_home)
        .env("USERPROFILE", e_home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", e_home.join("empty-gitconfig"))
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com");
    cmd
}

fn git_in(e: &Env, args: &[&str]) -> String {
    let output = git_cmd(&e.home, &e.upstream).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn write_skill(root: &Path, name: &str, body: &str) {
    let path = root.join(format!("skills/{name}/SKILL.md"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!(
            "---\nschema: 1\nname: {name}\ndescription: Use for testing the update check\n---\n\n{body}\n"
        ),
    )
    .unwrap();
}

/// An environment whose upstream holds the skills `alpha` and `beta` in one
/// commit.
fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    let outside = tmp.path().join("outside");
    let upstream = tmp.path().join("upstream");
    for dir in [&home, &outside, &upstream, &repo.join(".git")] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(home.join("empty-gitconfig"), "").unwrap();
    let e = Env {
        _tmp: tmp,
        home,
        repo,
        outside,
        upstream,
    };
    git_in(&e, &["init", "--initial-branch=main"]);
    write_skill(&e.upstream, "alpha", "Alpha content.");
    write_skill(&e.upstream, "beta", "Beta content.");
    git_in(&e, &["add", "."]);
    git_in(&e, &["commit", "-m", "initial"]);
    e
}

/// Runs `upskill` in `cwd` with `REMOTE` mapped to the local upstream.
fn run(e: &Env, cwd: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = common::upskill_cmd(&e.home);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", e.home.join("empty-gitconfig"))
        .env("GIT_CONFIG_COUNT", "2")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}.insteadOf", e.upstream.display()),
        )
        .env("GIT_CONFIG_VALUE_0", format!("{REMOTE}.git"))
        .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_1", "always")
        // A regression that bypasses the rewrite fails locally, before any network use.
        .env("GIT_ALLOW_PROTOCOL", "file")
        .env("GIT_TERMINAL_PROMPT", "0");
    cmd.args(args).output().unwrap()
}

fn upstream_head(e: &Env) -> String {
    git_in(e, &["rev-parse", "HEAD"])
}

/// Writes `body` to `path` in the upstream, commits it and returns the new HEAD.
#[allow(dead_code)] // Used by the tests of later tasks.
fn advance(e: &Env, path: &str, body: &str) -> String {
    let file = e.upstream.join(path);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, body).unwrap();
    git_in(e, &["add", "."]);
    git_in(e, &["commit", "-m", "advance"]);
    upstream_head(e)
}

fn lock(root: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(root.join(".upskill-lock.json")).unwrap()).unwrap()
}

/// Every file under `root` except `.git`, with its bytes.
#[allow(dead_code)] // Used by the tests of later tasks.
fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_name() == ".git" {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                walk(root, &entry.path(), files);
            } else {
                files.insert(
                    entry.path().strip_prefix(root).unwrap().into(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

#[allow(dead_code)] // Used by the tests of later tasks.
fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn add_records_the_commit_of_a_git_source() {
    let e = env();
    let out = run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let head = upstream_head(&e);
    let lock = lock(&e.repo);
    let items = lock["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for item in items {
        assert_eq!(item["commit"], head.as_str(), "{item}");
    }
}

#[test]
fn add_records_no_commit_for_a_local_path_source() {
    let e = env();
    let out = run(
        &e,
        &e.repo,
        &["add", e.upstream.to_str().unwrap(), "--claude"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let lock = lock(&e.repo);
    let items = lock["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for item in items {
        assert!(item.get("commit").is_none(), "{item}");
    }
}
