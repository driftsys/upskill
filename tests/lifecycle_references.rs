use upskill::lifecycle::reconcile::{GraphChanges, reconcile_references};
use upskill::lifecycle::state::{
    BundleNode, CombinedState, Installation, Reference, SourceEvidence, validate_state,
};
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn item(id: &str) -> Installation {
    Installation {
        owner: ArtifactOwner {
            installation_id: id.into(),
            source: format!("./{id}"),
            source_revision: None,
            kind: ItemKind::Rule,
            source_name: id.into(),
            installed_name: id.into(),
        },
        root: InstallationRoot {
            scope: OwnershipScope::Project,
            path: std::env::temp_dir().join("upskill-reference-example"),
        },
        requested: vec![TargetSurface::ClaudeCli],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: "a".repeat(64),
        },
    }
}
fn bundle(id: &str) -> BundleNode {
    BundleNode {
        id: id.into(),
        source: format!("./{id}"),
        name: id.into(),
        source_revision: None,
    }
}
fn root(id: &str) -> Reference {
    Reference::DirectBundle {
        bundle_id: id.into(),
    }
}
fn direct(id: &str) -> Reference {
    Reference::DirectInstallation {
        installation_id: id.into(),
    }
}
fn dependency(from: &str, to: &str) -> Reference {
    Reference::BundleDependency {
        bundle_id: from.into(),
        dependency_id: to.into(),
    }
}
fn member(bundle: &str, item: &str) -> Reference {
    Reference::BundleInstallation {
        bundle_id: bundle.into(),
        installation_id: item.into(),
    }
}
fn empty() -> CombinedState {
    CombinedState {
        schema: 2,
        installations: vec![],
        bundles: vec![],
        references: vec![],
        artifacts: vec![],
        owner_consumers: vec![],
        legacy: None,
    }
}
fn diamond() -> CombinedState {
    let mut s = empty();
    s.installations = vec![item("shared")];
    s.bundles = vec![bundle("left"), bundle("right"), bundle("dependency")];
    s.references = vec![
        root("left"),
        root("right"),
        dependency("left", "dependency"),
        dependency("right", "dependency"),
        member("dependency", "shared"),
    ];
    validate_state(&s).unwrap();
    s
}
fn reject(s: &CombinedState, changes: &GraphChanges) -> String {
    let before = (s.clone(), changes.clone());
    let error = reconcile_references(s, changes).unwrap_err();
    assert_eq!((s.clone(), changes.clone()), before);
    format!("{error:#}")
}

