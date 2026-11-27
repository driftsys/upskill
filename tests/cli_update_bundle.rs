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
#[cfg(unix)]
fn update_keeps_the_kind_restriction_of_add() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;

    setup_kind_registry(registry.path());
    let bundle_path = registry.path().join("bun.bundle.yaml");
    let mut bundle = fs::read_to_string(&bundle_path).unwrap();
    bundle.push_str("plugins:\n  tool:\n    claude:\n      source: example/tools\n      marketplace: tools\n      plugin: tool\nmcps:\n  server:\n    local:\n      command: example-server\n");
    write(&bundle_path, &bundle);
    let bin = consumer.path().join("bin");
    let marker = consumer.path().join("claude-called");
    let claude = bin.join("claude");
    write(
        &claude,
        "#!/bin/sh\nprintf 'called\\n' >> \"$CLAUDE_MARKER\"\n",
    );
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let path = std::env::join_paths(paths).unwrap();
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
    write_rule_body(registry.path(), "r", "Changed rule body.");
    write(
        &registry.path().join("a/SKILL.md"),
        "---\nname: a\ndescription: A changed skill.\n---\nChanged skill body.\n",
    );
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .env("PATH", path)
        .env("CLAUDE_MARKER", &marker)
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(!marker.exists());
    assert!(!consumer.path().join(".mcp.json").exists());
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

#[test]
fn update_keeps_a_direct_item_of_another_kind_beside_a_kind_restricted_bundle() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
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
    let skill_path = consumer.path().join(".claude/skills/b/SKILL.md");
    let old_body = fs::read_to_string(&skill_path).unwrap();
    let lock_path = consumer.path().join(".upskill-lock.json");
    let before: serde_json::Value = serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    let old_entry = before["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "b" && item["kind"] == "skill")
        .unwrap();
    write_rule_body(registry.path(), "r", "Changed rule body.");
    write(
        &registry.path().join("b/SKILL.md"),
        "---\nname: b\ndescription: A changed skill.\n---\nChanged skill body.\n",
    );
    update_named_bundle(consumer.path());
    assert!(
        fs::read_to_string(consumer.path().join(".claude/rules/r.md"))
            .unwrap()
            .contains("Changed rule body.")
    );
    assert_eq!(fs::read_to_string(&skill_path).unwrap(), old_body);
    let after: serde_json::Value = serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    assert_eq!(
        after["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == "b" && item["kind"] == "skill")
            .unwrap(),
        old_entry
    );
}

/// A direct member of a vanished bundle that changed in the source stays and
/// is updated; the member that was installed only through the bundle goes.
#[test]
fn update_updates_a_direct_member_of_a_vanished_bundle() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    add_direct(consumer.path(), registry.path(), "a");
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    write(
        &registry.path().join("a/SKILL.md"),
        "---\nname: a\ndescription: A changed skill.\n---\nChanged skill body.\n",
    );
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    let body = fs::read_to_string(consumer.path().join(".claude/skills/a/SKILL.md")).unwrap();
    assert!(body.contains("Changed skill body."), "{body}");
    assert!(!consumer.path().join(".claude/rules/r.md").exists());
    assert!(
        read_update_lock(consumer.path())["bundles"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

/// Every item of a vanished bundle is also provided by a bundle that stays:
/// no item changes, and the record of the vanished bundle is still dropped.
#[test]
fn update_drops_a_vanished_bundle_whose_items_another_bundle_provides() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    write(
        &registry.path().join("keep.bundle.yaml"),
        "schema: 1\nname: keep\ndescription: A bundle that stays.\nitems:\n  skills: [a]\n  rules: [r]\n",
    );
    add_named_bundle(consumer.path(), registry.path(), &[]);
    add_direct(consumer.path(), registry.path(), "keep");
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    let lock = read_update_lock(consumer.path());
    let bundles: Vec<_> = lock["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(bundles, ["keep"], "{lock}");
    assert!(consumer.path().join(".claude/skills/a/SKILL.md").exists());
}

/// `update <name>` for one member of a vanished bundle removes that member
/// and keeps the record while another non-direct member stays installed; the
/// next full update removes that member and drops the record.
#[test]
fn update_by_name_of_a_member_of_a_vanished_bundle_keeps_the_record_until_every_member_is_reconciled()
 {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    // The record stays, so the preview does not announce its removal either.
    let dry = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "a", "--dry-run", "--project"])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&dry.get_output().stdout).to_string();
    assert!(!out.contains("bundle bun"), "{out}");
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "a", "--yes", "--project"])
        .assert()
        .success();
    assert!(!consumer.path().join(".claude/skills/a").exists());
    assert!(consumer.path().join(".claude/rules/r.md").exists());
    let lock = read_update_lock(consumer.path());
    assert_eq!(lock["bundles"].as_array().unwrap().len(), 1, "{lock}");
    update_named_bundle(consumer.path());
    assert!(!consumer.path().join(".claude/rules/r.md").exists());
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
}

/// A kind-restricted bundle vanishes while a skill of the same source is
/// installed directly: the run is not an error, and the skill stays.
#[test]
fn update_accepts_a_vanished_kind_restricted_bundle_beside_a_direct_skill() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
    add_direct(consumer.path(), registry.path(), "b");
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(consumer.path().join(".claude/skills/b/SKILL.md").exists());
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
}

/// A member of a vanished bundle that left the source and whose output file
/// was edited: the item record is removed as for `remove`, the edited file is
/// kept on disk, and the record of the bundle is dropped.
#[test]
fn update_drops_a_vanished_bundle_whose_removed_member_has_an_edited_file() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    let skill = consumer.path().join(".claude/skills/a/SKILL.md");
    fs::write(&skill, "edited by hand\n").unwrap();
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    fs::remove_dir_all(registry.path().join("a")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert_eq!(fs::read_to_string(&skill).unwrap(), "edited by hand\n");
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
    assert!(
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["name"] != "a"),
        "{lock}"
    );
}

/// An item that was installed on purpose from a hidden directory
/// (`add <source> .x`) stays installed: the scan skips hidden directories,
/// but the item is named in the lockfile and its file exists.
#[test]
fn update_keeps_a_hidden_item_that_was_installed_by_name() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    write_skill(registry.path(), ".x");
    add_direct(consumer.path(), registry.path(), "a");
    add_direct(consumer.path(), registry.path(), ".x");
    assert!(consumer.path().join(".claude/skills/.x/SKILL.md").is_file());
    for args in [
        vec!["update", ".x", "--yes", "--project"],
        vec!["update", "--yes", "--project"],
    ] {
        common::upskill_cmd(&fake_home(consumer.path()))
            .current_dir(consumer.path())
            .args(&args)
            .assert()
            .success();
        assert!(
            consumer.path().join(".claude/skills/.x/SKILL.md").is_file(),
            "{args:?}"
        );
        assert!(
            consumer.path().join(".claude/skills/a/SKILL.md").is_file(),
            "{args:?}"
        );
    }
}

/// A rule whose frontmatter is no longer valid YAML stops `update` with the
/// parse error. The rule lives in `folder/` and is named `policy`, so a
/// scan that fell back to the folder name would not find it and would remove
/// it as no longer in the source.
#[test]
fn update_stops_when_the_frontmatter_of_an_installed_rule_stops_parsing() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    write(
        &registry.path().join("folder/RULE.md"),
        "---\nname: policy\ndescription: A named rule.\n---\nBody.\n",
    );
    add_direct(consumer.path(), registry.path(), "policy");
    write(
        &registry.path().join("folder/RULE.md"),
        "---\nname: policy\ndescription: [unclosed\n---\nBody.\n",
    );
    let output_before = fs::read(consumer.path().join(".claude/rules/policy.md")).unwrap();
    let lock_before = fs::read(consumer.path().join(".upskill-lock.json")).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "policy", "--yes", "--project"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("parse"), "{stderr}");
    assert_eq!(
        fs::read(consumer.path().join(".claude/rules/policy.md")).unwrap(),
        output_before
    );
    assert_eq!(
        fs::read(consumer.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}

/// An item added directly and also listed by a bundle that vanishes counts as
/// a direct install: it stays, and only the bundle record goes.
#[test]
fn update_keeps_an_item_that_was_added_directly_and_by_a_vanished_bundle() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    add_direct(consumer.path(), registry.path(), "a");
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(consumer.path().join(".claude/skills/a/SKILL.md").exists());
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
}

/// A skill with the name of the vanished bundle does not keep the bundle: the
/// record is dropped, and the skill `bun` is not installed.
#[test]
fn update_drops_a_vanished_bundle_even_when_an_item_has_its_name() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    write_skill(registry.path(), "bun");
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
    assert!(!consumer.path().join(".claude/skills/bun").exists());
}

/// A source that holds only a bundle file (no item) still holds content: a
/// recorded direct item that left the source is removed.
#[test]
fn update_removes_an_item_when_the_source_holds_only_a_bundle_file() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    add_direct(consumer.path(), registry.path(), "a");
    fs::remove_dir_all(registry.path().join("a")).unwrap();
    write(
        &registry.path().join("empty.bundle.yaml"),
        "schema: 1\nname: empty\ndescription: No items.\nitems: {}\n",
    );
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(!consumer.path().join(".claude/skills/a").exists());
}

/// Two bundles require the same dependency; one vanishes. The dependency
/// keeps only the requirer that stays.
#[test]
fn update_removes_a_vanished_requirer_from_the_required_by_of_a_dependency() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    for name in ["one-skill", "two-skill", "dep-skill"] {
        write_skill(registry.path(), name);
    }
    write(
        &registry.path().join("dep.bundle.yaml"),
        "schema: 1\nname: dep\ndescription: Leaf.\nitems:\n  skills: [dep-skill]\n",
    );
    for (name, skill) in [("one", "one-skill"), ("two", "two-skill")] {
        write(
            &registry.path().join(format!("{name}.bundle.yaml")),
            &format!(
                "schema: 1\nname: {name}\ndescription: Root.\nitems:\n  skills: [{skill}]\nrequires:\n  - {{ name: dep }}\n"
            ),
        );
        add_direct(consumer.path(), registry.path(), name);
    }
    let required_by = |lock: &serde_json::Value| -> Vec<String> {
        let dep = lock["bundles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == "dep")
            .unwrap();
        dep["required_by"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(
        required_by(&read_update_lock(consumer.path())),
        ["bundle:one", "bundle:two"]
    );
    fs::remove_file(registry.path().join("one.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert_eq!(
        required_by(&read_update_lock(consumer.path())),
        ["bundle:two"]
    );
}

/// The strict bundle search reads directories that the item walk does not
/// reach: a bundle below an unreadable directory is not "vanished".
#[cfg(unix)]
#[test]
fn update_stops_when_the_bundle_search_meets_an_unreadable_directory() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    write(
        &registry.path().join("p/q/r/bun.bundle.yaml"),
        "schema: 1\nname: bun\ndescription: Deep.\nitems:\n  skills: [a]\n",
    );
    add_named_bundle(consumer.path(), registry.path(), &[]);
    let locked = registry.path().join("p/q/r");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read_dir(&locked).is_ok() {
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    assert_update_fails_unchanged(
        consumer.path(),
        ".claude/skills/a/SKILL.md",
        &[(&locked, 0o755)],
    );
}

/// A category directory that cannot be listed but can be searched (mode 0311)
/// stops `update`: the strict walk does not skip it.
#[cfg(unix)]
#[test]
fn update_stops_when_a_category_directory_cannot_be_listed_but_can_be_searched() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    write_skill(&registry.path().join("cat"), "b");
    for name in ["a", "b"] {
        add_direct(consumer.path(), registry.path(), name);
    }
    let dir = registry.path().join("cat");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o311)).unwrap();
    if fs::read_dir(&dir).is_ok() {
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    assert_update_fails_unchanged(
        consumer.path(),
        ".claude/skills/b/SKILL.md",
        &[(&dir, 0o755)],
    );
}

/// A source root that lists but whose children cannot be inspected (mode
/// 0444) stops `update` with the inspection error, not with the "resolved to
/// no items" guard.
#[cfg(unix)]
#[test]
fn update_reports_the_inspection_error_of_a_source_root_that_cannot_be_searched() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    add_direct(consumer.path(), registry.path(), "a");
    let root = registry.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o444)).unwrap();
    if fs::metadata(root.join("a")).is_ok() {
        fs::set_permissions(root, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    fs::set_permissions(root, fs::Permissions::from_mode(0o755)).unwrap();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert_eq!(assert.get_output().status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("inspect"), "{stderr}");
    assert!(!stderr.contains("resolved to no items"), "{stderr}");
}

/// A member of a vanished bundle that an installed item requires stays, and
/// so does its requirer: the member is resolved through the requirer.
#[test]
fn update_keeps_a_member_of_a_vanished_bundle_that_an_installed_item_requires() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    for name in ["a", "k", "c"] {
        write_skill(registry.path(), name);
    }
    write(
        &registry.path().join("old.bundle.yaml"),
        "schema: 1\nname: old\ndescription: Vanishes.\nitems:\n  skills: [a, k]\n",
    );
    write(
        &registry.path().join("c/SKILL.md"),
        "---\nschema: 1\nname: c\ndescription: Requires a.\nrequires:\n  skills: [a]\n---\nBody.\n",
    );
    add_direct(consumer.path(), registry.path(), "old");
    add_direct(consumer.path(), registry.path(), "c");
    fs::remove_file(registry.path().join("old.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(consumer.path().join(".claude/skills/a/SKILL.md").exists());
    assert!(consumer.path().join(".claude/skills/c/SKILL.md").exists());
    assert!(!consumer.path().join(".claude/skills/k").exists());
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
}

/// `add <source> <name>` marks an item that a bundle installed earlier as
/// direct; the bundle member that was not named stays not direct. A whole
/// directory source without names marks every item.
#[test]
fn add_records_the_direct_flag_for_named_and_whole_source_installs() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    let direct = |lock: &serde_json::Value, name: &str| -> bool {
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["name"] == name)
            .unwrap_or_else(|| panic!("{name} not installed: {lock}"))
            .get("direct")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    };
    add_named_bundle(consumer.path(), registry.path(), &[]);
    let lock = read_update_lock(consumer.path());
    assert!(!direct(&lock, "a") && !direct(&lock, "r"), "{lock}");
    assert!(
        lock["items"][0].get("direct").is_none(),
        "false is omitted: {lock}"
    );
    add_direct(consumer.path(), registry.path(), "a");
    let lock = read_update_lock(consumer.path());
    assert!(direct(&lock, "a") && !direct(&lock, "r"), "{lock}");
    // update keeps the recorded value.
    update_named_bundle(consumer.path());
    let lock = read_update_lock(consumer.path());
    assert!(direct(&lock, "a") && !direct(&lock, "r"), "{lock}");
    // A whole directory source without names.
    let whole = tempfile::tempdir().unwrap();
    write_skill(whole.path(), "w");
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            whole.path().to_str().unwrap(),
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert!(direct(&read_update_lock(consumer.path()), "w"));
}

