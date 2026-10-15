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
