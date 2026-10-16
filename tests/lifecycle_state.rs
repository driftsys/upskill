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
        owner_contracts: Vec::new(),
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

fn legacy_state() -> upskill::lockfile::Lockfile {
    use upskill::lockfile::*;
    let mut legacy = Lockfile::new();
    let item = LockedItem {
        kind: ItemKind::Skill,
        name: "z-alias".into(),
        source: "gitlab+host:group/repo@v1:skills/a".into(),
        git_ref: Some("v1".into()),
        hash: Some("c".repeat(64)),
        source_name: Some("original".into()),
        required_by: vec!["rule:z".into(), "rule:a".into(), "rule:z".into()],
        group: Some("skills/a".into()),
        clients: vec!["unknown".into()],
    };
    legacy.items = vec![item.clone(), item.clone(), item.clone(), item];
    legacy.items[3].name = "a-distinct".into();
    legacy.items[1].source = "malformed::label@".into();
    legacy.items[1].hash = None;
    legacy.items[1].git_ref = None;
    legacy.items[1].source_name = None;
    legacy.items[1].group = None;
    let bundle = LockedBundle {
        name: "z-bundle".into(),
        source: "github:org/repo@main:bundle".into(),
        git_ref: Some("main".into()),
        items: vec!["z-alias".into(), "a".into(), "z-alias".into()],
    };
    legacy.bundles = vec![bundle.clone(), bundle.clone(), bundle.clone(), bundle];
    legacy.bundles[1].name = "a-bundle".into();
    legacy.bundles[3].name = "c-bundle".into();
    for status in [
        PluginInstallStatus::Skipped,
        PluginInstallStatus::Instructions,
        PluginInstallStatus::Installed,
    ] {
        legacy.plugins.push(LockedPlugin {
            name: "p".into(),
            client: "future-client".into(),
            identifier: "p@source".into(),
            scope: None,
            bundle: "z-bundle".into(),
            status,
        });
    }
    for status in [McpInstallStatus::Skipped, McpInstallStatus::Installed] {
        legacy.mcps.push(LockedMcp {
            name: "m".into(),
            client: "future-client".into(),
            scope: Some("project".into()),
            bundle: "z-bundle".into(),
            status,
        });
    }
    legacy
}
#[test]
fn round_trip_preserves_legacy_rows_labels_and_statuses() {
    use upskill::lifecycle::state::{parse_state, project_legacy, serialize_state};
    let legacy = legacy_state();
    let before = legacy.clone();
    let mut value = project_legacy(&legacy).unwrap();
    assert!(
        value.installations.is_empty()
            && value.artifacts.is_empty()
            && value.references.is_empty()
            && value.bundles.is_empty()
            && value.owner_consumers.is_empty()
    );
    assert_eq!(legacy, before);
    assert_eq!(value.legacy.as_ref(), Some(&legacy));
    let mut owned = owned_state();
    owned.legacy = value.legacy.take();
    let text = serialize_state(&owned).unwrap();
    let parsed = parse_state(&text).unwrap();
    assert_eq!(parsed.legacy, Some(legacy));
    assert_eq!(parsed.artifacts, owned.artifacts);
    assert_eq!(parsed.owner_consumers, owned.owner_consumers);
    assert_eq!(upskill::lockfile::CURRENT_SCHEMA, 1);
    assert!(text.ends_with('\n') && !text.ends_with("\n\n"));
    assert!(text.starts_with("{\n  \"schema\": 2,\n"));
    let mut bad = before;
    bad.schema = 2;
    assert!(project_legacy(&bad).is_err());
}
#[test]
fn rejects_unknown_schema2_fields() {
    use serde_json::{Value, json};
    use upskill::lifecycle::state::{parse_state, serialize_state};
    let original: Value = serde_json::from_str(&serialize_state(&owned_state()).unwrap()).unwrap();
    for pointer in [
        "",
        "/installations/0",
        "/installations/0/owner",
        "/installations/0/root",
        "/installations/0/source_evidence",
        "/bundles/0",
        "/references/0",
        "/artifacts/0",
        "/artifacts/0/root",
        "/artifacts/0/owners/0",
        "/artifacts/0/content",
        "/owner_consumers/0",
        "/owner_consumers/0/root",
    ] {
        let mut value = original.clone();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        assert!(
            parse_state(&value.to_string()).is_err(),
            "accepted unknown field at {pointer}"
        );
    }
    let mut value = original;
    value["schema"] = json!(3);
    assert!(parse_state(&value.to_string()).is_err());
    assert!(parse_state(&serde_json::to_string(&legacy_state()).unwrap()).is_err());
    assert!(parse_state("not JSON").is_err());
}
#[test]
fn serialization_is_deterministic_without_input_mutation() {
    use upskill::lifecycle::state::{parse_state, serialize_state};
    let mut first = owned_state();
    first.installations[0]
        .requested
        .push(TargetSurface::CopilotCli);
    first.artifacts[0].consumers.push(TargetSurface::CopilotCli);
    first.owner_consumers[0]
        .consumers
        .push(TargetSurface::CopilotCli);
    let mut second = installation();
    second.owner.installation_id = "rule-b".into();
    second.owner.source_name = "b".into();
    second.owner.installed_name = "b".into();
    first.installations.push(second.clone());
    first.references.push(Reference::DirectInstallation {
        installation_id: "rule-b".into(),
    });
    first.artifacts[0].owners.push(second.owner);
    let mut row = first.owner_consumers[0].clone();
    row.installation_id = "rule-b".into();
    row.consumers = vec![TargetSurface::ClaudeCli];
    first.owner_consumers.push(row);
    let mut artifact = first.artifacts[0].clone();
    artifact.path = "rules/z.md".into();
    first.artifacts.push(artifact);
    let mut rows = first.owner_consumers.clone();
    for row in &mut rows {
        row.path = "rules/z.md".into();
    }
    first.owner_consumers.extend(rows);
    first.legacy = Some(legacy_state());
    let before = first.clone();
    let mut permuted = first.clone();
    permuted.installations.reverse();
    permuted.bundles.reverse();
    permuted.references.reverse();
    permuted.artifacts.reverse();
    permuted.owner_consumers.reverse();
    for i in &mut permuted.installations {
        i.requested.reverse();
    }
    for a in &mut permuted.artifacts {
        a.owners.reverse();
        a.consumers.reverse();
    }
    for a in &mut permuted.owner_consumers {
        a.consumers.reverse();
    }
    assert_eq!(
        serialize_state(&first).unwrap(),
        serialize_state(&permuted).unwrap()
    );
    assert_eq!(first, before);
    let canonical = parse_state(&serialize_state(&permuted).unwrap()).unwrap();
    assert_eq!(
        canonical
            .installations
            .iter()
            .map(|i| i.owner.installation_id.as_str())
            .collect::<Vec<_>>(),
        vec!["rule-a", "rule-b"]
    );
    assert_eq!(
        canonical
            .artifacts
            .iter()
            .map(|a| a.path.as_str())
            .collect::<Vec<_>>(),
        vec!["rules/a.md", "rules/z.md"]
    );
    assert_eq!(
        canonical.installations[0].requested,
        vec![TargetSurface::ClaudeCli, TargetSurface::CopilotCli]
    );
    assert_eq!(canonical.legacy, before.legacy);
}
#[test]
fn codec_validates_values_in_both_directions_and_accepts_empty_owned_state() {
    use upskill::lifecycle::state::{parse_state, project_legacy, serialize_state};
    let mut value = project_legacy(&upskill::lockfile::Lockfile::new()).unwrap();
    value.legacy = None;
    let text = serialize_state(&value).unwrap();
    assert_eq!(parse_state(&text).unwrap(), value);
    value.installations.push(installation());
    assert!(serialize_state(&value).is_err());
    assert!(parse_state(&serde_json::to_string(&value).unwrap()).is_err());
}
#[test]
fn typed_legacy_projection_does_not_claim_unknown_json_field_retention() {
    use upskill::lifecycle::state::{parse_state, project_legacy, serialize_state};
    let raw = "{\"schema\":1,\"items\":[],\"bundles\":[],\"future\":{\"x\":1}}";
    let legacy: upskill::lockfile::Lockfile = serde_json::from_str(raw).unwrap();
    let combined = project_legacy(&legacy).unwrap();
    let serialized = serialize_state(&combined).unwrap();
    assert!(!serialized.contains("future"));
    assert_eq!(parse_state(&serialized).unwrap().legacy, Some(legacy));
}
#[test]
fn contribution_codec_preserves_newline_and_selector_metadata() {
    use upskill::instructions::aggregate::OwnedContribution;
    use upskill::lifecycle::state::{parse_state, serialize_state};
    use upskill::ownership::ArtifactContent;
    let mut value = owned_state();
    value.artifacts[0].content = ArtifactContent::MarkdownContribution {
        contribution: OwnedContribution {
            id: "region-a".into(),
            content_sha256: "d".repeat(64),
            leading_newlines: 2,
            preceding_newlines: 7,
        },
    };
    value.owner_consumers[0].contribution_id = Some("region-a".into());
    let text = serialize_state(&value).unwrap();
    assert_eq!(parse_state(&text).unwrap(), value);
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["artifacts"][0]["content"]["contribution"]["extra"] = serde_json::json!(1);
    assert!(parse_state(&json.to_string()).is_err());
}

