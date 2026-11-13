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

const OLD: [&str; 3] = [
    ".github/skills/create-api-endpoint/SKILL.md",
    ".github/instructions/api-conventions.instructions.md",
    ".github/agents/security-reviewer.agent.md",
];
const NEW: [&str; 3] = [
    ".copilot/skills/create-api-endpoint/SKILL.md",
    ".copilot/instructions/api-conventions.instructions.md",
    ".copilot/agents/security-reviewer.agent.md",
];

#[test]
fn global_add_writes_the_personal_copilot_directories() {
    let e = env();
    let out = run(
        &e,
        &e.repo,
        &["add", "--global", e.source.to_str().unwrap()],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    for rel in NEW {
        assert!(e.home.join(rel).is_file(), "{rel}");
    }
    for rel in OLD {
        assert!(!e.home.join(rel).exists(), "{rel}");
    }
    // Claude Code paths are the same in both scopes.
    assert!(
        e.home
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .is_file()
    );
    assert!(e.home.join(".claude/rules/api-conventions.md").is_file());
    assert!(e.home.join(".claude/agents/security-reviewer.md").is_file());
    let lock = fs::read_to_string(e.home.join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("\".copilot/skills/create-api-endpoint/SKILL.md\""));
    assert!(!lock.contains(".github/"));
}

#[test]
fn project_add_keeps_the_github_paths() {
    let e = env();
    let out = run(&e, &e.repo, &["add", e.source.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    for rel in OLD {
        assert!(e.repo.join(rel).is_file(), "{rel}");
    }
    assert!(!e.repo.join(".copilot").exists());
}

/// A global install as a released version left it: files at the `.github`
/// paths, recorded in a `schema: 2` lockfile.
fn released_global_install(e: &Env) {
    let out = run(
        e,
        &e.outside,
        &["add", "--global", e.source.to_str().unwrap()],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let lock_path = e.home.join(".upskill-lock.json");
    let mut lock = fs::read_to_string(&lock_path).unwrap();
    for (old, new) in OLD.iter().zip(NEW.iter()) {
        let from = e.home.join(new);
        let to = e.home.join(old);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::rename(&from, &to).unwrap();
        lock = lock.replace(new, old);
    }
    fs::write(&lock_path, lock).unwrap();
    fs::remove_dir_all(e.home.join(".copilot")).unwrap();
}

fn assert_moved(e: &Env) {
    for rel in NEW {
        assert!(e.home.join(rel).is_file(), "{rel}");
    }
    for rel in OLD {
        assert!(!e.home.join(rel).exists(), "{rel}");
    }
    // Client roots and kind directories are kept, so only the item
    // directory is asserted.
    assert!(
        !e.home.join(".github/skills/create-api-endpoint").exists(),
        "the old item directory is removed"
    );
}

#[test]
fn global_update_moves_a_released_install_without_force() {
    let e = env();
    released_global_install(&e);
    let out = run(&e, &e.outside, &["update", "--global", "--yes"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_moved(&e);
}

#[test]
fn global_update_moves_a_schema_1_install() {
    let e = env();
    released_global_install(&e);
    // Rewrite the lockfile to `schema: 1`: no `schema` marker of 2, no `files`.
    let lock_path = e.home.join(".upskill-lock.json");
    let mut json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    json["schema"] = serde_json::json!(1);
    for item in json["items"].as_array_mut().unwrap() {
        item.as_object_mut().unwrap().remove("files");
    }
    fs::write(&lock_path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    let out = run(&e, &e.outside, &["update", "--global", "--yes"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_moved(&e);
    assert!(e.home.join(".upskill-lock.json.v1").exists());
}

#[test]
fn global_update_keeps_an_edited_file_at_the_old_path() {
    let e = env();
    released_global_install(&e);
    fs::write(e.home.join(OLD[1]), "edited by the user\n").unwrap();
    let out = run(&e, &e.outside, &["update", "--global", "--yes"]);
    assert!(e.home.join(NEW[1]).is_file());
    assert_eq!(
        fs::read_to_string(e.home.join(OLD[1])).unwrap(),
        "edited by the user\n"
    );
    assert!(stderr(&out).contains(OLD[1]) || stdout(&out).contains(OLD[1]));
}

#[test]
fn global_update_previews_the_move_in_a_dry_run() {
    let e = env();
    released_global_install(&e);
    let out = run(&e, &e.outside, &["update", "--global", "--dry-run"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    for name in [
        "create-api-endpoint",
        "api-conventions",
        "security-reviewer",
    ] {
        assert!(text.contains(name), "{text}");
    }
    assert!(!text.contains("up to date"), "{text}");
    // The skill writes one file at the new path and deletes one file at the
    // old path; the other clients are unchanged.
    assert!(
        text.contains("update   skill create-api-endpoint   2 files"),
        "{text}"
    );
    for rel in NEW {
        assert!(!e.home.join(rel).exists(), "{rel}");
    }
    for rel in OLD {
        assert!(e.home.join(rel).is_file(), "{rel}");
    }
}
