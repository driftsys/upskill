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
        out.contains("\n  update   skill create-api-endpoint"),
        "expected an `update` line: {out}"
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
        out.contains("\n  remove   skill create-api-endpoint   no longer in source"),
        "expected a `remove` line, got:\n{out}"
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
    // folder must not be falsely listed as `remove` by dry-run (the
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
        !out.contains("\n  remove "),
        "divergent-named rule must not be reported as removed:\n{out}"
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

/// Add `<tmp>/src/my-skill` to a project (with `add_args` appended), then
/// point the recorded source at `<proj>/<nested>`, a directory inside the
/// item's own output path that holds a changed copy of the skill. Assert that
/// `update` refuses the source and changes no file.
fn assert_update_refuses_source_inside_output(add_args: &[&str], installed: &str, nested: &str) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("my-skill")).unwrap();
    fs::write(
        src.join("my-skill/SKILL.md"),
        "---\nname: my-skill\ndescription: d\n---\nBody\n",
    )
    .unwrap();

    let mut args = vec!["add", src.to_str().unwrap(), "--claude", "--project"];
    args.extend_from_slice(add_args);
    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(&args)
        .assert()
        .success();

    let new_source = proj.join(nested);
    let copy = new_source.join("my-skill/SKILL.md");
    fs::create_dir_all(copy.parent().unwrap()).unwrap();
    let changed = "---\nname: my-skill\ndescription: d\n---\nBody v2\n";
    fs::write(&copy, changed).unwrap();
    let lock_path = proj.join(".upskill-lock.json");
    let mut lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    lock["items"][0]["source"] = format!("local:{}", new_source.display()).into();
    let lock_before = serde_json::to_string_pretty(&lock).unwrap();
    fs::write(&lock_path, &lock_before).unwrap();
    let output = proj.join(format!(".claude/skills/{installed}/SKILL.md"));
    let output_before = fs::read_to_string(&output).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["update"])
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains(&format!("'{installed}'")) && stderr.contains("overlaps the output path"),
        "stderr does not explain the refusal: {stderr}"
    );
    assert_eq!(fs::read_to_string(&copy).unwrap(), changed);
    assert_eq!(fs::read_to_string(&output).unwrap(), output_before);
    assert_eq!(fs::read_to_string(&lock_path).unwrap(), lock_before);
}

#[test]
fn update_refuses_source_inside_the_item_output_path() {
    assert_update_refuses_source_inside_output(&[], "my-skill", ".claude/skills/my-skill/nested");
}

#[test]
fn update_refuses_source_inside_the_recorded_alias_output_path() {
    // The lockfile records `source_name: my-skill` for the installed name
    // `other`; only the alias output path overlaps the source.
    assert_update_refuses_source_inside_output(
        &["--as", "other"],
        "other",
        ".claude/skills/other/nested",
    );
}

// -- Task 10: `update` keeps modified files --

fn res_skill_md(root: &Path) -> std::path::PathBuf {
    root.join(".claude")
        .join("skills")
        .join("res")
        .join("SKILL.md")
}

fn change_res_source(root: &Path) {
    let path = root.join("source").join("res").join("SKILL.md");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, format!("{text}Changed.\n")).unwrap();
}

#[test]
fn update_keeps_a_modified_file_and_skips_the_item() {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    fs::write(&dest, "my edit\n").unwrap();
    let hash_before = lockfile_hash_for(tmp.path(), "res");
    change_res_source(tmp.path());

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .code(1);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.get_output().stdout),
        String::from_utf8_lossy(&out.get_output().stderr)
    );
    assert!(all.contains("SKILL.md"), "{all}");
    assert!(all.contains("Use --force to replace."), "{all}");
    assert_eq!(fs::read(&dest).unwrap(), b"my edit\n");
    assert_eq!(lockfile_hash_for(tmp.path(), "res"), hash_before);
}

