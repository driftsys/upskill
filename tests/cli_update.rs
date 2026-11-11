//! ATDD tests for `upskill update`.
//!
//! Drives `add` then `update` end-to-end via the CLI. The local-path
//! source path lets us mutate the SSOT in place between `add` and
//! `update` to exercise the change-detection branch (lockfile hash vs
//! recomputed hash) without hitting the network.

mod common;

use std::fs;
use std::path::Path;

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

fn install(target: &Path, source: &Path, home: &Path) {
    common::upskill_cmd(home)
        .current_dir(target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();
}

/// Mutate one fixture skill so its content hash will change on the next
/// install. Appends a sentinel line to the SKILL body.
fn mutate_skill(source: &Path) {
    let path = source.join("create-api-endpoint/SKILL.md");
    let original = fs::read_to_string(&path).unwrap();
    fs::write(path, format!("{original}\n<!-- mutated by test -->\n")).unwrap();
}

fn lockfile_hash_for(target: &Path, name: &str) -> Option<String> {
    let raw = fs::read_to_string(target.join(".upskill-lock.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    parsed["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"].as_str() == Some(name))
        .and_then(|i| i["hash"].as_str().map(str::to_string))
}

#[test]
fn update_no_change_reports_up_to_date() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["update"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("everything is up to date"),
        "expected no changes, got:\n{out}"
    );
}

#[test]
fn update_after_ssot_mutation_rewrites_outputs_and_lockfile_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let original_hash = lockfile_hash_for(&target, "create-api-endpoint").unwrap();
    mutate_skill(&source);

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["update", "create-api-endpoint"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("Updated 1 of"),
        "expected 1 change, got:\n{out}"
    );

    let new_hash = lockfile_hash_for(&target, "create-api-endpoint").unwrap();
    assert_ne!(
        original_hash, new_hash,
        "lockfile hash should change after SSOT mutation"
    );

    // The rendered Claude SKILL output should now contain the mutation marker.
    let claude_out =
        fs::read_to_string(target.join(".claude/skills/create-api-endpoint/SKILL.md")).unwrap();
    assert!(
        claude_out.contains("mutated by test"),
        "regenerated output missing mutation marker"
    );
}

#[test]
fn update_dry_run_reports_change_without_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let original_hash = lockfile_hash_for(&target, "create-api-endpoint").unwrap();
    mutate_skill(&source);

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["update", "--dry-run"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("Dry-run"), "expected dry-run header: {out}");
    assert!(
        out.contains("would change"),
        "expected `would change` status: {out}"
    );

    // Lockfile must not have been touched.
    let after_hash = lockfile_hash_for(&target, "create-api-endpoint").unwrap();
    assert_eq!(
        original_hash, after_hash,
        "dry-run must not modify the lockfile"
    );

    // And the rendered output stays at the pre-mutation content.
    let claude_out =
        fs::read_to_string(target.join(".claude/skills/create-api-endpoint/SKILL.md")).unwrap();
    assert!(
        !claude_out.contains("mutated by test"),
        "dry-run must not regenerate per-client output"
    );
}

#[test]
fn update_unknown_name_is_general_error() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["update", "this-was-never-installed"])
        .assert()
        .failure()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("this-was-never-installed"),
        "stderr should name the missing item: {stderr}"
    );
}

#[test]
fn update_removes_orphaned_item_when_source_no_longer_contains_it() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    // Verify the skill is installed.
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "skill should be installed before removal"
    );
    assert!(lockfile_hash_for(&target, "create-api-endpoint").is_some());

    // Remove the item from the source so it becomes an orphan.
    fs::remove_dir_all(source.join("create-api-endpoint")).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["update", "--yes"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("removed") || out.contains("Removed"),
        "expected removed status, got:\n{out}"
    );

    // Output files should be gone.
    assert!(
        !target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "skill output should be deleted after orphan removal"
    );
    // Lockfile entry should be gone.
    assert!(
        lockfile_hash_for(&target, "create-api-endpoint").is_none(),
        "lockfile entry should be removed for orphaned item"
    );
}

#[test]
fn update_dry_run_reports_would_remove_without_deleting() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    // Remove the item from the source so it becomes an orphan.
    fs::remove_dir_all(source.join("create-api-endpoint")).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["update", "--dry-run"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("would remove"),
        "expected `would remove` status, got:\n{out}"
    );

    // Output files should still exist.
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "dry-run must not delete output files"
    );
    // Lockfile entry should still exist.
    assert!(
        lockfile_hash_for(&target, "create-api-endpoint").is_some(),
        "dry-run must not remove lockfile entry"
    );
}

#[test]
fn update_empty_lockfile_is_no_op() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("everything is up to date") || out.contains("nothing to update"),
        "expected empty-lockfile message: {out}"
    );
}