/// A cross-source `requires` dependency of an item of a whole directory
/// source installed without names is not recorded `direct`; the item of that
/// source is.
#[test]
fn add_of_a_whole_directory_source_does_not_record_a_cross_source_dependency_direct() {
    let consumer = tempfile::tempdir().unwrap();
    let whole = tempfile::tempdir().unwrap();
    let dep = tempfile::tempdir().unwrap();
    write_skill(dep.path(), "a");
    write(
        &whole.path().join("c/SKILL.md"),
        &format!(
            "---\nschema: 1\nname: c\ndescription: Requires a.\nrequires:\n  skills: [{{ name: a, source: {} }}]\n---\nBody.\n",
            dep.path().display()
        ),
    );
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            whole.path().to_str().unwrap(),
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    let lock = read_update_lock(consumer.path());
    let direct = |name: &str| -> bool {
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["name"] == name)
            .unwrap_or_else(|| panic!("{name} not installed: {lock}"))
            .get("direct")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    };
    assert!(direct("c"), "{lock}");
    assert!(!direct("a"), "{lock}");
}

/// A direct member of a vanished bundle that also left the source is
/// removed as no longer in source, and the run succeeds: the name that the
/// recovery requests is dropped from the request when the source no longer
/// holds it.
#[test]
fn update_removes_a_direct_member_that_left_the_source_with_its_vanished_bundle() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_direct(consumer.path(), registry.path(), "a");
    add_direct(consumer.path(), registry.path(), "r");
    add_named_bundle(consumer.path(), registry.path(), &[]);
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    fs::remove_dir_all(registry.path().join("a")).unwrap();

    let out = update_output(consumer.path());

    assert!(removed_line(&out, " a "), "{out}");
    assert!(!consumer.path().join(".claude/skills/a").exists());
    assert!(consumer.path().join(".claude/rules/r.md").exists());
    assert_eq!(recorded_items(consumer.path()), ["r"]);
    assert!(recorded_bundles(consumer.path()).is_empty());
}

/// An item of the vanished bundle's source that another source requires:
/// the source of `c` is processed first and refreshes `a`; the source of the
/// vanished bundle must not delete `a` afterwards.
#[test]
fn update_keeps_a_member_of_a_vanished_bundle_that_an_item_of_another_source_requires() {
    let consumer = tempfile::tempdir().unwrap();
    let roots = tempfile::tempdir().unwrap();
    // The labels sort `aaa` (the requirer) before `zzz` (the bundle).
    let bundle_source = roots.path().join("zzz");
    let requirer_source = roots.path().join("aaa");
    write_skill(&bundle_source, "a");
    write(
        &bundle_source.join("old.bundle.yaml"),
        "schema: 1\nname: old\ndescription: Vanishes.\nitems:\n  skills: [a]\n",
    );
    write(
        &requirer_source.join("c/SKILL.md"),
        &format!(
            "---\nschema: 1\nname: c\ndescription: Requires a.\nrequires:\n  skills: [{{ name: a, source: {} }}]\n---\nBody.\n",
            bundle_source.display()
        ),
    );
    add_direct(consumer.path(), &bundle_source, "old");
    add_direct(consumer.path(), &requirer_source, "c");
    fs::remove_file(bundle_source.join("old.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(consumer.path().join(".claude/skills/a/SKILL.md").exists());
    assert!(consumer.path().join(".claude/skills/c/SKILL.md").exists());
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
}

/// A direct item that a dependency bundle also lists stays when the root and
/// the dependency bundle both vanish.
#[test]
fn update_keeps_a_direct_item_that_a_dropped_dependency_bundle_lists() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    for name in ["a", "t"] {
        write_skill(registry.path(), name);
    }
    write(
        &registry.path().join("dep.bundle.yaml"),
        "schema: 1\nname: dep\ndescription: Leaf.\nitems:\n  skills: [a]\n",
    );
    write(
        &registry.path().join("top.bundle.yaml"),
        "schema: 1\nname: top\ndescription: Root.\nitems:\n  skills: [t]\nrequires:\n  - { name: dep }\n",
    );
    add_direct(consumer.path(), registry.path(), "a");
    add_direct(consumer.path(), registry.path(), "top");
    fs::remove_file(registry.path().join("dep.bundle.yaml")).unwrap();
    fs::remove_file(registry.path().join("top.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(consumer.path().join(".claude/skills/a/SKILL.md").exists());
    assert!(!consumer.path().join(".claude/skills/t").exists());
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
}

/// A direct item that a dependency bundle lists stays when only the
/// dependency bundle is dropped (the root stops requiring it).
#[test]
fn update_keeps_a_direct_item_when_a_root_stops_requiring_its_dependency_bundle() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    for name in ["a", "t"] {
        write_skill(registry.path(), name);
    }
    write(
        &registry.path().join("dep.bundle.yaml"),
        "schema: 1\nname: dep\ndescription: Leaf.\nitems:\n  skills: [a]\n",
    );
    let top = |requires: &str| {
        format!("schema: 1\nname: top\ndescription: Root.\nitems:\n  skills: [t]\n{requires}")
    };
    write(
        &registry.path().join("top.bundle.yaml"),
        &top("requires:\n  - { name: dep }\n"),
    );
    add_direct(consumer.path(), registry.path(), "a");
    add_direct(consumer.path(), registry.path(), "top");
    write(&registry.path().join("top.bundle.yaml"), &top(""));
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(consumer.path().join(".claude/skills/a/SKILL.md").exists());
    let lock = read_update_lock(consumer.path());
    let names: Vec<_> = lock["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["top"], "{lock}");
}

/// A hidden item with a readable rule and a skill that became a symlink loop:
/// the closure-free name resolution is walked strictly too.
#[cfg(unix)]
#[test]
fn update_stops_for_a_looping_entrypoint_of_a_hidden_named_item() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    write_skill(registry.path(), ".x");
    write(
        &registry.path().join(".x/RULE.md"),
        "---\nname: .x\ndescription: A rule.\n---\nBody.\n",
    );
    add_direct(consumer.path(), registry.path(), "a");
    add_direct(consumer.path(), registry.path(), ".x");
    assert!(consumer.path().join(".claude/skills/.x/SKILL.md").is_file());
    let skill = registry.path().join(".x/SKILL.md");
    fs::remove_file(&skill).unwrap();
    std::os::unix::fs::symlink(&skill, &skill).unwrap();
    let lock_before = fs::read(consumer.path().join(".upskill-lock.json")).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    assert_eq!(assert.get_output().status.code(), Some(1));
    assert_eq!(
        fs::read(consumer.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert!(consumer.path().join(".claude/skills/.x/SKILL.md").is_file());
}

/// A bundle lists the rule and the skill `shared`; `add --kind rule shared`
/// installs the rule directly. When the bundle vanishes, only the rule stays.
#[test]
fn update_keeps_only_the_direct_kind_of_a_name_that_two_kinds_share() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "shared");
    write(
        &registry.path().join("shared/RULE.md"),
        "---\nname: shared\ndescription: A rule.\n---\nBody.\n",
    );
    write(
        &registry.path().join("both.bundle.yaml"),
        "schema: 1\nname: both\ndescription: Both kinds.\nitems:\n  rules: [shared]\n  skills: [shared]\n",
    );
    add_direct(consumer.path(), registry.path(), "both");
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            registry.path().to_str().unwrap(),
            "shared",
            "--kind",
            "rule",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert!(
        consumer
            .path()
            .join(".claude/skills/shared/SKILL.md")
            .exists()
    );
    fs::remove_file(registry.path().join("both.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(consumer.path().join(".claude/rules/shared.md").exists());
    assert!(!consumer.path().join(".claude/skills/shared").exists());
}

/// A direct item of a source whose bundle `dep` is reached only through a
/// root of another source: when the root stops requiring `dep`, the direct
/// item stays.
#[test]
fn update_keeps_a_direct_item_of_a_dependency_only_source() {
    let consumer = tempfile::tempdir().unwrap();
    let roots = tempfile::tempdir().unwrap();
    let dep_source = roots.path().join("dep-source");
    let root_source = roots.path().join("root-source");
    write_skill(&dep_source, "a");
    write(
        &dep_source.join("dep.bundle.yaml"),
        "schema: 1\nname: dep\ndescription: Leaf.\nitems:\n  skills: [a]\n",
    );
    write_skill(&root_source, "own");
    let top = |requires: &str| {
        format!("schema: 1\nname: top\ndescription: Root.\nitems:\n  skills: [own]\n{requires}")
    };
    write(
        &root_source.join("top.bundle.yaml"),
        &top(&format!(
            "requires:\n  - {{ name: dep, source: {} }}\n",
            dep_source.display()
        )),
    );
    add_direct(consumer.path(), &dep_source, "a");
    add_direct(consumer.path(), &root_source, "top");
    write(&root_source.join("top.bundle.yaml"), &top(""));
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(consumer.path().join(".claude/skills/a/SKILL.md").exists());
    assert!(consumer.path().join(".claude/skills/own/SKILL.md").exists());
}

/// A hidden direct item sends the run through the name resolution; the
/// recorded kind of the shared name still holds there.
#[test]
fn update_keeps_the_direct_kind_of_a_shared_name_beside_a_hidden_direct_item() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "shared");
    write_skill(registry.path(), ".x");
    write(
        &registry.path().join("shared/RULE.md"),
        "---\nname: shared\ndescription: A rule.\n---\nBody.\n",
    );
    write(
        &registry.path().join("both.bundle.yaml"),
        "schema: 1\nname: both\ndescription: Both kinds.\nitems:\n  rules: [shared]\n  skills: [shared]\n",
    );
    add_direct(consumer.path(), registry.path(), "both");
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            registry.path().to_str().unwrap(),
            "shared",
            "--kind",
            "rule",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    add_direct(consumer.path(), registry.path(), ".x");
    fs::remove_file(registry.path().join("both.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(consumer.path().join(".claude/rules/shared.md").exists());
    assert!(!consumer.path().join(".claude/skills/shared").exists());
    assert!(consumer.path().join(".claude/skills/.x/SKILL.md").exists());
}

/// A stale `required_by` (its requirer was removed) does not explain a
/// member: `update a` keeps the vanished record while `b` stays.
#[test]
fn update_keeps_the_record_of_a_vanished_bundle_when_a_member_has_only_a_stale_requirer() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    for name in ["a", "b"] {
        write_skill(registry.path(), name);
    }
    write(
        &registry.path().join("c/SKILL.md"),
        "---\nschema: 1\nname: c\ndescription: Requires b.\nrequires:\n  skills: [b]\n---\nBody.\n",
    );
    write(
        &registry.path().join("bun.bundle.yaml"),
        "schema: 1\nname: bun\ndescription: Two skills.\nitems:\n  skills: [a, b]\n",
    );
    add_direct(consumer.path(), registry.path(), "bun");
    add_direct(consumer.path(), registry.path(), "c");
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["remove", "c", "--project"])
        .assert()
        .success();
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "a", "--yes", "--project"])
        .assert()
        .success();
    assert!(!consumer.path().join(".claude/skills/a").exists());
    assert!(consumer.path().join(".claude/skills/b/SKILL.md").exists());
    let lock = read_update_lock(consumer.path());
    assert_eq!(lock["bundles"].as_array().unwrap().len(), 1, "{lock}");
}

