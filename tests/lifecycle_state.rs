use upskill::lifecycle::state::{
    BundleNode, CombinedState, Installation, Reference, SourceEvidence, validate_state,
};
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn root() -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: std::env::temp_dir().join("upskill-state-example"),
    }
}
fn installation() -> Installation {
    Installation {
        owner: ArtifactOwner {
            installation_id: "rule-a".into(),
            source: "./rules".into(),
            source_revision: None,
            kind: ItemKind::Rule,
            source_name: "a".into(),
            installed_name: "a".into(),
        },
        root: root(),
        requested: vec![TargetSurface::ClaudeCli],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: "a".repeat(64),
        },
    }
}
fn bundle(id: &str) -> BundleNode {
    BundleNode {
        id: id.into(),
        source: "./bundle".into(),
        name: id.into(),
        source_revision: None,
    }
}
fn state() -> CombinedState {
    CombinedState {
        schema: 2,
        installations: vec![installation()],
        bundles: vec![bundle("first"), bundle("second")],
        references: vec![
            Reference::DirectBundle {
                bundle_id: "first".into(),
            },
            Reference::DirectBundle {
                bundle_id: "second".into(),
            },
            Reference::BundleInstallation {
                bundle_id: "first".into(),
                installation_id: "rule-a".into(),
            },
            Reference::BundleInstallation {
                bundle_id: "second".into(),
                installation_id: "rule-a".into(),
            },
        ],
        artifacts: vec![],
        owner_consumers: vec![],
        legacy: None,
    }
}
fn rejects(mutator: impl FnOnce(&mut CombinedState)) {
    let mut value = state();
    mutator(&mut value);
    let before = value.clone();
    assert!(validate_state(&value).is_err(), "accepted {value:?}");
    assert_eq!(value, before);
}