#[test]
fn requested_surface_check_is_independent_of_artifact_union() {
    rejects_owned(|s| {
        s.owner_consumers[0].consumers = vec![TargetSurface::CodexCli];
        s.artifacts[0].consumers = vec![TargetSurface::CodexCli];
    });
}
#[test]
fn authoritative_root_scope_check_is_independent_of_association_lookup() {
    rejects_owned(|s| {
        s.artifacts[0].root.scope = OwnershipScope::User;
        s.owner_consumers[0].root.scope = OwnershipScope::User;
    });
}
#[test]
fn rejects_surplus_association_without_losing_required_associations() {
    rejects_owned(|s| {
        let mut row = s.owner_consumers[0].clone();
        row.path = "absent.md".into();
        s.owner_consumers.push(row);
    });
}
#[test]
fn requires_nonempty_association_when_another_owner_supplies_the_union() {
    let mut value = owned_state();
    let mut item = installation();
    item.owner.installation_id = "rule-b".into();
    item.owner.installed_name = "b".into();
    value.artifacts[0].owners.push(item.owner.clone());
    value.installations.push(item);
    value.references.push(Reference::DirectInstallation {
        installation_id: "rule-b".into(),
    });
    let mut row = value.owner_consumers[0].clone();
    row.installation_id = "rule-b".into();
    value.owner_consumers.push(row);
    validate_state(&value).unwrap();
    value.owner_consumers[1].consumers.clear();
    assert!(validate_state(&value).is_err());
}
#[test]
fn contribution_selectors_keep_distinct_consumer_sets() {
    use upskill::instructions::aggregate::OwnedContribution;
    use upskill::ownership::ArtifactContent;
    let mut value = owned_state();
    value.installations[0]
        .requested
        .push(TargetSurface::CopilotCli);
    value.artifacts.clear();
    value.owner_consumers.clear();
    for (id, surface) in [
        ("first", TargetSurface::ClaudeCli),
        ("second", TargetSurface::CopilotCli),
    ] {
        let mut artifact = owned_state().artifacts.remove(0);
        artifact.path = "AGENTS.md".into();
        artifact.consumers = vec![surface];
        artifact.content = ArtifactContent::MarkdownContribution {
            contribution: OwnedContribution {
                id: id.into(),
                content_sha256: "b".repeat(64),
                leading_newlines: 1,
                preceding_newlines: 3,
            },
        };
        let mut row = owned_state().owner_consumers.remove(0);
        row.path = artifact.path.clone();
        row.contribution_id = Some(id.into());
        row.consumers = vec![surface];
        value.artifacts.push(artifact);
        value.owner_consumers.push(row);
    }
    validate_state(&value).unwrap();
    value.owner_consumers[0].consumers = vec![TargetSurface::CopilotCli];
    value.owner_consumers[1].consumers = vec![TargetSurface::ClaudeCli];
    assert!(validate_state(&value).is_err());
}