/// True when the process cannot create a file in the read-only `dir`, that
/// is, when permissions are enforced (not the case for root).
#[cfg(unix)]
fn permissions_enforced(dir: &Path) -> bool {
    let probe = dir.join(".probe");
    let created = fs::write(&probe, "x").is_ok();
    let _ = fs::remove_file(&probe);
    !created
}

/// A removal that fails keeps the record of the vanished bundle.
#[cfg(unix)]
#[test]
fn update_keeps_the_record_of_a_vanished_bundle_when_a_removal_fails() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    fs::remove_dir_all(registry.path().join("a")).unwrap();
    let dir = consumer.path().join(".claude/skills/a");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    if !permissions_enforced(&dir) {
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let out = assert.get_output().clone();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        stderr.to_lowercase().contains("permission denied"),
        "{stderr}"
    );
    let lock = read_update_lock(consumer.path());
    assert_eq!(lock["bundles"].as_array().unwrap().len(), 1, "{lock}");
}

/// A bundle directory that cannot be read is not a vanished bundle: `update`
/// fails before any write and keeps the files and the records.
#[cfg(unix)]
#[test]
fn update_fails_when_the_bundle_directory_of_the_source_is_unreadable() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    write(
        &registry.path().join("bundles/bun.bundle.yaml"),
        "schema: 1\nname: bun\ndescription: In a directory.\nitems:\n  skills: [a]\n",
    );
    add_named_bundle(consumer.path(), registry.path(), &[]);
    let lock_before = fs::read(consumer.path().join(".upskill-lock.json")).unwrap();
    let skill_before = fs::read(consumer.path().join(".claude/skills/a/SKILL.md")).unwrap();
    let dir = registry.path().join("bundles");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read_dir(&dir).is_ok() {
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let out = assert.get_output().clone();
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        fs::read(consumer.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert_eq!(
        fs::read(consumer.path().join(".claude/skills/a/SKILL.md")).unwrap(),
        skill_before
    );
}

/// As the test above, with a bundle directory that lists but whose entries
/// cannot be inspected (mode 0444: no search permission).
#[cfg(unix)]
#[test]
fn update_fails_when_the_bundle_directory_of_the_source_cannot_be_searched() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    write(
        &registry.path().join("bundles/bun.bundle.yaml"),
        "schema: 1\nname: bun\ndescription: In a directory.\nitems:\n  skills: [a]\n",
    );
    add_named_bundle(consumer.path(), registry.path(), &[]);
    let lock_before = fs::read(consumer.path().join(".upskill-lock.json")).unwrap();
    let skill_before = fs::read(consumer.path().join(".claude/skills/a/SKILL.md")).unwrap();
    let dir = registry.path().join("bundles");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o444)).unwrap();
    // Permissions are enforced when the entry cannot be inspected.
    if fs::metadata(dir.join("bun.bundle.yaml")).is_ok() {
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let out = assert.get_output().clone();
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        fs::read(consumer.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert_eq!(
        fs::read(consumer.path().join(".claude/skills/a/SKILL.md")).unwrap(),
        skill_before
    );
}

/// An item subdirectory that cannot be read does not make its recorded items
/// "no longer in source": `update` fails before any write.
#[cfg(unix)]
#[test]
fn update_fails_when_a_category_directory_of_the_source_is_unreadable() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    write_skill(&registry.path().join("cat"), "b");
    for name in ["a", "b"] {
        add_direct(consumer.path(), registry.path(), name);
    }
    let lock_before = fs::read(consumer.path().join(".upskill-lock.json")).unwrap();
    let skill_before = fs::read(consumer.path().join(".claude/skills/b/SKILL.md")).unwrap();
    let dir = registry.path().join("cat");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read_dir(&dir).is_ok() {
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let out = assert.get_output().clone();
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        fs::read(consumer.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert_eq!(
        fs::read(consumer.path().join(".claude/skills/b/SKILL.md")).unwrap(),
        skill_before
    );
}

/// Runs `update --yes` after `chmod`, with the lockfile and `output` bytes
/// compared before and after; the update must fail.
#[cfg(unix)]
fn assert_update_fails_unchanged(consumer: &Path, output: &str, restore: &[(&Path, u32)]) {
    use std::os::unix::fs::PermissionsExt;
    let lock_before = fs::read(consumer.join(".upskill-lock.json")).unwrap();
    let output_before = fs::read(consumer.join(output)).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer))
        .current_dir(consumer)
        .args(["update", "--yes", "--project"])
        .assert();
    for (path, mode) in restore {
        fs::set_permissions(path, fs::Permissions::from_mode(*mode)).unwrap();
    }
    let out = assert.get_output().clone();
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        fs::read(consumer.join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert_eq!(fs::read(consumer.join(output)).unwrap(), output_before);
}

/// A category directory that lists but whose entries cannot be inspected
/// (mode 0444) must not make its recorded items "no longer in source".
#[cfg(unix)]
#[test]
fn update_fails_when_the_items_of_a_category_directory_cannot_be_inspected() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    write_skill(&registry.path().join("cat"), "b");
    for name in ["a", "b"] {
        add_direct(consumer.path(), registry.path(), name);
    }
    let dir = registry.path().join("cat");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o444)).unwrap();
    if fs::metadata(dir.join("b/SKILL.md")).is_ok() {
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    assert_update_fails_unchanged(
        consumer.path(),
        ".claude/skills/b/SKILL.md",
        &[(&dir, 0o755)],
    );
}

