//! ATDD tests for the recorded source commit and `update --check`
//! (re-steer design, section 4).
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REMOTE: &str = "https://example.invalid/team/skills";
const OTHER_REMOTE: &str = "https://example.invalid/team/other";

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
    git_in_dir(e, &e.upstream, args)
}

fn git_in_dir(e: &Env, dir: &Path, args: &[&str]) -> String {
    let output = git_cmd(&e.home, dir).args(args).output().unwrap();
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
    run_mapped(e, cwd, args, None)
}

/// As [`run`], with `OTHER_REMOTE` mapped to the repository `other`.
fn run_with_other(e: &Env, other: &Path, args: &[&str]) -> std::process::Output {
    run_mapped(e, &e.repo, args, Some(other))
}

fn run_mapped(e: &Env, cwd: &Path, args: &[&str], other: Option<&Path>) -> std::process::Output {
    let mut cmd = common::upskill_cmd(&e.home);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", e.home.join("empty-gitconfig"))
        .env("GIT_CONFIG_COUNT", if other.is_some() { "3" } else { "2" })
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
    if let Some(other) = other {
        cmd.env(
            "GIT_CONFIG_KEY_2",
            format!("url.file://{}.insteadOf", other.display()),
        )
        .env("GIT_CONFIG_VALUE_2", format!("{OTHER_REMOTE}.git"));
    }
    cmd.args(args).output().unwrap()
}

fn upstream_head(e: &Env) -> String {
    git_in(e, &["rev-parse", "HEAD"])
}

/// Writes `body` to `path` in the upstream, commits it and returns the new HEAD.
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

#[test]
fn add_records_the_commit_of_each_source_of_a_cross_source_dependency() {
    let e = env();
    // A second upstream behind a second remote URL, holding the skill `gamma`.
    let other = e.upstream.parent().unwrap().join("other");
    fs::create_dir_all(&other).unwrap();
    git_in_dir(&e, &other, &["init", "--initial-branch=main"]);
    write_skill(&other, "gamma", "Gamma content.");
    git_in_dir(&e, &other, &["add", "."]);
    git_in_dir(&e, &other, &["commit", "-m", "other"]);
    // The commit differs from the first upstream's, which holds other content.
    advance(&e, "NOTES.md", "second commit in the first upstream\n");
    // `alpha` requires `gamma` from the second source.
    let alpha = e.upstream.join("skills/alpha/SKILL.md");
    fs::write(
        &alpha,
        format!(
            "---\nschema: 1\nname: alpha\ndescription: Use for testing the update check\nrequires:\n  skills: [{{ name: gamma, source: {OTHER_REMOTE} }}]\n---\n\nAlpha content.\n"
        ),
    )
    .unwrap();
    git_in(&e, &["add", "."]);
    git_in(&e, &["commit", "-m", "require gamma"]);

    let out = run_with_other(&e, &other, &["add", REMOTE, "alpha", "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));

    let head = upstream_head(&e);
    let other_head = git_in_dir(&e, &other, &["rev-parse", "HEAD"]);
    assert_ne!(head, other_head);
    let lock = lock(&e.repo);
    let items = lock["items"].as_array().unwrap();
    let commit_of = |name: &str| {
        items
            .iter()
            .find(|i| i["name"] == name)
            .unwrap_or_else(|| panic!("{name} not installed: {lock}"))["commit"]
            .clone()
    };
    assert_eq!(commit_of("alpha"), head.as_str());
    assert_eq!(commit_of("gamma"), other_head.as_str());
}

#[test]
fn update_records_a_new_commit_that_changes_no_item() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let head = advance(&e, "README.md", "unrelated\n");
    let before = tree(&e.repo.join(".claude"));
    let out = run(&e, &e.repo, &["update", "--yes"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(tree(&e.repo.join(".claude")), before);
    let lock = lock(&e.repo);
    let items = lock["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for item in items {
        assert_eq!(item["commit"], head.as_str());
    }
}

#[cfg(unix)]
#[test]
fn a_terminal_update_without_changes_records_the_commit_and_asks_nothing() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let head = advance(&e, "README.md", "unrelated\n");
    let env_vars = [
        ("GIT_CONFIG_NOSYSTEM", "1".to_string()),
        (
            "GIT_CONFIG_GLOBAL",
            e.home.join("empty-gitconfig").display().to_string(),
        ),
        ("GIT_CONFIG_COUNT", "2".to_string()),
        (
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}.insteadOf", e.upstream.display()),
        ),
        ("GIT_CONFIG_VALUE_0", format!("{REMOTE}.git")),
        ("GIT_CONFIG_KEY_1", "protocol.file.allow".to_string()),
        ("GIT_CONFIG_VALUE_1", "always".to_string()),
        ("GIT_ALLOW_PROTOCOL", "file".to_string()),
        ("GIT_TERMINAL_PROMPT", "0".to_string()),
    ];
    let result = common::update_in_terminal_with_env(&e.repo, &e.home, &[], "", &env_vars);
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        !result["stdout"].as_str().unwrap().contains("Apply?"),
        "{result}"
    );
    let lock = lock(&e.repo);
    let items = lock["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for item in items {
        assert_eq!(item["commit"], head.as_str());
    }
}

#[test]
fn update_keeps_an_edited_file_when_only_the_commit_changed() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let edited = e.repo.join(".claude/skills/alpha/SKILL.md");
    std::fs::write(&edited, "edited by the user\n").unwrap();
    let head = advance(&e, "README.md", "unrelated\n");
    let out = run(&e, &e.repo, &["update", "--yes"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(
        std::fs::read_to_string(&edited).unwrap(),
        "edited by the user\n"
    );
    assert!(!stderr(&out).contains("skipped"), "{}", stderr(&out));
    for item in lock(&e.repo)["items"].as_array().unwrap() {
        assert_eq!(item["commit"], head.as_str());
    }
}

#[test]
fn a_dry_run_records_no_commit() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let before = std::fs::read(e.repo.join(".upskill-lock.json")).unwrap();
    advance(
        &e,
        "skills/alpha/SKILL.md",
        "---\nschema: 1\nname: alpha\ndescription: changed\n---\nnew\n",
    );
    run(&e, &e.repo, &["update", "--dry-run"]);
    assert_eq!(
        std::fs::read(e.repo.join(".upskill-lock.json")).unwrap(),
        before
    );
}
