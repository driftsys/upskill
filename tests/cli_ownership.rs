//! ATDD tests for the ownership record: `add` records every file it writes,
//! with its SHA-256, in the item's `files` list, and adopts a `schema: 1`
//! lockfile on the first change.

mod common;

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn copy_dir_all(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let to_path = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir_all(&entry.path(), &to_path);
        } else {
            fs::copy(entry.path(), &to_path).unwrap();
        }
    }
}

/// A project directory with the fixture items staged under `source/`.
fn project() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    copy_dir_all(
        &Path::new(FIXTURES).join("items"),
        &tmp.path().join("source"),
    );
    (tmp, home)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn lockfile(root: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(root.join(".upskill-lock.json")).unwrap()).unwrap()
}

fn item<'a>(lock: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == name)
        .unwrap_or_else(|| panic!("no item {name} in {lock}"))
}

fn file_paths(item: &serde_json::Value) -> Vec<String> {
    item["files"]
        .as_array()
        .unwrap_or_else(|| panic!("no files in {item}"))
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn add_records_written_files_with_hashes() {
    let (tmp, home) = project();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./source",
            "create-api-endpoint",
            "--claude",
            "--copilot",
            "--project",
        ])
        .assert()
        .success();

    let lock = lockfile(tmp.path());
    assert_eq!(lock["schema"], 2);
    let files = item(&lock, "create-api-endpoint")["files"]
        .as_array()
        .unwrap()
        .clone();
    for (client, path) in [
        ("claude", ".claude/skills/create-api-endpoint/SKILL.md"),
        ("copilot", ".github/skills/create-api-endpoint/SKILL.md"),
    ] {
        let entry = files
            .iter()
            .find(|f| f["path"] == path)
            .unwrap_or_else(|| panic!("no record for {path}: {files:?}"));
        assert_eq!(entry["client"], client);
        let on_disk = fs::read(tmp.path().join(path)).unwrap();
        assert_eq!(entry["sha256"], sha256_hex(&on_disk));
    }
}

#[test]
fn add_records_resource_files() {
    let (tmp, home) = project();
    let skill = tmp.path().join("source/with-script");
    fs::create_dir_all(skill.join("scripts")).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nschema: 1\nname: with-script\ndescription: A skill with a script\n---\nRun scripts/run.sh.\n",
    )
    .unwrap();
    fs::write(skill.join("scripts/run.sh"), "#!/bin/sh\necho hi\n").unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "with-script", "--claude", "--project"])
        .assert()
        .success();

    let lock = lockfile(tmp.path());
    let paths = file_paths(item(&lock, "with-script"));
    assert!(
        paths.contains(&".claude/skills/with-script/scripts/run.sh".to_string()),
        "{paths:?}"
    );
}

#[test]
fn alias_install_records_alias_paths() {
    let (tmp, home) = project();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./source",
            "create-api-endpoint",
            "--as",
            "create-api-endpoint=make-endpoint",
            "--claude",
            "--project",
        ])
        .assert()
        .success();

    let lock = lockfile(tmp.path());
    let paths = file_paths(item(&lock, "make-endpoint"));
    assert!(!paths.is_empty());
    for path in &paths {
        assert!(path.starts_with(".claude/skills/make-endpoint/"), "{path}");
    }
    assert!(
        !tmp.path()
            .join(".claude/skills/create-api-endpoint")
            .exists()
    );
}

/// Writes a schema-1 lockfile for the rule `api-conventions` (Claude only)
/// and its output file by hand. Returns the lockfile bytes.
fn write_schema_1_install(root: &Path) -> Vec<u8> {
    let rule = root.join(".claude/rules/api-conventions.md");
    fs::create_dir_all(rule.parent().unwrap()).unwrap();
    fs::write(&rule, "hand-written rule\n").unwrap();
    let lock = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "rule",
            "name": "api-conventions",
            "source": "./source",
            "clients": ["claude"]
        }],
        "bundles": []
    });
    let bytes = serde_json::to_vec_pretty(&lock).unwrap();
    fs::write(root.join(".upskill-lock.json"), &bytes).unwrap();
    bytes
}

#[test]
fn first_change_adopts_a_schema_1_lockfile() {
    let (tmp, home) = project();
    let original = write_schema_1_install(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "create-api-endpoint", "--claude"])
        .assert()
        .success();

    let lock = lockfile(tmp.path());
    assert_eq!(lock["schema"], 2);
    assert_eq!(
        file_paths(item(&lock, "api-conventions")),
        vec![".claude/rules/api-conventions.md".to_string()]
    );
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json.v1")).unwrap(),
        original
    );
}

