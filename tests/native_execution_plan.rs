//! Durable effect recipes remain separate from native mutation or completion.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/native_delivery.rs"]
mod fixture;
use anyhow::Result;
use upskill::filesystem::QualifiedScope;
use upskill::lifecycle::native_receipt::*;
use upskill::native_delivery::adapter::*;
use upskill::native_delivery::journal::{NativeJournalStatus, NativeOperationStore};
use upskill::native_delivery::roots::{NativeRoot, lock_native_roots};
use upskill::ownership::{InstallationRoot, OwnershipScope};

struct PlanningAdapter {
    exposure: NativeExposureOrder,
    bad_profile: bool,
    fail_inverse: bool,
    calls: usize,
    observation: Option<NativeEffectObservation>,
}
impl NativeDeliveryAdapter for PlanningAdapter {
    fn profile(
        &mut self,
        target: &SkillTarget,
        scope: &InstallationRoot,
    ) -> Result<NativeDeliveryProfile> {
        Ok(NativeDeliveryProfile {
            id: "fixture".into(),
            revision: 1,
            surface: target.surface,
            scope: scope.clone(),
            client_version: if self.bad_profile {
                "wrong"
            } else {
                "fixture-1"
            }
            .into(),
            platform: "fixture-os".into(),
            exposure: self.exposure,
            action: NativeActionKind::Command,
        })
    }
    fn capture(&mut self, _: &SkillTarget, _: &InstallationRoot) -> Result<NativeSkillObservation> {
        anyhow::bail!("not an execution")
    }
    fn plan_inverse(&mut self, request: &NativeEffectRequest) -> Result<Vec<u8>> {
        if self.fail_inverse {
            anyhow::bail!("inverse unavailable");
        }
        Ok(serde_json::to_vec(&request.change.before)?)
    }
    fn apply(&mut self, _: &PlannedNativeEffect) -> Result<NativeBindingOutcome> {
        self.calls += 1;
        anyhow::bail!("must not execute")
    }
    fn observe(&mut self, _: &PlannedNativeEffect) -> Result<NativeEffectObservation> {
        self.observation
            .clone()
            .ok_or_else(|| anyhow::anyhow!("observer unavailable"))
    }
    fn inverse(&mut self, _: &PlannedNativeEffect) -> Result<NativeBindingOutcome> {
        self.calls += 1;
        anyhow::bail!("must not execute")
    }
}

