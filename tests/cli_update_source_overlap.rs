//! ATDD tests for the overlap check of `upskill update` across the sources
//! of the lockfile (issue #547).
//!
//! `update` refuses a run, before its first write, when the source directory
//! of an item is, is inside, or contains a path that the run writes for an
//! item that it installs. The check compares every source that the run reads
//! with each other, and also the local-path sources recorded in the lockfile
//! that the run does not otherwise read. The check also covers the recorded
//! files that the run deletes for an item that its source no longer contains
//! (issue #550), and the same layout is checked for `add` and `remove`.

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
    stage_issue_layout_with_bundle_at(with_z, "bundles/meta.bundle.yaml")
}

/// As [`stage_issue_layout`], with the bundle file `meta` at `bundle_rel`
/// under `.agents/skills`.
fn stage_issue_layout_with_bundle_at(
    with_z: bool,
    bundle_rel: &str,
) -> (tempfile::TempDir, PathBuf, PathBuf) {
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
    let bundle = skills.join(bundle_rel);
    fs::create_dir_all(bundle.parent().unwrap()).unwrap();
    fs::write(
        &bundle,
        format!(
            "schema: 1\nname: meta\ndescription: d\nitems: {{}}\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    )
    .unwrap();
    add(&home, &proj, &format!("./.agents/skills/{bundle_rel}"));
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
fn update_skips_a_later_source_that_cannot_be_fetched_and_applies_the_first() {
    // The labels sort `../aaa` before `../zzz`. The first source changes;
    // the second no longer exists, so its fetch fails and it is skipped
    // (issue #551). The first source is applied, and the run exits 1.
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
    let b_before = fs::read(proj.join(".claude/skills/b/SKILL.md")).unwrap();

    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["update", "--project", "--yes"])
        .assert()
        .code(1);
    let a = fs::read_to_string(proj.join(".claude/skills/a/SKILL.md")).unwrap();
    assert!(
        a.contains("Changed"),
        "the first source was not applied: {a}"
    );
    assert_eq!(
        fs::read(proj.join(".claude/skills/b/SKILL.md")).unwrap(),
        b_before
    );
}

#[test]
fn add_accepts_a_bundle_file_whose_source_holds_no_item() {
    // The bundle `meta` selects no item and only declares `requires`; its
    // source directory `.agents/skills` holds no item.
    let (_tmp, _home, proj) = stage_issue_layout(false);
    assert!(proj.join(".claude/skills/q/SKILL.md").is_file());
}

#[test]
fn update_reads_a_recorded_bundle_source_that_holds_no_item_from_the_same_root_as_add() {
    // The issue layout without `z` and without `y`: the recorded bundle
    // source `.agents/skills` holds no item. The project directory holds the
    // directory `.claude` with a `SKILL.md`, which a registry root search
    // from the bundle file finds first. `add` resolves the bundle against
    // `.agents/skills`, so `update` protects the items of `.agents/skills`
    // only, and the directory `.claude`, which contains the output path of
    // `q`, is not compared.
    let (_tmp, home, proj) = stage_issue_layout(false);
    fs::remove_dir_all(proj.join(".agents/skills/q")).unwrap();
    write_skill(&proj.join(".claude"), "c", "# c");

    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["update", "--project", "--yes"])
        .assert()
        .success();
    let q = fs::read_to_string(proj.join(".claude/skills/q/SKILL.md")).unwrap();
    assert!(q.contains("New body"), "{q}");
}

#[test]
fn update_of_a_name_protects_a_lockfile_source_that_it_does_not_update() {
    // `update a` reads only the source of `a`; the source of `b` is a
    // recorded local-path source that the run does not read.
    for a_dir in ["../aaa", "./src-a"] {
        let (_tmp, home, proj) = stage_two_sources(a_dir);
        assert_update_refused(
            &home,
            &proj,
            &["a"],
            &["the source directory of skill 'b' (", "of skill 'a';"],
        );
    }
}

#[test]
fn update_refuses_an_overlap_before_it_applies_an_earlier_source_with_changes() {
    // The source of `a` sorts first and has a change; the overlap is found
    // with the source of `b`, which sorts after it.
    let (tmp, home, proj) = stage_two_sources("../aaa");
    write_skill(&tmp.path().join("aaa/a"), "a", "Changed");
    assert_update_refused(
        &home,
        &proj,
        &[],
        &["the source directory of skill 'b' (", "of skill 'a';"],
    );
}

#[test]
fn update_refuses_an_overlap_with_an_unselected_item_of_another_source() {
    // The source `.agents/skills` holds the installed skill `b` (folder
    // `grp/b`) and the skill `u` (folder `a/u`), which is not installed. The
    // OpenCode output path `.agents/skills/a` of the skill `a` of another
    // source contains the directory of `u` only.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    write_skill(&proj.join(".agents/skills/grp/b"), "b", "# b");
    write_skill(&proj.join(".agents/skills/a/u"), "u", "# u");
    write_skill(&tmp.path().join("aaa/a"), "a", "# a");
    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["add", "./.agents/skills", "b", "--claude", "--project"])
        .assert()
        .success();
    add(&home, &proj, "../aaa");
    assert!(!proj.join(".claude/skills/u").exists());

    assert_update_refused(
        &home,
        &proj,
        &[],
        &["the source directory of skill 'u' (", "of skill 'a';"],
    );
}