#[test]
fn adoption_prints_the_adopted_files_once() {
    let (tmp, home) = project();
    write_schema_1_install(tmp.path());

    let first = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "create-api-endpoint", "--claude"])
        .assert()
        .success();
    let stderr = String::from_utf8(first.get_output().stderr.clone()).unwrap();
    let expected = "note: adopted 1 files installed by an earlier upskill version as unchanged.\n\
                    Manual edits made to these files before now are not protected:\n  \
                    .claude/rules/api-conventions.md\n\
                    The schema 1 lockfile is copied to .upskill-lock.json.v1, \
                    unless that file already exists.\n";
    assert!(stderr.contains(expected), "stderr: {stderr}");

    let second = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "create-api-endpoint", "--claude"])
        .assert()
        .success();
    let stderr = String::from_utf8(second.get_output().stderr.clone()).unwrap();
    assert!(!stderr.contains("note: adopted"), "stderr: {stderr}");
}

#[cfg(unix)]
#[test]
fn add_does_not_write_through_a_symlink() {
    let (tmp, home) = project();
    let outside = tmp.path().join("outside.md");
    fs::write(&outside, "outside bytes\n").unwrap();
    let link = tmp.path().join(".claude/rules/api-conventions.md");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&outside, &link).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "api-conventions", "--claude"])
        .assert()
        .failure();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains(".claude/rules/api-conventions.md"),
        "stderr: {stderr}"
    );

    assert_eq!(fs::read(&outside).unwrap(), b"outside bytes\n");
}

/// Writes a skill `name` under `root/skills/<name>` with `body`.
fn write_skill(dir: &Path, name: &str, body: &str) {
    fs::create_dir_all(dir.join(name)).unwrap();
    fs::write(
        dir.join(name).join("SKILL.md"),
        format!("---\nschema: 1\nname: {name}\ndescription: Use for {name}\n---\n{body}\n"),
    )
    .unwrap();
}

fn stderr_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stderr.clone()).unwrap()
}

#[test]
fn exclude_matches_the_source_name_not_the_alias() {
    let (tmp, home) = project();
    let src = tmp.path().join("ab");
    write_skill(&src, "alpha", "Alpha body.");
    write_skill(&src, "beta", "Beta body.");

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./ab",
            "alpha",
            "beta",
            "--exclude",
            "beta",
            "--as",
            "alpha=beta",
            "--claude",
            "--project",
        ])
        .assert()
        .success();

    let written = fs::read_to_string(tmp.path().join(".claude/skills/beta/SKILL.md")).unwrap();
    assert!(written.contains("Alpha body."), "{written}");
    let lock = lockfile(tmp.path());
    let beta = item(&lock, "beta");
    assert_eq!(beta["source_name"], "alpha");
    assert!(!file_paths(beta).is_empty());
}

#[test]
fn excluded_item_is_neither_written_nor_recorded() {
    let (tmp, home) = project();
    let reg = tmp.path().join("reg");
    write_skill(&reg.join("skills"), "alpha", "Alpha body.");
    write_skill(&reg.join("skills"), "beta", "Beta body.");
    fs::create_dir_all(reg.join("bundles")).unwrap();
    fs::write(
        reg.join("bundles/team.bundle.yaml"),
        "schema: 1\nname: team\ndescription: Team bundle\nitems:\n  skills:\n    - alpha\n    - beta\n",
    )
    .unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./reg/bundles/team.bundle.yaml",
            "--exclude",
            "beta",
            "--claude",
        ])
        .assert()
        .success();

    assert!(tmp.path().join(".claude/skills/alpha/SKILL.md").exists());
    assert!(!tmp.path().join(".claude/skills/beta").exists());
    let lock = lockfile(tmp.path());
    assert!(
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["name"] != "beta"),
        "{lock}"
    );
}

#[test]
fn update_dry_run_does_not_adopt_a_schema_1_lockfile() {
    let (tmp, home) = project();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "create-api-endpoint", "--claude"])
        .assert()
        .success();
    downgrade_to_schema_1(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--dry-run"])
        .assert()
        .success();

    assert_eq!(lockfile(tmp.path())["schema"], 1);
    assert!(!tmp.path().join(".upskill-lock.json.v1").exists());
}

/// Rewrites the lockfile as schema 1 without `files` records.
fn downgrade_to_schema_1(root: &Path) {
    let mut lock = lockfile(root);
    lock["schema"] = 1.into();
    for entry in lock["items"].as_array_mut().unwrap() {
        entry.as_object_mut().unwrap().remove("files");
    }
    fs::write(
        root.join(".upskill-lock.json"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
fn adoption_notice_is_printed_when_add_fails_after_adoption() {
    let (tmp, home) = project();
    write_schema_1_install(tmp.path());
    let outside = tmp.path().join("outside.md");
    fs::write(&outside, "outside bytes\n").unwrap();
    let link = tmp
        .path()
        .join(".claude/skills/create-api-endpoint/SKILL.md");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&outside, &link).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "create-api-endpoint", "--claude"])
        .assert()
        .code(1);
    let stderr = stderr_of(&out);
    assert!(stderr.contains("note: adopted 1 files"), "stderr: {stderr}");
    assert!(
        stderr.contains("  .claude/rules/api-conventions.md\n"),
        "stderr: {stderr}"
    );
    assert_eq!(fs::read(&outside).unwrap(), b"outside bytes\n");
}