#[test]
fn update_force_replaces_a_modified_file() {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    fs::write(&dest, "my edit\n").unwrap();
    change_res_source(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project", "--force"])
        .assert()
        .success();
    assert!(fs::read_to_string(&dest).unwrap().contains("Changed."));
}

#[test]
fn update_deletes_a_file_the_new_version_no_longer_contains() {
    let (tmp, home) = res_project(|skill| {
        fs::create_dir_all(skill.join("scripts")).unwrap();
        fs::write(skill.join("scripts").join("a.sh"), "echo a\n").unwrap();
    });
    let scripts = tmp
        .path()
        .join(".claude")
        .join("skills")
        .join("res")
        .join("scripts");
    assert!(scripts.join("a.sh").is_file());
    fs::remove_dir_all(tmp.path().join("source").join("res").join("scripts")).unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .success();
    assert!(!scripts.join("a.sh").exists());
    assert!(!scripts.exists());
}

fn copilot_res_project() -> (tempfile::TempDir, std::path::PathBuf) {
    let (tmp, home) = res_project(|_| {});
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./source",
            "res",
            "--claude",
            "--copilot",
            "--project",
        ])
        .assert()
        .success();
    (tmp, home)
}

fn copilot_res_md(root: &Path) -> std::path::PathBuf {
    root.join(".github")
        .join("skills")
        .join("res")
        .join("SKILL.md")
}

#[test]
fn update_with_a_narrower_selection_deletes_the_owned_copilot_file() {
    let (tmp, home) = copilot_res_project();
    assert!(copilot_res_md(tmp.path()).is_file());

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project", "--claude"])
        .assert()
        .success();
    assert!(!copilot_res_md(tmp.path()).exists());
    assert_eq!(
        recorded_paths(tmp.path(), "res"),
        vec![".claude/skills/res/SKILL.md".to_string()]
    );
}

#[test]
fn update_with_a_narrower_selection_keeps_a_modified_copilot_file() {
    let (tmp, home) = copilot_res_project();
    fs::write(copilot_res_md(tmp.path()), "my edit\n").unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project", "--claude"])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("kept modified file: "), "{stderr}");
    assert!(stderr.contains("SKILL.md"), "{stderr}");
    assert_eq!(fs::read(copilot_res_md(tmp.path())).unwrap(), b"my edit\n");
    assert_eq!(
        recorded_paths(tmp.path(), "res"),
        vec![
            ".claude/skills/res/SKILL.md".to_string(),
            ".github/skills/res/SKILL.md".to_string()
        ]
    );
}

#[test]
fn update_keeps_a_modified_file_of_an_item_the_source_no_longer_contains() {
    let (tmp, home) = res_project(|_| {});
    // A second item, so the source does not resolve to zero items.
    let other = tmp.path().join("source").join("other");
    fs::create_dir_all(&other).unwrap();
    fs::write(
        other.join("SKILL.md"),
        "---\nschema: 1\nname: other\ndescription: Use for other\n---\nOther.\n",
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "other", "--claude", "--project"])
        .assert()
        .success();
    let dest = res_skill_md(tmp.path());
    fs::write(&dest, "my edit\n").unwrap();
    fs::remove_dir_all(tmp.path().join("source").join("res")).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("kept modified file: "), "{stderr}");
    assert_eq!(fs::read(&dest).unwrap(), b"my edit\n");
    let lock = fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(!lock.contains("\"name\": \"res\""), "{lock}");
}

// -- Task 10, fix round 1 --

/// `res_project` plus a second skill `other`, both installed.
fn two_skill_project() -> (tempfile::TempDir, std::path::PathBuf) {
    let (tmp, home) = res_project(|_| {});
    let other = tmp.path().join("source").join("other");
    fs::create_dir_all(&other).unwrap();
    fs::write(
        other.join("SKILL.md"),
        "---\nschema: 1\nname: other\ndescription: Use for other\n---\nOther.\n",
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "other", "--claude", "--project"])
        .assert()
        .success();
    (tmp, home)
}

#[test]
fn update_force_deletes_a_modified_file_of_an_item_the_source_no_longer_contains() {
    let (tmp, home) = two_skill_project();
    let dest = res_skill_md(tmp.path());
    fs::write(&dest, "my edit\n").unwrap();
    fs::remove_dir_all(tmp.path().join("source").join("res")).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project", "--force"])
        .assert()
        .success();
    assert!(!dest.exists());
    assert!(!dest.parent().unwrap().exists());
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(
        stdout.contains("Replaced or deleted 1 modified or unmanaged file(s)"),
        "{stdout}"
    );
    assert_forced_paths_are_relative(&stdout, tmp.path());
}