#[test]
fn update_skips_a_recorded_directory_source_that_no_longer_exists() {
    let (_tmp, home, proj) = stage_two_sources("../aaa");
    fs::remove_dir_all(proj.join(".agents")).unwrap();

    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["update", "--project", "--yes", "a"])
        .assert()
        .success();
}

#[test]
fn update_fails_when_a_recorded_source_that_it_does_not_update_cannot_be_read() {
    // The recorded directory source of `b` is now a plain file.
    let (_tmp, home, proj) = stage_two_sources("../aaa");
    let source_b = proj.join(".agents/skills/a");
    fs::remove_dir_all(&source_b).unwrap();
    fs::write(&source_b, "a file\n").unwrap();
    let label = recorded_source(&proj, "items", "b");
    let before = files_under(&proj);

    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["update", "--project", "--yes", "a"])
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    let hint = format!(
        "read the recorded source `{label}` for the overlap check; restore it, or remove its \
         items with `upskill remove --source {label}`"
    );
    assert!(stderr.contains(&hint), "{stderr}");
    assert_eq!(files_under(&proj), before);
}

/// The `source` of the entry `name` in the lockfile array `array`
/// (`items` or `bundles`).
fn recorded_source(proj: &Path, array: &str, name: &str) -> String {
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(proj.join(".upskill-lock.json")).unwrap())
            .unwrap();
    lock[array]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == name)
        .and_then(|e| e["source"].as_str())
        .unwrap()
        .to_string()
}

#[test]
fn update_hint_for_an_unreadable_bundle_source_without_items_only_asks_to_restore_it() {
    // The bundle `meta`, added by name from `.agents/skills`, selects no
    // item: its source is in the lockfile `bundles` only. That source is
    // now a plain file.
    let (_tmp, home, proj) = stage_issue_layout(true);
    let skills = proj.join(".agents/skills");
    // Record `meta` again, by name from the directory source, without `y`.
    fs::remove_file(proj.join(".upskill-lock.json")).unwrap();
    fs::remove_dir_all(skills.join("q")).unwrap();
    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args([
            "add",
            "./.agents/skills",
            "meta",
            "--claude",
            "--project",
            "--force",
        ])
        .assert()
        .success();
    let label = recorded_source(&proj, "bundles", "meta");
    // The label holds the path separators of the operating system.
    assert!(
        label.replace('\\', "/").ends_with(".agents/skills"),
        "{label}"
    );
    fs::remove_dir_all(&skills).unwrap();
    fs::write(&skills, "a file\n").unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["update", "--project", "--yes"])
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    let hint = format!("read the recorded source `{label}` for the overlap check; restore it");
    assert!(stderr.contains(&hint), "{stderr}");
    assert!(!stderr.contains("remove --source"), "{stderr}");
}

