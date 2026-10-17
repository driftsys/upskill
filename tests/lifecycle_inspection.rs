use sha2::{Digest, Sha256};
use upskill::files::plan::{FileLocation, FileValue, ObservedFile};
use upskill::lifecycle::inspection::{ArtifactIssue, doctor, inspect_state};
use upskill::lifecycle::state::{
    CombinedState, Installation, Reference, SourceEvidence, serialize_state,
};
use upskill::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn state() -> CombinedState {
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: std::env::temp_dir().join("upskill-inspection"),
    };
    let owner = ArtifactOwner {
        installation_id: "a".into(),
        source: "./rules".into(),
        source_revision: None,
        kind: ItemKind::Rule,
        source_name: "a".into(),
        installed_name: "alias".into(),
    };
    CombinedState {
        origin: None,
        schema: 2,
        installations: vec![Installation {
            owner: owner.clone(),
            root: root.clone(),
            requested: vec![TargetSurface::ClaudeCli],
            source_evidence: SourceEvidence::Local {
                manifest_sha256: "a".repeat(64),
            },
        }],
        bundles: vec![],
        references: vec![Reference::DirectInstallation {
            installation_id: "a".into(),
        }],
        artifacts: vec![OwnedArtifact {
            root: root.clone(),
            path: "rule.md".into(),
            content: ArtifactContent::File {
                content_sha256: Sha256::digest(b"original")
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
                mode: Some(0o600),
            },
            owners: vec![owner],
            consumers: vec![TargetSurface::ClaudeCli],
        }],
        owner_consumers: vec![upskill::lifecycle::state::OwnerConsumers {
            root,
            path: "rule.md".into(),
            contribution_id: None,
            installation_id: "a".into(),
            consumers: vec![TargetSurface::ClaudeCli],
        }],
        owner_contracts: vec![],
        legacy: None,
    }
}
fn observation(state: &CombinedState, value: Option<FileValue>) -> ObservedFile {
    ObservedFile {
        location: FileLocation {
            root: state.artifacts[0].root.clone(),
            path: "rule.md".into(),
        },
        value,
    }
}
#[test]
fn legacy_inspection_retains_exact_bytes_unknown_fields_and_duplicate_rows() {
    let bytes = br#"{ "schema":1,"future":{"keep":true},"items":[{"kind":"rule","name":"a","source":"bad source","clients":["unknown"]},{"kind":"rule","name":"a","source":"bad source"}]}"#;
    let report = inspect_state(bytes).unwrap();
    assert_eq!(report.original_bytes, bytes);
    assert!(report.state.installations.is_empty());
    let legacy = report.legacy.unwrap();
    assert_eq!(legacy.items.len(), 2);
    assert!(!legacy.diagnostics.is_empty());
    assert_eq!(legacy.items[0].record.source, "bad source");
}
#[test]
fn doctor_distinguishes_missing_evidence_missing_file_content_and_mode() {
    let state = state();
    let report = inspect_state(serialize_state(&state).unwrap().as_bytes()).unwrap();
    assert_eq!(report.state, state);
    assert_eq!(
        doctor(&report, &[]).unwrap()[0].issue,
        ArtifactIssue::Unobserved
    );
    assert_eq!(
        doctor(&report, &[observation(&state, None)]).unwrap()[0].issue,
        ArtifactIssue::Missing
    );
    let good = FileValue {
        bytes: b"original".to_vec(),
        mode: Some(0o600),
    };
    assert!(
        doctor(&report, &[observation(&state, Some(good.clone()))])
            .unwrap()
            .is_empty()
    );
    let changed = FileValue {
        bytes: b"edited".to_vec(),
        ..good.clone()
    };
    assert_eq!(
        doctor(&report, &[observation(&state, Some(changed))]).unwrap()[0].issue,
        ArtifactIssue::ContentChanged
    );
    let mode = FileValue {
        mode: Some(0o644),
        ..good.clone()
    };
    assert_eq!(
        doctor(&report, &[observation(&state, Some(mode))]).unwrap()[0].issue,
        ArtifactIssue::ModeChanged
    );
    let unknown_mode = FileValue { mode: None, ..good };
    assert_eq!(
        doctor(&report, &[observation(&state, Some(unknown_mode))]).unwrap()[0].issue,
        ArtifactIssue::ModeUnknown
    );
    let duplicate = observation(&state, None);
    assert!(doctor(&report, &[duplicate.clone(), duplicate]).is_err());
}
#[test]
fn rejects_unsupported_invalid_and_non_utf8_state() {
    for bytes in [
        b"{\"schema\":3}".as_slice(),
        b"{}",
        b"\xff",
        b"{\"schema\":2}",
    ] {
        assert!(inspect_state(bytes).is_err());
    }
}
#[test]
fn contribution_inspection_preserves_manual_bytes_and_reports_owned_edits() {
    use upskill::instructions::aggregate::{Contribution, reconcile};
    let edit = reconcile(
        "Manual introduction\n",
        &[],
        &[Contribution::new("a", "Owned\n")],
    )
    .unwrap();
    let mut state = state();
    state.artifacts[0].content = ArtifactContent::MarkdownContribution {
        contribution: edit.owned[0].clone(),
    };
    state.owner_consumers[0].contribution_id = Some("a".into());
    let report = inspect_state(serialize_state(&state).unwrap().as_bytes()).unwrap();
    let manual = edit
        .content
        .replace("Manual introduction", "Updated manual introduction")
        + "Manual ending\n";
    let captured = observation(
        &state,
        Some(FileValue {
            bytes: manual.into_bytes(),
            mode: Some(0o644),
        }),
    );
    assert!(
        doctor(&report, std::slice::from_ref(&captured))
            .unwrap()
            .is_empty()
    );
    let mut changed = captured.clone();
    changed.value.as_mut().unwrap().bytes = edit.content.replace("Owned", "Edited").into_bytes();
    assert!(matches!(
        doctor(&report, &[changed]).unwrap()[0].issue,
        ArtifactIssue::ContributionChanged { .. }
    ));
    assert!(doctor(&report, &[captured]).unwrap().is_empty());
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn qualified_inspection_does_not_create_storage_backup_or_lock() {
    use upskill::filesystem::QualifiedScope;
    use upskill::lifecycle::inspection::inspect_scope;
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().to_path_buf(),
    };
    let bytes = b"{\"schema\":1,\"unknown\":true}\n";
    std::fs::write(tmp.path().join(".upskill-lock.json"), bytes).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let report = inspect_scope(&scope, &root).unwrap();
    assert!(!report.pending_recovery);
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.state.unwrap().original_bytes, bytes);
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        bytes
    );
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 1);
    std::fs::remove_file(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(inspect_scope(&scope, &root).unwrap().state.is_none());
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
}

