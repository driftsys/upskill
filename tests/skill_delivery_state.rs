//! End-to-end pure ownership and receipt planning over captured packages.
#[path = "common/scope_path.rs"]
mod scope_path;

use std::collections::BTreeMap;
use std::fs;
use upskill::compile::skill_package::{SkillPackageRequest, compile_skill_package};
use upskill::compile::skill_selection::{SkillBodyOrigin, SkillSelectionRequest, select_skill};
use upskill::files::plan::{FileLocation, FileValue, ObservedFile, PlannedFile};
use upskill::lifecycle::native_receipt::{
    NativeBindingOutcome, NativeBindingReceipt, NativeObservationState, NativeReceiptError,
    NativeResourceRetention, NativeSkillObservation, NativeSkillSelection, SkillTarget,
};
use upskill::lifecycle::reconcile::GraphChanges;
use upskill::lifecycle::skill_delivery::{
    PackageRevision, SkillDeliveryInput, SkillDeliveryIntent, SkillDeliveryState,
    SkillIntentSelection, plan_skill_delivery,
};
use upskill::lifecycle::state::{
    BundleNode, CombinedState, Installation, Reference, SourceEvidence, parse_state,
    serialize_state,
};
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::source_manifest::capture_source;
use upskill::target::{
    CapabilityCategory, CapabilityDeclaration, CapabilityKey, Representation, Requirement,
    TargetSurface,
};