#[test]
fn update_prints_the_adoption_notice() {
    let (tmp, home) = project();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "create-api-endpoint", "--claude"])
        .assert()
        .success();
    downgrade_to_schema_1(tmp.path());
    let skill = tmp.path().join("source/create-api-endpoint/SKILL.md");
    let body = fs::read_to_string(&skill).unwrap();
    fs::write(&skill, format!("{body}\nChanged.\n")).unwrap();

    let first = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes"])
        .assert()
        .success();
    let stderr = stderr_of(&first);
    assert_eq!(
        stderr.matches("note: adopted").count(),
        1,
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("  .claude/skills/create-api-endpoint/SKILL.md\n"),
        "stderr: {stderr}"
    );

    let second = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes"])
        .assert()
        .success();
    assert!(!stderr_of(&second).contains("note: adopted"));
}

#[cfg(unix)]
#[test]
fn add_does_not_write_through_a_symlinked_item_directory() {
    let (tmp, home) = project();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("SKILL.md"), "outside bytes\n").unwrap();
    fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();
    std::os::unix::fs::symlink(
        &outside,
        tmp.path().join(".claude/skills/create-api-endpoint"),
    )
    .unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "create-api-endpoint", "--claude"])
        .assert()
        .failure();
    assert!(
        stderr_of(&out).contains(".claude/skills/create-api-endpoint"),
        "stderr: {}",
        stderr_of(&out)
    );
    assert_eq!(
        fs::read(outside.join("SKILL.md")).unwrap(),
        b"outside bytes\n"
    );
}

#[test]
fn add_refuses_two_items_aliased_to_one_name() {
    let (tmp, home) = project();
    let src = tmp.path().join("ab");
    write_skill(&src, "alpha", "Alpha body.");
    write_skill(&src, "beta", "Beta body.");

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add", "./ab", "alpha", "beta", "--as", "alpha=z", "--as", "beta=z", "--claude",
        ])
        .assert()
        .code(1);
    assert!(
        stderr_of(&out).contains(
            "two planned items install the skill `z`; each installed name must be unique"
        ),
        "stderr: {}",
        stderr_of(&out)
    );

    assert!(!tmp.path().join(".claude/skills/z").exists());
}

#[test]
fn a_file_the_lockfile_does_not_record_survives_add() {
    let (tmp, home) = project();
    let add = || {
        common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(["add", "./source", "create-api-endpoint", "--claude"])
            .assert()
            .success();
    };
    add();
    let notes = tmp
        .path()
        .join(".claude/skills/create-api-endpoint/NOTES.md");
    fs::write(&notes, "mine\n").unwrap();
    add();
    assert_eq!(fs::read(&notes).unwrap(), b"mine\n");
}

// ---------------------------------------------------------------------------
// `remove` deletes recorded, unchanged files only
// ---------------------------------------------------------------------------

fn add_claude(tmp: &Path, home: &Path, name: &str) {
    common::upskill_cmd(home)
        .current_dir(tmp)
        .args(["add", "./source", name, "--claude", "--project"])
        .assert()
        .success();
}

fn remove_cmd(tmp: &Path, home: &Path, args: &[&str]) -> assert_cmd::Command {
    let mut cmd = common::upskill_cmd(home);
    cmd.current_dir(tmp).arg("remove").args(args);
    cmd
}

fn lists_item(root: &Path, name: &str) -> bool {
    lockfile(root)["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["name"] == name)
}

#[test]
fn remove_keeps_a_manual_file_in_the_skill_directory() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "create-api-endpoint");
    let dir = tmp.path().join(".claude/skills/create-api-endpoint");
    fs::write(dir.join("NOTES.md"), "my notes\n").unwrap();

    remove_cmd(tmp.path(), &home, &["create-api-endpoint", "--project"])
        .assert()
        .success();

    assert!(!dir.join("SKILL.md").exists());
    assert_eq!(fs::read(dir.join("NOTES.md")).unwrap(), b"my notes\n");
    assert!(dir.is_dir());
}

#[test]
fn remove_deletes_empty_item_directories_but_not_client_roots() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "create-api-endpoint");
    fs::write(tmp.path().join(".claude/settings.json"), "{}\n").unwrap();

    remove_cmd(tmp.path(), &home, &["create-api-endpoint", "--project"])
        .assert()
        .success();

    assert!(
        !tmp.path()
            .join(".claude/skills/create-api-endpoint")
            .exists()
    );
    assert!(tmp.path().join(".claude/settings.json").exists());
    assert!(tmp.path().join(".claude/skills").is_dir());
}

#[test]
fn remove_keeps_the_kind_directory_and_client_root_when_empty() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "create-api-endpoint");

    remove_cmd(tmp.path(), &home, &["create-api-endpoint", "--project"])
        .assert()
        .success();

    assert!(
        !tmp.path()
            .join(".claude/skills/create-api-endpoint")
            .exists()
    );
    assert!(tmp.path().join(".claude/skills").is_dir());
    assert!(tmp.path().join(".claude").is_dir());
}

