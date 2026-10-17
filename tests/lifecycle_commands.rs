#![cfg(any(target_os = "linux", target_os = "macos"))]

use upskill::filesystem::QualifiedScope;
use upskill::lifecycle::commands::{LifecycleSession, SessionOutcome};
use upskill::ownership::{InstallationRoot, OwnershipScope};

#[test]
fn fresh_session_captures_state_without_creating_files_and_retains_writer() {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let SessionOutcome::Ready(session) = LifecycleSession::open(&scope, &guard, root).unwrap()
    else {
        panic!("unexpected recovery")
    };
    assert!(session.state().installations.is_empty());
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
    assert!(scope.try_lock_writer().is_err());
}

use upskill::files::permissions::ResourceExecutionPolicy;
use upskill::files::plan::{DesiredFile, FileLocation, FileValue};
use upskill::filesystem::{ExecutionOutcome, SourceSnapshot};
use upskill::lifecycle::commands::{CapturedUpsert, CommandDiagnostic, PrepareOutcome};
use upskill::lifecycle::desired::DesiredInstallationSet;
use upskill::lifecycle::selection::Selection;
use upskill::lifecycle::state::{Installation, Reference, SourceEvidence};
use upskill::operations::plan::DesiredArtifact;
use upskill::ownership::ArtifactOwner;
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn supplied(root: &InstallationRoot, source: &SourceSnapshot<'_>, text: &[u8]) -> CapturedUpsert {
    let installation = Installation {
        owner: ArtifactOwner {
            installation_id: "owner".into(),
            source: "./source".into(),
            source_revision: None,
            kind: ItemKind::Rule,
            source_name: "example".into(),
            installed_name: "example".into(),
        },
        root: root.clone(),
        requested: vec![TargetSurface::ClaudeCli],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: source.manifest().content_sha256.clone(),
        },
    };
    CapturedUpsert {
        sets: vec![DesiredInstallationSet {
            installation,
            artifacts: vec![DesiredArtifact::File(DesiredFile {
                location: FileLocation {
                    root: root.clone(),
                    path: "rules/example.md".into(),
                },
                value: FileValue {
                    bytes: text.into(),
                    mode: None,
                },
                consumers: vec![TargetSurface::ClaudeCli],
            })],
            audience_excluded: vec![],
        }],
        add_references: vec![Reference::DirectInstallation {
            installation_id: "owner".into(),
        }],
        ..Default::default()
    }
}

#[test]
fn fresh_and_migrated_add_update_remove_preserve_extra_files_and_exact_backup() {
    for migrated in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().into(),
        };
        let original = b"{\"schema\":1,\"items\":[],\"future-field\":true}\n";
        if migrated {
            std::fs::write(tmp.path().join(".upskill-lock.json"), original).unwrap();
        }
        std::fs::create_dir(tmp.path().join("source")).unwrap();
        std::fs::write(tmp.path().join("source/RULE.md"), b"captured").unwrap();
        let source_scope = QualifiedScope::open(&InstallationRoot {
            path: tmp.path().join("source"),
            ..root.clone()
        })
        .unwrap();
        let source = source_scope.observe_source_tree().unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let PrepareOutcome::Ready(prepared) = session
            .prepare_upsert(None, &[&source], ResourceExecutionPolicy::Disabled, |_| {
                Ok(supplied(&root, &source, b"first"))
            })
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            prepared.apply().unwrap().outcome,
            ExecutionOutcome::Complete
        );
        std::fs::write(tmp.path().join("rules/extra.md"), b"manual").unwrap();
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let selection = Selection::Names(vec!["example".into()]);
        let PrepareOutcome::Ready(prepared) = session
            .prepare_upsert(
                Some(&selection),
                &[&source],
                ResourceExecutionPolicy::Disabled,
                |_| {
                    let mut request = supplied(&root, &source, b"second");
                    request.add_references.clear();
                    Ok(request)
                },
            )
            .unwrap()
        else {
            panic!()
        };
        prepared.apply().unwrap();
        assert_eq!(
            std::fs::read(tmp.path().join("rules/example.md")).unwrap(),
            b"second"
        );
        let SessionOutcome::Ready(session) = LifecycleSession::open(&scope, &guard, root).unwrap()
        else {
            panic!()
        };
        let PrepareOutcome::Ready(prepared) = session.prepare_remove(&selection).unwrap() else {
            panic!()
        };
        prepared.apply().unwrap();
        assert!(!tmp.path().join("rules/example.md").exists());
        assert_eq!(
            std::fs::read(tmp.path().join("rules/extra.md")).unwrap(),
            b"manual"
        );
        let backup = tmp.path().join(".upskill-lock.json.schema1.bak");
        if migrated {
            assert_eq!(std::fs::read(backup).unwrap(), original);
        } else {
            assert!(!backup.exists());
        }
    }
}