fn root(path: &str) -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: scope_path::absolute(path),
    }
}
fn empty_graph() -> CombinedState {
    CombinedState {
        schema: 2,
        origin: None,
        installations: vec![],
        bundles: vec![],
        references: vec![],
        artifacts: vec![],
        owner_consumers: vec![],
        owner_contracts: vec![],
        legacy: None,
    }
}
fn empty(scope: &InstallationRoot) -> SkillDeliveryState {
    SkillDeliveryState {
        scope: scope.clone(),
        graph: empty_graph(),
        revisions: vec![],
        staged: vec![],
        receipts: vec![],
    }
}
fn key() -> SkillTarget {
    SkillTarget {
        installation_id: "demo-install".into(),
        surface: TargetSurface::ClaudeCli,
    }
}
fn direct() -> Reference {
    Reference::DirectInstallation {
        installation_id: "demo-install".into(),
    }
}
fn intent() -> SkillDeliveryIntent {
    SkillDeliveryIntent {
        key: key(),
        selection: SkillIntentSelection::Selected {
            body_origin: SkillBodyOrigin::Canonical,
        },
    }
}
fn revision(scope: &InstallationRoot, body: &str, override_body: Option<&str>) -> PackageRevision {
    revision_for(TargetSurface::ClaudeCli, scope, body, override_body)
}
fn revision_for(
    surface: TargetSurface,
    scope: &InstallationRoot,
    body: &str,
    override_body: Option<&str>,
) -> PackageRevision {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("SKILL.md"),
        format!("---\nschema: 1\ndescription: Demo.\n---\n\n{body}\n"),
    )
    .unwrap();
    fs::write(dir.path().join("resource.bin"), [0, 255, 1]).unwrap();
    if let Some(body) = override_body {
        fs::write(dir.path().join("SKILL.claude.md"), body).unwrap();
    }
    let source = capture_source(dir.path()).unwrap();
    let capabilities = ["name", "description"].map(|name| CapabilityDeclaration {
        surface,
        key: CapabilityKey::new(CapabilityCategory::Skills, name),
        representation: Representation::Native,
        requirement: Requirement::RequiredBehavior,
        reason: "offline fixture projection",
    });
    let selected = select_skill(&SkillSelectionRequest {
        source: &source,
        folder_name: "demo",
        alias: None,
        surface,
        capabilities: &capabilities,
    })
    .unwrap()
    .unwrap();
    let installation = Installation {
        owner: ArtifactOwner {
            installation_id: "demo-install".into(),
            source: "./source".into(),
            source_revision: None,
            kind: ItemKind::Skill,
            source_name: "demo".into(),
            installed_name: "demo".into(),
        },
        root: scope.clone(),
        requested: vec![surface],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: selected.source_manifest_sha256.clone(),
        },
    };
    let package = compile_skill_package(&SkillPackageRequest {
        selected: &selected,
        surface: selected.surface,
        scope,
        owner: &installation.owner,
    })
    .unwrap();
    PackageRevision {
        installation,
        selected,
        package,
    }
}
fn recompile(revision: &mut PackageRevision) {
    revision.package = compile_skill_package(&SkillPackageRequest {
        selected: &revision.selected,
        surface: revision.selected.surface,
        scope: &revision.installation.root,
        owner: &revision.installation.owner,
    })
    .unwrap();
}
// Captured native values are encoded independently from the producer projection.
fn observed_selection(
    package: &upskill::compile::skill_package::SkillPackage,
) -> NativeSkillSelection {
    NativeSkillSelection {
        native_name: package.native_name.clone(),
        payload_sha256: package.payload_sha256.clone(),
        package_sha256: package.package_sha256.clone(),
        revision_root: package.revision_root.clone(),
        binding: package.binding.clone(),
    }
}
fn assert_exact_file_plan(
    result: &upskill::lifecycle::skill_delivery::SkillDeliveryPlan,
    scope: &InstallationRoot,
    observations: &[ObservedFile],
    revisions: &[&PackageRevision],
    selectors: &[(&upskill::compile::item::CompiledItemFile, bool)],
) {
    let before: BTreeMap<_, _> = observations
        .iter()
        .map(|row| (row.location.path.clone(), row.value.clone()))
        .collect();
    let mut staging = BTreeMap::new();
    for revision in revisions {
        for file in &revision.package.revision_files {
            staging.insert(
                file.path.clone(),
                PlannedFile {
                    location: FileLocation {
                        root: scope.clone(),
                        path: file.path.clone(),
                    },
                    before: before[&file.path].clone(),
                    after: Some(FileValue {
                        bytes: file.bytes.clone(),
                        mode: file.mode,
                    }),
                },
            );
        }
    }
    let mut actual = result.stage_files.clone();
    actual.sort_by(|left, right| left.location.path.cmp(&right.location.path));
    assert_eq!(actual, staging.into_values().collect::<Vec<_>>());
    let mut expected: Vec<_> = selectors
        .iter()
        .map(|(file, keep)| PlannedFile {
            location: FileLocation {
                root: scope.clone(),
                path: file.path.clone(),
            },
            before: before[&file.path].clone(),
            after: keep.then(|| FileValue {
                bytes: file.bytes.clone(),
                mode: file.mode,
            }),
        })
        .collect();
    expected.sort_by(|left, right| left.location.path.cmp(&right.location.path));
    let mut actual = result.selector_files.clone();
    actual.sort_by(|left, right| left.location.path.cmp(&right.location.path));
    assert_eq!(actual, expected);
}
fn installed(revision: &PackageRevision) -> SkillDeliveryState {
    let mut state = empty(&revision.installation.root);
    state
        .graph
        .installations
        .push(revision.installation.clone());
    state.graph.references.push(direct());
    state.revisions.push(revision.clone());
    state.receipts.push(NativeBindingReceipt {
        key: SkillTarget {
            installation_id: revision.installation.owner.installation_id.clone(),
            surface: revision.selected.surface,
        },
        scope: state.scope.clone(),
        client_version: "fixture-1".into(),
        platform: "fixture-os".into(),
        operation_id: "install-1".into(),
        before: None,
        after: Some(observed_selection(&revision.package)),
        captured_prestate: b"absent".to_vec(),
        outcome: NativeBindingOutcome::Confirmed,
    });
    state
}
fn native(state: &SkillDeliveryState) -> NativeSkillObservation {
    let after = state
        .receipts
        .first()
        .and_then(|receipt| receipt.after.clone());
    NativeSkillObservation {
        key: state
            .receipts
            .first()
            .map_or_else(key, |receipt| receipt.key.clone()),
        scope: state.scope.clone(),
        client_version: "fixture-1".into(),
        platform: "fixture-os".into(),
        observation_id: "observation-1".into(),
        captured_prestate: b"captured-native-state".to_vec(),
        state: after.map_or(
            NativeObservationState::Absent,
            NativeObservationState::Present,
        ),
        retention: NativeResourceRetention::SessionsCleared {
            evidence_id: "clearance-1".into(),
        },
    }
}
fn file_observations(
    state: &SkillDeliveryState,
    candidates: &[PackageRevision],
) -> Vec<ObservedFile> {
    let mut files = BTreeMap::new();
    for revision in &state.revisions {
        for file in &revision.package.revision_files {
            files.insert(
                file.path.clone(),
                Some(FileValue {
                    bytes: file.bytes.clone(),
                    mode: file.mode,
                }),
            );
        }
        if state.receipts.iter().any(|receipt| {
            receipt.key.installation_id == revision.installation.owner.installation_id
                && receipt.key.surface == revision.selected.surface
                && receipt.after.as_ref().is_some_and(|selected| {
                    selected.payload_sha256 == revision.package.payload_sha256
                })
        }) {
            if let Some(file) = &revision.package.catalog_file {
                files.insert(
                    file.path.clone(),
                    Some(FileValue {
                        bytes: file.bytes.clone(),
                        mode: file.mode,
                    }),
                );
            }
        }
    }
    for revision in candidates {
        for file in revision
            .package
            .revision_files
            .iter()
            .chain(revision.package.catalog_file.iter())
        {
            files.entry(file.path.clone()).or_insert(None);
        }
    }
    files
        .into_iter()
        .map(|(path, value)| ObservedFile {
            location: FileLocation {
                root: state.scope.clone(),
                path,
            },
            value,
        })
        .collect()
}
fn plan<'a>(
    state: &'a SkillDeliveryState,
    changes: &'a GraphChanges,
    intents: &'a [SkillDeliveryIntent],
    candidates: &'a [PackageRevision],
    native: &'a [NativeSkillObservation],
    files: &'a [ObservedFile],
) -> SkillDeliveryInput<'a> {
    SkillDeliveryInput {
        current: state,
        scope: &state.scope,
        graph_changes: changes,
        intents,
        candidates,
        native_observations: native,
        file_observations: files,
    }
}
fn add(revision: &PackageRevision) -> GraphChanges {
    GraphChanges {
        installations: vec![revision.installation.clone()],
        add_references: vec![direct()],
        ..Default::default()
    }
}