#[test]
fn remove_keeps_a_modified_file_and_reports_it() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "api-conventions");
    let rule = tmp.path().join(".claude/rules/api-conventions.md");
    let mut bytes = fs::read(&rule).unwrap();
    bytes.extend_from_slice(b"my local line\n");
    fs::write(&rule, &bytes).unwrap();

    let out = remove_cmd(tmp.path(), &home, &["api-conventions", "--project"])
        .assert()
        .success();

    assert_eq!(fs::read(&rule).unwrap(), bytes);
    let stderr = stderr_of(&out);
    assert!(
        stderr.contains("kept modified file: .claude/rules/api-conventions.md"),
        "stderr: {stderr}"
    );
    assert!(!lists_item(tmp.path(), "api-conventions"));
}

#[test]
fn remove_force_deletes_a_modified_file() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "api-conventions");
    let rule = tmp.path().join(".claude/rules/api-conventions.md");
    let mut bytes = fs::read(&rule).unwrap();
    bytes.extend_from_slice(b"my local line\n");
    fs::write(&rule, &bytes).unwrap();

    let out = remove_cmd(
        tmp.path(),
        &home,
        &["api-conventions", "--force", "--project"],
    )
    .assert()
    .success();

    assert!(!rule.exists());
    assert!(!stderr_of(&out).contains("kept modified file"));
    assert!(!lists_item(tmp.path(), "api-conventions"));
}

#[test]
fn remove_cleans_an_install_made_by_schema_1() {
    let (tmp, home) = project();
    write_schema_1_install(tmp.path());

    remove_cmd(tmp.path(), &home, &["api-conventions"])
        .assert()
        .success();

    assert!(!tmp.path().join(".claude/rules/api-conventions.md").exists());
    assert!(!lists_item(tmp.path(), "api-conventions"));
}

#[test]
fn remove_prints_the_adoption_notice() {
    let (tmp, home) = project();
    write_schema_1_install(tmp.path());

    let out = remove_cmd(tmp.path(), &home, &["api-conventions"])
        .assert()
        .success();

    let stderr = stderr_of(&out);
    assert_eq!(
        stderr.matches("note: adopted 1 files").count(),
        1,
        "{stderr}"
    );
    assert!(
        stderr.contains("  .claude/rules/api-conventions.md\n"),
        "{stderr}"
    );
    assert!(tmp.path().join(".upskill-lock.json.v1").exists());
    assert!(!tmp.path().join(".claude/rules/api-conventions.md").exists());
}

#[test]
fn remove_of_an_unknown_name_does_not_adopt() {
    let (tmp, home) = project();
    let original = write_schema_1_install(tmp.path());

    let out = remove_cmd(tmp.path(), &home, &["no-such-item"])
        .assert()
        .failure();

    let stderr = stderr_of(&out);
    assert!(stderr.contains("not in lockfile: no-such-item"), "{stderr}");
    assert!(!stderr.contains("note: adopted"), "{stderr}");
    assert!(!tmp.path().join(".upskill-lock.json.v1").exists());
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        original
    );
}

#[cfg(unix)]
#[test]
fn remove_does_not_delete_through_a_symlinked_item_directory() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "create-api-endpoint");
    let dir = tmp.path().join(".claude/skills/create-api-endpoint");
    let outside = tmp.path().join("outside");
    fs::rename(&dir, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &dir).unwrap();
    let before = fs::read(outside.join("SKILL.md")).unwrap();

    let out = remove_cmd(tmp.path(), &home, &["create-api-endpoint", "--project"])
        .assert()
        .failure();

    let stderr = stderr_of(&out);
    assert!(
        stderr.contains(".claude/skills/create-api-endpoint/SKILL.md"),
        "stderr: {stderr}"
    );
    assert_eq!(fs::read(outside.join("SKILL.md")).unwrap(), before);
    assert!(lists_item(tmp.path(), "create-api-endpoint"));
}

#[cfg(unix)]
#[test]
fn remove_force_keeps_a_symlink_at_a_recorded_path() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "api-conventions");
    let rule = tmp.path().join(".claude/rules/api-conventions.md");
    let outside = tmp.path().join("outside.md");
    fs::write(&outside, "outside bytes\n").unwrap();
    fs::remove_file(&rule).unwrap();
    std::os::unix::fs::symlink(&outside, &rule).unwrap();

    let out = remove_cmd(
        tmp.path(),
        &home,
        &["api-conventions", "--force", "--project"],
    )
    .assert()
    .success();

    assert!(
        fs::symlink_metadata(&rule)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&outside).unwrap(), b"outside bytes\n");
    assert!(
        stderr_of(&out).contains("kept modified file: .claude/rules/api-conventions.md"),
        "stderr: {}",
        stderr_of(&out)
    );
    assert!(!lists_item(tmp.path(), "api-conventions"));
}

