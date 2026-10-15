//! ATDD tests for `pipeline::install_with_lockfile`.
//!
//! Validates that a successful install writes a `schema: 1` lockfile
//! (`.upskill-lock.json`) at the consumer-project root with one entry
//! per `(kind, name)` carrying the source label, optional git ref, and
//! SSOT content hash.

use std::fs;
use std::path::Path;
use upskill::lockfile::{CURRENT_SCHEMA, LockedPlugin, Lockfile, PluginInstallStatus};
use upskill::pipeline::{doctor, install_with_lockfile};
use upskill::plugin::PluginScope;
use upskill::source::InstallSource;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn stage_source(source: &Path) {
    let from = format!("{FIXTURES}/items");
    copy_dir_all(Path::new(&from), source).unwrap();
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
fn install_writes_lockfile_at_target_root() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    install_with_lockfile(
        &InstallSource::LocalPath(source.clone()),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    let lock_path = target.join(".upskill-lock.json");
    assert!(
        lock_path.exists(),
        "lockfile must be written at target root"
    );

    let raw = fs::read_to_string(&lock_path).unwrap();
    let parsed: Lockfile = serde_json::from_str(&raw).expect("valid lockfile JSON");
    assert_eq!(parsed.schema, CURRENT_SCHEMA);

    // 1 skill + 2 rules + 1 agent = 4 unique items (one entry per kind/name,
    // not per client).
    assert_eq!(parsed.items.len(), 4, "{:#?}", parsed.items);

    // Every entry has the source label and a SHA-256 hash; local-path
    // sources record no git_ref.
    let expected_label = format!("local:{}", source.display());
    for item in &parsed.items {
        assert_eq!(item.source, expected_label, "source label per item");
        assert!(item.git_ref.is_none(), "no ref for local source");
        let h = item.hash.as_ref().expect("hash present");
        assert!(
            h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()),
            "hash looks like sha-256 hex: {h}"
        );
    }

    // Items are sorted by (kind, name) for deterministic on-disk output.
    let keys: Vec<_> = parsed
        .items
        .iter()
        .map(|i| (i.kind, i.name.as_str()))
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

#[test]
fn re_install_upserts_existing_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    install_with_lockfile(
        &InstallSource::LocalPath(source.clone()),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install 1");
    let lock1: Lockfile =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();

    install_with_lockfile(
        &InstallSource::LocalPath(source.clone()),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install 2");
    let lock2: Lockfile =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();

    // Re-installing the same source MUST NOT duplicate entries.
    assert_eq!(lock1.items.len(), lock2.items.len());
    assert_eq!(lock1, lock2, "lockfile is byte-identical on re-install");
}

#[test]
fn install_preserves_unrelated_existing_entries() {
    use upskill::lockfile::LockedItem;

    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    // Pre-seed the lockfile with an entry from a different source.
    let mut seed = Lockfile::new();
    seed.upsert(LockedItem {
        kind: upskill::pipeline::ItemKind::Skill,
        name: "from-other-source".into(),
        source: "github:other/repo@v1.0".into(),
        git_ref: Some("v1.0".into()),
        hash: Some("a".repeat(64)),
        source_name: None,
        required_by: vec![],
        group: None,
        clients: vec![],
    });
    seed.save(&target).unwrap();

    install_with_lockfile(
        &InstallSource::LocalPath(source.clone()),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    let lock = Lockfile::load(&target).expect("load");
    // 4 from the install + 1 pre-seeded = 5.
    assert_eq!(lock.items.len(), 5);
    assert!(
        lock.items.iter().any(|i| i.name == "from-other-source"),
        "pre-existing entry must survive install"
    );
}

#[test]
fn install_creates_claude_bridge_when_absent() {
    // ADR-0003 / format-spec §7.4: a fresh consumer project gets a CLAUDE.md
    // bridge file (single line `@AGENTS.md`) so Claude Code picks up
    // project-level instructions.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    install_with_lockfile(
        &InstallSource::LocalPath(source.clone()),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    let claude_md = target.join("CLAUDE.md");
    assert!(claude_md.exists(), "CLAUDE.md must be created");
    assert_eq!(fs::read_to_string(&claude_md).unwrap(), "@AGENTS.md\n");
}

#[test]
fn install_registers_opencode_rules_glob_when_rules_present() {
    // The fixture corpus includes 2 rules, so opencode.json should gain the
    // `.agents/rules/**/RULE.md` entry under instructions[].
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    install_with_lockfile(
        &InstallSource::LocalPath(source.clone()),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    let raw = fs::read_to_string(target.join("opencode.json")).expect("opencode.json present");
    let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let arr = doc["instructions"].as_array().expect("instructions array");
    assert!(
        arr.iter()
            .any(|v| v.as_str() == Some(".agents/rules/**/RULE.md")),
        "rules glob registered: {arr:?}"
    );
}

#[test]
fn install_registers_vscode_instructions_location_when_rules_present() {
    // The fixture corpus includes 2 rules, so .vscode/settings.json should
    // gain `.github/instructions: true` under chat.instructionsFilesLocations
    // for VS Code Copilot rule discovery.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    install_with_lockfile(
        &InstallSource::LocalPath(source.clone()),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    let raw = fs::read_to_string(target.join(".vscode/settings.json"))
        .expect(".vscode/settings.json present");
    let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let map = doc["chat.instructionsFilesLocations"]
        .as_object()
        .expect("instructions-locations map");
    assert_eq!(
        map[".github/instructions"], true,
        "vscode instructions path registered: {map:?}"
    );
}

#[test]
fn install_preserves_existing_claude_bridge() {
    // User customisations must not be clobbered.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    let user_content = "# Project CLAUDE.md\n\n@AGENTS.md\n\nExtra project notes here.\n";
    fs::write(target.join("CLAUDE.md"), user_content).unwrap();

    install_with_lockfile(
        &InstallSource::LocalPath(source.clone()),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    assert_eq!(
        fs::read_to_string(target.join("CLAUDE.md")).unwrap(),
        user_content,
        "user CLAUDE.md must be preserved verbatim"
    );
}

// ---------------------------------------------------------------------------
// Plugin pipeline tests
// ---------------------------------------------------------------------------

/// Stage `tests/fixtures/items` + `tests/fixtures/bundles` into a registry
/// (like `cli_bundle_install` does), then install the `with-plugins` bundle
/// via the library function inside an isolated child with no native client CLI
/// on PATH. Plugins must be warn-skipped without external registrations.
// Run library plugin tests in a child process so PATH/HOME isolation is local
// to that test and cannot race other parallel Rust tests.
fn isolated_plugin_test(test: &str) -> bool {
    if std::env::var("UPSKILL_PIPELINE_PLUGIN_TEST").as_deref() == Ok(test) {
        let expected = std::env::var_os("UPSKILL_PIPELINE_PLUGIN_HOME")
            .expect("parent must supply isolated home evidence");
        assert_eq!(std::env::var_os("HOME"), Some(expected.clone()));
        assert_eq!(std::env::var_os("USERPROFILE"), Some(expected));
        return false;
    }
    let tmp = tempfile::tempdir().unwrap();
    let empty_path = tmp.path().join("empty-path");
    let home = tmp.path().join("home");
    fs::create_dir_all(&empty_path).unwrap();
    fs::create_dir_all(&home).unwrap();
    assert_cmd::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("UPSKILL_PIPELINE_PLUGIN_TEST", test)
        .env("UPSKILL_PIPELINE_PLUGIN_HOME", &home)
        .env("PATH", &empty_path)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .current_dir(tmp.path())
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicates::str::contains(format!("test {test} ... ok")));
    true
}

fn stage_registry(root: &Path) {
    let items = format!("{FIXTURES}/items");
    let bundles = format!("{FIXTURES}/bundles");
    copy_dir_all(Path::new(&items), root).unwrap();
    copy_dir_all(Path::new(&bundles), &root.join("bundles")).unwrap();
}

#[test]
fn bundle_with_plugins_produces_plugin_results() {
    if isolated_plugin_test("bundle_with_plugins_produces_plugin_results") {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();

    let bundle_path = registry.join("bundles/with-plugins.bundle.yaml");
    let report = install_with_lockfile(
        &InstallSource::LocalPath(bundle_path),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    // Three plugins attempted: superpowers (claude + copilot + vscode).
    assert_eq!(
        report.plugin_results.len(),
        3,
        "expected 3 plugin results (claude + copilot + vscode)"
    );

    // Results are deterministic even when the developer has native CLIs.
    assert!(
        report
            .plugin_results
            .iter()
            .all(|result| result.outcome.is_cli_not_found())
    );
    for pr in &report.plugin_results {
        assert!(!pr.name.is_empty(), "plugin result should carry a name");
        assert!(!pr.client.is_empty(), "plugin result should carry a client");
        assert!(
            !pr.identifier.is_empty(),
            "plugin result should carry an identifier"
        );
    }
}

#[test]
fn cli_not_found_plugins_recorded_as_skipped_in_lockfile() {
    if isolated_plugin_test("cli_not_found_plugins_recorded_as_skipped_in_lockfile") {
        return;
    }
    // CliNotFound outcomes (warn-skip) must be recorded in the lockfile with
    // status "skipped" so doctor can surface them. Failed outcomes are NOT
    // recorded (only transient — the CLI is present but misbehaving).
    //
    // The child excludes real native CLIs. Every outcome must be CliNotFound,
    // and each must be persisted as Skipped.
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();

    let bundle_path = registry.join("bundles/with-plugins.bundle.yaml");
    let report = install_with_lockfile(
        &InstallSource::LocalPath(bundle_path),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    let lock = Lockfile::load(&target).expect("load lockfile");

    // CliNotFound outcomes are recorded as Skipped; Success as Installed.
    // Failed outcomes are never recorded.
    let cli_not_found_count = report
        .plugin_results
        .iter()
        .filter(|pr| pr.outcome.is_cli_not_found())
        .count();
    let success_count = report
        .plugin_results
        .iter()
        .filter(|pr| pr.outcome.is_success())
        .count();
    let skipped_count = lock
        .plugins
        .iter()
        .filter(|p| p.status == PluginInstallStatus::Skipped)
        .count();
    let installed_count = lock
        .plugins
        .iter()
        .filter(|p| p.status == PluginInstallStatus::Installed)
        .count();

    assert_eq!(cli_not_found_count, 3);
    assert_eq!(success_count, 0);
    assert_eq!(
        cli_not_found_count, skipped_count,
        "each CliNotFound outcome must produce exactly one Skipped lockfile entry \
         (cli_not_found={cli_not_found_count}, skipped_in_lockfile={skipped_count})"
    );
    assert_eq!(
        success_count, installed_count,
        "each Success outcome must produce exactly one Installed lockfile entry \
         (success={success_count}, installed_in_lockfile={installed_count})"
    );

    // No Failed outcomes should leak into the lockfile.
    let failed_count = report
        .plugin_results
        .iter()
        .filter(|pr| !pr.outcome.is_success() && !pr.outcome.is_cli_not_found())
        .count();
    assert_eq!(
        lock.plugins.len(),
        cli_not_found_count + success_count,
        "lockfile should contain only CliNotFound (Skipped) + Success (Installed); \
         {failed_count} Failed outcomes must not appear"
    );
}

#[test]
fn plugin_results_carry_correct_metadata() {
    if isolated_plugin_test("plugin_results_carry_correct_metadata") {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();

    let bundle_path = registry.join("bundles/with-plugins.bundle.yaml");
    let report = install_with_lockfile(
        &InstallSource::LocalPath(bundle_path),
        &target,
        &[],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    let claude_result = report
        .plugin_results
        .iter()
        .find(|r| r.client == "claude")
        .expect("claude result");
    assert_eq!(claude_result.name, "superpowers");
    // Identifier uses the marketplace NAME (`claude-plugins`), not the source
    // (`anthropics/claude-plugins`) — `claude plugin install` requires
    // `<plugin>@<marketplace>` (#227).
    assert_eq!(claude_result.identifier, "superpowers@claude-plugins");
    assert_eq!(claude_result.bundle, "with-plugins");
    assert_eq!(
        claude_result.install_url.as_deref(),
        Some("https://github.com/obra/superpowers#install")
    );

    let vscode_result = report
        .plugin_results
        .iter()
        .find(|r| r.client == "vscode")
        .expect("vscode result");
    assert_eq!(vscode_result.name, "superpowers");
    assert_eq!(vscode_result.identifier, "anthropic.superpowers");
    assert_eq!(vscode_result.bundle, "with-plugins");

    let copilot_result = report
        .plugin_results
        .iter()
        .find(|r| r.client == "copilot")
        .expect("copilot result");
    assert_eq!(copilot_result.name, "superpowers");
    // Marketplace name (`superpowers-marketplace`), not source.
    assert_eq!(
        copilot_result.identifier,
        "superpowers@superpowers-marketplace"
    );
    assert_eq!(copilot_result.bundle, "with-plugins");
    assert_eq!(
        copilot_result.install_url.as_deref(),
        Some("https://github.com/obra/superpowers#install")
    );
}

// ---------------------------------------------------------------------------
// Bundle-by-name discovery tests
// ---------------------------------------------------------------------------

#[test]
fn install_by_name_discovers_bundle_when_no_item_matches() {
    if isolated_plugin_test("install_by_name_discovers_bundle_when_no_item_matches") {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();

    // "with-plugins" is a bundle name, not an item name.
    let report = install_with_lockfile(
        &InstallSource::LocalPath(registry.clone()),
        &target,
        &["with-plugins".into()],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    // Bundle dispatch should have fired — items from the bundle are installed.
    assert!(
        !report.items.is_empty(),
        "expected bundle items to be installed"
    );
    assert!(
        !report.bundles.is_empty(),
        "expected bundle to appear in report"
    );
    assert_eq!(report.bundles[0].name, "with-plugins");
}

#[test]
fn install_by_name_errors_on_ambiguity() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();

    // Create an item directory that collides with bundle name "with-plugins"
    let collision_dir = registry.join("with-plugins");
    fs::create_dir_all(&collision_dir).unwrap();
    fs::write(
        collision_dir.join("SKILL.md"),
        "---\nschema: 1\nname: with-plugins\ndescription: collision\n---\n# body\n",
    )
    .unwrap();

    let err = install_with_lockfile(
        &InstallSource::LocalPath(registry.clone()),
        &target,
        &["with-plugins".into()],
        PluginScope::Project,
        &Default::default(),
    )
    .expect_err("should error on ambiguity");

    let msg = format!("{:#}", err);
    assert!(
        msg.contains("matches both"),
        "error mentions ambiguity: {msg}"
    );
}

#[test]
fn install_by_name_prefers_items_when_only_items_match() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();

    // "license-awareness" is a rule item, not a bundle.
    let report = install_with_lockfile(
        &InstallSource::LocalPath(registry.clone()),
        &target,
        &["license-awareness".into()],
        PluginScope::Project,
        &Default::default(),
    )
    .expect("install");

    assert!(!report.items.is_empty());
    assert!(
        report.bundles.is_empty(),
        "no bundle dispatch for item-only match"
    );
}

// ---------------------------------------------------------------------------
// Doctor plugin reconciliation unit tests (issue #151)
// ---------------------------------------------------------------------------

#[test]
fn doctor_reports_skipped_plugin_from_lockfile() {
    // A lockfile with a Skipped plugin should surface it in skipped_plugins.
    // No CLI calls needed — status: skipped is sufficient.
    let tmp = tempfile::tempdir().unwrap();
    let mut lock = Lockfile::new();
    lock.upsert_plugin(LockedPlugin {
        name: "superpowers".into(),
        client: "claude".into(),
        identifier: "superpowers@anthropics/claude-plugins".into(),
        scope: Some("project".into()),
        bundle: "baseline".into(),
        status: PluginInstallStatus::Skipped,
    });
    lock.save(tmp.path()).expect("save");

    let report = doctor(tmp.path()).expect("doctor");

    assert_eq!(
        report.skipped_plugins.len(),
        1,
        "expected 1 skipped plugin, got: {:?}",
        report.skipped_plugins
    );
    assert_eq!(report.skipped_plugins[0].name, "superpowers");
    assert_eq!(report.skipped_plugins[0].client, "claude");
}

#[test]
fn doctor_skipped_plugins_do_not_cause_is_clean_false() {
    // Skipped plugins are warnings (the CLI was absent), not drift.
    // is_clean() should remain true when only skipped_plugins is non-empty.
    let tmp = tempfile::tempdir().unwrap();
    let mut lock = Lockfile::new();
    lock.upsert_plugin(LockedPlugin {
        name: "superpowers".into(),
        client: "vscode".into(),
        identifier: "anthropic.superpowers".into(),
        scope: None,
        bundle: "baseline".into(),
        status: PluginInstallStatus::Skipped,
    });
    lock.save(tmp.path()).expect("save");

    let report = doctor(tmp.path()).expect("doctor");

    assert!(!report.skipped_plugins.is_empty());
    assert!(
        report.is_clean(),
        "is_clean() should be true when only skipped_plugins present"
    );
}

#[test]
fn doctor_installed_plugin_cli_not_found_is_not_reported_as_missing() {
    if isolated_plugin_test("doctor_installed_plugin_cli_not_found_is_not_reported_as_missing") {
        return;
    }
    // When a plugin is recorded as Installed but the client CLI is gone,
    // we cannot verify the install state.  Doctor should NOT add it to
    // missing_plugins — it cannot determine the actual state.
    let tmp = tempfile::tempdir().unwrap();
    let mut lock = Lockfile::new();
    // The child PATH excludes the VS Code CLI.
    lock.upsert_plugin(LockedPlugin {
        name: "some-plugin".into(),
        client: "vscode".into(),
        identifier: "vendor.some-plugin".into(),
        scope: None,
        bundle: "baseline".into(),
        status: PluginInstallStatus::Installed,
    });
    lock.save(tmp.path()).expect("save");

    let report = doctor(tmp.path()).expect("doctor");
    assert!(report.missing_plugins.is_empty());
}

#[test]
fn doctor_reports_multiple_skipped_plugins() {
    let tmp = tempfile::tempdir().unwrap();
    let mut lock = Lockfile::new();
    lock.upsert_plugin(LockedPlugin {
        name: "plugin-a".into(),
        client: "claude".into(),
        identifier: "plugin-a@source".into(),
        scope: Some("project".into()),
        bundle: "bundle-x".into(),
        status: PluginInstallStatus::Skipped,
    });
    lock.upsert_plugin(LockedPlugin {
        name: "plugin-b".into(),
        client: "vscode".into(),
        identifier: "vendor.plugin-b".into(),
        scope: None,
        bundle: "bundle-y".into(),
        status: PluginInstallStatus::Skipped,
    });
    lock.save(tmp.path()).expect("save");

    let report = doctor(tmp.path()).expect("doctor");

    assert_eq!(report.skipped_plugins.len(), 2);
    let names: Vec<&str> = report
        .skipped_plugins
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert!(names.contains(&"plugin-a"), "plugin-a missing: {names:?}");
    assert!(names.contains(&"plugin-b"), "plugin-b missing: {names:?}");
    assert!(report.is_clean());
}
