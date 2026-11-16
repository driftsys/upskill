//! ATDD tests for the overlap check of `upskill update` across the sources
//! of the lockfile (issue #547).
//!
//! `update` refuses a run, before its first write, when the source directory
//! of an item is, is inside, or contains an output path of an item of the
//! run. The check compares every source that the run reads with each other,
//! and also the local-path sources recorded in the lockfile that the run does
//! not otherwise read.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Write the skill `name` at `<dir>/SKILL.md` with `body`.
fn write_skill(dir: &Path, name: &str, body: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nschema: 1\nname: {name}\ndescription: d\n---\n{body}\n"),
    )
    .unwrap();
}

/// Write a bundle manifest at `<root>/bundles/<name>.bundle.yaml`.
fn write_bundle(root: &Path, name: &str, body: &str) {
    let dir = root.join("bundles");
    fs::create_dir_all(&dir).unwrap();
    let content = format!("schema: 1\nname: {name}\ndescription: d\n{body}");
    fs::write(dir.join(format!("{name}.bundle.yaml")), content).unwrap();
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

fn add(home: &Path, target: &Path, source: &str) {
    common::upskill_cmd(home)
        .current_dir(target)
        .args(["add", source, "--claude", "--project"])
        .assert()
        .success();
}

/// Run `update --project --yes` with `extra` and assert that it exits 1
/// with an overlap message that contains each of `parts`, and that no file
/// under `target` changed.
fn assert_update_refused(home: &Path, target: &Path, extra: &[&str], parts: &[&str]) {
    let before = files_under(target);
    let mut args = vec!["update", "--project", "--yes"];
    args.extend_from_slice(extra);
    let assert = common::upskill_cmd(home)
        .current_dir(target)
        .args(&args)
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("installing would delete or overwrite the source"),
        "stderr does not explain the refusal: {stderr}"
    );
    for part in parts {
        assert!(stderr.contains(part), "stderr lacks {part:?}: {stderr}");
    }
    assert_eq!(files_under(target), before, "update changed a file");
}

/// The layout of issue #547. The dependency source `<tmp>/reg-dep` holds
/// the skill `q` and the bundle `dep`. The project `<tmp>/proj` holds the
/// skill `z` and the bundle `meta` in `.agents/skills`; `meta` selects no
/// item and requires `dep`. After `add` of `meta`, the skill `y` is created
/// at `.agents/skills/q/y`, inside the OpenCode output path of `q`, and the
/// body of `q` changes. Returns `(tmp, home, proj)`.
fn stage_issue_layout(with_z: bool) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let reg_dep = tmp.path().join("reg-dep");
    write_skill(&reg_dep.join("q"), "q", "# q");
    write_bundle(&reg_dep, "dep", "items:\n  skills:\n  - q\n");

    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    let skills = proj.join(".agents/skills");
    if with_z {
        write_skill(&skills.join("z"), "z", "# z");
    }
    write_bundle(
        &skills,
        "meta",
        &format!(
            "items: {{}}\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );
    add(&home, &proj, "./.agents/skills/bundles/meta.bundle.yaml");
    write_skill(&skills.join("q/y"), "y", "# y");
    write_skill(&reg_dep.join("q"), "q", "New body");
    (tmp, home, proj)
}

#[test]
fn update_refuses_an_overlap_with_the_source_of_the_bundle_that_pulled_a_dependency() {
    let (_tmp, home, proj) = stage_issue_layout(true);
    assert_update_refused(
        &home,
        &proj,
        &[],
        &["the source directory of skill 'y' (", "of skill 'q';"],
    );
}

#[test]
fn update_force_refuses_an_overlap_with_the_source_of_the_bundle_that_pulled_a_dependency() {
    let (_tmp, home, proj) = stage_issue_layout(true);
    assert_update_refused(
        &home,
        &proj,
        &["--force"],
        &["the source directory of skill 'y' (", "of skill 'q';"],
    );
}

#[test]
fn update_dry_run_refuses_an_overlap_with_the_source_of_the_bundle_that_pulled_a_dependency() {
    let (_tmp, home, proj) = stage_issue_layout(true);
    assert_update_refused(
        &home,
        &proj,
        &["--dry-run"],
        &["the source directory of skill 'y' (", "of skill 'q';"],
    );
}

#[test]
fn update_skips_a_recorded_local_bundle_source_that_no_longer_exists() {
    let (_tmp, home, proj) = stage_issue_layout(true);
    fs::remove_dir_all(proj.join(".agents")).unwrap();

    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["update", "--project", "--yes"])
        .assert()
        .success();
    let q = fs::read_to_string(proj.join(".claude/skills/q/SKILL.md")).unwrap();
    assert!(q.contains("New body"), "{q}");
}

