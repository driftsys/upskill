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
    command(e, cwd, args, upstream, None).output().unwrap()
}

/// As [`run`], with `XDG_CACHE_HOME` set to `xdg`.
fn run_xdg(e: &Env, cwd: &Path, args: &[&str], xdg: &str) -> std::process::Output {
    command(e, cwd, args, &e.upstream, Some(xdg))
        .output()
        .unwrap()
}

fn command(
    e: &Env,
    cwd: &Path,
    args: &[&str],
    upstream: &Path,
    xdg: Option<&str>,
) -> assert_cmd::Command {
    let mut cmd = common::upskill_cmd(&e.home);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    let xdg = match xdg {
        Some(value) => value.into(),
        None => e.home.parent().unwrap().join("cache").into_os_string(),
    };
    cmd.current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", e.home.join("empty-gitconfig"))
        .env("XDG_CACHE_HOME", xdg)
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
    cmd.args(args);
    cmd
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

/// As [`age_cache`], for the scope roots whose path ends with `suffix`.
fn age_scope(e: &Env, suffix: &str, secs: u64) {
    let path = cache_path(e);
    let mut cache: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let mut aged = 0;
    for (root, state) in cache["scopes"].as_object_mut().unwrap() {
        if root.ends_with(suffix) {
            let at = state["checked_at"].as_u64().unwrap();
            state["checked_at"] = (at - secs).into();
            aged += 1;
        }
    }
    assert_eq!(aged, 1, "no scope root ends with {suffix}: {cache}");
    fs::write(&path, serde_json::to_vec(&cache).unwrap()).unwrap();
}

/// The scope roots of the cache file.
fn cached_roots(e: &Env) -> Vec<String> {
    let cache: serde_json::Value =
        serde_json::from_slice(&fs::read(cache_path(e)).unwrap()).unwrap();
    cache["scopes"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

const GLOBAL_LINE: &str =
    "upskill: 2 updates available (global). Run \"upskill update --global\".\n";
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
fn notice_prints_a_set_again_after_the_updates_were_applied_in_between() {
    // A local path source has the identity `<kind>:<name>` without a commit,
    // so the second set equals the first one. Only the clearing of the
    // reported set lets the second notice print.
    let e = env();
    let upstream = e.upstream.to_str().unwrap().to_string();
    let added = run(&e, &e.repo, &["add", &upstream, "--claude"]);
    assert!(added.status.success(), "{}", stderr(&added));
    let line = "upskill: 1 update available (project). Run \"upskill update\".\n";
    write_skill(&e.upstream, "alpha", "Changed once.");
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), line);
    let applied = run(&e, &e.repo, &["update", "--yes"]);
    assert!(applied.status.success(), "{}", stderr(&applied));
    age_cache(&e, 86_400);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
    write_skill(&e.upstream, "alpha", "Changed twice.");
    age_cache(&e, 86_400);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), line);
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
    if fs::write(cache_dir.join("probe"), "").is_ok() {
        eprintln!("skipped: the process can write to a read-only directory (running as root)");
        return;
    }
    let out = run(&e, &e.repo, &["update", "--notice"]);
    // Nothing is stored, so the next session start checks and prints again.
    let again = run(&e, &e.repo, &["update", "--notice"]);
    fs::set_permissions(&cache_dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), PROJECT_LINE);
    assert_eq!(stderr(&out), "");
    assert_eq!(again.status.code(), Some(0));
    assert_eq!(stdout(&again), PROJECT_LINE);
    assert_eq!(stderr(&again), "");
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
    assert!(!cache_path(&e).exists());
}

/// Both scopes have a recorded, unreported set. Returns the environment.
fn two_scopes_with_unreported_sets() -> Env {
    let e = env();
    let project = run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    assert!(project.status.success(), "{}", stderr(&project));
    let global = run(&e, &e.outside, &["add", REMOTE, "--claude", "--global"]);
    assert!(global.status.success(), "{}", stderr(&global));
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let quiet = run(&e, &e.repo, &["--quiet", "update", "--notice"]);
    assert_eq!(stdout(&quiet), "");
    assert_eq!(cached_roots(&e).len(), 2);
    e
}

