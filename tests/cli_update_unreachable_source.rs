//! ATDD tests for the sources that `upskill update` skips (issue #551).
//!
//! `update` skips a source whose content cannot be obtained (a git clone
//! that fails, or a local path that does not exist, of the source or of a
//! dependency source that it requires), and then every source whose plan
//! writes an item that the lockfile records from a skipped source, repeated
//! until no further source is skipped. It reports each skipped source on
//! stderr with its label and the cause (also on the error path of a later
//! step), applies the other sources, and exits 1, in every mode. Every other
//! prepare error still stops the run before the first write. The tests also
//! pin the status lines, the closing summary, the `schema: 1` adoption with
//! a skipped source, and that `update <name>` fetches no unselected source.

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
    let alpha_lock_before = lock_item(&e, "alpha");

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(err.contains(OTHER_REMOTE), "stderr lacks the source: {err}");
    assert!(err.contains("could not be fetched"), "{err}");
    // The item of the skipped source has its own status line and is not
    // counted as changed.
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert!(stdout.contains("Updated 1 of 2 item(s)"), "{stdout}");
    let beta_line = stdout
        .lines()
        .find(|l| l.contains("beta"))
        .unwrap_or_default();
    assert!(
        beta_line.contains("skipped — source not reachable"),
        "{stdout}"
    );
    let alpha_lock = lock_item(&e, "alpha");
    assert_ne!(
        alpha_lock["hash"], alpha_lock_before["hash"],
        "hash unchanged"
    );
    assert_ne!(
        alpha_lock["commit"], alpha_lock_before["commit"],
        "commit unchanged"
    );
    let alpha = fs::read_to_string(skill_file(&e, "alpha")).unwrap();
    assert!(alpha.contains("Alpha two."), "{alpha}");
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
    let alpha = fs::read_to_string(skill_file(&e, "alpha")).unwrap();
    assert!(
        alpha.contains("Alpha two."),
        "the other source was not applied"
    );
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
    assert!(
        err.contains("git clone https://example.invalid/team/other.git"),
        "stderr lacks the root cause: {err}"
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
    assert!(
        err.contains("resolved to no items"),
        "stderr lacks the later error: {err}"
    );
    let skip_at = err.find("could not be fetched and was skipped").unwrap();
    let error_at = err
        .lines()
        .position(|l| l.starts_with("error:") && !l.contains("could not be fetched"))
        .map(|n| err.lines().take(n).map(|l| l.len() + 1).sum::<usize>())
        .expect("no error line of the later source");
    assert!(
        skip_at < error_at,
        "the skip line is not before the error: {err}"
    );
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
    assert_eq!(lock(&e)["schema"], 3);
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

/// An error of `prepare_install` that is not a fetch failure (a source
/// file that cannot be parsed) still stops the run before any write: the
/// git source, which sorts first and changed upstream, is not applied.
#[test]
fn a_non_fetch_prepare_error_still_stops_the_run_before_any_write() {
    let e = env();
    run_ok(&e, &["add", REMOTE, "--claude", "--project"]);
    let local = e.tmp.path().join("local-reg");
    write_skill(&local, "gamma", "Gamma one.", "");
    run_ok(
        &e,
        &["add", local.to_str().unwrap(), "--claude", "--project"],
    );
    fs::write(
        local.join("skills/gamma/SKILL.md"),
        "---\nschema: 1\nname: [gamma\n---\nbody\n",
    )
    .unwrap();
    write_skill(&e.upstream, "alpha", "Alpha two.", "");
    commit_all(&e, &e.upstream);
    let before = files_under(&e.proj);

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(!err.contains("could not be fetched"), "{err}");
    assert!(
        !err.contains("overlap"),
        "the error is not from the prepare step: {err}"
    );
    assert_eq!(files_under(&e.proj), before, "update changed a file");
}

/// One verdict per source (review of #555): the source `REMOTE` is skipped
/// because its dependency source `OTHER_REMOTE` cannot be fetched, but the
/// local source of `charlie` requires `delta` of `REMOTE` and fetches
/// `REMOTE` itself. That source is skipped too, so that `delta`, an item of
/// a skipped source, is not written.
#[test]
fn a_source_that_requires_an_item_of_a_skipped_source_is_skipped_too() {
    let e = env();
    let requires_beta =
        format!("requires:\n  skills: [{{ name: beta, source: {OTHER_REMOTE} }}]\n");
    write_skill(&e.upstream, "alpha", "Alpha one.", &requires_beta);
    write_skill(&e.upstream, "delta", "Delta one.", "");
    commit_all(&e, &e.upstream);
    run_ok(&e, &["add", REMOTE, "--claude", "--project"]);
    let local = e.tmp.path().join("local-reg");
    write_skill(
        &local,
        "charlie",
        "Charlie one.",
        &format!("requires:\n  skills: [{{ name: delta, source: {REMOTE} }}]\n"),
    );
    run_ok(
        &e,
        &["add", local.to_str().unwrap(), "--claude", "--project"],
    );
    write_skill(&e.upstream, "delta", "Delta two.", "");
    commit_all(&e, &e.upstream);
    fs::rename(&e.other, e.tmp.path().join("other-moved")).unwrap();
    let before = files_under(&e.proj);

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert_eq!(
        files_under(&e.proj),
        before,
        "an item of a skipped source was written"
    );
    let err = stderr(&out);
    assert!(
        err.contains(&format!("of the source `{REMOTE}`, which was skipped")),
        "stderr does not name the unreachable source: {err}"
    );
    assert!(
        err.contains(&format!(
            "source `{REMOTE}` could not be fetched and was skipped:"
        )),
        "{err}"
    );
    // The local source was fetched: its line does not say that it could not
    // be fetched.
    let local_line = err
        .lines()
        .find(|l| l.contains("local-reg"))
        .unwrap_or_else(|| panic!("no line for the local source: {err}"));
    assert!(
        local_line.contains(&format!(
            "` was skipped: it requires skill `delta` of the source `{REMOTE}`, which was skipped"
        )),
        "{local_line}"
    );
    assert!(!local_line.contains("could not be fetched"), "{local_line}");
}

/// A missing local-path source named in `requires` of an item skips the
/// source that requires it (`Resolver`), and `add` of such an item fails with
/// the same text.
#[test]
fn a_missing_local_dependency_of_an_item_skips_the_requiring_source() {
    let e = env();
    let dep = e.tmp.path().join("dep-reg");
    write_skill(&dep, "gamma", "Gamma one.", "");
    let requires = format!(
        "requires:\n  skills: [{{ name: gamma, source: {} }}]\n",
        dep.display()
    );
    write_skill(&e.upstream, "alpha", "Alpha one.", &requires);
    commit_all(&e, &e.upstream);
    run_ok(&e, &["add", REMOTE, "--claude", "--project"]);
    write_skill(&e.upstream, "alpha", "Alpha two.", &requires);
    commit_all(&e, &e.upstream);
    fs::remove_dir_all(&dep).unwrap();
    let before = files_under(&e.proj);

    let out = run(&e, &["update", "alpha", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(
        err.contains(&format!("source `{REMOTE}` could not be fetched")),
        "{err}"
    );
    assert!(err.contains("fetch requires source"), "{err}");
    assert!(
        err.contains(&format!("source path {} does not exist", dep.display())),
        "{err}"
    );
    assert_eq!(files_under(&e.proj), before, "update changed a file");

    // `add` of an item that requires a missing local-path source fails with
    // the same cause.
    let other_proj = e.tmp.path().join("other-proj");
    fs::create_dir_all(other_proj.join(".git")).unwrap();
    let mut cmd = common::upskill_cmd(&e.home);
    let out = cmd
        .current_dir(&other_proj)
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
        .args(["add", REMOTE, "--claude", "--project"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("fetch requires source")
            && err.contains(&format!("source path {} does not exist", dep.display())),
        "{err}"
    );
}

/// A missing local-path source named in `requires:` of a bundle skips the
/// bundle source (`BundleResolver`).
#[test]
fn a_missing_local_dependency_of_a_bundle_skips_the_bundle_source() {
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
    let skills = e.tmp.path().join("reg-meta");
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
            "schema: 1\nname: meta\ndescription: d\nitems:\n  skills:\n  - z\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    )
    .unwrap();
    let bundle = skills.join("bundles/meta.bundle.yaml");
    run_ok(
        &e,
        &["add", bundle.to_str().unwrap(), "--claude", "--project"],
    );
    fs::remove_dir_all(&reg_dep).unwrap();
    let before = files_under(&e.proj);

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(
        err.contains("meta.bundle.yaml` could not be fetched"),
        "{err}"
    );
    assert!(err.contains("fetch requires source"), "{err}");
    assert!(err.contains("does not exist"), "{err}");
    assert_eq!(files_under(&e.proj), before, "update changed a file");
}

/// A terminal run with a skipped source: the skip line, exit code 1, the
/// closing summary counts the prepared sources only and does not say that
/// everything is up to date.
#[cfg(unix)]
#[test]
fn a_terminal_run_reports_the_skipped_source() {
    let e = stage_two_sources_one_unreachable();
    let env = [
        ("GIT_CONFIG_NOSYSTEM", "1".to_string()),
        (
            "GIT_CONFIG_GLOBAL",
            e.home.join("empty-gitconfig").display().to_string(),
        ),
        ("GIT_CONFIG_COUNT", "3".to_string()),
        (
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}.insteadOf", e.upstream.display()),
        ),
        ("GIT_CONFIG_VALUE_0", format!("{REMOTE}.git")),
        (
            "GIT_CONFIG_KEY_1",
            format!("url.file://{}.insteadOf", e.other.display()),
        ),
        ("GIT_CONFIG_VALUE_1", format!("{OTHER_REMOTE}.git")),
        ("GIT_CONFIG_KEY_2", "protocol.file.allow".to_string()),
        ("GIT_CONFIG_VALUE_2", "always".to_string()),
        ("GIT_ALLOW_PROTOCOL", "file".to_string()),
        ("GIT_TERMINAL_PROMPT", "0".to_string()),
    ];

    let result = common::update_in_terminal_with_env(&e.proj, &e.home, &["--project"], "y\n", &env);

    assert_eq!(result["returncode"], 1, "{result}");
    let stdout = result["stdout"].as_str().unwrap();
    let stderr = result["stderr"].as_str().unwrap();
    assert!(
        stderr.contains(&format!("source `{OTHER_REMOTE}` could not be fetched")),
        "{stderr}"
    );
    assert!(
        stdout.contains("Updated 1 of 2 item(s) from 1 of 1 source(s). 1 source(s) skipped."),
        "{stdout}"
    );
    assert!(!stdout.contains("everything is up to date"), "{stdout}");
    let alpha = fs::read_to_string(skill_file(&e, "alpha")).unwrap();
    assert!(alpha.contains("Alpha two."), "{alpha}");

    // Every source unreachable: no source is applied, and the run still
    // does not say that everything is up to date.
    fs::rename(&e.upstream, e.tmp.path().join("upstream-moved")).unwrap();
    let result = common::update_in_terminal_with_env(&e.proj, &e.home, &["--project"], "", &env);
    assert_eq!(result["returncode"], 1, "{result}");
    let stdout = result["stdout"].as_str().unwrap();
    assert!(!stdout.contains("everything is up to date"), "{stdout}");
    assert!(stdout.contains("2 source(s) skipped."), "{stdout}");
}

/// The cascade layout. `REMOTE` holds `alpha` (requires `beta` of
/// `OTHER_REMOTE`) and the skill `delta`. `c-reg` holds `charlie` (no
/// requires) and `carl` (requires the skill `delta` of `REMOTE`). `d-reg`
/// holds `dave` (requires `charlie` of `c-reg` only), so the plan of `d-reg`
/// holds no item of `REMOTE`: it is a second-level dependent. `e-reg` holds the rule `delta`, which shares its name with
/// the skill `delta`. Everything changes upstream, then `OTHER_REMOTE` moves
/// away: `OTHER_REMOTE` and `REMOTE` cannot be fetched, `c-reg` and then
/// `d-reg` are skipped because they require an item of a skipped source,
/// and only `e-reg` is applied. Returns the environment and the three local
/// directories.
fn stage_cascade() -> (Env, PathBuf, PathBuf, PathBuf) {
    let e = env();
    let requires_beta =
        format!("requires:\n  skills: [{{ name: beta, source: {OTHER_REMOTE} }}]\n");
    write_skill(&e.upstream, "alpha", "Alpha one.", &requires_beta);
    write_skill(&e.upstream, "delta", "Delta one.", "");
    commit_all(&e, &e.upstream);
    let c = e.tmp.path().join("c-reg");
    let requires_delta = format!("requires:\n  skills: [{{ name: delta, source: {REMOTE} }}]\n");
    write_skill(&c, "charlie", "Charlie one.", "");
    write_skill(&c, "carl", "Carl one.", &requires_delta);
    let d = e.tmp.path().join("d-reg");
    let requires_charlie = format!(
        "requires:\n  skills: [{{ name: charlie, source: {} }}]\n",
        c.display()
    );
    write_skill(&d, "dave", "Dave one.", &requires_charlie);
    let r = e.tmp.path().join("e-reg");
    let write_rule = |body: &str| {
        let path = r.join("rules/delta/RULE.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            format!("---\nschema: 1\nname: delta\ndescription: d\n---\n\n{body}\n"),
        )
        .unwrap();
    };
    write_rule("Rule one.");
    run_ok(&e, &["add", REMOTE, "--claude", "--project"]);
    for dir in [&c, &d, &r] {
        run_ok(&e, &["add", dir.to_str().unwrap(), "--claude", "--project"]);
    }
    write_skill(&e.upstream, "delta", "Delta two.", "");
    commit_all(&e, &e.upstream);
    write_skill(&c, "charlie", "Charlie two.", "");
    write_skill(&c, "carl", "Carl two.", &requires_delta);
    write_skill(&d, "dave", "Dave two.", &requires_charlie);
    write_rule("Rule two.");
    fs::rename(&e.other, e.tmp.path().join("other-moved")).unwrap();
    (e, c, d, r)
}

/// The installed files of every skill of the cascade layout.
fn cascade_skill_files(e: &Env) -> Vec<Vec<u8>> {
    ["alpha", "beta", "carl", "charlie", "dave", "delta"]
        .iter()
        .map(|n| fs::read(skill_file(e, n)).unwrap())
        .collect()
}

/// The installed rule `delta`, whatever its file name for Claude.
fn rule_text(e: &Env) -> String {
    // Compare path components, not text: on Windows the separator is `\`.
    let rules = e.proj.join(".claude").join("rules");
    let mut out = String::new();
    for (path, bytes) in files_under(&e.proj) {
        let is_delta = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().contains("delta"))
            || path
                .parent()
                .and_then(|p| p.file_name())
                .is_some_and(|n| n == "delta");
        if path.starts_with(&rules) && is_delta {
            out.push_str(&String::from_utf8(bytes).unwrap());
        }
    }
    out
}

#[test]
fn a_cascade_two_levels_deep_skips_every_dependent_source() {
    let (e, c, d, _r) = stage_cascade();
    let skills_before = cascade_skill_files(&e);
    let charlie_lock = lock_item(&e, "charlie");
    let carl_lock = lock_item(&e, "carl");
    let dave_lock = lock_item(&e, "dave");

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    // Two levels: `c-reg` requires an item of `REMOTE`, and `d-reg` requires
    // an item of `c-reg`, which is known only after the first drop.
    assert_eq!(
        cascade_skill_files(&e),
        skills_before,
        "a skipped item was written"
    );
    assert_eq!(lock_item(&e, "charlie"), charlie_lock);
    assert_eq!(lock_item(&e, "carl"), carl_lock);
    assert_eq!(lock_item(&e, "dave"), dave_lock);
    let err = stderr(&out);
    for (dir, item, from) in [
        (&c, "delta", REMOTE.to_string()),
        (&d, "charlie", format!("{}", c.display())),
    ] {
        let line = err
            .lines()
            .find(|l| l.contains(&format!("source `local:{}` was skipped", dir.display())))
            .unwrap_or_else(|| panic!("no cascade line for {}: {err}", dir.display()));
        assert!(
            line.contains(&format!("it requires skill `{item}` of the source `")),
            "{line}"
        );
        assert!(line.contains(&from), "{line}");
    }
    // The rule `delta` shares its name with the skill `delta` of the skipped
    // source `REMOTE`, but it is another item: its source is applied.
    let rule = rule_text(&e);
    assert!(
        rule.contains("Rule two."),
        "the rule source was not applied: rule {rule:?}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        stderr(&out)
    );
    // Item lines and the item count.
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert!(stdout.contains("Updated 1 of 7 item(s)"), "{stdout}");
    for (name, status) in [
        ("charlie", "skipped — requires a source that was skipped"),
        ("carl", "skipped — requires a source that was skipped"),
        ("dave", "skipped — requires a source that was skipped"),
        ("alpha", "skipped — source not reachable"),
        ("beta", "skipped — source not reachable"),
    ] {
        let line = stdout
            .lines()
            .find(|l| l.contains(&format!(" {name} ")))
            .unwrap_or_else(|| panic!("no line for {name}: {stdout}"));
        assert!(line.contains(status), "{line}");
    }
}

#[test]
fn a_cascade_under_dry_run_writes_nothing_and_exits_1() {
    let (e, c, d, _r) = stage_cascade();
    let before = files_under(&e.proj);

    let out = run(&e, &["update", "--project", "--dry-run"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    for dir in [&c, &d] {
        assert!(
            stderr(&out).contains(&format!("source `local:{}` was skipped", dir.display())),
            "{}",
            stderr(&out)
        );
    }
    assert_eq!(files_under(&e.proj), before, "a dry run changed a file");
}

/// A terminal run with a cascade: the summary counts every skipped source as
/// skipped, and `q` at the only prepared source stops with no source left
/// (the count of sources is taken after the cascade).
#[cfg(unix)]
#[test]
fn a_terminal_run_with_a_cascade_counts_the_sources_after_the_cascade() {
    let (e, _c, _d, _r) = stage_cascade();
    let env = [
        ("GIT_CONFIG_NOSYSTEM", "1".to_string()),
        (
            "GIT_CONFIG_GLOBAL",
            e.home.join("empty-gitconfig").display().to_string(),
        ),
        ("GIT_CONFIG_COUNT", "3".to_string()),
        (
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}.insteadOf", e.upstream.display()),
        ),
        ("GIT_CONFIG_VALUE_0", format!("{REMOTE}.git")),
        (
            "GIT_CONFIG_KEY_1",
            format!("url.file://{}.insteadOf", e.other.display()),
        ),
        ("GIT_CONFIG_VALUE_1", format!("{OTHER_REMOTE}.git")),
        ("GIT_CONFIG_KEY_2", "protocol.file.allow".to_string()),
        ("GIT_CONFIG_VALUE_2", "always".to_string()),
        ("GIT_ALLOW_PROTOCOL", "file".to_string()),
        ("GIT_TERMINAL_PROMPT", "0".to_string()),
    ];

    let result = common::update_in_terminal_with_env(&e.proj, &e.home, &["--project"], "q\n", &env);

    assert_eq!(result["returncode"], 1, "{result}");
    let stdout = result["stdout"].as_str().unwrap();
    assert!(
        stdout.contains(
            "from 0 of 1 source(s). 1 source(s) not applied. 4 source(s) skipped. Stopped."
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("not examined"), "{stdout}");
}

/// A later step fails after a cascade skip (the whole-run overlap check, with
/// the layout of issue #547 added): the error path still prints the
/// "was skipped: it requires" line.
#[test]
fn a_cascade_skip_is_reported_when_a_later_step_fails() {
    let (e, c, _d, _r) = stage_cascade();
    let reg_dep = e.tmp.path().join("reg-dep");
    fs::create_dir_all(reg_dep.join("q")).unwrap();
    fs::write(
        reg_dep.join("q/SKILL.md"),
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
    fs::create_dir_all(skills.join("bundles")).unwrap();
    fs::write(
        skills.join("bundles/meta.bundle.yaml"),
        format!(
            "schema: 1\nname: meta\ndescription: d\nitems: {{}}\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    )
    .unwrap();
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
    let before = files_under(&e.proj);

    let out = run(&e, &["update", "--project", "--yes"]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(
        err.contains("installing would delete or overwrite the source"),
        "{err}"
    );
    let cascade = format!("source `local:{}` was skipped: it requires", c.display());
    assert!(err.contains(&cascade), "{err}");
    assert_eq!(files_under(&e.proj), before, "update changed a file");
}

/// A source skipped because it requires an item of a skipped source adds no
/// planned item to `<m>`: the new skill `echo` that its plan would install
/// is not counted (issue #506).
#[test]
fn a_source_skipped_for_a_skipped_requirement_adds_no_planned_item() {
    let e = env();
    let requires_beta =
        format!("requires:\n  skills: [{{ name: beta, source: {OTHER_REMOTE} }}]\n");
    write_skill(&e.upstream, "alpha", "Alpha one.", &requires_beta);
    write_skill(&e.upstream, "delta", "Delta one.", "");
    commit_all(&e, &e.upstream);
    run_ok(&e, &["add", REMOTE, "--claude", "--project"]);
    let local = e.tmp.path().join("local-reg");
    write_skill(
        &local,
        "charlie",
        "Charlie one.",
        &format!("requires:\n  skills: [{{ name: delta, source: {REMOTE} }}]\n"),
    );
    run_ok(
        &e,
        &["add", local.to_str().unwrap(), "--claude", "--project"],
    );
    let entries = lock(&e)["items"].as_array().unwrap().len();
    // `charlie` now also requires the new skill `echo` of its own source.
    write_skill(&local, "echo", "Echo one.", "");
    write_skill(
        &local,
        "charlie",
        "Charlie two.",
        &format!("requires:\n  skills: [echo, {{ name: delta, source: {REMOTE} }}]\n"),
    );
    write_skill(&e.upstream, "delta", "Delta two.", "");
    commit_all(&e, &e.upstream);
    fs::rename(&e.other, e.tmp.path().join("other-moved")).unwrap();

    let out = run(&e, &["update", "--project", "--yes"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains(&format!("Updated 0 of {entries} item(s)")),
        "{entries}: {stdout} {}",
        stderr(&out)
    );
}