#[test]
fn remove_of_an_item_with_an_empty_record_deletes_nothing() {
    let (tmp, home) = project();
    let rule = tmp.path().join(".claude/rules/api-conventions.md");
    fs::create_dir_all(rule.parent().unwrap()).unwrap();
    fs::write(&rule, "hand-written rule\n").unwrap();
    let lock = serde_json::json!({
        "schema": 2,
        "items": [{
            "kind": "rule",
            "name": "api-conventions",
            "source": "./source",
            "clients": ["claude"]
        }],
        "bundles": []
    });
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();

    remove_cmd(tmp.path(), &home, &["api-conventions"])
        .assert()
        .success();

    assert_eq!(fs::read(&rule).unwrap(), b"hand-written rule\n");
    assert!(!lists_item(tmp.path(), "api-conventions"));
}

#[test]
fn remove_drops_an_item_whose_directory_was_replaced_by_a_file() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "create-api-endpoint");
    let dir = tmp.path().join(".claude/skills/create-api-endpoint");
    fs::remove_dir_all(&dir).unwrap();
    fs::write(&dir, "not a directory\n").unwrap();

    remove_cmd(tmp.path(), &home, &["create-api-endpoint", "--project"])
        .assert()
        .success();

    assert_eq!(fs::read(&dir).unwrap(), b"not a directory\n");
    assert!(!lists_item(tmp.path(), "create-api-endpoint"));
}

#[cfg(unix)]
#[test]
fn remove_prints_the_adoption_notice_when_it_fails_after_adoption() {
    let (tmp, home) = project();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("SKILL.md"), "outside skill\n").unwrap();
    fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();
    std::os::unix::fs::symlink(
        &outside,
        tmp.path().join(".claude/skills/create-api-endpoint"),
    )
    .unwrap();
    let lock = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "skill",
            "name": "create-api-endpoint",
            "source": "./source",
            "clients": ["claude"]
        }],
        "bundles": []
    });
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();

    let out = remove_cmd(tmp.path(), &home, &["create-api-endpoint"])
        .assert()
        .failure()
        .code(1);

    let stderr = stderr_of(&out);
    let notice = stderr.find("note: adopted 1 files").expect(&stderr);
    let error = stderr.find("symbolic link").expect(&stderr);
    assert!(notice < error, "{stderr}");
    assert_eq!(
        fs::read(outside.join("SKILL.md")).unwrap(),
        b"outside skill\n"
    );
}

#[test]
fn remove_reports_a_path_recorded_by_two_items_once() {
    let (tmp, home) = project();
    let shared = tmp.path().join(".claude/rules/shared.md");
    fs::create_dir_all(shared.parent().unwrap()).unwrap();
    fs::write(&shared, "shared rule\n").unwrap();
    let record = serde_json::json!([{
        "client": "claude",
        "path": ".claude/rules/shared.md",
        "sha256": sha256_hex(b"shared rule\n")
    }]);
    let lock = serde_json::json!({
        "schema": 2,
        "items": [
            { "kind": "rule", "name": "first", "source": "./source",
              "clients": ["claude"], "files": record },
            { "kind": "rule", "name": "second", "source": "./source",
              "clients": ["claude"], "files": record }
        ],
        "bundles": []
    });
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();

    let out = remove_cmd(tmp.path(), &home, &["first", "second"])
        .assert()
        .success();

    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.matches("(1 file(s))").count(), 1, "{stdout}");
    assert_eq!(stdout.matches("(0 file(s))").count(), 1, "{stdout}");
    assert!(!shared.exists());
}

fn write_rule(dir: &Path, name: &str, body: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("RULE.md"),
        format!(
            "---\nschema: 1\nname: {name}\ndescription: Use when testing things\n---\n\n## Body\n\n{body}\n"
        ),
    )
    .unwrap();
}

#[test]
fn stale_file_of_one_item_does_not_delete_a_file_another_item_writes() {
    let (tmp, home) = project();
    let src = tmp.path().join("rules");
    write_rule(&src.join("a"), "a", "See [b](./b.md).");
    fs::write(src.join("a/b.md"), "resource\n").unwrap();
    write_rule(&src.join("0"), "zero", "other rule");
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./rules", "a", "--claude", "--project"])
        .assert()
        .success();

    // Rule `a` loses its resource `.claude/rules/a/b.md`; rule `zero`,
    // aliased to `a/b` and planned first, now writes that path.
    fs::remove_file(src.join("a/b.md")).unwrap();
    write_rule(&src.join("a"), "a", "no resource");
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./rules",
            "--as",
            "zero=a/b",
            "--claude",
            "--project",
        ])
        .assert()
        .success();

    let written = fs::read_to_string(tmp.path().join(".claude/rules/a/b.md")).unwrap();
    assert!(written.contains("other rule"), "{written}");
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor", "--project"])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Fix wave for PR #468
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn add_replaces_a_read_only_installed_file() {
    use std::os::unix::fs::PermissionsExt;
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "create-api-endpoint");
    let installed = tmp
        .path()
        .join(".claude/skills/create-api-endpoint/SKILL.md");
    fs::set_permissions(&installed, fs::Permissions::from_mode(0o444)).unwrap();
    let skill = tmp.path().join("source/create-api-endpoint/SKILL.md");
    let body = fs::read_to_string(&skill).unwrap();
    fs::write(&skill, format!("{body}\nChanged.\n")).unwrap();

    add_claude(tmp.path(), &home, "create-api-endpoint");

    assert!(
        fs::read_to_string(&installed).unwrap().contains("Changed."),
        "the installed file was not replaced"
    );
}

