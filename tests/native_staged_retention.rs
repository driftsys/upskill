//! Staging ownership is independent of native activation or detachment evidence.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/native_delivery.rs"]
mod fixture;
use upskill::files::plan::{FileLocation, FileValue, ObservedFile};
use upskill::lifecycle::native_receipt::{
    NativeBindingOutcome, NativeBindingReceipt, NativeResourceRetention, NativeSkillSelection,
};
use upskill::lifecycle::reconcile::GraphChanges;
use upskill::lifecycle::skill_delivery::{
    SkillDeliveryInput, SkillDeliveryState, StagedSkillRevision,
};
use upskill::lifecycle::state::CombinedState;
use upskill::native_delivery::plan_native_skill_files;
use upskill::native_delivery::state::{
    NativeDeliveryState, NativeStateContext, parse_native_state, serialize_native_state,
};
use upskill::ownership::{InstallationRoot, OwnershipScope};

fn context() -> NativeStateContext {
    NativeStateContext {
        scope: InstallationRoot {
            scope: OwnershipScope::Project,
            path: "/workspace/project".into(),
        },
        private_root: InstallationRoot {
            scope: OwnershipScope::User,
            path: "/home/fixture".into(),
        },
        namespace: "fixture-context".into(),
        additional_roots: vec![],
    }
}
fn empty(context: &NativeStateContext) -> SkillDeliveryState {
    SkillDeliveryState {
        scope: context.scope.clone(),
        graph: CombinedState {
            schema: 2,
            origin: None,
            installations: vec![],
            bundles: vec![],
            references: vec![],
            artifacts: vec![],
            owner_consumers: vec![],
            owner_contracts: vec![],
            legacy: None,
        },
        revisions: vec![],
        staged: vec![],
        receipts: vec![],
    }
}

#[test]
fn partial_unbound_exports_remain_owned_without_a_fabricated_detachment_receipt() {
    let context = context();
    let capture = fixture::capture(&context);
    let revision = capture.candidates[0].clone();
    assert!(revision.package.revision_files.len() > 1);
    let first = &revision.package.revision_files[0];
    let staged = StagedSkillRevision {
        revision: revision.clone(),
        operation_id: "a".repeat(32),
        paths: vec![first.path.clone()],
    };
    let mut current = empty(&context);
    current.staged.push(staged.clone());
    let observations = [ObservedFile {
        location: FileLocation {
            root: context.scope.clone(),
            path: first.path.clone(),
        },
        value: Some(FileValue {
            bytes: first.bytes.clone(),
            mode: Some(0o600),
        }),
    }];
    let changes = GraphChanges::default();
    let plan = plan_native_skill_files(&SkillDeliveryInput {
        current: &current,
        scope: &context.scope,
        graph_changes: &changes,
        intents: &[],
        candidates: &[],
        native_observations: &capture.native_observations,
        file_observations: &observations,
    })
    .unwrap();
    assert!(plan.revisions.is_empty());
    assert_eq!(plan.staged, vec![staged]);
    assert!(plan.native_changes.is_empty());
    assert!(plan.selector_files.is_empty());
    assert_eq!(plan.stage_files.len(), 1);
    assert_eq!(plan.stage_files[0].before, plan.stage_files[0].after);
    let state = NativeDeliveryState {
        context: context.clone(),
        skills: current.clone(),
        intents: vec![],
        files: plan.file_ownership,
        configurations: vec![],
    };
    let encoded = serialize_native_state(&state).unwrap();
    let json: serde_json::Value = serde_json::from_str(&encoded.json).unwrap();
    assert_eq!(json["schema"], 4);
    assert!(json["receipts"].as_array().unwrap().is_empty());
    assert_eq!(
        parse_native_state(&encoded.json, &context, &encoded.private_evidence).unwrap(),
        state
    );
    current.receipts.push(NativeBindingReceipt {
        key: capture.native_observations[0].key.clone(),
        scope: context.scope.clone(),
        client_version: "fixture-1".into(),
        platform: "fixture-os".into(),
        operation_id: "false-detachment".into(),
        before: Some(NativeSkillSelection::from_package(&revision.package)),
        after: None,
        captured_prestate: vec![],
        outcome: NativeBindingOutcome::Confirmed,
    });
    let forged = NativeDeliveryState {
        skills: current,
        ..state
    };
    assert!(serialize_native_state(&forged).is_err());
}

