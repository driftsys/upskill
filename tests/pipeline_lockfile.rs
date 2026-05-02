//! ATDD tests for `pipeline::install_with_lockfile`.
//!
//! Validates that a successful install writes a v0.2 schema-2 lockfile
//! (`.upskill-lock.json`) at the consumer-project root with one entry
//! per `(kind, name)` carrying the source label, optional git ref, and
//! SSOT content hash.

use std::fs;
use std::path::Path;
use upskill::lockfile_v2::{CURRENT_SCHEMA, LockfileV2};
use upskill::pipeline::install_with_lockfile;
use upskill::source::InstallSource;

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

#[test]
fn install_writes_schema_2_lockfile_at_target_root() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target).expect("install");

    let lock_path = target.join(".upskill-lock.json");
    assert!(
        lock_path.exists(),
        "lockfile must be written at target root"
    );

    let raw = fs::read_to_string(&lock_path).unwrap();
    let parsed: LockfileV2 = serde_json::from_str(&raw).expect("valid schema-2 JSON");
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

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target).expect("install 1");
    let lock1: LockfileV2 =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target).expect("install 2");
    let lock2: LockfileV2 =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();

    // Re-installing the same source MUST NOT duplicate entries.
    assert_eq!(lock1.items.len(), lock2.items.len());
    assert_eq!(lock1, lock2, "lockfile is byte-identical on re-install");
}

#[test]
fn install_preserves_unrelated_existing_entries() {
    use upskill::lockfile_v2::LockedItem;

    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();

    // Pre-seed the lockfile with an entry from a different source.
    let mut seed = LockfileV2::new();
    seed.upsert(LockedItem {
        kind: "skill".into(),
        name: "from-other-source".into(),
        source: "github:other/repo@v1.0".into(),
        git_ref: Some("v1.0".into()),
        hash: Some("a".repeat(64)),
    });
    seed.save(&target).unwrap();

    install_with_lockfile(&InstallSource::LocalPath(source.clone()), &target).expect("install");

    let lock = LockfileV2::load(&target).expect("load");
    // 4 from the install + 1 pre-seeded = 5.
    assert_eq!(lock.items.len(), 5);
    assert!(
        lock.items.iter().any(|i| i.name == "from-other-source"),
        "pre-existing entry must survive install"
    );
}
