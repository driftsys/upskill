//! ATDD tests for `add` of a source directory that is itself one item
//! directory (issues #472 and #488).
mod common;

use std::fs;
use std::path::{Path, PathBuf};

/// `(tmp, home, project)`: a fake home and a project directory that holds
/// `.git`.
fn env() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    // Canonical, so that a path printed from the current directory matches.
    let base = fs::canonicalize(tmp.path()).unwrap();
    let home = base.join("home");
    let project = base.join("project");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(project.join(".git")).unwrap();
    (tmp, home, project)
}

fn write_skill(dir: &Path, name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Use for testing add of an item directory\n---\n\nBody.\n"),
    )
    .unwrap();
}

/// Every file under `dir`, relative to `dir`, sorted.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.push(path.strip_prefix(base).unwrap().to_path_buf());
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn stderr_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stderr.clone()).unwrap()
}

/// R1: `.` alone is a local path, the same as `./`.
#[test]
fn add_dot_is_a_local_path() {
    let (_tmp, home, project) = env();
    write_skill(&project.join("skills/foo"), "foo");
    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", ".", "--project", "--claude"])
        .assert()
        .success();
    assert!(project.join(".claude/skills/foo/SKILL.md").is_file());
}

/// R2: a local source directory that holds an entrypoint at its root is
/// refused, with every spelling of the path, and nothing is written.
#[test]
fn add_of_an_item_directory_is_refused_and_writes_nothing() {
    let (_tmp, home, project) = env();
    let parent = project.join("reg");
    let item = parent.join("foo");
    write_skill(&item, "foo");
    let before = files_under(&project);
    let item_str = item.to_str().unwrap().to_string();
    for (cwd, spelling) in [(&project, item_str.as_str()), (&item, "./"), (&item, ".")] {
        let assert = common::upskill_cmd(&home)
            .current_dir(cwd)
            .args(["add", spelling, "--project", "--claude"])
            .assert()
            .code(1);
        let stderr = stderr_of(&assert);
        assert!(
            stderr.contains("is an item directory (it holds SKILL.md)"),
            "{spelling}: {stderr}"
        );
        assert!(
            stderr.contains(&format!("upskill add {} foo", parent.display())),
            "{spelling}: stderr does not name the command to use: {stderr}"
        );
        assert!(
            stderr.contains(&format!("source {}", item.display())),
            "{spelling}: stderr does not name the path: {stderr}"
        );
        assert_eq!(
            files_under(&project),
            before,
            "{spelling}: files were written"
        );
    }
}

/// R3: a single-item source that is itself an output path is refused, the
/// path is named and nothing is written.
#[test]
fn add_of_an_installed_output_item_directory_is_refused() {
    let (tmp, home, _project) = env();
    let item = home.join(".claude/skills/x");
    write_skill(&item, "x");
    let before = files_under(&home);
    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", item.to_str().unwrap(), "--global", "--claude"])
        .assert()
        .code(1);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains(&format!("source {} is an item directory", item.display())),
        "{stderr}"
    );
    assert_eq!(files_under(&home), before);
}

/// R4: when an output path is a hand-made symbolic link into the source
/// root, the error says so and does not claim that the source is rooted in a
/// client directory.
#[cfg(unix)]
#[test]
fn overlap_through_a_symbolic_link_names_the_link() {
    let (tmp, home, project) = env();
    let registry = tmp.path().join("registry");
    write_skill(&registry.join("skills/x"), "x");
    fs::create_dir_all(project.join(".claude/skills")).unwrap();
    let link = project.join(".claude/skills/x");
    std::os::unix::fs::symlink(registry.join("skills/x"), &link).unwrap();
    let before = files_under(&registry);
    let assert = common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap(), "--project", "--claude"])
        .assert()
        .code(1);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains(&format!(
            "The output path {} under the client directory {} is or passes through a \
             symbolic link into the source root {}",
            fs::canonicalize(&project)
                .unwrap()
                .join(".claude/skills/x")
                .display(),
            fs::canonicalize(&project)
                .unwrap()
                .join(".claude")
                .display(),
            fs::canonicalize(&registry).unwrap().display()
        )),
        "{stderr}"
    );
    assert!(stderr.contains("Remove the symbolic link"), "{stderr}");
    assert!(
        !stderr.contains("A source must not be rooted in a client directory"),
        "{stderr}"
    );
    assert_eq!(files_under(&registry), before);
    assert!(!project.join(".upskill-lock.json").exists());
}
