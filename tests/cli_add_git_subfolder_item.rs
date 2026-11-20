//! ATDD tests for `add` of a git source whose subfolder is itself one item
//! directory (issue #577). The source repositories are local; git maps the
//! https URLs to them with `insteadOf`, so the tests use no network.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REMOTE: &str = "https://example.invalid/team/skills";
/// A repository whose root is one item directory.
const ROOT_REMOTE: &str = "https://example.invalid/team/root";

struct Env {
    _tmp: tempfile::TempDir,
    home: PathBuf,
    proj: PathBuf,
    upstream: PathBuf,
    root_repo: PathBuf,
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

fn write_skill(dir: &Path, name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Use for testing add of an item directory\n---\n\nBody.\n"),
    )
    .unwrap();
}

/// The upstream holds `skills/foo/SKILL.md`, `bar/SKILL.md`,
/// `nested/SKILL.md`, `nested/baz/SKILL.md`, `rules/r-dir/RULE.md` (name
/// `my-rule`) and `bundled/SKILL.md` with `bundled/meta.bundle.yaml`,
/// committed on `main` and tagged `v1`. The root repository holds
/// `SKILL.md` at its root.
fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let mut base = fs::canonicalize(tmp.path()).unwrap();
    if cfg!(windows) {
        let text = base.display().to_string();
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            base = PathBuf::from(rest);
        }
    }
    let home = base.join("home");
    let proj = base.join("proj");
    let upstream = base.join("upstream");
    let root_repo = base.join("root");
    for dir in [&home, &upstream, &root_repo, &proj.join(".git")] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(home.join("empty-gitconfig"), "").unwrap();
    git(&home, &upstream, &["init", "-q", "--initial-branch=main"]);
    write_skill(&upstream.join("skills").join("foo"), "foo");
    write_skill(&upstream.join("bar"), "bar");
    write_skill(&upstream.join("nested"), "nested");
    write_skill(&upstream.join("nested").join("baz"), "baz");
    let rule = upstream.join("rules").join("r-dir");
    fs::create_dir_all(&rule).unwrap();
    fs::write(
        rule.join("RULE.md"),
        "---\nname: my-rule\ndescription: Use for testing a rule name\n---\n\nBody.\n",
    )
    .unwrap();
    write_skill(&upstream.join("bundled"), "bundled");
    fs::write(
        upstream.join("bundled").join("meta.bundle.yaml"),
        "schema: 1\nname: meta\ndescription: d\nitems: {}\n",
    )
    .unwrap();
    git(&home, &upstream, &["add", "."]);
    git(&home, &upstream, &["commit", "-q", "-m", "init"]);
    git(&home, &upstream, &["tag", "v1"]);
    git(&home, &root_repo, &["init", "-q", "--initial-branch=main"]);
    write_skill(&root_repo, "root");
    git(&home, &root_repo, &["add", "."]);
    git(&home, &root_repo, &["commit", "-q", "-m", "init"]);
    Env {
        _tmp: tmp,
        home,
        proj,
        upstream,
        root_repo,
    }
}

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
        // Keys 0 and 2 map `REMOTE` and the github.com URL of `team/skills`
        // to the upstream, key 3 maps `ROOT_REMOTE` to the root repository.
        // Git rewrites a URL that starts with an `insteadOf` value.
        .env("GIT_CONFIG_COUNT", "4")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}.insteadOf", e.upstream.display()),
        )
        .env("GIT_CONFIG_VALUE_0", format!("{REMOTE}.git"))
        .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_1", "always")
        .env(
            "GIT_CONFIG_KEY_2",
            format!("url.file://{}.insteadOf", e.upstream.display()),
        )
        .env("GIT_CONFIG_VALUE_2", "https://github.com/team/skills.git")
        .env(
            "GIT_CONFIG_KEY_3",
            format!("url.file://{}.insteadOf", e.root_repo.display()),
        )
        .env("GIT_CONFIG_VALUE_3", format!("{ROOT_REMOTE}.git"))
        .env("GIT_ALLOW_PROTOCOL", "file")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(args);
    cmd.output().unwrap()
}

/// Every entry under the project, other than `.git`.
fn written(e: &Env) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(&e.proj)
        .unwrap()
        .flatten()
        .map(|d| d.file_name().to_string_lossy().into_owned())
        .filter(|n| n != ".git")
        .collect();
    out.sort();
    out
}

fn assert_refused(e: &Env, source: &str, shown: &str, command: &str) {
    assert_refused_entry(e, source, shown, "SKILL.md", command);
}

fn assert_refused_entry(e: &Env, source: &str, shown: &str, entry: &str, command: &str) {
    let out = run(e, &["add", source, "--project", "--claude"]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "stderr: {err}");
    assert!(
        err.contains(&format!(
            "source {shown} is an item directory (it holds {entry})"
        )),
        "stderr: {err}"
    );
    assert!(
        err.contains(&format!("Use instead: {command}")),
        "stderr: {err}"
    );
    assert!(written(e).is_empty(), "written: {:?}", written(e));
}

