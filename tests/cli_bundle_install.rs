//! ATDD tests for `upskill add <bundle-file>` (bundle install path).
//!
//! `install_with_lockfile` detects when its source resolves to a
//! `.bundle.yaml` file, walks up to the registry root, discovers sibling
//! bundles, resolves transitively, and installs only the items the
//! resolution names. The lockfile records the entry bundle and every
//! transitive dependency.

mod common;

use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// Stage `tests/fixtures/items` and `tests/fixtures/bundles` into a
/// fresh registry root so each test starts with a clean copy. Items
/// land at the root level (format-spec §2.1, ADR-0006); bundles keep
/// their `bundles/` subdirectory.
fn stage_registry(root: &Path) {
    copy_dir_all(Path::new(&format!("{FIXTURES}/items")), root).unwrap();
    copy_dir_all(
        Path::new(&format!("{FIXTURES}/bundles")),
        &root.join("bundles"),
    )
    .unwrap();
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

#[test]
fn bundle_install_writes_only_referenced_items() {
    // `platform-baseline.bundle.yaml` references every fixture item, so a
    // bundle install renders the same files as a directory install.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    let bundle = registry.join("bundles/platform-baseline.bundle.yaml");
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", bundle.to_str().unwrap()])
        .assert()
        .success();

    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "skill installed"
    );
    assert!(
        target
            .join(".github/instructions/api-conventions.instructions.md")
            .exists(),
        "rule installed"
    );
    assert!(
        target
            .join(".opencode/agents/security-reviewer.md")
            .exists(),
        "agent installed"
    );
}

#[test]
fn bundle_install_records_bundle_in_lockfile() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    let bundle = registry.join("bundles/platform-baseline.bundle.yaml");
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", bundle.to_str().unwrap()])
        .assert()
        .success();

    let raw = fs::read_to_string(target.join(".upskill-lock.json")).unwrap();
    let lock: serde_json::Value = serde_json::from_str(&raw).unwrap();

    let bundles = lock["bundles"].as_array().expect("bundles array");
    assert_eq!(bundles.len(), 1, "one bundle resolved");
    assert_eq!(bundles[0]["name"], "platform-baseline");
    let items = bundles[0]["items"].as_array().expect("bundle items array");
    assert_eq!(items.len(), 4, "1 skill + 2 rules + 1 agent");

    // Item entries land in the regular per-item array too.
    let names: Vec<&str> = lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"create-api-endpoint"));
    assert!(names.contains(&"api-conventions"));
    assert!(names.contains(&"license-awareness"));
    assert!(names.contains(&"security-reviewer"));
}

#[test]
fn bundle_install_resolves_transitive_requires() {
    // `platform-extras` requires `platform-baseline`. Installing extras
    // should record both bundles in the lockfile and install items from
    // both bundles' union.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // `extras` has rule `license-awareness`; baseline has the rest.
    // Mark them disjoint so the union is exact (no conflict).
    // Adjust the extras fixture in-place: drop overlap with baseline.
    let extras_path = registry.join("bundles/platform-extras.bundle.yaml");
    let extras = fs::read_to_string(&extras_path).unwrap();
    // Normalise line endings for Windows compatibility (git autocrlf).
    let extras = extras.replace("\r\n", "\n");
    // The fixture lists `license-awareness` which overlaps with baseline.
    // Replace with a non-overlapping rule so the resolver succeeds.
    fs::write(
        &extras_path,
        extras.replace(
            "items:\n  rules:\n    - license-awareness",
            "items:\n  rules: []",
        ),
    )
    .unwrap();

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", extras_path.to_str().unwrap()])
        .assert()
        .success();

    let raw = fs::read_to_string(target.join(".upskill-lock.json")).unwrap();
    let lock: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let bundles = lock["bundles"].as_array().expect("bundles array");
    let names: Vec<&str> = bundles
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"platform-extras") && names.contains(&"platform-baseline"),
        "transitive bundles recorded: {names:?}"
    );

    // Items from baseline must be installed too — extras alone has no rules.
    assert!(
        target
            .join(".github/instructions/api-conventions.instructions.md")
            .exists(),
        "baseline rule installed via transitive resolution"
    );
}

#[test]
fn bundle_install_errors_on_item_conflict() {
    // The fixture's `extras` lists `license-awareness` which baseline
    // also lists — that's a (kind=rule, name=license-awareness) conflict
    // when extras requires baseline. Resolution must fail.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    let extras = registry.join("bundles/platform-extras.bundle.yaml");
    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", extras.to_str().unwrap()])
        .assert()
        .failure()
        .code(1);

    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("item conflict") && stderr.contains("license-awareness"),
        "expected item-conflict error: {stderr}"
    );
}

/// Stage a registry with the **sibling layout**: bundles and items live
/// in separate peer directories under the registry root. This is the
/// layout described in format-spec §2.2 and used by metapowers.
///
/// ```text
/// registry/
/// ├── bundles/
/// │   └── sibling-test.bundle.yaml
/// └── skills/
///     └── license-awareness/
///         └── RULE.md
/// ```
fn stage_sibling_layout_registry(root: &Path) {
    // bundles/ subdir
    let bundles_dir = root.join("bundles");
    fs::create_dir_all(&bundles_dir).unwrap();
    fs::write(
        bundles_dir.join("sibling-test.bundle.yaml"),
        "\
schema: 1
name: sibling-test
description: Test bundle with sibling layout
license: MIT

items:
  rules:
    - license-awareness
",
    )
    .unwrap();

    // skills/ subdir containing the item
    let item_dir = root.join("skills/license-awareness");
    fs::create_dir_all(&item_dir).unwrap();
    fs::copy(
        Path::new(&format!("{FIXTURES}/items/license-awareness/RULE.md")),
        item_dir.join("RULE.md"),
    )
    .unwrap();
}

#[test]
fn bundle_install_sibling_layout_discovers_items_in_peer_directories() {
    // Regression test for #161: when bundles/ and skills/ are siblings
    // under the registry root, `find_registry_root` must detect the
    // registry root and `install_*` must find items in subdirectories.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_sibling_layout_registry(&registry);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    let bundle = registry.join("bundles/sibling-test.bundle.yaml");
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", bundle.to_str().unwrap()])
        .assert()
        .success();

    // The rule must be installed for all clients.
    assert!(
        target
            .join(".github/instructions/license-awareness.instructions.md")
            .exists(),
        "copilot rule installed from sibling layout"
    );
    assert!(
        target
            .join(".agents/rules/license-awareness/RULE.md")
            .exists(),
        "opencode rule installed from sibling layout"
    );

    // Lockfile must record the bundle and item.
    let raw = fs::read_to_string(target.join(".upskill-lock.json")).unwrap();
    let lock: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let items: Vec<&str> = lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert!(
        items.contains(&"license-awareness"),
        "lockfile records installed item: {items:?}"
    );
}