#[test]
fn fresh_install_requires_absence_and_stages_a_single_selected_revision() {
    let scope = root("/checkout");
    let state = empty(&scope);
    let rev = revision(&scope, "Body", None);
    let changes = add(&rev);
    let candidates = vec![rev.clone()];
    let intents = vec![intent()];
    let mut observation = native(&state);
    observation.retention = NativeResourceRetention::Unknown;
    let before = state.clone();
    let files = file_observations(&state, &candidates);
    let result = plan_skill_delivery(&plan(
        &state,
        &changes,
        &intents,
        &candidates,
        &[observation],
        &files,
    ))
    .unwrap();
    assert_exact_file_plan(
        &result,
        &scope,
        &files,
        &[&rev],
        &[(rev.package.catalog_file.as_ref().unwrap(), true)],
    );
    assert_eq!(
        result.native_changes[0].after,
        Some(observed_selection(&rev.package))
    );
    assert_eq!(
        result.native_changes[0].captured_prestate,
        b"captured-native-state"
    );
    assert_eq!(result.native_changes[0].observation_id, "observation-1");
    assert_eq!(result.native_changes[0].client_version, "fixture-1");
    assert_eq!(result.native_changes[0].platform, "fixture-os");
    assert_eq!(result.native_changes[0].scope, scope);
    assert_eq!(
        result.native_changes[0].retention,
        NativeResourceRetention::Unknown
    );
    assert_eq!(result.graph.installations, [rev.installation]);
    assert_eq!(result.revisions.len(), 1);
    assert_eq!(result.native_changes.len(), 1);
    assert!(result.native_changes[0].before.is_none());
    assert_eq!(
        result.native_changes[0]
            .after
            .as_ref()
            .unwrap()
            .payload_sha256,
        rev.package.payload_sha256
    );
    assert!(
        result
            .stage_files
            .iter()
            .all(|file| file.before.is_none() && file.after.is_some())
    );
    assert_eq!(result.selector_files.len(), 1);
    assert!(result.selector_files[0].after.is_some());
    assert_eq!(state, before);
}

#[test]
fn update_retains_old_revision_and_separates_selector_from_inactive_staging() {
    let scope = root("/checkout");
    let old = revision(&scope, "Old", None);
    let state = installed(&old);
    let new = revision(&scope, "New", None);
    let changes = GraphChanges {
        installations: vec![new.installation.clone()],
        ..Default::default()
    };
    let candidates = vec![new.clone()];
    let files = file_observations(&state, &candidates);
    let result = plan_skill_delivery(&plan(
        &state,
        &changes,
        &[intent()],
        &candidates,
        &[native(&state)],
        &files,
    ))
    .unwrap();
    assert_exact_file_plan(
        &result,
        &scope,
        &files,
        &[&old, &new],
        &[(new.package.catalog_file.as_ref().unwrap(), true)],
    );
    assert_eq!(
        result.native_changes[0].retention,
        NativeResourceRetention::SessionsCleared {
            evidence_id: "clearance-1".into()
        }
    );
    assert_eq!(result.revisions.len(), 2);
    assert!(result.revisions.contains(&old));
    assert!(result.revisions.contains(&new));
    assert_eq!(
        result.native_changes[0]
            .before
            .as_ref()
            .unwrap()
            .payload_sha256,
        old.package.payload_sha256
    );
    assert_eq!(
        result.native_changes[0]
            .after
            .as_ref()
            .unwrap()
            .payload_sha256,
        new.package.payload_sha256
    );
    assert!(result.stage_files.iter().all(|file| file.after.is_some()));
    assert_eq!(result.selector_files.len(), 1);
    assert_ne!(
        result.selector_files[0].before,
        result.selector_files[0].after
    );
    assert!(
        result
            .file_ownership
            .iter()
            .any(|row| row.path.starts_with(&old.package.revision_root))
    );
}

fn bundle(id: &str) -> BundleNode {
    BundleNode {
        id: id.into(),
        source: "./bundles".into(),
        name: id.into(),
        source_revision: None,
    }
}
fn root_bundle(id: &str) -> Reference {
    Reference::DirectBundle {
        bundle_id: id.into(),
    }
}
fn member(id: &str) -> Reference {
    Reference::BundleInstallation {
        bundle_id: id.into(),
        installation_id: "demo-install".into(),
    }
}
fn diamond(revision: &PackageRevision) -> SkillDeliveryState {
    let mut state = installed(revision);
    state.graph.bundles = vec![bundle("left"), bundle("right"), bundle("dependency")];
    state.graph.references = vec![
        root_bundle("left"),
        root_bundle("right"),
        Reference::BundleDependency {
            bundle_id: "left".into(),
            dependency_id: "dependency".into(),
        },
        Reference::BundleDependency {
            bundle_id: "right".into(),
            dependency_id: "dependency".into(),
        },
        member("dependency"),
    ];
    state
}

#[test]
fn diamond_and_direct_references_retain_one_binding_until_final_root_removal() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let mut state = diamond(&rev);
    let changes = GraphChanges {
        remove_references: vec![root_bundle("left")],
        ..Default::default()
    };
    let files = file_observations(&state, std::slice::from_ref(&rev));
    let result = plan_skill_delivery(&plan(
        &state,
        &changes,
        &[intent()],
        std::slice::from_ref(&rev),
        &[native(&state)],
        &files,
    ))
    .unwrap();
    assert_eq!(result.revisions.len(), 1);
    assert!(result.native_changes.is_empty());
    assert_eq!(result.graph.installations.len(), 1);
    state.graph.references.push(direct());
    let changes = GraphChanges {
        remove_references: vec![root_bundle("left"), root_bundle("right")],
        ..Default::default()
    };
    let result = plan_skill_delivery(&plan(
        &state,
        &changes,
        &[intent()],
        std::slice::from_ref(&rev),
        &[native(&state)],
        &files,
    ))
    .unwrap();
    assert_eq!(result.graph.references, [direct()]);
    assert!(result.native_changes.is_empty());
}

