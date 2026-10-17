#![cfg(any(target_os = "linux", target_os = "macos"))]

use sha2::Digest;
use upskill::files::plan::{FileLocation, FileOperationPlan, FileValue, PlannedFile};
use upskill::filesystem::{QualifiedScope, execution::ExecutionOutcome};
use upskill::lifecycle::application::{
    ExistingSchema2Write, FirstSchema2Write, apply_first_schema2_write, apply_schema2_write,
};
use upskill::lifecycle::state::{
    CombinedState, Installation, OwnerConsumers, Reference, SourceEvidence, serialize_state,
};
use upskill::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn owned_rule_state(root: &InstallationRoot, bytes: &[u8]) -> CombinedState {
    let owner = ArtifactOwner {
        installation_id: "rule-a".into(),
        source: "./rules".into(),
        source_revision: None,
        kind: ItemKind::Rule,
        source_name: "a".into(),
        installed_name: "a".into(),
    };
    let content_sha256 = sha2::Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let artifact = OwnedArtifact {
        root: root.clone(),
        path: "rules/a.md".into(),
        content: ArtifactContent::File {
            content_sha256,
            mode: Some(0o600),
        },
        owners: vec![owner.clone()],
        consumers: vec![TargetSurface::ClaudeCli],
    };
    CombinedState {
        schema: 2,
        installations: vec![Installation {
            owner,
            root: root.clone(),
            requested: vec![TargetSurface::ClaudeCli],
            source_evidence: SourceEvidence::Local {
                manifest_sha256: "a".repeat(64),
            },
        }],
        bundles: vec![],
        references: vec![Reference::DirectInstallation {
            installation_id: "rule-a".into(),
        }],
        artifacts: vec![artifact],
        owner_consumers: vec![OwnerConsumers {
            root: root.clone(),
            path: "rules/a.md".into(),
            contribution_id: None,
            installation_id: "rule-a".into(),
            consumers: vec![TargetSurface::ClaudeCli],
        }],
        owner_contracts: vec![],
        legacy: None,
    }
}

