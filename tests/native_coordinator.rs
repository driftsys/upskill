//! Real durable coordination with an independently observed fake native binding.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/native_delivery.rs"]
mod fixture;
use upskill::filesystem::QualifiedScope;
use upskill::lifecycle::native_receipt::{NativeBindingOutcome, NativeSkillSelection};
use upskill::native_delivery::adapter::*;
use upskill::native_delivery::coordinator::{
    NativeDeliveryCoordinator, NativeDeliveryOutcome, RecoveryDisposition,
};
use upskill::native_delivery::journal::NativeOperationStore;
use upskill::native_delivery::roots::{NativeRoot, lock_native_roots};
use upskill::ownership::{InstallationRoot, OwnershipScope};

#[path = "common/native_adapter.rs"]
mod fake;
use fake::Fake;
fn with_store(
    run: impl FnOnce(
        &NativeOperationStore<'_, '_>,
        &InstallationRoot,
        &InstallationRoot,
        &upskill::native_delivery::roots::NativeRootLocks<'_>,
    ),
) {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let extra = tempfile::tempdir().unwrap();
    let primary = InstallationRoot {
        scope: OwnershipScope::Project,
        path: project.path().into(),
    };
    let private = InstallationRoot {
        scope: OwnershipScope::User,
        path: home.path().into(),
    };
    let additional = InstallationRoot {
        scope: OwnershipScope::Project,
        path: extra.path().into(),
    };
    let es = QualifiedScope::open(&additional).unwrap();
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
        NativeRoot {
            root: additional,
            scope: &es,
        },
    ])
    .unwrap();
    let store = NativeOperationStore::initialize(&locks, &primary, &private).unwrap();
    run(&store, &primary, &private, &locks);
}

#[test]
fn observed_native_success_commits_state_and_retires_exact_markers() {
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
    let capture = fixture::capture(store.context());
    let mut adapter = Fake::default();
    let result = NativeDeliveryCoordinator::apply(&store, &mut adapter, capture).unwrap();
    assert!(matches!(result, NativeDeliveryOutcome::Complete { .. }));
    assert_eq!(adapter.calls, 1);
    let NativeDeliveryOutcome::Complete { operation_id } = &result else {
        unreachable!()
    };
    let inspection = store.inspect(operation_id).unwrap();
    let call = &inspection.journal.runtime.as_ref().unwrap().calls[0];
    assert_eq!(call.precondition.effect, NativeEffectRelation::Before);
    assert_eq!(call.precondition.operation_id, *operation_id);
    assert_eq!(
        call.retention_capture.key,
        inspection.intent.plan.native_changes[0].key
    );
    assert_eq!(call.retention_capture.scope, primary);
    assert!(matches!(
        call.retention_capture.state,
        upskill::lifecycle::native_receipt::NativeObservationState::Absent
    ));

    assert!(project.path().join(".upskill-lock.json").exists());
    assert!(
        !project
            .path()
            .join(".upskill/native-pending/pending.json")
            .exists()
    );
    assert!(
        !home
            .path()
            .join(".upskill/native-pending/pending.json")
            .exists()
    );
}

#[test]
fn timeout_after_effect_requires_recovery_and_observed_after_does_not_replay_command() {
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
    let capture = fixture::capture(store.context());
    let mut adapter = Fake {
        timeout: true,
        ..Default::default()
    };
    let result = NativeDeliveryCoordinator::apply(&store, &mut adapter, capture).unwrap();
    let NativeDeliveryOutcome::RecoveryRequired {
        operation_id: Some(id),
        ..
    } = result
    else {
        panic!("timeout completed without recovery");
    };
    assert_eq!(adapter.calls, 1);
    assert!(!project.path().join(".upskill-lock.json").exists());
    let result = NativeDeliveryCoordinator::recover(
        &store,
        &mut adapter,
        &id,
        RecoveryDisposition::ObserveOnly,
    )
    .unwrap();
    assert!(matches!(result, NativeDeliveryOutcome::Complete { .. }));
    assert_eq!(adapter.calls, 1);
}

#[test]
fn explicit_restore_observes_native_absence_and_keeps_exports_owned_as_staging() {
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
    let capture = fixture::capture(store.context());
    let revision = capture.candidates[0].clone();
    let mut adapter = Fake {
        timeout: true,
        ..Default::default()
    };
    let NativeDeliveryOutcome::RecoveryRequired {
        operation_id: Some(id),
        ..
    } = NativeDeliveryCoordinator::apply(&store, &mut adapter, capture).unwrap()
    else {
        panic!("timeout completed")
    };
    let result =
        NativeDeliveryCoordinator::recover(&store, &mut adapter, &id, RecoveryDisposition::Restore)
            .unwrap();
    assert!(
        matches!(result, NativeDeliveryOutcome::Restored { .. }),
        "{result:?}"
    );
    assert_eq!(adapter.calls, 2);
    assert!(adapter.selected.is_none());
    let state_store =
        upskill::native_delivery::store::NativeStateStore::open(&locks, &primary, &private)
            .unwrap()
            .unwrap();
    let state = state_store.load().unwrap().unwrap();
    assert!(state.skills.graph.installations.is_empty());
    assert!(state.skills.revisions.is_empty());
    assert!(state.skills.receipts.is_empty());
    assert!(state.intents.is_empty());
    assert_eq!(state.skills.staged.len(), 1);
    assert_eq!(state.skills.staged[0].revision, revision);
    assert_eq!(state.skills.staged[0].operation_id, id);
    assert_eq!(state.files.len(), revision.package.revision_files.len());
    for file in &revision.package.revision_files {
        assert_eq!(
            std::fs::read(primary.path.join(&file.path)).unwrap(),
            file.bytes
        );
    }
    assert!(
        !primary
            .path
            .join(&revision.package.catalog_file.as_ref().unwrap().path)
            .exists()
    );
    assert!(
        !primary
            .path
            .join(".upskill/native-pending/pending.json")
            .exists()
    );
    assert!(
        !private
            .path
            .join(".upskill/native-pending/pending.json")
            .exists()
    );
}

