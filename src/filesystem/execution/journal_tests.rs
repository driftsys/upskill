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
    std::fs::set_permissions(
        tmp.path().join(".upskill/state"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    std::fs::write(tmp.path().join(".upskill/state/manual"), b"manual").unwrap();
    let error = Journal::create(&scope, &request).err().unwrap();
    assert!(format!("{error:#}").contains("unowned execution state directory"));
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

#[test]
fn mismatched_owner_and_ignore_records_reject_without_adoption() {
    for change in ["owner", "missing-ignore", "changed-ignore"] {
        let (tmp, scope, request) = fixture();
        Journal::create(&scope, &request).unwrap();
        let store = tmp.path().join(".upskill/state");
        std::fs::write(store.join("manual"), b"manual").unwrap();
        match change {
            "owner" => {
                let path = store.join("owner.json");
                let mut value: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                value["directory"][1] = 0.into();
                std::fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
            }
            "missing-ignore" => std::fs::remove_file(store.join(".gitignore")).unwrap(),
            _ => std::fs::write(store.join(".gitignore"), b"manual-ignore").unwrap(),
        }
        assert!(Journal::load(&scope).is_err(), "{change}");
        assert_eq!(std::fs::read(store.join("manual")).unwrap(), b"manual");
        assert!(!tmp.path().join(".upskill-lock.json").exists());
    }
}
#[test]
fn structurally_invalid_persisted_journals_reject_before_outputs() {
    for change in [
        "schema",
        "root",
        "reserved",
        "duplicate",
        "ancestor",
        "state-order",
    ] {
        let (tmp, scope, mut request) = fixture();
        let mut file = request.state.clone();
        file.location.path = "a".into();
        request.files.push(file);
        Journal::create(&scope, &request).unwrap();
        let path = tmp.path().join(".upskill/state/operation.json");
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        match change {
            "schema" => v["schema"] = 2.into(),
            "root" => v["root_identity"][1] = 0.into(),
            "reserved" => v["actions"][0]["path"] = ".upskill/state/owner.json".into(),
            "duplicate" => v["actions"][0]["path"] = ".upskill-lock.json".into(),
            "ancestor" => v["actions"][1]["path"] = "a/state".into(),
            _ => {
                v["actions"][0]["state"] = true.into();
                v["actions"][1]["state"] = false.into();
            }
        }
        std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        assert!(scope.recover_operation(&guard).is_err(), "{change}");
        assert!(!tmp.path().join("a").exists());
        assert!(!tmp.path().join(".upskill-lock.json").exists());
        assert!(path.exists());
    }
}
#[test]
fn store_and_journal_aliases_reject_and_preserve_targets() {
    for component in [".upskill/state", ".upskill/state/operation.json"] {
        let (tmp, scope, request) = fixture();
        let mut journal = Journal::create(&scope, &request).unwrap();
        let path = tmp.path().join(component);
        let retained = tmp.path().join("retained");
        std::fs::rename(&path, &retained).unwrap();
        std::os::unix::fs::symlink(&retained, &path).unwrap();
        assert!(Journal::load(&scope).is_err());
        assert!(journal.record(0, Receipt::Pending).is_err());
        assert!(
            std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(retained.exists());
        assert!(!tmp.path().join(".upskill-lock.json").exists());
    }
}