#[test]
fn first_schema2_write_backs_up_exact_schema1_bytes_before_outputs() {
    let tmp = tempfile::tempdir().unwrap();
    let root_path = tmp.path().join("project");
    std::fs::create_dir(&root_path).unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: root_path.clone(),
    };
    let state_path = root_path.join(".upskill-lock.json");
    let original = b"{ \"schema\": 1, \"unknown\": true }\n";
    std::fs::write(&state_path, original).unwrap();
    std::fs::set_permissions(&state_path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let metadata = std::fs::metadata(&state_path).unwrap();
    let before = FileValue {
        bytes: original.to_vec(),
        mode: Some(metadata.permissions().mode() & 0o777),
    };
    let after = owned_rule_state(&root, b"rule output");
    let expected_state = serialize_state(&after).unwrap();
    let output_path = root_path.join("rules/a.md");
    let result = apply_first_schema2_write(
        &scope,
        &guard,
        FirstSchema2Write {
            plan: FileOperationPlan {
                artifacts: after.artifacts.clone(),
                files: vec![PlannedFile {
                    location: FileLocation {
                        root: InstallationRoot {
                            scope: OwnershipScope::Project,
                            path: root_path.clone(),
                        },
                        path: "rules/a.md".into(),
                    },
                    before: None,
                    after: Some(FileValue {
                        bytes: b"rule output".to_vec(),
                        mode: Some(0o600),
                    }),
                }],
            },
            state_before: before,
            state_location: FileLocation {
                root: root.clone(),
                path: ".upskill-lock.json".into(),
            },
            backup_location: FileLocation {
                root,
                path: ".upskill-lock.json.schema1.bak".into(),
            },
            state_after: &after,
            sources: &[],
        },
    )
    .unwrap();

    assert_eq!(result.outcome, ExecutionOutcome::Complete);
    assert_eq!(
        std::fs::read(state_path).unwrap(),
        expected_state.as_bytes()
    );
    assert_eq!(
        std::fs::metadata(root_path.join(".upskill-lock.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
    assert_eq!(
        std::fs::read(root_path.join(".upskill-lock.json.schema1.bak")).unwrap(),
        original
    );
    assert_eq!(std::fs::read(output_path).unwrap(), b"rule output");
}

#[test]
fn mismatched_artifact_state_binding_rejects_before_backup_or_output() {
    let tmp = tempfile::tempdir().unwrap();
    let root_path = tmp.path().join("project");
    std::fs::create_dir(&root_path).unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: root_path.clone(),
    };
    let state_path = root_path.join(".upskill-lock.json");
    let original = b"{\"schema\":1}\n";
    std::fs::write(&state_path, original).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let mode = std::fs::metadata(&state_path).unwrap().permissions().mode() & 0o777;
    let planned_state = owned_rule_state(&root, b"planned");

    let error = apply_first_schema2_write(
        &scope,
        &guard,
        FirstSchema2Write {
            plan: FileOperationPlan {
                artifacts: vec![],
                files: vec![],
            },
            state_before: FileValue {
                bytes: original.to_vec(),
                mode: Some(mode),
            },
            state_location: FileLocation {
                root: root.clone(),
                path: ".upskill-lock.json".into(),
            },
            backup_location: FileLocation {
                root,
                path: ".upskill-lock.json.schema1.bak".into(),
            },
            state_after: &planned_state,
            sources: &[],
        },
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("complete owned-artifact plan"));

    assert_eq!(std::fs::read(state_path).unwrap(), original);
    assert!(!root_path.join(".upskill-lock.json.schema1.bak").exists());
    assert!(!root_path.join("rules/a.md").exists());
}

#[test]
fn changed_captured_source_rejects_first_write_before_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let root_path = tmp.path().join("project");
    let source_path = tmp.path().join("source");
    std::fs::create_dir(&root_path).unwrap();
    std::fs::create_dir(&source_path).unwrap();
    std::fs::write(source_path.join("rule.md"), b"captured source").unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: root_path.clone(),
    };
    let source_root = InstallationRoot {
        scope: OwnershipScope::User,
        path: source_path.clone(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let source_scope = QualifiedScope::open(&source_root).unwrap();
    let source = source_scope.observe_source_tree().unwrap();
    std::fs::write(source_path.join("rule.md"), b"changed source").unwrap();
    let state_path = root_path.join(".upskill-lock.json");
    let original = b"{\"schema\":1}\n";
    std::fs::write(&state_path, original).unwrap();
    let mode = std::fs::metadata(&state_path).unwrap().permissions().mode() & 0o777;
    let next_state = owned_rule_state(&root, b"compiled output");
    let guard = scope.try_lock_writer().unwrap();
    let sources = [&source];

    assert!(
        apply_first_schema2_write(
            &scope,
            &guard,
            FirstSchema2Write {
                plan: FileOperationPlan {
                    artifacts: next_state.artifacts.clone(),
                    files: vec![PlannedFile {
                        location: FileLocation {
                            root: root.clone(),
                            path: "rules/a.md".into()
                        },
                        before: None,
                        after: Some(FileValue {
                            bytes: b"compiled output".to_vec(),
                            mode: Some(0o600)
                        }),
                    }],
                },
                state_before: FileValue {
                    bytes: original.to_vec(),
                    mode: Some(mode)
                },
                state_location: FileLocation {
                    root: root.clone(),
                    path: ".upskill-lock.json".into()
                },
                backup_location: FileLocation {
                    root,
                    path: ".upskill-lock.json.schema1.bak".into()
                },
                state_after: &next_state,
                sources: &sources,
            }
        )
        .is_err()
    );

    assert_eq!(std::fs::read(state_path).unwrap(), original);
    assert!(!root_path.join(".upskill-lock.json.schema1.bak").exists());
    assert!(!root_path.join("rules/a.md").exists());
}

#[test]
fn changed_captured_source_rejects_later_write_before_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let root_path = tmp.path().join("project");
    let source_path = tmp.path().join("source");
    std::fs::create_dir(&root_path).unwrap();
    std::fs::create_dir(&source_path).unwrap();
    std::fs::write(source_path.join("rule.md"), b"captured source").unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: root_path.clone(),
    };
    let source_root = InstallationRoot {
        scope: OwnershipScope::User,
        path: source_path.clone(),
    };
    let source_scope = QualifiedScope::open(&source_root).unwrap();
    let source = source_scope.observe_source_tree().unwrap();
    std::fs::write(source_path.join("rule.md"), b"changed source").unwrap();
    let prior_state = owned_rule_state(&root, b"old output");
    let next_state = owned_rule_state(&root, b"new output");
    let state_path = root_path.join(".upskill-lock.json");
    let state_bytes = serialize_state(&prior_state).unwrap().into_bytes();
    std::fs::write(&state_path, &state_bytes).unwrap();
    let mode = std::fs::metadata(&state_path).unwrap().permissions().mode() & 0o777;
    let backup_path = root_path.join(".upskill-lock.json.schema1.bak");
    let backup_bytes = b"{\"schema\":1}\n";
    std::fs::write(&backup_path, backup_bytes).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let sources = [&source];
    let output_path = root_path.join("rules/a.md");
    std::fs::create_dir(root_path.join("rules")).unwrap();
    std::fs::write(&output_path, b"old output").unwrap();

    let error = apply_schema2_write(
        &scope,
        &guard,
        ExistingSchema2Write {
            plan: FileOperationPlan {
                artifacts: next_state.artifacts.clone(),
                files: vec![PlannedFile {
                    location: FileLocation {
                        root: root.clone(),
                        path: "rules/a.md".into(),
                    },
                    before: Some(FileValue {
                        bytes: b"old output".to_vec(),
                        mode: Some(0o644),
                    }),
                    after: Some(FileValue {
                        bytes: b"new output".to_vec(),
                        mode: Some(0o644),
                    }),
                }],
            },
            state_before: FileValue {
                bytes: state_bytes.clone(),
                mode: Some(mode),
            },
            state_location: FileLocation {
                root: root.clone(),
                path: ".upskill-lock.json".into(),
            },
            backup_original: FileValue {
                bytes: backup_bytes.to_vec(),
                mode: Some(0o644),
            },
            backup_location: FileLocation {
                root,
                path: ".upskill-lock.json.schema1.bak".into(),
            },
            state_after: &next_state,
            sources: &sources,
        },
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("revalidate retained source"),
        "unexpected rejection: {error:#}"
    );

    assert_eq!(std::fs::read(state_path).unwrap(), state_bytes);
    assert_eq!(std::fs::read(output_path).unwrap(), b"old output");
    assert_eq!(std::fs::read(backup_path).unwrap(), backup_bytes);
}