#[test]
fn stale_state_source_or_destination_rejects_without_partial_output() {
    for changed in ["state", "source", "destination"] {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().join("output"),
        };
        std::fs::create_dir(&root.path).unwrap();
        let source_root = InstallationRoot {
            path: tmp.path().join("source"),
            ..root.clone()
        };
        std::fs::create_dir(&source_root.path).unwrap();
        std::fs::write(source_root.path.join("RULE.md"), b"captured").unwrap();
        let source_scope = QualifiedScope::open(&source_root).unwrap();
        let source = source_scope.observe_source_tree().unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let PrepareOutcome::Ready(prepared) = session
            .prepare_upsert(None, &[&source], ResourceExecutionPolicy::Disabled, |_| {
                Ok(supplied(&root, &source, b"first"))
            })
            .unwrap()
        else {
            panic!()
        };
        match changed {
            "state" => std::fs::write(root.path.join(".upskill-lock.json"), b"late state").unwrap(),
            "source" => std::fs::write(source_root.path.join("RULE.md"), b"late source").unwrap(),
            _ => {
                std::fs::create_dir(root.path.join("rules")).unwrap();
                std::fs::write(root.path.join("rules/example.md"), b"late destination").unwrap();
            }
        }
        assert!(prepared.apply().is_err(), "{changed}");
        assert!(!root.path.join(".upskill/state/operation.json").exists());
        if changed != "state" {
            assert!(!root.path.join(".upskill-lock.json").exists());
        }
        if changed != "destination" {
            assert!(!root.path.join("rules/example.md").exists());
        }
    }
}

#[test]
fn complete_selection_blocks_before_builder_and_wrong_guard_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    assert!(matches!(
        session
            .prepare_upsert(
                Some(&Selection::Names(vec!["missing".into()])),
                &[],
                ResourceExecutionPolicy::Disabled,
                |_| panic!("blocked selection invoked builder")
            )
            .unwrap(),
        PrepareOutcome::Blocked(_)
    ));
    let other = tempfile::tempdir().unwrap();
    let other_root = InstallationRoot {
        path: other.path().into(),
        ..root
    };
    let other_scope = QualifiedScope::open(&other_root).unwrap();
    assert!(LifecycleSession::open(&other_scope, &guard, other_root).is_err());
}

#[test]
fn reserved_destinations_and_unbound_sources_are_rejected_before_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().join("output"),
    };
    std::fs::create_dir(&root.path).unwrap();
    let source_root = InstallationRoot {
        path: tmp.path().join("source"),
        ..root.clone()
    };
    std::fs::create_dir(&source_root.path).unwrap();
    std::fs::write(source_root.path.join("RULE.md"), b"captured").unwrap();
    let source_scope = QualifiedScope::open(&source_root).unwrap();
    let source = source_scope.observe_source_tree().unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    assert!(
        session
            .prepare_upsert(None, &[], ResourceExecutionPolicy::Disabled, |_| Ok(
                supplied(&root, &source, b"first")
            ))
            .is_err()
    );
    for path in [
        ".upskill-lock.json",
        ".upskill-lock.json.schema1.bak",
        ".UPSKILL-LOCK.JSON",
        ".upskill/state/other.json",
        ".upskill/state",
        ".upskill/state/operation.json/child",
    ] {
        assert!(
            session
                .prepare_upsert(None, &[&source], ResourceExecutionPolicy::Disabled, |_| {
                    let mut request = supplied(&root, &source, b"first");
                    let DesiredArtifact::File(file) = &mut request.sets[0].artifacts[0] else {
                        panic!()
                    };
                    file.location.path = path.into();
                    Ok(request)
                })
                .is_err()
        );
    }
    assert_eq!(std::fs::read_dir(&root.path).unwrap().count(), 0);
}