/// Installs the skills `a` and `h` from a registry, then moves `h` to
/// `dest_rel` (a hidden directory: the scan of `add` does not read those, but
/// the plan walk does) and sets `mode` on `locked_rel`. `update` must stop
/// before any write when the permission is enforced (probed with `probe`).
#[cfg(unix)]
fn assert_update_stops_for_unreadable(dest_rel: &str, locked_rel: &str, mode: u32, probe: &str) {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    write_skill(registry.path(), "h");
    for name in ["a", "h"] {
        add_direct(consumer.path(), registry.path(), name);
    }
    assert!(consumer.path().join(".claude/skills/h/SKILL.md").is_file());
    let dest = registry.path().join(dest_rel);
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    fs::rename(registry.path().join("h"), &dest).unwrap();
    let locked = registry.path().join(locked_rel);
    fs::set_permissions(&locked, fs::Permissions::from_mode(mode)).unwrap();
    if fs::metadata(registry.path().join(probe)).is_ok() {
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let lock_before = fs::read(consumer.path().join(".upskill-lock.json")).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(assert.get_output().status.code(), Some(1));
    assert_eq!(
        fs::read(consumer.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert!(consumer.path().join(".claude/skills/h/SKILL.md").is_file());
}

/// (a) A hidden directory that lists but whose children cannot be inspected.
#[cfg(unix)]
#[test]
fn update_stops_when_the_children_of_a_hidden_directory_cannot_be_inspected() {
    assert_update_stops_for_unreadable(".extra/h", ".extra", 0o444, ".extra/h/SKILL.md");
}

/// (b) An unsearchable item directory inside a hidden directory.
#[cfg(unix)]
#[test]
fn update_stops_when_an_item_directory_in_a_hidden_directory_cannot_be_searched() {
    assert_update_stops_for_unreadable(".extra/h", ".extra/h", 0o000, ".extra/h/SKILL.md");
}

/// A bundle-file source whose member `beta` is in an unsearchable directory
/// stops `update` before any write. This checks the outcome only: the bundle
/// resolution already fails on the unreadable member, so the strict walk of
/// the registry root is not what stops the run here.
#[cfg(unix)]
#[test]
fn update_stops_for_an_unsearchable_item_directory_of_a_bundle_file_source() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "alpha");
    write_skill(registry.path(), "beta");
    let bundle = registry.path().join("two.bundle.yaml");
    write(
        &bundle,
        "schema: 1\nname: two\ndescription: Two skills.\nitems:\n  skills: [alpha, beta]\n",
    );
    add_bundle(consumer.path(), &bundle);
    assert!(
        consumer
            .path()
            .join(".claude/skills/beta/SKILL.md")
            .is_file()
    );
    let locked = registry.path().join("beta");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::metadata(locked.join("SKILL.md")).is_ok() {
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let lock_before = fs::read(consumer.path().join(".upskill-lock.json")).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(assert.get_output().status.code(), Some(1));
    assert_eq!(
        fs::read(consumer.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert!(
        consumer
            .path()
            .join(".claude/skills/beta/SKILL.md")
            .is_file()
    );
}

/// The root of a cross-source dependency is walked strictly: its item in an
/// unsearchable directory is not removed as absent.
#[cfg(unix)]
#[test]
fn update_stops_for_an_unsearchable_item_directory_of_a_dependency_source() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let dep = tempfile::tempdir().unwrap();
    let meta = tempfile::tempdir().unwrap();
    write_skill(dep.path(), "dep-skill");
    write_skill(dep.path(), "keep-skill");
    write(
        &dep.path().join("dep.bundle.yaml"),
        "schema: 1\nname: dep\ndescription: Leaf.\nitems:\n  skills: [dep-skill, keep-skill]\n",
    );
    write_skill(meta.path(), "meta-skill");
    write(
        &meta.path().join("meta.bundle.yaml"),
        &format!(
            "schema: 1\nname: meta\ndescription: Root.\nitems:\n  skills: [meta-skill]\nrequires:\n  - {{ name: dep, source: {} }}\n",
            dep.path().display()
        ),
    );
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            meta.path().to_str().unwrap(),
            "meta",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-skill/SKILL.md")
            .is_file()
    );
    let locked = dep.path().join("dep-skill");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::metadata(locked.join("SKILL.md")).is_ok() {
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let lock_before = fs::read(consumer.path().join(".upskill-lock.json")).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert_eq!(assert.get_output().status.code(), Some(1), "{stderr}");
    assert_eq!(
        fs::read(consumer.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-skill/SKILL.md")
            .is_file()
    );
}

/// The strict walk checks every entrypoint of an item directory: `x` holds a
/// readable `RULE.md` and a `SKILL.md` that became a symlink loop, in a
/// cross-source dependency. The skill must not be removed as absent.
#[cfg(unix)]
#[test]
fn update_stops_for_a_looping_entrypoint_beside_a_readable_one_in_a_dependency() {
    let consumer = tempfile::tempdir().unwrap();
    let dep = tempfile::tempdir().unwrap();
    let meta = tempfile::tempdir().unwrap();
    write_skill(dep.path(), "x");
    write(
        &dep.path().join("x/RULE.md"),
        "---\nname: x\ndescription: A rule.\n---\nBody.\n",
    );
    write(
        &dep.path().join("dep.bundle.yaml"),
        "schema: 1\nname: dep\ndescription: Leaf.\nitems:\n  skills: [x]\n  rules: [x]\n",
    );
    write_skill(meta.path(), "meta-skill");
    write(
        &meta.path().join("meta.bundle.yaml"),
        &format!(
            "schema: 1\nname: meta\ndescription: Root.\nitems:\n  skills: [meta-skill]\nrequires:\n  - {{ name: dep, source: {} }}\n",
            dep.path().display()
        ),
    );
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            meta.path().to_str().unwrap(),
            "meta",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert!(consumer.path().join(".claude/skills/x/SKILL.md").is_file());
    let skill = dep.path().join("x/SKILL.md");
    fs::remove_file(&skill).unwrap();
    std::os::unix::fs::symlink(&skill, &skill).unwrap();
    let lock_before = fs::read(consumer.path().join(".upskill-lock.json")).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    assert_eq!(assert.get_output().status.code(), Some(1));
    assert_eq!(
        fs::read(consumer.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
    assert!(consumer.path().join(".claude/skills/x/SKILL.md").is_file());
}

/// (c) A walk that starts at a kind directory reaches `cat/.x`: the source is
/// `skills/`, and `cat/.x` cannot be searched.
#[cfg(unix)]
#[test]
fn update_stops_when_a_hidden_item_directory_under_a_category_cannot_be_searched() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let source = registry.path().join("skills");
    write_skill(&source.join("cat"), "b");
    write_skill(&source.join("cat"), ".x");
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            source.to_str().unwrap(),
            "b",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    let locked = source.join("cat/.x");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::metadata(locked.join("SKILL.md")).is_ok() {
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let lock_before = fs::read(consumer.path().join(".upskill-lock.json")).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(assert.get_output().status.code(), Some(1));
    assert_eq!(
        fs::read(consumer.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}

/// A rule whose entry file cannot be read keeps its recorded name: the scan
/// must not fall back to the folder name and remove the rule.
#[cfg(unix)]
#[test]
fn update_fails_when_the_entry_file_of_a_named_rule_cannot_be_read() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "a");
    let file = registry.path().join("folder/RULE.md");
    write(
        &file,
        "---\nname: policy\ndescription: A named rule.\n---\nBody.\n",
    );
    for name in ["a", "policy"] {
        add_direct(consumer.path(), registry.path(), name);
    }
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&file).is_ok() {
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
        return;
    }
    assert_update_fails_unchanged(
        consumer.path(),
        ".claude/rules/policy.md",
        &[(&file, 0o644)],
    );
}

/// `update <unknown-name>` still fails with the unknown-name error.
#[test]
fn update_of_an_unknown_name_still_fails_with_a_vanished_bundle() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "nope", "--yes", "--project"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("not in lockfile: nope"));
    assert!(consumer.path().join(".claude/skills/a/SKILL.md").exists());
}

/// A bundle that was added with `--kind` and then vanishes loses its record
/// and its members.
#[test]
fn update_removes_a_kind_restricted_bundle_that_vanished() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
    assert!(consumer.path().join(".claude/rules/r.md").exists());
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(!consumer.path().join(".claude/rules/r.md").exists());
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
}

/// A bundle with an MCP server leaves its source: `update` keeps the MCP
/// record, and `doctor` reports it with the command that removes the server
/// (issue #669).
#[test]
#[cfg(unix)]
fn doctor_reports_the_kept_mcp_record_of_a_bundle_that_left_its_source() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    let bundle_path = registry.path().join("bun.bundle.yaml");
    let mut bundle = fs::read_to_string(&bundle_path).unwrap();
    bundle.push_str("mcps:\n  server:\n    local:\n      command: example-server\n");
    write(&bundle_path, &bundle);
    let bin = consumer.path().join("bin");
    let claude = bin.join("claude");
    write(&claude, "#!/bin/sh\nexit 0\n");
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let path = std::env::join_paths(paths).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .env("PATH", &path)
        .args([
            "add",
            registry.path().to_str().unwrap(),
            "bun",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert_eq!(
        read_update_lock(consumer.path())["mcps"][0]["bundle"],
        "bun"
    );
    fs::remove_file(&bundle_path).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .env("PATH", &path)
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    let lock = read_update_lock(consumer.path());
    assert!(
        lock["bundles"].as_array().is_none_or(|b| b.is_empty()),
        "{lock}"
    );
    assert_eq!(lock["mcps"][0]["name"], "server", "{lock}");

    let out = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .env("PATH", &path)
        .args(["doctor", "--json", "--project"])
        .output()
        .unwrap();
    let doctor: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let orphaned = &doctor["scopes"][0]["orphaned_mcps"];
    assert_eq!(orphaned.as_array().map(Vec::len), Some(1), "{doctor}");
    assert_eq!(orphaned[0]["name"], "server", "{doctor}");
    assert_eq!(orphaned[0]["bundle"], "bun", "{doctor}");
    assert_eq!(
        orphaned[0]["remove_command"], "upskill remove-mcp server --project",
        "{doctor}"
    );
}

fn read_update_lock(consumer: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(consumer.join(".upskill-lock.json")).unwrap()).unwrap()
}

fn add_direct(consumer: &Path, registry: &Path, name: &str) {
    common::upskill_cmd(&fake_home(consumer))
        .current_dir(consumer)
        .args([
            "add",
            registry.to_str().unwrap(),
            name,
            "--project",
            "--claude",
        ])
        .assert()
        .success();
}

fn assert_kept_skill(consumer: &Path, registry: &Path, name: &str, change_skill: bool) {
    let path = consumer.join(format!(".claude/skills/{name}/SKILL.md"));
    let body = fs::read(&path).unwrap();
    let before = read_update_lock(consumer);
    let entry = before["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == name)
        .unwrap();
    if change_skill {
        write(
            &registry.join(format!("{name}/SKILL.md")),
            &format!("---\nname: {name}\ndescription: Changed skill.\n---\nChanged body.\n"),
        );
    }
    write_rule_body(registry, "r", "Changed rule.");
    let result = common::upskill_cmd(&fake_home(consumer))
        .current_dir(consumer)
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&result.get_output().stdout);
    assert!(
        stdout
            .lines()
            .any(|line| line.contains(name) && line.contains("up to date")),
        "{stdout}"
    );
    assert_eq!(fs::read(path).unwrap(), body);
    let after = read_update_lock(consumer);
    assert_eq!(
        after["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["name"] == name)
            .unwrap(),
        entry
    );
}

#[test]
fn update_keeps_an_outside_kind_item_that_is_a_bundle_member() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
    add_direct(consumer.path(), registry.path(), "a");
    assert_kept_skill(consumer.path(), registry.path(), "a", true);
}

#[test]
fn update_keeps_an_outside_kind_required_skill_and_its_provenance() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    write_skill(registry.path(), "s");
    write(
        &registry.path().join("r2/RULE.md"),
        "---\nname: r2\ndescription: Dependent rule.\nrequires:\n  skills: [s]\n---\nRule body.\n",
    );
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
    add_direct(consumer.path(), registry.path(), "r2");
    assert_eq!(
        read_update_lock(consumer.path())["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["name"] == "s")
            .unwrap()["required_by"],
        serde_json::json!(["rule:r2"])
    );
    assert_kept_skill(consumer.path(), registry.path(), "s", false);
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["doctor", "--project"])
        .assert()
        .success();
}

#[test]
fn update_with_an_unrestricted_bundle_does_not_restrict_the_source() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    write(
        &registry.path().join("other.bundle.yaml"),
        "schema: 1\nname: other\ndescription: Unrestricted bundle.\nitems:\n  skills: [b]\n",
    );
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
    add_direct(consumer.path(), registry.path(), "other");
    write(
        &registry.path().join("b/SKILL.md"),
        "---\nname: b\ndescription: Changed skill.\n---\nChanged body.\n",
    );
    update_named_bundle(consumer.path());
    assert!(
        fs::read_to_string(consumer.path().join(".claude/skills/b/SKILL.md"))
            .unwrap()
            .contains("Changed body.")
    );
}

#[test]
fn update_uses_the_union_of_recorded_bundle_kinds() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    write(
        &registry.path().join("g/AGENT.md"),
        "---\nname: g\ndescription: An agent.\n---\nOriginal agent.\n",
    );
    write(
        &registry.path().join("other.bundle.yaml"),
        "schema: 1\nname: other\ndescription: Agent bundle.\nitems:\n  agents: [g]\n  skills: [b]\n",
    );
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            registry.path().to_str().unwrap(),
            "other",
            "--kind",
            "agent",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    write_rule_body(registry.path(), "r", "Changed rule.");
    write(
        &registry.path().join("g/AGENT.md"),
        "---\nname: g\ndescription: An agent.\n---\nChanged agent.\n",
    );
    update_named_bundle(consumer.path());
    assert!(
        fs::read_to_string(consumer.path().join(".claude/rules/r.md"))
            .unwrap()
            .contains("Changed rule.")
    );
    assert!(
        fs::read_to_string(consumer.path().join(".claude/agents/g.md"))
            .unwrap()
            .contains("Changed agent.")
    );
    assert!(!consumer.path().join(".claude/skills").exists());
}

#[test]
fn update_resolves_cross_source_bundles_beside_direct_items() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let dependency = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    write_skill(dependency.path(), "dep-skill");
    write(
        &dependency.path().join("dep.bundle.yaml"),
        "schema: 1\nname: dep\ndescription: Dependency bundle.\nitems:\n  skills: [dep-skill]\n",
    );
    let path = registry.path().join("bun.bundle.yaml");
    let body = fs::read_to_string(&path).unwrap();
    write(
        &path,
        &format!(
            "{body}requires: [{{ name: dep, source: '{}' }}]\n",
            dependency.path().display()
        ),
    );
    add_named_bundle(consumer.path(), registry.path(), &[]);
    add_direct(consumer.path(), registry.path(), "b");
    write(
        &dependency.path().join("dep-skill/SKILL.md"),
        "---\nname: dep-skill\ndescription: Changed dependency.\n---\nChanged dependency body.\n",
    );
    update_named_bundle(consumer.path());
    assert!(
        fs::read_to_string(consumer.path().join(".claude/skills/dep-skill/SKILL.md"))
            .unwrap()
            .contains("Changed dependency body.")
    );
    assert!(consumer.path().join(".claude/skills/b/SKILL.md").exists());
}

/// The bundle `bun` is renamed. Its members were installed only as bundle
/// members (not direct), so they are removed as no longer in source, and the
/// record of `bun` goes.
#[test]
fn update_removes_the_members_and_the_record_of_a_renamed_bundle() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    fs::rename(
        registry.path().join("bun.bundle.yaml"),
        registry.path().join("renamed.bundle.yaml"),
    )
    .unwrap();
    write(
        &registry.path().join("renamed.bundle.yaml"),
        "schema: 1\nname: renamed\ndescription: The renamed bundle.\nitems:\n  skills: [a]\n  rules: [r]\n",
    );
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(
        out.lines()
            .any(|l| l.split_whitespace().take(2).eq(["bundle", "bun"])
                && l.contains("removed — no longer in source")
                && l.contains("(local:")),
        "{out}"
    );
    for (kind, name) in [("skill", "a"), ("rule", "r")] {
        assert!(
            out.lines()
                .any(|l| l.split_whitespace().take(2).eq([kind, name])
                    && l.contains("removed — no longer in source")),
            "{out}"
        );
    }
    assert!(!consumer.path().join(".claude/skills/a").exists());
    assert!(!consumer.path().join(".claude/rules/r.md").exists());
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
    assert!(lock["items"].as_array().unwrap().is_empty(), "{lock}");
}

/// A member that was added by name stays when its bundle vanishes (it is
/// direct); a member that was installed only through the bundle is removed.
#[test]
fn update_keeps_a_direct_member_of_a_vanished_bundle_and_removes_the_others() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    add_direct(consumer.path(), registry.path(), "r");
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(
        out.lines()
            .any(|l| l.split_whitespace().take(2).eq(["skill", "a"])
                && l.contains("removed — no longer in source")),
        "{out}"
    );
    assert!(!consumer.path().join(".claude/skills/a").exists());
    assert!(consumer.path().join(".claude/rules/r.md").exists());
    let lock = read_update_lock(consumer.path());
    assert!(lock["bundles"].as_array().unwrap().is_empty(), "{lock}");
}

/// A vanished bundle beside a recorded bundle of the same source that stays:
/// the members of the vanished bundle are removed, and the other bundle keeps
/// its record and its members.
#[test]
fn update_keeps_a_recorded_bundle_beside_a_vanished_one() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    write(
        &registry.path().join("keep.bundle.yaml"),
        "schema: 1\nname: keep\ndescription: A bundle that stays.\nitems:\n  skills: [b]\n",
    );
    add_named_bundle(consumer.path(), registry.path(), &[]);
    add_direct(consumer.path(), registry.path(), "keep");
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    assert!(!consumer.path().join(".claude/skills/a").exists());
    assert!(!consumer.path().join(".claude/rules/r.md").exists());
    assert!(consumer.path().join(".claude/skills/b/SKILL.md").exists());
    let lock = read_update_lock(consumer.path());
    let bundles: Vec<_> = lock["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(bundles, ["keep"], "{lock}");
}

