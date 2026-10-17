use super::*;
use crate::files::plan::{FileLocation, FileValue, PlannedFile};
use crate::filesystem::QualifiedScope;
use crate::ownership::{InstallationRoot, OwnershipScope};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

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
    std::fs::write(tmp.path().join("state"), b"original state\n").unwrap();
    std::fs::set_permissions(
        tmp.path().join("state"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let request = ExecutionRequest {
        files: vec![PlannedFile {
            location: location("output"),
            before: None,
            after: Some(value(b"new output\n")),
        }],
        state: PlannedFile {
            location: location("state"),
            before: Some(value(b"original state\n")),
            after: Some(value(b"new state\n")),
        },
    };
    let backup = location("backup");
    (tmp, QualifiedScope::open(&root).unwrap(), request, backup)
}

struct ResetHook;
impl Drop for ResetHook {
    fn drop(&mut self) {
        TEST_HOOK.with(|h| *h.borrow_mut() = None);
        backend::TEST_SYNC.with(|h| *h.borrow_mut() = None);
    }
}

#[test]
fn backup_publication_does_not_replace_a_late_arrival() {
    let (tmp, scope, request, backup) = fixture();
    let path = tmp.path().join("backup");
    let late_path = path.clone();
    let _reset = ResetHook;
    TEST_HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move |label, index| {
            if label == "before_effect" && index == 0 {
                std::fs::write(&late_path, b"late manual bytes")?;
                std::fs::set_permissions(&late_path, std::fs::Permissions::from_mode(0o600))?;
            }
            Ok(())
        }))
    });
    let guard = scope.try_lock_writer().unwrap();
    assert!(
        scope
            .apply_operation_with_backup(&guard, &[], request, backup)
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"late manual bytes");
    assert_eq!(
        std::fs::read(tmp.path().join("state")).unwrap(),
        b"original state\n"
    );
    assert!(!tmp.path().join("output").exists());
    TEST_HOOK.with(|h| *h.borrow_mut() = None);
    let report = scope.recover_operation(&guard).unwrap().unwrap();
    assert_eq!(report.outcome, ExecutionOutcome::Reconcile { index: 0 });
    assert_eq!(
        (report.recorded_prefix, report.verified_post_prefix),
        (0, 0)
    );
    assert_eq!(std::fs::read(path).unwrap(), b"late manual bytes");
}

#[test]
fn edited_completed_backup_blocks_following_outputs_during_recovery() {
    let (tmp, scope, request, backup) = fixture();
    let _reset = ResetHook;
    TEST_HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(|label, index| {
            anyhow::ensure!(label != "after_done" || index != 0, "interruption");
            Ok(())
        }))
    });
    let guard = scope.try_lock_writer().unwrap();
    assert!(
        scope
            .apply_operation_with_backup(&guard, &[], request, backup)
            .is_err()
    );
    assert_eq!(
        std::fs::read(tmp.path().join("backup")).unwrap(),
        b"original state\n"
    );
    assert!(!tmp.path().join("output").exists());
    std::fs::write(tmp.path().join("backup"), b"manual edit").unwrap();
    TEST_HOOK.with(|h| *h.borrow_mut() = None);
    let report = scope.recover_operation(&guard).unwrap().unwrap();
    assert_eq!(report.outcome, ExecutionOutcome::Reconcile { index: 0 });
    assert_eq!(
        (report.recorded_prefix, report.verified_post_prefix),
        (1, 0)
    );
    assert!(!tmp.path().join("output").exists());
    assert_eq!(
        std::fs::read(tmp.path().join("state")).unwrap(),
        b"original state\n"
    );
    assert_eq!(
        std::fs::read(tmp.path().join("backup")).unwrap(),
        b"manual edit"
    );
}