#[test]
fn a_queued_native_command_cannot_adopt_a_manual_binding() {
    with_store(|store, primary, _, _| {
        let capture = fixture::capture(store.context());
        let selected = NativeSkillSelection::from_package(&capture.candidates[0].package);
        let operation = store.prepare(&capture).unwrap();
        let mut adapter = Fake {
            selected: Some(selected),
            ..Default::default()
        };
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            operation.id(),
            RecoveryDisposition::Forward,
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::RecoveryRequired { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls, 0);
        assert!(!primary.path.join(".upskill-lock.json").exists());
        assert!(
            capture
                .file_observations
                .iter()
                .all(|file| !primary.path.join(&file.location.path).exists())
        );
    });
}

#[test]
fn both_exposure_orders_complete_using_independent_effect_and_binding_observations() {
    for exposure in [
        NativeExposureOrder::BeforeNative,
        NativeExposureOrder::AfterNative,
    ] {
        with_store(|store, primary, _, _| {
            let capture = fixture::capture(store.context());
            let mut adapter = Fake {
                exposure,
                selector: Some(
                    primary.path.join(
                        &capture.candidates[0]
                            .package
                            .catalog_file
                            .as_ref()
                            .unwrap()
                            .path,
                    ),
                ),
                ..Default::default()
            };
            let result = NativeDeliveryCoordinator::apply(store, &mut adapter, capture).unwrap();
            assert!(
                matches!(result, NativeDeliveryOutcome::Complete { .. }),
                "{result:?}"
            );
            assert_eq!(adapter.calls, 1);
        });
    }
}

#[test]
fn process_success_without_effective_binding_cannot_publish_state() {
    with_store(|store, primary, _, _| {
        let mut adapter = Fake {
            reject_observation: true,
            ..Default::default()
        };
        let result = NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap();
        assert!(matches!(
            result,
            NativeDeliveryOutcome::RecoveryRequired { .. }
        ));
        assert_eq!(adapter.calls, 1);
        assert!(!primary.path.join(".upskill-lock.json").exists());
        assert!(
            primary
                .path
                .join(".upskill/native-pending/pending.json")
                .exists()
        );
    });
}

#[test]
fn rejected_before_state_requires_explicit_forward_and_keeps_attempt_history() {
    with_store(|store, primary, _, _| {
        let mut adapter = Fake {
            refuse: true,
            ..Default::default()
        };
        let NativeDeliveryOutcome::RecoveryRequired {
            operation_id: Some(id),
            ..
        } = NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap()
        else {
            panic!("refusal completed")
        };
        assert_eq!(adapter.calls, 1);
        assert!(!primary.path.join(".upskill-lock.json").exists());
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &id,
            RecoveryDisposition::ObserveOnly,
        )
        .unwrap();
        assert!(matches!(
            result,
            NativeDeliveryOutcome::RecoveryRequired { .. }
        ));
        assert_eq!(adapter.calls, 1);
        adapter.refuse = false;
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &id,
            RecoveryDisposition::Forward,
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Complete { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls, 2);
        let inspection = store.inspect(&id).unwrap();
        assert_eq!(
            inspection
                .journal
                .runtime
                .unwrap()
                .calls
                .iter()
                .map(|call| call.outcome)
                .collect::<Vec<_>>(),
            vec![
                Some(NativeBindingOutcome::Rejected),
                Some(NativeBindingOutcome::Confirmed)
            ]
        );
    });
}

#[test]
fn inverse_timeout_is_observed_before_without_replaying_the_inverse() {
    with_store(|store, _, _, _| {
        let mut adapter = Fake {
            timeout: true,
            inverse_timeout: true,
            ..Default::default()
        };
        let NativeDeliveryOutcome::RecoveryRequired {
            operation_id: Some(id),
            ..
        } = NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap()
        else {
            panic!("timeout completed")
        };
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &id,
            RecoveryDisposition::Restore,
        )
        .unwrap();
        assert!(matches!(
            result,
            NativeDeliveryOutcome::RecoveryRequired { .. }
        ));
        assert_eq!(adapter.calls, 2);
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &id,
            RecoveryDisposition::ObserveOnly,
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Restored { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls, 2);
    });
}

#[test]
fn manual_export_changes_and_profile_drift_preserve_pending_evidence() {
    with_store(|store, primary, private, _| {
        let capture = fixture::capture(store.context());
        let resource = primary
            .path
            .join(&capture.candidates[0].package.revision_files[0].path);
        let mut adapter = Fake {
            timeout: true,
            ..Default::default()
        };
        let NativeDeliveryOutcome::RecoveryRequired {
            operation_id: Some(id),
            ..
        } = NativeDeliveryCoordinator::apply(store, &mut adapter, capture).unwrap()
        else {
            panic!("timeout completed")
        };
        let journal_path = private
            .path
            .join(format!(".upskill/native-evidence/journal-{id}.json"));
        let original = std::fs::read(&journal_path).unwrap();
        adapter.profile_revision += 1;
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &id,
            RecoveryDisposition::Forward,
        )
        .unwrap();
        assert!(matches!(
            result,
            NativeDeliveryOutcome::RecoveryRequired { .. }
        ));
        assert_eq!(std::fs::read(&journal_path).unwrap(), original);
        adapter.profile_revision -= 1;
        std::fs::write(&resource, b"manual bytes").unwrap();
        for disposition in [RecoveryDisposition::Forward, RecoveryDisposition::Restore] {
            let result =
                NativeDeliveryCoordinator::recover(store, &mut adapter, &id, disposition).unwrap();
            assert!(matches!(
                result,
                NativeDeliveryOutcome::RecoveryRequired { .. }
            ));
            assert_eq!(std::fs::read(&resource).unwrap(), b"manual bytes");
            assert_eq!(adapter.calls, 1);
        }
        assert!(!primary.path.join(".upskill-lock.json").exists());
    });
}