#[test]
fn final_reference_removal_detaches_without_deleting_session_resources() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let state = installed(&rev);
    let changes = GraphChanges {
        remove_references: vec![direct()],
        ..Default::default()
    };
    let files = file_observations(&state, &[]);
    let result =
        plan_skill_delivery(&plan(&state, &changes, &[], &[], &[native(&state)], &files)).unwrap();
    assert!(result.graph.installations.is_empty());
    assert_exact_file_plan(
        &result,
        &scope,
        &files,
        &[&rev],
        &[(rev.package.catalog_file.as_ref().unwrap(), false)],
    );
    assert_eq!(result.revisions, [rev]);
    assert_eq!(result.native_changes.len(), 1);
    assert!(result.native_changes[0].after.is_none());
    assert!(result.stage_files.iter().all(|file| file.after.is_some()));
    assert_eq!(result.selector_files.len(), 1);
    assert!(result.selector_files[0].after.is_none());
}

#[test]
fn replacement_removal_requires_unavailability_or_an_explicit_new_selection() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Canonical", Some("Replacement"));
    let state = installed(&rev);
    let unavailable = SkillDeliveryIntent {
        key: key(),
        selection: SkillIntentSelection::Unavailable,
    };
    let files = file_observations(&state, &[]);
    let result = plan_skill_delivery(&plan(
        &state,
        &GraphChanges::default(),
        std::slice::from_ref(&unavailable),
        &[],
        &[native(&state)],
        &files,
    ))
    .unwrap();
    assert_eq!(result.intents, [unavailable]);
    assert_eq!(result.revisions.len(), 1);
    assert!(result.native_changes[0].after.is_none());
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &GraphChanges::default(),
            &[intent()],
            &[],
            &[native(&state)],
            &files
        ))
        .is_err()
    );
}

#[test]
fn conflicting_revisions_intents_and_aliases_reject_instead_of_selecting_a_winner() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let state = installed(&rev);
    let other = revision(&scope, "Other", None);
    let files = file_observations(&state, &[rev.clone(), other.clone()]);
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &GraphChanges::default(),
            &[intent()],
            &[rev.clone(), other],
            &[native(&state)],
            &files
        ))
        .is_err()
    );
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &GraphChanges::default(),
            &[intent(), intent()],
            std::slice::from_ref(&rev),
            &[native(&state)],
            &files
        ))
        .is_err()
    );
    let mut aliased = rev.clone();
    aliased.installation.owner.installed_name = "different".into();
    let changes = GraphChanges {
        installations: vec![aliased.installation.clone()],
        ..Default::default()
    };
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &changes,
            &[intent()],
            &[aliased],
            &[native(&state)],
            &files
        ))
        .is_err()
    );
}

#[test]
fn missing_or_edited_owned_files_reject_even_for_a_noop() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let state = installed(&rev);
    let files = file_observations(&state, std::slice::from_ref(&rev));
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &GraphChanges::default(),
            &[intent()],
            std::slice::from_ref(&rev),
            &[native(&state)],
            &files
        ))
        .is_ok()
    );
    for replacement in [
        None,
        Some(FileValue {
            bytes: b"manual edit".to_vec(),
            mode: None,
        }),
    ] {
        let mut changed = files.clone();
        changed[0].value = replacement;
        assert!(
            plan_skill_delivery(&plan(
                &state,
                &GraphChanges::default(),
                &[intent()],
                std::slice::from_ref(&rev),
                &[native(&state)],
                &changed
            ))
            .is_err()
        );
    }
}

#[test]
fn unknown_rejected_or_contradicted_native_state_never_produces_a_new_plan() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let state = installed(&rev);
    let files = file_observations(&state, std::slice::from_ref(&rev));
    for outcome in [
        NativeBindingOutcome::Unknown,
        NativeBindingOutcome::Rejected,
    ] {
        let mut changed = state.clone();
        changed.receipts[0].outcome = outcome;
        let error = plan_skill_delivery(&plan(
            &changed,
            &GraphChanges::default(),
            &[intent()],
            std::slice::from_ref(&rev),
            &[native(&state)],
            &files,
        ))
        .unwrap_err();
        assert!(
            matches!(
                error.downcast_ref::<NativeReceiptError>(),
                Some(NativeReceiptError::RecoveryRequired { .. })
            ),
            "{error:#}"
        );
    }
    let mut observation = native(&state);
    observation.state = NativeObservationState::Unknown;
    let error = plan_skill_delivery(&plan(
        &state,
        &GraphChanges::default(),
        &[intent()],
        std::slice::from_ref(&rev),
        &[observation],
        &files,
    ))
    .unwrap_err();
    assert!(error.downcast_ref::<NativeReceiptError>().is_some());
    let mut observation = native(&state);
    observation.state = NativeObservationState::Absent;
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &GraphChanges::default(),
            &[intent()],
            std::slice::from_ref(&rev),
            &[observation],
            &files
        ))
        .is_err()
    );
}

#[test]
fn native_refresh_and_detachment_require_cache_retention_or_session_clearance() {
    let scope = root("/checkout");
    let old = revision(&scope, "Old", None);
    let state = installed(&old);
    let new = revision(&scope, "New", None);
    let changes = GraphChanges {
        installations: vec![new.installation.clone()],
        ..Default::default()
    };
    let files = file_observations(&state, std::slice::from_ref(&new));
    let mut observation = native(&state);
    observation.retention = NativeResourceRetention::Unknown;
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &changes,
            &[intent()],
            std::slice::from_ref(&new),
            &[observation.clone()],
            &files
        ))
        .is_err()
    );
    let remove = GraphChanges {
        remove_references: vec![direct()],
        ..Default::default()
    };
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &remove,
            &[],
            &[],
            &[observation.clone()],
            &file_observations(&state, &[])
        ))
        .is_err()
    );
    observation.retention = NativeResourceRetention::CachePreserved {
        evidence_id: "cache-receipt".into(),
    };
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &changes,
            &[intent()],
            std::slice::from_ref(&new),
            &[observation],
            &files
        ))
        .is_ok()
    );
}

