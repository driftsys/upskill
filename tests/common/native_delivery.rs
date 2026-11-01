//! Supplied offline fixtures; confirmed records here are not runtime activation proof.
#![allow(dead_code)]
use sha2::{Digest, Sha256};
use std::fs;
use upskill::compile::skill_package::{SkillPackageRequest, compile_skill_package};
use upskill::compile::skill_selection::{SkillBodyOrigin, SkillSelectionRequest, select_skill};
use upskill::files::plan::{FileLocation, ObservedFile};
use upskill::lifecycle::native_receipt::{
    NativeBindingOutcome, NativeBindingReceipt, NativeObservationState, NativeResourceRetention,
    NativeSkillObservation, NativeSkillSelection, SkillTarget,
};
use upskill::lifecycle::reconcile::GraphChanges;
use upskill::lifecycle::skill_delivery::{
    PackageRevision, SkillDeliveryIntent, SkillDeliveryState, SkillIntentSelection,
};
use upskill::lifecycle::state::{CombinedState, Installation, Reference, SourceEvidence};
use upskill::native_delivery::journal::NativeOperationCapture;
use upskill::native_delivery::state::{NativeDeliveryState, NativeStateContext};
use upskill::ownership::{ArtifactContent, ArtifactOwner};
use upskill::pipeline::ItemKind;
use upskill::source_manifest::capture_source;
use upskill::target::{
    CapabilityCategory, CapabilityDeclaration, CapabilityKey, Representation, Requirement,
    TargetSurface,
};

pub fn capture(context: &NativeStateContext) -> NativeOperationCapture {
    capture_for_surface(context, TargetSurface::ClaudeCli)
}

pub fn capture_for_surface(
    context: &NativeStateContext,
    surface: TargetSurface,
) -> NativeOperationCapture {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("SKILL.md"),
        "---\nschema: 1\ndescription: Fixture.\n---\nFixture body.\n",
    )
    .unwrap();
    let source = capture_source(tmp.path()).unwrap();
    let capabilities = ["name", "description"].map(|name| CapabilityDeclaration {
        surface,
        key: CapabilityKey::new(CapabilityCategory::Skills, name),
        representation: Representation::Native,
        requirement: Requirement::RequiredBehavior,
        reason: "offline fixture",
    });
    let selected = select_skill(&SkillSelectionRequest {
        source: &source,
        folder_name: "fixture",
        alias: None,
        surface,
        capabilities: &capabilities,
    })
    .unwrap()
    .unwrap();
    let installation = Installation {
        owner: ArtifactOwner {
            installation_id: "fixture-install".into(),
            source: "./fixture".into(),
            source_revision: None,
            kind: ItemKind::Skill,
            source_name: "fixture".into(),
            installed_name: selected.installed_name.clone(),
        },
        root: context.scope.clone(),
        requested: vec![surface],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: selected.source_manifest_sha256.clone(),
        },
    };
    let package = compile_skill_package(&SkillPackageRequest {
        selected: &selected,
        surface,
        scope: &context.scope,
        owner: &installation.owner,
    })
    .unwrap();
    let key = SkillTarget {
        installation_id: installation.owner.installation_id.clone(),
        surface,
    };
    let files = package
        .revision_files
        .iter()
        .chain(package.catalog_file.iter())
        .map(|file| ObservedFile {
            location: FileLocation {
                root: context.scope.clone(),
                path: file.path.clone(),
            },
            value: None,
        })
        .collect();
    NativeOperationCapture {
        graph_changes: GraphChanges {
            installations: vec![installation.clone()],
            add_references: vec![Reference::DirectInstallation {
                installation_id: key.installation_id.clone(),
            }],
            ..Default::default()
        },
        intents: vec![SkillDeliveryIntent {
            key: key.clone(),
            selection: SkillIntentSelection::Selected {
                body_origin: SkillBodyOrigin::Canonical,
            },
        }],
        candidates: vec![PackageRevision {
            installation,
            selected,
            package,
        }],
        native_observations: vec![NativeSkillObservation {
            key,
            scope: context.scope.clone(),
            client_version: "fixture-1".into(),
            platform: "fixture-os".into(),
            observation_id: "fixture-capture".into(),
            captured_prestate: b"absent".to_vec(),
            state: NativeObservationState::Absent,
            retention: NativeResourceRetention::Unknown,
        }],
        file_observations: files,
        configurations: vec![],
    }
}

pub fn state(context: &NativeStateContext) -> NativeDeliveryState {
    let capture = capture(context);
    let revision = capture.candidates[0].clone();
    let key = capture.intents[0].key.clone();
    let files = revision
        .package
        .revision_files
        .iter()
        .chain(revision.package.catalog_file.iter())
        .map(|file| upskill::ownership::OwnedArtifact {
            root: context.scope.clone(),
            path: file.path.clone(),
            content: ArtifactContent::File {
                content_sha256: Sha256::digest(&file.bytes)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect(),
                mode: Some(0o600),
            },
            owners: vec![revision.installation.owner.clone()],
            consumers: vec![key.surface],
        })
        .collect();
    NativeDeliveryState {
        context: context.clone(),
        skills: SkillDeliveryState {
            scope: context.scope.clone(),
            graph: CombinedState {
                schema: 2,
                origin: None,
                installations: vec![revision.installation.clone()],
                bundles: vec![],
                references: capture.graph_changes.add_references,
                artifacts: vec![],
                owner_consumers: vec![],
                owner_contracts: vec![],
                legacy: None,
            },
            revisions: vec![revision.clone()],
            staged: vec![],
            receipts: vec![NativeBindingReceipt {
                key,
                scope: context.scope.clone(),
                client_version: "fixture-1".into(),
                platform: "fixture-os".into(),
                operation_id: "fixture-operation".into(),
                before: None,
                after: Some(NativeSkillSelection::from_package(&revision.package)),
                captured_prestate: b"fixture-private-prestate".to_vec(),
                outcome: NativeBindingOutcome::Confirmed,
            }],
        },
        intents: capture.intents,
        files,
        configurations: vec![],
    }
}
