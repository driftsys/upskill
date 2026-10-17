#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::os::unix::fs::{MetadataExt, PermissionsExt};
use upskill::files::plan::{FileLocation, FileValue, PlannedFile};
use upskill::filesystem::{ExecutionOutcome, ExecutionRequest, QualifiedScope};
use upskill::ownership::{InstallationRoot, OwnershipScope};

const ORIGINAL: &[u8] = b"{ \"schema\": 1, \"unknown\": [true] }\n";

fn fixture() -> (
    tempfile::TempDir,
    QualifiedScope,
    ExecutionRequest,
    FileLocation,
) {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    let location = |path: &str| FileLocation {
        root: root.clone(),
        path: path.into(),
    };
    let value = |bytes: &[u8]| FileValue {
        bytes: bytes.to_vec(),
        mode: Some(0o600),
    };
    std::fs::write(tmp.path().join(".upskill-lock.json"), ORIGINAL).unwrap();
    std::fs::set_permissions(
        tmp.path().join(".upskill-lock.json"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let request = ExecutionRequest {
        files: vec![PlannedFile {
            location: location("rule.md"),
            before: None,
            after: Some(value(b"new rule\n")),
        }],
        state: PlannedFile {
            location: location(".upskill-lock.json"),
            before: Some(value(ORIGINAL)),
            after: Some(value(b"schema two\n")),
        },
    };
    let backup = location(".upskill-lock.schema-1.json");
    let scope = QualifiedScope::open(&root).unwrap();
    (tmp, scope, request, backup)
}

#[test]
fn preserves_exact_original_bytes_before_applying_outputs() {
    let (tmp, scope, request, backup) = fixture();
    let guard = scope.try_lock_writer().unwrap();
    let report = scope
        .apply_operation_with_backup(&guard, &[], request, backup)
        .unwrap();
    assert_eq!(report.outcome, ExecutionOutcome::Complete);
    assert_eq!(
        (report.recorded_prefix, report.verified_post_prefix),
        (3, 3)
    );
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.schema-1.json")).unwrap(),
        ORIGINAL
    );
    assert_eq!(
        std::fs::metadata(tmp.path().join(".upskill-lock.schema-1.json"))
            .unwrap()
            .mode()
            & 0o7777,
        0o600
    );
    assert_eq!(
        std::fs::read(tmp.path().join("rule.md")).unwrap(),
        b"new rule\n"
    );
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        b"schema two\n"
    );
    assert!(scope.recover_operation(&guard).unwrap().is_none());
}

#[test]
fn matching_backup_retains_physical_identity() {
    let (tmp, scope, request, backup) = fixture();
    let path = tmp.path().join(&backup.path);
    std::fs::write(&path, ORIGINAL).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let before = std::fs::metadata(&path).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    scope
        .apply_operation_with_backup(&guard, &[], request, backup)
        .unwrap();
    let after = std::fs::metadata(&path).unwrap();
    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
    assert_eq!(std::fs::read(path).unwrap(), ORIGINAL);
}

#[test]
fn mismatched_backup_blocks_before_intent_or_outputs() {
    for (bytes, mode) in [(b"manual".as_slice(), 0o600), (ORIGINAL, 0o644)] {
        let (tmp, scope, request, backup) = fixture();
        let path = tmp.path().join(&backup.path);
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        assert!(
            scope
                .apply_operation_with_backup(&guard, &[], request, backup)
                .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        assert_eq!(
            std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
            ORIGINAL
        );
        assert!(!tmp.path().join("rule.md").exists());
        assert!(!tmp.path().join(".upskill").exists());
    }
}

#[test]
fn stale_output_blocks_before_creating_backup() {
    let (tmp, scope, request, backup) = fixture();
    std::fs::write(tmp.path().join("rule.md"), b"manual").unwrap();
    let guard = scope.try_lock_writer().unwrap();
    assert!(
        scope
            .apply_operation_with_backup(&guard, &[], request, backup.clone())
            .is_err()
    );
    assert!(!tmp.path().join(backup.path).exists());
    assert_eq!(
        std::fs::read(tmp.path().join("rule.md")).unwrap(),
        b"manual"
    );
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        ORIGINAL
    );
}

#[test]
fn rejects_backup_aliases_reserved_paths_and_missing_original_state() {
    for path in [
        "rule.md",
        ".upskill-lock.json",
        ".upskill/state/backup",
        "../backup",
        "rule.md/backup",
    ] {
        let (tmp, scope, request, mut backup) = fixture();
        backup.path = path.into();
        let guard = scope.try_lock_writer().unwrap();
        assert!(
            scope
                .apply_operation_with_backup(&guard, &[], request, backup)
                .is_err(),
            "accepted {path}"
        );
        assert!(!tmp.path().join("rule.md").exists());
        assert!(!tmp.path().join(".upskill").exists());
    }
    let (tmp, scope, mut request, backup) = fixture();
    request.state.before = None;
    let guard = scope.try_lock_writer().unwrap();
    assert!(
        scope
            .apply_operation_with_backup(&guard, &[], request, backup)
            .is_err()
    );
    assert!(!tmp.path().join(".upskill").exists());
}