#[test]
fn copied_absolute_state_rejects_but_explicit_clone_materialization_uses_new_scope() {
    let old_scope = root("/old-checkout");
    let rev = revision(&old_scope, "Body", None);
    let current = installed(&rev);
    let new_scope = root("/new-checkout");
    let changes = GraphChanges::default();
    let intents = vec![intent()];
    let candidates = vec![rev.clone()];
    let observations = vec![native(&current)];
    let files = file_observations(&current, &candidates);
    let mut input = plan(
        &current,
        &changes,
        &intents,
        &candidates,
        &observations,
        &files,
    );
    input.scope = &new_scope;
    assert!(plan_skill_delivery(&input).is_err());
    let fresh = empty(&new_scope);
    let new = revision(&new_scope, "Body", None);
    let changes = add(&new);
    let result = plan_skill_delivery(&plan(
        &fresh,
        &changes,
        &intents,
        std::slice::from_ref(&new),
        &[native(&fresh)],
        &file_observations(&fresh, std::slice::from_ref(&new)),
    ))
    .unwrap();
    assert_eq!(result.graph.installations[0].root, new_scope);
    assert_eq!(result.revisions.len(), 1);
    assert_eq!(result.revisions[0].package, new.package);
    assert_ne!(new.package.native_name, rev.package.native_name);
}

#[test]
fn detached_capture_prevents_installation_id_reuse_for_another_source() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let mut current = installed(&rev);
    current.graph = empty_graph();
    current.receipts[0].before = current.receipts[0].after.take();
    let mut new = revision(&scope, "Body", None);
    new.installation.owner.source = "./another-source".into();
    recompile(&mut new);
    let changes = add(&new);
    let files = file_observations(&current, std::slice::from_ref(&new));
    assert!(
        plan_skill_delivery(&plan(
            &current,
            &changes,
            &[intent()],
            std::slice::from_ref(&new),
            &[native(&current)],
            &files
        ))
        .is_err()
    );
}

#[test]
fn prior_graph_codec_round_trip_and_input_values_remain_unchanged() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let state = installed(&rev);
    let before = state.clone();
    let encoded = serialize_state(&state.graph).unwrap();
    assert_eq!(parse_state(&encoded).unwrap(), state.graph);
    let files = file_observations(&state, std::slice::from_ref(&rev));
    plan_skill_delivery(&plan(
        &state,
        &GraphChanges::default(),
        &[intent()],
        std::slice::from_ref(&rev),
        &[native(&state)],
        &files,
    ))
    .unwrap();
    assert_eq!(state, before);
    assert_eq!(serialize_state(&state.graph).unwrap(), encoded);
}

fn git_revision(scope: &InstallationRoot, body: &str, resolved: &str) -> PackageRevision {
    let mut revision = revision(scope, body, None);
    revision.installation.owner.source = "https://example.invalid/skills.git".into();
    revision.installation.owner.source_revision = Some(resolved.into());
    revision.installation.source_evidence = SourceEvidence::Git {
        resolved_revision: resolved.into(),
        manifest_sha256: revision.selected.source_manifest_sha256.clone(),
    };
    recompile(&mut revision);
    revision
}

#[test]
fn detached_git_revisions_keep_capture_provenance_without_conflicting_current_owners() {
    let scope = root("/checkout");
    let old = git_revision(&scope, "Old", "r1");
    let new = git_revision(&scope, "New", "r2");
    let mut state = installed(&new);
    state.revisions.insert(0, old.clone());
    state.graph = empty_graph();
    state.receipts[0].before = state.receipts[0].after.take();
    let result = plan_skill_delivery(&plan(
        &state,
        &GraphChanges::default(),
        &[],
        &[],
        &[native(&state)],
        &file_observations(&state, &[]),
    ))
    .unwrap();
    assert!(result.native_changes.is_empty());
    assert_eq!(result.revisions.len(), 2);
    assert!(result.revisions.contains(&old));
    assert!(result.revisions.contains(&new));
    assert!(result.file_ownership.iter().all(|row| {
        row.owners
            .iter()
            .all(|owner| owner.source_revision.is_none())
    }));
    assert!(
        result
            .stage_files
            .iter()
            .all(|file| file.before == file.after)
    );
}

#[test]
fn all_four_cli_surfaces_use_their_own_binding_and_selector_shape() {
    for surface in [
        TargetSurface::ClaudeCli,
        TargetSurface::CopilotCli,
        TargetSurface::CodexCli,
        TargetSurface::OpenCodeCli,
    ] {
        let scope = root("/checkout");
        let state = empty(&scope);
        let candidate = revision_for(surface, &scope, "Body", None);
        let mut desired = intent();
        desired.key.surface = surface;
        let mut observation = native(&state);
        observation.key.surface = surface;
        let result = plan_skill_delivery(&plan(
            &state,
            &add(&candidate),
            &[desired],
            std::slice::from_ref(&candidate),
            &[observation],
            &file_observations(&state, std::slice::from_ref(&candidate)),
        ))
        .unwrap();
        assert_eq!(result.native_changes.len(), 1);
        assert_eq!(result.native_changes[0].key.surface, surface);
        assert_eq!(
            result.native_changes[0].after.as_ref().unwrap().binding,
            candidate.package.binding
        );
        assert_eq!(
            result.selector_files.len(),
            usize::from(surface != TargetSurface::OpenCodeCli)
        );
    }
}