#[test]
fn a_retry_uses_real_partial_observations_and_promotes_only_the_planned_candidate() {
    let context = context();
    let mut capture = fixture::capture(&context);
    let revision = capture.candidates[0].clone();
    let first = &revision.package.revision_files[0];
    let mut current = empty(&context);
    current.staged.push(StagedSkillRevision {
        revision: revision.clone(),
        operation_id: "b".repeat(32),
        paths: vec![first.path.clone()],
    });
    for file in &mut capture.file_observations {
        if file.location.path == first.path {
            file.value = Some(FileValue {
                bytes: first.bytes.clone(),
                mode: Some(0o600),
            });
        }
    }
    let input = |observations| SkillDeliveryInput {
        current: &current,
        scope: &context.scope,
        graph_changes: &capture.graph_changes,
        intents: &capture.intents,
        candidates: &capture.candidates,
        native_observations: observations,
        file_observations: &capture.file_observations,
    };
    assert!(
        plan_native_skill_files(&input(&capture.native_observations)).is_err(),
        "retained staging bypassed cache/session clearance"
    );
    let mut observations = capture.native_observations.clone();
    observations[0].retention = NativeResourceRetention::CachePreserved {
        evidence_id: "fixture-retention".into(),
    };
    let plan = plan_native_skill_files(&input(&observations)).unwrap();
    assert_eq!(plan.revisions, vec![revision.clone()]);
    assert!(plan.staged.is_empty());
    let old = plan
        .stage_files
        .iter()
        .find(|file| file.location.path == first.path)
        .unwrap();
    assert_eq!(old.before, old.after);
    assert!(
        plan.stage_files
            .iter()
            .any(|file| file.before.is_none() && file.after.is_some())
    );
    assert_eq!(
        capture
            .file_observations
            .iter()
            .filter(|file| file.value.is_some())
            .count(),
        1
    );
}

#[test]
fn empty_duplicate_foreign_and_selector_staging_paths_are_rejected() {
    let context = context();
    let capture = fixture::capture(&context);
    let revision = capture.candidates[0].clone();
    let first = revision.package.revision_files[0].path.clone();
    for paths in [
        vec![],
        vec![first.clone(), first.clone()],
        vec!["../outside".into()],
        vec![
            revision
                .package
                .catalog_file
                .as_ref()
                .map_or_else(|| "not-an-export".into(), |file| file.path.clone()),
        ],
    ] {
        let mut current = empty(&context);
        current.staged.push(StagedSkillRevision {
            revision: revision.clone(),
            operation_id: "c".repeat(32),
            paths,
        });
        let changes = GraphChanges::default();
        assert!(
            plan_native_skill_files(&SkillDeliveryInput {
                current: &current,
                scope: &context.scope,
                graph_changes: &changes,
                intents: &[],
                candidates: &[],
                native_observations: &capture.native_observations,
                file_observations: &[]
            })
            .is_err()
        );
    }
}

