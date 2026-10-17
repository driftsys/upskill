use upskill::lifecycle::state::{
    CombinedState, StateOrigin, parse_state, project_legacy, serialize_state,
};

fn empty() -> CombinedState {
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

#[test]
fn strict_origin_codec_retains_unbound_inspection_and_exact_backup_binding() {
    let original = br#"{ "schema":1,"unknown":[1,2,3] }"#;
    let mut migrated = project_legacy(&serde_json::from_slice(original).unwrap()).unwrap();
    migrated.origin = Some(StateOrigin::from_schema1_bytes(original).unwrap());
    assert_eq!(
        parse_state(&serialize_state(&migrated).unwrap()).unwrap(),
        migrated
    );
    let Some(StateOrigin::Migrated {
        backup_sha256,
        backup_length,
    }) = migrated.origin
    else {
        panic!("expected migrated origin")
    };
    assert_eq!(backup_length, original.len() as u64);
    assert_eq!(
        backup_sha256,
        "d2e224e9469d35da57da217ed2f881b279020dcda1e3f5e61c1d9565e4001b66"
    );
    assert_ne!(
        StateOrigin::from_schema1_bytes(original).unwrap(),
        StateOrigin::from_schema1_bytes(b"{\"schema\":1}").unwrap()
    );
    let unbound = empty();
    let bytes = serialize_state(&unbound).unwrap();
    assert!(!bytes.contains("origin"));
    assert_eq!(parse_state(&bytes).unwrap(), unbound);
    for invalid in [b"{}".as_slice(), b"{\"schema\":2}", b"broken"] {
        assert!(StateOrigin::from_schema1_bytes(invalid).is_err());
    }
}

#[test]
fn malformed_or_inconsistent_origin_is_rejected_without_pruning_legacy() {
    let mut state = empty();
    state.origin = Some(StateOrigin::Fresh {});
    assert!(serialize_state(&state).is_ok());
    state.legacy = Some(upskill::lockfile::Lockfile::default());
    assert!(serialize_state(&state).is_err());
    state.origin = Some(StateOrigin::Migrated {
        backup_sha256: "x".repeat(64),
        backup_length: 1,
    });
    assert!(serialize_state(&state).is_err());
    state.origin = Some(StateOrigin::Migrated {
        backup_sha256: "a".repeat(64),
        backup_length: 0,
    });
    assert!(serialize_state(&state).is_err());
    state.origin = Some(StateOrigin::Migrated {
        backup_sha256: "a".repeat(64),
        backup_length: 1,
    });
    assert!(serialize_state(&state).is_ok());
    state.legacy = None;
    assert!(serialize_state(&state).is_err());
    let mut json = serde_json::to_value(empty()).unwrap();
    json["origin"] = serde_json::json!({"kind":"fresh","unknown":true});
    assert!(parse_state(&json.to_string()).is_err());
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod application {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use upskill::files::plan::{FileLocation, FileOperationPlan, FileValue, PlannedFile};
    use upskill::filesystem::{ExecutionOutcome, QualifiedScope};
    use upskill::lifecycle::application::{
        BoundSchema2Write, SCHEMA1_BACKUP_NAME, apply_bound_schema2_write,
    };
    use upskill::ownership::{InstallationRoot, OwnershipScope};

    fn root(path: &std::path::Path) -> InstallationRoot {
        InstallationRoot {
            scope: OwnershipScope::Project,
            path: path.into(),
        }
    }
    fn location(root: &InstallationRoot, path: &str) -> FileLocation {
        FileLocation {
            root: root.clone(),
            path: path.into(),
        }
    }
    fn observed(path: &std::path::Path) -> FileValue {
        FileValue {
            bytes: std::fs::read(path).unwrap(),
            mode: Some(std::fs::metadata(path).unwrap().permissions().mode() & 0o7777),
        }
    }
    fn operation<'a>(
        root: &InstallationRoot,
        before: Option<FileValue>,
        backup: Option<FileValue>,
        after: &'a CombinedState,
        output: &str,
    ) -> BoundSchema2Write<'a, 'a> {
        BoundSchema2Write {
            plan: FileOperationPlan {
                artifacts: after.artifacts.clone(),
                files: vec![PlannedFile {
                    location: location(root, output),
                    before: None,
                    after: Some(FileValue {
                        bytes: b"planned output".to_vec(),
                        mode: Some(0o600),
                    }),
                }],
            },
            state_before: before,
            backup_before: backup,
            state_location: location(root, ".upskill-lock.json"),
            backup_location: location(root, SCHEMA1_BACKUP_NAME),
            state_after: after,
            sources: &[],
        }
    }

    #[test]
    fn fresh_initialization_and_reopened_write_require_no_invented_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let root = root(tmp.path());
        let mut state = empty();
        state.origin = Some(StateOrigin::Fresh {});
        {
            let scope = QualifiedScope::open(&root).unwrap();
            let guard = scope.try_lock_writer().unwrap();
            let result = apply_bound_schema2_write(
                &scope,
                &guard,
                operation(&root, None, None, &state, "first.md"),
            )
            .unwrap();
            assert_eq!(result.outcome, ExecutionOutcome::Complete);
        }
        let path = tmp.path().join(".upskill-lock.json");
        assert_eq!(observed(&path).mode, Some(0o600));
        assert_eq!(
            parse_state(std::str::from_utf8(&observed(&path).bytes).unwrap()).unwrap(),
            state
        );
        assert!(!tmp.path().join(SCHEMA1_BACKUP_NAME).exists());
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        apply_bound_schema2_write(
            &scope,
            &guard,
            operation(&root, Some(observed(&path)), None, &state, "second.md"),
        )
        .unwrap();
        assert!(!tmp.path().join(SCHEMA1_BACKUP_NAME).exists());
        assert_eq!(
            std::fs::read(tmp.path().join("first.md")).unwrap(),
            b"planned output"
        );
        assert_eq!(
            std::fs::read(tmp.path().join("second.md")).unwrap(),
            b"planned output"
        );
    }

    #[test]
    fn migrated_origin_rejects_replaced_backup_after_scope_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let root = root(tmp.path());
        let original = b"{ \"schema\":1,\"unknown\":true }\n";
        let path = tmp.path().join(".upskill-lock.json");
        std::fs::write(&path, original).unwrap();
        let mut state = project_legacy(&serde_json::from_slice(original).unwrap()).unwrap();
        state.origin = Some(StateOrigin::from_schema1_bytes(original).unwrap());
        {
            let scope = QualifiedScope::open(&root).unwrap();
            let guard = scope.try_lock_writer().unwrap();
            apply_bound_schema2_write(
                &scope,
                &guard,
                operation(&root, Some(observed(&path)), None, &state, "first.md"),
            )
            .unwrap();
        }
        let backup_path = tmp.path().join(SCHEMA1_BACKUP_NAME);
        assert_eq!(
            observed(&backup_path),
            FileValue {
                bytes: original.to_vec(),
                mode: Some(0o600)
            }
        );
        {
            let scope = QualifiedScope::open(&root).unwrap();
            let guard = scope.try_lock_writer().unwrap();
            apply_bound_schema2_write(
                &scope,
                &guard,
                operation(
                    &root,
                    Some(observed(&path)),
                    Some(observed(&backup_path)),
                    &state,
                    "second.md",
                ),
            )
            .unwrap();
        }
        let before = observed(&path);
        let changed = b"{ \"schema\":1,\"unknown\":null }\n";
        assert_eq!(changed.len(), original.len());
        std::fs::write(&backup_path, changed).unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let error = apply_bound_schema2_write(
            &scope,
            &guard,
            operation(
                &root,
                Some(before.clone()),
                Some(observed(&backup_path)),
                &state,
                "blocked.md",
            ),
        )
        .unwrap_err();
        assert!(error.to_string().contains("backup"), "{error:#}");
        assert_eq!(observed(&path), before);
        assert!(!tmp.path().join("blocked.md").exists());
        assert!(!scope.has_pending_operation().unwrap());
        std::fs::write(&backup_path, original).unwrap();
        std::fs::set_permissions(&backup_path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            apply_bound_schema2_write(
                &scope,
                &guard,
                operation(
                    &root,
                    Some(before.clone()),
                    Some(observed(&backup_path)),
                    &state,
                    "bad-mode.md"
                )
            )
            .is_err()
        );
        assert_eq!(observed(&path), before);
        assert!(!tmp.path().join("bad-mode.md").exists());
        std::fs::set_permissions(&backup_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        for candidate in [
            {
                let mut candidate = state.clone();
                candidate.origin = None;
                candidate
            },
            {
                let mut candidate = state.clone();
                candidate.origin = Some(StateOrigin::from_schema1_bytes(changed).unwrap());
                candidate
            },
            {
                let mut candidate = state.clone();
                candidate.legacy = None;
                candidate
            },
            {
                let mut candidate = state.clone();
                candidate.legacy = Some(serde_json::from_str(r#"{"schema":1,"items":[{"kind":"skill","name":"extra","source":"./extra"}]}"#).unwrap());
                candidate
            },
        ] {
            assert!(
                apply_bound_schema2_write(
                    &scope,
                    &guard,
                    operation(
                        &root,
                        Some(before.clone()),
                        Some(observed(&backup_path)),
                        &candidate,
                        "changed-origin.md"
                    )
                )
                .is_err()
            );
            assert_eq!(observed(&path), before);
            assert!(!tmp.path().join("changed-origin.md").exists());
        }
    }

    #[test]
    fn unknown_origin_changes_and_preexisting_fresh_backup_block_before_effects() {
        let tmp = tempfile::tempdir().unwrap();
        let root = root(tmp.path());
        let path = tmp.path().join(".upskill-lock.json");
        let mut state = empty();
        let unbound = serialize_state(&state).unwrap().into_bytes();
        std::fs::write(&path, &unbound).unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        assert!(
            apply_bound_schema2_write(
                &scope,
                &guard,
                operation(&root, Some(observed(&path)), None, &state, "blocked.md")
            )
            .is_err()
        );
        state.origin = Some(StateOrigin::Fresh {});
        assert!(
            apply_bound_schema2_write(
                &scope,
                &guard,
                operation(&root, Some(observed(&path)), None, &state, "blocked.md")
            )
            .is_err()
        );
        std::fs::remove_file(&path).unwrap();
        let backup_path = tmp.path().join(SCHEMA1_BACKUP_NAME);
        std::fs::write(&backup_path, b"historical backup").unwrap();
        assert!(
            apply_bound_schema2_write(
                &scope,
                &guard,
                operation(
                    &root,
                    None,
                    Some(observed(&backup_path)),
                    &state,
                    "blocked.md"
                )
            )
            .is_err()
        );
        // Even supplied absence cannot bypass the executor's actual observation.
        assert!(
            apply_bound_schema2_write(
                &scope,
                &guard,
                operation(&root, None, None, &state, "blocked.md")
            )
            .is_err()
        );
        assert!(!path.exists());
        assert!(!tmp.path().join("blocked.md").exists());
        assert_eq!(std::fs::read(&backup_path).unwrap(), b"historical backup");
    }
}

#[test]
fn planners_preserve_bound_origin_for_empty_complete_operations() {
    use upskill::lifecycle::upsert::{
        UpsertOutcome, UpsertRequest, plan_shared_upsert, plan_upsert,
    };
    let mut state = empty();
    state.origin = Some(StateOrigin::Fresh {});
    let request = UpsertRequest {
        selection: None,
        contracts: &[],
        sets: &[],
        bundles: &[],
        add_references: &[],
        remove_references: &[],
    };
    for outcome in [
        plan_upsert(&state, request, &[]).unwrap(),
        plan_shared_upsert(&state, request, &[]).unwrap(),
    ] {
        let UpsertOutcome::Ready(plan) = outcome else {
            panic!("expected ready")
        };
        assert_eq!(plan.state, state);
    }
    assert_eq!(
        upskill::lifecycle::retire::plan_reference_removal(&state, &[], &[])
            .unwrap()
            .state,
        state
    );
    let bytes = serialize_state(&state).unwrap().into_bytes();
    let upskill::lifecycle::preview::PreviewOutcome::Ready(plan) =
        upskill::lifecycle::preview::preview(
            &bytes,
            upskill::lifecycle::preview::PreviewRequest::RemoveReferences(&[]),
            &[],
        )
        .unwrap()
    else {
        panic!("expected preview")
    };
    assert_eq!(plan.state, state);
}
