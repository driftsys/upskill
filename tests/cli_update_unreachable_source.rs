//! ATDD tests for a source that `upskill update` cannot fetch (issue #551).
//!
//! `update` skips a source whose content cannot be obtained (a git clone
//! that fails, or a local path that does not exist), reports it on stderr
//! with its label and the cause, applies the other sources, and exits 1.
//! Every other prepare error still stops the run before the first write.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REMOTE: &str = "https://example.invalid/team/skills";
const OTHER_REMOTE: &str = "https://example.invalid/team/other";

/// `home`, `proj` (holds `.git`, the project scope), `upstream` (the
/// repository behind `REMOTE`) and `other` (the repository behind
/// `OTHER_REMOTE`).
struct Env {
    tmp: tempfile::TempDir,
    home: PathBuf,
    proj: PathBuf,
    upstream: PathBuf,
    other: PathBuf,
}

fn git(home: &Path, dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    let output = cmd
        .current_dir(dir)
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", home.join("empty-gitconfig"))
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Write the skill `name` at `<root>/skills/<name>/SKILL.md` with `body` and
/// the frontmatter lines `extra`.
fn write_skill(root: &Path, name: &str, body: &str, extra: &str) {
    let path = root.join(format!("skills/{name}/SKILL.md"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!("---\nschema: 1\nname: {name}\ndescription: d\n{extra}---\n\n{body}\n"),
    )
    .unwrap();
}

fn commit_all(e: &Env, repo: &Path) {
    git(&e.home, repo, &["add", "."]);
    git(&e.home, repo, &["commit", "-q", "-m", "change"]);
}

/// The upstream holds the skill `alpha`, the other repository the skill
/// `beta`. Nothing is installed.
fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let proj = tmp.path().join("proj");
    let upstream = tmp.path().join("upstream");
    let other = tmp.path().join("other");
    for dir in [&home, &upstream, &other, &proj.join(".git")] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(home.join("empty-gitconfig"), "").unwrap();
    let e = Env {
        tmp,
        home,
        proj,
        upstream,
        other,
    };
    for repo in [&e.upstream, &e.other] {
        git(&e.home, repo, &["init", "-q", "--initial-branch=main"]);
    }
    write_skill(&e.upstream, "alpha", "Alpha one.", "");
    commit_all(&e, &e.upstream);
    write_skill(&e.other, "beta", "Beta one.", "");
    commit_all(&e, &e.other);
    e
}

/// Runs `upskill` in `proj` with `REMOTE` and `OTHER_REMOTE` mapped to the
/// local repositories. Git writes its trace to `<tmp>/git-trace`.
fn run(e: &Env, args: &[&str]) -> std::process::Output {
    let mut cmd = common::upskill_cmd(&e.home);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.current_dir(&e.proj)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", e.home.join("empty-gitconfig"))
        .env("GIT_CONFIG_COUNT", "3")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}.insteadOf", e.upstream.display()),
        )
        .env("GIT_CONFIG_VALUE_0", format!("{REMOTE}.git"))
        .env(
            "GIT_CONFIG_KEY_1",
            format!("url.file://{}.insteadOf", e.other.display()),
        )
        .env("GIT_CONFIG_VALUE_1", format!("{OTHER_REMOTE}.git"))
        .env("GIT_CONFIG_KEY_2", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_2", "always")
        .env("GIT_ALLOW_PROTOCOL", "file")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_TRACE", e.tmp.path().join("git-trace"))
        .args(args);
    cmd.output().unwrap()
}

fn run_ok(e: &Env, args: &[&str]) {
    let out = run(e, args);
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

/// Every file under `dir`, with its bytes, except the files under `.git`.
fn files_under(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                if path.file_name().is_some_and(|n| n == ".git") {
                    continue;
                }
                stack.push(path);
            } else if meta.is_file() {
                out.insert(path.clone(), fs::read(&path).unwrap());
            }
        }
    }
    out
}

fn lock(e: &Env) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(e.proj.join(".upskill-lock.json")).unwrap()).unwrap()
}