/// `--dry-run` announces the removal of the record of a vanished bundle,
/// does not call the block `up to date`, and writes nothing. Every member is
/// direct and unchanged, so the bundle line is the only change of the block.
#[test]
fn update_dry_run_reports_a_vanished_bundle_and_writes_nothing() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_direct(consumer.path(), registry.path(), "a");
    add_direct(consumer.path(), registry.path(), "r");
    add_named_bundle(consumer.path(), registry.path(), &[]);
    let tracked = [
        ".upskill-lock.json",
        ".claude/skills/a/SKILL.md",
        ".claude/rules/r.md",
        "CLAUDE.md",
    ];
    let snapshot: Vec<_> = tracked
        .iter()
        .map(|p| fs::read(consumer.path().join(p)).ok())
        .collect();
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--dry-run", "--project"])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(
        out.lines().any(
            |l| l.split_whitespace().take(3).eq(["remove", "bundle", "bun"])
                && l.contains("(local:")
        ),
        "{out}"
    );
    assert!(!out.contains("up to date"), "{out}");
    let after: Vec<_> = tracked
        .iter()
        .map(|p| fs::read(consumer.path().join(p)).ok())
        .collect();
    assert_eq!(after, snapshot);
}

/// `update --yes` whose only change is the dropped record of a vanished
/// bundle (every member is direct and unchanged) reports the dropped record
/// and does not say that there was nothing to do.
#[test]
fn update_yes_reports_a_dropped_bundle_record_as_the_only_change() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_direct(consumer.path(), registry.path(), "a");
    add_direct(consumer.path(), registry.path(), "r");
    add_named_bundle(consumer.path(), registry.path(), &[]);
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();

    let out = update_output(consumer.path());

    assert!(
        out.lines().any(|l| {
            l.split_whitespace()
                .take(3)
                .eq(["bundle", "bun", "removed"])
                && l.contains("no longer in source")
        }),
        "{out}"
    );
    assert!(!out.contains("nothing to do"), "{out}");
    assert!(!removed_line(&out, "skill"), "{out}");
    assert!(consumer.path().join(".claude/skills/a/SKILL.md").exists());
    assert!(consumer.path().join(".claude/rules/r.md").exists());
    assert!(recorded_bundles(consumer.path()).is_empty());
}

#[test]
fn update_merges_direct_item_requires_into_the_bundle_closure() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    write_rule_body(registry.path(), "r2", "Direct rule.");
    add_named_bundle(consumer.path(), registry.path(), &[]);
    add_direct(consumer.path(), registry.path(), "r2");
    write(
        &registry.path().join("r2/RULE.md"),
        "---\nname: r2\ndescription: Dependent rule.\nrequires:\n  skills: [a, b]\n---\nChanged rule.\n",
    );
    update_named_bundle(consumer.path());
    let lock = read_update_lock(consumer.path());
    for name in ["a", "b"] {
        let items: Vec<_> = lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["name"] == name)
            .collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["required_by"], serde_json::json!(["rule:r2"]));
        assert!(
            consumer
                .path()
                .join(format!(".claude/skills/{name}/SKILL.md"))
                .is_file()
        );
    }
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["doctor", "--project"])
        .assert()
        .success();
}