#[test]
fn notice_prints_nothing_for_both_scopes_when_the_lockfile_of_a_due_scope_cannot_be_parsed() {
    let e = two_scopes_with_unreported_sets();
    age_scope(&e, "/repo", 86_400);
    fs::write(e.repo.join(".upskill-lock.json"), "not json").unwrap();
    let before = fs::read(cache_path(&e)).unwrap();
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), "");
    assert_eq!(stderr(&out), "");
    assert_eq!(fs::read(cache_path(&e)).unwrap(), before);
}

#[test]
fn notice_does_not_read_the_lockfile_of_a_scope_that_is_not_due() {
    let e = two_scopes_with_unreported_sets();
    fs::write(e.repo.join(".upskill-lock.json"), "not json").unwrap();
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), format!("{PROJECT_LINE}{GLOBAL_LINE}"));
    assert_eq!(stderr(&out), "");
}

#[test]
fn notice_contacts_the_source_only_for_the_scopes_that_are_due() {
    let e = env();
    let project = run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    assert!(project.status.success(), "{}", stderr(&project));
    let global = run(&e, &e.outside, &["add", REMOTE, "--claude", "--global"]);
    assert!(global.status.success(), "{}", stderr(&global));
    // Both scopes are checked once, and nothing is available yet.
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
    assert_eq!(cached_roots(&e).len(), 2);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    age_scope(&e, "/home", 86_400);
    // Only the global scope is due. A check of the project scope would print
    // its line too.
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), GLOBAL_LINE);
}

#[test]
fn notice_keeps_the_reported_set_after_a_check_that_reached_no_source() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    assert_eq!(
        stdout(&run(&e, &e.repo, &["update", "--notice"])),
        PROJECT_LINE
    );
    age_cache(&e, 86_400);
    let missing = e.home.join("missing");
    let offline = run_mapped(&e, &e.repo, &["update", "--notice"], &missing);
    assert_eq!(stdout(&offline), "");
    age_cache(&e, 86_400);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
}

#[test]
fn notice_drops_the_cache_entry_of_a_root_that_lost_its_lockfile() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
    assert_eq!(cached_roots(&e).len(), 1);
    fs::remove_file(e.repo.join(".upskill-lock.json")).unwrap();
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(cached_roots(&e).is_empty());
}

#[test]
fn notice_drops_the_cache_entry_of_a_root_that_no_longer_has_a_lockfile() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
    let mut cache: serde_json::Value =
        serde_json::from_slice(&fs::read(cache_path(&e)).unwrap()).unwrap();
    cache["scopes"]["/nonexistent/removed-project"] =
        serde_json::json!({"checked_at": 1, "updates": ["skill:a@c1"], "reported": []});
    fs::write(cache_path(&e), serde_json::to_vec(&cache).unwrap()).unwrap();
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(out.status.code(), Some(0));
    let roots = cached_roots(&e);
    assert_eq!(roots.len(), 1, "{roots:?}");
    assert!(roots[0].ends_with("/repo"), "{roots:?}");
}

#[test]
fn notice_writes_nothing_to_stderr_when_the_home_directory_is_not_known() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let out = command(&e, &e.repo, &["update", "--notice"], &e.upstream, None)
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), PROJECT_LINE);
    assert_eq!(stderr(&out), "");
}