#[test]
fn update_force_reports_the_replaced_file_when_no_source_changed() {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    let generated = fs::read(&dest).unwrap();
    fs::write(&dest, "my edit\n").unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project", "--force"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(!stdout.contains("nothing to do"), "{stdout}");
    assert!(
        stdout.contains("Replaced or deleted 1 modified or unmanaged file(s)"),
        "{stdout}"
    );
    assert_forced_paths_are_relative(&stdout, tmp.path());
    assert_eq!(fs::read(&dest).unwrap(), generated);
}

#[test]
fn update_of_a_name_reports_a_skipped_dependency_from_another_source() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let reg_b = tmp.path().join("reg-b").join("sarif");
    fs::create_dir_all(&reg_b).unwrap();
    fs::write(
        reg_b.join("SKILL.md"),
        "---\nschema: 1\nname: sarif\ndescription: Use for sarif\n---\nBody.\n",
    )
    .unwrap();
    let reg_a = tmp.path().join("reg-a").join("code-review");
    fs::create_dir_all(&reg_a).unwrap();
    let agent = reg_a.join("AGENT.md");
    fs::write(
        &agent,
        format!(
            "---\nschema: 1\nname: code-review\ndescription: Use for review\nrequires:\n  \
             skills: [{{ name: sarif, source: {} }}]\n---\nBody.\n",
            tmp.path().join("reg-b").display()
        ),
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "code-review", "--claude", "--project"])
        .assert()
        .success();
    let sarif = tmp
        .path()
        .join(".claude")
        .join("skills")
        .join("sarif")
        .join("SKILL.md");
    fs::write(&sarif, "my edit\n").unwrap();
    let text = fs::read_to_string(&agent).unwrap();
    fs::write(&agent, format!("{text}Changed.\n")).unwrap();
    // The source of `sarif` changes too: an update of `sarif` is withheld.
    let sarif_source = reg_b.join("SKILL.md");
    let text = fs::read_to_string(&sarif_source).unwrap();
    fs::write(&sarif_source, format!("{text}Changed.\n")).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "code-review", "--project"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("skipped skill sarif: "), "{stderr}");
    assert_eq!(fs::read(&sarif).unwrap(), b"my edit\n");
}

/// Installs `code-review` from `./reg-a`, which requires the skill `sarif`
/// from `reg-b`. Returns the agent source file and the installed `sarif` file.
fn cross_source_project(tmp: &Path, home: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    fs::create_dir_all(tmp.join(".git")).unwrap();
    let reg_b = tmp.join("reg-b").join("sarif");
    fs::create_dir_all(&reg_b).unwrap();
    fs::write(
        reg_b.join("SKILL.md"),
        "---\nschema: 1\nname: sarif\ndescription: Use for sarif\n---\nBody.\n",
    )
    .unwrap();
    let reg_a = tmp.join("reg-a").join("code-review");
    fs::create_dir_all(&reg_a).unwrap();
    let agent = reg_a.join("AGENT.md");
    fs::write(
        &agent,
        format!(
            "---\nschema: 1\nname: code-review\ndescription: Use for review\nrequires:\n  \
             skills: [{{ name: sarif, source: {} }}]\n---\nBody.\n",
            tmp.join("reg-b").display()
        ),
    )
    .unwrap();
    common::upskill_cmd(home)
        .current_dir(tmp)
        .args(["add", "./reg-a", "code-review", "--claude", "--project"])
        .assert()
        .success();
    let sarif = tmp
        .join(".claude")
        .join("skills")
        .join("sarif")
        .join("SKILL.md");
    (agent, sarif)
}

#[test]
fn update_keeps_an_edited_file_of_a_dependency_from_another_source() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let (_agent, sarif) = cross_source_project(tmp.path(), &home);
    fs::write(&sarif, "my edit\n").unwrap();
    let lock_before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "code-review", "--yes", "--project"])
        .assert()
        .code(0);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert_eq!(sarif_notes(&stderr), vec![sarif_rel()], "{stderr}");
    assert!(!stderr.contains("skipped"), "{stderr}");
    assert_eq!(fs::read(&sarif).unwrap(), b"my edit\n");
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}

fn sarif_rel() -> std::path::PathBuf {
    Path::new(".claude")
        .join("skills")
        .join("sarif")
        .join("SKILL.md")
}

/// The paths of the `kept modified file` notes in `stderr`.
fn sarif_notes(stderr: &str) -> Vec<std::path::PathBuf> {
    stderr
        .lines()
        .filter_map(|line| line.strip_prefix("kept modified file: "))
        .map(std::path::PathBuf::from)
        .collect()
}

