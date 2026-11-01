//! Native coordinator error propagation through the real private/file stores.
use super::backend;
use crate::filesystem::QualifiedScope;
use crate::native_delivery::coordinator::{
    NativeDeliveryCoordinator, NativeDeliveryOutcome, RecoveryDisposition,
};
use crate::native_delivery::journal::{NativeJournalStatus, NativeOperationStore};
use crate::native_delivery::roots::{NativeRoot, NativeRootLocks, lock_native_roots};
use crate::native_delivery::test_support::{fake, fixture};
use crate::ownership::{InstallationRoot, OwnershipScope};
use std::os::unix::fs::FileExt;

struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        backend::TEST_SYNC.with(|hook| *hook.borrow_mut() = None);
        crate::native_delivery::journal::TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
    }
}
fn with_store(
    run: impl FnOnce(&NativeOperationStore<'_, '_>, &InstallationRoot, &NativeRootLocks<'_>),
) {
    let _reset = Reset;
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let primary = InstallationRoot {
        scope: OwnershipScope::Project,
        path: project.path().into(),
    };
    let private = InstallationRoot {
        scope: OwnershipScope::User,
        path: home.path().into(),
    };
    let ps = QualifiedScope::open(&primary).unwrap();
    let hs = QualifiedScope::open(&private).unwrap();
    let locks = lock_native_roots(&[
        NativeRoot {
            root: primary.clone(),
            scope: &ps,
        },
        NativeRoot {
            root: private.clone(),
            scope: &hs,
        },
    ])
    .unwrap();
    let store = NativeOperationStore::initialize(&locks, &primary, &private).unwrap();
    run(&store, &primary, &locks);
}
fn bytes(file: &std::fs::File) -> anyhow::Result<Vec<u8>> {
    let length = usize::try_from(file.metadata()?.len())?;
    let mut bytes = vec![0; length];
    file.read_exact_at(&mut bytes, 0)?;
    Ok(bytes)
}
fn id(outcome: NativeDeliveryOutcome) -> String {
    let NativeDeliveryOutcome::RecoveryRequired {
        operation_id: Some(id),
        ..
    } = outcome
    else {
        panic!("missing recovery outcome: {outcome:?}")
    };
    id
}
#[test]
fn failed_pending_call_sync_never_invokes_a_native_command() {
    with_store(|store, primary, _| {
        backend::TEST_SYNC.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(|file, kind| {
                if kind == "file" {
                    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes(file)?) {
                        if value
                            .get("runtime")
                            .and_then(|runtime| runtime.get("calls"))
                            .and_then(|calls| calls.as_array())
                            .is_some_and(|calls| {
                                calls.last().is_some_and(|call| call["outcome"].is_null())
                            })
                        {
                            anyhow::bail!("injected pending-call sync failure");
                        }
                    }
                }
                file.sync_all()?;
                Ok(())
            }))
        });
        let mut adapter = fake::Fake::default();
        let operation = id(NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap());
        assert_eq!(adapter.calls, 0);
        assert!(!primary.path.join(".upskill-lock.json").exists());
        assert_eq!(
            store
                .inspect(&operation)
                .unwrap()
                .journal
                .runtime
                .unwrap()
                .calls
                .last()
                .unwrap()
                .outcome,
            None
        );
        backend::TEST_SYNC.with(|hook| *hook.borrow_mut() = None);
        assert!(matches!(
            NativeDeliveryCoordinator::recover(
                store,
                &mut adapter,
                &operation,
                RecoveryDisposition::ObserveOnly
            )
            .unwrap(),
            NativeDeliveryOutcome::RecoveryRequired { .. }
        ));
        assert_eq!(adapter.calls, 0);
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &operation,
            RecoveryDisposition::Forward,
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Complete { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls, 1);
    });
}
#[test]
fn failed_new_and_existing_private_blob_sync_cannot_publish_completed_state() {
    with_store(|store, primary, _| {
        backend::TEST_SYNC.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(|file, kind| {
                if kind == "file" && bytes(file)? == b"null" {
                    anyhow::bail!("injected private-prestate sync failure");
                }
                file.sync_all()?;
                Ok(())
            }))
        });
        let mut adapter = fake::Fake::default();
        let operation = id(NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap());
        assert_eq!(adapter.calls, 1);
        assert!(!primary.path.join(".upskill-lock.json").exists());
        // The first write left readable blob bytes. Existing readable content
        // still needs sync acknowledgement on the aggregate state-store route.
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &operation,
            RecoveryDisposition::ObserveOnly,
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::RecoveryRequired { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls, 1);
        assert!(!primary.path.join(".upskill-lock.json").exists());
        backend::TEST_SYNC.with(|hook| *hook.borrow_mut() = None);
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &operation,
            RecoveryDisposition::ObserveOnly,
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Complete { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls, 1);
    });
}
#[test]
fn failed_marker_retirement_sync_keeps_terminal_evidence_and_never_replays() {
    with_store(|store, primary, _| {
        let primary = primary.clone();
        crate::native_delivery::journal::TEST_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |label, _, _| {
                if label == "after_terminal_pending" {
                    use std::os::unix::fs::MetadataExt;
                    let expected = std::fs::metadata(primary.path.join(".upskill/native-pending"))?;
                    backend::TEST_SYNC.with(|sync| {
                        *sync.borrow_mut() = Some(Box::new(move |file, kind| {
                            let actual = file.metadata()?;
                            if kind == "directory"
                                && actual.dev() == expected.dev()
                                && actual.ino() == expected.ino()
                            {
                                anyhow::bail!("injected marker retirement sync failure");
                            }
                            file.sync_all()?;
                            Ok(())
                        }))
                    });
                }
                Ok(())
            }))
        });
        let mut adapter = fake::Fake::default();
        let operation = id(NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap());
        assert_eq!(adapter.calls, 1);
        assert_eq!(
            store.inspect(&operation).unwrap().journal.status,
            NativeJournalStatus::Retiring
        );
        backend::TEST_SYNC.with(|hook| *hook.borrow_mut() = None);
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &operation,
            RecoveryDisposition::ObserveOnly,
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Complete { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls, 1);
    });
}
