//! Native delivery infrastructure acceptance; coordinator checkpoints follow
//! the explicit file-permission projection prerequisite.
#![cfg(unix)]
use std::fs;
use std::os::unix::fs::PermissionsExt;

use upskill::compile::skill_package::{SkillPackageRequest, compile_skill_package};
use upskill::compile::skill_selection::{SkillBodyOrigin, SkillSelectionRequest, select_skill};
use upskill::files::plan::{FileLocation, FileValue, ObservedFile};
use upskill::lifecycle::native_receipt::{
    NativeBindingOutcome, NativeBindingReceipt, NativeObservationState, NativeResourceRetention,
    NativeSkillObservation, NativeSkillSelection, SkillTarget,
};
use upskill::lifecycle::reconcile::GraphChanges;
use upskill::lifecycle::skill_delivery::{
    PackageRevision, SkillDeliveryInput, SkillDeliveryIntent, SkillDeliveryState,
    SkillIntentSelection, plan_skill_delivery,
};
use upskill::lifecycle::state::{CombinedState, Installation, Reference, SourceEvidence};
use upskill::native_delivery::plan_native_skill_files;
use upskill::ownership::{ArtifactContent, ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::source_manifest::capture_source;
use upskill::target::{
    CapabilityCategory, CapabilityDeclaration, CapabilityKey, Representation, Requirement,
    TargetSurface,
};

fn scope() -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: "/workspace/project".into(),
    }
}
fn key(surface: TargetSurface) -> SkillTarget {
    SkillTarget {
        installation_id: "demo-install".into(),
        surface,
    }
}
fn intent(surface: TargetSurface) -> SkillDeliveryIntent {
    SkillDeliveryIntent {
        key: key(surface),
        selection: SkillIntentSelection::Selected {
            body_origin: SkillBodyOrigin::Canonical,
        },
    }
}
fn empty() -> SkillDeliveryState {
    SkillDeliveryState {
        scope: scope(),
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
        receipts: vec![],
    }
}
fn revision(surface: TargetSurface, body: &str) -> PackageRevision {
    revision_with_alias(surface, body, None)
}
fn revision_with_alias(surface: TargetSurface, body: &str, alias: Option<&str>) -> PackageRevision {
    let source = tempfile::tempdir().unwrap();
    for (path, bytes, mode) in [
        (
            "SKILL.md",
            format!("---\nschema: 1\ndescription: Demo.\n---\n{body}\n").into_bytes(),
            0o755,
        ),
        ("data.bin", vec![0, 255, 1], 0o644),
        ("run.sh", b"#!/bin/sh\nexit 0\n".to_vec(), 0o755),
    ] {
        let path = source.path().join(path);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }
    let manifest = capture_source(source.path()).unwrap();
    let capabilities = ["name", "description"].map(|name| CapabilityDeclaration {
        surface,
        key: CapabilityKey::new(CapabilityCategory::Skills, name),
        representation: Representation::Native,
        requirement: Requirement::RequiredBehavior,
        reason: "offline fixture",
    });
    let selected = select_skill(&SkillSelectionRequest {
        source: &manifest,
        folder_name: "demo",
        alias,
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
            installed_name: selected.installed_name.clone(),
        },
        root: scope(),
        requested: vec![surface],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: selected.source_manifest_sha256.clone(),
        },
    };
    let package = compile_skill_package(&SkillPackageRequest {
        selected: &selected,
        surface,
        scope: &scope(),
        owner: &installation.owner,
    })
    .unwrap();
    PackageRevision {
        installation,
        selected,
        package,
    }
}
fn observation(
    surface: TargetSurface,
    selection: Option<&upskill::compile::skill_package::SkillPackage>,
    retention: NativeResourceRetention,
) -> NativeSkillObservation {
    NativeSkillObservation {
        key: key(surface),
        scope: scope(),
        client_version: "fixture-1".into(),
        platform: "fixture-os".into(),
        observation_id: "observe-1".into(),
        captured_prestate: b"native fixture".to_vec(),
        state: selection.map_or(NativeObservationState::Absent, |p| {
            NativeObservationState::Present(NativeSkillSelection {
                native_name: p.native_name.clone(),
                payload_sha256: p.payload_sha256.clone(),
                package_sha256: p.package_sha256.clone(),
                revision_root: p.revision_root.clone(),
                binding: p.binding.clone(),
            })
        }),
        retention,
    }
}
fn absent(revision: &PackageRevision) -> Vec<ObservedFile> {
    revision
        .package
        .revision_files
        .iter()
        .chain(revision.package.catalog_file.iter())
        .map(|f| ObservedFile {
            location: FileLocation {
                root: scope(),
                path: f.path.clone(),
            },
            value: None,
        })
        .collect()
}
fn installed(revision: &PackageRevision) -> SkillDeliveryState {
    let mut state = empty();
    state
        .graph
        .installations
        .push(revision.installation.clone());
    state.graph.references.push(Reference::DirectInstallation {
        installation_id: "demo-install".into(),
    });
    state.revisions.push(revision.clone());
    let NativeObservationState::Present(selection) = observation(
        revision.selected.surface,
        Some(&revision.package),
        NativeResourceRetention::Unknown,
    )
    .state
    else {
        unreachable!()
    };
    state.receipts.push(NativeBindingReceipt {
        key: key(revision.selected.surface),
        scope: scope(),
        client_version: "fixture-1".into(),
        platform: "fixture-os".into(),
        operation_id: "install-1".into(),
        before: None,
        after: Some(selection),
        captured_prestate: b"absent".to_vec(),
        outcome: NativeBindingOutcome::Confirmed,
    });
    state
}
// Literal destination modes are independent of the mode-selection implementation.
fn applied(revision: &PackageRevision) -> Vec<ObservedFile> {
    revision
        .package
        .revision_files
        .iter()
        .chain(revision.package.catalog_file.iter())
        .map(|f| ObservedFile {
            location: FileLocation {
                root: scope(),
                path: f.path.clone(),
            },
            value: Some(FileValue {
                bytes: f.bytes.clone(),
                mode: Some(if f.path.ends_with("/run.sh") {
                    0o700
                } else {
                    0o600
                }),
            }),
        })
        .collect()
}

