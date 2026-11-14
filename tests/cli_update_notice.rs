//! ATDD tests for `update --notice`: the session notice of available updates.
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

/// Runs `upskill` in `cwd` with `REMOTE` mapped to the local upstream and
/// `XDG_CACHE_HOME` set to `<tmp>/cache`.
fn run(e: &Env, cwd: &Path, args: &[&str]) -> std::process::Output {
    run_mapped(e, cwd, args, &e.upstream)
}

fn run_mapped(e: &Env, cwd: &Path, args: &[&str], upstream: &Path) -> std::process::Output {
    let mut cmd = common::upskill_cmd(&e.home);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", e.home.join("empty-gitconfig"))
        .env("XDG_CACHE_HOME", e.home.parent().unwrap().join("cache"))
        .env("GIT_CONFIG_COUNT", "2")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}.insteadOf", upstream.display()),
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
fn advance(e: &Env, path: &str, body: &str) -> String {
    let file = e.upstream.join(path);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, body).unwrap();
    git_in(e, &["add", "."]);
    git_in(e, &["commit", "-m", "advance"]);
    upstream_head(e)
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

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn cache_path(e: &Env) -> PathBuf {
    e.home.parent().unwrap().join("cache/upskill/notice.json")
}

/// Subtracts `secs` from every `checked_at` of the cache file.
fn age_cache(e: &Env, secs: u64) {
    let path = cache_path(e);
    let mut cache: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for state in cache["scopes"].as_object_mut().unwrap().values_mut() {
        let at = state["checked_at"].as_u64().unwrap();
        state["checked_at"] = (at - secs).into();
    }
    fs::write(&path, serde_json::to_vec(&cache).unwrap()).unwrap();
}

const PROJECT_LINE: &str = "upskill: 2 updates available (project). Run \"upskill update\".\n";
const CHANGED_ALPHA: &str =
    "---\nschema: 1\nname: alpha\ndescription: Use for testing the update check\n---\n\nnew\n";

#[test]
fn notice_prints_nothing_when_no_update_exists() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), "");
    assert_eq!(stderr(&out), "");
}

#[test]
fn notice_prints_one_line_once_for_a_set_of_updates() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let first = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(first.status.code(), Some(0));
    assert_eq!(stdout(&first), PROJECT_LINE);
    assert_eq!(stderr(&first), "");
    let second = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(stdout(&second), "");
}

#[test]
fn notice_does_not_contact_the_source_again_within_the_period() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
    age_cache(&e, 86_400);
    assert_eq!(
        stdout(&run(&e, &e.repo, &["update", "--notice"])),
        PROJECT_LINE
    );
}

#[test]
fn notice_prints_again_when_the_set_changes() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    assert_eq!(
        stdout(&run(&e, &e.repo, &["update", "--notice"])),
        PROJECT_LINE
    );
    advance(&e, "NOTES.md", "another commit\n");
    age_cache(&e, 86_400);
    assert_eq!(
        stdout(&run(&e, &e.repo, &["update", "--notice"])),
        PROJECT_LINE
    );
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
}

#[test]
fn notice_clears_the_reported_set_when_the_updates_are_applied() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    assert_eq!(
        stdout(&run(&e, &e.repo, &["update", "--notice"])),
        PROJECT_LINE
    );
    let applied = run(&e, &e.repo, &["update", "--yes"]);
    assert!(applied.status.success(), "{}", stderr(&applied));
    age_cache(&e, 86_400);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
    advance(&e, "NOTES.md", "another commit\n");
    age_cache(&e, 86_400);
    assert_eq!(
        stdout(&run(&e, &e.repo, &["update", "--notice"])),
        PROJECT_LINE
    );
}

#[test]
fn notice_prints_nothing_and_exits_with_code_0_when_the_source_is_unreachable() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let missing = e.home.join("missing");
    let out = run_mapped(&e, &e.repo, &["update", "--notice"], &missing);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), "");
    assert_eq!(stderr(&out), "");
}

#[test]
fn notice_waits_for_the_period_after_a_check_that_reached_no_source() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let missing = e.home.join("missing");
    run_mapped(&e, &e.repo, &["update", "--notice"], &missing);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
}

#[test]
fn notice_survives_an_invalid_cache_file() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    fs::create_dir_all(cache_path(&e).parent().unwrap()).unwrap();
    fs::write(cache_path(&e), "not json").unwrap();
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), PROJECT_LINE);
    assert_eq!(stderr(&out), "");
}

#[cfg(unix)]
#[test]
fn notice_survives_an_unwritable_cache_directory() {
    use std::os::unix::fs::PermissionsExt;
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let cache_dir = e.home.parent().unwrap().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::set_permissions(&cache_dir, fs::Permissions::from_mode(0o555)).unwrap();
    let out = run(&e, &e.repo, &["update", "--notice"]);
    fs::set_permissions(&cache_dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), PROJECT_LINE);
    assert_eq!(stderr(&out), "");
}

#[test]
fn notice_prints_nothing_when_a_lockfile_cannot_be_read() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    fs::write(e.repo.join(".upskill-lock.json"), "not json").unwrap();
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), "");
    assert_eq!(stderr(&out), "");
}

#[test]
fn notice_changes_no_file_outside_the_cache() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let repo_before = tree(&e.repo);
    let home_before = tree(&e.home);
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(stdout(&out), PROJECT_LINE);
    assert_eq!(tree(&e.repo), repo_before);
    assert_eq!(tree(&e.home), home_before);
    assert!(cache_path(&e).is_file());
}

#[test]
fn notice_reports_the_global_scope_with_its_own_line() {
    let e = env();
    let added = run(&e, &e.outside, &["add", REMOTE, "--claude", "--global"]);
    assert!(added.status.success(), "{}", stderr(&added));
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let out = run(&e, &e.outside, &["update", "--notice"]);
    assert_eq!(
        stdout(&out),
        "upskill: 2 updates available (global). Run \"upskill update --global\".\n"
    );
    assert_eq!(stderr(&out), "");
}

#[test]
fn notice_without_a_lockfile_prints_nothing_and_writes_no_cache_entry() {
    let e = env();
    let out = run(&e, &e.outside, &["update", "--notice"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), "");
    assert_eq!(stderr(&out), "");
    assert!(!cache_path(&e).exists());
}

#[test]
fn notice_prints_no_line_under_quiet_and_keeps_the_set_unreported() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let quiet = run(&e, &e.repo, &["--quiet", "update", "--notice"]);
    assert_eq!(quiet.status.code(), Some(0));
    assert_eq!(stdout(&quiet), "");
    assert_eq!(stderr(&quiet), "");
    assert_eq!(
        stdout(&run(&e, &e.repo, &["update", "--notice"])),
        PROJECT_LINE
    );
}

#[test]
fn notice_refuses_other_flags_and_names() {
    let e = env();
    for args in [
        &["update", "--notice", "--check"][..],
        &["update", "--notice", "--global"],
        &["update", "--notice", "--project"],
        &["update", "--notice", "--yes"],
        &["update", "--notice", "--dry-run"],
        &["update", "--notice", "alpha"],
    ] {
        let out = run(&e, &e.repo, args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
}