#[test]
fn update_of_every_source_prints_the_note_of_a_dependency_once() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let (_agent, sarif) = cross_source_project(tmp.path(), &home);
    fs::write(&sarif, "my edit\n").unwrap();
    let lock_before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();

    // The reinstall of `reg-a` and the reinstall of `reg-b` (the own source
    // of `sarif`) both keep the edit; the note is printed once.
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(0);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert_eq!(sarif_notes(&stderr), vec![sarif_rel()], "{stderr}");
    assert!(!stderr.contains("skipped"), "{stderr}");
    assert_eq!(fs::read(&sarif).unwrap(), b"my edit\n");
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}

#[test]
fn update_skips_an_edited_item_when_a_deselected_client_file_is_absent() {
    // The record of the Copilot file would be dropped and `clients` would
    // change: an update is withheld.
    let (tmp, home) = copilot_res_project();
    fs::remove_file(copilot_res_md(tmp.path())).unwrap();
    fs::write(res_skill_md(tmp.path()), "my edit\n").unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project", "--claude"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("skipped skill res: "), "{stderr}");
    assert_eq!(fs::read(res_skill_md(tmp.path())).unwrap(), b"my edit\n");
}

#[test]
fn update_skips_an_item_whose_deselected_client_file_is_also_edited() {
    let (tmp, home) = copilot_res_project();
    fs::write(res_skill_md(tmp.path()), "my edit\n").unwrap();
    fs::write(copilot_res_md(tmp.path()), "my copilot edit\n").unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project", "--claude"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("skipped skill res: "), "{stderr}");
    assert_eq!(
        fs::read(copilot_res_md(tmp.path())).unwrap(),
        b"my copilot edit\n"
    );
}

#[test]
fn update_summary_counts_the_deleted_file_of_a_deselected_client() {
    let (tmp, home) = copilot_res_project();
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project", "--claude"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(!stdout.contains("nothing to do"), "{stdout}");
    assert!(
        stdout.contains("Changed 1 file(s) written or deleted"),
        "{stdout}"
    );
    assert!(!copilot_res_md(tmp.path()).exists());
}

// -- Fix wave after review pass 1 --

/// Rewrite the installed file at `path` with CRLF line endings, as a git
/// checkout with `core.autocrlf=true` does.
fn rewrite_with_crlf(path: &Path) {
    let lf = fs::read(path).unwrap();
    assert!(!lf.contains(&b'\r'));
    let mut crlf = Vec::new();
    for &b in &lf {
        if b == b'\n' {
            crlf.extend_from_slice(b"\r\n");
        } else {
            crlf.push(b);
        }
    }
    fs::write(path, crlf).unwrap();
}

#[test]
fn update_treats_a_crlf_checkout_as_unmodified() {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    rewrite_with_crlf(&dest);
    change_res_source(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .code(0);
    let text = fs::read_to_string(&dest).unwrap();
    assert!(text.contains("Changed."), "{text}");
    assert!(!text.contains('\r'), "{text}");
}

#[test]
fn remove_deletes_a_crlf_checkout_of_an_installed_file() {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    rewrite_with_crlf(&dest);

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["remove", "res", "--project"])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(!stderr.contains("kept modified file"), "{stderr}");
    assert!(!dest.exists());
}

#[test]
fn doctor_does_not_report_a_crlf_checkout_as_modified() {
    let (tmp, home) = res_project(|_| {});
    rewrite_with_crlf(&res_skill_md(tmp.path()));

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor", "--json", "--project"])
        .assert()
        .success();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert!(v["modified_files"].as_array().unwrap().is_empty(), "{v}");
}

#[test]
fn update_keeps_an_edited_file_with_a_note_when_no_source_changed() {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    fs::write(&dest, "my edit\n").unwrap();
    let lock_before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(0);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    let note = stderr
        .lines()
        .find_map(|line| line.strip_prefix("kept modified file: "))
        .unwrap_or_else(|| panic!("no note in {stderr}"));
    assert_eq!(
        Path::new(note),
        Path::new(".claude")
            .join("skills")
            .join("res")
            .join("SKILL.md")
    );
    assert!(!stderr.contains("skipped"), "{stderr}");
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(!stdout.contains("skipped"), "{stdout}");
    assert_eq!(fs::read(&dest).unwrap(), b"my edit\n");
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}