#[test]
fn mismatched_later_artifact_state_binding_rejects_before_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let root_path = tmp.path().join("project");
    std::fs::create_dir(&root_path).unwrap();
    std::fs::create_dir(root_path.join("rules")).unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: root_path.clone(),
    };
    let prior_state = owned_rule_state(&root, b"old output");
    let next_state = owned_rule_state(&root, b"new output");
    let state_path = root_path.join(".upskill-lock.json");
    let state_bytes = serialize_state(&prior_state).unwrap().into_bytes();
    std::fs::write(&state_path, &state_bytes).unwrap();
    let mode = std::fs::metadata(&state_path).unwrap().permissions().mode() & 0o777;
    let backup_path = root_path.join(".upskill-lock.json.schema1.bak");
    let backup_bytes = b"{\"schema\":1}\n";
    std::fs::write(&backup_path, backup_bytes).unwrap();
    let output_path = root_path.join("rules/a.md");
    std::fs::write(&output_path, b"old output").unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();

    assert!(
        apply_schema2_write(
            &scope,
            &guard,
            ExistingSchema2Write {
                plan: FileOperationPlan {
                    artifacts: vec![],
                    files: vec![PlannedFile {
                        location: FileLocation {
                            root: root.clone(),
                            path: "rules/a.md".into()
                        },
                        before: Some(FileValue {
                            bytes: b"old output".to_vec(),
                            mode: Some(0o644)
                        }),
                        after: Some(FileValue {
                            bytes: b"new output".to_vec(),
                            mode: Some(0o644)
                        }),
                    }],
                },
                state_before: FileValue {
                    bytes: state_bytes.clone(),
                    mode: Some(mode)
                },
                state_location: FileLocation {
                    root: root.clone(),
                    path: ".upskill-lock.json".into()
                },
                backup_original: FileValue {
                    bytes: backup_bytes.to_vec(),
                    mode: Some(0o644)
                },
                backup_location: FileLocation {
                    root,
                    path: ".upskill-lock.json.schema1.bak".into()
                },
                state_after: &next_state,
                sources: &[],
            }
        )
        .is_err()
    );

    assert_eq!(std::fs::read(state_path).unwrap(), state_bytes);
    assert_eq!(std::fs::read(output_path).unwrap(), b"old output");
    assert_eq!(std::fs::read(backup_path).unwrap(), backup_bytes);
}