/// The fixture items staged under `source/`, with the fixture bundles
/// under `source/bundles/`.
fn project_with_bundles() -> (tempfile::TempDir, std::path::PathBuf) {
    let (tmp, home) = project();
    copy_dir_all(
        &Path::new(FIXTURES).join("bundles"),
        &tmp.path().join("source/bundles"),
    );
    (tmp, home)
}

#[test]
fn add_of_a_bundle_and_one_of_its_items_installs_the_item_once() {
    let (tmp, home) = project_with_bundles();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./source",
            "platform-baseline",
            "create-api-endpoint",
            "--claude",
        ])
        .assert()
        .success();

    assert!(
        tmp.path()
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
    let lock = lockfile(tmp.path());
    let count = lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["name"] == "create-api-endpoint")
        .count();
    assert_eq!(count, 1, "{lock}");
}

#[test]
fn two_items_aliased_to_one_name_do_not_adopt_a_schema_1_lockfile() {
    let (tmp, home) = project();
    let original = write_schema_1_install(tmp.path());
    let src = tmp.path().join("ab");
    write_skill(&src, "alpha", "Alpha body.");
    write_skill(&src, "beta", "Beta body.");

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add", "./ab", "alpha", "beta", "--as", "alpha=z", "--as", "beta=z", "--claude",
        ])
        .assert()
        .code(1);

    assert!(
        stderr_of(&out).contains("two planned items install the skill `z`"),
        "stderr: {}",
        stderr_of(&out)
    );
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        original
    );
    assert!(!tmp.path().join(".upskill-lock.json.v1").exists());
}

#[test]
fn adoption_ignores_an_output_path_below_a_regular_file() {
    let (tmp, home) = project();
    fs::create_dir_all(tmp.path().join(".claude")).unwrap();
    fs::write(tmp.path().join(".claude/skills"), "a regular file\n").unwrap();
    let lock = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "skill",
            "name": "create-api-endpoint",
            "source": "./source",
            "clients": ["claude"]
        }],
        "bundles": []
    });
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();

    remove_cmd(tmp.path(), &home, &["create-api-endpoint"])
        .assert()
        .success();

    assert!(!lists_item(tmp.path(), "create-api-endpoint"));
    assert_eq!(
        fs::read(tmp.path().join(".claude/skills")).unwrap(),
        b"a regular file\n"
    );
}

#[test]
fn conflict_check_uses_the_exact_alias_over_the_bare_alias() {
    let (tmp, home) = project();
    write_skill(&tmp.path().join("s1"), "bar", "First source.");
    write_skill(&tmp.path().join("s2"), "orig", "Second source.");
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./s1", "--claude"])
        .assert()
        .success();
    let installed = tmp.path().join(".claude/skills/bar/SKILL.md");
    let before = fs::read(&installed).unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./s2", "--as", "foo", "--as", "orig=bar", "--claude"])
        .assert()
        .code(1);

    assert_eq!(fs::read(&installed).unwrap(), before);
}

#[cfg(unix)]
fn make_unreadable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o000)).unwrap();
}

#[cfg(unix)]
#[test]
fn remove_keeps_an_unreadable_recorded_file_as_modified() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "api-conventions");
    add_claude(tmp.path(), &home, "license-awareness");
    let rule = tmp.path().join(".claude/rules/api-conventions.md");
    make_unreadable(&rule);
    if fs::read(&rule).is_ok() {
        // Running as root: the permission has no effect.
        return;
    }
    let other = tmp.path().join(".claude/rules/license-awareness.md");

    let out = remove_cmd(
        tmp.path(),
        &home,
        &["api-conventions", "license-awareness", "--project"],
    )
    .assert()
    .success();

    assert!(fs::symlink_metadata(&rule).is_ok(), "the file was deleted");
    assert!(!other.exists());
    assert!(
        stderr_of(&out).contains("kept modified file: .claude/rules/api-conventions.md"),
        "stderr: {}",
        stderr_of(&out)
    );
    assert!(!lists_item(tmp.path(), "api-conventions"));
}

#[cfg(unix)]
#[test]
fn remove_force_deletes_an_unreadable_recorded_file() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "api-conventions");
    let rule = tmp.path().join(".claude/rules/api-conventions.md");
    make_unreadable(&rule);
    if fs::read(&rule).is_ok() {
        // Running as root: the permission has no effect.
        return;
    }

    remove_cmd(
        tmp.path(),
        &home,
        &["api-conventions", "--force", "--project"],
    )
    .assert()
    .success();

    assert!(fs::symlink_metadata(&rule).is_err());
    assert!(!lists_item(tmp.path(), "api-conventions"));
}