#[test]
fn canonical_codec_orders_roots_scopes_and_contribution_selectors() {
    use upskill::instructions::aggregate::OwnedContribution;
    use upskill::lifecycle::state::{parse_state, project_legacy, serialize_state};
    use upskill::ownership::ArtifactContent;
    let mut value = project_legacy(&upskill::lockfile::Lockfile::new()).unwrap();
    value.legacy = None;
    for (scope, suffix, owner_id) in [
        (OwnershipScope::User, "a", "a-owner"),
        (OwnershipScope::Project, "z", "m-owner"),
        (OwnershipScope::Project, "a", "z-owner"),
    ] {
        let mut item = installation();
        item.root.scope = scope;
        item.root.path = root().path.join(suffix);
        item.owner.installation_id = owner_id.into();
        for id in ["z-region", "a-region"] {
            let mut artifact = owned_state().artifacts.remove(0);
            artifact.root = item.root.clone();
            artifact.path = "AGENTS.md".into();
            artifact.owners = vec![item.owner.clone()];
            artifact.content = ArtifactContent::MarkdownContribution {
                contribution: OwnedContribution {
                    id: id.into(),
                    content_sha256: "b".repeat(64),
                    leading_newlines: 1,
                    preceding_newlines: 3,
                },
            };
            let mut association = owned_state().owner_consumers.remove(0);
            association.root = item.root.clone();
            association.path = artifact.path.clone();
            association.contribution_id = Some(id.into());
            association.installation_id = owner_id.into();
            value.artifacts.push(artifact);
            value.owner_consumers.push(association);
        }
        value.references.push(Reference::DirectInstallation {
            installation_id: owner_id.into(),
        });
        value.installations.push(item);
    }
    let serialized = serialize_state(&value).unwrap();
    let mut reversed = value.clone();
    reversed.artifacts.reverse();
    reversed.owner_consumers.reverse();
    assert_eq!(serialize_state(&reversed).unwrap(), serialized);
    let canonical = parse_state(&serialized).unwrap();
    let artifact_order: Vec<_> = canonical
        .artifacts
        .iter()
        .map(|a| match &a.content {
            ArtifactContent::MarkdownContribution { contribution } => (
                a.owners[0].installation_id.as_str(),
                contribution.id.as_str(),
            ),
            _ => panic!("expected contribution"),
        })
        .collect();
    let expected = vec![
        ("z-owner", "a-region"),
        ("z-owner", "z-region"),
        ("m-owner", "a-region"),
        ("m-owner", "z-region"),
        ("a-owner", "a-region"),
        ("a-owner", "z-region"),
    ];
    assert_eq!(artifact_order, expected);
    assert_eq!(
        canonical
            .owner_consumers
            .iter()
            .map(|a| (
                a.installation_id.as_str(),
                a.contribution_id.as_deref().unwrap()
            ))
            .collect::<Vec<_>>(),
        expected
    );
}
#[test]
fn git_evidence_and_bundle_dependencies_survive_codec_round_trip() {
    use upskill::lifecycle::state::{parse_state, serialize_state};
    let mut value = state();
    let revision = "0123456789abcdef0123456789abcdef01234567";
    value.installations[0].owner.source_revision = Some(revision.into());
    value.installations[0].source_evidence = SourceEvidence::Git {
        resolved_revision: revision.into(),
        manifest_sha256: "c".repeat(64),
    };
    value.bundles[0].source_revision = Some("bundle-pinned-revision".into());
    value.references.push(Reference::BundleDependency {
        bundle_id: "first".into(),
        dependency_id: "second".into(),
    });
    assert_eq!(
        parse_state(&serialize_state(&value).unwrap()).unwrap(),
        value
    );
}

