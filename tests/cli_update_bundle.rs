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

#[test]
fn update_fails_before_any_write_when_a_recorded_bundle_left_the_source() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    let paths = [
        ".claude/skills/a/SKILL.md",
        ".claude/rules/r.md",
        ".upskill-lock.json",
        "CLAUDE.md",
    ];
    let before: Vec<_> = paths
        .iter()
        .map(|path| fs::read(consumer.path().join(path)).unwrap())
        .collect();
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    write(
        &registry.path().join("a/SKILL.md"),
        "---\nname: a\ndescription: A changed skill.\n---\nChanged skill body.\n",
    );
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("bun"));
    for (path, contents) in paths.iter().zip(before) {
        assert_eq!(
            fs::read(consumer.path().join(path)).unwrap(),
            contents,
            "{path} changed"
        );
    }
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

#[test]
fn update_refuses_an_item_in_place_of_a_recorded_bundle_before_any_write() {
    let consumer = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    setup_kind_registry(registry.path());
    add_named_bundle(consumer.path(), registry.path(), &[]);
    let paths = [
        ".upskill-lock.json",
        ".claude/skills/a/SKILL.md",
        ".claude/rules/r.md",
        "CLAUDE.md",
    ];
    let before: Vec<_> = paths
        .iter()
        .map(|p| fs::read(consumer.path().join(p)).unwrap())
        .collect();
    fs::remove_file(registry.path().join("bun.bundle.yaml")).unwrap();
    write_skill(registry.path(), "bun");
    common::upskill_cmd(&fake_home(consumer.path()))
        .current_dir(consumer.path())
        .args(["update", "--yes", "--project"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "bundle 'bun' is no longer in source",
        ));
    for (path, bytes) in paths.iter().zip(before) {
        assert_eq!(fs::read(consumer.path().join(path)).unwrap(), bytes);
    }
    assert!(!consumer.path().join(".claude/skills/bun").exists());
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
fn add_under_a_kind_restriction_refreshes_the_provenance_of_an_unplanned_item() {
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
    write_rule_body(registry.path(), "r2", "No dependency remains.");
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
    assert!(item.get("required_by").is_none());
}