use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use upskill::instructions::plan::{DesiredInstructionContribution, InstructionLocation};
use upskill::lifecycle::contracts::{ConsumerContract, ContractContent, OwnerContract};
use upskill::lifecycle::state::BundleNode;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn instruction_request(
    root: &InstallationRoot,
    source: &SourceSnapshot<'_>,
    markdown: &str,
) -> CapturedUpsert {
    let mut request = supplied(root, source, b"unused");
    request.sets[0].artifacts = vec![DesiredArtifact::Instruction(
        DesiredInstructionContribution {
            location: InstructionLocation {
                root: root.clone(),
                path: "AGENTS.md".into(),
            },
            id: "example".into(),
            markdown: markdown.into(),
            consumers: vec![TargetSurface::ClaudeCli],
        },
    )];
    request
}

#[test]
fn instruction_add_update_remove_preserves_manual_bytes_and_ordinary_mode() {
    for existing in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().join("output"),
        };
        std::fs::create_dir(&root.path).unwrap();
        if existing {
            std::fs::write(root.path.join("AGENTS.md"), b"Manual prefix\n").unwrap();
            std::fs::set_permissions(
                root.path.join("AGENTS.md"),
                std::fs::Permissions::from_mode(0o640),
            )
            .unwrap();
        }
        let source_root = InstallationRoot {
            path: tmp.path().join("source"),
            ..root.clone()
        };
        std::fs::create_dir(&source_root.path).unwrap();
        std::fs::write(source_root.path.join("RULE.md"), b"captured").unwrap();
        let source_scope = QualifiedScope::open(&source_root).unwrap();
        let source = source_scope.observe_source_tree().unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let PrepareOutcome::Ready(prepared) = session
            .prepare_upsert(None, &[&source], ResourceExecutionPolicy::Disabled, |_| {
                Ok(instruction_request(&root, &source, "First body\n"))
            })
            .unwrap()
        else {
            panic!()
        };
        prepared.apply().unwrap();
        let mode = std::fs::metadata(root.path.join("AGENTS.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, if existing { 0o640 } else { 0o600 });
        let selection = Selection::Names(vec!["example".into()]);
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let PrepareOutcome::Ready(prepared) = session
            .prepare_upsert(
                Some(&selection),
                &[&source],
                ResourceExecutionPolicy::Disabled,
                |_| {
                    let mut request = instruction_request(&root, &source, "Second body\n");
                    request.add_references.clear();
                    Ok(request)
                },
            )
            .unwrap()
        else {
            panic!()
        };
        prepared.apply().unwrap();
        assert_eq!(
            std::fs::metadata(root.path.join("AGENTS.md"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            if existing { 0o640 } else { 0o600 }
        );
        let bytes = std::fs::read_to_string(root.path.join("AGENTS.md")).unwrap();
        assert!(bytes.contains("Second body"));
        if existing {
            assert!(bytes.starts_with("Manual prefix\n"));
        }
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let PrepareOutcome::Ready(prepared) = session.prepare_remove(&selection).unwrap() else {
            panic!()
        };
        prepared.apply().unwrap();
        assert_eq!(
            std::fs::metadata(root.path.join("AGENTS.md"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            if existing { 0o640 } else { 0o600 }
        );
        assert_eq!(
            std::fs::read(root.path.join("AGENTS.md")).unwrap(),
            if existing {
                b"Manual prefix\n".as_slice()
            } else {
                b"".as_slice()
            }
        );
    }
}

#[test]
fn bundle_removal_preserves_independent_member_root_and_member_only_remove_is_blocked() {
    for independent in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().join("output"),
        };
        std::fs::create_dir(&root.path).unwrap();
        let source_root = InstallationRoot {
            path: tmp.path().join("source"),
            ..root.clone()
        };
        std::fs::create_dir(&source_root.path).unwrap();
        std::fs::write(source_root.path.join("RULE.md"), b"captured").unwrap();
        let source_scope = QualifiedScope::open(&source_root).unwrap();
        let source = source_scope.observe_source_tree().unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let PrepareOutcome::Ready(prepared) = session
            .prepare_upsert(None, &[&source], ResourceExecutionPolicy::Disabled, |_| {
                let mut request = supplied(&root, &source, b"member");
                if !independent {
                    request.add_references.clear();
                }
                request.bundles.push(BundleNode {
                    id: "bundle".into(),
                    name: "bundle".into(),
                    source: "./source".into(),
                    source_revision: None,
                });
                request.add_references.extend([
                    Reference::DirectBundle {
                        bundle_id: "bundle".into(),
                    },
                    Reference::BundleInstallation {
                        bundle_id: "bundle".into(),
                        installation_id: "owner".into(),
                    },
                ]);
                Ok(request)
            })
            .unwrap()
        else {
            panic!()
        };
        prepared.apply().unwrap();
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        if !independent {
            assert!(
                matches!(session.prepare_remove(&Selection::Names(vec!["example".into()])).unwrap(), PrepareOutcome::Blocked(d) if d == vec![CommandDiagnostic::NoDirectReference("example".into())])
            );
        }
        let PrepareOutcome::Ready(prepared) = session
            .prepare_remove(&Selection::Names(vec!["bundle".into()]))
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(!prepared.state().installations.is_empty(), independent);
        prepared.apply().unwrap();
        assert_eq!(root.path.join("rules/example.md").exists(), independent);
    }
}

#[test]
fn selected_resource_modes_and_owner_contracts_agree_and_existing_modes_survive() {
    for (existing, policy, expected) in [
        (false, ResourceExecutionPolicy::Disabled, 0o600),
        (false, ResourceExecutionPolicy::OwnerExecutable, 0o700),
        (true, ResourceExecutionPolicy::OwnerExecutable, 0o640),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().join("output"),
        };
        std::fs::create_dir(&root.path).unwrap();
        let source_root = InstallationRoot {
            path: tmp.path().join("source"),
            ..root.clone()
        };
        std::fs::create_dir(&source_root.path).unwrap();
        std::fs::write(source_root.path.join("RULE.md"), b"captured").unwrap();
        let source_scope = QualifiedScope::open(&source_root).unwrap();
        let source = source_scope.observe_source_tree().unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        if existing {
            let SessionOutcome::Ready(session) =
                LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
            else {
                panic!()
            };
            let PrepareOutcome::Ready(prepared) = session
                .prepare_upsert(None, &[&source], ResourceExecutionPolicy::Disabled, |_| {
                    Ok(supplied(&root, &source, b"first"))
                })
                .unwrap()
            else {
                panic!()
            };
            prepared.apply().unwrap();
            // Preserve ordinary modes from the original owned record: prepare the
            // initial installation at this mode through a supplied existing file.
            let state_path = root.path.join(".upskill-lock.json");
            let mut state = upskill::lifecycle::state::parse_state(
                &std::fs::read_to_string(&state_path).unwrap(),
            )
            .unwrap();
            let upskill::ownership::ArtifactContent::File { mode, .. } =
                &mut state.artifacts[0].content
            else {
                panic!()
            };
            *mode = Some(0o640);
            std::fs::set_permissions(
                root.path.join("rules/example.md"),
                std::fs::Permissions::from_mode(0o640),
            )
            .unwrap();
            std::fs::write(
                &state_path,
                upskill::lifecycle::state::serialize_state(&state).unwrap(),
            )
            .unwrap();
        }
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let selection = Selection::Names(vec!["example".into()]);
        let PrepareOutcome::Ready(prepared) = session
            .prepare_upsert(existing.then_some(&selection), &[&source], policy, |_| {
                let mut request = supplied(&root, &source, b"resource");
                if existing {
                    request.add_references.clear();
                }
                let DesiredArtifact::File(file) = &mut request.sets[0].artifacts[0] else {
                    panic!()
                };
                file.value.mode = Some(0o755);
                request.contracts.push(OwnerContract {
                    root: root.clone(),
                    path: file.location.path.clone(),
                    contribution_id: None,
                    installation_id: "owner".into(),
                    source_manifest_sha256: source.manifest().content_sha256.clone(),
                    content: ContractContent::File {
                        content_sha256: hash(b"resource"),
                        mode: Some(0o755),
                    },
                    profiles: vec![ConsumerContract {
                        surface: TargetSurface::ClaudeCli,
                        contract_revision: 1,
                    }],
                });
                Ok(request)
            })
            .unwrap()
        else {
            panic!()
        };
        assert!(
            matches!(prepared.state().owner_contracts[0].content, ContractContent::File { mode: Some(mode), .. } if mode == expected)
        );
        prepared.apply().unwrap();
        assert_eq!(
            std::fs::metadata(root.path.join("rules/example.md"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            expected
        );
    }
}

#[test]
fn all_selection_rejects_legacy_before_building_desired_values() {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    // Use the production legacy type to keep this example independent of optional fields.
    let mut legacy = upskill::lockfile::Lockfile::default();
    legacy.bundles.push(
        serde_json::from_value(
            serde_json::json!({"name":"legacy","source":"old","clients":[],"skills":[]}),
        )
        .unwrap(),
    );
    let bytes = serde_json::to_vec(&legacy).unwrap();
    std::fs::write(tmp.path().join(".upskill-lock.json"), &bytes).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let SessionOutcome::Ready(session) = LifecycleSession::open(&scope, &guard, root).unwrap()
    else {
        panic!()
    };
    assert!(matches!(
        session
            .prepare_upsert(
                Some(&Selection::All),
                &[],
                ResourceExecutionPolicy::Disabled,
                |_| panic!("legacy preflight must precede builder")
            )
            .unwrap(),
        PrepareOutcome::Blocked(_)
    ));
    assert!(matches!(
        session.prepare_remove(&Selection::All).unwrap(),
        PrepareOutcome::Blocked(_)
    ));
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        bytes
    );
    assert!(!tmp.path().join(".upskill-lock.json.schema1.bak").exists());
}

#[test]
fn shared_file_contracts_retain_unselected_owner_and_edited_outputs_block() {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().join("output"),
    };
    std::fs::create_dir(&root.path).unwrap();
    let source_root = InstallationRoot {
        path: tmp.path().join("source"),
        ..root.clone()
    };
    std::fs::create_dir(&source_root.path).unwrap();
    std::fs::write(source_root.path.join("RULE.md"), b"captured").unwrap();
    let source_scope = QualifiedScope::open(&source_root).unwrap();
    let source = source_scope.observe_source_tree().unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    let PrepareOutcome::Ready(prepared) = session
        .prepare_upsert(None, &[&source], ResourceExecutionPolicy::Disabled, |_| {
            let mut request = supplied(&root, &source, b"shared");
            let mut second = request.sets[0].clone();
            second.installation.owner.installation_id = "second".into();
            second.installation.owner.installed_name = "second".into();
            request.sets.push(second);
            request.add_references.push(Reference::DirectInstallation {
                installation_id: "second".into(),
            });
            for id in ["owner", "second"] {
                request.contracts.push(OwnerContract {
                    root: root.clone(),
                    path: "rules/example.md".into(),
                    contribution_id: None,
                    installation_id: id.into(),
                    source_manifest_sha256: source.manifest().content_sha256.clone(),
                    content: ContractContent::File {
                        content_sha256: hash(b"shared"),
                        mode: None,
                    },
                    profiles: vec![ConsumerContract {
                        surface: TargetSurface::ClaudeCli,
                        contract_revision: 1,
                    }],
                });
            }
            Ok(request)
        })
        .unwrap()
    else {
        panic!()
    };
    prepared.apply().unwrap();
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    let PrepareOutcome::Ready(prepared) = session
        .prepare_remove(&Selection::Names(vec!["example".into()]))
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(prepared.state().artifacts[0].owners.len(), 1);
    prepared.apply().unwrap();
    assert_eq!(
        std::fs::read(root.path.join("rules/example.md")).unwrap(),
        b"shared"
    );
    let state_before = std::fs::read(root.path.join(".upskill-lock.json")).unwrap();
    std::fs::write(root.path.join("rules/example.md"), b"edited").unwrap();
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    assert!(
        session
            .prepare_remove(&Selection::Names(vec!["second".into()]))
            .is_err()
    );
    assert_eq!(
        std::fs::read(root.path.join("rules/example.md")).unwrap(),
        b"edited"
    );
    assert_eq!(
        std::fs::read(root.path.join(".upskill-lock.json")).unwrap(),
        state_before
    );
}

#[test]
fn updating_and_removing_one_instruction_preserves_an_unselected_contribution() {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().join("output"),
    };
    std::fs::create_dir(&root.path).unwrap();
    std::fs::write(root.path.join("AGENTS.md"), b"Manual prefix\n").unwrap();
    let source_root = InstallationRoot {
        path: tmp.path().join("source"),
        ..root.clone()
    };
    std::fs::create_dir(&source_root.path).unwrap();
    std::fs::write(source_root.path.join("RULE.md"), b"captured").unwrap();
    let source_scope = QualifiedScope::open(&source_root).unwrap();
    let source = source_scope.observe_source_tree().unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    let PrepareOutcome::Ready(prepared) = session
        .prepare_upsert(None, &[&source], ResourceExecutionPolicy::Disabled, |_| {
            let mut request = instruction_request(&root, &source, "Selected body\n");
            let mut second = request.sets[0].clone();
            second.installation.owner.installation_id = "second".into();
            second.installation.owner.installed_name = "second".into();
            let DesiredArtifact::Instruction(contribution) = &mut second.artifacts[0] else {
                panic!()
            };
            contribution.id = "second".into();
            contribution.markdown = "Retained body\n".into();
            request.sets.push(second);
            request.add_references.push(Reference::DirectInstallation {
                installation_id: "second".into(),
            });
            Ok(request)
        })
        .unwrap()
    else {
        panic!()
    };
    prepared.apply().unwrap();
    let selection = Selection::Names(vec!["example".into()]);
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    let PrepareOutcome::Ready(prepared) = session
        .prepare_upsert(
            Some(&selection),
            &[&source],
            ResourceExecutionPolicy::Disabled,
            |_| {
                let mut request = instruction_request(&root, &source, "Updated body\n");
                request.add_references.clear();
                Ok(request)
            },
        )
        .unwrap()
    else {
        panic!()
    };
    prepared.apply().unwrap();
    let content = std::fs::read_to_string(root.path.join("AGENTS.md")).unwrap();
    assert!(content.starts_with("Manual prefix\n"));
    assert!(content.contains("Updated body\n"));
    assert!(content.contains("Retained body\n"));
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    let PrepareOutcome::Ready(prepared) = session.prepare_remove(&selection).unwrap() else {
        panic!()
    };
    prepared.apply().unwrap();
    let content = std::fs::read_to_string(root.path.join("AGENTS.md")).unwrap();
    assert!(content.starts_with("Manual prefix\n"));
    assert!(content.contains("Retained body\n"));
    assert!(!content.contains("Updated body"));
}