#[test]
fn preserves_shared_installation_references() {
    let value = state();
    let before = value.clone();
    validate_state(&value).unwrap();
    assert_eq!(value, before);
}
#[test]
fn allows_referenced_audience_excluded_installation() {
    let mut value = state();
    value.bundles.clear();
    value.references = vec![Reference::DirectInstallation {
        installation_id: "rule-a".into(),
    }];
    validate_state(&value).unwrap();
    assert!(value.artifacts.is_empty());
}
#[test]
fn rejects_dangling_duplicate_and_cyclic_references() {
    for edge in [
        Reference::DirectInstallation {
            installation_id: "absent".into(),
        },
        Reference::DirectBundle {
            bundle_id: "absent".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "absent".into(),
            installation_id: "rule-a".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "first".into(),
            installation_id: "absent".into(),
        },
        Reference::BundleDependency {
            bundle_id: "absent".into(),
            dependency_id: "first".into(),
        },
        Reference::BundleDependency {
            bundle_id: "first".into(),
            dependency_id: "absent".into(),
        },
    ] {
        rejects(|s| s.references.push(edge));
    }
    rejects(|s| s.references.push(s.references[0].clone()));
    rejects(|s| {
        s.references.push(Reference::BundleDependency {
            bundle_id: "first".into(),
            dependency_id: "first".into(),
        })
    });
    rejects(|s| {
        s.references.extend([
            Reference::BundleDependency {
                bundle_id: "first".into(),
                dependency_id: "second".into(),
            },
            Reference::BundleDependency {
                bundle_id: "second".into(),
                dependency_id: "first".into(),
            },
        ])
    });
}
#[test]
fn requires_reachable_nodes_and_allows_transitive_dependencies() {
    rejects(|s| s.references.clear());
    rejects(|s| s.bundles.push(bundle("orphan")));
    rejects(|s| {
        let mut item = installation();
        item.owner.installation_id = "orphan".into();
        item.owner.installed_name = "orphan".into();
        s.installations.push(item);
    });
    let mut value = state();
    value.references = vec![
        Reference::DirectBundle {
            bundle_id: "first".into(),
        },
        Reference::BundleDependency {
            bundle_id: "first".into(),
            dependency_id: "second".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "second".into(),
            installation_id: "rule-a".into(),
        },
    ];
    validate_state(&value).unwrap();
}
#[test]
fn rejects_invalid_installation_and_bundle_fields() {
    rejects(|s| s.schema = 1);
    rejects(|s| s.installations.push(s.installations[0].clone()));
    rejects(|s| s.bundles.push(s.bundles[0].clone()));
    rejects(|s| s.installations[0].root.path = "relative".into());
    rejects(|s| s.installations[0].requested.clear());
    rejects(|s| s.installations[0].requested.push(TargetSurface::ClaudeCli));
    rejects(|s| s.installations[0].owner.source.clear());
    rejects(|s| s.installations[0].owner.source_name.clear());
    rejects(|s| s.installations[0].owner.installed_name.clear());
    rejects(|s| s.installations[0].owner.installation_id = "../unsafe".into());
    rejects(|s| {
        s.installations[0].source_evidence = SourceEvidence::Local {
            manifest_sha256: "A".repeat(64),
        }
    });
    rejects(|s| {
        s.installations[0].source_evidence = SourceEvidence::Local {
            manifest_sha256: "a".repeat(63),
        }
    });
    rejects(|s| s.installations[0].owner.source_revision = Some("revision".into()));
    rejects(|s| s.bundles[0].source.clear());
    rejects(|s| s.bundles[0].name.clear());
    rejects(|s| s.bundles[0].id = "bad/id".into());
    rejects(|s| s.bundles[0].source_revision = Some(String::new()));
}
#[test]
fn git_evidence_requires_matching_nonempty_revision() {
    let mut value = state();
    value.installations[0].source_evidence = SourceEvidence::Git {
        resolved_revision: "abc123".into(),
        manifest_sha256: "b".repeat(64),
    };
    assert!(validate_state(&value).is_err());
    value.installations[0].owner.source_revision = Some("abc123".into());
    validate_state(&value).unwrap();
    value.installations[0].owner.source_revision = Some("different".into());
    assert!(validate_state(&value).is_err());
    value.installations[0].owner.source_revision = Some(String::new());
    value.installations[0].source_evidence = SourceEvidence::Git {
        resolved_revision: String::new(),
        manifest_sha256: "b".repeat(64),
    };
    assert!(validate_state(&value).is_err());
}
#[test]
fn rejects_same_installation_identity_under_two_ids() {
    rejects(|s| {
        let mut item = s.installations[0].clone();
        item.owner.installation_id = "other-id".into();
        s.installations.push(item);
        s.references.push(Reference::DirectInstallation {
            installation_id: "other-id".into(),
        });
    });
}