#[test]
fn git_subfolder_item_is_refused_with_the_parent_subfolder() {
    let e = env();
    assert_refused(
        &e,
        &format!("{REMOTE}:skills/foo"),
        &format!("{REMOTE}:skills/foo"),
        &format!("upskill add {REMOTE}:skills foo"),
    );
}

#[test]
fn git_subfolder_item_with_a_ref_keeps_the_ref() {
    let e = env();
    assert_refused(
        &e,
        &format!("{REMOTE}@v1:skills/foo"),
        &format!("{REMOTE}@v1:skills/foo"),
        &format!("upskill add {REMOTE}@v1:skills foo"),
    );
}

#[test]
fn git_subfolder_item_at_the_repository_root_names_no_subfolder() {
    let e = env();
    assert_refused(
        &e,
        &format!("{REMOTE}:bar"),
        &format!("{REMOTE}:bar"),
        &format!("upskill add {REMOTE} bar"),
    );
}

#[test]
fn github_shorthand_subfolder_item_keeps_the_shorthand() {
    let e = env();
    assert_refused(
        &e,
        "team/skills@v1:skills/foo",
        "team/skills@v1:skills/foo",
        "upskill add team/skills@v1:skills foo",
    );
}

#[test]
fn git_subfolder_with_a_child_item_is_not_refused() {
    let e = env();
    let out = run(
        &e,
        &["add", &format!("{REMOTE}:nested"), "--project", "--claude"],
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        e.proj
            .join(".claude")
            .join("skills")
            .join("baz")
            .join("SKILL.md")
            .is_file()
    );
}

#[test]
fn the_suggested_command_installs_the_item() {
    let e = env();
    let out = run(
        &e,
        &[
            "add",
            &format!("{REMOTE}:skills"),
            "foo",
            "--project",
            "--claude",
        ],
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        e.proj
            .join(".claude")
            .join("skills")
            .join("foo")
            .join("SKILL.md")
            .is_file()
    );
}

#[test]
fn trailing_slash_subfolder_suggests_the_parent() {
    let e = env();
    assert_refused(
        &e,
        &format!("{REMOTE}:skills/foo/"),
        &format!("{REMOTE}:skills/foo/"),
        &format!("upskill add {REMOTE}:skills foo"),
    );
}

#[test]
fn dot_component_subfolder_suggests_the_normalised_parent() {
    let e = env();
    assert_refused(
        &e,
        &format!("{REMOTE}:skills/foo/."),
        &format!("{REMOTE}:skills/foo/."),
        &format!("upskill add {REMOTE}:skills foo"),
    );
}

#[test]
fn github_url_is_shown_as_the_shorthand() {
    let e = env();
    assert_refused(
        &e,
        "https://github.com/team/skills:skills/foo",
        "team/skills:skills/foo",
        "upskill add team/skills:skills foo",
    );
}

#[test]
fn rule_item_suggests_its_frontmatter_name() {
    let e = env();
    assert_refused_entry(
        &e,
        &format!("{REMOTE}:rules/r-dir"),
        &format!("{REMOTE}:rules/r-dir"),
        "RULE.md",
        &format!("upskill add {REMOTE}:rules my-rule"),
    );
}

#[test]
fn subfolder_item_with_a_bundle_file_is_not_refused() {
    let e = env();
    let out = run(
        &e,
        &["add", &format!("{REMOTE}:bundled"), "--project", "--claude"],
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {err}");
    assert!(!err.contains("is an item directory"), "stderr: {err}");
}

/// With an item name given, `prepare_install` fails first with its own
/// error; the refusal runs after it.
#[test]
fn subfolder_item_with_an_item_name_fails_with_no_matching_items() {
    let e = env();
    let out = run(
        &e,
        &[
            "add",
            &format!("{REMOTE}:skills/foo"),
            "foo",
            "--project",
            "--claude",
        ],
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "stderr: {err}");
    assert!(
        err.contains("no matching items or bundles"),
        "stderr: {err}"
    );
    assert!(written(&e).is_empty(), "written: {:?}", written(&e));
}

fn assert_root_refused(e: &Env, source: &str) {
    let out = run(e, &["add", source, "--project", "--claude"]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "stderr: {err}");
    assert!(
        err.contains(&format!(
            "source {source}: the repository root is one item directory (it holds SKILL.md)"
        )),
        "stderr: {err}"
    );
    assert!(
        err.contains(
            "upskill installs items from the directory that contains the item directories"
        ),
        "stderr: {err}"
    );
    assert!(!err.contains("Use instead"), "stderr: {err}");
    assert!(written(e).is_empty(), "written: {:?}", written(e));
}

#[test]
fn repository_root_item_is_refused() {
    let e = env();
    assert_root_refused(&e, ROOT_REMOTE);
}

#[test]
fn subfolder_that_normalises_to_the_root_is_refused_as_the_root() {
    let e = env();
    assert_root_refused(&e, &format!("{ROOT_REMOTE}:."));
}