#[test]
fn update_refused_by_the_whole_run_check_does_not_adopt_a_schema_1_lockfile() {
    let (_tmp, home, proj) = stage_two_sources("../aaa");
    let path = proj.join(".upskill-lock.json");
    let mut lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    lock["schema"] = 1.into();
    for entry in lock["items"].as_array_mut().unwrap() {
        let entry = entry.as_object_mut().unwrap();
        entry.remove("files");
        entry.remove("pending");
    }
    fs::write(&path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();

    assert_update_refused(
        &home,
        &proj,
        &[],
        &["the source directory of skill 'b' (", "of skill 'a';"],
    );
    assert!(!proj.join(".upskill-lock.json.v1").exists());
}

#[test]
fn update_protects_the_root_of_a_recorded_bundle_file_that_was_deleted() {
    // Only the bundle file `meta` is deleted; its registry root
    // `.agents/skills` still holds `z` and the skill `y`, whose directory is
    // inside the OpenCode output path of `q`.
    for extra in [&[][..], &["--force"][..]] {
        let (_tmp, home, proj) = stage_issue_layout(true);
        fs::remove_file(proj.join(".agents/skills/bundles/meta.bundle.yaml")).unwrap();
        assert_update_refused(
            &home,
            &proj,
            extra,
            &["the source directory of skill 'y' (", "of skill 'q';"],
        );
    }
}

/// The layout of issue #550. The skill `a` of `<tmp>/src-a` has the
/// resource `y/SKILL.md`, which is written for OpenCode at
/// `.agents/skills/a/y/SKILL.md`. With `with_y`, that directory is also the
/// source of the installed skill `y`. The skill `y` is added first, from a
/// copy of the file, so that the overlap check of `add` does not refuse it;
/// then `<tmp>/src-a` is added for OpenCode and Claude. Returns
/// `(tmp, home, proj, src_a)`.
fn stage_removed_item_layout(with_y: bool) -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    let src_a = tmp.path().join("src-a");
    write_skill(&src_a.join("a"), "a", "# a");
    write_skill(&src_a.join("a/y"), "y", "# y");
    write_skill(&src_a.join("other"), "other", "# other");
    if with_y {
        write_skill(&proj.join(".agents/skills/a/y"), "y", "# y");
        add(&home, &proj, "./.agents/skills/a");
    }
    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["add", "../src-a", "--project", "--opencode", "--claude"])
        .assert()
        .success();
    assert!(proj.join(".agents/skills/a/y/SKILL.md").is_file());
    if with_y {
        assert!(
            recorded_source(&proj, "items", "y")
                .replace('\\', "/")
                .ends_with(".agents/skills/a"),
            "{}",
            recorded_source(&proj, "items", "y")
        );
    }
    (tmp, home, proj, src_a)
}

#[test]
fn update_refuses_to_delete_a_removed_item_file_inside_the_source_of_another_item() {
    let (_tmp, home, proj, src_a) = stage_removed_item_layout(true);
    fs::remove_dir_all(src_a.join("a")).unwrap();
    assert_update_refused(
        &home,
        &proj,
        &[],
        &["the source directory of skill 'y' (", "of skill 'a';"],
    );
    assert!(proj.join(".agents/skills/a/y/SKILL.md").is_file());
}

#[test]
fn update_dry_run_refuses_to_delete_a_removed_item_file_inside_the_source_of_another_item() {
    let (_tmp, home, proj, src_a) = stage_removed_item_layout(true);
    fs::remove_dir_all(src_a.join("a")).unwrap();
    assert_update_refused(
        &home,
        &proj,
        &["--dry-run"],
        &["the source directory of skill 'y' (", "of skill 'a';"],
    );
}

#[test]
fn update_still_removes_an_item_that_left_its_source_without_an_overlap() {
    let (_tmp, home, proj, src_a) = stage_removed_item_layout(false);
    fs::remove_dir_all(src_a.join("a")).unwrap();
    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["update", "--project", "--yes"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("removed — no longer in source"), "{out}");
    assert!(!proj.join(".agents/skills/a").exists());
}

#[test]
fn add_refuses_a_source_item_that_contains_a_recorded_file_of_another_source() {
    // Step 2 of issue #550.
    let (_tmp, home, proj, _src_a) = stage_removed_item_layout(false);
    let before = files_under(&proj);
    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["add", "./.agents/skills/a", "--project", "--claude"])
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    for part in [
        "the source directory of skill 'y' (",
        ") contains the output path",
        "of skill 'a';",
        "installing would delete or overwrite the source",
    ] {
        assert!(stderr.contains(part), "stderr lacks {part:?}: {stderr}");
    }
    assert_eq!(files_under(&proj), before, "add changed a file");
}

