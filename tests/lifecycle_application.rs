use upskill::files::plan::{FileLocation, FileOperationPlan, FileValue};
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
    let result = apply_first_schema2_write(
        &scope,
        &guard,
        FirstSchema2Write {
            plan: FileOperationPlan {
                artifacts: vec![],
                files: vec![],
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

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