#[test]
fn native_attribution_duplicates_and_foreign_absent_state_are_not_accepted() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let state = installed(&rev);
    let files = file_observations(&state, std::slice::from_ref(&rev));
    let observations = vec![native(&state), native(&state)];
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &GraphChanges::default(),
            &[intent()],
            std::slice::from_ref(&rev),
            &observations,
            &files
        ))
        .is_err()
    );
    let mut changed = state.clone();
    changed.receipts.push(changed.receipts[0].clone());
    assert!(
        plan_skill_delivery(&plan(
            &changed,
            &GraphChanges::default(),
            &[intent()],
            std::slice::from_ref(&rev),
            &[native(&state)],
            &files
        ))
        .is_err()
    );
    for field in ["scope", "version", "platform", "identity", "clearance"] {
        let mut observation = native(&state);
        match field {
            "scope" => observation.scope = root("/elsewhere"),
            "version" => observation.client_version = "another-version".into(),
            "platform" => observation.platform = "another-os".into(),
            "identity" => observation.observation_id.clear(),
            _ => {
                observation.retention = NativeResourceRetention::SessionsCleared {
                    evidence_id: String::new(),
                }
            }
        }
        assert!(
            plan_skill_delivery(&plan(
                &state,
                &GraphChanges::default(),
                &[intent()],
                std::slice::from_ref(&rev),
                &[observation],
                &files
            ))
            .is_err(),
            "{field}"
        );
    }
    let fresh = empty(&scope);
    let observation = native(&state);
    assert!(
        plan_skill_delivery(&plan(
            &fresh,
            &add(&rev),
            &[intent()],
            std::slice::from_ref(&rev),
            &[observation],
            &file_observations(&fresh, std::slice::from_ref(&rev))
        ))
        .is_err()
    );
}

#[test]
fn identical_unowned_files_are_not_adopted_and_unknown_retention_does_not_block_a_noop() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let state = installed(&rev);
    let mut observation = native(&state);
    observation.retention = NativeResourceRetention::Unknown;
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &GraphChanges::default(),
            &[intent()],
            std::slice::from_ref(&rev),
            &[observation],
            &file_observations(&state, std::slice::from_ref(&rev))
        ))
        .is_ok()
    );
    let fresh = empty(&scope);
    let mut files = file_observations(&fresh, std::slice::from_ref(&rev));
    let file = rev
        .package
        .revision_files
        .iter()
        .chain(&rev.package.catalog_file)
        .find(|file| file.path == files[0].location.path)
        .unwrap();
    files[0].value = Some(FileValue {
        bytes: file.bytes.clone(),
        mode: file.mode,
    });
    assert!(
        plan_skill_delivery(&plan(
            &fresh,
            &add(&rev),
            &[intent()],
            std::slice::from_ref(&rev),
            &[native(&fresh)],
            &files
        ))
        .is_err()
    );
}

#[test]
fn detached_reinstallation_needs_clearance_even_with_an_absent_native_binding() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let mut state = installed(&rev);
    state.graph = empty_graph();
    state.receipts[0].before = state.receipts[0].after.take();
    let mut observation = native(&state);
    observation.retention = NativeResourceRetention::Unknown;
    let files = file_observations(&state, std::slice::from_ref(&rev));
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &add(&rev),
            &[intent()],
            std::slice::from_ref(&rev),
            &[observation],
            &files
        ))
        .is_err()
    );
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &add(&rev),
            &[intent()],
            std::slice::from_ref(&rev),
            &[native(&state)],
            &files
        ))
        .is_ok()
    );
}

#[test]
fn captured_source_evidence_and_recompiled_package_must_match() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let state = empty(&scope);
    let mut changed = rev.clone();
    changed.installation.source_evidence = SourceEvidence::Local {
        manifest_sha256: "a".repeat(64),
    };
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &add(&changed),
            &[intent()],
            std::slice::from_ref(&changed),
            &[native(&state)],
            &file_observations(&state, std::slice::from_ref(&changed))
        ))
        .is_err()
    );
    let mut changed = rev.clone();
    changed
        .package
        .catalog_file
        .as_mut()
        .unwrap()
        .bytes
        .push(b' ');
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &add(&changed),
            &[intent()],
            std::slice::from_ref(&changed),
            &[native(&state)],
            &file_observations(&state, std::slice::from_ref(&changed))
        ))
        .is_err()
    );
}

