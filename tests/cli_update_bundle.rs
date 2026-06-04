//! Regression tests for issue #196: `upskill update` misfiring on
//! bundle-sourced items.
//!
//! When content is installed from a `*.bundle.yaml` source, the lockfile
//! records each item's source as the bundle file (e.g.
//! `local:/abs/path/my-bundle.bundle.yaml`). `update` must re-resolve that
//! bundle source the same way `add` did — through bundle resolution — and
//! must NOT report the freshly-installed items as "would remove — no
//! longer in source". These tests use a local-path bundle source so no
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
        !out.contains("would remove") && !out.contains("no longer in source"),
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
        !out.contains("would remove") && !out.contains("no longer in source"),
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
