use upskill::files::plan::{FileLocation, FileOperationPlan, FileValue, PlannedFile};
use upskill::filesystem::{QualifiedScope, execution::ExecutionOutcome};
use upskill::lifecycle::application::{
    ExistingSchema2Write, FirstSchema2Write, apply_first_schema2_write, apply_schema2_write,
};
use upskill::lifecycle::state::{CombinedState, serialize_state};
use upskill::ownership::{InstallationRoot, OwnershipScope};

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
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let metadata = std::fs::metadata(&state_path).unwrap();
    let before = FileValue {
        bytes: original.to_vec(),
        mode: Some(metadata.permissions().mode() & 0o777),
    };
    let after = CombinedState {
        schema: 2,
        installations: vec![],
        bundles: vec![],
        references: vec![],
        artifacts: vec![],
        owner_consumers: vec![],
        owner_contracts: vec![],
        legacy: None,
    };
    let expected_state = serialize_state(&after).unwrap();
    let output_path = root_path.join("rules/a.md");
    let result = apply_first_schema2_write(
        &scope,
        &guard,
        FirstSchema2Write {
            plan: FileOperationPlan {
                artifacts: vec![],
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
        std::fs::read(root_path.join(".upskill-lock.json.schema1.bak")).unwrap(),
        original
    );
    assert_eq!(std::fs::read(output_path).unwrap(), b"rule output");
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
    let after = CombinedState {
        schema: 2,
        installations: vec![],
        bundles: vec![],
        references: vec![],
        artifacts: vec![],
        owner_consumers: vec![],
        owner_contracts: vec![],
        legacy: None,
    };

    assert!(
        apply_first_schema2_write(
            &scope,
            &guard,
            FirstSchema2Write {
                plan: FileOperationPlan {
                    artifacts: vec![],
                    files: vec![]
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
    let prior_state = CombinedState {
        schema: 2,
        installations: vec![],
        bundles: vec![],
        references: vec![],
        artifacts: vec![],
        owner_consumers: vec![],
        owner_contracts: vec![],
        legacy: None,
    };
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
    let original_after = CombinedState { ..prior_state };

    assert!(
        apply_schema2_write(
            &scope,
            &guard,
            ExistingSchema2Write {
                plan: FileOperationPlan {
                    artifacts: vec![],
                    files: vec![]
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
    let prior_state = CombinedState {
        schema: 2,
        installations: vec![],
        bundles: vec![],
        references: vec![],
        artifacts: vec![],
        owner_consumers: vec![],
        owner_contracts: vec![],
        legacy: None,
    };
    let mut next_state = prior_state.clone();
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
    std::fs::write(&backup_path, original_backup).unwrap();
    std::fs::set_permissions(&backup_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let state_mode = std::fs::metadata(&state_path).unwrap().permissions().mode() & 0o777;
    let output_path = root_path.join("rules/a.md");

    let result = apply_schema2_write(
        &scope,
        &guard,
        ExistingSchema2Write {
            plan: FileOperationPlan {
                artifacts: vec![],
                files: vec![PlannedFile {
                    location: FileLocation {
                        root: root.clone(),
                        path: "rules/a.md".into(),
                    },
                    before: None,
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