#[test]
fn update_skips_an_edited_item_when_a_deselected_client_file_would_be_deleted() {
    let (tmp, home) = copilot_res_project();
    fs::write(res_skill_md(tmp.path()), "my edit\n").unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project", "--claude"])
        .assert()
        .code(1);
    assert!(copilot_res_md(tmp.path()).is_file());
    assert_eq!(fs::read(res_skill_md(tmp.path())).unwrap(), b"my edit\n");
}

#[test]
fn update_skips_an_item_with_an_unmanaged_file_when_no_source_changed() {
    // `res` is installed for Claude only. A file that upskill did not write
    // is at the Copilot destination, which `--copilot` now plans: the
    // destination is unmanaged, and the source of `res` did not change.
    let (tmp, home) = res_project(|_| {});
    let copilot = copilot_res_md(tmp.path());
    fs::create_dir_all(copilot.parent().unwrap()).unwrap();
    fs::write(&copilot, "my own file\n").unwrap();
    fs::write(res_skill_md(tmp.path()), "my edit\n").unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project", "--claude", "--copilot"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("skipped skill res: "), "{stderr}");
    assert_eq!(fs::read(&copilot).unwrap(), b"my own file\n");
}

/// Rewrite the lockfile as schema 1 without `files` records.
fn downgrade_to_schema_1(root: &Path) {
    let path = root.join(".upskill-lock.json");
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

#[test]
fn update_that_adopts_a_schema_1_lockfile_deletes_the_files_of_an_orphan() {
    let (tmp, home) = two_skill_project();
    let dest = res_skill_md(tmp.path());
    assert!(dest.is_file());
    downgrade_to_schema_1(tmp.path());
    fs::remove_dir_all(tmp.path().join("source").join("res")).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stdout.contains("removed"), "{stdout}");
    assert!(!stderr.contains("kept modified file"), "{stderr}");
    assert!(!dest.exists());
    assert!(!dest.parent().unwrap().exists());
    let lock = fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(!lock.contains("\"name\": \"res\""), "{lock}");
    assert!(lock.contains("\"schema\": 2"), "{lock}");
}

#[test]
fn update_that_only_adopts_a_schema_1_lockfile_does_not_print_nothing_to_do() {
    let (tmp, home) = res_project(|_| {});
    downgrade_to_schema_1(tmp.path());

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(!stdout.contains("nothing to do"), "{stdout}");
    assert!(stdout.contains("Updated 0 of 1 item(s)"), "{stdout}");
}

/// Two sources that both require the skill `sarif` from a third source;
/// `sarif` is edited and its source changed. Returns the project, the fake
/// home and the edited file.
fn two_sources_withholding_sarif() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let reg_c = tmp.path().join("reg-c").join("sarif");
    fs::create_dir_all(&reg_c).unwrap();
    fs::write(
        reg_c.join("SKILL.md"),
        "---\nschema: 1\nname: sarif\ndescription: Use for sarif\n---\nBody.\n",
    )
    .unwrap();
    for (reg, name) in [("reg-a", "first"), ("reg-b", "second")] {
        let dir = tmp.path().join(reg).join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("AGENT.md"),
            format!(
                "---\nschema: 1\nname: {name}\ndescription: Use for {name}\nrequires:\n  \
                 skills: [{{ name: sarif, source: {} }}]\n---\nBody.\n",
                tmp.path().join("reg-c").display()
            ),
        )
        .unwrap();
        common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(["add", &format!("./{reg}"), name, "--claude", "--project"])
            .assert()
            .success();
    }
    let sarif = tmp
        .path()
        .join(".claude")
        .join("skills")
        .join("sarif")
        .join("SKILL.md");
    fs::write(&sarif, "my edit\n").unwrap();
    // The source of `sarif` changes: both sources withhold its update.
    let sarif_source = reg_c.join("SKILL.md");
    let text = fs::read_to_string(&sarif_source).unwrap();
    fs::write(&sarif_source, format!("{text}Changed.\n")).unwrap();

    (tmp, home, sarif)
}

#[test]
fn update_reports_an_item_skipped_by_two_sources_once() {
    let (tmp, home, sarif) = two_sources_withholding_sarif();
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "first", "second", "--project"])
        .assert()
        .code(1);
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert_eq!(
        stdout
            .matches("skipped — modified or unmanaged files kept")
            .count(),
        1,
        "{stdout}"
    );
    assert_eq!(fs::read(&sarif).unwrap(), b"my edit\n");
}

