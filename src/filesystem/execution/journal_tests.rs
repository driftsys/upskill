use super::*;
use crate::files::plan::{FileLocation, FileValue, PlannedFile};
use crate::ownership::{InstallationRoot, OwnershipScope};
use std::os::unix::fs::MetadataExt;
fn fixture() -> (tempfile::TempDir, QualifiedScope, ExecutionRequest) {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let state = PlannedFile {
        location: FileLocation {
            root,
            path: ".upskill-lock.json".into(),
        },
        before: None,
        after: Some(FileValue {
            bytes: b"state".to_vec(),
            mode: Some(0o600),
        }),
    };
    (
        tmp,
        scope,
        ExecutionRequest {
            files: vec![],
            state,
        },
    )
}
#[test]
fn journal_roundtrip_retains_complete_action_sequence() {
    let (_tmp, scope, request) = fixture();
    let journal = Journal::create(&scope, &request).unwrap();
    let loaded = Journal::load(&scope).unwrap().unwrap();
    assert_eq!(loaded.disk, journal.disk);
    assert_eq!(loaded.payload(0).unwrap().unwrap().bytes, b"state");
}
#[test]
fn corrupt_or_unknown_journal_rejects() {
    let (tmp, scope, request) = fixture();
    Journal::create(&scope, &request).unwrap();
    let path = tmp.path().join(".upskill/state/operation.json");
    std::fs::write(&path, b"{bad").unwrap();
    assert!(Journal::load(&scope).is_err());
}
#[test]
fn payload_digest_or_identity_mismatch_rejects() {
    let (tmp, scope, request) = fixture();
    let j = Journal::create(&scope, &request).unwrap();
    let name = &j.disk.actions[0].payload.as_ref().unwrap().name;
    std::fs::write(tmp.path().join(".upskill/state").join(name), b"other").unwrap();
    assert!(Journal::load(&scope).is_err());
}
#[test]
fn existing_unowned_state_directory_rejects() {
    let (tmp, scope, request) = fixture();
    std::fs::create_dir_all(tmp.path().join(".upskill/state")).unwrap();
    std::fs::write(tmp.path().join(".upskill/state/manual"), b"manual").unwrap();
    assert!(Journal::create(&scope, &request).is_err());
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill/state/manual")).unwrap(),
        b"manual"
    );
}
#[test]
fn journal_permissions_are_restricted() {
    let (tmp, scope, request) = fixture();
    Journal::create(&scope, &request).unwrap();
    for (name, mode) in [
        (".upskill/state", 0o700),
        (".upskill/state/operation.json", 0o600),
    ] {
        assert_eq!(
            std::fs::metadata(tmp.path().join(name)).unwrap().mode() & 0o7777,
            mode
        );
    }
}
#[test]
fn cleanup_failure_retains_completed_receipt() {
    let (tmp, scope, request) = fixture();
    let mut j = Journal::create(&scope, &request).unwrap();
    j.record(0, Receipt::Done).unwrap();
    let path = tmp
        .path()
        .join(".upskill/state")
        .join(&j.disk.actions[0].payload.as_ref().unwrap().name);
    std::fs::remove_file(&path).unwrap();
    std::fs::write(&path, b"manual").unwrap();
    assert!(j.cleanup().is_err());
    assert!(tmp.path().join(".upskill/state/operation.json").exists());
    assert_eq!(std::fs::read(path).unwrap(), b"manual");
}
#[test]
fn unknown_fields_and_replaced_identical_payload_reject() {
    let (tmp, scope, request) = fixture();
    let j = Journal::create(&scope, &request).unwrap();
    let path = tmp.path().join(".upskill/state/operation.json");
    let original = std::fs::read(&path).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    value["unknown"] = true.into();
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(Journal::load(&scope).is_err());
    std::fs::write(path, original).unwrap();
    let payload = tmp
        .path()
        .join(".upskill/state")
        .join(&j.disk.actions[0].payload.as_ref().unwrap().name);
    let held = std::fs::File::open(&payload).unwrap();
    std::fs::remove_file(&payload).unwrap();
    std::fs::write(&payload, b"state").unwrap();
    std::fs::set_permissions(&payload, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(Journal::load(&scope).is_err());
    drop(held);
}