fn owned_state() -> CombinedState {
    use upskill::lifecycle::state::OwnerConsumers;
    use upskill::ownership::{ArtifactContent, OwnedArtifact};
    let mut value = state();
    value.artifacts.push(OwnedArtifact {
        root: root(),
        path: "rules/a.md".into(),
        content: ArtifactContent::File {
            content_sha256: "b".repeat(64),
            mode: Some(0o644),
        },
        owners: vec![value.installations[0].owner.clone()],
        consumers: vec![TargetSurface::ClaudeCli],
    });
    value.owner_consumers.push(OwnerConsumers {
        root: root(),
        path: "rules/a.md".into(),
        contribution_id: None,
        installation_id: "rule-a".into(),
        consumers: vec![TargetSurface::ClaudeCli],
    });
    value
}
fn rejects_owned(mutator: impl FnOnce(&mut CombinedState)) {
    let mut value = owned_state();
    mutator(&mut value);
    let before = value.clone();
    assert!(validate_state(&value).is_err(), "accepted {value:?}");
    assert_eq!(value, before);
}
#[test]
fn rejects_artifact_owner_binding_drift() {
    rejects_owned(|s| s.artifacts[0].owners[0].source_revision = Some("different".into()));
    rejects_owned(|s| s.artifacts[0].owners[0].installed_name = "different".into());
    rejects_owned(|s| s.artifacts[0].owners[0].source = "other/source".into());
    rejects_owned(|s| s.artifacts[0].owners[0].kind = ItemKind::Skill);
    rejects_owned(|s| s.artifacts[0].owners[0].source_name = "different".into());
    rejects_owned(|s| s.artifacts[0].owners[0].installation_id = "unknown".into());
    rejects_owned(|s| s.artifacts[0].root.path = s.artifacts[0].root.path.join("different"));
    rejects_owned(|s| s.artifacts[0].root.scope = OwnershipScope::User);
    rejects_owned(|s| {
        s.artifacts[0].root.path = s.artifacts[0].root.path.join(".");
        s.owner_consumers[0].root = s.artifacts[0].root.clone();
    });
}
#[test]
fn requires_exact_per_owner_consumer_union() {
    validate_state(&owned_state()).unwrap();
    rejects_owned(|s| s.owner_consumers.clear());
    rejects_owned(|s| s.owner_consumers.push(s.owner_consumers[0].clone()));
    rejects_owned(|s| s.owner_consumers[0].path = "other.md".into());
    rejects_owned(|s| s.owner_consumers[0].contribution_id = Some("absent".into()));
    rejects_owned(|s| s.owner_consumers[0].installation_id = "unknown".into());
    rejects_owned(|s| s.owner_consumers[0].root.scope = OwnershipScope::User);
    rejects_owned(|s| s.owner_consumers[0].root.path = s.owner_consumers[0].root.path.join("."));
    rejects_owned(|s| s.owner_consumers[0].consumers.clear());
    rejects_owned(|s| {
        s.owner_consumers[0]
            .consumers
            .push(TargetSurface::ClaudeCli)
    });
    rejects_owned(|s| s.owner_consumers[0].consumers.push(TargetSurface::CodexCli));
    rejects_owned(|s| s.artifacts[0].consumers.push(TargetSurface::CodexCli));
    rejects_owned(|s| {
        s.installations[0].requested.push(TargetSurface::CopilotCli);
        s.owner_consumers[0].consumers = vec![TargetSurface::CopilotCli];
    });
}
#[test]
fn keeps_independent_consumers_for_shared_artifact_owners() {
    let mut value = owned_state();
    let mut second = installation();
    second.owner.installation_id = "rule-b".into();
    second.owner.source_name = "b".into();
    second.owner.installed_name = "b".into();
    second.requested = vec![TargetSurface::CopilotCli];
    value.artifacts[0].owners.push(second.owner.clone());
    value.artifacts[0].consumers.push(TargetSurface::CopilotCli);
    let mut association = value.owner_consumers[0].clone();
    association.installation_id = "rule-b".into();
    association.consumers = vec![TargetSurface::CopilotCli];
    value.owner_consumers.push(association);
    value.installations.push(second);
    value.references.push(Reference::DirectInstallation {
        installation_id: "rule-b".into(),
    });
    let before = value.clone();
    validate_state(&value).unwrap();
    assert_eq!(value, before);
    value.owner_consumers[1].consumers = vec![TargetSurface::ClaudeCli];
    assert!(validate_state(&value).is_err());
}
#[test]
fn keeps_root_scope_and_contribution_selectors_distinct() {
    use upskill::instructions::aggregate::OwnedContribution;
    use upskill::ownership::ArtifactContent;
    let mut value = owned_state();
    value.artifacts[0].path = "AGENTS.md".into();
    value.owner_consumers[0].path = "AGENTS.md".into();
    for id in ["first", "second"] {
        let mut artifact = value.artifacts[0].clone();
        artifact.content = ArtifactContent::MarkdownContribution {
            contribution: OwnedContribution {
                id: id.into(),
                content_sha256: "b".repeat(64),
                leading_newlines: 1,
                preceding_newlines: 3,
            },
        };
        let mut association = value.owner_consumers[0].clone();
        association.contribution_id = Some(id.into());
        value.artifacts.push(artifact);
        value.owner_consumers.push(association);
    }
    value.artifacts.remove(0);
    value.owner_consumers.remove(0);
    validate_state(&value).unwrap();
    let mut second = installation();
    second.owner.installation_id = "rule-b".into();
    second.root.scope = OwnershipScope::User;
    let mut artifact = value.artifacts[0].clone();
    artifact.root = second.root.clone();
    artifact.owners = vec![second.owner.clone()];
    let mut association = value.owner_consumers[0].clone();
    association.root = second.root.clone();
    association.installation_id = "rule-b".into();
    value.installations.push(second);
    value.references.push(Reference::DirectInstallation {
        installation_id: "rule-b".into(),
    });
    value.artifacts.push(artifact);
    value.owner_consumers.push(association);
    validate_state(&value).unwrap();
    value.owner_consumers.pop();
    assert!(validate_state(&value).is_err());
}