#[test]
fn refresh_cache_invalidation_blocks_completion_despite_both_exports_surviving() {
    for exposure in [
        NativeExposureOrder::BeforeNative,
        NativeExposureOrder::AfterNative,
    ] {
        with_store(|store, primary, _, _| {
            let old = fixture::capture(store.context());
            let old_revision = old.candidates[0].clone();
            let mut adapter = Fake {
                exposure,
                ..Default::default()
            };
            assert!(matches!(
                NativeDeliveryCoordinator::apply(store, &mut adapter, old).unwrap(),
                NativeDeliveryOutcome::Complete { .. }
            ));
            let before = std::fs::read(primary.path.join(".upskill-lock.json")).unwrap();
            let mut update = fixture::capture_with_body(
                store.context(),
                upskill::target::TargetSurface::ClaudeCli,
                "Updated payload.",
            );
            let new_revision = update.candidates[0].clone();
            update.graph_changes.add_references.clear();
            for file in old_revision
                .package
                .revision_files
                .iter()
                .chain(old_revision.package.catalog_file.iter())
            {
                let observation = upskill::files::plan::ObservedFile {
                    location: upskill::files::plan::FileLocation {
                        root: primary.clone(),
                        path: file.path.clone(),
                    },
                    value: Some(upskill::files::plan::FileValue {
                        bytes: file.bytes.clone(),
                        mode: Some(0o600),
                    }),
                };
                if let Some(existing) = update
                    .file_observations
                    .iter_mut()
                    .find(|row| row.location == observation.location)
                {
                    *existing = observation;
                } else {
                    update.file_observations.push(observation);
                }
            }
            // Caller-native claims are deliberately stale; coordinator must capture.
            adapter.invalidate_cache = true;
            let result = NativeDeliveryCoordinator::apply(store, &mut adapter, update).unwrap();
            assert!(
                matches!(
                    result,
                    NativeDeliveryOutcome::RecoveryRequired {
                        operation_id: Some(_),
                        ..
                    }
                ),
                "{result:?}"
            );
            assert_eq!(adapter.calls, 2);
            assert_eq!(
                std::fs::read(primary.path.join(".upskill-lock.json")).unwrap(),
                before
            );
            if exposure == NativeExposureOrder::AfterNative {
                let catalog = old_revision.package.catalog_file.as_ref().unwrap();
                assert_eq!(
                    std::fs::read(primary.path.join(&catalog.path)).unwrap(),
                    catalog.bytes,
                    "cache failure did not stop exposure publication"
                );
            }
            for revision in [old_revision, new_revision] {
                for file in revision.package.revision_files {
                    assert!(primary.path.join(file.path).exists());
                }
            }
        });
    }
}

#[test]
fn file_only_profiles_observe_binding_after_exposure_without_native_commands() {
    for exposure in [
        NativeExposureOrder::BeforeNative,
        NativeExposureOrder::AfterNative,
    ] {
        with_store(|store, primary, _, _| {
            let capture = fixture::capture(store.context());
            let revision = &capture.candidates[0];
            let mut adapter = Fake {
                exposure,
                action: NativeActionKind::FileOnly,
                selector: Some(
                    primary
                        .path
                        .join(&revision.package.catalog_file.as_ref().unwrap().path),
                ),
                file_selection: Some(NativeSkillSelection::from_package(&revision.package)),
                ..Default::default()
            };
            let result = NativeDeliveryCoordinator::apply(store, &mut adapter, capture).unwrap();
            assert!(
                matches!(result, NativeDeliveryOutcome::Complete { .. }),
                "{result:?}"
            );
            assert_eq!(adapter.calls, 0);
        });
    }
}

#[test]
fn observe_only_does_not_advance_files_when_native_effect_is_still_before() {
    with_store(|store, primary, _, _| {
        let capture = fixture::capture(store.context());
        let operation = store.prepare(&capture).unwrap();
        let mut adapter = Fake::default();
        store
            .prepare_execution(operation.id(), &mut adapter)
            .unwrap();
        store.begin_execution_step(operation.id(), 0).unwrap();
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            operation.id(),
            RecoveryDisposition::ObserveOnly,
        )
        .unwrap();
        assert!(matches!(
            result,
            NativeDeliveryOutcome::RecoveryRequired { .. }
        ));
        assert_eq!(adapter.calls, 0);
        assert!(
            capture
                .file_observations
                .iter()
                .all(|file| !primary.path.join(&file.location.path).exists())
        );
    });
}

