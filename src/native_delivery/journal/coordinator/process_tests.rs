//! Exit at each observed boundary; a separate process must resume without replay.
use super::*;
use crate::filesystem::QualifiedScope;
use crate::native_delivery::roots::{NativeRoot, lock_native_roots};
use crate::native_delivery::test_support::{fake, fixture};
use crate::ownership::OwnershipScope;
use std::path::{Path, PathBuf};

fn roots(base: &Path) -> Vec<InstallationRoot> {
    [
        ("project", OwnershipScope::Project),
        ("home", OwnershipScope::User),
        ("extra", OwnershipScope::Project),
    ]
    .into_iter()
    .map(|(name, scope)| InstallationRoot {
        scope,
        path: base.join(name),
    })
    .collect()
}
fn setup() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for name in ["project", "home", "extra", "control", "fake-home"] {
        std::fs::create_dir(tmp.path().join(name)).unwrap();
    }
    tmp
}
fn with_store(base: &Path, run: impl FnOnce(&NativeOperationStore<'_, '_>)) {
    let records = roots(base);
    let scopes = records
        .iter()
        .map(|root| QualifiedScope::open(root).unwrap())
        .collect::<Vec<_>>();
    let entries = records
        .iter()
        .zip(&scopes)
        .map(|(root, scope)| NativeRoot {
            root: root.clone(),
            scope,
        })
        .collect::<Vec<_>>();
    let locks = lock_native_roots(&entries).unwrap();
    let store = NativeOperationStore::initialize(&locks, &records[0], &records[1]).unwrap();
    run(&store);
}
fn calls(base: &Path) -> usize {
    std::fs::read_to_string(base.join("control/native.calls"))
        .ok()
        .map(|value| value.parse().unwrap())
        .unwrap_or(0)
}
fn spawn(
    base: &Path,
    mode: &str,
    target: &str,
    index: usize,
    recovery: &str,
) -> std::process::Output {
    std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "native_delivery::journal::coordinator::process_tests::probe",
            "--nocapture",
        ])
        .env("UPSKILL_NATIVE_COORD_ROOT", base)
        .env("UPSKILL_NATIVE_COORD_MODE", mode)
        .env("UPSKILL_NATIVE_COORD_TARGET", target)
        .env("UPSKILL_NATIVE_COORD_INDEX", index.to_string())
        .env("UPSKILL_NATIVE_COORD_RECOVERY", recovery)
        .env("HOME", base.join("fake-home"))
        .env("USERPROFILE", base.join("fake-home"))
        .output()
        .unwrap()
}
fn check_exit(output: std::process::Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
#[ignore]
fn probe() {
    let Some(base) = std::env::var_os("UPSKILL_NATIVE_COORD_ROOT") else {
        return;
    };
    let base = PathBuf::from(base);
    let mode = std::env::var("UPSKILL_NATIVE_COORD_MODE").unwrap();
    let target = std::env::var("UPSKILL_NATIVE_COORD_TARGET").unwrap();
    let target_index: usize = std::env::var("UPSKILL_NATIVE_COORD_INDEX")
        .unwrap()
        .parse()
        .unwrap();
    let recovery = std::env::var("UPSKILL_NATIVE_COORD_RECOVERY").unwrap();
    let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::<(String, usize)>::new()));
    let observed = events.clone();
    let control = base.join("control");
    super::super::TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |label, index, id| {
            observed.borrow_mut().push((label.into(), index));
            if label == target && index == target_index {
                std::fs::write(control.join("id"), id)?;
                std::process::exit(73);
            }
            Ok(())
        }))
    });
    with_store(&base, |store| {
        let mut adapter = fake::Fake {
            backing: Some(base.join("control/native.json")),
            calls: calls(&base),
            timeout: mode == "restore",
            ..Default::default()
        };
        let outcome = if recovery.is_empty() {
            let outcome = NativeDeliveryCoordinator::apply(
                store,
                &mut adapter,
                fixture::capture(store.context()),
            )
            .unwrap();
            if mode == "restore" {
                let NativeDeliveryOutcome::RecoveryRequired {
                    operation_id: Some(id),
                    ..
                } = outcome
                else {
                    panic!("forward timeout completed")
                };
                NativeDeliveryCoordinator::recover(
                    store,
                    &mut adapter,
                    &id,
                    RecoveryDisposition::Restore,
                )
                .unwrap()
            } else {
                outcome
            }
        } else {
            let id = std::fs::read_to_string(base.join("control/id")).unwrap();
            let disposition = match recovery.as_str() {
                "observe" => RecoveryDisposition::ObserveOnly,
                "forward" => RecoveryDisposition::Forward,
                "restore" => RecoveryDisposition::Restore,
                _ => panic!("invalid disposition"),
            };
            NativeDeliveryCoordinator::recover(store, &mut adapter, &id, disposition).unwrap()
        };
        let (kind, id) = match &outcome {
            NativeDeliveryOutcome::Complete { operation_id } => ("complete", Some(operation_id)),
            NativeDeliveryOutcome::Restored { operation_id } => ("restored", Some(operation_id)),
            NativeDeliveryOutcome::RecoveryRequired { operation_id, .. } => {
                ("recovery-required", operation_id.as_ref())
            }
        };
        if let Some(id) = id {
            std::fs::write(base.join("control/id"), id).unwrap();
        }
        std::fs::write(base.join("control/outcome"), kind).unwrap();
        std::fs::write(base.join("control/detail"), format!("{outcome:?}")).unwrap();
    });
    std::fs::write(
        base.join("control/events"),
        serde_json::to_vec(&*events.borrow()).unwrap(),
    )
    .unwrap();
}
fn audit_pending(base: &Path, label: &str, index: usize) {
    let id = std::fs::read_to_string(base.join("control/id")).unwrap();
    with_store(base, |store| {
        let inspection = store.inspect(&id).unwrap();
        let journal = &inspection.journal;
        match label {
            "after_pending" | "after_file_effect" => {
                assert_eq!(journal.checkpoints[index].phase, NativeStepPhase::Pending)
            }
            "after_observation" => assert_eq!(
                journal.checkpoints[index].phase,
                NativeStepPhase::ObservedAfter
            ),
            "after_call_pending" | "after_native_effect" => {
                let call = journal.runtime.as_ref().unwrap().calls.last().unwrap();
                assert_eq!(call.index, index);
                assert_eq!(call.direction, NativeCallDirection::Forward);
                assert_eq!(call.outcome, None);
                assert_eq!(call.precondition.effect, NativeEffectRelation::Before);
                assert_eq!(
                    call.retention_capture.key,
                    inspection.intent.plan.native_changes[0].key
                );
                assert_eq!(calls(base), usize::from(label == "after_native_effect"));
            }
            "after_attempt" => {
                assert_eq!(
                    journal.checkpoints[index].attempts.last().unwrap().outcome,
                    NativeBindingOutcome::Confirmed
                );
                assert_eq!(
                    journal
                        .runtime
                        .as_ref()
                        .unwrap()
                        .calls
                        .last()
                        .unwrap()
                        .outcome,
                    Some(NativeBindingOutcome::Confirmed)
                );
            }
            "after_state_pending" | "after_state_effect" => {
                assert_eq!(
                    journal.runtime.as_ref().unwrap().state_phase,
                    NativeStepPhase::Pending
                );
                assert_eq!(
                    base.join("project/.upskill-lock.json").exists(),
                    label == "after_state_effect"
                );
            }
            "after_terminal_pending" | "after_retired_marker" => {
                assert_eq!(journal.status, NativeJournalStatus::Retiring);
                assert_eq!(
                    journal.runtime.as_ref().unwrap().state_phase,
                    NativeStepPhase::ObservedAfter
                );
            }
            "after_inverse_call_pending" | "after_inverse_native" => {
                let call = journal.runtime.as_ref().unwrap().calls.last().unwrap();
                assert_eq!(call.direction, NativeCallDirection::Restore);
                assert_eq!(call.outcome, None);
                assert_eq!(call.precondition.effect, NativeEffectRelation::After);
                assert_eq!(
                    call.retention_capture.key,
                    inspection.intent.plan.native_changes[0].key
                );
                assert_eq!(
                    calls(base),
                    if label == "after_inverse_native" {
                        2
                    } else {
                        1
                    }
                );
            }
            "after_inverse_pending" => assert_eq!(
                journal
                    .runtime
                    .as_ref()
                    .unwrap()
                    .restore
                    .as_ref()
                    .unwrap()
                    .checkpoints[index]
                    .phase,
                NativeStepPhase::Pending
            ),
            "after_inverse_observation" => assert_eq!(
                journal
                    .runtime
                    .as_ref()
                    .unwrap()
                    .restore
                    .as_ref()
                    .unwrap()
                    .checkpoints[index]
                    .phase,
                NativeStepPhase::ObservedAfter
            ),
            "after_restore_state_pending" | "after_restore_state_effect" => {
                assert_eq!(
                    journal
                        .runtime
                        .as_ref()
                        .unwrap()
                        .restore
                        .as_ref()
                        .unwrap()
                        .state_phase,
                    NativeStepPhase::Pending
                );
                assert_eq!(
                    base.join("project/.upskill-lock.json").exists(),
                    label == "after_restore_state_effect"
                );
            }
            "after_restore_terminal_pending" | "after_restore_retired_marker" => {
                assert_eq!(journal.status, NativeJournalStatus::RetiringRestore)
            }
            _ => {}
        }
    });
}
#[test]
fn every_forward_and_inverse_boundary_recovers_in_fresh_processes_without_duplicate_calls() {
    for mode in ["forward", "restore"] {
        let baseline = setup();
        check_exit(spawn(baseline.path(), mode, "", 0, ""), 0);
        assert_eq!(
            std::fs::read_to_string(baseline.path().join("control/outcome")).unwrap(),
            if mode == "forward" {
                "complete"
            } else {
                "restored"
            }
        );
        let trace: Vec<(String, usize)> =
            serde_json::from_slice(&std::fs::read(baseline.path().join("control/events")).unwrap())
                .unwrap();
        let mut events = trace
            .into_iter()
            .filter(|(label, _)| {
                mode == "forward"
                    || label.starts_with("after_inverse")
                    || label.starts_with("after_restore")
            })
            .collect::<Vec<_>>();
        events.sort();
        events.dedup();
        assert!(events.len() >= if mode == "forward" { 20 } else { 10 });
        for (label, index) in events {
            let tmp = setup();
            check_exit(spawn(tmp.path(), mode, &label, index, ""), 73);
            audit_pending(tmp.path(), &label, index);
            let before = calls(tmp.path());
            check_exit(spawn(tmp.path(), mode, "", 0, "observe"), 0);
            assert_eq!(
                calls(tmp.path()),
                before,
                "ObserveOnly replayed a command at {mode}/{label}/{index}"
            );
            if std::fs::read_to_string(tmp.path().join("control/outcome")).unwrap()
                == "recovery-required"
            {
                check_exit(
                    spawn(
                        tmp.path(),
                        mode,
                        "",
                        0,
                        if mode == "forward" {
                            "forward"
                        } else {
                            "restore"
                        },
                    ),
                    0,
                );
            }
            assert_eq!(
                std::fs::read_to_string(tmp.path().join("control/outcome")).unwrap(),
                if mode == "forward" {
                    "complete"
                } else {
                    "restored"
                },
                "{mode}/{label}/{index}: {}",
                std::fs::read_to_string(tmp.path().join("control/detail")).unwrap()
            );
            assert_eq!(
                calls(tmp.path()),
                if mode == "forward" { 1 } else { 2 },
                "duplicate native call at {mode}/{label}/{index}"
            );
            let id = std::fs::read_to_string(tmp.path().join("control/id")).unwrap();
            with_store(tmp.path(), |store| {
                assert_eq!(
                    store.inspect(&id).unwrap().journal.status,
                    if mode == "forward" {
                        NativeJournalStatus::Complete
                    } else {
                        NativeJournalStatus::Restored
                    }
                );
                for root in roots(tmp.path()) {
                    assert!(
                        !root
                            .path
                            .join(".upskill/native-pending/pending.json")
                            .exists()
                    );
                }
                let state = store.state.load().unwrap().unwrap();
                if mode == "restore" {
                    assert!(state.skills.receipts.is_empty());
                    assert_eq!(state.skills.staged.len(), 1);
                    assert!(state.skills.graph.installations.is_empty());
                }
            });
        }
    }
}

#[test]
fn invalid_runtime_checkpoint_cannot_replace_a_recoverable_journal() {
    let tmp = setup();
    with_store(tmp.path(), |store| {
        let prepared = store.prepare(&fixture::capture(store.context())).unwrap();
        let mut adapter = fake::Fake::default();
        store
            .prepare_execution(prepared.id(), &mut adapter)
            .unwrap();
        initialize_runtime(store, prepared.id()).unwrap();
        let inspection = store.inspect(prepared.id()).unwrap();
        let path = store.context().private_root.path.join(format!(
            ".upskill/native-evidence/journal-{}.json",
            prepared.id()
        ));
        let before = std::fs::read(&path).unwrap();
        let mut invalid = inspection.journal.clone();
        invalid.status = NativeJournalStatus::Complete;
        assert!(persist(store, &inspection.journal, invalid).is_err());
        assert!(
            std::fs::read(&path).unwrap() == before,
            "invalid checkpoint replaced recovery evidence"
        );
        assert_eq!(
            store.inspect(prepared.id()).unwrap().journal,
            inspection.journal
        );
    });
}