#[test]
fn diamond_retains_shared_dependencies_until_last_root_ends() {
    let s = diamond();
    let before = s.clone();
    let result = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![root("left")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.installations, s.installations);
    assert_eq!(result.bundles, vec![bundle("right"), bundle("dependency")]);
    assert_eq!(
        result.references,
        vec![
            root("right"),
            dependency("right", "dependency"),
            member("dependency", "shared")
        ]
    );
    assert_eq!(result.removed_bundle_ids, ["left"]);
    assert!(result.removed_installation_ids.is_empty());
    let mut next = empty();
    next.installations = result.installations;
    next.bundles = result.bundles;
    next.references = result.references;
    let result = reconcile_references(
        &next,
        &GraphChanges {
            remove_references: vec![root("right")],
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        result.installations.is_empty()
            && result.bundles.is_empty()
            && result.references.is_empty()
    );
    assert_eq!(result.removed_installation_ids, ["shared"]);
    assert_eq!(result.removed_bundle_ids, ["right", "dependency"]);
    assert_eq!(s, before);
}

#[test]
fn direct_and_bundle_references_are_independent() {
    let mut s = diamond();
    s.references.push(direct("shared"));
    let result = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![root("left"), root("right")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.installations, vec![item("shared")]);
    assert!(result.bundles.is_empty());
    assert_eq!(result.references, vec![direct("shared")]);
    let result = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![direct("shared")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.installations, s.installations);
    assert!(result.removed_installation_ids.is_empty());
    // Bundle and installation IDs are distinct namespaces, not reference counters.
    let mut s = empty();
    s.installations = vec![item("same")];
    s.bundles = vec![bundle("same")];
    s.references = vec![root("same"), member("same", "same"), direct("same")];
    let result = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![direct("same")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.installations, s.installations);
    assert_eq!(
        result.references,
        vec![root("same"), member("same", "same")]
    );
}

#[test]
fn referenced_empty_installation_is_retained() {
    let mut s = empty();
    s.installations = vec![item("excluded")];
    s.references = vec![direct("excluded")];
    let result = reconcile_references(&s, &GraphChanges::default()).unwrap();
    assert_eq!(result.installations, s.installations);
    assert_eq!(result.references, s.references);
    assert!(result.removed_installation_ids.is_empty());
}

#[test]
fn invalid_unreachable_graph_edits_fail_before_pruning() {
    let s = diamond();
    assert!(
        reject(
            &s,
            &GraphChanges {
                add_references: vec![member("absent", "shared")],
                ..Default::default()
            }
        )
        .contains("dangling")
    );
    assert!(
        reject(
            &s,
            &GraphChanges {
                bundles: vec![bundle("a"), bundle("b")],
                add_references: vec![dependency("a", "b"), dependency("b", "a")],
                ..Default::default()
            }
        )
        .contains("cyclic")
    );
    let mut invalid = s.clone();
    invalid.references.push(direct("absent"));
    assert!(reject(&invalid, &GraphChanges::default()).contains("prior"));
    let mut invalid_legacy = s.clone();
    let mut legacy = upskill::lockfile::Lockfile::new();
    legacy.schema = 2;
    invalid_legacy.legacy = Some(legacy);
    assert!(reject(&invalid_legacy, &GraphChanges::default()).contains("prior"));
    let mut malformed = item("unused");
    malformed.source_evidence = SourceEvidence::Local {
        manifest_sha256: "invalid".into(),
    };
    assert!(
        reject(
            &s,
            &GraphChanges {
                installations: vec![malformed],
                ..Default::default()
            }
        )
        .contains("SHA-256")
    );
}

#[test]
fn immutable_binding_and_duplicate_identity_reject() {
    let s = diamond();
    let mutations: [fn(&mut Installation); 6] = [
        |i| i.owner.source = "./different".into(),
        |i| i.owner.kind = ItemKind::Skill,
        |i| i.owner.source_name = "different".into(),
        |i| i.owner.installed_name = "different".into(),
        |i| i.root.scope = OwnershipScope::User,
        |i| i.root.path.push("."),
    ];
    for mutate in mutations {
        let mut changed = s.installations[0].clone();
        mutate(&mut changed);
        assert!(
            reject(
                &s,
                &GraphChanges {
                    installations: vec![changed],
                    ..Default::default()
                }
            )
            .contains("binding")
        );
    }
    let mut duplicate = s.installations[0].clone();
    duplicate.owner.installation_id = "duplicate".into();
    assert!(
        reject(
            &s,
            &GraphChanges {
                installations: vec![duplicate],
                add_references: vec![direct("duplicate")],
                ..Default::default()
            }
        )
        .contains("identity")
    );
    for field in ["source", "name"] {
        let mut changed = s.bundles[0].clone();
        if field == "source" {
            changed.source = "./other".into()
        } else {
            changed.name = "other".into()
        }
        assert!(
            reject(
                &s,
                &GraphChanges {
                    bundles: vec![changed],
                    ..Default::default()
                }
            )
            .contains("binding")
        );
    }
}

#[test]
fn edits_are_complete_and_deterministic() {
    let s = diamond();
    let mut changes = GraphChanges {
        installations: vec![item("z"), item("a")],
        bundles: vec![bundle("z"), bundle("a")],
        add_references: vec![direct("z"), direct("a"), root("z"), root("a")],
        ..Default::default()
    };
    let result = reconcile_references(&s, &changes).unwrap();
    assert_eq!(
        result
            .installations
            .iter()
            .map(|i| i.owner.installation_id.as_str())
            .collect::<Vec<_>>(),
        ["shared", "a", "z"]
    );
    assert_eq!(
        result
            .bundles
            .iter()
            .map(|b| b.id.as_str())
            .collect::<Vec<_>>(),
        ["left", "right", "dependency", "a", "z"]
    );
    let mut expected = s.references.clone();
    expected.extend([direct("a"), direct("z"), root("a"), root("z")]);
    assert_eq!(result.references, expected);
    changes.installations.reverse();
    changes.bundles.reverse();
    changes.add_references.reverse();
    assert_eq!(result, reconcile_references(&s, &changes).unwrap());
    for changes in [
        GraphChanges {
            installations: vec![item("unused")],
            ..Default::default()
        },
        GraphChanges {
            bundles: vec![bundle("unused")],
            ..Default::default()
        },
        GraphChanges {
            installations: vec![item("a"), item("a")],
            ..Default::default()
        },
        GraphChanges {
            bundles: vec![bundle("a"), bundle("a")],
            ..Default::default()
        },
        GraphChanges {
            add_references: vec![root("left")],
            ..Default::default()
        },
        GraphChanges {
            add_references: vec![direct("shared"), direct("shared")],
            ..Default::default()
        },
        GraphChanges {
            remove_references: vec![root("left"), root("left")],
            ..Default::default()
        },
        GraphChanges {
            remove_references: vec![direct("shared")],
            ..Default::default()
        },
        GraphChanges {
            add_references: vec![root("left")],
            remove_references: vec![root("left")],
            ..Default::default()
        },
        GraphChanges {
            installations: vec![s.installations[0].clone()],
            remove_references: vec![root("left"), root("right")],
            ..Default::default()
        },
    ] {
        reject(&s, &changes);
    }
    let mut changed = s.installations[0].clone();
    changed.requested = vec![TargetSurface::CopilotCli];
    changed.owner.source_revision = Some("resolved".into());
    changed.source_evidence = SourceEvidence::Git {
        resolved_revision: "resolved".into(),
        manifest_sha256: "b".repeat(64),
    };
    let mut changed_bundle = s.bundles[0].clone();
    changed_bundle.source_revision = Some("bundle-revision".into());
    let result = reconcile_references(
        &s,
        &GraphChanges {
            installations: vec![changed.clone()],
            bundles: vec![changed_bundle.clone()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.installations, [changed]);
    assert_eq!(result.bundles[0], changed_bundle);
    assert_eq!(
        reconcile_references(&s, &GraphChanges::default())
            .unwrap()
            .references,
        s.references
    );
}

#[test]
fn deep_dependency_chain_is_iterative() {
    let mut s = empty();
    s.installations = vec![item("leaf")];
    s.bundles = (0..2000).map(|n| bundle(&format!("b{n}"))).collect();
    s.references = vec![root("b0")];
    s.references
        .extend((0..1999).map(|n| dependency(&format!("b{n}"), &format!("b{}", n + 1))));
    s.references.push(member("b1999", "leaf"));
    let result = reconcile_references(&s, &GraphChanges::default()).unwrap();
    assert_eq!(result.installations, s.installations);
    assert_eq!(result.bundles, s.bundles);
    let result = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![root("b0")],
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.installations.is_empty() && result.bundles.is_empty());
    assert_eq!(result.removed_bundle_ids.len(), 2000);
}

#[test]
fn tagged_legacy_and_prior_artifacts_are_not_mutated() {
    use upskill::files::plan::{
        DesiredFile, FileInstallationChange, FileLocation, FileValue, plan_files,
    };
    use upskill::lifecycle::state::OwnerConsumers;
    let mut s = empty();
    let i = item("owned");
    let file = DesiredFile {
        location: FileLocation {
            root: i.root.clone(),
            path: "owned.md".into(),
        },
        value: FileValue {
            bytes: b"owned".to_vec(),
            mode: None,
        },
        consumers: vec![TargetSurface::ClaudeCli],
    };
    let observation = upskill::files::plan::ObservedFile {
        location: file.location.clone(),
        value: None,
    };
    s.artifacts = plan_files(
        &[],
        &[FileInstallationChange::Set {
            owner: i.owner.clone(),
            files: vec![file],
        }],
        &[observation],
    )
    .unwrap()
    .artifacts;
    s.owner_consumers = vec![OwnerConsumers {
        root: i.root.clone(),
        path: "owned.md".into(),
        contribution_id: None,
        installation_id: "owned".into(),
        consumers: vec![TargetSurface::ClaudeCli],
    }];
    s.installations = vec![i];
    s.references = vec![direct("owned")];
    let mut legacy = upskill::lockfile::Lockfile::new();
    let row = upskill::lockfile::LockedItem {
        kind: ItemKind::Skill,
        name: "old".into(),
        source: "malformed::legacy@".into(),
        git_ref: Some("original-pin".into()),
        hash: None,
        source_name: None,
        required_by: vec!["unknown".into()],
        group: None,
        clients: vec!["unknown".into()],
    };
    legacy.items = vec![row.clone(), row];
    s.legacy = Some(legacy);
    let before = s.clone();
    let result = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![direct("owned")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(s, before);
    assert_eq!(result.removed_installation_ids, ["owned"]);
    assert!(result.installations.is_empty());
    let mut invalid = s.clone();
    invalid.owner_consumers[0].consumers = vec![TargetSurface::CopilotCli];
    assert!(reject(&invalid, &GraphChanges::default()).contains("prior"));
}

#[test]
fn internal_edge_removal_preserves_roots_and_other_paths() {
    let s = diamond();
    let result = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![dependency("left", "dependency")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.installations, s.installations);
    assert_eq!(result.bundles, s.bundles);
    assert!(result.removed_bundle_ids.is_empty() && result.removed_installation_ids.is_empty());
    assert_eq!(
        result.references,
        vec![
            root("left"),
            root("right"),
            dependency("right", "dependency"),
            member("dependency", "shared")
        ]
    );
    let result = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![
                dependency("left", "dependency"),
                dependency("right", "dependency"),
            ],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.bundles, vec![bundle("left"), bundle("right")]);
    assert!(result.installations.is_empty());
    assert_eq!(result.references, vec![root("left"), root("right")]);
    assert_eq!(result.removed_bundle_ids, ["dependency"]);
    assert_eq!(result.removed_installation_ids, ["shared"]);
    let result = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![member("dependency", "shared")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.bundles, s.bundles);
    assert!(result.installations.is_empty());
    assert!(result.removed_bundle_ids.is_empty());
    assert_eq!(result.removed_installation_ids, ["shared"]);
}

#[test]
fn reachable_duplicate_upserts_are_rejected() {
    let s = diamond();
    for changes in [
        GraphChanges {
            installations: vec![s.installations[0].clone(), s.installations[0].clone()],
            ..Default::default()
        },
        GraphChanges {
            bundles: vec![s.bundles[0].clone(), s.bundles[0].clone()],
            ..Default::default()
        },
    ] {
        let before = s.clone();
        let edits = changes.clone();
        let error = reconcile_references(&s, &changes).unwrap_err().to_string();
        assert!(error.contains("duplicate"), "{error}");
        assert_eq!(s, before);
        assert_eq!(changes, edits);
    }
}

#[test]
fn explicit_edges_from_detached_bundles_are_rejected() {
    let s = diamond();
    for edge in [member("left", "shared"), dependency("left", "right")] {
        for additions in [
            vec![edge.clone()],
            vec![member("right", "shared"), edge.clone()],
            vec![edge.clone(), member("right", "shared")],
        ] {
            let error = reject(
                &s,
                &GraphChanges {
                    remove_references: vec![root("left")],
                    add_references: additions,
                    ..Default::default()
                },
            );
            assert!(error.contains("added reference is unreachable"), "{error}");
        }
    }
}

#[test]
fn installation_root_does_not_retain_same_named_bundle() {
    let mut s = empty();
    s.installations = vec![item("same")];
    s.bundles = vec![bundle("same")];
    s.references = vec![root("same"), member("same", "same"), direct("same")];
    let result = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![root("same")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.installations, s.installations);
    assert!(result.bundles.is_empty());
    assert_eq!(result.references, vec![direct("same")]);
    assert_eq!(result.removed_bundle_ids, ["same"]);
}

#[test]
fn prior_order_and_new_internal_edge_order_are_stable() {
    let mut s = empty();
    s.installations = vec![item("z"), item("a"), item("m")];
    s.bundles = vec![bundle("z"), bundle("a")];
    s.references = vec![direct("z"), direct("a"), direct("m"), root("z"), root("a")];
    let mut changes = GraphChanges {
        add_references: vec![dependency("z", "a"), member("z", "m"), member("a", "z")],
        ..Default::default()
    };
    let result = reconcile_references(&s, &changes).unwrap();
    assert_eq!(result.installations, s.installations);
    let mut expected = s.references.clone();
    expected.extend([member("a", "z"), member("z", "m"), dependency("z", "a")]);
    assert_eq!(result.references, expected);
    changes.add_references.reverse();
    assert_eq!(result, reconcile_references(&s, &changes).unwrap());
    let removed = reconcile_references(
        &s,
        &GraphChanges {
            remove_references: vec![direct("a"), direct("z")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(removed.removed_installation_ids, ["z", "a"]);
    assert_eq!(removed.installations, vec![item("m")]);
}

#[test]
fn new_node_payloads_preserve_requested_surfaces_evidence_and_revisions() {
    let s = diamond();
    let mut git = item("new-git");
    git.owner.source = "https://github.com/example/registry".into();
    git.owner.source_revision = Some("a".repeat(40));
    git.requested = vec![
        upskill::target::TargetSurface::CodexCli,
        upskill::target::TargetSurface::OpenCodeCli,
    ];
    git.source_evidence = SourceEvidence::Git {
        resolved_revision: "a".repeat(40),
        manifest_sha256: "b".repeat(64),
    };
    let mut local = item("new-local");
    local.requested = vec![upskill::target::TargetSurface::CopilotCli];
    local.source_evidence = SourceEvidence::Local {
        manifest_sha256: "c".repeat(64),
    };
    let mut new_bundle = bundle("new-bundle");
    new_bundle.source_revision = Some("release-v2".into());
    let changes = GraphChanges {
        installations: vec![local.clone(), git.clone()],
        bundles: vec![new_bundle.clone()],
        add_references: vec![direct("new-local"), root("new-bundle"), direct("new-git")],
        ..Default::default()
    };
    let before = (s.clone(), changes.clone());
    let result = reconcile_references(&s, &changes).unwrap();
    let mut expected = s.installations.clone();
    expected.extend([git, local]);
    assert_eq!(result.installations, expected);
    let mut expected = s.bundles.clone();
    expected.push(new_bundle);
    assert_eq!(result.bundles, expected);
    assert_eq!((s, changes), before);
}