#[test]
fn native_projection_selects_private_modes_without_changing_captured_packages() {
    for surface in [
        TargetSurface::ClaudeCli,
        TargetSurface::CopilotCli,
        TargetSurface::CodexCli,
        TargetSurface::OpenCodeCli,
    ] {
        let revision = revision_with_alias(surface, "Body.", Some("alternate"));
        let snapshot = revision.clone();
        let current = empty();
        let files = absent(&revision);
        let changes = GraphChanges {
            installations: vec![revision.installation.clone()],
            add_references: vec![Reference::DirectInstallation {
                installation_id: "demo-install".into(),
            }],
            ..Default::default()
        };
        let intents = [intent(surface)];
        let candidates = [revision];
        let natives = [observation(surface, None, NativeResourceRetention::Unknown)];
        let input = SkillDeliveryInput {
            current: &current,
            scope: &current.scope,
            graph_changes: &changes,
            intents: &intents,
            candidates: &candidates,
            native_observations: &natives,
            file_observations: &files,
        };
        let plan = plan_native_skill_files(&input).unwrap();
        assert_eq!(plan.revisions, vec![snapshot.clone()]);
        assert_eq!(plan.native_changes.len(), 1);
        assert_eq!(plan.native_changes[0].before, None);
        assert!(plan.native_changes[0].after.is_some());
        assert_eq!(
            plan.selector_files.len(),
            if surface == TargetSurface::OpenCodeCli {
                0
            } else {
                1
            }
        );
        assert_eq!(
            plan.file_ownership.len(),
            plan.stage_files.len() + plan.selector_files.len()
        );
        assert_eq!(
            plan.stage_files.len(),
            snapshot.package.revision_files.len()
        );
        for file in plan.stage_files.iter().chain(&plan.selector_files) {
            let source = snapshot
                .package
                .revision_files
                .iter()
                .chain(snapshot.package.catalog_file.iter())
                .find(|f| f.path == file.location.path)
                .unwrap();
            assert_eq!(file.before, None);
            let value = file.after.as_ref().unwrap();
            assert_eq!(value.bytes, source.bytes);
            let expected = if source.path.ends_with("/run.sh") {
                0o700
            } else {
                0o600
            };
            assert_eq!(value.mode, Some(expected));
            let ownership = plan
                .file_ownership
                .iter()
                .find(|a| a.path == source.path)
                .unwrap();
            assert!(
                matches!(ownership.content,ArtifactContent::File{mode:Some(mode),..} if mode==expected)
            );
        }
        let captured = plan_skill_delivery(&input).unwrap();
        assert_eq!(
            captured
                .stage_files
                .iter()
                .find(|f| f.location.path.ends_with("/run.sh"))
                .unwrap()
                .after
                .as_ref()
                .unwrap()
                .mode,
            Some(0o755)
        );
        assert_eq!(candidates[0], snapshot);
    }
}

#[test]
fn native_no_op_accepts_real_private_modes_and_rejects_mode_drift() {
    let revision = revision(TargetSurface::ClaudeCli, "Body.");
    let current = installed(&revision);
    let files = applied(&revision);
    let changes = GraphChanges::default();
    let intents = [intent(revision.selected.surface)];
    let candidates = [revision.clone()];
    let natives = [observation(
        revision.selected.surface,
        Some(&revision.package),
        NativeResourceRetention::Unknown,
    )];
    let input = |files| SkillDeliveryInput {
        current: &current,
        scope: &current.scope,
        graph_changes: &changes,
        intents: &intents,
        candidates: &candidates,
        native_observations: &natives,
        file_observations: files,
    };
    let plan = plan_native_skill_files(&input(&files)).unwrap();
    assert!(plan.native_changes.is_empty());
    assert!(!plan.stage_files.is_empty());
    for file in plan.stage_files.iter().chain(&plan.selector_files) {
        assert_eq!(file.before, file.after);
    }
    let mut drifted = files.clone();
    drifted
        .iter_mut()
        .find(|f| f.location.path.ends_with("/run.sh"))
        .unwrap()
        .value
        .as_mut()
        .unwrap()
        .mode = Some(0o755);
    assert!(
        plan_native_skill_files(&input(&drifted))
            .unwrap_err()
            .to_string()
            .contains("mode evidence changed")
    );
}