#[test]
fn each_desired_installation_requires_its_matching_nonempty_source_capture() {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().join("output"),
    };
    std::fs::create_dir(&root.path).unwrap();
    let mut source_roots = Vec::new();
    for (name, bytes) in [
        ("matching", b"matching".as_slice()),
        ("unrelated", b"unrelated".as_slice()),
    ] {
        let source_root = InstallationRoot {
            path: tmp.path().join(name),
            ..root.clone()
        };
        std::fs::create_dir(&source_root.path).unwrap();
        std::fs::write(source_root.path.join("RULE.md"), bytes).unwrap();
        source_roots.push(QualifiedScope::open(&source_root).unwrap());
    }
    let matching = source_roots[0].observe_source_tree().unwrap();
    let unrelated = source_roots[1].observe_source_tree().unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    let result = session.prepare_upsert(
        None,
        &[&unrelated],
        ResourceExecutionPolicy::Disabled,
        |_| Ok(supplied(&root, &matching, b"desired")),
    );
    assert!(
        matches!(result, Err(error) if error.to_string().contains("matching captured source snapshot"))
    );
    let multi = || {
        let mut request = supplied(&root, &matching, b"first");
        let mut second = supplied(&root, &unrelated, b"second").sets.remove(0);
        second.installation.owner.installation_id = "second".into();
        second.installation.owner.installed_name = "second".into();
        let DesiredArtifact::File(file) = &mut second.artifacts[0] else {
            panic!()
        };
        file.location.path = "rules/second.md".into();
        request.sets.push(second);
        request.add_references.push(Reference::DirectInstallation {
            installation_id: "second".into(),
        });
        request
    };
    let result = session.prepare_upsert(
        None,
        &[&matching],
        ResourceExecutionPolicy::Disabled,
        |_| Ok(multi()),
    );
    assert!(
        matches!(result, Err(error) if error.to_string().contains("matching captured source snapshot"))
    );
    assert_eq!(std::fs::read_dir(&root.path).unwrap().count(), 0);
    assert!(matches!(
        session
            .prepare_upsert(
                None,
                &[&matching],
                ResourceExecutionPolicy::Disabled,
                |_| Ok(supplied(&root, &matching, b"desired"))
            )
            .unwrap(),
        PrepareOutcome::Ready(_)
    ));
    let PrepareOutcome::Ready(prepared) = session
        .prepare_upsert(
            None,
            &[&matching, &unrelated],
            ResourceExecutionPolicy::Disabled,
            |_| Ok(multi()),
        )
        .unwrap()
    else {
        panic!()
    };
    prepared.apply().unwrap();
    assert_eq!(
        std::fs::read(root.path.join("rules/second.md")).unwrap(),
        b"second"
    );
}

