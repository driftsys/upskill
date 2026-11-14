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
    // The prompt goes to stderr.
    for stream in ["stdout", "stderr"] {
        assert!(
            !result[stream].as_str().unwrap().contains("Apply?"),
            "{result}"
        );
    }
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

#[test]
fn update_keeps_the_recorded_commit_of_a_withheld_item() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let old = upstream_head(&e);
    std::fs::write(
        e.repo.join(".claude/skills/alpha/SKILL.md"),
        "edited by the user\n",
    )
    .unwrap();
    write_skill(&e.upstream, "alpha", "Alpha content, changed upstream.");
    git_in(&e, &["add", "."]);
    git_in(&e, &["commit", "-m", "change alpha"]);
    let head = upstream_head(&e);
    assert_ne!(old, head);
    let out = run(&e, &e.repo, &["update", "--yes"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("skipped"), "{}", stderr(&out));
    let lock = lock(&e.repo);
    let commit_of = |name: &str| {
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["name"] == name)
            .unwrap()["commit"]
            .clone()
    };
    // `alpha` is withheld and keeps its old commit. The installed content of
    // `beta` corresponds to the fetched source, so it holds the fetched commit.
    assert_eq!(commit_of("alpha"), old.as_str());
    assert_eq!(commit_of("beta"), head.as_str());
}

#[test]
fn a_dry_run_without_changes_records_no_commit() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let before = std::fs::read(e.repo.join(".upskill-lock.json")).unwrap();
    advance(&e, "README.md", "unrelated\n");
    let out = run(&e, &e.repo, &["update", "--dry-run"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        std::fs::read(e.repo.join(".upskill-lock.json")).unwrap(),
        before
    );
}

#[cfg(unix)]
#[test]
fn a_declined_source_records_no_commit() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let before = std::fs::read(e.repo.join(".upskill-lock.json")).unwrap();
    advance(
        &e,
        "skills/alpha/SKILL.md",
        "---\nschema: 1\nname: alpha\ndescription: changed\n---\nnew\n",
    );
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
    let result = common::update_in_terminal_with_env(&e.repo, &e.home, &[], "n\n", &env_vars);
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        result["stderr"].as_str().unwrap().contains("Apply?"),
        "{result}"
    );
    assert_eq!(
        std::fs::read(e.repo.join(".upskill-lock.json")).unwrap(),
        before
    );
}

#[cfg(unix)]
#[test]
fn a_terminal_update_keeps_the_recorded_commit_of_a_withheld_item() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let old = upstream_head(&e);
    let edited = e.repo.join(".claude/skills/alpha/SKILL.md");
    std::fs::write(&edited, "edited by the user\n").unwrap();
    write_skill(&e.upstream, "alpha", "Alpha content, changed upstream.");
    git_in(&e, &["add", "."]);
    git_in(&e, &["commit", "-m", "change alpha"]);
    let head = upstream_head(&e);
    assert_ne!(old, head);
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
    assert!(
        !result["stdout"].as_str().unwrap().contains("Apply?"),
        "{result}"
    );
    assert!(
        !result["stderr"].as_str().unwrap().contains("Apply?"),
        "{result}"
    );
    assert_eq!(
        std::fs::read_to_string(&edited).unwrap(),
        "edited by the user\n"
    );
    let lock = lock(&e.repo);
    let commit_of = |name: &str| {
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["name"] == name)
            .unwrap()["commit"]
            .clone()
    };
    assert_eq!(commit_of("alpha"), old.as_str(), "{result}");
    assert_eq!(commit_of("beta"), head.as_str(), "{result}");
}