#[test]
fn add_accepts_a_source_outside_every_recorded_output_path() {
    let (tmp, home, proj, _src_a) = stage_removed_item_layout(false);
    write_skill(&tmp.path().join("src-b/b"), "b", "# b");
    add(&home, &proj, "../src-b");
    assert!(proj.join(".claude/skills/b/SKILL.md").is_file());
}

/// Run `remove a` with `extra` and assert that it exits 1, names both
/// items, and changes no file.
fn assert_remove_refused(home: &Path, proj: &Path, extra: &[&str]) {
    let before = files_under(proj);
    let mut args = vec!["remove", "a", "--project"];
    args.extend_from_slice(extra);
    let assert = common::upskill_cmd(home)
        .current_dir(proj)
        .args(&args)
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    for part in [
        "of the installed skill 'y'",
        "the recorded path",
        "of skill 'a'",
        "Remove skill 'y' first",
    ] {
        assert!(stderr.contains(part), "stderr lacks {part:?}: {stderr}");
    }
    assert_eq!(files_under(proj), before, "remove changed a file");
}

#[test]
fn remove_refuses_to_delete_a_recorded_file_inside_the_source_of_another_item() {
    let (_tmp, home, proj, _src_a) = stage_removed_item_layout(true);
    assert_remove_refused(&home, &proj, &[]);
}

#[test]
fn remove_force_refuses_to_delete_a_recorded_file_inside_the_source_of_another_item() {
    let (_tmp, home, proj, _src_a) = stage_removed_item_layout(true);
    assert_remove_refused(&home, &proj, &["--force"]);
}

#[test]
fn remove_without_an_overlap_is_unchanged() {
    let (_tmp, home, proj, _src_a) = stage_removed_item_layout(false);
    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["remove", "a", "--project"])
        .assert()
        .success();
    assert!(!proj.join(".agents/skills/a").exists());
    assert!(proj.join(".agents/skills/other/SKILL.md").is_file());
}

#[test]
fn update_protects_the_root_of_a_deleted_bundle_file_in_a_group_layout() {
    // The bundle file is at `.agents/skills/grp/bundles/meta.bundle.yaml`;
    // `add` resolves it against `.agents/skills`, which holds `z`. After
    // only the bundle file is deleted, `update` protects the same root.
    for extra in [&[][..], &["--force"][..]] {
        let (_tmp, home, proj) =
            stage_issue_layout_with_bundle_at(true, "grp/bundles/meta.bundle.yaml");
        fs::remove_file(proj.join(".agents/skills/grp/bundles/meta.bundle.yaml")).unwrap();
        assert_update_refused(
            &home,
            &proj,
            extra,
            &["the source directory of skill 'y' (", "of skill 'q';"],
        );
    }
}

#[test]
fn update_protects_the_root_of_a_deleted_bundle_file_outside_a_bundles_directory() {
    let (_tmp, home, proj) = stage_issue_layout_with_bundle_at(true, "meta.bundle.yaml");
    fs::remove_file(proj.join(".agents/skills/meta.bundle.yaml")).unwrap();
    assert_update_refused(
        &home,
        &proj,
        &[],
        &["the source directory of skill 'y' (", "of skill 'q';"],
    );
}

#[test]
fn update_does_not_protect_the_parent_of_a_deleted_recorded_directory_source() {
    // The recorded directory source `.agents/skills/src-b` is deleted. The
    // directory `.agents/skills/a`, the OpenCode output path of `a`, holds
    // a `SKILL.md`; it would be refused if `.agents/skills` were protected.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    write_skill(&tmp.path().join("aaa/a"), "a", "# a");
    write_skill(&proj.join(".agents/skills/src-b/b"), "b", "# b");
    add(&home, &proj, "../aaa");
    add(&home, &proj, "./.agents/skills/src-b");
    fs::remove_dir_all(proj.join(".agents/skills/src-b")).unwrap();
    write_skill(&proj.join(".agents/skills/a"), "c", "# c");

    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["update", "--project", "--yes", "a"])
        .assert()
        .success();
}
