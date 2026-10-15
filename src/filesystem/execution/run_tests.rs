use super::*;
use crate::files::plan::{FileLocation, FileValue, PlannedFile};
use crate::ownership::{InstallationRoot, OwnershipScope};
use std::os::unix::fs::PermissionsExt;
#[test]
fn execution_types_are_exported_at_filesystem_boundary() {
    let _: Option<crate::filesystem::ExecutionRequest> = None;
    let _: Option<crate::filesystem::ExecutionReport> = None;
    let _ = crate::filesystem::ExecutionOutcome::Complete;
}
fn fixture() -> (tempfile::TempDir, QualifiedScope, ExecutionRequest) {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let action = |name: &str| PlannedFile {
        location: FileLocation {
            root: root.clone(),
            path: name.into(),
        },
        before: None,
        after: Some(FileValue {
            bytes: name.as_bytes().to_vec(),
            mode: Some(0o600),
        }),
    };
    (
        tmp,
        scope,
        ExecutionRequest {
            files: vec![action("a"), action("b")],
            state: action("state"),
        },
    )
}
#[test]
fn applies_complete_operation_and_final_state() {
    let (tmp, scope, request) = fixture();
    let guard = scope.try_lock_writer().unwrap();
    let report = scope.apply_operation(&guard, &[], request).unwrap();
    assert_eq!(report.outcome, ExecutionOutcome::Complete);
    assert_eq!(report.recorded_prefix, 3);
    assert_eq!(report.verified_post_prefix, 3);
    for path in ["a", "b", "state"] {
        assert_eq!(
            std::fs::read(tmp.path().join(path)).unwrap(),
            path.as_bytes()
        );
    }
    assert!(scope.recover_operation(&guard).unwrap().is_none());
}
#[test]
fn all_preconditions_checked_before_first_output() {
    let (tmp, scope, request) = fixture();
    let guard = scope.try_lock_writer().unwrap();
    std::fs::write(tmp.path().join("b"), b"manual").unwrap();
    std::fs::set_permissions(tmp.path().join("b"), std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(scope.apply_operation(&guard, &[], request).is_err());
    assert!(!tmp.path().join("a").exists());
    assert!(!tmp.path().join(".upskill").exists());
}
#[test]
fn absent_destinations_qualify_existing_parent_metadata_before_outputs() {
    for target in ["nested/b", "nested/missing/b"] {
        let (tmp, scope, mut request) = fixture();
        request.files[1].location.path = target.into();
        let path = tmp.path().join("nested");
        std::fs::create_dir(&path).unwrap();
        let dir = std::fs::File::open(&path).unwrap();
        #[cfg(target_os = "linux")]
        let name = "user.manual";
        #[cfg(target_os = "macos")]
        let name = "com.example.manual";
        rustix::fs::fsetxattr(&dir, name, b"manual", rustix::fs::XattrFlags::empty()).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        assert!(scope.apply_operation(&guard, &[], request).is_err());
        assert!(
            !tmp.path().join("a").exists(),
            "first output was applied before parent qualification"
        );
        assert!(
            !tmp.path().join(".upskill").exists(),
            "intent was published before parent qualification"
        );
        assert!(!path.join("missing").exists());
    }
}
#[test]
fn foreign_guard_rejects() {
    let (tmp, scope, request) = fixture();
    let (_other, foreign, _) = fixture();
    let guard = foreign.try_lock_writer().unwrap();
    assert!(scope.apply_operation(&guard, &[], request).is_err());
    assert!(!tmp.path().join("a").exists());
}
#[test]
fn source_changed_after_plan_rejects() {
    let (tmp, scope, request) = fixture();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("body"), b"old").unwrap();
    let source_root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: source.path().into(),
    };
    let source_scope = QualifiedScope::open(&source_root).unwrap();
    let snapshot = source_scope.observe_source_tree().unwrap();
    std::fs::write(source.path().join("body"), b"new").unwrap();
    let guard = scope.try_lock_writer().unwrap();
    assert!(
        scope
            .apply_operation(&guard, &[&snapshot], request)
            .is_err()
    );
    assert!(!tmp.path().join(".upskill").exists());
}
#[test]
fn state_only_and_noop_operations_complete() {
    let (tmp, scope, mut request) = fixture();
    request.files.clear();
    let guard = scope.try_lock_writer().unwrap();
    scope.apply_operation(&guard, &[], request.clone()).unwrap();
    request.state.before = request.state.after.clone();
    assert_eq!(
        scope
            .apply_operation(&guard, &[], request)
            .unwrap()
            .recorded_prefix,
        1
    );
    assert_eq!(std::fs::read(tmp.path().join("state")).unwrap(), b"state");
}
#[test]
fn observation_sorting_does_not_change_action_order() {
    let (tmp, scope, mut request) = fixture();
    request.files[0].location.path = "z".into();
    request.state.location.path = ".upskill-lock.json".into();
    let guard = scope.try_lock_writer().unwrap();
    assert_eq!(
        scope.apply_operation(&guard, &[], request).unwrap().outcome,
        ExecutionOutcome::Complete
    );
    assert_eq!(std::fs::read(tmp.path().join("z")).unwrap(), b"a");
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        b"state"
    );
}
#[test]
fn later_edit_preserves_applied_prefix() {
    let (tmp, scope, request) = fixture();
    let path = tmp.path().join("b");
    super::super::TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |point, index| {
            if point == "after_done" && index == 0 {
                std::fs::write(&path, b"manual")?;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            }
            Ok(())
        }))
    });
    let guard = scope.try_lock_writer().unwrap();
    let report = scope.apply_operation(&guard, &[], request).unwrap();
    super::super::TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
    assert_eq!(report.outcome, ExecutionOutcome::Reconcile { index: 1 });
    assert_eq!(report.recorded_prefix, 1);
    assert_eq!(report.verified_post_prefix, 1);
    assert_eq!(std::fs::read(tmp.path().join("a")).unwrap(), b"a");
    assert_eq!(std::fs::read(tmp.path().join("b")).unwrap(), b"manual");
    assert!(!tmp.path().join("state").exists());
}
#[test]
fn later_alias_error_reports_durable_prefix_and_preserves_alias() {
    let (tmp, scope, request) = fixture();
    let path = tmp.path().join("b");
    super::super::TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |point, index| {
            if point == "after_done" && index == 0 {
                std::os::unix::fs::symlink("a", &path)?;
            }
            Ok(())
        }))
    });
    let guard = scope.try_lock_writer().unwrap();
    let error = scope.apply_operation(&guard, &[], request).unwrap_err();
    super::super::TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
    let message = format!("{error:#}");
    assert!(message.contains("recorded prefix 1"), "{message}");
    assert!(
        message.contains("verified post-state prefix 0"),
        "{message}"
    );
    assert_eq!(
        std::fs::read_link(tmp.path().join("b")).unwrap(),
        std::path::Path::new("a")
    );
    assert_eq!(std::fs::read(tmp.path().join("a")).unwrap(), b"a");
    assert!(!tmp.path().join("state").exists());
}
#[test]
fn queued_poststate_requires_reconciliation() {
    let (tmp, scope, request) = fixture();
    let guard = scope.try_lock_writer().unwrap();
    let _j = super::super::journal::Journal::create(&scope, &request).unwrap();
    super::super::backend::apply(&scope, &request.files[0], 0).unwrap();
    let report = scope.recover_operation(&guard).unwrap().unwrap();
    assert_eq!(report.outcome, ExecutionOutcome::Reconcile { index: 0 });
    assert_eq!(report.recorded_prefix, 0);
    assert_eq!(std::fs::read(tmp.path().join("a")).unwrap(), b"a");
    assert!(!tmp.path().join("b").exists());
}
#[test]
fn failed_effect_sync_is_resynchronized_before_receipt() {
    let (tmp, scope, request) = fixture();
    let guard = scope.try_lock_writer().unwrap();
    super::super::TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(|point, index| {
            if point == "after_effect" && index == 0 {
                anyhow::bail!("injected failure before directory sync");
            }
            Ok(())
        }))
    });
    assert!(scope.apply_operation(&guard, &[], request).is_err());
    let inode =
        std::os::unix::fs::MetadataExt::ino(&std::fs::metadata(tmp.path().join("a")).unwrap());
    let synced = std::rc::Rc::new(std::cell::Cell::new(false));
    let observed = synced.clone();
    super::super::TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |point, index| {
            if point == "after_ack_sync" && index == 0 {
                observed.set(true);
            }
            Ok(())
        }))
    });
    let report = scope.recover_operation(&guard).unwrap().unwrap();
    super::super::TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
    assert!(
        synced.get(),
        "observed post-state was acknowledged without syncing its file and directory"
    );
    assert_eq!(report.outcome, ExecutionOutcome::Complete);
    assert_eq!(
        std::os::unix::fs::MetadataExt::ino(&std::fs::metadata(tmp.path().join("a")).unwrap()),
        inode
    );
}
#[test]
fn readonly_recovery_and_unsafe_root_create_no_storage() {
    let (tmp, scope, request) = fixture();
    let guard = scope.try_lock_writer().unwrap();
    assert!(scope.recover_operation(&guard).unwrap().is_none());
    assert!(!tmp.path().join(".upskill").exists());
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(scope.apply_operation(&guard, &[], request).is_err());
    assert!(!tmp.path().join(".upskill").exists());
    assert!(!tmp.path().join("a").exists());
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn recovery_requires_matching_guard() {
    let (tmp, scope, request) = fixture();
    super::super::journal::Journal::create(&scope, &request).unwrap();
    let (_other, foreign, _) = fixture();
    let guard = foreign.try_lock_writer().unwrap();
    assert!(scope.recover_operation(&guard).is_err());
    assert!(!tmp.path().join("a").exists());
    assert!(tmp.path().join(".upskill/state/operation.json").exists());
}
#[test]
fn mode_only_edit_requires_reconciliation_without_repair() {
    let (tmp, scope, request) = fixture();
    let guard = scope.try_lock_writer().unwrap();
    super::super::TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(|point, index| {
            if point == "after_done" && index == 0 {
                anyhow::bail!("stop after first receipt");
            }
            Ok(())
        }))
    });
    assert!(scope.apply_operation(&guard, &[], request).is_err());
    super::super::TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
    std::fs::set_permissions(tmp.path().join("a"), std::fs::Permissions::from_mode(0o644)).unwrap();
    let report = scope.recover_operation(&guard).unwrap().unwrap();
    assert_eq!(report.outcome, ExecutionOutcome::Reconcile { index: 0 });
    assert_eq!(report.recorded_prefix, 1);
    assert_eq!(report.verified_post_prefix, 0);
    assert_eq!(
        std::os::unix::fs::MetadataExt::mode(&std::fs::metadata(tmp.path().join("a")).unwrap())
            & 0o7777,
        0o644
    );
    assert_eq!(std::fs::read(tmp.path().join("a")).unwrap(), b"a");
    assert!(!tmp.path().join("b").exists());
}
#[test]
fn recovery_sync_failures_do_not_publish_done() {
    use std::os::unix::fs::MetadataExt;
    for deletion in [false, true] {
        for failure in [Some("file"), Some("directory"), None] {
            if deletion && failure == Some("file") {
                continue;
            }
            let (tmp, scope, mut request) = fixture();
            request.files[0].location.path = "nested/a".into();
            let parent = tmp.path().join("nested");
            std::fs::create_dir(&parent).unwrap();
            let path = parent.join("a");
            if deletion {
                std::fs::write(&path, b"old").unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
                request.files[0].before = Some(FileValue {
                    bytes: b"old".to_vec(),
                    mode: Some(0o600),
                });
                request.files[0].after = None;
            }
            let guard = scope.try_lock_writer().unwrap();
            let mut journal = super::super::journal::Journal::create(&scope, &request).unwrap();
            journal
                .record(0, super::super::journal::Receipt::Pending)
                .unwrap();
            super::super::backend::apply(&scope, &request.files[0], 0).unwrap();
            let identity = |path: &std::path::Path| {
                let m = std::fs::metadata(path).unwrap();
                (m.dev(), m.ino())
            };
            let parent_identity = identity(&parent);
            assert_ne!(parent_identity, identity(tmp.path()));
            let file_identity = if deletion {
                None
            } else {
                Some(identity(&path))
            };
            let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let validating = std::rc::Rc::new(std::cell::Cell::new(true));
            let active = validating.clone();
            let recorded = calls.clone();
            super::super::backend::TEST_SYNC.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |file, kind| {
                    if active.get() {
                        let metadata = file.metadata()?;
                        let expected = match kind {
                            "file" => file_identity.expect("deletion must not sync a file"),
                            "directory" => parent_identity,
                            _ => panic!("unexpected sync kind"),
                        };
                        assert_eq!(
                            (metadata.dev(), metadata.ino()),
                            expected,
                            "wrong {kind} sync descriptor"
                        );
                        recorded.borrow_mut().push(kind);
                        if failure == Some(kind) {
                            anyhow::bail!("injected {kind} sync failure");
                        }
                    }
                    file.sync_all()?;
                    Ok(())
                }))
            });
            let ordered = calls.clone();
            let expected_calls = if deletion {
                vec!["directory"]
            } else {
                vec!["file", "directory"]
            };
            let expected = expected_calls.clone();
            super::super::TEST_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |point, index| {
                    if point == "before_done" && index == 0 {
                        assert_eq!(*ordered.borrow(), expected);
                        validating.set(false);
                    }
                    Ok(())
                }))
            });
            let result = scope.recover_operation(&guard);
            super::super::backend::TEST_SYNC.with(|hook| *hook.borrow_mut() = None);
            super::super::TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
            if failure.is_some() {
                assert!(result.is_err());
                assert_eq!(
                    super::super::journal::Journal::load(&scope)
                        .unwrap()
                        .unwrap()
                        .disk
                        .actions[0]
                        .receipt,
                    super::super::journal::Receipt::Pending
                );
                assert_eq!(
                    *calls.borrow(),
                    if failure == Some("file") {
                        vec!["file"]
                    } else {
                        expected_calls
                    }
                );
            } else {
                assert_eq!(result.unwrap().unwrap().outcome, ExecutionOutcome::Complete);
                assert_eq!(*calls.borrow(), expected_calls);
            }
            assert_eq!(path.exists(), !deletion);
        }
    }
}
#[test]
fn recovery_qualifies_metadata_for_every_receipt_before_progress() {
    for phase in [
        "later-queued",
        "done",
        "queued-both",
        "pending-after",
        "all-done",
    ] {
        let (tmp, scope, mut request) = fixture();
        for action in &mut request.files {
            let path = tmp.path().join(&action.location.path);
            std::fs::write(&path, b"old").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
            action.before = Some(FileValue {
                bytes: b"old".to_vec(),
                mode: Some(0o600),
            });
        }
        if phase == "queued-both" {
            request.files[0].after = request.files[0].before.clone();
        }
        let guard = scope.try_lock_writer().unwrap();
        let mut journal = super::super::journal::Journal::create(&scope, &request).unwrap();
        if phase == "all-done" {
            for (index, action) in request
                .files
                .iter()
                .chain(std::iter::once(&request.state))
                .enumerate()
            {
                journal
                    .record(index, super::super::journal::Receipt::Pending)
                    .unwrap();
                super::super::backend::apply(&scope, action, index).unwrap();
                journal
                    .record(index, super::super::journal::Receipt::Done)
                    .unwrap();
            }
        } else {
            if phase != "queued-both" {
                journal
                    .record(0, super::super::journal::Receipt::Pending)
                    .unwrap();
            }
            if phase == "done" || phase == "pending-after" {
                super::super::backend::apply(&scope, &request.files[0], 0).unwrap();
            }
            if phase == "done" {
                journal
                    .record(0, super::super::journal::Receipt::Done)
                    .unwrap();
            }
        }
        let target = tmp
            .path()
            .join(if phase == "later-queued" { "b" } else { "a" });
        let file = std::fs::File::open(&target).unwrap();
        #[cfg(target_os = "linux")]
        let name = "user.upskill-recovery";
        #[cfg(target_os = "macos")]
        let name = "upskill-recovery";
        rustix::fs::fsetxattr(&file, name, b"manual", rustix::fs::XattrFlags::empty()).unwrap();
        let before_a = std::fs::read(tmp.path().join("a")).unwrap();
        let before_b = std::fs::read(tmp.path().join("b")).unwrap();
        let before_state = std::fs::read(tmp.path().join("state")).ok();
        let before_journal =
            std::fs::read(tmp.path().join(".upskill/state/operation.json")).unwrap();
        assert!(scope.recover_operation(&guard).is_err(), "{phase}");
        assert_eq!(
            std::fs::read(tmp.path().join("a")).unwrap(),
            before_a,
            "{phase}: earlier output changed"
        );
        assert_eq!(std::fs::read(tmp.path().join("b")).unwrap(), before_b);
        assert_eq!(
            std::fs::read(tmp.path().join(".upskill/state/operation.json")).unwrap(),
            before_journal,
            "{phase}: receipt advanced"
        );
        let mut value = [0u8; 64];
        let length = rustix::fs::fgetxattr(&file, name, &mut value[..]).unwrap();
        assert_eq!(&value[..length], b"manual");
        assert_eq!(std::fs::read(tmp.path().join("state")).ok(), before_state);
        for action in &journal.disk.actions {
            if let Some(payload) = &action.payload {
                assert!(
                    tmp.path()
                        .join(".upskill/state")
                        .join(&payload.name)
                        .is_file(),
                    "{phase}: payload removed"
                );
            }
        }
    }
}
#[test]
fn journal_publication_sync_precedes_output_and_receipt_progress() {
    use std::os::unix::fs::MetadataExt;
    for failure in [Some(0usize), Some(1), Some(2), None] {
        let (tmp, scope, request) = fixture();
        let guard = scope.try_lock_writer().unwrap();
        let store = tmp.path().join(".upskill/state");
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let observed = events.clone();
        super::super::TEST_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |point, index| {
                match point {
                    "after_journal_rename" => observed.borrow_mut().push(("rename", index)),
                    "after_journal_sync" => {
                        assert_eq!(observed.borrow().last(), Some(&("sync", index)))
                    }
                    "before_effect" => {
                        assert_eq!(observed.borrow().last(), Some(&("sync", 2 * index + 1)))
                    }
                    _ => {}
                }
                Ok(())
            }))
        });
        let synchronized = events.clone();
        super::super::journal::TEST_PUBLICATION_SYNC.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |file, generation| {
                let expected = std::fs::metadata(&store)?;
                let actual = file.metadata()?;
                assert_eq!(
                    (actual.dev(), actual.ino()),
                    (expected.dev(), expected.ino())
                );
                assert_eq!(synchronized.borrow().last(), Some(&("rename", generation)));
                synchronized.borrow_mut().push(("sync", generation));
                if failure == Some(generation) {
                    anyhow::bail!("injected publication directory sync failure");
                }
                file.sync_all()?;
                Ok(())
            }))
        });
        let result = scope.apply_operation(&guard, &[], request);
        super::super::journal::TEST_PUBLICATION_SYNC.with(|hook| *hook.borrow_mut() = None);
        super::super::TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
        if let Some(generation) = failure {
            assert!(result.is_err());
            assert_eq!(tmp.path().join("a").exists(), generation == 2);
            assert!(!tmp.path().join("b").exists());
            assert!(!tmp.path().join("state").exists());
            let journal = super::super::journal::Journal::load(&scope)
                .unwrap()
                .unwrap();
            assert_eq!(journal.disk.generation, generation as u64);
            assert_eq!(
                journal.disk.actions[0].receipt,
                match generation {
                    0 => super::super::journal::Receipt::Queued,
                    1 => super::super::journal::Receipt::Pending,
                    _ => super::super::journal::Receipt::Done,
                }
            );
        } else {
            assert_eq!(result.unwrap().outcome, ExecutionOutcome::Complete);
        }
    }
}