#[test]
fn stage_origin_role_and_prior_wire_version_cannot_be_reinterpreted() {
    let context = context();
    let capture = fixture::capture(&context);
    let revision = capture.candidates[0].clone();
    let first = &revision.package.revision_files[0];
    let staged = StagedSkillRevision {
        revision: revision.clone(),
        operation_id: "d".repeat(32),
        paths: vec![first.path.clone()],
    };
    let mut current = empty(&context);
    current.staged.push(staged.clone());
    let observations = [ObservedFile {
        location: FileLocation {
            root: context.scope.clone(),
            path: first.path.clone(),
        },
        value: Some(FileValue {
            bytes: first.bytes.clone(),
            mode: Some(0o600),
        }),
    }];
    let changes = GraphChanges::default();
    let plan = plan_native_skill_files(&SkillDeliveryInput {
        current: &current,
        scope: &context.scope,
        graph_changes: &changes,
        intents: &[],
        candidates: &[],
        native_observations: &capture.native_observations,
        file_observations: &observations,
    })
    .unwrap();
    let state = NativeDeliveryState {
        context: context.clone(),
        skills: current.clone(),
        intents: vec![],
        files: plan.file_ownership,
        configurations: vec![],
    };
    let encoded = serialize_native_state(&state).unwrap();
    let mut old: serde_json::Value = serde_json::from_str(&encoded.json).unwrap();
    old["schema"] = 3.into();
    assert!(parse_native_state(&old.to_string(), &context, &encoded.private_evidence).is_err());
    for operation_id in [
        "".to_owned(),
        "../owner".into(),
        "F".repeat(32),
        "e".repeat(33),
    ] {
        let mut bad = state.clone();
        bad.skills.staged[0].operation_id = operation_id;
        assert!(serialize_native_state(&bad).is_err());
    }
    let mut duplicate = state.clone();
    duplicate.skills.staged.push(staged);
    assert!(serialize_native_state(&duplicate).is_err());
    let mut active = capture.native_observations.clone();
    active[0].state = upskill::lifecycle::native_receipt::NativeObservationState::Present(
        NativeSkillSelection::from_package(&revision.package),
    );
    assert!(
        plan_native_skill_files(&SkillDeliveryInput {
            current: &current,
            scope: &context.scope,
            graph_changes: &changes,
            intents: &[],
            candidates: &[],
            native_observations: &active,
            file_observations: &observations
        })
        .is_err(),
        "staging proof adopted an unowned native binding"
    );
    let mut bad = state.clone();
    if let upskill::ownership::ArtifactContent::File { mode, .. } = &mut bad.files[0].content {
        *mode = Some(0o644);
    }
    assert!(serialize_native_state(&bad).is_err());
}

#[test]
fn all_four_clients_keep_catalogs_out_of_partial_staging_ownership() {
    use upskill::target::TargetSurface;
    for surface in [
        TargetSurface::ClaudeCli,
        TargetSurface::CopilotCli,
        TargetSurface::CodexCli,
        TargetSurface::OpenCodeCli,
    ] {
        let context = context();
        let capture = fixture::capture_for_surface(&context, surface);
        let revision = &capture.candidates[0];
        let first = &revision.package.revision_files[0];
        let mut current = empty(&context);
        current.staged.push(StagedSkillRevision {
            revision: revision.clone(),
            operation_id: "e".repeat(32),
            paths: vec![first.path.clone()],
        });
        let observations = [ObservedFile {
            location: FileLocation {
                root: context.scope.clone(),
                path: first.path.clone(),
            },
            value: Some(FileValue {
                bytes: first.bytes.clone(),
                mode: Some(0o600),
            }),
        }];
        let changes = GraphChanges::default();
        let plan = plan_native_skill_files(&SkillDeliveryInput {
            current: &current,
            scope: &context.scope,
            graph_changes: &changes,
            intents: &[],
            candidates: &[],
            native_observations: &capture.native_observations,
            file_observations: &observations,
        })
        .unwrap();
        let state = NativeDeliveryState {
            context: context.clone(),
            skills: current,
            intents: vec![],
            files: plan.file_ownership,
            configurations: vec![],
        };
        let encoded = serialize_native_state(&state).unwrap();
        assert_eq!(
            parse_native_state(&encoded.json, &context, &encoded.private_evidence).unwrap(),
            state
        );
        if let Some(catalog) = &revision.package.catalog_file {
            let mut invalid = state.clone();
            invalid.skills.staged[0].paths = vec![catalog.path.clone()];
            assert!(serialize_native_state(&invalid).is_err());
        }
    }
}