#[test]
fn validates_id_syntax_with_consistent_reference_endpoints() {
    for invalid in ["", "bad/id", "with space", "é"] {
        let mut value = state();
        value.bundles.clear();
        value.installations[0].owner.installation_id = invalid.into();
        value.references = vec![Reference::DirectInstallation {
            installation_id: invalid.into(),
        }];
        assert!(
            validate_state(&value).is_err(),
            "invalid installation ID: {invalid:?}"
        );
        value.installations.clear();
        value.bundles = vec![bundle(invalid)];
        value.references = vec![Reference::DirectBundle {
            bundle_id: invalid.into(),
        }];
        assert!(
            validate_state(&value).is_err(),
            "invalid bundle ID: {invalid:?}"
        );
    }
}
#[test]
fn duplicate_ids_cannot_hide_behind_distinct_installation_identities() {
    rejects(|s| {
        let mut item = s.installations[0].clone();
        item.owner.installed_name = "distinct-alias".into();
        s.installations.push(item);
    });
}
#[test]
fn validates_endpoints_even_when_reachability_counts_balance() {
    let mut value = state();
    value.bundles.clear();
    value.references = vec![Reference::DirectInstallation {
        installation_id: "unknown".into(),
    }];
    assert!(validate_state(&value).is_err());
    value.installations.clear();
    value.bundles = vec![bundle("real")];
    value.references = vec![Reference::DirectBundle {
        bundle_id: "unknown".into(),
    }];
    assert!(validate_state(&value).is_err());
    value.installations = vec![installation()];
    value.references = vec![
        Reference::DirectBundle {
            bundle_id: "real".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "real".into(),
            installation_id: "unknown".into(),
        },
    ];
    assert!(validate_state(&value).is_err());
}
#[test]
fn rejects_duplicate_edges_of_every_kind() {
    let mut value = state();
    value.references.push(Reference::DirectInstallation {
        installation_id: "rule-a".into(),
    });
    value.references.push(Reference::BundleDependency {
        bundle_id: "first".into(),
        dependency_id: "second".into(),
    });
    validate_state(&value).unwrap();
    for reference in value.references.clone() {
        let mut duplicate = value.clone();
        duplicate.references.push(reference);
        assert!(validate_state(&duplicate).is_err());
    }
}
#[test]
fn validates_artifact_shape_and_retained_legacy_schema() {
    rejects_owned(|s| {
        s.artifacts[0].content = upskill::ownership::ArtifactContent::File {
            content_sha256: "z".repeat(64),
            mode: Some(0o644),
        }
    });
    rejects_owned(|s| {
        s.artifacts[0].content = upskill::ownership::ArtifactContent::File {
            content_sha256: "b".repeat(64),
            mode: Some(0o10000),
        }
    });
    rejects(|s| {
        s.installations[0].source_evidence = SourceEvidence::Local {
            manifest_sha256: "z".repeat(64),
        }
    });
    rejects(|s| {
        let mut legacy = upskill::lockfile::Lockfile::new();
        legacy.schema = 2;
        s.legacy = Some(legacy);
    });
}
#[test]
fn installation_identity_distinguishes_all_binding_fields_and_typed_ids() {
    for field in 0..6 {
        let mut value = state();
        let mut item = installation();
        item.owner.installation_id = "first".into();
        match field {
            0 => item.owner.source = "different-source".into(),
            1 => item.owner.kind = ItemKind::Agent,
            2 => item.owner.source_name = "different-name".into(),
            3 => item.owner.installed_name = "different-alias".into(),
            4 => item.root.path = item.root.path.join("different"),
            5 => item.root.scope = OwnershipScope::User,
            _ => unreachable!(),
        }
        value.installations.push(item);
        value.references.push(Reference::DirectInstallation {
            installation_id: "first".into(),
        });
        validate_state(&value).unwrap();
    }
}