#[test]
fn update_under_a_kind_restriction_keeps_the_provenance_of_a_required_item_of_another_source() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let dependency = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    write_skill(dependency.path(), "s");
    write(
        &registry.path().join("r2/RULE.md"),
        &format!(
            "---\nname: r2\ndescription: Dependent rule.\nrequires:\n  skills: [{{name: s, source: {}}}]\n---\nRule body.\n",
            dependency.path().display()
        ),
    );
    add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
    add_direct(consumer.path(), registry.path(), "r2");
    let before = read_update_lock(consumer.path());
    let required_by = &before["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "s" && item["kind"] == "skill")
        .unwrap()["required_by"];
    assert_eq!(*required_by, serde_json::json!(["rule:r2"]));
    update_named_bundle(consumer.path());
    let after = read_update_lock(consumer.path());
    assert_eq!(
        after["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == "s" && item["kind"] == "skill")
            .unwrap()["required_by"],
        *required_by
    );
}

#[test]
fn kind_restricted_update_refreshes_provenance_by_kind_and_name() {
    for change_rule in [false, true] {
        let consumer = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        write_rule_body(registry.path(), "shared", "Shared rule.");
        write_rule_body(registry.path(), "base", "Base rule.");
        write(
            &registry.path().join("shared/SKILL.md"),
            "---\nname: shared\ndescription: Dependent skill.\nrequires:\n  rules: [base]\n---\nSkill body.\n",
        );
        write(
            &registry.path().join("r2/RULE.md"),
            "---\nname: r2\ndescription: Dependent rule.\nrequires:\n  rules: [shared]\n  skills: [shared]\n---\nRule body.\n",
        );
        write(
            &registry.path().join("bun.bundle.yaml"),
            "schema: 1\nname: bun\ndescription: Mixed bundle.\nitems:\n  rules: [base, r2, shared]\n  skills: [shared]\n",
        );
        add_named_bundle(consumer.path(), registry.path(), &["--kind", "rule"]);
        add_direct(consumer.path(), registry.path(), "r2");
        let before = read_update_lock(consumer.path());
        for kind in ["rule", "skill"] {
            let item = before["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["kind"] == kind && item["name"] == "shared")
                .unwrap();
            assert_eq!(item["required_by"], serde_json::json!(["rule:r2"]));
        }
        if change_rule {
            write_rule_body(registry.path(), "shared", "Changed shared rule.");
        }
        update_named_bundle(consumer.path());
        let after = read_update_lock(consumer.path());
        let item = |kind: &str| {
            after["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["kind"] == kind && item["name"] == "shared")
                .unwrap()
        };
        // The bundle closure evaluates r2 but contributes no item-level edges.
        assert!(item("rule").get("required_by").is_none(), "{after}");
        assert_eq!(
            item("skill")["required_by"],
            serde_json::json!(["rule:r2"]),
            "an unplanned skill must keep its provenance even when a rule has the same name"
        );
        let base = after["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["kind"] == "rule" && item["name"] == "base")
            .unwrap();
        assert_eq!(base["required_by"], serde_json::json!(["skill:shared"]));
    }
}

#[test]
fn update_evaluates_bundle_members_even_without_direct_items() {
    for change_rule in [false, true] {
        let consumer = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        setup_kind_registry(registry.path());
        write(
            &registry.path().join("r/RULE.md"),
            "---\nname: r\ndescription: Dependent rule.\nrequires:\n  skills: [a]\n---\nRule body.\n",
        );
        add_direct(consumer.path(), registry.path(), "r");
        add_named_bundle(consumer.path(), registry.path(), &[]);
        let before = read_update_lock(consumer.path());
        let dependency = before["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == "a")
            .unwrap();
        assert_eq!(dependency["required_by"], serde_json::json!(["rule:r"]));
        if change_rule {
            write_rule_body(registry.path(), "r", "Changed bundle rule.");
        }
        update_named_bundle(consumer.path());
        let after = read_update_lock(consumer.path());
        let dependency = after["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == "a")
            .unwrap();
        assert!(dependency.get("required_by").is_none(), "{after}");
        assert_eq!(after["items"].as_array().unwrap().len(), 2);
    }
}

#[test]
fn update_does_not_keep_removed_bundle_members_as_direct_items() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    write_skill(registry.path(), "direct");
    write_skill(registry.path(), "stray");
    add_named_bundle(consumer.path(), registry.path(), &[]);
    add_direct(consumer.path(), registry.path(), "direct");
    // a remains in the registry, but b replaces it in the bundle.
    write(
        &registry.path().join("bun.bundle.yaml"),
        "schema: 1\nname: bun\ndescription: Changed bundle.\nitems:\n  rules: [r]\n  skills: [b]\n",
    );
    update_named_bundle(consumer.path());
    assert!(!consumer.path().join(".claude/skills/a").exists());
    assert!(consumer.path().join(".claude/skills/b/SKILL.md").is_file());
    assert!(
        consumer
            .path()
            .join(".claude/skills/direct/SKILL.md")
            .is_file()
    );
    assert!(!consumer.path().join(".claude/skills/stray").exists());
    let lock = read_update_lock(consumer.path());
    let mut names: Vec<_> = lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["b", "direct", "r"]);
}

#[test]
fn add_under_a_kind_restriction_keeps_the_provenance_of_an_unplanned_item() {
    for remove_requirement in [false, true] {
        let consumer = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        write_skill(registry.path(), "s");
        write(
            &registry.path().join("r2/RULE.md"),
            "---\nname: r2\ndescription: Dependent rule.\nrequires:\n  skills: [s]\n---\nRule body.\n",
        );
        add_direct(consumer.path(), registry.path(), "r2");
        assert_eq!(
            read_update_lock(consumer.path())["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["name"] == "s")
                .unwrap()["required_by"],
            serde_json::json!(["rule:r2"])
        );
        let skill_path = consumer.path().join(".claude/skills/s/SKILL.md");
        let skill_before = fs::read(&skill_path).unwrap();
        let entry_before = read_update_lock(consumer.path())["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == "s")
            .unwrap()
            .clone();
        // A changed source makes a rewrite of `s` visible.
        write(
            &registry.path().join("s/SKILL.md"),
            "---\nschema: 1\nname: s\ndescription: changed skill s.\n---\n\nChanged.\n",
        );
        if remove_requirement {
            write_rule_body(registry.path(), "r2", "No dependency remains.");
        }
        common::upskill_cmd(&fake_home(consumer.path()))
            .current_dir(consumer.path())
            .args([
                "add",
                registry.path().to_str().unwrap(),
                "r2",
                "--kind",
                "rule",
                "--project",
                "--claude",
            ])
            .assert()
            .success();
        let after = read_update_lock(consumer.path());
        let item = after["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == "s")
            .unwrap();
        assert_eq!(item["required_by"], serde_json::json!(["rule:r2"]));
        assert_eq!(fs::read(&skill_path).unwrap(), skill_before);
        assert_eq!(item["hash"], entry_before["hash"]);
        assert_eq!(item["files"], entry_before["files"]);
    }
}

fn stage_kind_restricted_source_skipped_later() -> (tempfile::TempDir, tempfile::TempDir, PathBuf) {
    let consumer = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let registry = sources.path().join("a-registry");
    let dependency = sources.path().join("b-dependency");
    let missing = sources.path().join("c-missing");
    write_rule_body(&registry, "base", "Base rule.");
    write_skill(&registry, "kept");
    write_rule_body(&dependency, "delta", "Dependency rule.");
    write_skill(&missing, "gamma");
    write(
        &registry.join("r/RULE.md"),
        &format!(
            "---\nname: r\ndescription: Dependent rule.\nrequires:\n  rules: [{{name: delta, source: {}}}]\n---\nRule body.\n",
            dependency.display()
        ),
    );
    write(
        &dependency.join("beta/SKILL.md"),
        &format!(
            "---\nname: beta\ndescription: Dependent skill.\nrequires:\n  skills: [{{name: gamma, source: {}}}]\n---\nSkill body.\n",
            missing.display()
        ),
    );
    write(
        &registry.join("bun.bundle.yaml"),
        "schema: 1\nname: bun\ndescription: Rule bundle.\nitems:\n  rules: [base]\n",
    );
    add_named_bundle(consumer.path(), &registry, &["--kind", "rule"]);
    add_direct(consumer.path(), &registry, "r");
    add_direct(consumer.path(), &registry, "kept");
    add_direct(consumer.path(), &dependency, "beta");
    fs::remove_dir_all(&missing).unwrap();
    (consumer, sources, registry)
}

#[test]
fn kind_restricted_update_reports_an_unplanned_item_of_a_later_skipped_source_as_skipped() {
    let (consumer, _sources, registry) = stage_kind_restricted_source_skipped_later();
    let lock_path = consumer.path().join(".upskill-lock.json");
    let before = fs::read(&lock_path).unwrap();
    let kept_path = consumer.path().join(".claude/skills/kept/SKILL.md");
    let kept_before = fs::read(&kept_path).unwrap();
    let mut cmd = common::upskill_cmd(&fake_home(consumer.path()));
    cmd.current_dir(consumer.path())
        .args(["update", "--project", "--yes"]);
    let result = cmd.assert().code(1);
    let out = String::from_utf8_lossy(&result.get_output().stdout);
    let err = String::from_utf8_lossy(&result.get_output().stderr);
    assert!(
        err.contains(&format!(
            "source `local:{}` was skipped",
            registry.display()
        )),
        "{err}"
    );
    let kept_lines: Vec<_> = out
        .lines()
        .filter(|line| line.split_whitespace().take(2).eq(["skill", "kept"]))
        .collect();
    assert_eq!(kept_lines.len(), 1, "{out}");
    assert!(
        kept_lines[0].contains("skipped — requires a source that was skipped"),
        "{out}"
    );
    assert_eq!(fs::read(&lock_path).unwrap(), before);
    assert_eq!(fs::read(&kept_path).unwrap(), kept_before);
}

#[test]
fn a_later_skipped_source_does_not_mark_an_applied_sources_kept_item_as_skipped() {
    let (consumer, sources, _registry) = stage_kind_restricted_source_skipped_later();
    let applied = sources.path().join("z-applied");
    write_skill(&applied, "untouched");
    write_rule_body(&applied, "current", "Original applied rule.");
    write(
        &applied.join("other.bundle.yaml"),
        "schema: 1\nname: other\ndescription: Another rule bundle.\nitems:\n  rules: [current]\n",
    );
    add_direct(consumer.path(), &applied, "untouched");
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            applied.to_str().unwrap(),
            "other",
            "--kind",
            "rule",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    let before = read_update_lock(consumer.path());
    let kept = before["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "untouched")
        .unwrap();
    let kept_path = consumer.path().join(".claude/skills/untouched/SKILL.md");
    let bytes_before = fs::read(&kept_path).unwrap();
    write_rule_body(&applied, "current", "Changed applied rule.");
    write_skill(&applied, "unlisted");

    let result = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--project", "--yes"])
        .assert()
        .code(1);
    let out = String::from_utf8_lossy(&result.get_output().stdout);
    for (name, status) in [
        ("kept", "skipped — requires a source that was skipped"),
        ("untouched", "up to date"),
    ] {
        let lines: Vec<_> = out
            .lines()
            .filter(|line| line.split_whitespace().take(2).eq(["skill", name]))
            .collect();
        assert_eq!(lines.len(), 1, "{out}");
        assert!(lines[0].contains(status), "{out}");
    }
    let after = read_update_lock(consumer.path());
    assert_eq!(
        after["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == "untouched")
            .unwrap(),
        kept
    );
    assert_eq!(fs::read(&kept_path).unwrap(), bytes_before);
    assert!(
        fs::read_to_string(consumer.path().join(".claude/rules/current.md"))
            .unwrap()
            .contains("Changed applied rule.")
    );
    assert!(!consumer.path().join(".claude/skills/unlisted").exists());
}

#[test]
fn kind_restricted_update_reports_out_of_kind_items_of_an_unreachable_source_with_its_cause() {
    let consumer = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let registry = sources.path().join("registry");
    setup_kind_registry(&registry);
    add_named_bundle(consumer.path(), &registry, &["--kind", "rule"]);
    add_direct(consumer.path(), &registry, "b");
    let lock_path = consumer.path().join(".upskill-lock.json");
    let lock_before = fs::read(&lock_path).unwrap();
    let skill_path = consumer.path().join(".claude/skills/b/SKILL.md");
    let skill_before = fs::read(&skill_path).unwrap();
    fs::remove_dir_all(&registry).unwrap();

    let result = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--project", "--yes"])
        .assert()
        .code(1);
    let out = String::from_utf8_lossy(&result.get_output().stdout);
    let err = String::from_utf8_lossy(&result.get_output().stderr);
    assert!(
        err.contains(&format!(
            "source `local:{}` could not be fetched and was skipped:",
            registry.display()
        )),
        "{err}"
    );
    for (kind, name) in [("rule", "r"), ("skill", "b")] {
        let lines: Vec<_> = out
            .lines()
            .filter(|line| line.split_whitespace().take(2).eq([kind, name]))
            .collect();
        assert_eq!(lines.len(), 1, "{out}");
        assert!(lines[0].contains("skipped — source not reachable"), "{out}");
    }
    assert_eq!(fs::read(&lock_path).unwrap(), lock_before);
    assert_eq!(fs::read(&skill_path).unwrap(), skill_before);
}

// Issue #648: a bundle that stops requiring another bundle.

/// Writes the root bundle `top` in `registry`: it lists the skill `own` and
/// requires each entry of `requires` (a YAML flow sequence item each).
fn write_top_bundle(registry: &Path, requires: &[String]) {
    let requires = if requires.is_empty() {
        String::new()
    } else {
        format!("requires: [{}]\n", requires.join(", "))
    };
    write(
        &registry.join("top.bundle.yaml"),
        &format!(
            "schema: 1\nname: top\ndescription: Root bundle.\nitems:\n  skills: [own]\n{requires}"
        ),
    );
}

fn write_skill_bundle(registry: &Path, name: &str, skills: &[&str]) {
    write(
        &registry.join(format!("{name}.bundle.yaml")),
        &format!(
            "schema: 1\nname: {name}\ndescription: Dependency bundle.\nitems:\n  skills: [{}]\n",
            skills.join(", ")
        ),
    );
}

fn add_top(consumer: &Path, registry: &Path) {
    common::upskill_cmd(&fake_home(consumer))
        .current_dir(consumer)
        .args([
            "add",
            registry.to_str().unwrap(),
            "top",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
}

fn update_output(consumer: &Path) -> String {
    let assert = common::upskill_cmd(&fake_home(consumer))
        .current_dir(consumer)
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

fn recorded_bundles(consumer: &Path) -> Vec<String> {
    read_update_lock(consumer)["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap().to_string())
        .collect()
}

fn recorded_items(consumer: &Path) -> Vec<String> {
    read_update_lock(consumer)["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap().to_string())
        .collect()
}

fn removed_line(out: &str, name: &str) -> bool {
    out.lines()
        .any(|l| l.contains(name) && l.contains("no longer in source"))
}

#[test]
fn add_records_the_requirer_of_a_dependency_bundle() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(registry.path(), "dep-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    write_top_bundle(registry.path(), &["{ name: dep }".to_string()]);
    add_top(consumer.path(), registry.path());
    let lock = read_update_lock(consumer.path());
    let bundles = lock["bundles"].as_array().unwrap();
    let by_name = |n: &str| bundles.iter().find(|b| b["name"] == n).unwrap().clone();
    assert_eq!(
        by_name("dep")["required_by"],
        serde_json::json!(["bundle:top"])
    );
    assert!(by_name("top").get("required_by").is_none());
}

#[test]
fn update_removes_the_items_of_a_bundle_that_is_no_longer_required() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(registry.path(), "dep-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    write_top_bundle(registry.path(), &["{ name: dep }".to_string()]);
    add_top(consumer.path(), registry.path());
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-only/SKILL.md")
            .exists()
    );

    write_top_bundle(registry.path(), &[]);
    let out = update_output(consumer.path());

    assert!(removed_line(&out, "dep-only"), "{out}");
    assert!(!consumer.path().join(".claude/skills/dep-only").exists());
    assert!(consumer.path().join(".claude/skills/own/SKILL.md").exists());
    assert_eq!(recorded_bundles(consumer.path()), vec!["top"]);
    assert_eq!(recorded_items(consumer.path()), vec!["own"]);
}

/// The plugin records of a bundle that is no longer required are removed, as
/// `remove` removes the plugin records of a removed bundle (issue #670).
#[test]
#[cfg(unix)]
fn update_drops_the_plugin_record_of_a_bundle_that_is_no_longer_required() {
    use std::os::unix::fs::PermissionsExt;
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(registry.path(), "dep-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    let dep_path = registry.path().join("dep.bundle.yaml");
    let mut dep = fs::read_to_string(&dep_path).unwrap();
    dep.push_str("plugins:\n  tool:\n    claude:\n      source: example/tools\n      marketplace: tools\n      plugin: tool\n");
    write(&dep_path, &dep);
    write_top_bundle(registry.path(), &["{ name: dep }".to_string()]);
    let bin = consumer.path().join("bin");
    let claude = bin.join("claude");
    write(&claude, "#!/bin/sh\nexit 0\n");
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let path = std::env::join_paths(paths).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .env("PATH", &path)
        .args([
            "add",
            registry.path().to_str().unwrap(),
            "top",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    let lock = read_update_lock(consumer.path());
    assert_eq!(lock["plugins"][0]["bundle"], "dep", "{lock}");

    write_top_bundle(registry.path(), &[]);
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .env("PATH", &path)
        .args(["update", "--yes", "--project"])
        .assert()
        .success();
    let lock = read_update_lock(consumer.path());
    assert_eq!(recorded_bundles(consumer.path()), vec!["top"]);
    assert!(
        lock["plugins"].as_array().is_none_or(|p| p.is_empty()),
        "{lock}"
    );
}

#[test]
fn update_keeps_an_item_that_a_bundle_still_required_provides() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(registry.path(), "dep-only");
    write_skill(registry.path(), "shifted");
    write_skill(registry.path(), "kept-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only", "shifted"]);
    write_skill_bundle(registry.path(), "kept", &["kept-only"]);
    write_top_bundle(
        registry.path(),
        &["{ name: dep }".to_string(), "{ name: kept }".to_string()],
    );
    add_top(consumer.path(), registry.path());

    // `dep` is dropped; its item `shifted` now comes from the bundle `kept`.
    write_skill_bundle(registry.path(), "kept", &["kept-only", "shifted"]);
    write_top_bundle(registry.path(), &["{ name: kept }".to_string()]);
    let out = update_output(consumer.path());

    assert!(removed_line(&out, "dep-only"), "{out}");
    assert!(!removed_line(&out, "shifted"), "{out}");
    assert!(!removed_line(&out, "kept-only"), "{out}");
    for name in ["own", "shifted", "kept-only"] {
        assert!(
            consumer
                .path()
                .join(format!(".claude/skills/{name}/SKILL.md"))
                .exists(),
            "{name}"
        );
    }
    assert!(!consumer.path().join(".claude/skills/dep-only").exists());
    assert_eq!(recorded_bundles(consumer.path()), vec!["kept", "top"]);
}

#[test]
fn update_removes_the_items_of_a_cross_source_bundle_that_is_no_longer_required() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let dependency = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(dependency.path(), "dep-only");
    write_skill_bundle(dependency.path(), "dep", &["dep-only"]);
    let requires = format!("{{ name: dep, source: '{}' }}", dependency.path().display());
    write_top_bundle(registry.path(), &[requires]);
    add_top(consumer.path(), registry.path());
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-only/SKILL.md")
            .exists()
    );

    write_top_bundle(registry.path(), &[]);
    let out = update_output(consumer.path());

    assert!(removed_line(&out, "dep-only"), "{out}");
    assert_eq!(
        out.lines().filter(|l| l.contains("dep-only")).count(),
        1,
        "{out}"
    );
    assert!(!consumer.path().join(".claude/skills/dep-only").exists());
    assert!(consumer.path().join(".claude/skills/own/SKILL.md").exists());
    assert_eq!(recorded_bundles(consumer.path()), vec!["top"]);
    assert_eq!(recorded_items(consumer.path()), vec!["own"]);
}

#[test]
fn update_refreshes_a_cross_source_dependency_once() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let dependency = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(dependency.path(), "dep-only");
    write_skill_bundle(dependency.path(), "dep", &["dep-only"]);
    let requires = format!("{{ name: dep, source: '{}' }}", dependency.path().display());
    write_top_bundle(registry.path(), &[requires]);
    add_top(consumer.path(), registry.path());
    write(
        &dependency.path().join("dep-only/SKILL.md"),
        "---\nname: dep-only\ndescription: Changed.\n---\nChanged body.\n",
    );
    let out = update_output(consumer.path());
    assert_eq!(
        out.lines().filter(|l| l.contains("dep-only")).count(),
        1,
        "{out}"
    );
    assert!(
        fs::read_to_string(consumer.path().join(".claude/skills/dep-only/SKILL.md"))
            .unwrap()
            .contains("Changed body.")
    );
    assert_eq!(recorded_bundles(consumer.path()), vec!["dep", "top"]);
}

/// Writes a root bundle `name` that lists the skill `own_skill` and requires
/// each entry of `requires`.
fn write_root_bundle(registry: &Path, name: &str, own_skill: &str, requires: &[String]) {
    let requires = if requires.is_empty() {
        String::new()
    } else {
        format!("requires: [{}]\n", requires.join(", "))
    };
    write(
        &registry.join(format!("{name}.bundle.yaml")),
        &format!(
            "schema: 1\nname: {name}\ndescription: Root bundle.\nitems:\n  skills: [{own_skill}]\n{requires}"
        ),
    );
}

fn add_bundle_named(consumer: &Path, registry: &Path, name: &str) {
    common::upskill_cmd(&fake_home(consumer))
        .current_dir(consumer)
        .args([
            "add",
            registry.to_str().unwrap(),
            name,
            "--project",
            "--claude",
        ])
        .assert()
        .success();
}

/// Two roots require `dep`; `first` drops it and `second` keeps it. The
/// roots are in `first_registry` and `second_registry` (the same directory
/// for the same-source case); `dep` is in `dep_registry`.
fn assert_dependency_of_a_second_root_is_kept(
    first_registry: &Path,
    second_registry: &Path,
    dep_registry: &Path,
) {
    let consumer = tempfile::tempdir().unwrap();
    write_skill(first_registry, "own1");
    write_skill(second_registry, "own2");
    write_skill(dep_registry, "dep-only");
    write_skill_bundle(dep_registry, "dep", &["dep-only"]);
    let dep_ref = |registry: &Path| {
        if registry == dep_registry {
            "{ name: dep }".to_string()
        } else {
            format!("{{ name: dep, source: '{}' }}", dep_registry.display())
        }
    };
    write_root_bundle(first_registry, "first", "own1", &[dep_ref(first_registry)]);
    write_root_bundle(
        second_registry,
        "second",
        "own2",
        &[dep_ref(second_registry)],
    );
    add_bundle_named(consumer.path(), first_registry, "first");
    add_bundle_named(consumer.path(), second_registry, "second");

    write_root_bundle(first_registry, "first", "own1", &[]);
    let out = update_output(consumer.path());

    assert!(!removed_line(&out, "dep-only"), "{out}");
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-only/SKILL.md")
            .exists(),
        "{out}"
    );
    assert!(recorded_bundles(consumer.path()).contains(&"dep".to_string()));
    assert!(recorded_items(consumer.path()).contains(&"dep-only".to_string()));
    // The requirer that dropped the dependency leaves its `required_by`.
    assert_eq!(
        bundle_required_by(consumer.path(), "dep"),
        serde_json::json!(["bundle:second"])
    );
}

/// The `required_by` value of the bundle record `name` (`null` when absent).
fn bundle_required_by(consumer: &Path, name: &str) -> serde_json::Value {
    read_update_lock(consumer)["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == name)
        .unwrap_or_else(|| panic!("bundle {name} not recorded"))
        .get("required_by")
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

/// A registry with the skills `own1`, `own2` and `dep-only`, the bundle
/// `dep` (lists `dep-only`) and the roots `first` and `second`, which both
/// require `dep`.
fn setup_two_roots_of_one_dependency(registry: &Path) {
    write_skill(registry, "own1");
    write_skill(registry, "own2");
    write_skill(registry, "dep-only");
    write_skill_bundle(registry, "dep", &["dep-only"]);
    write_root_bundle(registry, "first", "own1", &["{ name: dep }".to_string()]);
    write_root_bundle(registry, "second", "own2", &["{ name: dep }".to_string()]);
}

#[test]
fn a_second_add_keeps_the_requirer_of_the_first_add() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_two_roots_of_one_dependency(registry.path());
    add_bundle_named(consumer.path(), registry.path(), "first");
    add_bundle_named(consumer.path(), registry.path(), "second");
    assert_eq!(
        bundle_required_by(consumer.path(), "dep"),
        serde_json::json!(["bundle:first", "bundle:second"])
    );
}

#[test]
fn a_bundle_installed_directly_stays_a_root_when_a_later_add_requires_it() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_two_roots_of_one_dependency(registry.path());
    add_bundle_named(consumer.path(), registry.path(), "dep");
    add_bundle_named(consumer.path(), registry.path(), "first");
    assert_eq!(
        bundle_required_by(consumer.path(), "dep"),
        serde_json::Value::Null
    );
}

#[test]
fn a_bundle_requested_beside_its_requirer_is_a_root() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_two_roots_of_one_dependency(registry.path());
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            registry.path().to_str().unwrap(),
            "first",
            "dep",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert_eq!(
        bundle_required_by(consumer.path(), "dep"),
        serde_json::Value::Null
    );
}

#[test]
fn update_keeps_a_dependency_that_a_root_of_another_source_still_requires() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let dep = tempfile::tempdir().unwrap();
    assert_dependency_of_a_second_root_is_kept(first.path(), second.path(), dep.path());
}

#[test]
fn update_keeps_a_dependency_that_a_root_of_the_dependency_source_still_requires() {
    // `second` and `dep` share a source, `first` is in another one.
    let first = tempfile::tempdir().unwrap();
    let shared = tempfile::tempdir().unwrap();
    assert_dependency_of_a_second_root_is_kept(first.path(), shared.path(), shared.path());
}

#[test]
fn update_keeps_a_dependency_that_a_root_of_the_same_source_still_requires() {
    let registry = tempfile::tempdir().unwrap();
    assert_dependency_of_a_second_root_is_kept(registry.path(), registry.path(), registry.path());
}

#[test]
fn named_update_keeps_the_dependency_bundle_and_its_items() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(registry.path(), "dep-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    write_top_bundle(registry.path(), &["{ name: dep }".to_string()]);
    add_top(consumer.path(), registry.path());

    write_top_bundle(registry.path(), &[]);
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project", "own"])
        .assert()
        .success();

    // A named update does not read the whole resolution: the dependency and
    // its record stay until a full `update`.
    assert_eq!(recorded_bundles(consumer.path()), vec!["dep", "top"]);
    let lock = read_update_lock(consumer.path());
    let dep = lock["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "dep")
        .unwrap()
        .clone();
    assert_eq!(dep["required_by"], serde_json::json!(["bundle:top"]));
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-only/SKILL.md")
            .exists()
    );

    let out = update_output(consumer.path());
    assert!(removed_line(&out, "dep-only"), "{out}");
    assert_eq!(recorded_bundles(consumer.path()), vec!["top"]);
}

#[test]
fn update_keeps_an_aliased_member_of_a_dependency_bundle() {
    // `add` applies `--as` to a member of a same-source dependency bundle
    // only; a member of a cross-source dependency keeps its name.
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(registry.path(), "dep-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    write_top_bundle(registry.path(), &["{ name: dep }".to_string()]);
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args([
            "add",
            registry.path().to_str().unwrap(),
            "top",
            "--project",
            "--claude",
            "--as",
            "dep-only=renamed",
        ])
        .assert()
        .success();
    assert!(recorded_items(consumer.path()).contains(&"renamed".to_string()));

    let out = update_output(consumer.path());
    assert!(!removed_line(&out, "renamed"), "{out}");
    assert!(
        consumer
            .path()
            .join(".claude/skills/renamed/SKILL.md")
            .exists()
    );
    assert!(!consumer.path().join(".claude/skills/dep-only").exists());
}

#[test]
fn update_reports_a_cross_source_dependency_of_an_unreachable_root_as_skipped() {
    let consumer = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let registry = root.path().join("registry");
    let dependency = tempfile::tempdir().unwrap();
    write_skill(&registry, "own");
    write_skill(dependency.path(), "dep-only");
    write_skill_bundle(dependency.path(), "dep", &["dep-only"]);
    let requires = format!("{{ name: dep, source: '{}' }}", dependency.path().display());
    write_top_bundle(&registry, &[requires]);
    add_top(consumer.path(), &registry);
    fs::remove_dir_all(&registry).unwrap();

    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let line = out
        .lines()
        .find(|l| l.contains("dep-only"))
        .unwrap_or_else(|| panic!("{out}"));
    assert!(line.contains("source not reachable"), "{out}");
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-only/SKILL.md")
            .exists()
    );
    assert_eq!(recorded_bundles(consumer.path()), vec!["dep", "top"]);
}

#[test]
fn bare_update_keeps_the_clients_of_a_dependency_item() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(registry.path(), "dep-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    write_top_bundle(registry.path(), &["{ name: dep }".to_string()]);
    let add = |args: &[&str]| {
        common::upskill_cmd(&fake_home(consumer.path()))
            .current_dir(consumer.path())
            .arg("add")
            .arg(registry.path())
            .args(args)
            .arg("--project")
            .assert()
            .success();
    };
    // The bundle for Copilot, then its own item again for Claude only.
    add(&["top", "--copilot"]);
    add(&["own", "--claude"]);
    let clients = |name: &str| {
        read_update_lock(consumer.path())["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["name"] == name)
            .unwrap()["clients"]
            .clone()
    };
    let dep_clients = clients("dep-only");
    assert_eq!(dep_clients, serde_json::json!(["copilot"]));
    assert_ne!(clients("own"), dep_clients);
    let copilot_output = consumer.path().join(".github/skills/dep-only/SKILL.md");
    assert!(copilot_output.exists());

    update_output(consumer.path());

    // Each item keeps exactly its recorded clients (#655).
    assert_eq!(clients("dep-only"), serde_json::json!(["copilot"]));
    assert!(copilot_output.exists());
    assert!(!consumer.path().join(".claude/skills/dep-only").exists());
}

/// `dropped` (required by `first`) and `kept` (required by `second`, whose
/// source is unreachable at update time) both list `shared`. `first` drops
/// `dropped`: `shared` belongs to the skipped installation too.
fn assert_item_of_two_dependency_bundles_is_kept(dropped: &str, kept: &str) {
    let consumer = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let second = parent.path().join("second");
    let dep = tempfile::tempdir().unwrap();
    write_skill(first.path(), "own1");
    write_skill(&second, "own2");
    write_skill(dep.path(), "shared");
    write_skill_bundle(dep.path(), dropped, &["shared"]);
    write_skill_bundle(dep.path(), kept, &["shared"]);
    let requires = |name: &str| format!("{{ name: {name}, source: '{}' }}", dep.path().display());
    write_root_bundle(first.path(), "first", "own1", &[requires(dropped)]);
    write_root_bundle(&second, "second", "own2", &[requires(kept)]);
    add_bundle_named(consumer.path(), first.path(), "first");
    add_bundle_named(consumer.path(), &second, "second");

    write_root_bundle(first.path(), "first", "own1", &[]);
    fs::remove_dir_all(&second).unwrap();
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(!removed_line(&out, "shared"), "{out}");
    assert!(
        consumer
            .path()
            .join(".claude/skills/shared/SKILL.md")
            .exists(),
        "{out}"
    );
    assert!(recorded_items(consumer.path()).contains(&"shared".to_string()));
    // The root source of `kept` was not applied: its record stays.
    let bundles = recorded_bundles(consumer.path());
    assert!(bundles.contains(&kept.to_string()), "{bundles:?}");
    assert!(!bundles.contains(&dropped.to_string()), "{bundles:?}");
    assert_eq!(
        bundle_required_by(consumer.path(), kept),
        serde_json::json!(["bundle:second"])
    );
}

#[test]
fn update_keeps_an_item_of_two_dependency_bundles_while_a_root_source_is_skipped() {
    assert_item_of_two_dependency_bundles_is_kept("dep1", "dep2");
}

#[test]
fn update_keeps_an_item_of_two_dependency_bundles_when_the_dropped_one_sorts_last() {
    assert_item_of_two_dependency_bundles_is_kept("dep2", "dep1");
}

#[test]
fn update_refreshes_a_dependency_that_a_prepared_root_provides_beside_a_skipped_root() {
    let consumer = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let second = parent.path().join("second");
    let dep = tempfile::tempdir().unwrap();
    write_skill(first.path(), "own1");
    write_skill(&second, "own2");
    write_skill(dep.path(), "dep-only");
    write_skill_bundle(dep.path(), "dep", &["dep-only"]);
    let requires = format!("{{ name: dep, source: '{}' }}", dep.path().display());
    write_root_bundle(
        first.path(),
        "first",
        "own1",
        std::slice::from_ref(&requires),
    );
    write_root_bundle(&second, "second", "own2", &[requires]);
    add_bundle_named(consumer.path(), first.path(), "first");
    add_bundle_named(consumer.path(), &second, "second");

    fs::remove_dir_all(&second).unwrap();
    write(
        &dep.path().join("dep-only/SKILL.md"),
        "---\nname: dep-only\ndescription: Changed.\n---\nChanged body.\n",
    );
    let assert = common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    let line = out
        .lines()
        .find(|l| l.contains("dep-only"))
        .unwrap_or_else(|| panic!("{out}"));
    assert!(!line.contains("skipped"), "{out}");
    assert!(
        fs::read_to_string(consumer.path().join(".claude/skills/dep-only/SKILL.md"))
            .unwrap()
            .contains("Changed body."),
        "{out}"
    );
}

fn write_rule(registry: &Path, name: &str) {
    write_rule_body(registry, name, &format!("Rule {name}."));
}

fn add_with(consumer: &Path, registry: &Path, args: &[&str]) {
    common::upskill_cmd(&fake_home(consumer))
        .current_dir(consumer)
        .arg("add")
        .arg(registry)
        .args(args)
        .args(["--project", "--claude"])
        .assert()
        .success();
}

#[test]
fn update_keeps_an_out_of_kind_item_of_a_dropped_dependency() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_rule(registry.path(), "r");
    write_rule(registry.path(), "dep-rule");
    write_skill(registry.path(), "dep-skill");
    write(
        &registry.path().join("dep.bundle.yaml"),
        "schema: 1\nname: dep\ndescription: Dependency bundle.\nitems:\n  rules: [dep-rule]\n  skills: [dep-skill]\n",
    );
    let top = |requires: &str| {
        write(
            &registry.path().join("top.bundle.yaml"),
            &format!(
                "schema: 1\nname: top\ndescription: Root bundle.\nitems:\n  rules: [r]\n{requires}"
            ),
        )
    };
    top("requires: [{ name: dep }]\n");
    add_with(consumer.path(), registry.path(), &["top"]);
    add_with(consumer.path(), registry.path(), &["top", "--kind", "rule"]);
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-skill/SKILL.md")
            .exists()
    );

    top("");
    let out = update_output(consumer.path());

    // The skill is outside the recorded kind restriction: it is kept.
    assert!(removed_line(&out, "dep-rule"), "{out}");
    assert!(
        out.lines()
            .any(|l| l.contains("dep-skill") && l.contains("up to date")),
        "{out}"
    );
    assert!(!removed_line(&out, "dep-skill"), "{out}");
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-skill/SKILL.md")
            .exists(),
        "{out}"
    );
    assert!(recorded_items(consumer.path()).contains(&"dep-skill".to_string()));
}

/// The root `top` in `registry` lists the rule `r` and the skill `own` and
/// requires `dep` from `dep_registry`, which lists the skill `dep-skill`.
fn setup_kind_restricted_root(registry: &Path, dep_registry: &Path) {
    write_rule(registry, "r");
    write_skill(registry, "own");
    write_skill(dep_registry, "dep-skill");
    write_skill_bundle(dep_registry, "dep", &["dep-skill"]);
    write(
        &registry.join("top.bundle.yaml"),
        &format!(
            "schema: 1\nname: top\ndescription: Root bundle.\nitems:\n  rules: [r]\n  skills: [own]\nrequires: [{{ name: dep, source: '{}' }}]\n",
            dep_registry.display()
        ),
    );
}

#[test]
fn an_unrestricted_dependency_does_not_lift_the_kind_restriction_of_its_root() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let dep = tempfile::tempdir().unwrap();
    setup_kind_restricted_root(registry.path(), dep.path());
    write_skill(second.path(), "own2");
    write_root_bundle(
        second.path(),
        "second",
        "own2",
        &[format!(
            "{{ name: dep, source: '{}' }}",
            dep.path().display()
        )],
    );
    add_with(consumer.path(), registry.path(), &["top", "--kind", "rule"]);
    add_with(consumer.path(), second.path(), &["second"]);
    assert!(!consumer.path().join(".claude/skills/own").exists());

    let out = update_output(consumer.path());

    assert!(
        !consumer.path().join(".claude/skills/own").exists(),
        "{out}"
    );
    assert!(consumer.path().join(".claude/rules/r.md").exists(), "{out}");
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-skill/SKILL.md")
            .exists(),
        "{out}"
    );
}

#[test]
fn a_restricted_dependency_keeps_the_kind_restriction_of_its_root() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let dep = tempfile::tempdir().unwrap();
    setup_kind_restricted_root(registry.path(), dep.path());
    write_skill(second.path(), "own2");
    write_root_bundle(
        second.path(),
        "second",
        "own2",
        &[format!(
            "{{ name: dep, source: '{}' }}",
            dep.path().display()
        )],
    );
    add_with(consumer.path(), registry.path(), &["top", "--kind", "rule"]);
    add_with(
        consumer.path(),
        second.path(),
        &["second", "--kind", "skill"],
    );
    assert!(!consumer.path().join(".claude/skills/own").exists());

    let out = update_output(consumer.path());

    // `top` keeps its kind `rule`; `dep` takes the kinds of both its roots
    // (#655).
    assert!(
        !consumer.path().join(".claude/skills/own").exists(),
        "{out}"
    );
    assert!(consumer.path().join(".claude/rules/r.md").exists(), "{out}");
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-skill/SKILL.md")
            .exists(),
        "{out}"
    );
}

#[test]
fn a_shared_dependency_keeps_the_kinds_of_each_of_its_roots() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let dep = tempfile::tempdir().unwrap();
    setup_kind_restricted_root(registry.path(), dep.path());
    write_rule(dep.path(), "dep-rule");
    write(
        &dep.path().join("dep.bundle.yaml"),
        "schema: 1\nname: dep\ndescription: Dependency bundle.\nitems:\n  rules: [dep-rule]\n  skills: [dep-skill]\n",
    );
    write_skill(second.path(), "own2");
    write_root_bundle(
        second.path(),
        "second",
        "own2",
        &[format!(
            "{{ name: dep, source: '{}' }}",
            dep.path().display()
        )],
    );
    add_with(consumer.path(), registry.path(), &["top", "--kind", "rule"]);
    add_with(
        consumer.path(),
        second.path(),
        &["second", "--kind", "skill"],
    );
    let dep_rule = consumer.path().join(".claude/rules/dep-rule.md");
    assert!(dep_rule.exists());

    let out = update_output(consumer.path());

    // The rule of `dep` belongs to the restriction of `top`, its skill to
    // the restriction of `second`.
    assert!(dep_rule.exists(), "{out}");
    assert!(!removed_line(&out, "dep-rule"), "{out}");
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-skill/SKILL.md")
            .exists(),
        "{out}"
    );
    assert!(
        !consumer.path().join(".claude/skills/own").exists(),
        "{out}"
    );
}