/// A skill `tool` under `<root>/tools` with the resource `scripts/run.sh`
/// (mode 0755 on Unix).
fn write_skill_with_script(root: &Path) {
    let src = root.join("tools");
    write_skill(&src, "tool", "Run [the script](./scripts/run.sh).");
    let script = src.join("tool/scripts/run.sh");
    fs::create_dir_all(script.parent().unwrap()).unwrap();
    fs::write(&script, "#!/bin/sh\necho run\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn narrowing_the_clients_deletes_the_dropped_client_files() {
    let (tmp, home) = project();
    write_skill_with_script(tmp.path());
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./tools", "--claude", "--copilot", "--project"])
        .assert()
        .success();
    let copilot_script = tmp.path().join(".github/skills/tool/scripts/run.sh");
    assert!(copilot_script.exists());

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./tools", "--claude", "--project"])
        .assert()
        .success();

    assert!(!copilot_script.exists());
    assert!(!tmp.path().join(".github/skills/tool").exists());
    assert!(
        tmp.path()
            .join(".claude/skills/tool/scripts/run.sh")
            .exists()
    );
}

#[cfg(unix)]
#[test]
fn add_keeps_the_executable_bit_of_a_source_resource() {
    use std::os::unix::fs::PermissionsExt;
    let (tmp, home) = project();
    write_skill_with_script(tmp.path());

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./tools", "--claude", "--project"])
        .assert()
        .success();

    let mode = fs::metadata(tmp.path().join(".claude/skills/tool/scripts/run.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);
}

#[cfg(unix)]
#[test]
fn remove_of_two_items_fails_before_any_deletion_when_the_second_is_blocked() {
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "api-conventions");
    add_claude(tmp.path(), &home, "create-api-endpoint");
    let rule = tmp.path().join(".claude/rules/api-conventions.md");
    let dir = tmp.path().join(".claude/skills/create-api-endpoint");
    let outside = tmp.path().join("outside");
    fs::rename(&dir, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &dir).unwrap();

    remove_cmd(
        tmp.path(),
        &home,
        &["api-conventions", "create-api-endpoint", "--project"],
    )
    .assert()
    .failure();

    assert!(rule.exists(), "the first item was deleted");
    assert!(outside.join("SKILL.md").exists());
    assert!(lists_item(tmp.path(), "api-conventions"));
}

#[test]
fn remove_ignores_a_missing_recorded_file_and_prunes_its_directory() {
    let (tmp, home) = project();
    write_skill_with_script(tmp.path());
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./tools", "--claude", "--project"])
        .assert()
        .success();
    fs::remove_file(tmp.path().join(".claude/skills/tool/scripts/run.sh")).unwrap();

    remove_cmd(tmp.path(), &home, &["tool", "--project"])
        .assert()
        .success();

    assert!(!tmp.path().join(".claude/skills/tool").exists());
    assert!(!lists_item(tmp.path(), "tool"));
}

/// Installs the skill `bar` from `./s1` for Claude and returns its
/// installed bytes.
fn install_bar_from_another_source(tmp: &Path, home: &Path) -> Vec<u8> {
    write_skill(&tmp.join("s1"), "bar", "First source.");
    common::upskill_cmd(home)
        .current_dir(tmp)
        .args(["add", "./s1", "--claude"])
        .assert()
        .success();
    fs::read(tmp.join(".claude/skills/bar/SKILL.md")).unwrap()
}

#[test]
fn bundle_and_item_names_check_the_alias_for_conflicts() {
    let (tmp, home) = project_with_bundles();
    let before = install_bar_from_another_source(tmp.path(), &home);

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./source",
            "platform-baseline",
            "create-api-endpoint",
            "--as",
            "create-api-endpoint=bar",
            "--claude",
        ])
        .assert()
        .code(1);

    assert!(
        stderr_of(&out).contains("skill `bar` is already installed from"),
        "stderr: {}",
        stderr_of(&out)
    );
    assert_eq!(
        fs::read(tmp.path().join(".claude/skills/bar/SKILL.md")).unwrap(),
        before
    );
}

#[test]
fn bundle_and_item_names_check_the_exact_alias_over_the_bare_alias() {
    let (tmp, home) = project();
    let before = install_bar_from_another_source(tmp.path(), &home);
    let src = tmp.path().join("s2");
    write_skill(&src, "orig", "Second source.");
    fs::create_dir_all(src.join("bundles")).unwrap();
    fs::write(
        src.join("bundles/b.bundle.yaml"),
        "schema: 1\nname: b\ndescription: One skill\nitems:\n  skills:\n    - orig\n",
    )
    .unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add", "./s2", "b", "orig", "--as", "foo", "--as", "orig=bar", "--claude",
        ])
        .assert()
        .code(1);

    assert!(
        stderr_of(&out).contains("skill `bar` is already installed from"),
        "stderr: {}",
        stderr_of(&out)
    );
    assert_eq!(
        fs::read(tmp.path().join(".claude/skills/bar/SKILL.md")).unwrap(),
        before
    );
}