#[test]
fn rejects_association_superset_of_artifact_consumers() {
    rejects_owned(|s| {
        s.installations[0].requested.push(TargetSurface::CopilotCli);
        s.owner_consumers[0]
            .consumers
            .push(TargetSurface::CopilotCli);
    });
}

#[test]
fn revisions_and_requested_surfaces_do_not_split_installation_identity() {
    let mut value = state();
    let first_revision = "0123456789abcdef0123456789abcdef01234567";
    let second_revision = "89abcdef0123456789abcdef0123456789abcdef";
    value.installations[0].owner.source = "https://example.test/rules.git".into();
    value.installations[0].owner.source_revision = Some(first_revision.into());
    value.installations[0].source_evidence = SourceEvidence::Git {
        resolved_revision: first_revision.into(),
        manifest_sha256: "a".repeat(64),
    };
    let mut second = value.installations[0].clone();
    second.owner.installation_id = "rule-b".into();
    second.owner.source_revision = Some(second_revision.into());
    second.source_evidence = SourceEvidence::Git {
        resolved_revision: second_revision.into(),
        manifest_sha256: "b".repeat(64),
    };
    value.installations.push(second);
    value.references.push(Reference::DirectInstallation {
        installation_id: "rule-b".into(),
    });
    assert!(validate_state(&value).is_err());
    value.installations[1].owner.source_revision = Some(first_revision.into());
    value.installations[1].source_evidence = value.installations[0].source_evidence.clone();
    value.installations[1].requested = vec![TargetSurface::CopilotCli];
    assert!(validate_state(&value).is_err());
}

#[test]
fn accepts_uppercase_digits_underscores_and_hex_digit_evidence() {
    let mut value = state();
    value.bundles = vec![bundle("Bundle_2")];
    value.installations[0].owner.installation_id = "Rule_A1".into();
    value.installations[0].source_evidence = SourceEvidence::Local {
        manifest_sha256: "0123456789abcdef".repeat(4),
    };
    value.references = vec![
        Reference::DirectBundle {
            bundle_id: "Bundle_2".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "Bundle_2".into(),
            installation_id: "Rule_A1".into(),
        },
    ];
    validate_state(&value).unwrap();
}
