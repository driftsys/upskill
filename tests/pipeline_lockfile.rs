//! ATDD tests for `pipeline::install_with_lockfile`.
//!
//! Validates that a successful install writes a `schema: 1` lockfile
//! (`.upskill-lock.json`) at the consumer-project root with one entry
//! per `(kind, name)` carrying the source label, optional git ref, and
//! SSOT content hash.

use std::fs;
use std::path::Path;
use upskill::lockfile::{CURRENT_SCHEMA, Lockfile};
use upskill::pipeline::install_with_lockfile;
use upskill::source::InstallSource;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn stage_source(source: &Path) {
    let from = format!("{FIXTURES}/items");
    copy_dir_all(Path::new(&from), &source).unwrap();
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

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target, &[])
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
        .map(|i| (i.kind.as_str(), i.name.as_str()))
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

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target, &[])
        .expect("install 1");
    let lock1: Lockfile =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target, &[])
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
        kind: "skill".into(),
        name: "from-other-source".into(),
        source: "github:other/repo@v1.0".into(),
        git_ref: Some("v1.0".into()),
        hash: Some("a".repeat(64)),
    });
    seed.save(&target).unwrap();

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target, &[])
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

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target, &[])
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

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target, &[])
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

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target, &[])
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

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target, &[])
        .expect("install");

    assert_eq!(
        fs::read_to_string(target.join("CLAUDE.md")).unwrap(),
        user_content,
        "user CLAUDE.md must be preserved verbatim"
    );
}