#[test]
fn unavailable_prestate_returns_an_attributable_recovery_outcome_without_intent() {
    with_store(|store, primary, private, _| {
        let mut adapter = Fake {
            fail_capture: true,
            ..Default::default()
        };
        let result = NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap();
        assert!(matches!(
            result,
            NativeDeliveryOutcome::RecoveryRequired {
                operation_id: None,
                ..
            }
        ));
        assert_eq!(adapter.calls, 0);
        assert!(!primary.path.join(".upskill-lock.json").exists());
        assert!(!primary.path.join(".upskill/native-pending").exists());
        assert!(
            !std::fs::read_dir(private.path.join(".upskill/native-evidence"))
                .unwrap()
                .any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("intent-"))
        );
    });
}
#[test]
fn existing_pending_marker_is_reported_before_native_capture() {
    with_store(|store, _, _, _| {
        let capture = fixture::capture(store.context());
        let operation = store.prepare(&capture).unwrap();
        let mut adapter = Fake {
            fail_profile: true,
            fail_capture: true,
            ..Default::default()
        };
        let result = NativeDeliveryCoordinator::apply(store, &mut adapter, capture).unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::RecoveryRequired { operation_id: Some(ref id), .. } if id == operation.id())
        );
        assert_eq!(adapter.calls, 0);
    });
}

fn refresh_capture(
    store: &NativeOperationStore<'_, '_>,
    old: &upskill::lifecycle::skill_delivery::PackageRevision,
) -> upskill::native_delivery::journal::NativeOperationCapture {
    let mut update = fixture::capture_with_body(
        store.context(),
        upskill::target::TargetSurface::ClaudeCli,
        "Another revision.",
    );
    update.graph_changes.add_references.clear();
    for file in old
        .package
        .revision_files
        .iter()
        .chain(old.package.catalog_file.iter())
    {
        let observation = upskill::files::plan::ObservedFile {
            location: upskill::files::plan::FileLocation {
                root: store.context().scope.clone(),
                path: file.path.clone(),
            },
            value: Some(upskill::files::plan::FileValue {
                bytes: file.bytes.clone(),
                mode: Some(0o600),
            }),
        };
        if let Some(existing) = update
            .file_observations
            .iter_mut()
            .find(|row| row.location == observation.location)
        {
            *existing = observation;
        } else {
            update.file_observations.push(observation);
        }
    }
    update
}
#[test]
fn restoring_a_refresh_preserves_prior_selection_receipt_and_the_new_export() {
    with_store(|store, primary, private, locks| {
        let original = fixture::capture(store.context());
        let old = original.candidates[0].clone();
        let mut adapter = Fake::default();
        assert!(matches!(
            NativeDeliveryCoordinator::apply(store, &mut adapter, original).unwrap(),
            NativeDeliveryOutcome::Complete { .. }
        ));
        let state_store =
            upskill::native_delivery::store::NativeStateStore::open(locks, primary, private)
                .unwrap()
                .unwrap();
        let prior = state_store.load().unwrap().unwrap();
        let update = refresh_capture(store, &old);
        let new = update.candidates[0].clone();
        adapter.timeout = true;
        let NativeDeliveryOutcome::RecoveryRequired {
            operation_id: Some(id),
            ..
        } = NativeDeliveryCoordinator::apply(store, &mut adapter, update).unwrap()
        else {
            panic!("refresh timeout completed")
        };
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &id,
            RecoveryDisposition::Restore,
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Restored { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls, 3);
        assert_eq!(
            adapter.selected,
            Some(NativeSkillSelection::from_package(&old.package))
        );
        let restored = state_store.load().unwrap().unwrap();
        assert_eq!(restored.skills.graph, prior.skills.graph);
        assert_eq!(restored.skills.receipts, prior.skills.receipts);
        assert_eq!(restored.skills.revisions, prior.skills.revisions);
        assert_eq!(restored.skills.staged.len(), 1);
        assert_eq!(restored.skills.staged[0].revision, new);
        assert_eq!(
            std::fs::read(
                primary
                    .path
                    .join(&old.package.catalog_file.as_ref().unwrap().path)
            )
            .unwrap(),
            old.package.catalog_file.as_ref().unwrap().bytes
        );
        for revision in [old, new] {
            for file in revision.package.revision_files {
                assert!(primary.path.join(file.path).exists());
            }
        }
    });
}
#[test]
fn detach_keeps_native_bound_history_and_exports_after_removing_its_logical_owner() {
    with_store(|store, primary, private, locks| {
        let original = fixture::capture(store.context());
        let old = original.candidates[0].clone();
        let mut adapter = Fake::default();
        assert!(matches!(
            NativeDeliveryCoordinator::apply(store, &mut adapter, original.clone()).unwrap(),
            NativeDeliveryOutcome::Complete { .. }
        ));
        let mut remove = upskill::native_delivery::journal::NativeOperationCapture::default();
        remove.graph_changes.remove_references = original.graph_changes.add_references;
        for file in old
            .package
            .revision_files
            .iter()
            .chain(old.package.catalog_file.iter())
        {
            remove
                .file_observations
                .push(upskill::files::plan::ObservedFile {
                    location: upskill::files::plan::FileLocation {
                        root: primary.clone(),
                        path: file.path.clone(),
                    },
                    value: Some(upskill::files::plan::FileValue {
                        bytes: file.bytes.clone(),
                        mode: Some(0o600),
                    }),
                });
        }
        let result = NativeDeliveryCoordinator::apply(store, &mut adapter, remove).unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Complete { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls, 2);
        let state =
            upskill::native_delivery::store::NativeStateStore::open(locks, primary, private)
                .unwrap()
                .unwrap()
                .load()
                .unwrap()
                .unwrap();
        assert!(state.skills.graph.installations.is_empty());
        assert!(state.skills.staged.is_empty());
        assert!(state.intents.is_empty());
        assert_eq!(state.skills.revisions, vec![old.clone()]);
        assert_eq!(
            state.skills.receipts[0].before,
            Some(NativeSkillSelection::from_package(&old.package))
        );
        assert_eq!(state.skills.receipts[0].after, None);
        assert!(
            !primary
                .path
                .join(&old.package.catalog_file.as_ref().unwrap().path)
                .exists()
        );
        for file in old.package.revision_files {
            assert!(primary.path.join(file.path).exists());
        }
    });
}