#[test]
fn mixed_owned_and_missing_or_legacy_selection_blocks_the_entire_request() {
    for legacy in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().join("output"),
        };
        std::fs::create_dir(&root.path).unwrap();
        if legacy {
            let mut lock = upskill::lockfile::Lockfile::default();
            lock.bundles.push(upskill::lockfile::LockedBundle {
                name: "blocked".into(),
                source: "old".into(),
                git_ref: None,
                items: vec![],
            });
            std::fs::write(
                root.path.join(".upskill-lock.json"),
                serde_json::to_vec(&lock).unwrap(),
            )
            .unwrap();
        }
        let source_root = InstallationRoot {
            path: tmp.path().join("source"),
            ..root.clone()
        };
        std::fs::create_dir(&source_root.path).unwrap();
        std::fs::write(source_root.path.join("RULE.md"), b"captured").unwrap();
        let source_scope = QualifiedScope::open(&source_root).unwrap();
        let source = source_scope.observe_source_tree().unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let PrepareOutcome::Ready(prepared) = session
            .prepare_upsert(None, &[&source], ResourceExecutionPolicy::Disabled, |_| {
                Ok(supplied(&root, &source, b"owned"))
            })
            .unwrap()
        else {
            panic!()
        };
        prepared.apply().unwrap();
        let original_state = std::fs::read(root.path.join(".upskill-lock.json")).unwrap();
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let selection = Selection::Names(vec!["example".into(), "blocked".into()]);
        let PrepareOutcome::Blocked(diagnostics) = session
            .prepare_upsert(
                Some(&selection),
                &[&source],
                ResourceExecutionPolicy::Disabled,
                |_| panic!("mixed blocked selection invoked builder"),
            )
            .unwrap()
        else {
            panic!("mixed request must have no ready subset")
        };
        use upskill::lifecycle::selection::{LegacyRow, SelectionDiagnostic};
        assert_eq!(
            diagnostics,
            vec![CommandDiagnostic::Selection(if legacy {
                SelectionDiagnostic::LegacyRow(LegacyRow::Bundle(0))
            } else {
                SelectionDiagnostic::UnknownName("blocked".into())
            })]
        );
        assert!(matches!(
            session.prepare_remove(&selection).unwrap(),
            PrepareOutcome::Blocked(_)
        ));
        assert_eq!(
            std::fs::read(root.path.join(".upskill-lock.json")).unwrap(),
            original_state
        );
        assert_eq!(
            std::fs::read(root.path.join("rules/example.md")).unwrap(),
            b"owned"
        );
        assert!(!root.path.join(".upskill/state/operation.json").exists());
    }
}