/// The lockfile entry of the item `name`.
fn lock_item(e: &Env, name: &str) -> serde_json::Value {
    lock(e)["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == name)
        .unwrap_or_else(|| panic!("no lockfile entry {name}"))
        .clone()
}

fn skill_file(e: &Env, name: &str) -> PathBuf {
    e.proj.join(format!(".claude/skills/{name}/SKILL.md"))
}

/// Both remotes installed; `alpha` changed upstream; the repository behind
/// `OTHER_REMOTE` moved away, so that it cannot be cloned.
fn stage_two_sources_one_unreachable() -> Env {
    let e = env();
    run_ok(&e, &["add", REMOTE, "--claude", "--project"]);
    run_ok(&e, &["add", OTHER_REMOTE, "--claude", "--project"]);
    write_skill(&e.upstream, "alpha", "Alpha two.", "");
    commit_all(&e, &e.upstream);
    fs::rename(&e.other, e.tmp.path().join("other-moved")).unwrap();
    e
}

#[test]
fn update_skips_a_source_that_cannot_be_fetched_and_applies_the_other() {
    let e = stage_two_sources_one_unreachable();
    let beta_before = fs::read(skill_file(&e, "beta")).unwrap();
    let beta_lock_before = lock_item(&e, "beta");

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(err.contains(OTHER_REMOTE), "stderr lacks the source: {err}");
    assert!(err.contains("could not be fetched"), "{err}");
    let alpha = fs::read_to_string(skill_file(&e, "alpha")).unwrap();
    assert!(alpha.contains("Alpha two."), "{alpha}");
    assert_ne!(
        lock_item(&e, "alpha")["hash"],
        serde_json::Value::Null,
        "alpha has no hash"
    );
    assert_eq!(fs::read(skill_file(&e, "beta")).unwrap(), beta_before);
    assert_eq!(lock_item(&e, "beta"), beta_lock_before);
}

#[test]
fn update_without_a_terminal_and_without_yes_skips_the_source() {
    let e = stage_two_sources_one_unreachable();
    let out = run(&e, &["update", "--project"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains(OTHER_REMOTE));
    let alpha = fs::read_to_string(skill_file(&e, "alpha")).unwrap();
    assert!(alpha.contains("Alpha two."), "{alpha}");
}

#[test]
fn update_quiet_still_reports_the_skipped_source() {
    let e = stage_two_sources_one_unreachable();
    let out = run(&e, &["update", "--project", "--yes", "--quiet"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains(OTHER_REMOTE), "{}", stderr(&out));
}

#[test]
fn update_dry_run_with_an_unreachable_source_exits_1_and_writes_nothing() {
    let e = stage_two_sources_one_unreachable();
    let before = files_under(&e.proj);

    let out = run(&e, &["update", "--project", "--dry-run"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains(OTHER_REMOTE), "{}", stderr(&out));
    // The reachable source is still previewed.
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert!(stdout.contains(REMOTE), "{stdout}");
    assert_eq!(files_under(&e.proj), before, "a dry run changed a file");
}

#[test]
fn update_with_every_source_unreachable_exits_1_and_writes_nothing() {
    let e = stage_two_sources_one_unreachable();
    fs::rename(&e.upstream, e.tmp.path().join("upstream-moved")).unwrap();
    let before = files_under(&e.proj);

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(err.contains(REMOTE) && err.contains(OTHER_REMOTE), "{err}");
    assert_eq!(files_under(&e.proj), before, "update changed a file");
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert!(!stdout.contains("up to date"), "{stdout}");
}

/// Acceptance criterion 3 of issue #551: `update <name>` does not fetch a
/// source that the names do not select. The git trace proves that no git
/// command named the unselected remote.
#[test]
fn update_of_a_name_does_not_fetch_an_unselected_source() {
    let e = stage_two_sources_one_unreachable();
    let trace = e.tmp.path().join("git-trace");
    let _ = fs::remove_file(&trace);

    let out = run(&e, &["update", "alpha", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(!stderr(&out).contains(OTHER_REMOTE), "{}", stderr(&out));
    let trace = fs::read_to_string(&trace).unwrap();
    assert!(trace.contains("team/skills"), "the trace is empty: {trace}");
    assert!(
        !trace.contains("team/other"),
        "the other remote was fetched"
    );
    let alpha = fs::read_to_string(skill_file(&e, "alpha")).unwrap();
    assert!(alpha.contains("Alpha two."), "{alpha}");
}

#[test]
fn update_skips_a_local_path_source_whose_directory_does_not_exist() {
    let e = env();
    let local = e.tmp.path().join("local-reg");
    write_skill(&local, "gamma", "Gamma one.", "");
    run_ok(
        &e,
        &["add", local.to_str().unwrap(), "--claude", "--project"],
    );
    run_ok(&e, &["add", REMOTE, "--claude", "--project"]);
    write_skill(&e.upstream, "alpha", "Alpha two.", "");
    commit_all(&e, &e.upstream);
    fs::remove_dir_all(&local).unwrap();
    let gamma_before = fs::read(skill_file(&e, "gamma")).unwrap();
    let gamma_lock_before = lock_item(&e, "gamma");

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(err.contains("local-reg"), "stderr lacks the source: {err}");
    assert!(err.contains("does not exist"), "{err}");
    // The overlap check of the run did not fail on the missing directory.
    assert!(!err.contains("overlap check"), "{err}");
    let alpha = fs::read_to_string(skill_file(&e, "alpha")).unwrap();
    assert!(alpha.contains("Alpha two."), "{alpha}");
    assert_eq!(fs::read(skill_file(&e, "gamma")).unwrap(), gamma_before);
    assert_eq!(lock_item(&e, "gamma"), gamma_lock_before);
}

#[test]
fn update_skips_a_source_whose_dependency_source_cannot_be_fetched() {
    let e = env();
    let requires = format!("requires:\n  skills: [{{ name: beta, source: {OTHER_REMOTE} }}]\n");
    write_skill(&e.upstream, "alpha", "Alpha one.", &requires);
    commit_all(&e, &e.upstream);
    run_ok(&e, &["add", REMOTE, "--claude", "--project"]);
    let local = e.tmp.path().join("local-reg");
    write_skill(&local, "gamma", "Gamma one.", "");
    run_ok(
        &e,
        &["add", local.to_str().unwrap(), "--claude", "--project"],
    );
    write_skill(&e.upstream, "alpha", "Alpha two.", &requires);
    commit_all(&e, &e.upstream);
    write_skill(&local, "gamma", "Gamma two.", "");
    fs::rename(&e.other, e.tmp.path().join("other-moved")).unwrap();
    let alpha_before = fs::read(skill_file(&e, "alpha")).unwrap();
    let alpha_lock_before = lock_item(&e, "alpha");

    // The source of `alpha` is selected; its dependency source cannot be
    // fetched. The source of `gamma` is selected too and must be applied.
    let out = run(&e, &["update", "alpha", "gamma", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(
        err.contains(REMOTE),
        "stderr lacks the skipped source: {err}"
    );
    assert!(
        err.contains(&format!("fetch requires source `{OTHER_REMOTE}`")),
        "stderr does not name the dependency source: {err}"
    );
    assert_eq!(fs::read(skill_file(&e, "alpha")).unwrap(), alpha_before);
    assert_eq!(lock_item(&e, "alpha"), alpha_lock_before);
    let gamma = fs::read_to_string(skill_file(&e, "gamma")).unwrap();
    assert!(
        gamma.contains("Gamma two."),
        "the run did not continue: {gamma}"
    );
}

/// A source is skipped, and a later source fails a check that is not a
/// fetch: the error path still reports the skipped source.
#[test]
fn update_reports_a_skipped_source_when_a_later_source_fails() {
    let e = env();
    run_ok(&e, &["add", REMOTE, "--claude", "--project"]);
    let local = e.tmp.path().join("local-reg");
    write_skill(&local, "gamma", "Gamma one.", "");
    run_ok(
        &e,
        &["add", local.to_str().unwrap(), "--claude", "--project"],
    );
    // The git label sorts before the `local:` label, so it is prepared first.
    fs::rename(&e.upstream, e.tmp.path().join("upstream-moved")).unwrap();
    // The local directory still exists, but no longer holds `gamma`.
    fs::remove_dir_all(local.join("skills")).unwrap();
    let before = files_under(&e.proj);

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(
        err.contains(&format!("source `{REMOTE}` could not be fetched")),
        "stderr lacks the skipped source: {err}"
    );
    assert!(err.contains("gamma"), "stderr lacks the later error: {err}");
    assert!(
        !err.contains("sources were skipped before this error"),
        "the marker text leaked: {err}"
    );
    assert_eq!(files_under(&e.proj), before, "update changed a file");
}

/// Rewrite the lockfile as schema 1 without `files` records.
fn downgrade_to_schema_1(e: &Env) {
    let path = e.proj.join(".upskill-lock.json");
    let mut lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    lock["schema"] = 1.into();
    for entry in lock["items"].as_array_mut().unwrap() {
        let entry = entry.as_object_mut().unwrap();
        entry.remove("files");
        entry.remove("pending");
    }
    fs::write(&path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();
}

/// The adoption of a `schema: 1` lockfile covers the whole lockfile: the
/// entry of a skipped source gets its `files` record from the files on
/// disk, as built. No other field of that entry changes, and its files are
/// not written.
#[test]
fn a_schema_1_adoption_with_a_skipped_source_records_its_files_only() {
    let e = stage_two_sources_one_unreachable();
    downgrade_to_schema_1(&e);
    let beta_before = fs::read(skill_file(&e, "beta")).unwrap();
    let mut beta_lock_before = lock_item(&e, "beta");

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains(OTHER_REMOTE), "{}", stderr(&out));
    assert_eq!(lock(&e)["schema"], 2);
    let alpha = fs::read_to_string(skill_file(&e, "alpha")).unwrap();
    assert!(alpha.contains("Alpha two."), "{alpha}");
    assert_eq!(fs::read(skill_file(&e, "beta")).unwrap(), beta_before);
    let mut beta_lock = lock_item(&e, "beta");
    let files = beta_lock.as_object_mut().unwrap().remove("files");
    assert!(
        files.is_some_and(|f| f.to_string().contains(".claude/skills/beta/SKILL.md")),
        "the adoption did not record the files of the skipped source"
    );
    beta_lock.as_object_mut().unwrap().remove("pending");
    beta_lock_before.as_object_mut().unwrap().remove("pending");
    assert_eq!(beta_lock, beta_lock_before);
}

/// The layout of issue #547 (see `tests/cli_update_source_overlap.rs`), with
/// the git source `REMOTE` installed as well and changed upstream.
#[test]
fn a_non_fetch_prepare_error_still_stops_the_run_before_any_write() {
    let e = env();
    let reg_dep = e.tmp.path().join("reg-dep");
    let q = reg_dep.join("q");
    fs::create_dir_all(&q).unwrap();
    fs::write(
        q.join("SKILL.md"),
        "---\nschema: 1\nname: q\ndescription: d\n---\n# q\n",
    )
    .unwrap();
    fs::create_dir_all(reg_dep.join("bundles")).unwrap();
    fs::write(
        reg_dep.join("bundles/dep.bundle.yaml"),
        "schema: 1\nname: dep\ndescription: d\nitems:\n  skills:\n  - q\n",
    )
    .unwrap();
    let skills = e.proj.join(".agents/skills");
    fs::create_dir_all(skills.join("z")).unwrap();
    fs::write(
        skills.join("z/SKILL.md"),
        "---\nschema: 1\nname: z\ndescription: d\n---\n# z\n",
    )
    .unwrap();
    fs::create_dir_all(skills.join("bundles")).unwrap();
    fs::write(
        skills.join("bundles/meta.bundle.yaml"),
        format!(
            "schema: 1\nname: meta\ndescription: d\nitems: {{}}\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    )
    .unwrap();
    run_ok(&e, &["add", REMOTE, "--claude", "--project"]);
    run_ok(
        &e,
        &[
            "add",
            "./.agents/skills/bundles/meta.bundle.yaml",
            "--claude",
            "--project",
        ],
    );
    fs::create_dir_all(skills.join("q/y")).unwrap();
    fs::write(
        skills.join("q/y/SKILL.md"),
        "---\nschema: 1\nname: y\ndescription: d\n---\n# y\n",
    )
    .unwrap();
    write_skill(&e.upstream, "alpha", "Alpha two.", "");
    commit_all(&e, &e.upstream);
    let before = files_under(&e.proj);

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(
        err.contains("installing would delete or overwrite the source"),
        "{err}"
    );
    assert!(!err.contains("could not be fetched"), "{err}");
    assert_eq!(files_under(&e.proj), before, "update changed a file");
}