#[test]
fn cross_root_configuration_and_noop_owned_drift_preserve_manual_data() {
    use upskill::files::plan::{FileLocation, FileValue, ObservedFile};
    use upskill::native_config::{
        CapturedConfig, ConfigContribution, ConfigFormat, ConfigLayer, ConfigSelector,
    };
    use upskill::native_delivery::journal::{NativeConfigurationCapture, NativeOperationCapture};
    use upskill::native_delivery::state::ConfigOwner;
    for exposure in [
        NativeExposureOrder::BeforeNative,
        NativeExposureOrder::AfterNative,
    ] {
        with_store(|store, primary, private, _| {
            let mut capture = fixture::capture(store.context());
            let extra = store.context().additional_roots[0].clone();
            let path = extra.path.join(".settings/native.json");
            std::fs::create_dir(path.parent().unwrap()).unwrap();
            let manual = b"{\n \"manual\": \"PRIVATE_MANUAL_SETTING\"\n}\n";
            std::fs::write(&path, manual).unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            capture.configurations.push(NativeConfigurationCapture {
                before: CapturedConfig {
                    surface: upskill::target::TargetSurface::ClaudeCli,
                    scope: extra.clone(),
                    target: ConfigLayer {
                        format: ConfigFormat::Jsonc,
                        file: ObservedFile {
                            location: FileLocation {
                                root: extra.clone(),
                                path: ".settings/native.json".into(),
                            },
                            value: Some(FileValue {
                                bytes: manual.to_vec(),
                                mode: Some(0o644),
                            }),
                        },
                        tracked: false,
                        ignored: true,
                    },
                    lower_layers: vec![],
                    higher_layers: vec![],
                    ownership: None,
                },
                contribution: ConfigContribution {
                    id: "registration".into(),
                    selector: ConfigSelector::Key(vec!["plugins".into(), "fixture".into()]),
                    expected: None,
                    desired: Some(serde_json::json!(
                        capture.candidates[0].package.revision_root
                    )),
                },
                additional_contributions: vec![ConfigContribution {
                    id: "enabled".into(),
                    selector: ConfigSelector::Key(vec!["enabled".into()]),
                    expected: None,
                    desired: Some(serde_json::json!(true)),
                }],
                owners_after: vec![
                    ConfigOwner {
                        entry_id: "registration".into(),
                        target: capture.intents[0].key.clone(),
                    },
                    ConfigOwner {
                        entry_id: "enabled".into(),
                        target: capture.intents[0].key.clone(),
                    },
                ],
            });
            let mut adapter = Fake {
                exposure,
                exposure_probe: Some((
                    path.clone(),
                    capture.candidates[0].package.revision_root.clone(),
                )),
                ..Default::default()
            };
            let result =
                NativeDeliveryCoordinator::apply(store, &mut adapter, capture.clone()).unwrap();
            assert!(
                matches!(result, NativeDeliveryOutcome::Complete { .. }),
                "{result:?}"
            );
            let before = std::fs::read(primary.path.join(".upskill-lock.json")).unwrap();
            assert!(
                !String::from_utf8(before.clone())
                    .unwrap()
                    .contains("PRIVATE_MANUAL_SETTING")
            );
            let configured = std::fs::read(&path).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&configured).unwrap();
            assert_eq!(
                value["plugins"]["fixture"],
                capture.candidates[0].package.revision_root
            );
            assert_eq!(value["enabled"], true);
            assert_eq!(value["manual"], "PRIVATE_MANUAL_SETTING");
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o644
            );
            let mut noop = NativeOperationCapture {
                intents: capture.intents.clone(),
                candidates: capture.candidates.clone(),
                ..Default::default()
            };
            for file in &capture.file_observations {
                noop.file_observations.push(ObservedFile {
                    location: file.location.clone(),
                    value: Some(FileValue {
                        bytes: std::fs::read(primary.path.join(&file.location.path)).unwrap(),
                        mode: Some(0o600),
                    }),
                });
            }
            let result =
                NativeDeliveryCoordinator::apply(store, &mut adapter, noop.clone()).unwrap();
            assert!(
                matches!(result, NativeDeliveryOutcome::Complete { .. }),
                "{result:?}"
            );
            assert_eq!(adapter.calls, 1);
            let stable_before = std::fs::read(primary.path.join(".upskill-lock.json")).unwrap();
            let canonical = |bytes: &[u8]| {
                let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
                value["files"]
                    .as_array_mut()
                    .unwrap()
                    .sort_by_key(|row| row["path"].as_str().unwrap().to_owned());
                value
            };
            assert_eq!(canonical(&stable_before), canonical(&before));
            let before = stable_before;
            let edited = String::from_utf8(configured).unwrap().replace(
                &capture.candidates[0].package.revision_root,
                "manual-foreign-root",
            );
            std::fs::write(&path, edited.as_bytes()).unwrap();
            let result = NativeDeliveryCoordinator::apply(store, &mut adapter, noop).unwrap();
            assert!(
                matches!(result, NativeDeliveryOutcome::RecoveryRequired { .. }),
                "{result:?}"
            );
            assert_eq!(adapter.calls, 1);
            assert_eq!(std::fs::read(&path).unwrap(), edited.as_bytes());
            assert_eq!(
                std::fs::read(primary.path.join(".upskill-lock.json")).unwrap(),
                before
            );
            assert!(
                !private
                    .path
                    .join(".upskill/native-pending/pending.json")
                    .exists()
            );
        });
    }
}