#[test]
fn corrupt_backup_journal_policy_rejects_before_effects() {
    for change in [
        "omit-policy",
        "move-policy",
        "downgrade",
        "replace",
        "mode",
        "bytes",
        "original-digest",
        "original-length",
    ] {
        let (tmp, scope, request, backup) = fixture();
        let _reset = ResetHook;
        TEST_HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(|label, _| {
                anyhow::ensure!(label != "after_initial_journal", "interruption");
                Ok(())
            }))
        });
        let guard = scope.try_lock_writer().unwrap();
        assert!(
            scope
                .apply_operation_with_backup(&guard, &[], request, backup)
                .is_err()
        );
        TEST_HOOK.with(|h| *h.borrow_mut() = None);
        let path = tmp.path().join(".upskill/state/operation.json");
        let mut disk: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        match change {
            "omit-policy" => {
                disk["actions"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("create_only");
            }
            "move-policy" => disk["actions"][1]["create_only"] = true.into(),
            "downgrade" => disk["schema"] = 1.into(),
            "replace" => {
                disk["actions"][0]["before"] = disk["actions"][2]["after"].clone();
            }
            "mode" => disk["actions"][0]["after"]["mode"] = 0o644.into(),
            "bytes" => disk["actions"][0]["after"]["digest"] = "f".repeat(64).into(),
            "original-digest" => disk["actions"][2]["before"]["digest"] = "f".repeat(64).into(),
            "original-length" => disk["actions"][2]["before"]["length"] = 99.into(),
            _ => unreachable!(),
        }
        std::fs::write(path, serde_json::to_vec(&disk).unwrap()).unwrap();
        let error = scope
            .recover_operation(&guard)
            .expect_err(&format!("accepted {change}"));
        if change.starts_with("original-") {
            assert!(format!("{error:#}").contains("backup evidence differs from original state"));
        }
        assert!(!tmp.path().join("backup").exists());
        assert!(!tmp.path().join("output").exists());
        assert_eq!(
            std::fs::read(tmp.path().join("state")).unwrap(),
            b"original state\n"
        );
    }
}

#[test]
fn existing_backup_is_synced_before_first_output_without_replacement() {
    use std::cell::RefCell;
    use std::rc::Rc;
    for fail_at in ["file", "directory"] {
        let (tmp, scope, request, backup) = fixture();
        let path = tmp.path().join("backup");
        std::fs::write(&path, b"original state\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let before = std::fs::metadata(&path).unwrap();
        let backup_id = (before.dev(), before.ino());
        let root = std::fs::metadata(tmp.path()).unwrap();
        let root_id = (root.dev(), root.ino());
        let calls = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&calls);
        let _reset = ResetHook;
        backend::TEST_SYNC.with(|h| {
            *h.borrow_mut() = Some(Box::new(move |file, kind| {
                let metadata = file.metadata()?;
                let expected = if kind == "file" { backup_id } else { root_id };
                assert_eq!((metadata.dev(), metadata.ino()), expected);
                seen.borrow_mut().push(kind);
                anyhow::ensure!(kind != fail_at, "injected backup {kind} sync failure");
                file.sync_all()?;
                Ok(())
            }))
        });
        let guard = scope.try_lock_writer().unwrap();
        let error = scope
            .apply_operation_with_backup(&guard, &[], request, backup)
            .unwrap_err();
        assert!(format!("{error:#}").contains(&format!("injected backup {fail_at} sync failure")));
        let expected = if fail_at == "file" {
            vec!["file"]
        } else {
            vec!["file", "directory"]
        };
        assert_eq!(*calls.borrow(), expected);
        assert!(!tmp.path().join("output").exists());
        assert_eq!(
            std::fs::read(tmp.path().join("state")).unwrap(),
            b"original state\n"
        );
        let disk: serde_json::Value = serde_json::from_slice(
            &std::fs::read(tmp.path().join(".upskill/state/operation.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(disk["actions"][0]["receipt"], "queued");
        assert_eq!(disk["generation"], 0);
        backend::TEST_SYNC.with(|h| *h.borrow_mut() = None);
        assert_eq!(
            scope.recover_operation(&guard).unwrap().unwrap().outcome,
            ExecutionOutcome::Complete
        );
        let after = std::fs::metadata(&path).unwrap();
        assert_eq!(backup_id, (after.dev(), after.ino()));
    }
}