#[test]
fn the_first_alias_given_for_a_name_is_used() {
    let (tmp, home) = project();
    write_skill(&tmp.path().join("s"), "orig", "Body.");

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./s",
            "--as",
            "orig=first",
            "--as",
            "orig=second",
            "--claude",
        ])
        .assert()
        .success();

    assert!(tmp.path().join(".claude/skills/first/SKILL.md").exists());
    assert!(!tmp.path().join(".claude/skills/second").exists());
}

#[cfg(unix)]
#[test]
fn remove_fails_before_any_deletion_when_a_recorded_path_cannot_be_inspected() {
    use std::os::unix::fs::PermissionsExt;
    let (tmp, home) = project();
    add_claude(tmp.path(), &home, "api-conventions");
    add_claude(tmp.path(), &home, "create-api-endpoint");
    let rule = tmp.path().join(".claude/rules/api-conventions.md");
    let dir = tmp.path().join(".claude/skills/create-api-endpoint");
    make_unreadable(&dir);
    if fs::read_dir(&dir).is_ok() {
        // Running as root: the permission has no effect.
        return;
    }

    let out = remove_cmd(
        tmp.path(),
        &home,
        &["api-conventions", "create-api-endpoint", "--project"],
    )
    .assert()
    .code(1);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        stderr_of(&out).contains("cannot inspect the recorded file"),
        "stderr: {}",
        stderr_of(&out)
    );
    assert!(rule.exists(), "the first item was deleted");
    assert!(dir.join("SKILL.md").exists());
    assert!(lists_item(tmp.path(), "api-conventions"));
    assert!(lists_item(tmp.path(), "create-api-endpoint"));
}

/// A schema-1 lockfile with the rule `api-conventions`, and the skill `foo`
/// staged at `.claude/skills/foo`, an output path of `foo` itself.
fn schema_1_project_with_overlapping_source() -> (tempfile::TempDir, std::path::PathBuf, Vec<u8>) {
    let (tmp, home) = project();
    let original = write_schema_1_install(tmp.path());
    write_skill(
        &tmp.path().join(".claude").join("skills"),
        "foo",
        "Foo body.",
    );
    (tmp, home, original)
}

fn assert_overlap_refused_without_adoption(
    tmp: &tempfile::TempDir,
    out: &assert_cmd::assert::Assert,
    original: &[u8],
) {
    assert!(
        stderr_of(out).contains("overlaps the output path"),
        "stderr: {}",
        stderr_of(out)
    );
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        original
    );
    assert!(!tmp.path().join(".upskill-lock.json.v1").exists());
}

#[test]
fn add_refused_for_an_overlapping_source_does_not_adopt_a_schema_1_lockfile() {
    let (tmp, home, original) = schema_1_project_with_overlapping_source();

    // The source argument keeps `/`: the source parser reads `./` as a local
    // path on every platform, but not `.\`.
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./.claude/skills", "--claude", "--project"])
        .assert()
        .code(1);

    assert_overlap_refused_without_adoption(&tmp, &out, &original);
}

#[test]
fn update_refused_for_an_overlapping_source_does_not_adopt_a_schema_1_lockfile() {
    let (tmp, home, _) = schema_1_project_with_overlapping_source();
    let source = tmp.path().join(".claude").join("skills");
    let lock = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "skill",
            "name": "foo",
            "source": format!("local:{}", source.display()),
            "clients": ["claude"]
        }],
        "bundles": []
    });
    let original = serde_json::to_vec_pretty(&lock).unwrap();
    fs::write(tmp.path().join(".upskill-lock.json"), &original).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--project"])
        .assert()
        .code(1);

    assert_overlap_refused_without_adoption(&tmp, &out, &original);
}

/// Stages the single skill `foo` in a source directory below the output
/// directory of the skill `under`, and runs `add` with a bare `--as x` and
/// an exact `--as foo=y`. The exact alias wins, so `foo` installs as `y`.
fn add_with_bare_and_exact_alias(under: &str) -> (tempfile::TempDir, assert_cmd::assert::Assert) {
    let (tmp, home) = project();
    let source = tmp
        .path()
        .join(".claude")
        .join("skills")
        .join(under)
        .join("nested");
    write_skill(&source, "foo", "Foo body.");

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .arg("add")
        .arg(&source)
        .args(["--as", "x", "--as", "foo=y", "--claude", "--project"])
        .assert();
    (tmp, out)
}

#[test]
fn overlap_check_uses_the_exact_alias_over_the_bare_alias() {
    let (_tmp, out) = add_with_bare_and_exact_alias("y");
    let out = out.code(1);
    assert!(
        stderr_of(&out).contains("skill 'y'")
            && stderr_of(&out).contains("overlaps the output path"),
        "stderr: {}",
        stderr_of(&out)
    );
}

#[test]
fn overlap_check_ignores_the_bare_alias_that_the_exact_alias_replaces() {
    let (tmp, out) = add_with_bare_and_exact_alias("x");
    out.success();
    let skills = tmp.path().join(".claude").join("skills");
    assert!(skills.join("y").join("SKILL.md").is_file());
    assert!(!skills.join("x").join("SKILL.md").exists());
}