#[test]
fn stale_metadata_or_source_before_preparation_stops_before_builder() {
    for stale in ["metadata", "source"] {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().join("output"),
        };
        std::fs::create_dir(&root.path).unwrap();
        let source_root = InstallationRoot {
            path: tmp.path().join("source"),
            ..root.clone()
        };
        std::fs::create_dir(&source_root.path).unwrap();
        std::fs::write(source_root.path.join("RULE.md"), b"captured").unwrap();
        let source_scope = QualifiedScope::open(&source_root).unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        let source = source_scope.observe_source_tree().unwrap();
        if stale == "metadata" {
            std::fs::write(root.path.join(".upskill-lock.json"), b"late metadata").unwrap();
        } else {
            std::fs::write(source_root.path.join("RULE.md"), b"late source").unwrap();
        }
        assert!(
            session
                .prepare_upsert(
                    None,
                    &[&source],
                    ResourceExecutionPolicy::Disabled,
                    |_| panic!("stale evidence invoked builder")
                )
                .is_err()
        );
        assert!(!root.path.join("rules").exists());
        assert!(!root.path.join(".upskill/state/operation.json").exists());
    }
}

#[test]
fn session_open_rejects_a_mismatching_existing_schema1_backup_without_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    let state = b"{\"schema\":1}\n";
    let backup = b"{\"schema\":1,\"unexpected\":true}\n";
    std::fs::write(root.path.join(".upskill-lock.json"), state).unwrap();
    let backup_path = root.path.join(".upskill-lock.json.schema1.bak");
    std::fs::write(&backup_path, backup).unwrap();
    std::fs::set_permissions(&backup_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    assert!(
        matches!(LifecycleSession::open(&scope, &guard, root.clone()), Err(error) if error.to_string().contains("backup differs"))
    );
    assert_eq!(
        std::fs::read(root.path.join(".upskill-lock.json")).unwrap(),
        state
    );
    assert_eq!(std::fs::read(backup_path).unwrap(), backup);
    assert_eq!(std::fs::read_dir(&root.path).unwrap().count(), 2);
}