/// Two lockfile sources: the skill `a` at `<proj>/<a_dir>` and the skill `b`
/// in the source `<tmp>/proj/.agents/skills/a`, so the directory of `b` is
/// inside the OpenCode output path of `a`. Both are installed for Claude
/// only, so `add` accepts each. Returns `(tmp, home, proj)`.
fn stage_two_sources(a_dir: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    write_skill(&proj.join(a_dir).join("a"), "a", "# a");
    write_skill(&proj.join(".agents/skills/a/b"), "b", "# b");
    add(&home, &proj, a_dir);
    add(&home, &proj, "./.agents/skills/a");
    (tmp, home, proj)
}

#[test]
fn update_refuses_an_overlap_between_two_lockfile_sources_in_both_orders() {
    // The labels are absolute. `<proj>/../aaa` sorts before
    // `<proj>/.agents/skills/a`, and `<proj>/src-a` sorts after it.
    for a_dir in ["../aaa", "./src-a"] {
        let (_tmp, home, proj) = stage_two_sources(a_dir);
        assert_update_refused(
            &home,
            &proj,
            &[],
            &["the source directory of skill 'b' (", "of skill 'a';"],
        );
    }
}

#[test]
fn update_refuses_an_overlap_with_a_dependency_source_read_for_a_lockfile_source() {
    // The bundle `meta` at `<tmp>/reg-meta` requires `dep` from the source
    // `<tmp>/proj/.agents/skills/a`, which holds the skill `d`. The lockfile
    // source `<tmp>/reg-a` holds the skill `a`, whose OpenCode output path
    // `.agents/skills/a` contains the directory of `d`.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    let reg_d = proj.join(".agents/skills/a");
    write_skill(&reg_d.join("d"), "d", "# d");
    write_bundle(&reg_d, "dep", "items:\n  skills:\n  - d\n");
    let reg_meta = tmp.path().join("reg-meta");
    write_skill(&reg_meta.join("m"), "m", "# m");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  skills:\n  - m\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_d.display()
        ),
    );
    write_skill(&tmp.path().join("reg-a/a"), "a", "# a");
    add(&home, &proj, "../reg-meta/bundles/meta.bundle.yaml");
    add(&home, &proj, "../reg-a");

    assert_update_refused(
        &home,
        &proj,
        &[],
        &["the source directory of skill 'd' (", "of skill 'a';"],
    );
}

#[test]
fn update_applies_no_source_when_a_later_source_fails_to_prepare() {
    // The labels sort `../aaa` before `../zzz`. The first source changes;
    // the second no longer exists, so its fetch fails.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    write_skill(&tmp.path().join("aaa/a"), "a", "# a");
    write_skill(&tmp.path().join("zzz/b"), "b", "# b");
    add(&home, &proj, "../aaa");
    add(&home, &proj, "../zzz");
    write_skill(&tmp.path().join("aaa/a"), "a", "Changed");
    fs::remove_dir_all(tmp.path().join("zzz")).unwrap();
    let before = files_under(&proj);

    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["update", "--project", "--yes"])
        .assert()
        .code(1);
    assert_eq!(files_under(&proj), before, "the first source was applied");
}

#[test]
fn add_accepts_a_bundle_file_whose_source_holds_no_item() {
    // The bundle `meta` selects no item and only declares `requires`; its
    // source directory `.agents/skills` holds no item.
    let (_tmp, _home, proj) = stage_issue_layout(false);
    assert!(proj.join(".claude/skills/q/SKILL.md").is_file());
}