#[test]
fn unknown_effective_binding_blocks_after_native_exposure_even_with_known_registration() {
    with_store(|store, primary, _, _| {
        let capture = fixture::capture(store.context());
        let catalog = primary.path.join(
            &capture.candidates[0]
                .package
                .catalog_file
                .as_ref()
                .unwrap()
                .path,
        );
        let mut adapter = Fake {
            unknown_after_apply: true,
            exposure: NativeExposureOrder::AfterNative,
            ..Default::default()
        };
        let result = NativeDeliveryCoordinator::apply(store, &mut adapter, capture).unwrap();
        assert!(matches!(
            result,
            NativeDeliveryOutcome::RecoveryRequired {
                operation_id: Some(_),
                ..
            }
        ));
        assert_eq!(adapter.calls, 1);
        assert!(
            !catalog.exists(),
            "unknown binding did not block later exposure"
        );
        assert!(!primary.path.join(".upskill-lock.json").exists());
    });
}

#[derive(Default)]
struct Dual {
    clients: std::collections::BTreeMap<(String, &'static str), Fake>,
}
impl Dual {
    fn client(&mut self, key: &upskill::lifecycle::native_receipt::SkillTarget) -> &mut Fake {
        self.clients
            .entry((key.installation_id.clone(), key.surface.name()))
            .or_insert_with(|| Fake {
                exposure: if key.surface == upskill::target::TargetSurface::CopilotCli {
                    NativeExposureOrder::AfterNative
                } else {
                    NativeExposureOrder::BeforeNative
                },
                ..Default::default()
            })
    }
    fn calls(&self) -> usize {
        self.clients.values().map(|client| client.calls).sum()
    }
}
impl NativeDeliveryAdapter for Dual {
    fn profile(
        &mut self,
        key: &upskill::lifecycle::native_receipt::SkillTarget,
        scope: &InstallationRoot,
    ) -> anyhow::Result<NativeDeliveryProfile> {
        self.client(key).profile(key, scope)
    }
    fn capture(
        &mut self,
        key: &upskill::lifecycle::native_receipt::SkillTarget,
        scope: &InstallationRoot,
    ) -> anyhow::Result<upskill::lifecycle::native_receipt::NativeSkillObservation> {
        self.client(key).capture(key, scope)
    }
    fn plan_inverse(&mut self, request: &NativeEffectRequest) -> anyhow::Result<Vec<u8>> {
        self.client(&request.change.key).plan_inverse(request)
    }
    fn apply(&mut self, effect: &PlannedNativeEffect) -> anyhow::Result<NativeBindingOutcome> {
        self.client(&effect.request.change.key).apply(effect)
    }
    fn observe(&mut self, effect: &PlannedNativeEffect) -> anyhow::Result<NativeEffectObservation> {
        self.client(&effect.request.change.key).observe(effect)
    }
    fn inverse(&mut self, effect: &PlannedNativeEffect) -> anyhow::Result<NativeBindingOutcome> {
        self.client(&effect.request.change.key).inverse(effect)
    }
}
fn two_clients(
    store: &NativeOperationStore<'_, '_>,
    body: &str,
) -> upskill::native_delivery::journal::NativeOperationCapture {
    use upskill::target::TargetSurface;
    let mut first = fixture::capture_with_body(store.context(), TargetSurface::ClaudeCli, body);
    let mut second = fixture::capture_with_body(store.context(), TargetSurface::CopilotCli, body);
    assert_eq!(
        first.candidates[0].installation.source_evidence,
        second.candidates[0].installation.source_evidence
    );
    let mut installation = first.graph_changes.installations[0].clone();
    installation.requested = vec![TargetSurface::ClaudeCli, TargetSurface::CopilotCli];
    first.graph_changes.installations = vec![installation.clone()];
    first.candidates[0].installation = installation.clone();
    second.candidates[0].installation = installation;
    first.candidates.extend(second.candidates);
    first.intents.extend(second.intents);
    first.file_observations.extend(second.file_observations);
    first.native_observations.clear();
    first
}
#[test]
fn mixed_client_refresh_restores_only_the_effected_target_with_its_own_inverse() {
    with_store(|store, primary, private, locks| {
        let original = two_clients(store, "Original two-client body.");
        let old = original.candidates.clone();
        let mut adapter = Dual::default();
        let result =
            NativeDeliveryCoordinator::apply(store, &mut adapter, original.clone()).unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Complete { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls(), 2);
        let state_store =
            upskill::native_delivery::store::NativeStateStore::open(locks, primary, private)
                .unwrap()
                .unwrap();
        let prior = state_store.load().unwrap().unwrap();
        let mut update = two_clients(store, "Updated two-client body.");
        let new = update.candidates.clone();
        update.graph_changes.add_references.clear();
        for revision in &old {
            for file in revision
                .package
                .revision_files
                .iter()
                .chain(revision.package.catalog_file.iter())
            {
                let observation = upskill::files::plan::ObservedFile {
                    location: upskill::files::plan::FileLocation {
                        root: primary.clone(),
                        path: file.path.clone(),
                    },
                    value: Some(upskill::files::plan::FileValue {
                        bytes: file.bytes.clone(),
                        mode: Some(0o600),
                    }),
                };
                if let Some(existing) = update
                    .file_observations
                    .iter_mut()
                    .find(|row| row.location == observation.location)
                {
                    *existing = observation;
                } else {
                    update.file_observations.push(observation);
                }
            }
        }
        adapter.client(&original.intents[0].key).timeout = true;
        let NativeDeliveryOutcome::RecoveryRequired {
            operation_id: Some(id),
            ..
        } = NativeDeliveryCoordinator::apply(store, &mut adapter, update).unwrap()
        else {
            panic!("partial refresh completed")
        };
        assert_eq!(adapter.calls(), 3);
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &id,
            RecoveryDisposition::Restore,
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Restored { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls(), 4);
        let restored = state_store.load().unwrap().unwrap();
        assert_eq!(restored.skills.graph, prior.skills.graph);
        assert_eq!(restored.skills.receipts, prior.skills.receipts);
        assert_eq!(restored.skills.staged.len(), 2);
        for revision in &old {
            let key = upskill::lifecycle::native_receipt::SkillTarget {
                installation_id: revision.installation.owner.installation_id.clone(),
                surface: revision.selected.surface,
            };
            assert_eq!(
                adapter.client(&key).selected,
                Some(NativeSkillSelection::from_package(&revision.package))
            );
        }
        for revision in old.iter().chain(&new) {
            for file in &revision.package.revision_files {
                assert!(primary.path.join(&file.path).exists());
            }
        }
    });
}

#[test]
fn a_refused_inverse_requires_another_explicit_restore_and_retains_its_outcome() {
    with_store(|store, _, _, _| {
        let mut adapter = Fake {
            timeout: true,
            inverse_refuse: true,
            ..Default::default()
        };
        let NativeDeliveryOutcome::RecoveryRequired {
            operation_id: Some(id),
            ..
        } = NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap()
        else {
            panic!("timeout completed")
        };
        assert!(matches!(
            NativeDeliveryCoordinator::recover(
                store,
                &mut adapter,
                &id,
                RecoveryDisposition::Restore
            )
            .unwrap(),
            NativeDeliveryOutcome::RecoveryRequired { .. }
        ));
        assert_eq!(adapter.calls, 2);
        assert!(matches!(
            NativeDeliveryCoordinator::recover(
                store,
                &mut adapter,
                &id,
                RecoveryDisposition::ObserveOnly
            )
            .unwrap(),
            NativeDeliveryOutcome::RecoveryRequired { .. }
        ));
        assert_eq!(adapter.calls, 2);
        adapter.inverse_refuse = false;
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &id,
            RecoveryDisposition::Restore,
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Restored { .. }),
            "{result:?}"
        );
        assert_eq!(adapter.calls, 3);
        let inspection = store.inspect(&id).unwrap();
        assert_eq!(
            inspection
                .journal
                .runtime
                .as_ref()
                .unwrap()
                .calls
                .iter()
                .map(|call| call.outcome)
                .collect::<Vec<_>>(),
            vec![
                Some(NativeBindingOutcome::Unknown),
                Some(NativeBindingOutcome::Rejected),
                Some(NativeBindingOutcome::Confirmed)
            ]
        );
    });
}

