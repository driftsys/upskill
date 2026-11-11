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
                    The previous lockfile is kept as .upskill-lock.json.v1.\n";
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

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add", "./ab", "alpha", "beta", "--as", "alpha=z", "--as", "beta=z", "--claude",
        ])
        .assert()
        .failure();

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