#[test]
fn notice_uses_no_cache_when_no_cache_directory_is_known() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let out = command(&e, &e.repo, &["update", "--notice"], &e.upstream, None)
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .env_remove("XDG_CACHE_HOME")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), "");
    assert_eq!(stderr(&out), "");
    // No file is written relative to the current directory.
    assert!(!e.repo.join(".cache").exists());
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
    let cache: serde_json::Value =
        serde_json::from_slice(&fs::read(cache_path(&e)).unwrap()).unwrap();
    let states: Vec<_> = cache["scopes"].as_object().unwrap().values().collect();
    assert_eq!(states.len(), 1, "{cache}");
    assert!(states[0]["checked_at"].as_u64().unwrap() > 0, "{cache}");
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
        &["update", "--notice", "--force"],
        &["update", "--notice", "--claude"],
        &["update", "--notice", "--copilot"],
        &["update", "--notice", "--vscode"],
        &["update", "--notice", "--opencode"],
        &["update", "--notice", "alpha"],
    ] {
        let out = run(&e, &e.repo, args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn notice_prints_the_project_line_before_the_global_line() {
    // The two scopes hold different numbers of items, so a line that is
    // printed with the state of the other scope is detected.
    let e = env();
    let project = run(&e, &e.repo, &["add", REMOTE, "alpha", "--claude"]);
    assert!(project.status.success(), "{}", stderr(&project));
    let global = run(&e, &e.outside, &["add", REMOTE, "--claude", "--global"]);
    assert!(global.status.success(), "{}", stderr(&global));
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(
        stdout(&out),
        format!("upskill: 1 update available (project). Run \"upskill update\".\n{GLOBAL_LINE}")
    );
}

#[test]
fn notice_in_a_repository_without_a_lockfile_reports_the_global_scope_only() {
    let e = env();
    let added = run(&e, &e.outside, &["add", REMOTE, "--claude", "--global"]);
    assert!(added.status.success(), "{}", stderr(&added));
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(
        stdout(&out),
        "upskill: 2 updates available (global). Run \"upskill update --global\".\n"
    );
    let cache = fs::read_to_string(cache_path(&e)).unwrap();
    assert!(!cache.contains("/repo\""), "{cache}");
    let parsed: serde_json::Value = serde_json::from_str(&cache).unwrap();
    assert_eq!(parsed["scopes"].as_object().unwrap().len(), 1, "{cache}");
}

#[test]
fn notice_ignores_a_relative_or_empty_cache_directory_variable() {
    for xdg in ["relative-cache", ""] {
        let e = env();
        run(&e, &e.repo, &["add", REMOTE, "--claude"]);
        let out = run_xdg(&e, &e.repo, &["update", "--notice"], xdg);
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(stderr(&out), "");
        if !xdg.is_empty() {
            assert!(!e.repo.join(xdg).exists(), "{xdg}");
        }
        assert!(
            e.home.join(".cache/upskill/notice.json").is_file(),
            "{xdg:?}"
        );
    }
}

#[test]
fn notice_leaves_only_the_notice_file_in_the_cache_directory() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    assert_eq!(
        stdout(&run(&e, &e.repo, &["update", "--notice"])),
        PROJECT_LINE
    );
    let names: Vec<_> = fs::read_dir(cache_path(&e).parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["notice.json"]);
}

#[cfg(unix)]
#[test]
fn notice_does_not_rewrite_a_cache_file_that_has_no_change() {
    use std::os::unix::fs::MetadataExt;
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
    let bytes = fs::read(cache_path(&e)).unwrap();
    // A save writes a temporary file and renames it, which gives a new inode.
    let inode = fs::metadata(cache_path(&e)).unwrap().ino();
    let out = run(&e, &e.repo, &["update", "--notice"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(fs::metadata(cache_path(&e)).unwrap().ino(), inode);
    assert_eq!(fs::read(cache_path(&e)).unwrap(), bytes);
}

/// A second upstream with the skill `gamma`, reachable as `OTHER_REMOTE` only
/// when `map` is true.
const OTHER_REMOTE: &str = "https://example.invalid/team/other";

fn run_with_other(e: &Env, args: &[&str], other: &Path, map: bool) -> std::process::Output {
    let mut cmd = command(e, &e.repo, args, &e.upstream, None);
    if map {
        cmd.env("GIT_CONFIG_COUNT", "3")
            .env(
                "GIT_CONFIG_KEY_2",
                format!("url.file://{}.insteadOf", other.display()),
            )
            .env("GIT_CONFIG_VALUE_2", format!("{OTHER_REMOTE}.git"));
    }
    cmd.output().unwrap()
}

#[test]
fn notice_keeps_the_earlier_updates_of_a_source_that_was_not_reachable_in_a_later_check() {
    let e = env();
    let other = e.home.parent().unwrap().join("upstream2");
    fs::create_dir_all(&other).unwrap();
    git_in_dir(&e, &other, &["init", "--initial-branch=main"]);
    write_skill(&other, "gamma", "Gamma content.");
    git_in_dir(&e, &other, &["add", "."]);
    git_in_dir(&e, &other, &["commit", "-m", "initial"]);
    for source in [REMOTE, OTHER_REMOTE] {
        let added = run_with_other(&e, &["add", source, "--claude"], &other, true);
        assert!(added.status.success(), "{}", stderr(&added));
    }
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    write_skill(&other, "gamma", "Gamma changed.");
    git_in_dir(&e, &other, &["commit", "-am", "advance"]);
    let first = run_with_other(&e, &["update", "--notice"], &other, true);
    assert_eq!(
        stdout(&first),
        "upskill: 3 updates available (project). Run \"upskill update\".\n"
    );
    // The second source is not reachable now, and the first one moved on.
    advance(&e, "NOTES.md", "another commit\n");
    age_cache(&e, 86_400);
    let second = run_with_other(&e, &["update", "--notice"], &other, false);
    assert_eq!(second.status.code(), Some(0));
    // The update of `gamma` that the first check found stays in the count.
    assert_eq!(
        stdout(&second),
        "upskill: 3 updates available (project). Run \"upskill update\".\n"
    );
}

/// The command as a standard command, to control its stdio and its process.
#[cfg(unix)]
fn standard_command(built: &assert_cmd::Command, cwd: &Path) -> std::process::Command {
    let mut std_cmd = std::process::Command::new(built.get_program());
    std_cmd.args(built.get_args());
    for (key, value) in built.get_envs() {
        match value {
            Some(value) => std_cmd.env(key, value),
            None => std_cmd.env_remove(key),
        };
    }
    std_cmd.current_dir(cwd);
    std_cmd
}

/// A notice run that waits inside its first `git` call, and that call.
#[cfg(unix)]
struct BlockedRun {
    child: std::process::Child,
    git_pid: String,
}

#[cfg(unix)]
impl BlockedRun {
    /// Starts `update --notice` in `cwd` with a `git` that records its process
    /// id and then sleeps. Returns when that `git` has started.
    fn start(e: &Env, cwd: &Path) -> BlockedRun {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Stdio;
        let base = e.home.parent().unwrap();
        let bin = base.join("blocking-bin");
        fs::create_dir_all(&bin).unwrap();
        let marker = base.join(format!("git-started-{}", cwd.display().to_string().len()));
        let script = format!(
            "#!/bin/sh\necho $$ > '{m}.tmp'\nmv '{m}.tmp' '{m}'\nexec sleep 60\n",
            m = marker.display()
        );
        fs::write(bin.join("git"), script).unwrap();
        fs::set_permissions(bin.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
        let built = command(e, cwd, &["update", "--notice"], &e.upstream, None);
        let mut std_cmd = standard_command(&built, cwd);
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut dirs = vec![bin];
        dirs.extend(std::env::split_paths(&path));
        std_cmd.env("PATH", std::env::join_paths(dirs).unwrap());
        let mut child = std_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut git_pid = None;
        for _ in 0..200 {
            if let Ok(pid) = fs::read_to_string(&marker) {
                git_pid = Some(pid.trim().to_string());
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let Some(git_pid) = git_pid else {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the command did not call git within 10 seconds");
        };
        BlockedRun { child, git_pid }
    }

    /// Ends the blocked `git` call, so that the lookup fails.
    fn release(&self) {
        Command::new("kill").arg(&self.git_pid).status().unwrap();
    }
}

#[cfg(unix)]
#[test]
fn notice_exits_with_code_0_and_prints_nothing_when_it_is_interrupted_during_a_lookup() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let mut blocked = BlockedRun::start(&e, &e.repo);
    let status = Command::new("kill")
        .args(["-INT", &blocked.child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success());
    // The `git` call stays blocked: the command must end because of the
    // interrupt, not because the lookup returned.
    let mut ended = false;
    for _ in 0..60 {
        if blocked.child.try_wait().unwrap().is_some() {
            ended = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    if !ended {
        let _ = blocked.child.kill();
    }
    blocked.release();
    let out = blocked.child.wait_with_output().unwrap();
    assert!(
        ended,
        "the command did not end within 3 seconds of the interrupt"
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
    assert_eq!(stderr(&out), "");
    // The command ended before its lookup: no cache file was written.
    assert!(!cache_path(&e).exists());
}

#[cfg(unix)]
#[test]
fn an_interrupted_notice_leaves_an_existing_cache_file_unchanged() {
    use std::os::unix::fs::MetadataExt;
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    assert_eq!(stdout(&run(&e, &e.repo, &["update", "--notice"])), "");
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    age_cache(&e, 86_400);
    let aged = fs::read(cache_path(&e)).unwrap();
    let aged_inode = fs::metadata(cache_path(&e)).unwrap().ino();
    let mut blocked = BlockedRun::start(&e, &e.repo);
    let status = Command::new("kill")
        .args(["-INT", &blocked.child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success());
    let mut ended = false;
    for _ in 0..60 {
        if blocked.child.try_wait().unwrap().is_some() {
            ended = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    if !ended {
        let _ = blocked.child.kill();
    }
    blocked.release();
    let out = blocked.child.wait_with_output().unwrap();
    assert!(
        ended,
        "the command did not end within 3 seconds of the interrupt"
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
    assert_eq!(fs::read(cache_path(&e)).unwrap(), aged);
    assert_eq!(fs::metadata(cache_path(&e)).unwrap().ino(), aged_inode);
}

#[cfg(unix)]
#[test]
fn notice_runs_that_overlap_keep_the_cache_entries_of_each_other() {
    let e = env();
    let other = e.home.parent().unwrap().join("repo2");
    fs::create_dir_all(other.join(".git")).unwrap();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let added = run(&e, &other, &["add", REMOTE, "--claude"]);
    assert!(added.status.success(), "{}", stderr(&added));
    // The cache file holds an outdated entry of the other project.
    assert_eq!(stdout(&run(&e, &other, &["update", "--notice"])), "");
    let mut cache: serde_json::Value =
        serde_json::from_slice(&fs::read(cache_path(&e)).unwrap()).unwrap();
    let outdated =
        serde_json::json!({"checked_at": 1, "updates": ["skill:outdated@c0"], "reported": []});
    for (root, state) in cache["scopes"].as_object_mut().unwrap() {
        assert!(root.ends_with("/repo2"), "{root}");
        *state = outdated.clone();
    }
    fs::write(cache_path(&e), serde_json::to_vec(&cache).unwrap()).unwrap();
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    // The first run loaded that cache and waits in its lookup.
    let mut blocked = BlockedRun::start(&e, &e.repo);
    let second = run(&e, &other, &["update", "--notice"]);
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(stdout(&second), PROJECT_LINE);
    blocked.release();
    let status = blocked.child.wait().unwrap();
    assert_eq!(status.code(), Some(0));
    let cache: serde_json::Value =
        serde_json::from_slice(&fs::read(cache_path(&e)).unwrap()).unwrap();
    let states = cache["scopes"].as_object().unwrap();
    assert_eq!(states.len(), 2, "{cache}");
    let (_, other_state) = states.iter().find(|(r, _)| r.ends_with("/repo2")).unwrap();
    // The newer entry of the second run stays; the first run did not write
    // back the outdated one.
    assert!(other_state["checked_at"].as_u64().unwrap() > 1, "{cache}");
    assert_ne!(other_state["updates"], outdated["updates"], "{cache}");
}

#[cfg(unix)]
#[test]
fn notice_exits_with_code_0_and_writes_nothing_to_stderr_when_stdout_is_closed() {
    use std::process::Stdio;
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "skills/alpha/SKILL.md", CHANGED_ALPHA);
    let built = command(&e, &e.repo, &["update", "--notice"], &e.upstream, None);
    let mut std_cmd = standard_command(&built, &e.repo);
    let mut child = std_cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Closing the read end before the command prints gives it a closed pipe.
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stderr(&out), "");
}