#[test]
fn mismatched_existing_backup_rejects_transition_without_changing_state() {
    let tmp = tempfile::tempdir().unwrap();
    let root_path = tmp.path().join("project");
    std::fs::create_dir(&root_path).unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: root_path.clone(),
    };
    let state_path = root_path.join(".upskill-lock.json");
    let original = b"{ \"schema\": 1 }\n";
    std::fs::write(&state_path, original).unwrap();
    let backup_path = root_path.join(".upskill-lock.json.schema1.bak");
    std::fs::write(&backup_path, b"wrong backup").unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let metadata = std::fs::metadata(&state_path).unwrap();
    let before = FileValue {
        bytes: original.to_vec(),
        mode: Some(metadata.permissions().mode() & 0o777),
    };
    let after = owned_rule_state(&root, b"new rule");
    let output_path = root_path.join("rules/a.md");

    assert!(
        apply_first_schema2_write(
            &scope,
            &guard,
            FirstSchema2Write {
                plan: FileOperationPlan {
                    artifacts: after.artifacts.clone(),
                    files: vec![PlannedFile {
                        location: FileLocation {
                            root: root.clone(),
                            path: "rules/a.md".into(),
                        },
                        before: None,
                        after: Some(FileValue {
                            bytes: b"new rule".to_vec(),
                            mode: Some(0o600),
                        }),
                    }]
                },
                state_before: before,
                state_location: FileLocation {
                    root: root.clone(),
                    path: ".upskill-lock.json".into()
                },
                backup_location: FileLocation {
                    root,
                    path: ".upskill-lock.json.schema1.bak".into()
                },
                state_after: &after,
                sources: &[],
            },
        )
        .is_err()
    );
    assert_eq!(std::fs::read(state_path).unwrap(), original);
    assert_eq!(std::fs::read(backup_path).unwrap(), b"wrong backup");
    assert!(!output_path.exists());
}

#[test]
fn later_schema2_write_rechecks_the_retained_backup_before_state_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let root_path = tmp.path().join("project");
    std::fs::create_dir(&root_path).unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: root_path.clone(),
    };
    let state_path = root_path.join(".upskill-lock.json");
    let backup_path = root_path.join(".upskill-lock.json.schema1.bak");
    let prior_state = owned_rule_state(&root, b"old rule");
    let output_path = root_path.join("rules/a.md");
    std::fs::create_dir(root_path.join("rules")).unwrap();
    std::fs::write(&output_path, b"old rule").unwrap();
    std::fs::set_permissions(&output_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let state_bytes = serialize_state(&prior_state).unwrap().into_bytes();
    std::fs::write(&state_path, &state_bytes).unwrap();
    std::fs::write(&backup_path, b"{\"schema\":1,\"unexpected\":true}\n").unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let state_mode = std::fs::metadata(&state_path).unwrap().permissions().mode() & 0o777;
    let backup_mode = std::fs::metadata(&backup_path)
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    let mut original_after = owned_rule_state(&root, b"new rule");
    original_after
        .bundles
        .push(upskill::lifecycle::state::BundleNode {
            id: "bundle-a".into(),
            source: "./bundle".into(),
            name: "bundle-a".into(),
            source_revision: None,
        });
    original_after.references.push(Reference::DirectBundle {
        bundle_id: "bundle-a".into(),
    });

    assert!(
        apply_schema2_write(
            &scope,
            &guard,
            ExistingSchema2Write {
                plan: FileOperationPlan {
                    artifacts: original_after.artifacts.clone(),
                    files: vec![PlannedFile {
                        location: FileLocation {
                            root: root.clone(),
                            path: "rules/a.md".into(),
                        },
                        before: Some(FileValue {
                            bytes: b"old rule".to_vec(),
                            mode: Some(0o600),
                        }),
                        after: Some(FileValue {
                            bytes: b"new rule".to_vec(),
                            mode: Some(0o600),
                        }),
                    }]
                },
                state_before: FileValue {
                    bytes: state_bytes.clone(),
                    mode: Some(state_mode)
                },
                state_location: FileLocation {
                    root: root.clone(),
                    path: ".upskill-lock.json".into()
                },
                backup_original: FileValue {
                    bytes: b"{\"schema\":1}\n".to_vec(),
                    mode: Some(backup_mode)
                },
                backup_location: FileLocation {
                    root,
                    path: ".upskill-lock.json.schema1.bak".into()
                },
                state_after: &original_after,
                sources: &[],
            }
        )
        .is_err()
    );
    assert_eq!(std::fs::read(state_path).unwrap(), state_bytes);
    assert_eq!(std::fs::read(&output_path).unwrap(), b"old rule");
    assert_eq!(
        std::fs::read(&backup_path).unwrap(),
        b"{\"schema\":1,\"unexpected\":true}\n"
    );
}