#[test]
fn generated_and_resource_paths_keep_distinct_selected_modes_and_contracts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().join("output"),
    };
    std::fs::create_dir(&root.path).unwrap();
    let source_root = InstallationRoot {
        path: tmp.path().join("source"),
        ..root.clone()
    };
    std::fs::create_dir(&source_root.path).unwrap();
    std::fs::write(source_root.path.join("RULE.md"), b"captured").unwrap();
    let source_scope = QualifiedScope::open(&source_root).unwrap();
    let source = source_scope.observe_source_tree().unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    let outcome = session
        .prepare_upsert(
            None,
            &[&source],
            ResourceExecutionPolicy::OwnerExecutable,
            |_| {
                let mut request = supplied(&root, &source, b"resource");
                let mut second = request.sets[0].artifacts[0].clone();
                let DesiredArtifact::File(file) = &mut second else {
                    panic!()
                };
                file.location.path = "rules/script.sh".into();
                file.value.mode = Some(0o755);
                request.sets[0].artifacts.push(second);
                for artifact in &request.sets[0].artifacts {
                    let DesiredArtifact::File(file) = artifact else {
                        panic!()
                    };
                    request.contracts.push(OwnerContract {
                        root: root.clone(),
                        path: file.location.path.clone(),
                        contribution_id: None,
                        installation_id: "owner".into(),
                        source_manifest_sha256: source.manifest().content_sha256.clone(),
                        content: ContractContent::File {
                            content_sha256: hash(b"resource"),
                            mode: file.value.mode,
                        },
                        profiles: vec![ConsumerContract {
                            surface: TargetSurface::ClaudeCli,
                            contract_revision: 1,
                        }],
                    });
                }
                Ok(request)
            },
        )
        .unwrap();
    let PrepareOutcome::Ready(prepared) = outcome else {
        panic!("two paths failed preparation")
    };
    for (path, expected) in [("rules/example.md", 0o600), ("rules/script.sh", 0o700)] {
        let c = prepared
            .state()
            .owner_contracts
            .iter()
            .find(|c| c.path == path)
            .unwrap();
        assert!(
            matches!(c.content, ContractContent::File { mode: Some(mode), .. } if mode == expected)
        );
    }
    prepared.apply().unwrap();
    for (path, expected) in [("rules/example.md", 0o600), ("rules/script.sh", 0o700)] {
        assert_eq!(
            std::fs::metadata(root.path.join(path))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            expected
        );
    }
}