/// The environment variables of `run_mapped`, for the terminal fixture.
#[cfg(unix)]
fn terminal_env(e: &Env, other: Option<&Path>) -> Vec<(&'static str, String)> {
    let mut vars = vec![
        ("GIT_CONFIG_NOSYSTEM", "1".to_string()),
        (
            "GIT_CONFIG_GLOBAL",
            e.home.join("empty-gitconfig").display().to_string(),
        ),
        (
            "GIT_CONFIG_COUNT",
            if other.is_some() { "3" } else { "2" }.to_string(),
        ),
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
    if let Some(other) = other {
        vars.push((
            "GIT_CONFIG_KEY_2",
            format!("url.file://{}.insteadOf", other.display()),
        ));
        vars.push(("GIT_CONFIG_VALUE_2", format!("{OTHER_REMOTE}.git")));
    }
    vars
}

/// The recorded commit of the item `name` in the lockfile at `root`.
fn commit_of(root: &Path, name: &str) -> serde_json::Value {
    lock(root)["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == name)
        .unwrap_or_else(|| panic!("{name} not in the lockfile"))["commit"]
        .clone()
}

#[cfg(unix)]
#[test]
fn a_terminal_update_of_one_item_keeps_the_commit_of_an_unselected_item() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let old = upstream_head(&e);
    // `beta` changes upstream; `alpha` does not, so the preview of
    // `update alpha` has no changes.
    write_skill(&e.upstream, "beta", "Beta content, changed upstream.");
    git_in(&e, &["add", "."]);
    git_in(&e, &["commit", "-m", "change beta"]);
    let head = upstream_head(&e);
    assert_ne!(old, head);

    let result = common::update_in_terminal_with_env(
        &e.repo,
        &e.home,
        &["alpha"],
        "",
        &terminal_env(&e, None),
    );
    assert_eq!(result["returncode"], 0, "{result}");
    assert_eq!(commit_of(&e.repo, "alpha"), head.as_str(), "{result}");
    assert_eq!(commit_of(&e.repo, "beta"), old.as_str(), "{result}");

    let out = run(&e, &e.repo, &["update", "--check", "--project"]);
    let text = stdout(&out);
    assert!(text.contains("update available"), "{text}");
    assert!(text.contains("    skill beta\n"), "{text}");

    // The status of a check with names comes from the named entries only.
    let out = run(&e, &e.repo, &["update", "--check", "--project", "alpha"]);
    let text = stdout(&out);
    assert!(text.contains("  up to date\n"), "{text}");
    assert!(!text.contains("update available"), "{text}");
}

fn short(commit: &str) -> &str {
    &commit[..7]
}

#[test]
fn check_reports_no_update_after_an_install() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let out = run(&e, &e.repo, &["update", "--check"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(stdout(&out).contains("  up to date\n"), "{}", stdout(&out));
    assert!(
        stdout(&out).ends_with("No updates available.\n"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_lists_the_items_of_a_source_with_a_new_commit_and_changes_no_file() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let old = upstream_head(&e);
    let new = advance(
        &e,
        "skills/alpha/SKILL.md",
        "---\nname: alpha\ndescription: changed\n---\nnew\n",
    );
    let before = (tree(&e.repo), tree(&e.home));
    let out = run(&e, &e.repo, &["update", "--check"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(&format!(
            "update available    {REMOTE}  {} -> {}",
            short(&old),
            short(&new)
        )),
        "{text}"
    );
    assert!(
        text.contains("    skill alpha\n") && text.contains("    skill beta\n"),
        "{text}"
    );
    assert!(
        text.ends_with("2 updates available. Run \"upskill update\".\n"),
        "{text}"
    );
    assert_eq!((tree(&e.repo), tree(&e.home)), before);
}

#[test]
fn check_reports_both_scopes_and_a_flag_limits_it() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let out = run(&e, &e.outside, &["add", REMOTE, "--global", "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    advance(&e, "NOTES.md", "new commit\n");

    let out = run(&e, &e.repo, &["update", "--check"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("project ("), "{text}");
    assert!(text.contains("global ("), "{text}");
    assert!(
        text.ends_with(
            "4 updates available. Run \"upskill update\" and \"upskill update --global\".\n"
        ),
        "{text}"
    );

    let out = run(&e, &e.repo, &["update", "--check", "--project"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("project ("), "{text}");
    assert!(!text.contains("global ("), "{text}");
    assert!(
        text.ends_with("2 updates available. Run \"upskill update\".\n"),
        "{text}"
    );

    let out = run(&e, &e.repo, &["update", "--check", "--global"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("project ("), "{text}");
    assert!(
        text.ends_with("2 updates available. Run \"upskill update --global\".\n"),
        "{text}"
    );
}

#[test]
fn check_says_nothing_installed_for_a_scope_without_a_source() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let out = run(&e, &e.repo, &["update", "--check"]);
    let text = stdout(&out);
    assert!(text.contains("global ("), "{text}");
    assert!(text.contains("  nothing installed\n"), "{text}");
}

#[test]
fn check_reports_an_unreachable_source_and_exits_with_code_0() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let gone = e.upstream.parent().unwrap().join("upstream-gone");
    fs::rename(&e.upstream, gone).unwrap();
    let out = run(&e, &e.repo, &["update", "--check"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.lines()
            .any(|l| l.starts_with("  not reachable       ") && l.contains(REMOTE)),
        "{text}"
    );
    assert!(text.ends_with("No updates available.\n"), "{text}");
}

#[test]
fn check_does_not_look_for_newer_tags() {
    let e = env();
    git_in(&e, &["tag", "v1"]);
    run(&e, &e.repo, &["add", &format!("{REMOTE}@v1"), "--claude"]);
    advance(&e, "NOTES.md", "second\n");
    git_in(&e, &["tag", "v2"]);
    let out = run(&e, &e.repo, &["update", "--check"]);
    assert!(stdout(&out).contains("  up to date\n"), "{}", stdout(&out));
    git_in(&e, &["tag", "-f", "v1"]);
    let out = run(&e, &e.repo, &["update", "--check"]);
    assert!(
        stdout(&out).contains("  update available    "),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_skips_a_source_pinned_to_a_commit() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let head = upstream_head(&e);
    let path = e.repo.join(".upskill-lock.json");
    let mut doc = lock(&e.repo);
    for item in doc["items"].as_array_mut().unwrap() {
        item["source"] = format!("{REMOTE}@{head}").into();
        item["ref"] = head.clone().into();
    }
    fs::write(&path, serde_json::to_vec_pretty(&doc).unwrap()).unwrap();
    advance(&e, "NOTES.md", "second\n");
    let out = run(&e, &e.repo, &["update", "--check", "--project"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("pinned to a commit"), "{text}");
    assert!(text.ends_with("No updates available.\n"), "{text}");
}

#[test]
fn check_of_a_local_path_source_lists_only_the_changed_item() {
    let e = env();
    let path = e.upstream.to_str().unwrap().to_string();
    let out = run(&e, &e.repo, &["add", &path, "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    write_skill(&e.upstream, "alpha", "Alpha content, edited.");
    let out = run(&e, &e.repo, &["update", "--check", "--project"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("    skill alpha\n"), "{text}");
    assert!(!text.contains("skill beta"), "{text}");
    assert!(!text.contains("->"), "{text}");
    assert!(
        text.ends_with("1 update available. Run \"upskill update\".\n"),
        "{text}"
    );
}

#[test]
fn check_after_an_update_reports_no_update() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "NOTES.md", "second\n");
    let out = run(&e, &e.repo, &["update", "--yes"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let out = run(&e, &e.repo, &["update", "--check", "--project"]);
    assert!(
        stdout(&out).ends_with("No updates available.\n"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_of_an_entry_without_a_commit_asks_for_one_update() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let mut doc = lock(&e.repo);
    for item in doc["items"].as_array_mut().unwrap() {
        item.as_object_mut().unwrap().remove("commit");
    }
    fs::write(
        e.repo.join(".upskill-lock.json"),
        serde_json::to_vec_pretty(&doc).unwrap(),
    )
    .unwrap();
    let out = run(&e, &e.repo, &["update", "--check", "--project"]);
    let text = stdout(&out);
    assert!(
        text.contains(&format!(
            "  no recorded commit  {REMOTE}: run \"upskill update\" once\n"
        )),
        "{text}"
    );
    assert!(text.ends_with("No updates available.\n"), "{text}");
}

#[test]
fn check_prints_nothing_to_stdout_with_quiet() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "NOTES.md", "second\n");
    let out = run(&e, &e.repo, &["update", "--check", "--quiet"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
}

#[test]
fn check_fails_with_code_1_for_an_unreadable_lockfile() {
    let e = env();
    fs::write(e.repo.join(".upskill-lock.json"), "not json").unwrap();
    let out = run(&e, &e.repo, &["update", "--check", "--project"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("lockfile"), "{}", stderr(&out));
}

#[test]
fn check_refuses_flags_that_apply_changes() {
    for flag in [
        "--dry-run",
        "--yes",
        "--force",
        "--claude",
        "--copilot",
        "--vscode",
        "--opencode",
    ] {
        let e = env();
        let out = run(&e, &e.repo, &["update", "--check", flag]);
        assert_eq!(out.status.code(), Some(2), "{flag}");
    }
}

#[test]
fn check_aligns_the_source_column_across_statuses() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let other = e.upstream.parent().unwrap().join("other");
    fs::create_dir_all(&other).unwrap();
    git_in_dir(&e, &other, &["init", "--initial-branch=main"]);
    write_skill(&other, "gamma", "Gamma content.");
    git_in_dir(&e, &other, &["add", "."]);
    git_in_dir(&e, &other, &["commit", "-m", "other"]);
    let out = run_with_other(&e, &other, &["add", OTHER_REMOTE, "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    advance(&e, "NOTES.md", "second\n");
    // The second source is not reachable: its repository is gone.
    fs::rename(&other, other.with_file_name("other-gone")).unwrap();
    let out = run_with_other(&e, &other, &["update", "--check", "--project"]);
    let text = stdout(&out);
    let offset = |label: &str, source: &str| {
        let line = text
            .lines()
            .find(|l| l.trim_start().starts_with(label))
            .unwrap_or_else(|| panic!("no {label} line: {text}"));
        line.find(source).unwrap()
    };
    assert_eq!(
        offset("update available", REMOTE),
        offset("not reachable", OTHER_REMOTE),
        "{text}"
    );
}

#[test]
fn check_with_an_item_name_covers_only_that_item() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    advance(&e, "NOTES.md", "second\n");
    let out = run(&e, &e.repo, &["update", "--check", "--project", "alpha"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("    skill alpha\n"), "{text}");
    assert!(!text.contains("skill beta"), "{text}");
    assert!(
        text.ends_with("1 update available. Run \"upskill update\".\n"),
        "{text}"
    );
}

#[test]
fn check_omits_the_up_to_date_sources_of_a_scope_that_has_an_update() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let other = e.upstream.parent().unwrap().join("other");
    fs::create_dir_all(&other).unwrap();
    git_in_dir(&e, &other, &["init", "--initial-branch=main"]);
    write_skill(&other, "gamma", "Gamma content.");
    git_in_dir(&e, &other, &["add", "."]);
    git_in_dir(&e, &other, &["commit", "-m", "other"]);
    let out = run_with_other(&e, &other, &["add", OTHER_REMOTE, "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    advance(&e, "NOTES.md", "second\n");
    let out = run_with_other(&e, &other, &["update", "--check", "--project"]);
    let text = stdout(&out);
    assert!(text.contains(REMOTE), "{text}");
    assert!(!text.contains(OTHER_REMOTE), "{text}");
    assert!(!text.contains("up to date"), "{text}");
}

#[test]
fn check_of_an_annotated_tag_compares_the_tagged_commit() {
    let e = env();
    git_in(&e, &["tag", "-a", "v1", "-m", "release one"]);
    let out = run(&e, &e.repo, &["add", &format!("{REMOTE}@v1"), "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    // The tag object differs from the commit it names.
    let out = run(&e, &e.repo, &["update", "--check", "--project"]);
    assert!(stdout(&out).contains("  up to date\n"), "{}", stdout(&out));
    advance(&e, "NOTES.md", "second\n");
    git_in(&e, &["tag", "-a", "-f", "v1", "-m", "release one, moved"]);
    let out = run(&e, &e.repo, &["update", "--check", "--project"]);
    assert!(
        stdout(&out).contains("  update available    "),
        "{}",
        stdout(&out)
    );
}

#[test]
fn update_records_the_commit_of_a_source_only_on_its_own_entries() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let other = e.upstream.parent().unwrap().join("other");
    fs::create_dir_all(&other).unwrap();
    git_in_dir(&e, &other, &["init", "--initial-branch=main"]);
    write_skill(&other, "gamma", "Gamma content.");
    git_in_dir(&e, &other, &["add", "."]);
    git_in_dir(&e, &other, &["commit", "-m", "other"]);
    let out = run_with_other(&e, &other, &["add", OTHER_REMOTE, "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let other_head = git_in_dir(&e, &other, &["rev-parse", "HEAD"]);
    // Only the first source gets a new commit, and no item changes.
    let head = advance(&e, "NOTES.md", "second\n");
    assert_ne!(head, other_head);
    let out = run_with_other(&e, &other, &["update", "--yes"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(commit_of(&e.repo, "alpha"), head.as_str());
    assert_eq!(commit_of(&e.repo, "beta"), head.as_str());
    assert_eq!(commit_of(&e.repo, "gamma"), other_head.as_str());
}

#[cfg(unix)]
#[test]
fn a_terminal_update_without_changes_records_the_commit_of_its_own_source_only() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let other = e.upstream.parent().unwrap().join("other");
    fs::create_dir_all(&other).unwrap();
    git_in_dir(&e, &other, &["init", "--initial-branch=main"]);
    write_skill(&other, "gamma", "Gamma content.");
    git_in_dir(&e, &other, &["add", "."]);
    git_in_dir(&e, &other, &["commit", "-m", "other"]);
    let out = run_with_other(&e, &other, &["add", OTHER_REMOTE, "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let other_head = git_in_dir(&e, &other, &["rev-parse", "HEAD"]);
    // Only the first source gets a new commit; no item changes, so the
    // terminal run asks nothing and takes the not-applied path.
    let head = advance(&e, "NOTES.md", "second\n");
    assert_ne!(head, other_head);
    let result = common::update_in_terminal_with_env(
        &e.repo,
        &e.home,
        &[],
        "",
        &terminal_env(&e, Some(&other)),
    );
    assert_eq!(result["returncode"], 0, "{result}");
    assert_eq!(commit_of(&e.repo, "alpha"), head.as_str(), "{result}");
    assert_eq!(commit_of(&e.repo, "beta"), head.as_str(), "{result}");
    assert_eq!(commit_of(&e.repo, "gamma"), other_head.as_str(), "{result}");
}

#[test]
fn check_prints_no_scope_line() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    run(&e, &e.outside, &["add", REMOTE, "--global", "--claude"]);
    for args in [
        &["update", "--check"][..],
        &["update", "--check", "--project"][..],
        &["update", "--check", "--global"][..],
    ] {
        let out = run(&e, &e.repo, args);
        assert_eq!(out.status.code(), Some(0), "{args:?}: {}", stderr(&out));
        assert!(
            !stderr(&out).contains("scope:"),
            "{args:?}: {}",
            stderr(&out)
        );
    }
    // Contrast: `update` prints the line.
    let out = run(&e, &e.repo, &["update", "--yes"]);
    assert!(
        stderr(&out).contains("scope: project ("),
        "{}",
        stderr(&out)
    );
}

#[test]
fn check_in_the_home_directory_reports_the_global_scope_and_refuses_project() {
    let e = env();
    let out = run(&e, &e.outside, &["add", REMOTE, "--global", "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    // The home directory is a git repository, the case in which `update`
    // refuses the project scope too; the check must report the global scope.
    fs::create_dir_all(e.home.join(".git")).unwrap();
    let out = run(&e, &e.home, &["update", "--check"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("global ("), "{text}");
    assert!(!text.contains("project ("), "{text}");
    let out = run(&e, &e.home, &["update", "--check", "--project"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stderr(&out).contains("pass --global"), "{}", stderr(&out));
}

#[test]
fn an_applied_update_with_a_name_records_the_commit_on_the_named_entry_only() {
    let e = env();
    run(&e, &e.repo, &["add", REMOTE, "--claude"]);
    let old = upstream_head(&e);
    write_skill(&e.upstream, "alpha", "Alpha content, changed upstream.");
    git_in(&e, &["add", "."]);
    git_in(&e, &["commit", "-m", "change alpha"]);
    let head = upstream_head(&e);
    assert_ne!(old, head);
    let out = run(&e, &e.repo, &["update", "--yes", "alpha"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(commit_of(&e.repo, "alpha"), head.as_str());
    // `beta` was not selected and is not reinstalled.
    assert_eq!(commit_of(&e.repo, "beta"), old.as_str());
}

#[test]
fn an_applied_update_records_the_commit_on_an_entry_that_a_named_item_requires() {
    let e = env();
    let skill = |body: &str| {
        format!(
            "---\nschema: 1\nname: alpha\ndescription: Use for testing the update check\nrequires:\n  skills: [beta]\n---\n\n{body}\n"
        )
    };
    let path = e.upstream.join("skills/alpha/SKILL.md");
    fs::write(&path, skill("Alpha content.")).unwrap();
    git_in(&e, &["add", "."]);
    git_in(&e, &["commit", "-m", "alpha requires beta"]);
    let out = run(&e, &e.repo, &["add", REMOTE, "alpha", "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let old = upstream_head(&e);
    assert_eq!(commit_of(&e.repo, "beta"), old.as_str());
    fs::write(&path, skill("Alpha content, changed upstream.")).unwrap();
    git_in(&e, &["add", "."]);
    git_in(&e, &["commit", "-m", "change alpha"]);
    let head = upstream_head(&e);
    let out = run(&e, &e.repo, &["update", "--yes", "alpha"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(commit_of(&e.repo, "alpha"), head.as_str());
    // `beta` is reinstalled through the `requires` closure of `alpha`.
    assert_eq!(commit_of(&e.repo, "beta"), head.as_str());
}

#[test]
fn an_applied_bundle_update_records_the_commit_on_every_member() {
    let e = env();
    fs::write(
        e.upstream.join("pack.bundle.yaml"),
        "schema: 1\nname: pack\ndescription: bundle for the update check\nitems:\n  skills:\n    - alpha\n    - beta\n",
    )
    .unwrap();
    git_in(&e, &["add", "."]);
    git_in(&e, &["commit", "-m", "bundle"]);
    let source = format!("{REMOTE}:pack.bundle.yaml");
    let out = run(&e, &e.repo, &["add", &source, "--claude"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let old = upstream_head(&e);
    assert_eq!(commit_of(&e.repo, "alpha"), old.as_str());
    assert_eq!(commit_of(&e.repo, "beta"), old.as_str());
    write_skill(&e.upstream, "alpha", "Alpha content, changed upstream.");
    git_in(&e, &["add", "."]);
    git_in(&e, &["commit", "-m", "change alpha"]);
    let head = upstream_head(&e);
    // Only `beta` is named; the bundle source is reinstalled as a whole.
    let out = run(&e, &e.repo, &["update", "--yes", "beta"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(commit_of(&e.repo, "alpha"), head.as_str());
    assert_eq!(commit_of(&e.repo, "beta"), head.as_str());
}
