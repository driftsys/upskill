//! Regression tests for issue #196: `upskill update` misfiring on
//! bundle-sourced items.
//!
//! When content is installed from a `*.bundle.yaml` source, the lockfile
//! records each item's source as the bundle file (e.g.
//! `local:/abs/path/my-bundle.bundle.yaml`). `update` must re-resolve that
//! bundle source the same way `add` did — through bundle resolution — and
//! must NOT report the freshly-installed items with a `remove` line ("no
//! longer in source"). These tests use a local-path bundle source so no
//! network is involved, and pin every command to `--project` scope so the
//! consumer tempdir is the only lockfile touched (never the real `$HOME`).

mod common;

use std::fs;
use std::path::{Path, PathBuf};

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

fn write_skill(root: &Path, name: &str) {
    let body = format!(
        "---\nschema: 1\nname: {name}\ndescription: bundle item {name} for the issue 196 regression.\n---\n\n## Body\n\nText for {name}.\n"
    );
    write(&root.join(name).join("SKILL.md"), &body);
}

/// Lay out a registry with two skills plus a bundle naming both. Returns
/// the absolute path to the bundle manifest.
fn setup_registry(registry: &Path) -> PathBuf {
    write_skill(registry, "alpha");
    write_skill(registry, "beta");
    let bundle = registry.join("my-bundle.bundle.yaml");
    write(
        &bundle,
        "schema: 1\nname: my-bundle\ndescription: bundle for the issue 196 regression.\nitems:\n  skills:\n    - alpha\n    - beta\n",
    );
    bundle
}

/// Fake `$HOME` for a consumer: a `home/` subdir inside the consumer tempdir.
/// Commands are `--project`-scoped, so this is purely the belt-and-suspenders
/// safety net that keeps any stray global write out of the real `$HOME`.
fn fake_home(consumer: &Path) -> PathBuf {
    let home = consumer.join("home");
    fs::create_dir_all(&home).unwrap();
    home
}

fn add_bundle(consumer: &Path, bundle: &Path) {
    common::upskill_cmd(&fake_home(consumer))
        .current_dir(consumer)
        .args(["add", bundle.to_str().unwrap(), "--project"])
        .assert()
        .success();
}

#[test]
fn update_dry_run_does_not_report_bundle_items_as_removed() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let bundle = setup_registry(registry.path());

    add_bundle(consumer.path(), &bundle);

    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--dry-run", "--project"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        !out.contains("\n  remove ") && !out.contains("no longer in source"),
        "bundle items must not be scheduled for removal: {out}"
    );
    assert!(
        out.contains("up to date") || out.contains("up-to-date"),
        "freshly installed bundle items must read as up to date: {out}"
    );
}

#[test]
fn update_dry_run_named_bundle_item_is_not_removed() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let bundle = setup_registry(registry.path());

    add_bundle(consumer.path(), &bundle);

    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--dry-run", "--project", "alpha"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        !out.contains("\n  remove ") && !out.contains("no longer in source"),
        "named bundle item must not be scheduled for removal: {out}"
    );
}

#[test]
fn update_apply_does_not_delete_bundle_items() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let bundle = setup_registry(registry.path());

    add_bundle(consumer.path(), &bundle);

    // A real (apply) update must reinstall the bundle, not delete its items.
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();

    // Both items' generated Claude output must still be present.
    for name in ["alpha", "beta"] {
        let out = consumer
            .path()
            .join(format!(".claude/skills/{name}/SKILL.md"));
        assert!(
            out.exists(),
            "bundle item {name} output must survive `update`: {}",
            out.display()
        );
    }
}

fn setup_kind_registry(registry: &Path) {
    write_skill(registry, "a");
    write_skill(registry, "b");
    write_rule_body(registry, "r", "Original rule body.");
    write_kind_bundle(registry, false);
}

fn write_rule_body(registry: &Path, name: &str, body: &str) {
    write(
        &registry.join(name).join("RULE.md"),
        &format!("---\nname: {name}\ndescription: A bundle rule.\n---\n{body}\n"),
    );
}

fn write_kind_bundle(registry: &Path, second_rule: bool) {
    let rules = if second_rule { "[r, r2]" } else { "[r]" };
    write(
        &registry.join("bun.bundle.yaml"),
        &format!(
            "schema: 1\nname: bun\ndescription: A mixed bundle.\nitems:\n  skills: [a]\n  rules: {rules}\n"
        ),
    );
}

fn add_named_bundle(consumer: &Path, registry: &Path, kinds: &[&str]) {
    common::upskill_cmd(&fake_home(consumer))
        .current_dir(consumer)
        .args([
            "add",
            registry.to_str().unwrap(),
            "bun",
            "--project",
            "--claude",
        ])
        .args(kinds)
        .assert()
        .success();
}

fn update_named_bundle(consumer: &Path) {
    common::upskill_cmd(&fake_home(consumer))
        .current_dir(consumer)
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
}

#[test]
fn update_does_not_install_an_item_of_the_source_that_the_bundle_does_not_list() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    assert!(!consumer.path().join(".claude/skills/b").exists());
    update_named_bundle(consumer.path());
    assert!(consumer.path().join(".claude/skills/a/SKILL.md").exists());
    assert!(consumer.path().join(".claude/rules/r.md").exists());
    assert!(!consumer.path().join(".claude/skills/b").exists());
}

#[test]
fn update_keeps_the_kind_restriction_of_add() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
    write_rule_body(registry.path(), "r", "Changed rule body.");
    write(
        &registry.path().join("a/SKILL.md"),
        "---\nname: a\ndescription: A changed skill.\n---\nChanged skill body.\n",
    );
    update_named_bundle(consumer.path());
    assert!(
        fs::read_to_string(consumer.path().join(".claude/rules/r.md"))
            .unwrap()
            .contains("Changed rule body.")
    );
    assert!(!consumer.path().join(".claude/skills").exists());
    let lock: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(consumer.path().join(".upskill-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(lock["bundles"][0]["kinds"], serde_json::json!(["rule"]));
    assert!(lock.get("plugins").is_none());
    assert!(lock.get("mcps").is_none());
}

#[test]
fn update_installs_a_rule_that_the_bundle_gained() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
    write_rule_body(registry.path(), "r2", "New rule body.");
    write_kind_bundle(registry.path(), true);
    update_named_bundle(consumer.path());
    assert!(
        fs::read_to_string(consumer.path().join(".claude/rules/r2.md"))
            .unwrap()
            .contains("New rule body.")
    );
    assert!(!consumer.path().join(".claude/skills").exists());
}

#[test]
fn update_keeps_an_item_installed_directly_beside_a_bundle_of_the_same_source() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            registry.path().to_str().unwrap(),
            "b",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    update_named_bundle(consumer.path());
    assert!(consumer.path().join(".claude/skills/b/SKILL.md").exists());
    let lock: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(consumer.path().join(".upskill-lock.json")).unwrap(),
    )
    .unwrap();
    assert!(
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["name"] == "b" && item["kind"] == "skill")
    );
}