#[test]
fn update_lets_a_bundle_without_items_drop_its_requires() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "dep-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    write(
        &registry.path().join("top.bundle.yaml"),
        "schema: 1\nname: top\ndescription: Aggregator bundle.\nitems: {}\nrequires: [{ name: dep }]\n",
    );
    add_top(consumer.path(), registry.path());
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-only/SKILL.md")
            .exists()
    );

    write(
        &registry.path().join("top.bundle.yaml"),
        "schema: 1\nname: top\ndescription: Aggregator bundle.\nitems: {}\n",
    );
    let out = update_output(consumer.path());

    assert!(removed_line(&out, "dep-only"), "{out}");
    assert!(!consumer.path().join(".claude/skills/dep-only").exists());
    assert_eq!(recorded_bundles(consumer.path()), vec!["top"]);
}

#[test]
fn update_keeps_the_requirer_of_a_dependency_while_another_root_source_is_skipped() {
    // `first` and `second` require `mid`, which requires `dep`. `mid` drops
    // `dep` while the source of `second` is unreachable: the run evaluated
    // `mid` but cannot drop `dep`, so `dep` keeps its recorded requirer.
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let second = parent.path().join("second");
    write_skill(registry.path(), "own1");
    write_skill(registry.path(), "mid-skill");
    write_skill(registry.path(), "dep-only");
    write_skill(&second, "own2");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    let mid = |requires: &str| {
        write(
            &registry.path().join("mid.bundle.yaml"),
            &format!(
                "schema: 1\nname: mid\ndescription: Middle bundle.\nitems:\n  skills: [mid-skill]\n{requires}"
            ),
        )
    };
    mid("requires: [{ name: dep }]\n");
    write_root_bundle(
        registry.path(),
        "first",
        "own1",
        &["{ name: mid }".to_string()],
    );
    write_root_bundle(
        &second,
        "second",
        "own2",
        &[format!(
            "{{ name: mid, source: '{}' }}",
            registry.path().display()
        )],
    );
    add_bundle_named(consumer.path(), registry.path(), "first");
    add_bundle_named(consumer.path(), &second, "second");
    assert_eq!(
        bundle_required_by(consumer.path(), "dep"),
        serde_json::json!(["bundle:mid"])
    );

    mid("");
    fs::remove_dir_all(&second).unwrap();
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1);

    assert!(recorded_bundles(consumer.path()).contains(&"dep".to_string()));
    assert_eq!(
        bundle_required_by(consumer.path(), "dep"),
        serde_json::json!(["bundle:mid"])
    );
}

