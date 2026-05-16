//! ATDD tests for `upskill registry build [--check]`.
//!
//! Author command: generates `.upskill-registry.json` from `REGISTRY.md`
//! plus a tree scan. `--check` exits non-zero when the manifest is
//! stale. Refuses to run inside a consumer project (ADR-0007 / ADR-0004).

use assert_cmd::Command;
use std::fs;

fn registry_md(dir: &std::path::Path) {
    fs::write(
        dir.join("REGISTRY.md"),
        "---\nschema: 1\nname: platform-registry\ndescription: Baseline\n---\nbody\n",
    )
    .unwrap();
}

#[test]
fn registry_build_writes_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    registry_md(tmp.path());
    fs::create_dir(tmp.path().join("alpha")).unwrap();
    fs::write(
        tmp.path().join("alpha/RULE.md"),
        "---\nschema: 1\nname: alpha\ndescription: a rule\n---\nbody\n",
    )
    .unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["registry", "build"])
        .assert()
        .success();

    let manifest = fs::read_to_string(tmp.path().join(".upskill-registry.json")).unwrap();
    assert!(
        manifest.contains("\"name\": \"platform-registry\""),
        "{manifest}"
    );
    assert!(manifest.contains("\"name\": \"alpha\""), "{manifest}");
    assert!(manifest.ends_with("}\n"), "trailing newline: {manifest:?}");
}

#[test]
fn registry_build_check_fails_when_stale_and_passes_when_fresh() {
    let tmp = tempfile::tempdir().unwrap();
    registry_md(tmp.path());

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["registry", "build", "--check"])
        .assert()
        .failure()
        .code(1);

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["registry", "build"])
        .assert()
        .success();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["registry", "build", "--check"])
        .assert()
        .success();
}

#[test]
fn registry_build_refuses_consumer_project() {
    let tmp = tempfile::tempdir().unwrap();
    // Presence of .upskill-lock.json marks this as a consumer project; build must refuse.
    fs::write(tmp.path().join(".upskill-lock.json"), "{}").unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["registry", "build"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn registry_build_matches_golden_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/registry");
    for entry in walkdir(&src) {
        let rel = entry.strip_prefix(&src).unwrap();
        let dest = tmp.path().join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&dest).unwrap();
        } else if entry.file_name().unwrap() != "expected-manifest.json" {
            fs::create_dir_all(dest.parent().unwrap()).unwrap();
            fs::copy(&entry, &dest).unwrap();
        }
    }

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["registry", "build"])
        .assert()
        .success();

    let got = fs::read_to_string(tmp.path().join(".upskill-registry.json")).unwrap();
    let want = fs::read_to_string(src.join("expected-manifest.json")).unwrap();
    assert_eq!(got, want, "manifest drifted from golden fixture");
}

fn walkdir(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = vec![root.to_path_buf()];
    if root.is_dir() {
        for e in fs::read_dir(root).unwrap().flatten() {
            out.extend(walkdir(&e.path()));
        }
    }
    out
}
