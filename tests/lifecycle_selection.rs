use upskill::lifecycle::selection::{
    LegacyRow, SelectedNodes, Selection, SelectionDiagnostic as D, SelectionOutcome as O,
    select_owned_nodes,
};
use upskill::lifecycle::state::{
    BundleNode, CombinedState, Installation, Reference, SourceEvidence,
};
use upskill::lockfile::{
    LockedBundle, LockedItem, LockedMcp, LockedPlugin, Lockfile, McpInstallStatus,
    PluginInstallStatus,
};
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn state() -> CombinedState {
    let installations = ["z", "a"]
        .into_iter()
        .map(|id| Installation {
            owner: ArtifactOwner {
                installation_id: id.into(),
                source: format!("./{id}"),
                source_revision: None,
                kind: ItemKind::Rule,
                source_name: format!("original-{id}"),
                installed_name: id.into(),
            },
            root: InstallationRoot {
                scope: OwnershipScope::Project,
                path: std::env::temp_dir().join("upskill-selection"),
            },
            requested: vec![TargetSurface::ClaudeCli],
            source_evidence: SourceEvidence::Local {
                manifest_sha256: "a".repeat(64),
            },
        })
        .collect();
    CombinedState {
        schema: 2,
        installations,
        bundles: vec![BundleNode {
            id: "bundle-z".into(),
            source: "./bundle".into(),
            name: "bundle".into(),
            source_revision: None,
        }],
        references: vec![
            Reference::DirectInstallation {
                installation_id: "z".into(),
            },
            Reference::DirectInstallation {
                installation_id: "a".into(),
            },
            Reference::DirectBundle {
                bundle_id: "bundle-z".into(),
            },
        ],
        artifacts: vec![],
        owner_consumers: vec![],
        legacy: None,
    }
}
fn legacy_item(name: &str) -> LockedItem {
    LockedItem {
        kind: ItemKind::Rule,
        name: name.into(),
        source: "invalid legacy source!?".into(),
        git_ref: None,
        hash: Some("not-an-output-hash".into()),
        source_name: None,
        required_by: vec![],
        group: None,
        clients: vec!["unknown-client".into()],
    }
}
fn names(values: &[&str]) -> Selection {
    Selection::Names(values.iter().map(|v| (*v).into()).collect())
}
fn selected(ids: &[&str], bundles: &[&str]) -> SelectedNodes {
    SelectedNodes {
        installation_ids: ids.iter().map(|v| (*v).into()).collect(),
        bundle_ids: bundles.iter().map(|v| (*v).into()).collect(),
    }
}
fn blocked(s: &CombinedState, request: Selection) -> (SelectedNodes, Vec<D>) {
    let before = s.clone();
    let outcome = select_owned_nodes(s, &request).unwrap();
    assert_eq!(*s, before);
    match outcome {
        O::Blocked {
            candidates,
            diagnostics,
        } => (candidates, diagnostics),
        other => panic!("expected blocked, got {other:?}"),
    }
}
#[test]
fn all_owned_selection_preserves_prior_order_and_empty_installations() {
    let s = state();
    assert_eq!(
        select_owned_nodes(&s, &Selection::All).unwrap(),
        O::Ready(selected(&["z", "a"], &["bundle-z"]))
    );
    assert_eq!(
        select_owned_nodes(&s, &names(&["a", "z"])).unwrap(),
        O::Ready(selected(&["z", "a"], &[]))
    );
}
#[test]
fn named_disjoint_work_ignores_malformed_unselected_legacy() {
    let mut s = state();
    let mut legacy = Lockfile::new();
    legacy.items = vec![legacy_item("old"), legacy_item("old")];
    s.legacy = Some(legacy);
    let before = s.clone();
    assert_eq!(
        select_owned_nodes(&s, &names(&["a"])).unwrap(),
        O::Ready(selected(&["a"], &[]))
    );
    assert_eq!(s, before);
    assert_eq!(
        blocked(&s, Selection::All),
        (
            selected(&["z", "a"], &["bundle-z"]),
            vec![
                D::LegacyRow(LegacyRow::Item(0)),
                D::LegacyRow(LegacyRow::Item(1))
            ]
        )
    );
}
#[test]
fn all_selection_reports_each_legacy_category_and_duplicate_index() {
    let mut s = state();
    let mut legacy = Lockfile::new();
    legacy.items = vec![legacy_item("old"), legacy_item("old")];
    legacy.bundles.push(LockedBundle {
        name: "old-bundle".into(),
        source: "?".into(),
        git_ref: Some("old-ref".into()),
        items: vec!["old".into()],
    });
    legacy.plugins.push(LockedPlugin {
        name: "plugin".into(),
        client: "unknown".into(),
        identifier: "id".into(),
        scope: None,
        bundle: "old-bundle".into(),
        status: PluginInstallStatus::Skipped,
    });
    legacy.mcps.push(LockedMcp {
        name: "mcp".into(),
        client: "unknown".into(),
        scope: None,
        bundle: "old-bundle".into(),
        status: McpInstallStatus::Skipped,
    });
    s.legacy = Some(legacy);
    assert_eq!(
        blocked(&s, Selection::All).1,
        vec![
            D::LegacyRow(LegacyRow::Item(0)),
            D::LegacyRow(LegacyRow::Item(1)),
            D::LegacyRow(LegacyRow::Bundle(0)),
            D::LegacyRow(LegacyRow::Plugin(0)),
            D::LegacyRow(LegacyRow::Mcp(0))
        ]
    );
    assert_eq!(
        blocked(&s, names(&["old-bundle"])).1,
        vec![
            D::AmbiguousName {
                name: "old-bundle".into(),
                matches: 3
            },
            D::LegacyRow(LegacyRow::Bundle(0)),
            D::LegacyRow(LegacyRow::Plugin(0)),
            D::LegacyRow(LegacyRow::Mcp(0))
        ]
    );
    assert_eq!(
        blocked(&s, names(&["plugin"])).1,
        vec![D::LegacyRow(LegacyRow::Plugin(0))]
    );
    assert_eq!(
        blocked(&s, names(&["mcp"])).1,
        vec![D::LegacyRow(LegacyRow::Mcp(0))]
    );
}
#[test]
fn name_collisions_collect_owned_and_legacy_without_first_match() {
    let mut s = state();
    s.installations[0].owner.installed_name = "a".into();
    let mut l = Lockfile::new();
    l.items.push(legacy_item("a"));
    s.legacy = Some(l);
    assert_eq!(
        blocked(&s, names(&["a"])),
        (
            selected(&["z", "a"], &[]),
            vec![
                D::AmbiguousName {
                    name: "a".into(),
                    matches: 3
                },
                D::LegacyRow(LegacyRow::Item(0))
            ]
        )
    );
    s.legacy = None;
    assert_eq!(
        blocked(&s, names(&["a"])).1,
        vec![D::AmbiguousName {
            name: "a".into(),
            matches: 2
        }]
    );
}
#[test]
fn bundle_installation_collision_and_alias_selection_are_literal() {
    let mut s = state();
    s.bundles[0].name = "a".into();
    assert_eq!(
        blocked(&s, names(&["a"])),
        (
            selected(&["a"], &["bundle-z"]),
            vec![D::AmbiguousName {
                name: "a".into(),
                matches: 2
            }]
        )
    );
    assert_eq!(
        blocked(&s, names(&["original-z"])).1,
        vec![D::UnknownName("original-z".into())]
    );
    assert_eq!(
        select_owned_nodes(&s, &names(&["z"])).unwrap(),
        O::Ready(selected(&["z"], &[]))
    );
}
#[test]
fn unsupported_owned_kinds_block_only_when_selected() {
    for kind in [ItemKind::Skill, ItemKind::Agent] {
        let mut s = state();
        s.installations[0].owner.kind = kind;
        assert_eq!(
            select_owned_nodes(&s, &names(&["a"])).unwrap(),
            O::Ready(selected(&["a"], &[]))
        );
        assert_eq!(
            blocked(&s, Selection::All),
            (
                selected(&["z", "a"], &["bundle-z"]),
                vec![D::UnsupportedInstallation {
                    installation_id: "z".into(),
                    kind
                }]
            )
        );
    }
}
#[test]
fn name_diagnostics_are_lexical_and_complete() {
    let mut s = state();
    let mut l = Lockfile::new();
    l.items = vec![legacy_item("a"), legacy_item("old")];
    s.legacy = Some(l);
    assert_eq!(
        blocked(&s, names(&["missing-z", "old", "a", "missing-b"])),
        (
            selected(&["a"], &[]),
            vec![
                D::AmbiguousName {
                    name: "a".into(),
                    matches: 2
                },
                D::UnknownName("missing-b".into()),
                D::UnknownName("missing-z".into()),
                D::LegacyRow(LegacyRow::Item(0)),
                D::LegacyRow(LegacyRow::Item(1))
            ]
        )
    );
}
#[test]
fn invalid_requests_and_unselected_invalid_state_are_rejected() {
    for request in [names(&[]), names(&[""]), names(&["a", "a"])] {
        assert!(select_owned_nodes(&state(), &request).is_err());
    }
    let mut s = state();
    s.installations[0].source_evidence = SourceEvidence::Local {
        manifest_sha256: "invalid".into(),
    };
    let before = s.clone();
    assert!(select_owned_nodes(&s, &names(&["a"])).is_err());
    assert_eq!(s, before);
}
#[test]
fn empty_state_and_empty_legacy_snapshot_are_ready() {
    let mut s = state();
    s.installations.clear();
    s.bundles.clear();
    s.references.clear();
    s.legacy = Some(Lockfile::new());
    assert_eq!(
        select_owned_nodes(&s, &Selection::All).unwrap(),
        O::Ready(selected(&[], &[]))
    );
    assert_eq!(
        blocked(&s, names(&["absent"])).1,
        vec![D::UnknownName("absent".into())]
    );
}
#[test]
fn selected_bundle_expands_dependency_graph_and_blocks_unsupported_descendants() {
    let mut s = state();
    s.bundles.push(BundleNode {
        id: "dep".into(),
        source: "./dep".into(),
        name: "dependency".into(),
        source_revision: None,
    });
    s.references.extend([
        Reference::BundleDependency {
            bundle_id: "bundle-z".into(),
            dependency_id: "dep".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "dep".into(),
            installation_id: "z".into(),
        },
    ]);
    assert_eq!(
        select_owned_nodes(&s, &names(&["bundle"])).unwrap(),
        O::Ready(selected(&["z"], &["bundle-z", "dep"]))
    );
    assert_eq!(
        select_owned_nodes(&s, &names(&["a"])).unwrap(),
        O::Ready(selected(&["a"], &[]))
    );
    s.bundles.reverse();
    s.installations[0].owner.kind = ItemKind::Skill;
    assert_eq!(
        blocked(&s, names(&["bundle"])),
        (
            selected(&["z"], &["dep", "bundle-z"]),
            vec![D::UnsupportedInstallation {
                installation_id: "z".into(),
                kind: ItemKind::Skill
            }]
        )
    );
}
#[test]
fn transitive_bundle_selection_reports_historical_native_associations_once() {
    let mut s = state();
    s.bundles.push(BundleNode {
        id: "dep".into(),
        source: "./dep".into(),
        name: "dependency".into(),
        source_revision: None,
    });
    s.references.push(Reference::BundleDependency {
        bundle_id: "bundle-z".into(),
        dependency_id: "dep".into(),
    });
    let mut l = Lockfile::new();
    l.plugins.push(LockedPlugin {
        name: "dependency".into(),
        client: "unknown".into(),
        identifier: "id".into(),
        scope: None,
        bundle: "dependency".into(),
        status: PluginInstallStatus::Instructions,
    });
    l.mcps.push(LockedMcp {
        name: "native".into(),
        client: "unknown".into(),
        scope: None,
        bundle: "dependency".into(),
        status: McpInstallStatus::Skipped,
    });
    s.legacy = Some(l);
    assert_eq!(
        blocked(&s, names(&["bundle"])),
        (
            selected(&[], &["bundle-z", "dep"]),
            vec![
                D::LegacyRow(LegacyRow::Plugin(0)),
                D::LegacyRow(LegacyRow::Mcp(0))
            ]
        )
    );
    assert_eq!(
        blocked(&s, names(&["dependency"])),
        (
            selected(&[], &["dep"]),
            vec![
                D::AmbiguousName {
                    name: "dependency".into(),
                    matches: 3
                },
                D::LegacyRow(LegacyRow::Plugin(0)),
                D::LegacyRow(LegacyRow::Mcp(0))
            ]
        )
    );
    assert_eq!(
        select_owned_nodes(&s, &names(&["a"])).unwrap(),
        O::Ready(selected(&["a"], &[]))
    );
}
#[test]
fn combined_blockers_are_reported_before_any_ready_subset() {
    let mut s = state();
    s.installations[0].owner.kind = ItemKind::Agent;
    let mut l = Lockfile::new();
    l.items.push(legacy_item("old"));
    s.legacy = Some(l);
    assert_eq!(
        blocked(&s, names(&["z", "a", "old", "missing"])),
        (
            selected(&["z", "a"], &[]),
            vec![
                D::UnknownName("missing".into()),
                D::LegacyRow(LegacyRow::Item(0)),
                D::UnsupportedInstallation {
                    installation_id: "z".into(),
                    kind: ItemKind::Agent
                }
            ]
        )
    );
}
#[test]
fn equal_aliases_in_distinct_scopes_are_ambiguous() {
    let mut s = state();
    let mut other = s.installations[0].clone();
    other.owner.installation_id = "user-z".into();
    other.root.scope = OwnershipScope::User;
    s.installations.push(other);
    s.references.push(Reference::DirectInstallation {
        installation_id: "user-z".into(),
    });
    assert_eq!(
        blocked(&s, names(&["z"])),
        (
            selected(&["z", "user-z"], &[]),
            vec![D::AmbiguousName {
                name: "z".into(),
                matches: 2
            }]
        )
    );
}
#[test]
fn unsupported_populated_installations_preserve_unselected_artifacts() {
    for kind in [ItemKind::Skill, ItemKind::Agent] {
        let mut s = state();
        s.installations[0].owner.kind = kind;
        let item = &s.installations[0];
        s.artifacts.push(upskill::ownership::OwnedArtifact {
            root: item.root.clone(),
            path: "owned/data.bin".into(),
            content: upskill::ownership::ArtifactContent::File {
                content_sha256: "a".repeat(64),
                mode: Some(0o640),
            },
            owners: vec![item.owner.clone()],
            consumers: item.requested.clone(),
        });
        s.owner_consumers
            .push(upskill::lifecycle::state::OwnerConsumers {
                root: item.root.clone(),
                path: "owned/data.bin".into(),
                contribution_id: None,
                installation_id: "z".into(),
                consumers: item.requested.clone(),
            });
        let before = s.clone();
        assert_eq!(
            select_owned_nodes(&s, &names(&["a"])).unwrap(),
            O::Ready(selected(&["a"], &[]))
        );
        assert_eq!(s, before);
        assert_eq!(
            blocked(&s, names(&["z"])),
            (
                selected(&["z"], &[]),
                vec![D::UnsupportedInstallation {
                    installation_id: "z".into(),
                    kind
                }]
            )
        );
    }
}
#[test]
fn bundle_diamond_and_explicit_member_selection_deduplicate_nodes() {
    let mut s = state();
    for id in ["right", "left", "leaf"] {
        s.bundles.push(BundleNode {
            id: id.into(),
            source: format!("./{id}"),
            name: id.into(),
            source_revision: None,
        });
    }
    for (from, to) in [
        ("bundle-z", "left"),
        ("bundle-z", "right"),
        ("left", "leaf"),
        ("right", "leaf"),
    ] {
        s.references.push(Reference::BundleDependency {
            bundle_id: from.into(),
            dependency_id: to.into(),
        });
    }
    s.references.push(Reference::BundleInstallation {
        bundle_id: "leaf".into(),
        installation_id: "z".into(),
    });
    assert_eq!(
        select_owned_nodes(&s, &names(&["bundle", "z", "left"])).unwrap(),
        O::Ready(selected(&["z"], &["bundle-z", "right", "left", "leaf"]))
    );
}