/// The dry run lists `sarif`, which both sources would withhold, once.
#[test]
fn update_dry_run_counts_an_item_withheld_by_two_sources_once() {
    let (tmp, home, sarif) = two_sources_withholding_sarif();
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "first", "second", "--project", "--dry-run"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    // `first`, `second` and `sarif`.
    assert!(stdout.contains("would update 0 of 3 item(s)"), "{stdout}");
    assert_eq!(fs::read(&sarif).unwrap(), b"my edit\n");
}

#[test]
#[cfg(unix)]
fn update_force_in_a_terminal_changes_nothing_on_no() {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    fs::write(&dest, "my edit\n").unwrap();
    let lock_before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();

    let result = common::update_in_terminal(tmp.path(), &home, &["--project", "--force"], "n\n");
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        result["stderr"].as_str().unwrap().contains("[y/N/q]"),
        "{result}"
    );
    // The block names the edited file that `--force` would replace.
    assert!(
        result["stdout"]
            .as_str()
            .unwrap()
            .contains("replaces .claude/skills/res/SKILL.md (--force)"),
        "{result}"
    );
    assert_eq!(fs::read(&dest).unwrap(), b"my edit\n");
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}

#[test]
#[cfg(unix)]
fn update_force_in_a_terminal_replaces_the_file_on_yes() {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    let generated = fs::read(&dest).unwrap();
    fs::write(&dest, "my edit\n").unwrap();

    let result = common::update_in_terminal(tmp.path(), &home, &["--project", "--force"], "y\n");
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        result["stderr"].as_str().unwrap().contains("[y/N/q]"),
        "{result}"
    );
    assert_eq!(fs::read(&dest).unwrap(), generated);
}

#[test]
#[cfg(unix)]
fn update_in_a_terminal_changes_nothing_on_no_after_a_source_change() {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    let before = fs::read(&dest).unwrap();
    change_res_source(tmp.path());
    let lock_before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();

    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], "n\n");
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        result["stderr"].as_str().unwrap().contains("[y/N/q]"),
        "{result}"
    );
    let stdout = result["stdout"].as_str().unwrap();
    // The block is printed before the question.
    assert!(stdout.contains("\n  update   skill res"), "{result}");
    assert!(stdout.contains("1 source(s) not applied."), "{result}");
    assert_eq!(fs::read(&dest).unwrap(), before);
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}

/// Every path line after the `--force` summary is relative to the install
/// root: it holds a file name and not the path of the temporary directory.
fn assert_forced_paths_are_relative(stdout: &str, root: &Path) {
    let root = root.display().to_string();
    let mut lines = stdout
        .lines()
        .skip_while(|line| !line.contains("Replaced or deleted"))
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .peekable();
    assert!(lines.peek().is_some(), "{stdout}");
    for line in lines {
        assert!(line.trim_end().ends_with(".md"), "{stdout}");
        assert!(!line.contains(&root), "{stdout}");
    }
}

#[cfg(unix)]
fn assert_terminal_update_applies_after_a_source_change(answer: &str) {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    change_res_source(tmp.path());

    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], answer);
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        result["stderr"].as_str().unwrap().contains("[y/N/q]"),
        "{result}"
    );
    let stdout = result["stdout"].as_str().unwrap();
    assert!(stdout.contains("\n  update   skill res"), "{result}");
    assert!(
        stdout.contains("Updated 1 of 1 item(s) from 1 of 1 source(s)."),
        "{result}"
    );
    assert!(fs::read_to_string(&dest).unwrap().contains("Changed."));
}

#[test]
#[cfg(unix)]
fn update_in_a_terminal_applies_on_y_after_a_source_change() {
    assert_terminal_update_applies_after_a_source_change("y\n");
}

#[test]
#[cfg(unix)]
fn update_in_a_terminal_accepts_yes_in_any_letter_case() {
    assert_terminal_update_applies_after_a_source_change("YeS\n");
}