#[test]
fn native_projection_rejects_unknown_resource_modes_before_effects() {
    let mut revision = revision(TargetSurface::ClaudeCli, "Body.");
    revision.selected.resources[0].mode = None;
    revision.package = compile_skill_package(&SkillPackageRequest {
        selected: &revision.selected,
        surface: revision.selected.surface,
        scope: &scope(),
        owner: &revision.installation.owner,
    })
    .unwrap();
    let current = empty();
    let files = absent(&revision);
    let intents = [intent(revision.selected.surface)];
    let natives = [observation(
        revision.selected.surface,
        None,
        NativeResourceRetention::Unknown,
    )];
    let changes = GraphChanges {
        installations: vec![revision.installation.clone()],
        add_references: vec![Reference::DirectInstallation {
            installation_id: "demo-install".into(),
        }],
        ..Default::default()
    };
    let candidates = [revision];
    let input = SkillDeliveryInput {
        current: &current,
        scope: &current.scope,
        graph_changes: &changes,
        intents: &intents,
        candidates: &candidates,
        native_observations: &natives,
        file_observations: &files,
    };
    assert!(
        plan_native_skill_files(&input)
            .unwrap_err()
            .to_string()
            .contains("captured mode")
    );
}

#[test]
fn intact_exports_do_not_satisfy_native_refresh_retention() {
    use std::collections::BTreeMap;
    let previous = revision(TargetSurface::ClaudeCli, "Old.");
    let next = revision(TargetSurface::ClaudeCli, "New.");
    let current = installed(&previous);
    let mut observations: BTreeMap<_, _> = applied(&previous)
        .into_iter()
        .map(|f| (f.location.path.clone(), f))
        .collect();
    for file in absent(&next) {
        observations
            .entry(file.location.path.clone())
            .or_insert(file);
    }
    let files: Vec<_> = observations.into_values().collect();
    let changes = GraphChanges {
        installations: vec![next.installation.clone()],
        ..Default::default()
    };
    let intents = [intent(previous.selected.surface)];
    let candidates = [next];
    let unknown = [observation(
        previous.selected.surface,
        Some(&previous.package),
        NativeResourceRetention::Unknown,
    )];
    let input = |natives| SkillDeliveryInput {
        current: &current,
        scope: &current.scope,
        graph_changes: &changes,
        intents: &intents,
        candidates: &candidates,
        native_observations: natives,
        file_observations: &files,
    };
    assert!(
        plan_native_skill_files(&input(&unknown))
            .unwrap_err()
            .to_string()
            .contains("cache preservation or session clearance")
    );
    let cleared = [observation(
        previous.selected.surface,
        Some(&previous.package),
        NativeResourceRetention::SessionsCleared {
            evidence_id: "clearance-1".into(),
        },
    )];
    let plan = plan_native_skill_files(&input(&cleared)).unwrap();
    assert_eq!(plan.native_changes.len(), 1);
    assert_eq!(plan.revisions.len(), 2);
    assert_eq!(
        plan.stage_files.len(),
        previous.package.revision_files.len() + candidates[0].package.revision_files.len()
    );
    for old in &previous.package.revision_files {
        let file = plan
            .stage_files
            .iter()
            .find(|f| f.location.path == old.path)
            .unwrap();
        assert_eq!(file.before, file.after);
        assert_eq!(file.after.as_ref().unwrap().bytes, old.bytes);
    }
    assert!(plan.stage_files.iter().all(|f| f.after.is_some()));
}

#[test]
fn native_permission_projection_does_not_adopt_identical_manual_exports() {
    let revision = revision(TargetSurface::ClaudeCli, "Body.");
    let current = empty();
    let mut files = absent(&revision);
    let manual = applied(&revision)
        .into_iter()
        .find(|f| f.location.path.ends_with("/run.sh"))
        .unwrap();
    files
        .iter_mut()
        .find(|f| f.location == manual.location)
        .unwrap()
        .value = manual.value;
    let changes = GraphChanges {
        installations: vec![revision.installation.clone()],
        add_references: vec![Reference::DirectInstallation {
            installation_id: "demo-install".into(),
        }],
        ..Default::default()
    };
    let intents = [intent(revision.selected.surface)];
    let natives = [observation(
        revision.selected.surface,
        None,
        NativeResourceRetention::Unknown,
    )];
    let candidates = [revision];
    let input = SkillDeliveryInput {
        current: &current,
        scope: &current.scope,
        graph_changes: &changes,
        intents: &intents,
        candidates: &candidates,
        native_observations: &natives,
        file_observations: &files,
    };
    assert!(
        plan_native_skill_files(&input)
            .unwrap_err()
            .to_string()
            .contains("unowned file already exists")
    );
}