#[test]
fn mixed_state_keeps_unresolved_native_and_legacy_rows_visible() {
    let mut state = state();
    let legacy: upskill::lockfile::Lockfile = serde_json::from_str(r#"{
        "schema":1,
        "items":[{"kind":"skill","name":"retained","source":"invalid source"}],
        "plugins":[{"name":"p","client":"future-client","identifier":"p@source","bundle":"b","status":"skipped"}],
        "mcps":[{"name":"m","client":"claude","bundle":"b","status":"skipped"}]
    }"#).unwrap();
    state.legacy = Some(legacy.clone());
    let bytes = serialize_state(&state).unwrap().into_bytes();
    let report = inspect_state(&bytes).unwrap();
    assert_eq!(report.state.installations[0].owner.installed_name, "alias");
    let projection = report.legacy.unwrap();
    assert_eq!(projection.snapshot, legacy);
    assert_eq!(projection.items.len(), 1);
    assert_eq!(projection.integrations.len(), 2);
    assert!(!projection.diagnostics.is_empty());
    assert_eq!(report.original_bytes, bytes);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn qualified_doctor_reports_drift_without_changing_state_or_outputs() {
    use std::os::unix::fs::PermissionsExt;
    use upskill::filesystem::QualifiedScope;
    use upskill::lifecycle::inspection::inspect_scope;
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    let mut state = state();
    state.installations[0].root = root.clone();
    state.artifacts[0].root = root.clone();
    state.owner_consumers[0].root = root.clone();
    let bytes = serialize_state(&state).unwrap().into_bytes();
    let path = tmp.path().join(".upskill-lock.json");
    std::fs::write(&path, &bytes).unwrap();
    std::fs::write(tmp.path().join("rule.md"), b"edited").unwrap();
    std::fs::set_permissions(
        tmp.path().join("rule.md"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let report = inspect_scope(&scope, &root).unwrap();
    assert_eq!(report.diagnostics[0].issue, ArtifactIssue::ContentChanged);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        std::fs::read(tmp.path().join("rule.md")).unwrap(),
        b"edited"
    );
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 2);
}

#[test]
fn doctor_accepts_multiple_recorded_contributions_in_one_file() {
    use upskill::instructions::aggregate::{Contribution, reconcile};
    let edit = reconcile(
        "Manual\n",
        &[],
        &[
            Contribution::new("a", "First\n"),
            Contribution::new("b", "Second\n"),
        ],
    )
    .unwrap();
    let mut state = state();
    state.artifacts[0].content = ArtifactContent::MarkdownContribution {
        contribution: edit.owned[0].clone(),
    };
    state.owner_consumers[0].contribution_id = Some(edit.owned[0].id.clone());
    let mut other = state.artifacts[0].clone();
    other.content = ArtifactContent::MarkdownContribution {
        contribution: edit.owned[1].clone(),
    };
    state.artifacts.push(other);
    let mut association = state.owner_consumers[0].clone();
    association.contribution_id = Some(edit.owned[1].id.clone());
    state.owner_consumers.push(association);
    let report = inspect_state(serialize_state(&state).unwrap().as_bytes()).unwrap();
    let output = observation(
        &state,
        Some(FileValue {
            bytes: edit.content.into_bytes(),
            mode: Some(0o644),
        }),
    );
    assert_eq!(doctor(&report, &[output]).unwrap(), vec![]);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn scope_inspection_remains_read_only_while_writer_guard_is_held() {
    use upskill::filesystem::QualifiedScope;
    use upskill::lifecycle::inspection::inspect_scope;
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let before: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert!(inspect_scope(&scope, &root).unwrap().state.is_none());
    let after: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(before, after);
    guard.verify_binding().unwrap();
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn scope_inspection_rejects_foreign_artifact_roots_without_writes() {
    use upskill::filesystem::QualifiedScope;
    use upskill::lifecycle::inspection::inspect_scope;
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    let bytes = serialize_state(&state()).unwrap().into_bytes();
    std::fs::write(tmp.path().join(".upskill-lock.json"), &bytes).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    assert!(
        inspect_scope(&scope, &root)
            .unwrap_err()
            .to_string()
            .contains("another scope")
    );
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        bytes
    );
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 1);
}