#[test]
fn a_bundle_file_source_records_the_requirer_of_its_dependency() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(registry.path(), "dep-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    write_top_bundle(registry.path(), &["{ name: dep }".to_string()]);
    add_bundle(consumer.path(), &registry.path().join("top.bundle.yaml"));
    assert_eq!(
        bundle_required_by(consumer.path(), "dep"),
        serde_json::json!(["bundle:top"])
    );
}

#[test]
fn update_keeps_a_dependency_recorded_before_required_by_existed() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(registry.path(), "dep-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    write_top_bundle(registry.path(), &["{ name: dep }".to_string()]);
    add_top(consumer.path(), registry.path());
    // A lockfile written before `required_by` existed.
    let path = consumer.path().join(".upskill-lock.json");
    let mut lock = read_update_lock(consumer.path());
    for bundle in lock["bundles"].as_array_mut().unwrap() {
        bundle.as_object_mut().unwrap().remove("required_by");
    }
    fs::write(&path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();

    write_top_bundle(registry.path(), &[]);
    let out = update_output(consumer.path());

    assert!(!removed_line(&out, "dep-only"), "{out}");
    assert!(
        consumer
            .path()
            .join(".claude/skills/dep-only/SKILL.md")
            .exists()
    );
    assert_eq!(recorded_bundles(consumer.path()), vec!["dep", "top"]);
}

/// An item that was added by name stays when the dependency bundle that also
/// lists it is dropped: the lockfile marks it `direct`.
#[test]
fn update_keeps_a_directly_installed_item_with_its_dropped_dependency() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(registry.path(), "dep-only");
    write_skill_bundle(registry.path(), "dep", &["dep-only"]);
    write_top_bundle(registry.path(), &["{ name: dep }".to_string()]);
    add_with(consumer.path(), registry.path(), &["dep-only"]);
    add_top(consumer.path(), registry.path());

    write_top_bundle(registry.path(), &[]);
    let out = update_output(consumer.path());

    assert!(!removed_line(&out, "dep-only"), "{out}");
    assert!(consumer.path().join(".claude/skills/dep-only").exists());
}

/// An item of a dropped dependency bundle that another source still installs
/// is recorded with the requirer of that source, so a later update does not
/// take it for a direct install (#655).
#[test]
fn an_item_of_a_dropped_dependency_keeps_the_provenance_of_its_installer() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let dep = tempfile::tempdir().unwrap();
    write_skill(registry.path(), "own");
    write_skill(dep.path(), "shared");
    write_skill(dep.path(), "other");
    write_skill_bundle(dep.path(), "dep", &["shared"]);
    let requires = format!("{{ name: dep, source: '{}' }}", dep.path().display());
    write_root_bundle(registry.path(), "top", "own", &[requires]);
    add_with(consumer.path(), registry.path(), &["top"]);
    add_with(consumer.path(), dep.path(), &["other"]);

    write_root_bundle(registry.path(), "top", "own", &[]);
    write(
        &dep.path().join("other/SKILL.md"),
        "---\nname: other\ndescription: other.\nrequires:\n  skills: [shared]\n---\nBody.\n",
    );
    let out = update_output(consumer.path());

    let lock = read_update_lock(consumer.path());
    let shared = lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "shared")
        .unwrap_or_else(|| panic!("{out}"));
    assert_eq!(
        shared["required_by"],
        serde_json::json!(["skill:other"]),
        "{out}"
    );
    assert_eq!(recorded_bundles(consumer.path()), vec!["top"]);
}