#[test]
fn recipe_is_durable_ordered_and_does_not_execute_or_claim_completion() {
    for exposure in [
        NativeExposureOrder::BeforeNative,
        NativeExposureOrder::AfterNative,
    ] {
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
        let prepared = store.prepare(&capture).unwrap();
        let before = store.inspect(prepared.id()).unwrap();
        let mut adapter = PlanningAdapter {
            exposure,
            bad_profile: true,
            fail_inverse: false,
            calls: 0,
            observation: None,
        };
        assert!(
            store
                .prepare_execution(prepared.id(), &mut adapter)
                .is_err()
        );
        assert_eq!(
            store.inspect(prepared.id()).unwrap().journal,
            before.journal
        );
        adapter.bad_profile = false;
        adapter.fail_inverse = true;
        assert!(
            store
                .prepare_execution(prepared.id(), &mut adapter)
                .is_err()
        );
        assert_eq!(
            store.inspect(prepared.id()).unwrap().journal,
            before.journal
        );
        adapter.fail_inverse = false;
        let marker_path = primary.path.join(".upskill/native-pending/pending.json");
        let marker_bytes = std::fs::read(&marker_path).unwrap();
        std::fs::remove_file(&marker_path).unwrap();
        assert!(
            store
                .prepare_execution(prepared.id(), &mut adapter)
                .is_err(),
            "partial marker coverage accepted"
        );
        assert_eq!(
            store.inspect(prepared.id()).unwrap().journal,
            before.journal
        );
        std::fs::write(&marker_path, marker_bytes).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&marker_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        store
            .prepare_execution(prepared.id(), &mut adapter)
            .unwrap();
        drop(store);
        let reopened = NativeOperationStore::initialize(&locks, &primary, &private).unwrap();
        let inspection = reopened.inspect(prepared.id()).unwrap();
        assert!(inspection.markers_complete);
        assert_eq!(inspection.journal.status, NativeJournalStatus::Executing);
        assert_eq!(inspection.journal.generation, 1);
        let execution = inspection.journal.execution.as_ref().unwrap();
        let native = execution
            .steps
            .iter()
            .position(|step| matches!(step, NativeExecutionStep::Native(_)))
            .unwrap();
        let exposure_index = execution
            .steps
            .iter()
            .position(|step| matches!(step, NativeExecutionStep::Exposure(_)))
            .unwrap();
        assert_eq!(
            native > exposure_index,
            exposure == NativeExposureOrder::BeforeNative
        );
        assert!(
            execution.steps[..native.min(exposure_index)]
                .iter()
                .all(|step| matches!(step, NativeExecutionStep::Stage(_)))
        );
        let NativeExecutionStep::Native(effect) = &execution.steps[native] else {
            unreachable!()
        };
        assert_eq!(effect.request.operation_id, prepared.id());
        assert_eq!(
            effect.request.change,
            inspection.intent.plan.native_changes[0]
        );
        assert_eq!(effect.inverse_payload, b"null");
        assert!(
            reopened
                .prepare_execution(prepared.id(), &mut adapter)
                .is_err(),
            "recipe was replaceable"
        );
        assert_eq!(
            reopened.inspect(prepared.id()).unwrap().journal,
            inspection.journal
        );
        let journal_path = private.path.join(format!(
            ".upskill/native-evidence/journal-{}.json",
            prepared.id()
        ));
        let original = std::fs::read(&journal_path).unwrap();
        let mut tampered: serde_json::Value = serde_json::from_slice(&original).unwrap();
        tampered["execution"]["steps"][0]["Stage"]["after"]["mode"] = 0o644.into();
        std::fs::write(&journal_path, serde_json::to_vec(&tampered).unwrap()).unwrap();
        assert!(
            reopened
                .inspect(prepared.id())
                .unwrap_err()
                .to_string()
                .contains("recipe differs")
        );
        std::fs::write(&journal_path, &original).unwrap();
        let mut tampered: serde_json::Value = serde_json::from_slice(&original).unwrap();
        tampered["checkpoints"][0]["phase"] = "ObservedAfter".into();
        std::fs::write(&journal_path, serde_json::to_vec(&tampered).unwrap()).unwrap();
        assert!(
            reopened
                .inspect(prepared.id())
                .unwrap_err()
                .to_string()
                .contains("without observed after-state")
        );
        std::fs::write(&journal_path, &original).unwrap();
        assert_eq!(
            reopened.inspect(prepared.id()).unwrap().journal,
            inspection.journal
        );
        assert_eq!(adapter.calls, 0);
        for file in &capture.file_observations {
            assert!(!primary.path.join(&file.location.path).exists());
        }
        assert!(!primary.path.join(".upskill-lock.json").exists());
    }
}

#[test]
fn pending_file_checkpoint_survives_restart_and_requires_actual_bytes_and_mode() {
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
    let prepared = store.prepare(&fixture::capture(store.context())).unwrap();
    let mut adapter = PlanningAdapter {
        exposure: NativeExposureOrder::BeforeNative,
        bad_profile: false,
        fail_inverse: false,
        calls: 0,
        observation: None,
    };
    store
        .prepare_execution(prepared.id(), &mut adapter)
        .unwrap();
    assert!(
        store.begin_execution_step(prepared.id(), 1).is_err(),
        "skipped queued export"
    );
    let NativeExecutionStep::Stage(file) = store.begin_execution_step(prepared.id(), 0).unwrap()
    else {
        panic!("first step was not staging")
    };
    let pending = store.inspect(prepared.id()).unwrap().journal;
    assert_eq!(pending.checkpoints[0].phase, NativeStepPhase::Pending);
    assert!(
        store.begin_execution_step(prepared.id(), 0).is_err(),
        "pending step replay authorized"
    );
    drop(store);
    let store = NativeOperationStore::initialize(&locks, &primary, &private).unwrap();
    assert_eq!(store.inspect(prepared.id()).unwrap().journal, pending);
    assert!(
        !store
            .observe_pending_step(prepared.id(), 0, &mut adapter)
            .unwrap()
    );
    let path = primary.path.join(&file.location.path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"manual unrelated bytes").unwrap();
    assert!(
        !store
            .observe_pending_step(prepared.id(), 0, &mut adapter)
            .unwrap()
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"manual unrelated bytes");
    let desired = file.after.as_ref().unwrap();
    std::fs::write(&path, &desired.bytes).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        !store
            .observe_pending_step(prepared.id(), 0, &mut adapter)
            .unwrap()
    );
    std::fs::set_permissions(
        &path,
        std::fs::Permissions::from_mode(desired.mode.unwrap()),
    )
    .unwrap();
    assert!(
        store
            .observe_pending_step(prepared.id(), 0, &mut adapter)
            .unwrap()
    );
    let acknowledged = store.inspect(prepared.id()).unwrap().journal;
    assert_eq!(
        acknowledged.checkpoints[0].phase,
        NativeStepPhase::ObservedAfter
    );
    assert_eq!(acknowledged.checkpoints[0].observations.len(), 4);
    assert!(store.begin_execution_step(prepared.id(), 0).is_err());
    store.begin_execution_step(prepared.id(), 1).unwrap();
    assert_eq!(adapter.calls, 0);
    assert!(!primary.path.join(".upskill-lock.json").exists());
}

