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
    revision_with_frontmatter(surface, body, alias, "")
}
fn revision_with_frontmatter(
    surface: TargetSurface,
    body: &str,
    alias: Option<&str>,
    extra: &str,
) -> PackageRevision {
    revision_with_source(surface, body, alias, extra, None)
}
fn revision_with_source(
    surface: TargetSurface,
    body: &str,
    alias: Option<&str>,
    extra: &str,
    override_body: Option<&str>,
) -> PackageRevision {
    let source = tempfile::tempdir().unwrap();
    for (path, bytes, mode) in [
        (
            "SKILL.md",
            format!("---\nschema: 1\ndescription: Demo.\n{extra}---\n{body}\n").into_bytes(),
            0o755,
        ),
        ("data.bin", vec![0, 255, 1], 0o644),
        ("run.sh", b"#!/bin/sh\nexit 0\n".to_vec(), 0o644),
        ("scripts/run.sh", b"#!/bin/sh\nexit 7\n".to_vec(), 0o755),
    ] {
        let path = source.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }
    if let Some(body) = override_body {
        let suffix = match surface {
            TargetSurface::ClaudeCli => "claude",
            TargetSurface::CopilotCli => "copilot",
            TargetSurface::OpenCodeCli => "opencode",
            _ => panic!("fixture target has no override"),
        };
        fs::write(source.path().join(format!("SKILL.{suffix}.md")), body).unwrap();
    }
    let manifest = capture_source(source.path()).unwrap();
    let capabilities =
        ["name", "description", "license", "metadata"].map(|name| CapabilityDeclaration {
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
                mode: Some(if f.path.ends_with("/scripts/run.sh") {
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
            let expected = if source.path.ends_with("/scripts/run.sh") {
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
                .find(|f| f.location.path.ends_with("/scripts/run.sh"))
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
    for resource_path in ["data.bin", "scripts/run.sh"] {
        let mut revision = revision(TargetSurface::ClaudeCli, "Body.");
        revision
            .selected
            .resources
            .iter_mut()
            .find(|r| r.path == resource_path)
            .unwrap()
            .mode = None;
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

use upskill::native_delivery::state::{
    ConfigOwner, NativeConfigState, NativeDeliveryState, NativeStateContext, parse_native_state,
    serialize_native_state,
};

fn state_context() -> NativeStateContext {
    NativeStateContext {
        scope: scope(),
        private_root: InstallationRoot {
            scope: OwnershipScope::User,
            path: "/home/fixture".into(),
        },
        namespace: "machine-fixture-1".into(),
        additional_roots: vec![],
    }
}
fn native_state(revision: &PackageRevision) -> NativeDeliveryState {
    use sha2::{Digest, Sha256};
    let mut skills = installed(revision);
    skills.receipts[0].captured_prestate = b"PRIVATE_NATIVE_SEED".to_vec();
    let files = applied(revision)
        .into_iter()
        .map(|file| {
            let value = file.value.unwrap();
            upskill::ownership::OwnedArtifact {
                root: file.location.root,
                path: file.location.path,
                content: ArtifactContent::File {
                    content_sha256: Sha256::digest(&value.bytes)
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect(),
                    mode: value.mode,
                },
                owners: vec![revision.installation.owner.clone()],
                consumers: vec![revision.selected.surface],
            }
        })
        .collect();
    NativeDeliveryState {
        context: state_context(),
        skills,
        intents: vec![intent(revision.selected.surface)],
        files,
        configurations: vec![],
    }
}
fn with_configuration(state: &mut NativeDeliveryState) {
    use upskill::native_config::{
        CapturedConfig, ConfigContribution, ConfigFormat, ConfigLayer, ConfigSelector,
        plan_config_patch,
    };
    let scope = state.context.private_root.clone();
    let mut capture = CapturedConfig {
        surface: TargetSurface::ClaudeCli,
        scope: scope.clone(),
        target: ConfigLayer {
            format: ConfigFormat::Jsonc,
            file: ObservedFile {
                location: FileLocation {
                    root: scope,
                    path: ".claude/settings.json".into(),
                },
                value: Some(FileValue {
                    bytes: b"{\"manual\":\"PRIVATE_CONFIG_SEED\"}".to_vec(),
                    mode: Some(0o600),
                }),
            },
            tracked: false,
            ignored: false,
        },
        lower_layers: vec![],
        higher_layers: vec![],
        ownership: None,
    };
    let patch = plan_config_patch(
        &capture,
        &ConfigContribution {
            id: "config-demo".into(),
            selector: ConfigSelector::Key(vec!["managed".into()]),
            expected: None,
            desired: Some(serde_json::json!(true)),
        },
    )
    .unwrap();
    capture.target.file.value = patch.file.after;
    capture.ownership = patch.ownership_after;
    state.configurations.push(NativeConfigState {
        capture,
        owners: vec![ConfigOwner {
            entry_id: "config-demo".into(),
            target: key(TargetSurface::ClaudeCli),
        }],
    });
}

#[test]
fn native_state_round_trip_keeps_one_graph_and_private_evidence_out_of_lockfile() {
    let mut state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    with_configuration(&mut state);
    let snapshot = state.clone();
    let encoded = serialize_native_state(&state).unwrap();
    let json: serde_json::Value = serde_json::from_str(&encoded.json).unwrap();
    assert_eq!(json["schema"], 3);
    assert_eq!(json["file_policy"], "native-skill-files-v1");
    assert!(json["receipts"][0].get("captured_prestate").is_none());
    assert_eq!(
        json["receipts"][0]["prestate"].as_object().unwrap().len(),
        3
    );
    assert!(json["configurations"][0].get("capture").is_none());
    assert_eq!(
        json["configurations"][0]["snapshot"]
            .as_object()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(encoded.private_evidence.len(), 2);
    assert!(
        encoded
            .private_evidence
            .iter()
            .any(|b| b.bytes == b"PRIVATE_NATIVE_SEED")
    );
    assert_eq!(
        parse_native_state(&encoded.json, &state.context, &encoded.private_evidence).unwrap(),
        snapshot
    );
    assert_eq!(state, snapshot);
    assert!(upskill::lifecycle::state::parse_state(&encoded.json).is_err());
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".upskill-lock.json"), &encoded.json).unwrap();
    assert!(upskill::lockfile::Lockfile::load(dir.path()).is_err());
    assert_eq!(
        fs::read_to_string(dir.path().join(".upskill-lock.json")).unwrap(),
        encoded.json
    );
}

#[test]
fn native_state_rejects_missing_corrupt_or_misbound_private_evidence() {
    let state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    let encoded = serialize_native_state(&state).unwrap();
    assert!(parse_native_state(&encoded.json, &state.context, &[]).is_err());
    let mut blobs = encoded.private_evidence.clone();
    blobs[0].bytes.push(0);
    assert!(parse_native_state(&encoded.json, &state.context, &blobs).is_err());
    let mut blobs = encoded.private_evidence.clone();
    blobs[0].bytes[0] ^= 1;
    assert!(parse_native_state(&encoded.json, &state.context, &blobs).is_err());
    let mut context = state.context.clone();
    context.namespace = "another-machine".into();
    assert!(parse_native_state(&encoded.json, &context, &encoded.private_evidence).is_err());
    let mut context = state.context.clone();
    context.scope.path = "/other/project".into();
    assert!(parse_native_state(&encoded.json, &context, &encoded.private_evidence).is_err());
    let mut context = state.context.clone();
    context.private_root.path = "/other/home".into();
    assert!(parse_native_state(&encoded.json, &context, &encoded.private_evidence).is_err());
}

#[test]
fn native_state_rejects_unknown_fields_versions_duplicate_json_and_conflicting_rows() {
    let state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    let encoded = serialize_native_state(&state).unwrap();
    let base: serde_json::Value = serde_json::from_str(&encoded.json).unwrap();
    for mutation in [
        "version",
        "policy",
        "unknown",
        "nested-unknown",
        "duplicate-file",
        "duplicate-receipt",
        "missing-intent",
    ] {
        let mut value = base.clone();
        match mutation {
            "version" => value["schema"] = 4.into(),
            "policy" => value["file_policy"] = "future-policy".into(),
            "unknown" => value["extra"] = true.into(),
            "nested-unknown" => {
                let mut selected: serde_yaml_ng::Value = serde_yaml_ng::from_str(
                    value["revisions"][0]["selected_yaml"].as_str().unwrap(),
                )
                .unwrap();
                selected["requires"]["extra"] = true.into();
                value["revisions"][0]["selected_yaml"] =
                    serde_yaml_ng::to_string(&selected).unwrap().into();
            }
            "duplicate-file" => {
                let row = value["files"][0].clone();
                value["files"].as_array_mut().unwrap().push(row);
            }
            "duplicate-receipt" => {
                let row = value["receipts"][0].clone();
                value["receipts"].as_array_mut().unwrap().push(row);
            }
            "missing-intent" => value["intents"] = serde_json::json!([]),
            _ => unreachable!(),
        }
        assert!(
            parse_native_state(
                &value.to_string(),
                &state.context,
                &encoded.private_evidence
            )
            .is_err(),
            "{mutation}"
        );
    }
    let duplicate = encoded
        .json
        .replacen("\"schema\": 3", "\"schema\": 4, \"schema\": 3", 1);
    assert_ne!(duplicate, encoded.json);
    assert!(parse_native_state(&duplicate, &state.context, &encoded.private_evidence).is_err());
}

#[test]
fn native_state_validates_applied_modes_package_evidence_and_confirmed_selection() {
    let state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    for mutation in [
        "mode",
        "package",
        "selection",
        "outcome",
        "missing-file",
        "logical-ownership",
        "hash",
        "owner",
        "consumer",
        "root-spelling",
    ] {
        let mut bad = state.clone();
        match mutation {
            "mode" => {
                let ArtifactContent::File { mode, .. } = &mut bad.files[0].content else {
                    unreachable!()
                };
                *mode = Some(0o644);
            }
            "package" => bad.skills.revisions[0].package.payload_sha256 = "a".repeat(64),
            "selection" => bad.skills.receipts[0].after = None,
            "outcome" => bad.skills.receipts[0].outcome = NativeBindingOutcome::Unknown,
            "missing-file" => {
                bad.files.pop();
            }
            "logical-ownership" => bad.skills.graph.artifacts.push(bad.files[0].clone()),
            "hash" => {
                let ArtifactContent::File { content_sha256, .. } = &mut bad.files[0].content else {
                    unreachable!()
                };
                *content_sha256 = "a".repeat(64);
            }
            "owner" => bad.files[0].owners[0].installation_id = "foreign".into(),
            "consumer" => bad.files[0].consumers = vec![TargetSurface::CopilotCli],
            "root-spelling" => bad.files[0].root.path = "/workspace//project".into(),
            _ => unreachable!(),
        }
        assert!(serialize_native_state(&bad).is_err(), "{mutation}");
    }
}

#[test]
fn native_state_private_configuration_ownership_requires_exact_target_bindings() {
    let mut state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    with_configuration(&mut state);
    for mutation in [
        "owner",
        "missing-owner",
        "document",
        "raw-token",
        "duplicate-document",
        "scope",
        "empty-ownership",
        "root-spelling",
    ] {
        let mut bad = state.clone();
        match mutation {
            "owner" => bad.configurations[0].owners[0].target.installation_id = "foreign".into(),
            "missing-owner" => bad.configurations[0].owners.clear(),
            "document" => {
                bad.configurations[0]
                    .capture
                    .ownership
                    .as_mut()
                    .unwrap()
                    .location
                    .path = "other.json".into()
            }
            "raw-token" => {
                bad.configurations[0]
                    .capture
                    .target
                    .file
                    .value
                    .as_mut()
                    .unwrap()
                    .bytes = b"{\"managed\":false}".to_vec()
            }
            "duplicate-document" => bad.configurations.push(bad.configurations[0].clone()),
            "empty-ownership" => {
                bad.configurations[0]
                    .capture
                    .ownership
                    .as_mut()
                    .unwrap()
                    .entries
                    .clear();
                bad.configurations[0].owners.clear();
            }
            "root-spelling" => {
                bad.configurations[0].capture.target.file.location.root.path =
                    "/home//fixture".into()
            }
            "scope" => {
                bad.configurations[0].capture.scope.path = "/external/home".into();
                bad.configurations[0].capture.target.file.location.root =
                    bad.configurations[0].capture.scope.clone();
            }
            _ => unreachable!(),
        }
        assert!(serialize_native_state(&bad).is_err(), "{mutation}");
    }
}

#[test]
fn native_state_preserves_four_client_packages_and_rich_source_metadata() {
    for surface in [
        TargetSurface::ClaudeCli,
        TargetSurface::CopilotCli,
        TargetSurface::CodexCli,
        TargetSurface::OpenCodeCli,
    ] {
        let revision = revision_with_frontmatter(
            surface,
            "Body with é and directives.\r\n",
            Some("alternate"),
            "license: MIT\nmetadata:\n  author: Fixture\n  nested:\n    1: value\nrequires:\n  rules: [helper]\n",
        );
        let state = native_state(&revision);
        let encoded = serialize_native_state(&state).unwrap();
        assert_eq!(
            parse_native_state(&encoded.json, &state.context, &encoded.private_evidence).unwrap(),
            state
        );
        assert_eq!(state.skills.revisions[0], revision);
    }
}

#[test]
fn native_state_retains_detached_git_revisions_without_an_active_graph_node() {
    fn git(revision: &mut PackageRevision, tag: &str) {
        revision.installation.owner.source = "org/repo".into();
        revision.installation.owner.source_revision = Some(tag.into());
        revision.installation.source_evidence = SourceEvidence::Git {
            resolved_revision: tag.into(),
            manifest_sha256: revision.selected.source_manifest_sha256.clone(),
        };
        revision.package = compile_skill_package(&SkillPackageRequest {
            selected: &revision.selected,
            surface: revision.selected.surface,
            scope: &scope(),
            owner: &revision.installation.owner,
        })
        .unwrap();
    }
    let mut old = revision(TargetSurface::ClaudeCli, "Old.");
    git(&mut old, &"1".repeat(40));
    let mut new = revision(TargetSurface::ClaudeCli, "New.");
    git(&mut new, &"2".repeat(40));
    let mut state = native_state(&new);
    state.skills.revisions.push(old.clone());
    let old_files = native_state(&old).files;
    for mut file in old_files {
        if old
            .package
            .catalog_file
            .as_ref()
            .is_some_and(|c| c.path == file.path)
        {
            continue;
        }
        file.owners = vec![new.installation.owner.clone()];
        state.files.push(file);
    }
    let active = serialize_native_state(&state).unwrap();
    assert_eq!(
        parse_native_state(&active.json, &state.context, &active.private_evidence).unwrap(),
        state
    );
    state.skills.graph = empty().graph;
    state.intents.clear();
    state.skills.receipts[0].before = state.skills.receipts[0].after.take();
    state.files.retain(|f| {
        new.package
            .catalog_file
            .as_ref()
            .is_none_or(|c| c.path != f.path)
    });
    for file in &mut state.files {
        file.owners[0].source_revision = None;
    }
    let encoded = serialize_native_state(&state).unwrap();
    let parsed =
        parse_native_state(&encoded.json, &state.context, &encoded.private_evidence).unwrap();
    assert!(parsed.skills.graph.installations.is_empty());
    assert_eq!(parsed.skills.revisions.len(), 2);
    assert_eq!(parsed, state);
    assert_eq!(
        parsed.skills.revisions[0]
            .installation
            .owner
            .source_revision,
        Some("2".repeat(40))
    );
    assert_eq!(
        parsed.skills.revisions[1]
            .installation
            .owner
            .source_revision,
        Some("1".repeat(40))
    );
    state.skills.receipts[0].before = None;
    assert!(serialize_native_state(&state).is_err());
}

#[test]
fn native_state_keeps_legacy_order_and_origin_without_reinterpreting_old_rows() {
    let original=b"{\"schema\":1,\"items\":[{\"kind\":\"rule\",\"name\":\"z\",\"source\":\"./z\"},{\"kind\":\"skill\",\"name\":\"a\",\"source\":\"./a\"}],\"bundles\":[],\"manual\":true}";
    let mut state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    state.skills.graph.legacy = Some(serde_json::from_slice(original).unwrap());
    state.skills.graph.origin =
        Some(upskill::lifecycle::state::StateOrigin::from_schema1_bytes(original).unwrap());
    let encoded = serialize_native_state(&state).unwrap();
    let parsed =
        parse_native_state(&encoded.json, &state.context, &encoded.private_evidence).unwrap();
    assert_eq!(parsed.skills.graph, state.skills.graph);
    assert_eq!(
        parsed
            .skills
            .graph
            .legacy
            .unwrap()
            .items
            .iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>(),
        vec!["z", "a"]
    );
}

#[test]
fn native_state_private_reference_kind_and_length_are_not_interchangeable() {
    let mut state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    with_configuration(&mut state);
    let encoded = serialize_native_state(&state).unwrap();
    let json: serde_json::Value = serde_json::from_str(&encoded.json).unwrap();
    let mut wrong = json.clone();
    wrong["receipts"][0]["prestate"] = wrong["configurations"][0]["snapshot"].clone();
    assert!(
        parse_native_state(
            &wrong.to_string(),
            &state.context,
            &encoded.private_evidence
        )
        .is_err()
    );
    let mut wrong = json.clone();
    let length = wrong["receipts"][0]["prestate"]["length"].as_u64().unwrap();
    wrong["receipts"][0]["prestate"]["length"] = (length + 1).into();
    assert!(
        parse_native_state(
            &wrong.to_string(),
            &state.context,
            &encoded.private_evidence
        )
        .is_err()
    );
}

#[test]
fn native_state_preserves_unrelated_rule_ownership_and_rejects_config_overlap() {
    use upskill::lifecycle::state::OwnerConsumers;
    use upskill::ownership::OwnedArtifact;
    let mut state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    let owner = ArtifactOwner {
        installation_id: "manual-rule".into(),
        source: "./manual".into(),
        source_revision: None,
        kind: ItemKind::Rule,
        source_name: "manual".into(),
        installed_name: "manual".into(),
    };
    state.skills.graph.installations.push(Installation {
        owner: owner.clone(),
        root: scope(),
        requested: vec![TargetSurface::ClaudeCli],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: "e".repeat(64),
        },
    });
    state
        .skills
        .graph
        .references
        .push(Reference::DirectInstallation {
            installation_id: owner.installation_id.clone(),
        });
    state.skills.graph.artifacts.push(OwnedArtifact {
        root: scope(),
        path: ".claude/rules/manual.md".into(),
        content: ArtifactContent::File {
            content_sha256: "f".repeat(64),
            mode: Some(0o644),
        },
        owners: vec![owner.clone()],
        consumers: vec![TargetSurface::ClaudeCli],
    });
    state.skills.graph.owner_consumers.push(OwnerConsumers {
        root: scope(),
        path: ".claude/rules/manual.md".into(),
        contribution_id: None,
        installation_id: owner.installation_id,
        consumers: vec![TargetSurface::ClaudeCli],
    });
    let encoded = serialize_native_state(&state).unwrap();
    assert_eq!(
        parse_native_state(&encoded.json, &state.context, &encoded.private_evidence)
            .unwrap()
            .skills
            .graph,
        state.skills.graph
    );
    with_configuration(&mut state);
    let config = &mut state.configurations[0].capture;
    config.scope = scope();
    config.target.file.location.root = scope();
    config.target.file.location.path = ".claude/rules/manual.md".into();
    config.ownership.as_mut().unwrap().location = config.target.file.location.clone();
    assert!(
        serialize_native_state(&state)
            .unwrap_err()
            .to_string()
            .contains("overlaps another ownership")
    );
}

#[test]
fn native_state_retains_verbatim_override_capture_and_rejects_invalid_contexts() {
    let revision = revision_with_source(
        TargetSurface::ClaudeCli,
        "Canonical.",
        Some("alternate"),
        "",
        Some("Override with <!-- if:copilot --> literal directives.\r\n"),
    );
    let mut state = native_state(&revision);
    state.intents[0].selection = SkillIntentSelection::Selected {
        body_origin: SkillBodyOrigin::Override {
            path: "SKILL.claude.md".into(),
        },
    };
    let encoded = serialize_native_state(&state).unwrap();
    assert_eq!(
        parse_native_state(&encoded.json, &state.context, &encoded.private_evidence).unwrap(),
        state
    );
    assert_eq!(
        state.skills.revisions[0].selected.body,
        "Override with <!-- if:copilot --> literal directives.\r\n"
    );
    for mutation in [
        "namespace",
        "private-scope",
        "additional-root",
        "view-scope",
    ] {
        let mut bad = state.clone();
        match mutation {
            "namespace" => bad.context.namespace = "../other".into(),
            "private-scope" => bad.context.private_root.scope = OwnershipScope::Project,
            "additional-root" => bad
                .context
                .additional_roots
                .push(bad.context.private_root.clone()),
            "view-scope" => bad.skills.scope.path = "/elsewhere/project".into(),
            _ => unreachable!(),
        }
        assert!(serialize_native_state(&bad).is_err(), "{mutation}");
    }
}

#[test]
fn native_state_configuration_envelope_must_match_unchanged_private_snapshot() {
    let mut state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    with_configuration(&mut state);
    let encoded = serialize_native_state(&state).unwrap();
    assert_eq!(
        parse_native_state(&encoded.json, &state.context, &encoded.private_evidence).unwrap(),
        state
    );
    let base: serde_json::Value = serde_json::from_str(&encoded.json).unwrap();
    for mutation in ["location", "surface"] {
        let mut value = base.clone();
        match mutation {
            "location" => value["configurations"][0]["location"]["path"] = "other.json".into(),
            "surface" => value["configurations"][0]["surface"] = "copilot.cli".into(),
            _ => unreachable!(),
        }
        assert!(
            parse_native_state(
                &value.to_string(),
                &state.context,
                &encoded.private_evidence
            )
            .unwrap_err()
            .to_string()
            .contains("snapshot binding differs")
        );
    }
}

#[test]
fn native_state_expected_context_must_include_the_exact_additional_roots() {
    let mut state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    state.context.additional_roots.push(InstallationRoot {
        scope: OwnershipScope::User,
        path: "/external/xdg".into(),
    });
    let encoded = serialize_native_state(&state).unwrap();
    assert_eq!(
        parse_native_state(&encoded.json, &state.context, &encoded.private_evidence).unwrap(),
        state
    );
    let mut missing = state.context.clone();
    missing.additional_roots.clear();
    assert!(
        parse_native_state(&encoded.json, &missing, &encoded.private_evidence)
            .unwrap_err()
            .to_string()
            .contains("another root or private namespace")
    );
    let mut different = state.context.clone();
    different.additional_roots[0].path = "/different/xdg".into();
    assert!(
        parse_native_state(&encoded.json, &different, &encoded.private_evidence)
            .unwrap_err()
            .to_string()
            .contains("another root or private namespace")
    );
}

#[test]
fn native_state_config_owner_cannot_switch_to_an_existing_foreign_surface() {
    let claude = revision(TargetSurface::ClaudeCli, "Body.");
    let mut copilot = revision(TargetSurface::CopilotCli, "Body.");
    assert_eq!(
        claude.installation.source_evidence,
        copilot.installation.source_evidence
    );
    let mut state = native_state(&claude);
    let mut other = native_state(&copilot);
    state.skills.graph.installations[0]
        .requested
        .push(TargetSurface::CopilotCli);
    state.skills.revisions[0]
        .installation
        .requested
        .push(TargetSurface::CopilotCli);
    copilot.installation.requested = state.skills.graph.installations[0].requested.clone();
    state.skills.revisions.push(copilot);
    state.skills.receipts.append(&mut other.skills.receipts);
    state.files.append(&mut other.files);
    state.intents.push(intent(TargetSurface::CopilotCli));
    with_configuration(&mut state);
    let encoded = serialize_native_state(&state).unwrap();
    assert_eq!(
        parse_native_state(&encoded.json, &state.context, &encoded.private_evidence).unwrap(),
        state
    );
    state.configurations[0].owners[0].target.surface = TargetSurface::CopilotCli;
    assert!(
        serialize_native_state(&state)
            .unwrap_err()
            .to_string()
            .contains("owner target is unknown or has a different surface")
    );
}

#[test]
fn custom_tags_on_additional_metadata_are_rejected_by_the_source_parser() {
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("SKILL.md"),"---\nschema: 1\ndescription: Demo.\nmetadata:\n  tagged: !fixture\n    key: value\n---\nBody.\n").unwrap();
    let manifest = capture_source(source.path()).unwrap();
    let capabilities = ["name", "description", "metadata"].map(|name| CapabilityDeclaration {
        surface: TargetSurface::ClaudeCli,
        key: CapabilityKey::new(CapabilityCategory::Skills, name),
        representation: Representation::Native,
        requirement: Requirement::RequiredBehavior,
        reason: "offline fixture",
    });
    let plain = tempfile::tempdir().unwrap();
    fs::write(
        plain.path().join("SKILL.md"),
        "---\nschema: 1\ndescription: Demo.\nmetadata:\n  tagged:\n    key: value\n---\nBody.\n",
    )
    .unwrap();
    let plain_manifest = capture_source(plain.path()).unwrap();
    assert!(
        select_skill(&SkillSelectionRequest {
            source: &plain_manifest,
            folder_name: "demo",
            alias: None,
            surface: TargetSurface::ClaudeCli,
            capabilities: &capabilities
        })
        .unwrap()
        .is_some()
    );
    let result = select_skill(&SkillSelectionRequest {
        source: &manifest,
        folder_name: "demo",
        alias: None,
        surface: TargetSurface::ClaudeCli,
        capabilities: &capabilities,
    });
    let error = format!("{:#}", result.unwrap_err());
    assert!(error.contains("parsing YAML frontmatter"));
}

#[test]
fn native_state_rejects_duplicate_private_blobs_without_losing_the_valid_control() {
    let state = native_state(&revision(TargetSurface::ClaudeCli, "Body."));
    let encoded = serialize_native_state(&state).unwrap();
    assert_eq!(
        parse_native_state(&encoded.json, &state.context, &encoded.private_evidence).unwrap(),
        state
    );
    let mut duplicates = encoded.private_evidence.clone();
    duplicates.push(duplicates[0].clone());
    assert!(
        parse_native_state(&encoded.json, &state.context, &duplicates)
            .unwrap_err()
            .to_string()
            .contains("duplicate private evidence")
    );
}