#[test]
fn update_removes_stale_output_file_from_previous_generation() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    // A resource that the previous generation copies and the next one does
    // not: the install records it as a written file.
    fs::write(
        source.join("create-api-endpoint/OLD_FILE.md"),
        "# stale content",
    )
    .unwrap();
    install(&target, &source, &home);
    let stale_path = target.join(".claude/skills/create-api-endpoint/OLD_FILE.md");
    assert!(stale_path.exists());

    // Drop the resource and mutate the source so update will regenerate.
    fs::remove_file(source.join("create-api-endpoint/OLD_FILE.md")).unwrap();
    mutate_skill(&source);

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["update", "--yes"])
        .assert()
        .success();

    // Stale file should be removed.
    assert!(
        !stale_path.exists(),
        "stale file should have been removed during regeneration"
    );

    // Real output should still exist.
    let real_output = target.join(".claude/skills/create-api-endpoint/SKILL.md");
    assert!(
        real_output.exists(),
        "regenerated output should exist after update"
    );
}

/// Non-TTY (test harness) should auto-proceed without `--yes`.
#[test]
fn update_without_yes_proceeds_in_non_tty() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    // Mutate source so there's something to update.
    mutate_skill(&source);

    let hash_before = lockfile_hash_for(&target, "create-api-endpoint");

    // Run update WITHOUT --yes; non-TTY should auto-proceed.
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["update"])
        .assert()
        .success();

    let hash_after = lockfile_hash_for(&target, "create-api-endpoint");
    assert_ne!(
        hash_before, hash_after,
        "update should have applied without --yes in non-TTY"
    );
}

#[test]
fn update_dry_run_reports_up_to_date_for_divergent_named_rule() {
    // Regression for #214: a rule whose frontmatter name diverges from its
    // folder must not be falsely reported as `would remove` by dry-run (the
    // source-side hash map is keyed by folder, the lookup must match).
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // Folder `stuff/` holds a rule named `security-baseline` (name diverges).
    let rule_dir = source.join("stuff");
    fs::create_dir_all(&rule_dir).unwrap();
    fs::write(
        rule_dir.join("RULE.md"),
        "---\nschema: 1\nname: security-baseline\ndescription: divergent rule\n---\n# body\n",
    )
    .unwrap();

    install(&target, &source, &home);

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["update", "--dry-run"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        !out.contains("would remove"),
        "divergent-named rule must not be reported as would-remove:\n{out}"
    );
}

#[test]
fn update_changes_nothing_when_a_directory_was_replaced_by_a_file() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    stage_source(&tmp.path().join("source"));
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./source",
            "api-conventions",
            "create-api-endpoint",
            "--claude",
            "--project",
        ])
        .assert()
        .success();
    let rules = tmp.path().join(".claude/rules");
    fs::rename(&rules, tmp.path().join("rules-moved")).unwrap();
    fs::write(&rules, b"a file\n").unwrap();
    let skill_md = tmp.path().join("source/create-api-endpoint/SKILL.md");
    let mut src = fs::read(&skill_md).unwrap();
    src.extend_from_slice(b"\nA new line.\n");
    fs::write(&skill_md, src).unwrap();
    let installed = tmp
        .path()
        .join(".claude/skills/create-api-endpoint/SKILL.md");
    let before = fs::read(&installed).unwrap();
    let lock_before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .failure()
        .code(1);

    assert_eq!(fs::read(&installed).unwrap(), before);
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert_eq!(fs::read(&rules).unwrap(), b"a file\n");
}

/// A project with a skill `res` whose source holds `SKILL.md` and the
/// resource entries written by `setup`, installed for Claude.
fn res_project(setup: impl Fn(&Path)) -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let skill = tmp.path().join("source/res");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nschema: 1\nname: res\ndescription: Use for res\n---\nRead notes.\n",
    )
    .unwrap();
    setup(&skill);
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "res", "--claude", "--project"])
        .assert()
        .success();
    (tmp, home)
}

fn recorded_paths(root: &Path, name: &str) -> Vec<String> {
    let raw = fs::read_to_string(root.join(".upskill-lock.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let item = v["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == name)
        .unwrap()
        .clone();
    let mut paths: Vec<String> = item["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_string())
        .collect();
    paths.sort();
    paths
}

#[test]
fn update_replaces_a_resource_file_with_a_directory() {
    let (tmp, home) = res_project(|skill| fs::write(skill.join("notes"), "old file\n").unwrap());
    let notes = tmp.path().join("source/res/notes");
    fs::remove_file(&notes).unwrap();
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join("a.md"), "new nested\n").unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .success();

    assert_eq!(
        fs::read(tmp.path().join(".claude/skills/res/notes/a.md")).unwrap(),
        b"new nested\n"
    );
    assert_eq!(
        recorded_paths(tmp.path(), "res"),
        vec![
            ".claude/skills/res/SKILL.md".to_string(),
            ".claude/skills/res/notes/a.md".to_string()
        ]
    );
}

#[test]
fn update_replaces_a_resource_directory_with_a_file() {
    let (tmp, home) = res_project(|skill| {
        fs::create_dir_all(skill.join("notes")).unwrap();
        fs::write(skill.join("notes/a.md"), "old nested\n").unwrap();
    });
    let notes = tmp.path().join("source/res/notes");
    fs::remove_dir_all(&notes).unwrap();
    fs::write(&notes, "new file\n").unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .success();

    assert_eq!(
        fs::read(tmp.path().join(".claude/skills/res/notes")).unwrap(),
        b"new file\n"
    );
    assert_eq!(
        recorded_paths(tmp.path(), "res"),
        vec![
            ".claude/skills/res/SKILL.md".to_string(),
            ".claude/skills/res/notes".to_string()
        ]
    );
}