#[test]
fn native_attempts_never_acknowledge_effects_and_bad_attribution_cannot_be_adopted() {
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
    let prepared = store.prepare(&capture).unwrap();
    let mut adapter = PlanningAdapter {
        exposure: NativeExposureOrder::BeforeNative,
        bad_profile: false,
        fail_inverse: false,
        calls: 0,
        observation: None,
    };
    store
        .prepare_execution(prepared.id(), &mut adapter)
        .unwrap();
    let execution = store
        .inspect(prepared.id())
        .unwrap()
        .journal
        .execution
        .unwrap();
    let index = execution
        .steps
        .iter()
        .position(|step| matches!(step, NativeExecutionStep::Native(_)))
        .unwrap();
    assert!(
        store
            .record_native_attempt(prepared.id(), index, NativeBindingOutcome::Confirmed, None)
            .is_err()
    );
    for (i, step) in execution.steps[..index].iter().enumerate() {
        store.begin_execution_step(prepared.id(), i).unwrap();
        let file = match step {
            NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file) => file,
            _ => unreachable!(),
        };
        let path = file.location.root.path.join(&file.location.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let value = file.after.as_ref().unwrap();
        std::fs::write(&path, &value.bytes).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(value.mode.unwrap()))
            .unwrap();
        assert!(
            store
                .observe_pending_step(prepared.id(), i, &mut adapter)
                .unwrap()
        );
    }
    let NativeExecutionStep::Native(effect) =
        store.begin_execution_step(prepared.id(), index).unwrap()
    else {
        unreachable!()
    };
    store
        .record_native_attempt(prepared.id(), index, NativeBindingOutcome::Confirmed, None)
        .unwrap();
    assert!(
        !store
            .observe_pending_step(prepared.id(), index, &mut adapter)
            .unwrap()
    );
    store
        .record_native_attempt(
            prepared.id(),
            index,
            NativeBindingOutcome::Unknown,
            Some("timeout after effect".into()),
        )
        .unwrap();
    let mut binding = capture.native_observations[0].clone();
    binding.observation_id = "independent-probe".into();
    let observation = NativeEffectObservation {
        operation_id: prepared.id().into(),
        profile_id: effect.request.profile.id.clone(),
        profile_revision: 1,
        effect: NativeEffectRelation::Before,
        binding,
    };
    adapter.observation = Some(NativeEffectObservation {
        operation_id: "f".repeat(32),
        ..observation.clone()
    });
    let before = store.inspect(prepared.id()).unwrap().journal;
    assert!(
        store
            .observe_pending_step(prepared.id(), index, &mut adapter)
            .is_err()
    );
    assert_eq!(store.inspect(prepared.id()).unwrap().journal, before);
    adapter.observation = Some(observation.clone());
    adapter.bad_profile = true;
    assert!(
        store
            .observe_pending_step(prepared.id(), index, &mut adapter)
            .is_err()
    );
    assert_eq!(store.inspect(prepared.id()).unwrap().journal, before);
    adapter.bad_profile = false;
    for relation in [
        NativeEffectRelation::Before,
        NativeEffectRelation::Unknown,
        NativeEffectRelation::Diverged,
    ] {
        adapter.observation = Some(NativeEffectObservation {
            effect: relation,
            ..observation.clone()
        });
        assert!(
            !store
                .observe_pending_step(prepared.id(), index, &mut adapter)
                .unwrap()
        );
    }
    adapter.observation = Some(NativeEffectObservation {
        effect: NativeEffectRelation::After,
        ..observation
    });
    assert!(
        store
            .observe_pending_step(prepared.id(), index, &mut adapter)
            .unwrap()
    );
    let journal = store.inspect(prepared.id()).unwrap().journal;
    assert_eq!(
        journal.checkpoints[index]
            .attempts
            .iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![
            NativeBindingOutcome::Confirmed,
            NativeBindingOutcome::Unknown
        ]
    );
    assert_eq!(journal.checkpoints[index].observations.len(), 5);
    // Effect acknowledgement cannot claim an effective binding or terminal state.
    assert!(!primary.path.join(".upskill-lock.json").exists());
    assert_eq!(journal.status, NativeJournalStatus::Executing);
    assert_eq!(adapter.calls, 0);
}