#[test]
fn one_logical_installation_can_stage_all_clients_and_detach_one_independently() {
    let scope = root("/checkout");
    let fresh = empty(&scope);
    let surfaces = vec![
        TargetSurface::ClaudeCli,
        TargetSurface::CopilotCli,
        TargetSurface::CodexCli,
        TargetSurface::OpenCodeCli,
    ];
    let mut revisions: Vec<_> = surfaces
        .iter()
        .map(|surface| revision_for(*surface, &scope, "Body", None))
        .collect();
    for revision in &mut revisions {
        revision.installation.requested = surfaces.clone();
        recompile(revision);
    }
    let installation = revisions[0].installation.clone();
    assert!(
        revisions
            .iter()
            .all(|revision| revision.installation == installation)
    );
    let intents: Vec<_> = surfaces
        .iter()
        .map(|surface| {
            let mut desired = intent();
            desired.key.surface = *surface;
            desired
        })
        .collect();
    let observations: Vec<_> = surfaces
        .iter()
        .map(|surface| {
            let mut observed = native(&fresh);
            observed.key.surface = *surface;
            observed
        })
        .collect();
    let result = plan_skill_delivery(&plan(
        &fresh,
        &add(&revisions[0]),
        &intents,
        &revisions,
        &observations,
        &file_observations(&fresh, &revisions),
    ))
    .unwrap();
    assert_eq!(
        result.graph.installations.as_slice(),
        std::slice::from_ref(&installation)
    );
    let selector_expectations: Vec<_> = revisions
        .iter()
        .filter_map(|revision| {
            revision
                .package
                .catalog_file
                .as_ref()
                .map(|file| (file, true))
        })
        .collect();
    assert_exact_file_plan(
        &result,
        &scope,
        &file_observations(&fresh, &revisions),
        &revisions.iter().collect::<Vec<_>>(),
        &selector_expectations,
    );
    assert_eq!(result.revisions.len(), 4);
    assert_eq!(result.native_changes.len(), 4);
    assert_eq!(result.selector_files.len(), 3);
    assert!(
        result
            .file_ownership
            .iter()
            .all(|file| file.owners == [installation.owner.clone()])
    );

    let mut current = empty(&scope);
    current.graph.installations = vec![installation.clone()];
    current.graph.references = vec![direct()];
    current.revisions = revisions.clone();
    current.receipts = revisions
        .iter()
        .map(|revision| NativeBindingReceipt {
            key: SkillTarget {
                installation_id: installation.owner.installation_id.clone(),
                surface: revision.selected.surface,
            },
            scope: scope.clone(),
            client_version: "fixture-1".into(),
            platform: "fixture-os".into(),
            operation_id: "install-all".into(),
            before: None,
            after: Some(observed_selection(&revision.package)),
            captured_prestate: vec![],
            outcome: NativeBindingOutcome::Confirmed,
        })
        .collect();
    let mut desired_installation = installation;
    desired_installation.requested.remove(0);
    let mut candidates: Vec<_> = revisions
        .into_iter()
        .filter(|revision| revision.selected.surface != TargetSurface::ClaudeCli)
        .collect();
    for candidate in &mut candidates {
        candidate.installation = desired_installation.clone();
    }
    let desired_intents: Vec<_> = intents
        .into_iter()
        .filter(|intent| intent.key.surface != TargetSurface::ClaudeCli)
        .collect();
    let observations: Vec<_> = current
        .receipts
        .iter()
        .map(|receipt| {
            let mut observation = native(&current);
            observation.key = receipt.key.clone();
            observation.state = NativeObservationState::Present(receipt.after.clone().unwrap());
            observation
        })
        .collect();
    let changes = GraphChanges {
        installations: vec![desired_installation],
        ..Default::default()
    };
    let result = plan_skill_delivery(&plan(
        &current,
        &changes,
        &desired_intents,
        &candidates,
        &observations,
        &file_observations(&current, &candidates),
    ))
    .unwrap();
    assert_eq!(result.revisions.len(), 4);
    assert_eq!(result.native_changes.len(), 1);
    assert_eq!(
        result.native_changes[0].key.surface,
        TargetSurface::ClaudeCli
    );
    assert!(result.native_changes[0].after.is_none());
    assert_eq!(
        result
            .selector_files
            .iter()
            .filter(|file| file.after.is_none())
            .count(),
        1
    );
    assert!(result.stage_files.iter().all(|file| file.after.is_some()));
}

#[test]
fn different_present_native_selection_rejects_each_identity_and_binding_mismatch() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let state = installed(&rev);
    let files = file_observations(&state, std::slice::from_ref(&rev));
    for field in ["name", "payload", "package", "revision", "binding"] {
        let mut observed = native(&state);
        let NativeObservationState::Present(selection) = &mut observed.state else {
            panic!("fixture must be present")
        };
        match field {
            "name" => selection.native_name = "foreign".into(),
            "payload" => selection.payload_sha256 = "a".repeat(64),
            "package" => selection.package_sha256 = "b".repeat(64),
            "revision" => selection.revision_root = "elsewhere".into(),
            _ => {
                selection.binding =
                    upskill::compile::skill_package::SkillPackageBinding::LocalMarketplace {
                        directory: "/foreign".into(),
                    }
            }
        }
        assert!(
            plan_skill_delivery(&plan(
                &state,
                &GraphChanges::default(),
                &[intent()],
                std::slice::from_ref(&rev),
                &[observed],
                &files
            ))
            .is_err(),
            "{field}"
        );
    }
}

#[test]
fn explicit_origin_must_match_a_valid_candidate_instead_of_falling_back() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Canonical", None);
    let state = empty(&scope);
    let files = file_observations(&state, std::slice::from_ref(&rev));
    let changes = add(&rev);
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &changes,
            &[intent()],
            std::slice::from_ref(&rev),
            &[native(&state)],
            &files
        ))
        .is_ok()
    );
    let wrong = SkillDeliveryIntent {
        key: key(),
        selection: SkillIntentSelection::Selected {
            body_origin: SkillBodyOrigin::Override {
                path: "SKILL.claude.md".into(),
            },
        },
    };
    assert!(
        plan_skill_delivery(&plan(
            &state,
            &changes,
            &[wrong],
            std::slice::from_ref(&rev),
            &[native(&state)],
            &files
        ))
        .is_err()
    );
}

