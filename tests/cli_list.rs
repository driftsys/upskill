//! ATDD tests for `upskill list`.
//!
//! Reads `.upskill-lock.json` and prints installed items grouped by kind.
//! Bundles, when present, are surfaced as a separate section. The command
//! never touches per-client output files or fetches anything — it is a
//! lockfile dump.

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
fn list_empty_lockfile_reports_no_items() {
    let tmp = tempfile::tempdir().unwrap();
    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["list"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("no items installed"),
        "expected empty message: {out}"
    );
}

#[test]
fn list_groups_installed_items_by_kind() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    install(&target, &source);

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["list"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // Grouped section headers per kind. Counts are kind-specific.
    assert!(
        out.contains("rules (2)"),
        "expected rules section with count: {out}"
    );
    assert!(
        out.contains("skills (1)"),
        "expected skills section with count: {out}"
    );
    assert!(
        out.contains("agents (1)"),
        "expected agents section with count: {out}"
    );

    // Each item appears under its section with the resolved source label.
    for name in [
        "license-awareness",
        "api-conventions",
        "create-api-endpoint",
        "security-reviewer",
    ] {
        assert!(out.contains(name), "expected {name} in list: {out}");
    }
    // local: source label flows through to the printout.
    assert!(out.contains("local:"), "expected source label: {out}");
}

#[test]
fn list_bundle_install_surfaces_bundles_section() {
    // Use the bundle fixture corpus so the install records a LockedBundle
    // entry alongside the per-item entries.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&target).unwrap();

    // Stage SSOT items + bundle file.
    stage_source(&source);
    fs::create_dir_all(source.join("bundles")).unwrap();
    fs::copy(
        format!("{FIXTURES}/bundles/platform-baseline.bundle.md"),
        source.join("bundles/platform-baseline.bundle.md"),
    )
    .unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args([
            "add",
            source
                .join("bundles/platform-baseline.bundle.md")
                .to_str()
                .unwrap(),
        ])
        .assert()
        .success();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["list"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        out.contains("bundles (1)"),
        "expected bundles section: {out}"
    );
    assert!(
        out.contains("platform-baseline"),
        "expected bundle name in output: {out}"
    );
}
