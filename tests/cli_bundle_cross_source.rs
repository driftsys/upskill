//! ATDD tests for cross-source bundle `requires` (ADR-0009).
//!
//! A bundle MAY depend on a bundle living in another source via a
//! `{ name, source }` entry in `requires`. Installing the entry bundle pulls
//! in the required bundle's items from that other source; the lockfile records
//! each bundle and each item under its OWN source.

mod common;

use std::fs;
use std::path::Path;

/// Write a rule item at `<root>/<name>/RULE.md`.
fn write_rule(root: &Path, name: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let body =
        format!("---\nschema: 1\nname: {name}\ndescription: test rule {name}\n---\n# {name}\n");
    fs::write(dir.join("RULE.md"), body).unwrap();
}

/// Write a bundle manifest at `<root>/bundles/<name>.bundle.yaml`.
fn write_bundle(root: &Path, name: &str, body: &str) {
    let dir = root.join("bundles");
    fs::create_dir_all(&dir).unwrap();
    let content = format!("schema: 1\nname: {name}\ndescription: test bundle {name}\n{body}");
    fs::write(dir.join(format!("{name}.bundle.yaml")), content).unwrap();
}

#[test]
fn add_bundle_pulls_cross_source_required_bundle() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // Source DEP: holds the leaf bundle `dep` and its item.
    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "dep-rule");
    write_bundle(&reg_dep, "dep", "items:\n  rules:\n  - dep-rule\n");

    // Source META: bundle `meta` requires `dep` from the DEP source by path.
    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    let bundle = reg_meta.join("bundles/meta.bundle.yaml");
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", bundle.to_str().unwrap()])
        .assert()
        .success();

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();

    // The entry source label for a bundle-file `add` is the file path itself;
    // the cross-source dependency records its OWN (registry) source.
    let entry_src = format!("local:{}", bundle.display());
    let dep_src = format!("local:{}", reg_dep.display());

    let items = lock["items"].as_array().unwrap();
    let find = |name: &str| {
        items
            .iter()
            .find(|i| i["kind"] == "rule" && i["name"] == name)
            .unwrap_or_else(|| panic!("rule {name} must be installed: {lock}"))
    };
    assert_eq!(find("meta-rule")["source"], entry_src);
    assert_eq!(find("dep-rule")["source"], dep_src);

    // Both bundles recorded, each under its OWN source.
    let bundles = lock["bundles"].as_array().unwrap();
    let bundle_src = |name: &str| {
        bundles
            .iter()
            .find(|b| b["name"] == name)
            .unwrap_or_else(|| panic!("bundle {name} must be recorded: {lock}"))["source"]
            .clone()
    };
    assert_eq!(bundle_src("meta"), entry_src);
    assert_eq!(bundle_src("dep"), dep_src);
}

#[test]
fn add_bundle_by_name_pulls_cross_source_required_bundle() {
    // Same as above, but via name resolution (`add <source> meta`) rather
    // than a bundle-file path — the seed scenario.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "dep-rule");
    write_bundle(&reg_dep, "dep", "items:\n  rules:\n  - dep-rule\n");

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", reg_meta.to_str().unwrap(), "meta"])
        .assert()
        .success();

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();
    let items = lock["items"].as_array().unwrap();
    let has = |name: &str| {
        items
            .iter()
            .any(|i| i["kind"] == "rule" && i["name"] == name)
    };
    assert!(has("meta-rule"), "entry bundle item installed: {lock}");
    assert!(
        has("dep-rule"),
        "cross-source required bundle item installed: {lock}"
    );
}

#[test]
fn add_bundle_cross_source_dep_with_same_source_subdeps() {
    // The seed/metapowers shape: a cross-source required bundle whose OWN
    // requires are bare (same-source) names within its own source.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // DEP source: bundle `dep` bare-requires sibling `leaf` (same source).
    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "dep-rule");
    write_rule(&reg_dep, "leaf-rule");
    write_bundle(&reg_dep, "leaf", "items:\n  rules:\n  - leaf-rule\n");
    write_bundle(
        &reg_dep,
        "dep",
        "items:\n  rules:\n  - dep-rule\nrequires:\n  - { name: leaf }\n",
    );

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", reg_meta.to_str().unwrap(), "meta"])
        .assert()
        .success();

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();
    let items = lock["items"].as_array().unwrap();
    let has = |name: &str| {
        items
            .iter()
            .any(|i| i["kind"] == "rule" && i["name"] == name)
    };
    assert!(has("meta-rule"), "entry item: {lock}");
    assert!(has("dep-rule"), "cross-source dep item: {lock}");
    assert!(
        has("leaf-rule"),
        "same-source sub-dependency of the cross-source bundle: {lock}"
    );
}

#[test]
fn add_bundle_cross_source_cycle_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    let reg_a = tmp.path().join("reg-a");
    let reg_b = tmp.path().join("reg-b");
    write_rule(&reg_a, "a-rule");
    write_rule(&reg_b, "b-rule");
    write_bundle(
        &reg_a,
        "a",
        &format!(
            "items:\n  rules:\n  - a-rule\nrequires:\n  - {{ name: b, source: {} }}\n",
            reg_b.display()
        ),
    );
    write_bundle(
        &reg_b,
        "b",
        &format!(
            "items:\n  rules:\n  - b-rule\nrequires:\n  - {{ name: a, source: {} }}\n",
            reg_a.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", reg_a.to_str().unwrap(), "a"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cycle"));
}

#[test]
fn add_bundle_cross_source_item_conflict_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // Both bundles provide a rule named `shared` from different sources.
    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "shared");
    write_bundle(&reg_dep, "dep", "items:\n  rules:\n  - shared\n");

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "shared");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - shared\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            reg_meta.join("bundles/meta.bundle.yaml").to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("conflict"));
}

#[test]
fn add_bundle_missing_cross_source_bundle_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // DEP source exists but does NOT contain a bundle named `dep`.
    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "unrelated");

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    let bundle = reg_meta.join("bundles/meta.bundle.yaml");
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", bundle.to_str().unwrap()])
        .assert()
        .failure();
}
