use sha2::{Digest, Sha256};
use upskill::files::plan::{DesiredFile, FileLocation, FileValue, ObservedFile, PlannedFile};
use upskill::lifecycle::desired::DesiredInstallationSet;
use upskill::lifecycle::preview::{
    FileChange, FilePreview, PreviewOutcome, PreviewRequest, StateChange, preview,
};
use upskill::lifecycle::selection::{LegacyRow, Selection, SelectionDiagnostic};
use upskill::lifecycle::state::{
    CombinedState, Installation, OwnerConsumers, Reference, SourceEvidence, serialize_state,
};
use upskill::lifecycle::upsert::UpsertRequest;
use upskill::operations::plan::DesiredArtifact;
use upskill::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn root() -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: std::env::temp_dir().join("upskill-preview-example"),
    }
}
fn item(root: &InstallationRoot) -> Installation {
    Installation {
        owner: ArtifactOwner {
            installation_id: "a".into(),
            source: "./unavailable-source".into(),
            source_revision: None,
            kind: ItemKind::Rule,
            source_name: "a".into(),
            installed_name: "a".into(),
        },
        root: root.clone(),
        requested: vec![TargetSurface::ClaudeCli],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: "a".repeat(64),
        },
    }
}
fn direct() -> Reference {
    Reference::DirectInstallation {
        installation_id: "a".into(),
    }
}
fn value(bytes: &[u8], mode: u32) -> FileValue {
    FileValue {
        bytes: bytes.into(),
        mode: Some(mode),
    }
}
fn location(root: &InstallationRoot, path: &str) -> FileLocation {
    FileLocation {
        root: root.clone(),
        path: path.into(),
    }
}
fn empty() -> CombinedState {
    CombinedState {
        origin: None,
        schema: 2,
        installations: vec![],
        bundles: vec![],
        references: vec![],
        artifacts: vec![],
        owner_consumers: vec![],
        owner_contracts: vec![],
        legacy: None,
    }
}
fn prior(root: &InstallationRoot, files: &[(&str, &[u8])]) -> CombinedState {
    let mut state = empty();
    state.installations.push(item(root));
    state.references.push(direct());
    for (path, bytes) in files {
        state.artifacts.push(OwnedArtifact {
            root: root.clone(),
            path: (*path).into(),
            content: ArtifactContent::File {
                content_sha256: Sha256::digest(bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
                mode: Some(0o640),
            },
            owners: vec![item(root).owner],
            consumers: vec![TargetSurface::ClaudeCli],
        });
        state.owner_consumers.push(OwnerConsumers {
            root: root.clone(),
            path: (*path).into(),
            contribution_id: None,
            installation_id: "a".into(),
            consumers: vec![TargetSurface::ClaudeCli],
        });
    }
    state
}
fn set(root: &InstallationRoot, files: &[(&str, &[u8], u32)]) -> DesiredInstallationSet {
    DesiredInstallationSet {
        installation: item(root),
        artifacts: files
            .iter()
            .map(|(path, bytes, mode)| {
                DesiredArtifact::File(DesiredFile {
                    location: location(root, path),
                    value: value(bytes, *mode),
                    consumers: vec![TargetSurface::ClaudeCli],
                })
            })
            .collect(),
        audience_excluded: vec![],
    }
}
fn request<'a>(
    selection: Option<&'a Selection>,
    sets: &'a [DesiredInstallationSet],
    add: &'a [Reference],
) -> UpsertRequest<'a> {
    UpsertRequest {
        selection,
        sets,
        contracts: &[],
        bundles: &[],
        add_references: add,
        remove_references: &[],
    }
}
fn ready(outcome: PreviewOutcome) -> upskill::lifecycle::preview::LifecyclePreview {
    match outcome {
        PreviewOutcome::Ready(plan) => *plan,
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn reports_complete_ordered_actions_for_distinct_files_with_reversed_observations() {
    let root = root();
    let state = prior(
        &root,
        &[
            ("a.md", b"a0"),
            ("b.md", b"b0"),
            ("c.md", b"c0"),
            ("d.md", b"d0"),
            ("e.md", b"e0"),
        ],
    );
    let bytes = serialize_state(&state).unwrap().into_bytes();
    let sets = [set(
        &root,
        &[
            ("a.md", b"a1", 0o640),
            ("b.md", b"b0", 0o600),
            ("c.md", b"c1", 0o600),
            ("d.md", b"d0", 0o640),
            ("f.md", b"f1", 0o600),
        ],
    )];
    let mut observations: Vec<_> = ["a", "b", "c", "d", "e"]
        .into_iter()
        .map(|name| ObservedFile {
            location: location(&root, &format!("{name}.md")),
            value: Some(value(format!("{name}0").as_bytes(), 0o640)),
        })
        .collect();
    observations.push(ObservedFile {
        location: location(&root, "f.md"),
        value: None,
    });
    observations.reverse();
    let saved = (bytes.clone(), sets.clone(), observations.clone());
    let selection = Selection::Names(vec!["a".into()]);
    let plan = ready(
        preview(
            &bytes,
            PreviewRequest::Upsert(request(Some(&selection), &sets, &[])),
            &observations,
        )
        .unwrap(),
    );
    let expected = [
        (
            "a.md",
            Some(value(b"a0", 0o640)),
            Some(value(b"a1", 0o640)),
            FileChange::Content,
        ),
        (
            "b.md",
            Some(value(b"b0", 0o640)),
            Some(value(b"b0", 0o600)),
            FileChange::Mode,
        ),
        (
            "c.md",
            Some(value(b"c0", 0o640)),
            Some(value(b"c1", 0o600)),
            FileChange::ContentAndMode,
        ),
        (
            "d.md",
            Some(value(b"d0", 0o640)),
            Some(value(b"d0", 0o640)),
            FileChange::Unchanged,
        ),
        ("e.md", Some(value(b"e0", 0o640)), None, FileChange::Remove),
        ("f.md", None, Some(value(b"f1", 0o600)), FileChange::Create),
    ]
    .into_iter()
    .map(|(path, before, after, change)| FilePreview {
        plan: PlannedFile {
            location: location(&root, path),
            before,
            after,
        },
        change,
    })
    .collect::<Vec<_>>();
    assert_eq!(plan.files, expected);
    assert_eq!(plan.state_change, StateChange::Changed);
    assert_eq!(plan.original_state, bytes);
    assert_eq!(plan.state.artifacts.len(), 5);
    assert_eq!(plan.state.references, vec![direct()]);
    assert_eq!((bytes, sets, observations), saved);
}

#[test]
fn previews_schema1_upgrade_without_losing_unknown_bytes_or_legacy_native_rows() {
    let bytes = br#"{ "schema":1,"future":true,"items":[{"kind":"skill","name":"old","source":"./missing"}],"plugins":[{"name":"p","client":"future","identifier":"p@source","bundle":"b","status":"skipped"}],"mcps":[{"name":"m","client":"claude","bundle":"b","status":"skipped"}]}"#;
    let root = root();
    let sets = [set(&root, &[("new.md", b"new", 0o600)])];
    let observations = [ObservedFile {
        location: location(&root, "new.md"),
        value: None,
    }];
    let plan = ready(
        preview(
            bytes,
            PreviewRequest::Upsert(request(None, &sets, &[direct()])),
            &observations,
        )
        .unwrap(),
    );
    assert_eq!(plan.original_state, bytes);
    assert_eq!(plan.state_change, StateChange::UpgradeFromSchema1);
    assert_eq!(plan.files[0].change, FileChange::Create);
    let legacy = plan.state.legacy.unwrap();
    assert_eq!(legacy.items[0].name, "old");
    assert_eq!(legacy.plugins[0].client, "future");
    assert_eq!(legacy.mcps[0].name, "m");
}

#[test]
fn selected_legacy_blocks_without_returning_partial_actions() {
    let bytes = br#"{"schema":1,"items":[{"kind":"rule","name":"a","source":"./missing"}]}"#;
    let selection = Selection::All;
    let result = preview(
        bytes,
        PreviewRequest::Upsert(request(Some(&selection), &[], &[])),
        &[],
    )
    .unwrap();
    assert_eq!(
        result,
        PreviewOutcome::Blocked(vec![SelectionDiagnostic::LegacyRow(LegacyRow::Item(0))])
    );
}

#[test]
fn exact_reference_removal_proposes_file_deletion_and_retains_unrelated_legacy() {
    let root = root();
    let mut state = prior(&root, &[("a.md", b"owned")]);
    state.legacy = Some(
        serde_json::from_str(
            r#"{"schema":1,"items":[{"kind":"skill","name":"extra","source":"./old"}]}"#,
        )
        .unwrap(),
    );
    let bytes = serialize_state(&state).unwrap().into_bytes();
    let observation = ObservedFile {
        location: location(&root, "a.md"),
        value: Some(value(b"owned", 0o640)),
    };
    let plan = ready(
        preview(
            &bytes,
            PreviewRequest::RemoveReferences(&[direct()]),
            std::slice::from_ref(&observation),
        )
        .unwrap(),
    );
    assert_eq!(
        plan.files,
        vec![FilePreview {
            plan: PlannedFile {
                location: observation.location,
                before: observation.value,
                after: None
            },
            change: FileChange::Remove
        }]
    );
    assert!(plan.state.installations.is_empty());
    assert_eq!(plan.state.legacy, state.legacy);
    assert_eq!(plan.state_change, StateChange::Changed);
}

#[test]
fn no_op_state_preview_ignores_serialization_whitespace() {
    let state = empty();
    let bytes = serde_json::to_vec(&state).unwrap();
    let plan = ready(preview(&bytes, PreviewRequest::RemoveReferences(&[]), &[]).unwrap());
    assert_eq!(plan.original_state, bytes);
    assert_eq!(plan.state_change, StateChange::Unchanged);
    assert_eq!(plan.state, state);
    assert!(plan.files.is_empty());
}

#[test]
fn rejects_complete_semantically_invalid_state_and_edited_output_evidence() {
    let root = root();
    let mut state = prior(&root, &[("a.md", b"owned")]);
    state.references[0] = Reference::DirectInstallation {
        installation_id: "absent".into(),
    };
    let invalid = serde_json::to_vec(&state).unwrap();
    assert!(preview(&invalid, PreviewRequest::RemoveReferences(&[]), &[]).is_err());
    let state = prior(&root, &[("a.md", b"owned")]);
    let bytes = serialize_state(&state).unwrap().into_bytes();
    let observation = ObservedFile {
        location: location(&root, "a.md"),
        value: Some(value(b"edited", 0o640)),
    };
    let saved = (bytes.clone(), observation.clone());
    assert!(
        preview(
            &bytes,
            PreviewRequest::RemoveReferences(&[direct()]),
            std::slice::from_ref(&observation)
        )
        .is_err()
    );
    assert_eq!((bytes, observation), saved);
}

#[test]
fn shared_owner_release_requires_explicit_shared_request_and_preserves_file_bytes() {
    let root = root();
    let mut state = prior(&root, &[("shared.md", b"shared")]);
    let mut other = item(&root);
    other.owner.installation_id = "b".into();
    other.owner.installed_name = "b".into();
    other.owner.source_name = "b".into();
    state.artifacts[0].owners.push(other.owner.clone());
    state.installations.push(other);
    state.references.push(Reference::DirectInstallation {
        installation_id: "b".into(),
    });
    let mut association = state.owner_consumers[0].clone();
    association.installation_id = "b".into();
    state.owner_consumers.push(association);
    let bytes = serialize_state(&state).unwrap().into_bytes();
    let observation = ObservedFile {
        location: location(&root, "shared.md"),
        value: Some(value(b"shared", 0o640)),
    };
    let selection = Selection::Names(vec!["a".into()]);
    let remove = [direct()];
    let request = UpsertRequest {
        remove_references: &remove,
        ..request(Some(&selection), &[], &[])
    };
    assert!(
        preview(
            &bytes,
            PreviewRequest::Upsert(request),
            std::slice::from_ref(&observation)
        )
        .is_err()
    );
    let plan = ready(
        preview(
            &bytes,
            PreviewRequest::SharedUpsert(request),
            std::slice::from_ref(&observation),
        )
        .unwrap(),
    );
    assert_eq!(
        plan.files,
        vec![FilePreview {
            plan: PlannedFile {
                location: observation.location,
                before: observation.value.clone(),
                after: observation.value
            },
            change: FileChange::Unchanged
        }]
    );
    assert_eq!(plan.state_change, StateChange::Changed);
    assert_eq!(plan.state.installations.len(), 1);
    assert_eq!(plan.state.installations[0].owner.installation_id, "b");
}

#[test]
fn reference_only_edit_retains_installation_and_reports_metadata_change() {
    use upskill::lifecycle::state::BundleNode;
    let root = root();
    let mut state = prior(&root, &[("a.md", b"owned")]);
    state.bundles.push(BundleNode {
        id: "bundle".into(),
        source: "./missing-bundle".into(),
        name: "bundle".into(),
        source_revision: None,
    });
    state.references.extend([
        Reference::DirectBundle {
            bundle_id: "bundle".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "bundle".into(),
            installation_id: "a".into(),
        },
    ]);
    let bytes = serialize_state(&state).unwrap().into_bytes();
    let plan = ready(preview(&bytes, PreviewRequest::RemoveReferences(&[direct()]), &[]).unwrap());
    assert_eq!(plan.state.installations, state.installations);
    assert_eq!(plan.state.artifacts, state.artifacts);
    assert_eq!(plan.state.references.len(), 2);
    assert!(plan.files.is_empty());
    assert_eq!(plan.state_change, StateChange::Changed);
}

#[test]
fn preview_preserves_physical_state_sources_outputs_and_unowned_resources() {
    use std::collections::BTreeMap;
    use std::path::Path;
    fn capture(dir: &Path) -> BTreeMap<std::path::PathBuf, Option<Vec<u8>>> {
        fn walk(
            base: &Path,
            dir: &Path,
            result: &mut BTreeMap<std::path::PathBuf, Option<Vec<u8>>>,
        ) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let relative = path.strip_prefix(base).unwrap().to_path_buf();
                if path.is_dir() {
                    result.insert(relative, None);
                    walk(base, &path, result);
                } else {
                    result.insert(relative, Some(std::fs::read(path).unwrap()));
                }
            }
        }
        let mut result = BTreeMap::new();
        walk(dir, dir, &mut result);
        result
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().to_path_buf(),
    };
    std::fs::create_dir(tmp.path().join("source")).unwrap();
    std::fs::write(
        tmp.path().join("source/SSOT.md"),
        b"source must remain untouched",
    )
    .unwrap();
    std::fs::write(tmp.path().join("extra.resource"), b"unowned resource").unwrap();
    std::fs::write(tmp.path().join("a.md"), b"owned").unwrap();
    let state = prior(&root, &[("a.md", b"owned")]);
    let bytes = serialize_state(&state).unwrap().into_bytes();
    std::fs::write(tmp.path().join(".upskill-lock.json"), &bytes).unwrap();
    let observation = ObservedFile {
        location: location(&root, "a.md"),
        value: Some(value(b"owned", 0o640)),
    };
    let before = capture(tmp.path());
    let plan = ready(
        preview(
            &bytes,
            PreviewRequest::RemoveReferences(&[direct()]),
            std::slice::from_ref(&observation),
        )
        .unwrap(),
    );
    assert_eq!(plan.files[0].change, FileChange::Remove);
    assert_eq!(capture(tmp.path()), before);
    let edited = ObservedFile {
        value: Some(value(b"edited", 0o640)),
        ..observation
    };
    assert!(
        preview(
            &bytes,
            PreviewRequest::RemoveReferences(&[direct()]),
            &[edited]
        )
        .is_err()
    );
    assert_eq!(capture(tmp.path()), before);
    assert!(!tmp.path().join(".upskill").exists());
    assert!(!tmp.path().join(".upskill-lock.json.schema1.bak").exists());
}

#[test]
fn equivalent_record_order_and_noop_outputs_do_not_report_state_change() {
    let root = root();
    let state = prior(&root, &[("b.md", b"b0"), ("a.md", b"a0")]);
    let bytes = serde_json::to_vec(&state).unwrap();
    let sets = [set(
        &root,
        &[("a.md", b"a0", 0o640), ("b.md", b"b0", 0o640)],
    )];
    let observations = [
        ObservedFile {
            location: location(&root, "b.md"),
            value: Some(value(b"b0", 0o640)),
        },
        ObservedFile {
            location: location(&root, "a.md"),
            value: Some(value(b"a0", 0o640)),
        },
    ];
    let selection = Selection::Names(vec!["a".into()]);
    let plan = ready(
        preview(
            &bytes,
            PreviewRequest::Upsert(request(Some(&selection), &sets, &[])),
            &observations,
        )
        .unwrap(),
    );
    assert_eq!(plan.state_change, StateChange::Unchanged);
    assert_eq!(plan.original_state, bytes);
    assert_eq!(plan.files.len(), 2);
    assert!(plan.files.iter().all(|f| f.change == FileChange::Unchanged));
}