#[test]
fn receipt_selections_must_match_captures_and_detached_captures_need_detachment_receipts() {
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let state = installed(&rev);
    let files = file_observations(&state, std::slice::from_ref(&rev));
    for slot in ["before", "after"] {
        let mut changed = state.clone();
        let mut foreign = observed_selection(&rev.package);
        foreign.package_sha256 = "a".repeat(64);
        if slot == "before" {
            changed.receipts[0].before = Some(foreign);
        } else {
            changed.receipts[0].after = Some(foreign);
        }
        let observations = vec![native(&changed)];
        assert!(
            plan_skill_delivery(&plan(
                &changed,
                &GraphChanges::default(),
                &[intent()],
                std::slice::from_ref(&rev),
                &observations,
                &files
            ))
            .is_err(),
            "{slot}"
        );
    }
    let mut detached = state.clone();
    detached.graph = empty_graph();
    let files = file_observations(&detached, &[]);
    assert!(
        plan_skill_delivery(&plan(
            &detached,
            &GraphChanges::default(),
            &[],
            &[],
            &[native(&detached)],
            &files
        ))
        .is_err()
    );
    // A confirmed no-op proves absence, not detachment of these captures.
    let mut no_detachment = detached.clone();
    no_detachment.receipts[0].before = None;
    no_detachment.receipts[0].after = None;
    let absent_files = file_observations(&no_detachment, &[]);
    let error = plan_skill_delivery(&plan(
        &no_detachment,
        &GraphChanges::default(),
        &[],
        &[],
        &[native(&no_detachment)],
        &absent_files,
    ))
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("detached capture"),
        "{error:#}"
    );
    detached.receipts.clear();
    assert!(
        plan_skill_delivery(&plan(
            &detached,
            &GraphChanges::default(),
            &[],
            &[],
            &[native(&detached)],
            &files
        ))
        .is_err()
    );
}

#[test]
fn graph_skill_artifacts_cannot_duplicate_revision_ownership() {
    use sha2::{Digest, Sha256};
    use upskill::lifecycle::state::OwnerConsumers;
    use upskill::ownership::{ArtifactContent, OwnedArtifact};
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let mut state = installed(&rev);
    let file = rev
        .package
        .revision_files
        .iter()
        .find(|file| file.path.ends_with("/SKILL.md"))
        .unwrap();
    let direct_path = ".claude/skills/demo/SKILL.md".to_string();
    state.graph.artifacts.push(OwnedArtifact {
        root: scope.clone(),
        path: direct_path.clone(),
        content: ArtifactContent::File {
            content_sha256: Sha256::digest(&file.bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            mode: file.mode,
        },
        owners: vec![rev.installation.owner.clone()],
        consumers: vec![TargetSurface::ClaudeCli],
    });
    state.graph.owner_consumers.push(OwnerConsumers {
        root: scope,
        path: direct_path.clone(),
        contribution_id: None,
        installation_id: "demo-install".into(),
        consumers: vec![TargetSurface::ClaudeCli],
    });
    upskill::lifecycle::state::validate_state(&state.graph).unwrap();
    let mut observations = file_observations(&state, std::slice::from_ref(&rev));
    observations.push(ObservedFile {
        location: FileLocation {
            root: state.scope.clone(),
            path: direct_path,
        },
        value: Some(FileValue {
            bytes: file.bytes.clone(),
            mode: file.mode,
        }),
    });
    let error = plan_skill_delivery(&plan(
        &state,
        &GraphChanges::default(),
        &[intent()],
        std::slice::from_ref(&rev),
        &[native(&state)],
        &observations,
    ))
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("second ownership"),
        "{error:#}"
    );
}

#[test]
fn unrelated_owned_rule_artifacts_and_legacy_survive_and_cannot_be_retired_here() {
    use sha2::{Digest, Sha256};
    use upskill::lifecycle::state::OwnerConsumers;
    use upskill::ownership::{ArtifactContent, OwnedArtifact};
    let scope = root("/checkout");
    let rev = revision(&scope, "Body", None);
    let mut state = installed(&rev);
    let mut rule = rev.installation.clone();
    rule.owner.installation_id = "rule-install".into();
    rule.owner.kind = ItemKind::Rule;
    rule.owner.source_name = "rule".into();
    rule.owner.installed_name = "rule".into();
    state.graph.installations.push(rule.clone());
    let reference = Reference::DirectInstallation {
        installation_id: "rule-install".into(),
    };
    state.graph.references.push(reference.clone());
    let artifact = OwnedArtifact {
        root: scope.clone(),
        path: ".claude/rules/manual-safe.md".into(),
        content: ArtifactContent::File {
            content_sha256: Sha256::digest(b"Rule\n")
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            mode: Some(0o644),
        },
        owners: vec![rule.owner.clone()],
        consumers: vec![TargetSurface::ClaudeCli],
    };
    state.graph.artifacts.push(artifact.clone());
    state.graph.owner_consumers.push(OwnerConsumers {
        root: scope,
        path: artifact.path.clone(),
        contribution_id: None,
        installation_id: "rule-install".into(),
        consumers: vec![TargetSurface::ClaudeCli],
    });
    state.graph.legacy = Some(
        serde_json::from_str(r#"{"schema":1,"items":[],"bundles":[],"plugins":[],"mcps":[]}"#)
            .unwrap(),
    );
    let before = state.clone();
    let files = file_observations(&state, std::slice::from_ref(&rev));
    let result = plan_skill_delivery(&plan(
        &state,
        &GraphChanges::default(),
        &[intent()],
        std::slice::from_ref(&rev),
        &[native(&state)],
        &files,
    ))
    .unwrap();
    assert!(result.file_ownership.contains(&artifact));
    assert!(result.graph.installations.contains(&rule));
    assert_eq!(state, before);
    assert!(
        result
            .stage_files
            .iter()
            .chain(&result.selector_files)
            .all(|file| file.location.path != artifact.path)
    );
    let remove = GraphChanges {
        remove_references: vec![reference],
        ..Default::default()
    };
    let error = plan_skill_delivery(&plan(
        &state,
        &remove,
        &[intent()],
        std::slice::from_ref(&rev),
        &[native(&state)],
        &files,
    ))
    .unwrap_err();
    assert!(format!("{error:#}").contains("non-skill"), "{error:#}");
}