fn recorded_hash(root: &Path, path: &str) -> String {
    let raw = fs::read_to_string(root.join(".upskill-lock.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["items"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|i| i["files"].as_array().unwrap().iter())
        .find(|f| f["path"] == path)
        .unwrap()["sha256"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn update_leaves_a_crlf_checkout_as_it_is_until_the_content_changes() {
    let (tmp, home) = res_project(|_| {});
    let dest = res_skill_md(tmp.path());
    let record = ".claude/skills/res/SKILL.md";
    let hash_before = recorded_hash(tmp.path(), record);
    rewrite_with_crlf(&dest);
    let crlf = fs::read(&dest).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .code(0);
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(
        stdout.contains("everything is up to date — nothing to do"),
        "{stdout}"
    );
    assert_eq!(fs::read(&dest).unwrap(), crlf);
    assert_eq!(recorded_hash(tmp.path(), record), hash_before);

    change_res_source(tmp.path());
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .code(0);
    let text = fs::read_to_string(&dest).unwrap();
    assert!(text.contains("Changed."), "{text}");
    assert!(!text.contains('\r'), "{text}");
}

#[test]
fn update_writes_a_source_change_from_crlf_to_lf() {
    let (tmp, home) = res_project(|skill| {
        fs::create_dir_all(skill.join("scripts")).unwrap();
        fs::write(skill.join("scripts").join("a.sh"), b"echo a\r\n").unwrap();
    });
    let installed = tmp
        .path()
        .join(".claude")
        .join("skills")
        .join("res")
        .join("scripts")
        .join("a.sh");
    assert_eq!(fs::read(&installed).unwrap(), b"echo a\r\n");
    fs::write(
        tmp.path()
            .join("source")
            .join("res")
            .join("scripts")
            .join("a.sh"),
        b"echo a\n",
    )
    .unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .code(0);
    assert_eq!(fs::read(&installed).unwrap(), b"echo a\n");
    use sha2::Digest;
    let lf: String = sha2::Sha256::digest(b"echo a\n")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        recorded_hash(tmp.path(), ".claude/skills/res/scripts/a.sh"),
        lf
    );
}

// -- `update` source by source --

#[test]
fn update_dry_run_writes_nothing_with_a_schema_1_lockfile() {
    let (tmp, home) = res_project(|_| {});
    downgrade_to_schema_1(tmp.path());
    change_res_source(tmp.path());
    let lock_path = tmp.path().join(".upskill-lock.json");
    let before = fs::read(&lock_path).unwrap();
    let dest_before = fs::read(res_skill_md(tmp.path())).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--dry-run", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("res"), "{stdout}");
    assert!(stdout.contains("\n  update   skill res"), "{stdout}");
    assert_eq!(fs::read(&lock_path).unwrap(), before);
    assert!(!tmp.path().join(".upskill-lock.json.v1").exists());
    assert_eq!(fs::read(res_skill_md(tmp.path())).unwrap(), dest_before);
}

/// Writes `<root>/<dir>/<name>/SKILL.md` with `body`.
fn write_skill_in(root: &Path, dir: &str, name: &str, body: &str) {
    let path = root.join(dir).join(name);
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("SKILL.md"),
        format!("---\nschema: 1\nname: {name}\ndescription: Use for {name}\n---\n{body}\n"),
    )
    .unwrap();
}

#[test]
fn update_refuses_an_empty_source_before_any_write() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    // The first source is a directory, the second a bundle. The labels
    // sort `a-src` before `b-src`.
    write_skill_in(tmp.path(), "a-src", "first", "First.");
    write_skill_in(tmp.path(), "b-src", "second", "Second.");
    let bundle = tmp.path().join("b-src").join("b.bundle.yaml");
    fs::write(
        &bundle,
        "schema: 1\nname: b\ndescription: bundle b\nitems:\n  skills:\n    - second\n",
    )
    .unwrap();
    for source in ["./a-src", "./b-src/b.bundle.yaml"] {
        common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(["add", source, "--claude", "--project"])
            .assert()
            .success();
    }
    write_skill_in(tmp.path(), "a-src", "first", "First, changed.");
    fs::write(
        &bundle,
        "schema: 1\nname: b\ndescription: bundle b\nitems:\n  skills: []\n",
    )
    .unwrap();
    let second = tmp
        .path()
        .join(".claude")
        .join("skills")
        .join("second")
        .join("SKILL.md");
    let second_before = fs::read(&second).unwrap();
    let lock_path = tmp.path().join(".upskill-lock.json");
    let bundles_before =
        serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&lock_path).unwrap())
            .unwrap()["bundles"]
            .clone();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("resolved to no items"), "{stderr}");
    // The first source was applied.
    let first = fs::read_to_string(
        tmp.path()
            .join(".claude")
            .join("skills")
            .join("first")
            .join("SKILL.md"),
    )
    .unwrap();
    assert!(first.contains("First, changed."), "{first}");
    // The second source was not: its file and its bundle record are as before.
    assert_eq!(fs::read(&second).unwrap(), second_before);
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    assert_eq!(lock["bundles"], bundles_before, "{lock}");
    assert!(
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["name"] == "second"),
        "{lock}"
    );
}

