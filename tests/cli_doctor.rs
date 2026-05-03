//! ATDD tests for `upskill doctor`.
//!
//! Verifies the three drift buckets per ADR-0004:
//! - missing per-client output files (reinstall fixes)
//! - SSOT hash drift on `local:` sources (update fixes)
//! - lockfile entries with no recoverable source (manual remove)
//!
//! Doctor never fetches; remote-source drift detection is `update --dry-run`.

use assert_cmd::Command;
use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn stage_source(source: &Path) {
    for kind in ["skills", "rules", "agents"] {
        let from = format!("{FIXTURES}/{kind}");
        let to = source.join(kind);
        copy_dir_all(Path::new(&from), &to).unwrap();
    }
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

fn install(target: &Path, source: &Path) {
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn doctor_clean_install_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["doctor"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("doctor: clean"), "expected clean: {out}");
}

#[test]
fn doctor_empty_lockfile_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success();
}

#[test]
fn doctor_detects_missing_output_files() {
    // Install, then delete one rendered output behind upskill's back —
    // the bucket-1 path. Doctor must exit 1 and name the missing file.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    let stolen = target.join(".claude/skills/create-api-endpoint/SKILL.md");
    fs::remove_file(&stolen).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["doctor"])
        .assert()
        .failure()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("missing per-client outputs") && out.contains("create-api-endpoint"),
        "expected missing-output report: {out}"
    );
    assert!(
        out.contains(".claude/skills/create-api-endpoint/SKILL.md"),
        "expected the missing path listed: {out}"
    );
}

#[test]
fn doctor_detects_ssot_hash_drift() {
    // Install from a local source, mutate the SSOT in place — the
    // bucket-2 path. Doctor must exit 1 and surface the stale-hash bucket.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    let skill_md = source.join("skills/create-api-endpoint/SKILL.md");
    let original = fs::read_to_string(&skill_md).unwrap();
    fs::write(&skill_md, format!("{original}\n<!-- mutation -->\n")).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["doctor"])
        .assert()
        .failure()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("SSOT hash drift") && out.contains("create-api-endpoint"),
        "expected stale-hash bucket: {out}"
    );
    assert!(
        out.contains("upskill update"),
        "should suggest the fix command: {out}"
    );
}

#[test]
fn doctor_detects_orphan_when_local_path_gone() {
    // Install from a local source, remove the source dir — the bucket-3
    // path. Doctor must exit 1 and surface every entry as orphan.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    fs::remove_dir_all(&source).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["doctor"])
        .assert()
        .failure()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("no recoverable source"),
        "expected orphan bucket: {out}"
    );
    assert!(out.contains("local path gone"), "should explain why: {out}");
    assert!(
        out.contains("upskill remove"),
        "should suggest the fix command: {out}"
    );
}

#[test]
fn doctor_detects_orphan_when_item_removed_from_source() {
    // Install from a local source, remove just one item dir from the
    // SSOT (leaving the source root). Doctor must report that one item
    // as orphan with "item not in source", and other items stay clean.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    fs::remove_dir_all(source.join("skills/create-api-endpoint")).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["doctor"])
        .assert()
        .failure()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("create-api-endpoint") && out.contains("item not in source"),
        "expected orphan with reason: {out}"
    );
}