#[test]
fn an_exception_after_effect_keeps_error_detail_and_recovers_without_replay() {
    with_store(|store, primary, _, _| {
        let mut adapter = Fake {
            error_after_apply: true,
            ..Default::default()
        };
        let NativeDeliveryOutcome::RecoveryRequired {
            operation_id: Some(id),
            ..
        } = NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap()
        else {
            panic!("exception completed")
        };
        assert_eq!(adapter.calls, 1);
        assert!(!primary.path.join(".upskill-lock.json").exists());
        let inspection = store.inspect(&id).unwrap();
        let call = &inspection.journal.runtime.as_ref().unwrap().calls[0];
        assert_eq!(call.outcome, Some(NativeBindingOutcome::Unknown));
        assert_eq!(
            call.error.as_deref(),
            Some("fixture native command failed after effect")
        );
        let row = &inspection.journal.checkpoints[call.index];
        assert_eq!(row.attempts.last().unwrap().detail, call.error);
        let result = NativeDeliveryCoordinator::recover(
            store,
            &mut adapter,
            &id,
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
fn file_only_restore_requires_retention_before_releasing_owned_exposure() {
    for exposure in [
        NativeExposureOrder::BeforeNative,
        NativeExposureOrder::AfterNative,
    ] {
        with_store(|store, primary, private, locks| {
            let capture = fixture::capture(store.context());
            let revision = capture.candidates[0].clone();
            let catalog = primary
                .path
                .join(&revision.package.catalog_file.as_ref().unwrap().path);
            let mut adapter = Fake {
                exposure,
                action: NativeActionKind::FileOnly,
                selector: Some(catalog.clone()),
                file_selection: Some(NativeSkillSelection::from_package(&revision.package)),
                ..Default::default()
            };
            let operation = store.prepare(&capture).unwrap();
            store
                .prepare_execution(operation.id(), &mut adapter)
                .unwrap();
            let execution = store
                .inspect(operation.id())
                .unwrap()
                .journal
                .execution
                .unwrap();
            for (index, step) in execution.steps.iter().enumerate() {
                let file = match step {
                    NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file) => file,
                    NativeExecutionStep::Native(_) => break,
                };
                store.begin_execution_step(operation.id(), index).unwrap();
                let path = primary.path.join(&file.location.path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                let value = file.after.as_ref().unwrap();
                std::fs::write(&path, &value.bytes).unwrap();
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    &path,
                    std::fs::Permissions::from_mode(value.mode.unwrap()),
                )
                .unwrap();
                assert!(
                    store
                        .observe_pending_step(operation.id(), index, &mut adapter)
                        .unwrap()
                );
            }
            adapter.cache_valid = false;
            let result = NativeDeliveryCoordinator::recover(
                store,
                &mut adapter,
                operation.id(),
                RecoveryDisposition::Restore,
            )
            .unwrap();
            assert!(matches!(
                result,
                NativeDeliveryOutcome::RecoveryRequired { .. }
            ));
            assert!(catalog.exists());
            assert_eq!(adapter.calls, 0);
            adapter.cache_valid = true;
            let result = NativeDeliveryCoordinator::recover(
                store,
                &mut adapter,
                operation.id(),
                RecoveryDisposition::Restore,
            )
            .unwrap();
            assert!(
                matches!(result, NativeDeliveryOutcome::Restored { .. }),
                "{result:?}"
            );
            assert!(!catalog.exists());
            assert_eq!(adapter.calls, 0);
            let state =
                upskill::native_delivery::store::NativeStateStore::open(locks, primary, private)
                    .unwrap()
                    .unwrap()
                    .load()
                    .unwrap()
                    .unwrap();
            assert!(state.skills.receipts.is_empty());
            assert_eq!(state.skills.staged.len(), 1);
        });
    }
}

#[test]
fn unsupported_profile_is_rejected_before_capture_or_pending_operation_creation() {
    with_store(|store, primary, private, _| {
        let mut adapter = Fake {
            fail_profile: true,
            fail_capture: true,
            ..Default::default()
        };
        let result = NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap();
        let NativeDeliveryOutcome::RecoveryRequired {
            operation_id: None,
            reason,
        } = result
        else {
            panic!("unsupported profile created an operation: {result:?}")
        };
        assert!(
            reason.contains("unsupported native profile"),
            "capture happened before profile admission: {reason}"
        );
        assert!(!primary.path.join(".upskill/native-pending").exists());
        assert!(
            !std::fs::read_dir(private.path.join(".upskill/native-evidence"))
                .unwrap()
                .any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("intent-"))
        );
        assert_eq!(adapter.calls, 0);
    });
}

#[test]
fn user_delivery_can_share_one_physical_root_with_private_evidence() {
    let home = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::User,
        path: home.path().into(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let locks = lock_native_roots(&[NativeRoot {
        root: root.clone(),
        scope: &scope,
    }])
    .unwrap();
    let store = NativeOperationStore::initialize(&locks, &root, &root).unwrap();
    let mut adapter = Fake::default();
    let result =
        NativeDeliveryCoordinator::apply(&store, &mut adapter, fixture::capture(store.context()))
            .unwrap();
    let NativeDeliveryOutcome::Complete { operation_id } = result else {
        panic!("{result:?}")
    };
    let inspection = store.inspect(&operation_id).unwrap();
    assert_eq!(inspection.intent.roots.len(), 1);
    assert_eq!(inspection.intent.context.scope, root);
    assert_eq!(inspection.intent.context.private_root, root);
    assert_eq!(adapter.calls, 1);
    assert!(
        !root
            .path
            .join(".upskill/native-pending/pending.json")
            .exists()
    );
    let state = upskill::native_delivery::store::NativeStateStore::open(&locks, &root, &root)
        .unwrap()
        .unwrap()
        .load()
        .unwrap()
        .unwrap();
    assert_eq!(state.skills.scope, root);
    assert_eq!(state.skills.receipts.len(), 1);
}

struct Reentrant<'store, 'locks, 'scope> {
    store: &'store NativeOperationStore<'locks, 'scope>,
    inner: Fake,
    nested: Option<NativeDeliveryOutcome>,
}
impl NativeDeliveryAdapter for Reentrant<'_, '_, '_> {
    fn profile(
        &mut self,
        key: &upskill::lifecycle::native_receipt::SkillTarget,
        scope: &InstallationRoot,
    ) -> anyhow::Result<NativeDeliveryProfile> {
        self.inner.profile(key, scope)
    }
    fn capture(
        &mut self,
        key: &upskill::lifecycle::native_receipt::SkillTarget,
        scope: &InstallationRoot,
    ) -> anyhow::Result<upskill::lifecycle::native_receipt::NativeSkillObservation> {
        self.inner.capture(key, scope)
    }
    fn plan_inverse(&mut self, request: &NativeEffectRequest) -> anyhow::Result<Vec<u8>> {
        self.inner.plan_inverse(request)
    }
    fn observe(&mut self, effect: &PlannedNativeEffect) -> anyhow::Result<NativeEffectObservation> {
        self.inner.observe(effect)
    }
    fn inverse(&mut self, effect: &PlannedNativeEffect) -> anyhow::Result<NativeBindingOutcome> {
        self.inner.inverse(effect)
    }
    fn apply(&mut self, effect: &PlannedNativeEffect) -> anyhow::Result<NativeBindingOutcome> {
        let mut second = Fake {
            backing: self.inner.backing.clone(),
            ..Default::default()
        };
        self.nested = Some(NativeDeliveryCoordinator::recover(
            self.store,
            &mut second,
            &effect.request.operation_id,
            RecoveryDisposition::Forward,
        )?);
        self.inner.apply(effect)
    }
}
#[test]
fn one_held_root_set_cannot_run_a_reentrant_native_recovery() {
    with_store(|store, primary, _, _| {
        let mut adapter = Reentrant {
            store,
            inner: Fake {
                backing: Some(primary.path.join("native-fixture.json")),
                ..Default::default()
            },
            nested: None,
        };
        let result = NativeDeliveryCoordinator::apply(
            store,
            &mut adapter,
            fixture::capture(store.context()),
        )
        .unwrap();
        assert!(
            matches!(result, NativeDeliveryOutcome::Complete { .. }),
            "{result:?}"
        );
        let Some(NativeDeliveryOutcome::RecoveryRequired { reason, .. }) = adapter.nested else {
            panic!("nested recovery was not excluded")
        };
        assert!(reason.contains("already active"));
        assert_eq!(adapter.inner.calls, 1);
    });
}