#[test]
fn update_yes_applies_a_source_without_a_change() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    // No client CLI on PATH: every plugin is recorded as skipped.
    let empty_path = tmp.path().join("empty-path");
    fs::create_dir_all(&empty_path).unwrap();
    let registry = tmp.path().join("registry");
    copy_dir_all(&Path::new(FIXTURES).join("items"), &registry).unwrap();
    copy_dir_all(
        &Path::new(FIXTURES).join("bundles"),
        &registry.join("bundles"),
    )
    .unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join(".git")).unwrap();
    let bundle = registry.join("bundles").join("with-plugins.bundle.yaml");
    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &empty_path)
        .args(["add", bundle.to_str().unwrap(), "--project"])
        .assert()
        .success();
    // Drop the plugin records: only an apply of the source writes them again.
    let lock_path = project.join(".upskill-lock.json");
    let mut lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    assert!(!lock["plugins"].as_array().unwrap().is_empty(), "{lock}");
    lock.as_object_mut().unwrap().remove("plugins");
    fs::write(&lock_path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();

    common::upskill_cmd(&home)
        .current_dir(&project)
        .env("PATH", &empty_path)
        .args(["update", "--yes", "--project"])
        .assert()
        .code(0);
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    assert!(
        !lock["plugins"].as_array().is_none_or(Vec::is_empty),
        "{lock}"
    );
}

#[test]
fn update_yes_with_a_kept_edit_does_not_print_nothing_to_do() {
    let (tmp, home) = res_project(|_| {});
    fs::write(res_skill_md(tmp.path()), "my edit\n").unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(0);
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("kept modified file: "), "{stderr}");
    assert!(!stdout.contains("nothing to do"), "{stdout}");
}

#[test]
fn update_reports_a_kept_stale_file_as_a_note_and_not_as_a_change() {
    let (tmp, home) = res_project(|skill| {
        fs::write(skill.join("notes.md"), "notes\n").unwrap();
    });
    let notes = tmp
        .path()
        .join(".claude")
        .join("skills")
        .join("res")
        .join("notes.md");
    fs::write(&notes, "my notes\n").unwrap();
    fs::remove_file(tmp.path().join("source").join("res").join("notes.md")).unwrap();
    // The first update writes the new version and keeps the edited file.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(0);

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--dry-run", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("would update 0 of 1"), "{stdout}");
    assert!(stdout.contains("up to date"), "{stdout}");
    assert!(!stdout.contains("\n  update "), "{stdout}");

    let lock_before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(0);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    let noted: Vec<std::path::PathBuf> = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("kept modified file: "))
        .map(|line| std::path::PathBuf::from(line.trim_end()))
        .collect();
    assert_eq!(
        noted,
        vec![
            Path::new(".claude")
                .join("skills")
                .join("res")
                .join("notes.md")
        ],
        "{stderr}"
    );
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert_eq!(fs::read(&notes).unwrap(), b"my notes\n");
}

#[test]
fn update_yes_with_an_unsupported_opencode_json_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let rule = tmp.path().join("source").join("style").join("RULE.md");
    fs::create_dir_all(rule.parent().unwrap()).unwrap();
    fs::write(
        &rule,
        "---\nschema: 1\nname: style\ndescription: Use for style\n---\nBody.\n",
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "--project"])
        .assert()
        .success();
    fs::write(
        tmp.path().join("opencode.json"),
        "{\n  \"instructions\": \"x\"\n}\n",
    )
    .unwrap();
    fs::write(
        &rule,
        "---\nschema: 1\nname: style\ndescription: Use for style\n---\nChanged.\n",
    )
    .unwrap();
    let claude_rule = tmp.path().join(".claude").join("rules").join("style.md");
    let rule_before = fs::read(&claude_rule).unwrap();
    let lock_before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1);
    assert_eq!(fs::read(&claude_rule).unwrap(), rule_before);
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}