#[test]
fn later_schema2_write_commits_new_state_and_preserves_backup_bytes_and_mode() {
    let tmp = tempfile::tempdir().unwrap();
    let root_path = tmp.path().join("project");
    std::fs::create_dir(&root_path).unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: root_path.clone(),
    };
    let state_path = root_path.join(".upskill-lock.json");
    let backup_path = root_path.join(".upskill-lock.json.schema1.bak");
    let prior_state = owned_rule_state(&root, b"old generation");
    let mut next_state = owned_rule_state(&root, b"second generation");
    next_state
        .bundles
        .push(upskill::lifecycle::state::BundleNode {
            id: "bundle-a".into(),
            source: "./bundle".into(),
            name: "bundle-a".into(),
            source_revision: None,
        });
    next_state
        .references
        .push(upskill::lifecycle::state::Reference::DirectBundle {
            bundle_id: "bundle-a".into(),
        });
    let state_bytes = serialize_state(&prior_state).unwrap().into_bytes();
    let expected_state = serialize_state(&next_state).unwrap().into_bytes();
    let original_backup = b"{\"schema\":1}\n";
    std::fs::write(&state_path, &state_bytes).unwrap();
    std::fs::create_dir(root_path.join("rules")).unwrap();
    let output_path = root_path.join("rules/a.md");
    std::fs::write(&output_path, b"old generation").unwrap();
    std::fs::set_permissions(&output_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(&backup_path, original_backup).unwrap();
    std::fs::set_permissions(&backup_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let state_mode = std::fs::metadata(&state_path).unwrap().permissions().mode() & 0o777;

    let result = apply_schema2_write(
        &scope,
        &guard,
        ExistingSchema2Write {
            plan: FileOperationPlan {
                artifacts: next_state.artifacts.clone(),
                files: vec![PlannedFile {
                    location: FileLocation {
                        root: root.clone(),
                        path: "rules/a.md".into(),
                    },
                    before: Some(FileValue {
                        bytes: b"old generation".to_vec(),
                        mode: Some(0o600),
                    }),
                    after: Some(FileValue {
                        bytes: b"second generation".to_vec(),
                        mode: Some(0o600),
                    }),
                }],
            },
            state_before: FileValue {
                bytes: state_bytes,
                mode: Some(state_mode),
            },
            state_location: FileLocation {
                root: root.clone(),
                path: ".upskill-lock.json".into(),
            },
            backup_original: FileValue {
                bytes: original_backup.to_vec(),
                mode: Some(0o600),
            },
            backup_location: FileLocation {
                root,
                path: ".upskill-lock.json.schema1.bak".into(),
            },
            state_after: &next_state,
            sources: &[],
        },
    )
    .unwrap();

    assert_eq!(result.outcome, ExecutionOutcome::Complete);
    assert_eq!(std::fs::read(state_path).unwrap(), expected_state);
    assert_eq!(std::fs::read(&backup_path).unwrap(), original_backup);
    assert_eq!(
        std::fs::metadata(&backup_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(std::fs::read(output_path).unwrap(), b"second generation");
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
