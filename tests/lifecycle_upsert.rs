use sha2::{Digest, Sha256};
use upskill::files::plan::{DesiredFile, FileLocation, FileValue, ObservedFile};
use upskill::lifecycle::desired::DesiredInstallationSet;
use upskill::lifecycle::selection::Selection;
use upskill::lifecycle::state::{
    CombinedState, Installation, OwnerConsumers, Reference, SourceEvidence, validate_state,
};
use upskill::lifecycle::upsert::{UpsertOutcome, UpsertPlan, UpsertRequest, plan_upsert};
use upskill::operations::plan::DesiredArtifact;
use upskill::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn root() -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: std::env::temp_dir().join("upskill-upsert-example"),
    }
}
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
        root: root(),
        requested: vec![TargetSurface::ClaudeCli],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: "a".repeat(64),
        },
    }
}
fn direct(id: &str) -> Reference {
    Reference::DirectInstallation {
        installation_id: id.into(),
    }
}
fn location(path: &str) -> FileLocation {
    FileLocation {
        root: root(),
        path: path.into(),
    }
}
fn value(bytes: &[u8]) -> FileValue {
    FileValue {
        bytes: bytes.to_vec(),
        mode: Some(0o640),
    }
}
fn set(id: &str, path: &str, bytes: &[u8]) -> DesiredInstallationSet {
    DesiredInstallationSet {
        installation: item(id),
        artifacts: vec![DesiredArtifact::File(DesiredFile {
            location: location(path),
            value: value(bytes),
            consumers: vec![TargetSurface::ClaudeCli],
        })],
        audience_excluded: vec![],
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
fn prior() -> CombinedState {
    let mut s = empty();
    s.installations = vec![item("a"), item("b")];
    s.references = vec![direct("a"), direct("b")];
    for id in ["a", "b"] {
        s.artifacts.push(OwnedArtifact {
            root: root(),
            path: format!("{id}.md"),
            content: ArtifactContent::File {
                content_sha256: Sha256::digest(id.as_bytes())
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
                mode: Some(0o640),
            },
            owners: vec![item(id).owner],
            consumers: vec![TargetSurface::ClaudeCli],
        });
        s.owner_consumers.push(OwnerConsumers {
            root: root(),
            path: format!("{id}.md"),
            contribution_id: None,
            installation_id: id.into(),
            consumers: vec![TargetSurface::ClaudeCli],
        });
    }
    validate_state(&s).unwrap();
    s
}
fn request<'a>(
    selection: Option<&'a Selection>,
    sets: &'a [DesiredInstallationSet],
    add: &'a [Reference],
    remove: &'a [Reference],
) -> UpsertRequest<'a> {
    UpsertRequest {
        selection,
        sets,
        bundles: &[],
        add_references: add,
        remove_references: remove,
    }
}
fn ready(outcome: UpsertOutcome) -> UpsertPlan {
    match outcome {
        UpsertOutcome::Ready(p) => *p,
        other => panic!("unexpected {other:?}"),
    }
}
fn observed(path: &str, bytes: Option<&[u8]>) -> ObservedFile {
    ObservedFile {
        location: location(path),
        value: bytes.map(value),
    }
}
fn named(name: &str) -> Selection {
    Selection::Names(vec![name.into()])
}

#[test]
fn adds_complete_new_installation_and_exact_association_without_mutation() {
    let state = empty();
    let sets = vec![set("a", "a.md", b"new")];
    let obs = vec![observed("a.md", None)];
    let saved = (state.clone(), sets.clone(), obs.clone());
    let p = ready(plan_upsert(&state, request(None, &sets, &[direct("a")], &[]), &obs).unwrap());
    assert_eq!(p.state.installations, vec![item("a")]);
    assert_eq!(p.state.references, vec![direct("a")]);
    assert_eq!(p.state.artifacts[0].owners, vec![item("a").owner]);
    assert_eq!(
        p.state.owner_consumers,
        vec![OwnerConsumers {
            root: root(),
            path: "a.md".into(),
            contribution_id: None,
            installation_id: "a".into(),
            consumers: vec![TargetSurface::ClaudeCli]
        }]
    );
    assert_eq!(p.files.len(), 1);
    assert_eq!(p.files[0].location, location("a.md"));
    assert_eq!(p.files[0].before, None);
    assert_eq!(p.files[0].after, Some(value(b"new")));
    validate_state(&p.state).unwrap();
    assert_eq!((state, sets, obs), saved);
}
#[test]
fn updates_complete_set_and_preserves_untouched_history() {
    let mut state = prior();
    state.legacy = Some(upskill::lockfile::Lockfile::default());
    let mut sets = vec![set("a", "new.md", b"next")];
    sets[0].installation.source_evidence = SourceEvidence::Local {
        manifest_sha256: "b".repeat(64),
    };
    let p = ready(
        plan_upsert(
            &state,
            request(Some(&named("a")), &sets, &[], &[]),
            &[observed("a.md", Some(b"a")), observed("new.md", None)],
        )
        .unwrap(),
    );
    assert_eq!(
        p.state.installations,
        vec![sets[0].installation.clone(), item("b")]
    );
    assert_eq!(
        p.state
            .artifacts
            .iter()
            .map(|a| a.path.as_str())
            .collect::<Vec<_>>(),
        vec!["b.md", "new.md"]
    );
    assert_eq!(p.state.owner_consumers[0], state.owner_consumers[1]);
    assert_eq!(p.state.legacy, state.legacy);
    assert_eq!(p.files.len(), 2);
    assert_eq!(
        p.files
            .iter()
            .map(|f| f.location.clone())
            .collect::<Vec<_>>(),
        vec![location("a.md"), location("new.md")]
    );
    assert_eq!(p.files[0].after, None);
    assert_eq!(p.files[1].after, Some(value(b"next")));
}
#[test]
fn empty_complete_set_retires_only_output_and_retains_referenced_installation() {
    let state = prior();
    let mut s = set("a", "a.md", b"a");
    s.artifacts.clear();
    s.audience_excluded = vec![TargetSurface::ClaudeCli];
    let p = ready(
        plan_upsert(
            &state,
            request(Some(&named("a")), &[s.clone()], &[], &[]),
            &[observed("a.md", Some(b"a"))],
        )
        .unwrap(),
    );
    assert_eq!(p.state.installations, vec![s.installation, item("b")]);
    assert_eq!(p.state.references, state.references);
    assert_eq!(p.state.artifacts, vec![state.artifacts[1].clone()]);
    assert_eq!(
        p.state.owner_consumers,
        vec![state.owner_consumers[1].clone()]
    );
    assert_eq!(p.files[0].after, None);
}
#[test]
fn new_empty_set_retains_source_and_requested_history_without_files() {
    let mut s = set("a", "a.md", b"a");
    s.artifacts.clear();
    s.audience_excluded = vec![TargetSurface::ClaudeCli];
    let p = ready(
        plan_upsert(
            &empty(),
            request(None, &[s.clone()], &[direct("a")], &[]),
            &[],
        )
        .unwrap(),
    );
    assert_eq!(p.state.installations, vec![s.installation]);
    assert_eq!(p.state.references, vec![direct("a")]);
    assert!(p.files.is_empty());
    assert!(p.state.artifacts.is_empty());
}
#[test]
fn graph_retirement_requires_selection_and_preserves_other_owners() {
    let state = prior();
    let p = ready(
        plan_upsert(
            &state,
            request(Some(&named("a")), &[], &[], &[direct("a")]),
            &[observed("a.md", Some(b"a"))],
        )
        .unwrap(),
    );
    assert_eq!(p.state.installations, vec![item("b")]);
    assert_eq!(p.state.references, vec![direct("b")]);
    assert_eq!(p.state.artifacts, vec![state.artifacts[1].clone()]);
    assert_eq!(p.files[0].after, None);
}
#[test]
fn rejects_unselected_updates_and_reference_edits() {
    let state = prior();
    for r in [
        request(None, &[set("a", "a.md", b"new")], &[], &[]),
        request(None, &[], &[], &[direct("a")]),
    ] {
        assert!(format!("{:#}", plan_upsert(&state, r, &[]).unwrap_err()).contains("unselected"));
    }
    assert!(
        format!(
            "{:#}",
            plan_upsert(
                &state,
                request(Some(&named("b")), &[set("a", "a.md", b"new")], &[], &[]),
                &[]
            )
            .unwrap_err()
        )
        .contains("unselected")
    );
}
#[test]
fn requires_complete_selected_survivor_sets() {
    let state = prior();
    let e = plan_upsert(
        &state,
        request(Some(&Selection::All), &[set("a", "a.md", b"new")], &[], &[]),
        &[],
    )
    .unwrap_err();
    assert!(format!("{e:#}").contains("complete desired set"));
}
#[test]
fn validates_roots_consumers_and_exclusions_before_planning() {
    let mut variants = vec![];
    let mut s = set("a", "a.md", b"a");
    if let DesiredArtifact::File(f) = &mut s.artifacts[0] {
        f.location.root.path = root().path.join("other");
    }
    variants.push(s);
    let mut s = set("a", "a.md", b"a");
    if let DesiredArtifact::File(f) = &mut s.artifacts[0] {
        f.consumers = vec![TargetSurface::CodexCli];
    }
    variants.push(s);
    let mut s = set("a", "a.md", b"a");
    s.audience_excluded = vec![TargetSurface::ClaudeCli];
    variants.push(s);
    let mut s = set("a", "a.md", b"a");
    s.artifacts.clear();
    variants.push(s);
    let mut s = set("a", "a.md", b"a");
    s.artifacts.clear();
    s.audience_excluded = vec![TargetSurface::ClaudeCli, TargetSurface::ClaudeCli];
    variants.push(s);
    for s in variants {
        assert!(
            plan_upsert(
                &empty(),
                request(None, &[s], &[direct("a")], &[]),
                &[observed("a.md", None)]
            )
            .is_err()
        );
    }
}
#[test]
fn noop_requires_observation_and_preserves_bytes() {
    let state = prior();
    let sets = [set("a", "a.md", b"a")];
    assert!(plan_upsert(&state, request(Some(&named("a")), &sets, &[], &[]), &[]).is_err());
    let p = ready(
        plan_upsert(
            &state,
            request(Some(&named("a")), &sets, &[], &[]),
            &[observed("a.md", Some(b"a"))],
        )
        .unwrap(),
    );
    assert_eq!(p.state, state);
    assert_eq!(p.files.len(), 1);
    assert_eq!(p.files[0].before, p.files[0].after);
}
#[test]
fn blocks_selected_legacy_without_exposing_owned_subset() {
    let mut state = prior();
    let mut lock = upskill::lockfile::Lockfile::default();
    lock.items.push(upskill::lockfile::LockedItem {
        kind: ItemKind::Rule,
        name: "legacy".into(),
        source: "malformed".into(),
        git_ref: None,
        hash: None,
        source_name: None,
        required_by: vec![],
        group: None,
        clients: vec![],
    });
    state.legacy = Some(lock);
    let result = plan_upsert(
        &state,
        request(
            Some(&Selection::All),
            &[set("a", "a.md", b"next"), set("b", "b.md", b"next")],
            &[],
            &[],
        ),
        &[],
    )
    .unwrap();
    assert!(matches!(result,UpsertOutcome::Blocked(d) if !d.is_empty()));
}
#[test]
fn rejects_existing_unmarked_output_and_edited_before_value() {
    assert!(
        plan_upsert(
            &empty(),
            request(None, &[set("a", "a.md", b"same")], &[direct("a")], &[]),
            &[observed("a.md", Some(b"same"))]
        )
        .is_err()
    );
    assert!(
        plan_upsert(
            &prior(),
            request(Some(&named("a")), &[set("a", "a.md", b"new")], &[], &[]),
            &[observed("a.md", Some(b"edited"))]
        )
        .is_err()
    );
}
#[test]
fn rejects_duplicate_sets_and_unsupported_kinds_even_when_empty() {
    let s = set("a", "a.md", b"a");
    assert!(
        plan_upsert(
            &empty(),
            request(None, &[s.clone(), s], &[direct("a")], &[]),
            &[]
        )
        .is_err()
    );
    let mut s = set("a", "a.md", b"a");
    s.installation.owner.kind = ItemKind::Skill;
    s.artifacts.clear();
    s.audience_excluded = vec![TargetSurface::ClaudeCli];
    assert!(plan_upsert(&empty(), request(None, &[s], &[direct("a")], &[]), &[]).is_err());
}

fn legacy(name: &str) -> upskill::lockfile::LockedItem {
    upskill::lockfile::LockedItem {
        kind: ItemKind::Rule,
        name: name.into(),
        source: "./old".into(),
        git_ref: None,
        hash: None,
        source_name: None,
        required_by: vec![],
        group: None,
        clients: vec!["claude".into()],
    }
}
#[test]
fn blocks_new_legacy_names_and_native_declaring_bundle_names() {
    let mut state = empty();
    let mut lock = upskill::lockfile::Lockfile::default();
    lock.items.push(legacy("a"));
    state.legacy = Some(lock);
    assert!(matches!(
        plan_upsert(
            &state,
            request(None, &[set("a", "a.md", b"new")], &[direct("a")], &[]),
            &[]
        )
        .unwrap(),
        UpsertOutcome::Blocked(_)
    ));
    let mut lock = upskill::lockfile::Lockfile::default();
    lock.plugins.push(upskill::lockfile::LockedPlugin {
        name: "native".into(),
        client: "claude".into(),
        identifier: "x@y".into(),
        scope: None,
        bundle: "new-bundle".into(),
        status: Default::default(),
    });
    state.legacy = Some(lock);
    let bundles = [upskill::lifecycle::state::BundleNode {
        id: "bundle".into(),
        source: "./bundle".into(),
        name: "new-bundle".into(),
        source_revision: None,
    }];
    let added = [Reference::DirectBundle {
        bundle_id: "bundle".into(),
    }];
    let r = UpsertRequest {
        bundles: &bundles,
        ..request(None, &[], &added, &[])
    };
    assert!(matches!(
        plan_upsert(&state, r, &[]).unwrap(),
        UpsertOutcome::Blocked(_)
    ));
}
#[test]
fn known_legacy_destinations_conflict_even_with_different_alias_and_missing_file() {
    let mut state = empty();
    let mut lock = upskill::lockfile::Lockfile::default();
    lock.items.push(legacy("old"));
    state.legacy = Some(lock);
    for path in [
        ".claude/rules/old.md",
        ".claude/rules/old/data.bin",
        ".claude/rules",
    ] {
        let e = plan_upsert(
            &state,
            request(None, &[set("new", path, b"new")], &[direct("new")], &[]),
            &[observed(path, None)],
        )
        .unwrap_err();
        assert!(format!("{e:#}").contains("legacy destination"));
    }
    let path = ".claude/rules/oldish.md";
    let p = ready(
        plan_upsert(
            &state,
            request(None, &[set("new", path, b"new")], &[direct("new")], &[]),
            &[observed(path, None)],
        )
        .unwrap(),
    );
    assert_eq!(p.state.legacy, state.legacy);
}
#[test]
fn instruction_update_preserves_manual_bytes_and_observed_mode() {
    use upskill::instructions::{
        aggregate::{self, Contribution},
        plan::{DesiredInstructionContribution, InstructionLocation},
    };
    let mut state = empty();
    state.installations = vec![item("a")];
    state.references = vec![direct("a")];
    let edit = aggregate::reconcile("Manual\r\n", &[], &[Contribution::new("a", "Old\n")]).unwrap();
    state.artifacts.push(OwnedArtifact {
        root: root(),
        path: "AGENTS.md".into(),
        content: ArtifactContent::MarkdownContribution {
            contribution: edit.owned[0].clone(),
        },
        owners: vec![item("a").owner],
        consumers: vec![TargetSurface::ClaudeCli],
    });
    state.owner_consumers.push(OwnerConsumers {
        root: root(),
        path: "AGENTS.md".into(),
        contribution_id: Some("a".into()),
        installation_id: "a".into(),
        consumers: vec![TargetSurface::ClaudeCli],
    });
    let s = DesiredInstallationSet {
        installation: item("a"),
        artifacts: vec![DesiredArtifact::Instruction(
            DesiredInstructionContribution {
                location: InstructionLocation {
                    root: root(),
                    path: "AGENTS.md".into(),
                },
                id: "a".into(),
                markdown: "New\n".into(),
                consumers: vec![TargetSurface::ClaudeCli],
            },
        )],
        audience_excluded: vec![],
    };
    let obs = observed("AGENTS.md", Some(edit.content.as_bytes()));
    let p = ready(
        plan_upsert(
            &state,
            request(Some(&named("a")), &[s], &[], &[]),
            std::slice::from_ref(&obs),
        )
        .unwrap(),
    );
    assert_eq!(p.files.len(), 1);
    assert_eq!(p.files[0].before, obs.value);
    let after = p.files[0].after.as_ref().unwrap();
    assert_eq!(after.mode, Some(0o640));
    assert!(after.bytes.starts_with(b"Manual\r\n"));
    assert!(String::from_utf8_lossy(&after.bytes).contains("New\n"));
    assert!(!String::from_utf8_lossy(&after.bytes).contains("Old\n"));
    assert_eq!(p.state.owner_consumers, state.owner_consumers);
    validate_state(&p.state).unwrap();
}
#[test]
fn affected_shared_artifacts_and_equal_byte_new_sharing_remain_blocked() {
    let mut state = prior();
    state.artifacts.remove(1);
    state.artifacts[0].owners.push(item("b").owner);
    state.owner_consumers[1].path = "a.md".into();
    validate_state(&state).unwrap();
    assert!(
        format!(
            "{:#}",
            plan_upsert(
                &state,
                request(Some(&named("a")), &[set("a", "a.md", b"a")], &[], &[]),
                &[observed("a.md", Some(b"a"))]
            )
            .unwrap_err()
        )
        .contains("multi-owner")
    );
    assert!(
        plan_upsert(
            &empty(),
            request(
                None,
                &[set("a", "same.md", b"same"), set("b", "same.md", b"same")],
                &[direct("a"), direct("b")],
                &[]
            ),
            &[observed("same.md", None)]
        )
        .is_err()
    );
    // Unaffected sharing survives disjoint new work unchanged.
    let p = ready(
        plan_upsert(
            &state,
            request(None, &[set("c", "c.md", b"c")], &[direct("c")], &[]),
            &[observed("c.md", None)],
        )
        .unwrap(),
    );
    assert_eq!(p.state.artifacts[0], state.artifacts[0]);
    assert_eq!(&p.state.owner_consumers[..2], &state.owner_consumers);
}
#[test]
fn bundle_selection_covers_members_and_retains_empty_members() {
    use upskill::lifecycle::state::BundleNode;
    let mut state = empty();
    state.installations = vec![item("a")];
    state.bundles = vec![BundleNode {
        id: "bundle".into(),
        source: "./bundle".into(),
        name: "bundle".into(),
        source_revision: None,
    }];
    state.references = vec![
        Reference::DirectBundle {
            bundle_id: "bundle".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "bundle".into(),
            installation_id: "a".into(),
        },
    ];
    validate_state(&state).unwrap();
    let mut s = set("a", "a.md", b"a");
    s.artifacts.clear();
    s.audience_excluded = vec![TargetSurface::ClaudeCli];
    let p =
        ready(plan_upsert(&state, request(Some(&named("bundle")), &[s], &[], &[]), &[]).unwrap());
    assert_eq!(p.state, state);
    assert!(plan_upsert(&state, request(Some(&named("bundle")), &[], &[], &[]), &[]).is_err());
}

#[test]
fn aggregates_existing_selection_and_new_root_legacy_blockers() {
    use upskill::lifecycle::selection::{LegacyRow, SelectionDiagnostic};
    let mut state = prior();
    let lock = upskill::lockfile::Lockfile {
        items: vec![legacy("old"), legacy("new")],
        ..Default::default()
    };
    state.legacy = Some(lock);
    let result = plan_upsert(
        &state,
        request(
            Some(&named("old")),
            &[set("new", "new.md", b"new")],
            &[direct("new")],
            &[],
        ),
        &[],
    )
    .unwrap();
    assert_eq!(
        result,
        UpsertOutcome::Blocked(vec![
            SelectionDiagnostic::LegacyRow(LegacyRow::Item(0)),
            SelectionDiagnostic::LegacyRow(LegacyRow::Item(1))
        ])
    );
}

#[test]
fn new_bundle_can_reference_untouched_installation_without_replacing_it() {
    use upskill::lifecycle::state::BundleNode;
    let state = prior();
    let bundles = [BundleNode {
        id: "new-bundle".into(),
        source: "./bundle".into(),
        name: "new-bundle".into(),
        source_revision: None,
    }];
    let edges = [
        Reference::DirectBundle {
            bundle_id: "new-bundle".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "new-bundle".into(),
            installation_id: "a".into(),
        },
    ];
    let p = ready(
        plan_upsert(
            &state,
            UpsertRequest {
                bundles: &bundles,
                ..request(None, &[], &edges, &[])
            },
            &[],
        )
        .unwrap(),
    );
    assert_eq!(p.state.installations, state.installations);
    assert_eq!(p.state.artifacts, state.artifacts);
    assert_eq!(p.state.owner_consumers, state.owner_consumers);
    assert_eq!(p.state.bundles, bundles);
    assert_eq!(
        p.state.references,
        [state.references, edges.to_vec()].concat()
    );
    assert!(p.files.is_empty());
}

#[test]
fn invalid_prior_and_source_binding_changes_fail_before_any_file_plan() {
    let mut state = prior();
    state.schema = 1;
    assert!(plan_upsert(&state, request(None, &[], &[], &[]), &[]).is_err());
    let state = prior();
    for field in ["source", "alias", "scope", "raw-root"] {
        let mut s = set("a", "a.md", b"next");
        match field {
            "source" => s.installation.owner.source = "./different".into(),
            "alias" => s.installation.owner.installed_name = "different".into(),
            "scope" => s.installation.root.scope = OwnershipScope::User,
            _ => s.installation.root.path = root().path.join(".").join("different"),
        }
        assert!(
            plan_upsert(
                &state,
                request(Some(&named("a")), &[s], &[], &[]),
                &[observed("a.md", Some(b"a"))]
            )
            .is_err(),
            "{field}"
        );
    }
}

fn with_bundle() -> CombinedState {
    let mut state = prior();
    state.bundles = vec![upskill::lifecycle::state::BundleNode {
        id: "bundle".into(),
        source: "./bundle".into(),
        name: "bundle".into(),
        source_revision: None,
    }];
    state.references.extend([
        Reference::DirectBundle {
            bundle_id: "bundle".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "bundle".into(),
            installation_id: "a".into(),
        },
    ]);
    validate_state(&state).unwrap();
    state
}
#[test]
fn rejects_unselected_reference_edits_without_retiring_any_node() {
    let state = with_bundle();
    for edge in [
        direct("a"),
        Reference::BundleInstallation {
            bundle_id: "bundle".into(),
            installation_id: "a".into(),
        },
    ] {
        let error = plan_upsert(&state, request(None, &[], &[], &[edge]), &[]).unwrap_err();
        assert!(format!("{error:#}").contains("unselected existing root"));
    }
}
#[test]
fn rejects_unselected_existing_bundle_metadata_upsert() {
    let state = with_bundle();
    let mut bundles = state.bundles.clone();
    bundles[0].source_revision = Some("next".into());
    let error = plan_upsert(
        &state,
        UpsertRequest {
            bundles: &bundles,
            ..request(None, &[], &[], &[])
        },
        &[],
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("unselected bundle"));
    assert_eq!(state.bundles[0].source_revision, None);
}
#[test]
fn legacy_collision_on_old_path_blocks_retirement_and_relocation() {
    let mut state = prior();
    let old = ".claude/rules/old.md";
    state.artifacts[0].path = old.into();
    state.owner_consumers[0].path = old.into();
    state.legacy = Some(upskill::lockfile::Lockfile {
        items: vec![legacy("old")],
        ..Default::default()
    });
    validate_state(&state).unwrap();
    let e = plan_upsert(
        &state,
        request(Some(&named("a")), &[], &[], &[direct("a")]),
        &[observed(old, Some(b"a"))],
    )
    .unwrap_err();
    assert!(format!("{e:#}").contains("legacy destination"));
    let e = plan_upsert(
        &state,
        request(Some(&named("a")), &[set("a", "new.md", b"new")], &[], &[]),
        &[observed(old, Some(b"a")), observed("new.md", None)],
    )
    .unwrap_err();
    assert!(format!("{e:#}").contains("legacy destination"));
}
#[test]
fn newly_appended_associations_follow_location_order_not_request_order() {
    let sets = [set("a", "z.md", b"a"), set("z", "a.md", b"z")];
    let p = ready(
        plan_upsert(
            &empty(),
            request(None, &sets, &[direct("z"), direct("a")], &[]),
            &[observed("z.md", None), observed("a.md", None)],
        )
        .unwrap(),
    );
    assert_eq!(
        p.state
            .owner_consumers
            .iter()
            .map(|r| (r.path.as_str(), r.installation_id.as_str()))
            .collect::<Vec<_>>(),
        vec![("a.md", "z"), ("z.md", "a")]
    );
}
#[test]
fn duplicate_legacy_blocker_from_both_probes_is_reported_once() {
    use upskill::lifecycle::selection::{LegacyRow, SelectionDiagnostic};
    let mut state = empty();
    state.legacy = Some(upskill::lockfile::Lockfile {
        items: vec![legacy("legacy")],
        ..Default::default()
    });
    let mut s = set("new", "new.md", b"new");
    s.installation.owner.installed_name = "legacy".into();
    let outcome = plan_upsert(
        &state,
        request(Some(&named("legacy")), &[s], &[direct("new")], &[]),
        &[],
    )
    .unwrap();
    assert_eq!(
        outcome,
        UpsertOutcome::Blocked(vec![SelectionDiagnostic::LegacyRow(LegacyRow::Item(0))])
    );
}

#[test]
fn added_existing_references_require_selection_and_selected_counterparts_pass() {
    use upskill::lifecycle::state::BundleNode;
    let mut bundled = with_bundle();
    bundled.references.retain(|r| r != &direct("a"));
    let member_state = with_bundle();
    let member = Reference::BundleInstallation {
        bundle_id: "bundle".into(),
        installation_id: "b".into(),
    };
    let dep = BundleNode {
        id: "dep".into(),
        source: "./dep".into(),
        name: "dep".into(),
        source_revision: None,
    };
    let mut dependency_state = with_bundle();
    dependency_state.bundles.push(dep.clone());
    dependency_state.references.push(Reference::DirectBundle {
        bundle_id: "dep".into(),
    });
    let dependency = Reference::BundleDependency {
        bundle_id: "bundle".into(),
        dependency_id: "dep".into(),
    };
    let mut direct_bundle_state = with_bundle();
    direct_bundle_state.bundles.push(dep);
    direct_bundle_state.references.push(dependency.clone());
    let cases = vec![
        (
            bundled,
            direct("a"),
            named("a"),
            vec![set("a", "a.md", b"a")],
            vec![observed("a.md", Some(b"a"))],
        ),
        (
            member_state,
            member,
            named("bundle"),
            vec![set("a", "a.md", b"a")],
            vec![observed("a.md", Some(b"a"))],
        ),
        (
            dependency_state,
            dependency,
            named("bundle"),
            vec![set("a", "a.md", b"a")],
            vec![observed("a.md", Some(b"a"))],
        ),
        (
            direct_bundle_state,
            Reference::DirectBundle {
                bundle_id: "dep".into(),
            },
            named("dep"),
            vec![],
            vec![],
        ),
    ];
    for (state, edge, selection, sets, observations) in cases {
        validate_state(&state).unwrap();
        let error = plan_upsert(
            &state,
            request(None, &[], std::slice::from_ref(&edge), &[]),
            &[],
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("unselected existing root"));
        let p = ready(
            plan_upsert(
                &state,
                request(Some(&selection), &sets, std::slice::from_ref(&edge), &[]),
                &observations,
            )
            .unwrap(),
        );
        let mut expected = state.references.clone();
        expected.push(edge);
        assert_eq!(p.state.references, expected);
        assert_eq!(p.state.installations, state.installations);
        assert_eq!(p.state.bundles, state.bundles);
        assert_eq!(p.state.artifacts, state.artifacts);
        assert_eq!(p.state.owner_consumers, state.owner_consumers);
        validate_state(&p.state).unwrap();
    }
}
#[test]
fn selected_existing_bundle_metadata_upsert_is_accepted() {
    let state = with_bundle();
    let mut bundles = state.bundles.clone();
    bundles[0].source_revision = Some("next".into());
    let sets = [set("a", "a.md", b"a")];
    let p = ready(
        plan_upsert(
            &state,
            UpsertRequest {
                bundles: &bundles,
                ..request(Some(&named("bundle")), &sets, &[], &[])
            },
            &[observed("a.md", Some(b"a"))],
        )
        .unwrap(),
    );
    assert_eq!(p.state.bundles, bundles);
    assert_eq!(p.state.installations, state.installations);
    assert_eq!(p.state.references, state.references);
    assert_eq!(p.state.artifacts, state.artifacts);
    assert_eq!(p.state.owner_consumers, state.owner_consumers);
    assert_eq!(p.files[0].before, p.files[0].after);
}
#[test]
fn merged_blockers_keep_prior_probe_then_original_additional_row_order() {
    use upskill::lifecycle::selection::{LegacyRow, SelectionDiagnostic};
    let mut state = empty();
    state.legacy = Some(upskill::lockfile::Lockfile {
        items: vec![legacy("new-z"), legacy("old"), legacy("new-a")],
        ..Default::default()
    });
    let outcome = plan_upsert(
        &state,
        request(
            Some(&named("old")),
            &[set("new-a", "a.md", b"a"), set("new-z", "z.md", b"z")],
            &[],
            &[],
        ),
        &[],
    )
    .unwrap();
    assert_eq!(
        outcome,
        UpsertOutcome::Blocked(vec![
            SelectionDiagnostic::LegacyRow(LegacyRow::Item(1)),
            SelectionDiagnostic::LegacyRow(LegacyRow::Item(0)),
            SelectionDiagnostic::LegacyRow(LegacyRow::Item(2))
        ])
    );
}

#[test]
fn consumer_associations_remain_specific_to_each_artifact() {
    let mut s = set("a", "a.md", b"a");
    s.installation
        .requested
        .extend([TargetSurface::CopilotCli, TargetSurface::OpenCodeCli]);
    s.audience_excluded = vec![TargetSurface::OpenCodeCli];
    s.artifacts.push(DesiredArtifact::File(DesiredFile {
        location: location("b.md"),
        value: value(b"b"),
        consumers: vec![TargetSurface::CopilotCli],
    }));
    let p = ready(
        plan_upsert(
            &empty(),
            request(None, &[s], &[direct("a")], &[]),
            &[observed("b.md", None), observed("a.md", None)],
        )
        .unwrap(),
    );
    assert_eq!(
        p.state
            .owner_consumers
            .iter()
            .map(|r| (r.path.as_str(), r.consumers.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("a.md", vec![TargetSurface::ClaudeCli]),
            ("b.md", vec![TargetSurface::CopilotCli])
        ]
    );
    assert_eq!(
        p.state
            .artifacts
            .iter()
            .map(|r| (r.path.as_str(), r.consumers.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("a.md", vec![TargetSurface::ClaudeCli]),
            ("b.md", vec![TargetSurface::CopilotCli])
        ]
    );
}
#[test]
fn replacing_association_keeps_its_prior_position_among_untouched_rows() {
    let mut state = prior();
    state.owner_consumers.reverse();
    validate_state(&state).unwrap();
    let mut s = set("a", "a.md", b"next");
    s.installation.requested.push(TargetSurface::CopilotCli);
    if let DesiredArtifact::File(file) = &mut s.artifacts[0] {
        file.consumers = vec![TargetSurface::CopilotCli];
    }

    s.artifacts.push(DesiredArtifact::File(DesiredFile {
        location: location("c.md"),
        value: value(b"c"),
        consumers: vec![TargetSurface::ClaudeCli],
    }));
    let p = ready(
        plan_upsert(
            &state,
            request(Some(&named("a")), &[s], &[], &[]),
            &[observed("a.md", Some(b"a")), observed("c.md", None)],
        )
        .unwrap(),
    );
    let mut expected = state.owner_consumers.clone();
    expected[1].consumers = vec![TargetSurface::CopilotCli];
    expected.push(OwnerConsumers {
        root: root(),
        path: "c.md".into(),
        contribution_id: None,
        installation_id: "a".into(),
        consumers: vec![TargetSurface::ClaudeCli],
    });
    assert_eq!(p.state.owner_consumers, expected);
    assert_eq!(
        p.state
            .owner_consumers
            .iter()
            .map(|r| r.installation_id.as_str())
            .collect::<Vec<_>>(),
        vec!["b", "a", "a"]
    );
    assert_eq!(p.state.artifacts[0].owners, vec![item("a").owner]);
    assert_eq!(
        p.state.artifacts[0].consumers,
        vec![TargetSurface::CopilotCli]
    );
    assert_eq!(
        p.state.installations[0].requested,
        vec![TargetSurface::ClaudeCli, TargetSurface::CopilotCli]
    );
    assert_eq!(p.files[0].after, Some(value(b"next")));
}

#[test]
fn disjoint_update_preserves_duplicate_legacy_and_native_rows_with_statuses() {
    use upskill::lockfile::{
        LockedBundle, LockedMcp, LockedPlugin, Lockfile, McpInstallStatus, PluginInstallStatus,
    };
    let mut state = prior();
    let plugin = LockedPlugin {
        name: "native-old".into(),
        client: "unknown".into(),
        identifier: "opaque".into(),
        scope: Some("original-scope".into()),
        bundle: "old-bundle".into(),
        status: PluginInstallStatus::Instructions,
    };
    let mcp = LockedMcp {
        name: "mcp-old".into(),
        client: "unknown".into(),
        scope: Some("original-scope".into()),
        bundle: "old-bundle".into(),
        status: McpInstallStatus::Skipped,
    };
    let bundle = LockedBundle {
        name: "old-bundle".into(),
        source: "malformed".into(),
        git_ref: Some("original-ref".into()),
        items: vec!["old".into(), "missing".into()],
    };
    let lock = Lockfile {
        schema: 1,
        items: vec![legacy("old"), legacy("old"), legacy("aaa-old")],
        bundles: vec![
            bundle.clone(),
            bundle,
            LockedBundle {
                name: "aaa-bundle".into(),
                source: "opaque-label".into(),
                git_ref: None,
                items: vec!["aaa-old".into()],
            },
        ],
        plugins: vec![
            plugin.clone(),
            plugin,
            LockedPlugin {
                name: "aaa-native".into(),
                client: "claude".into(),
                identifier: "aaa-opaque".into(),
                scope: None,
                bundle: "aaa-bundle".into(),
                status: PluginInstallStatus::Skipped,
            },
        ],
        mcps: vec![
            mcp.clone(),
            mcp,
            LockedMcp {
                name: "aaa-mcp".into(),
                client: "claude".into(),
                scope: None,
                bundle: "aaa-bundle".into(),
                status: McpInstallStatus::Installed,
            },
        ],
    };
    state.legacy = Some(lock.clone());
    validate_state(&state).unwrap();
    let p = ready(
        plan_upsert(
            &state,
            request(Some(&named("a")), &[set("a", "a.md", b"next")], &[], &[]),
            &[observed("a.md", Some(b"a"))],
        )
        .unwrap(),
    );
    assert_eq!(p.state.legacy, Some(lock));
    assert_eq!(p.state.installations, state.installations);
    assert_eq!(p.state.references, state.references);
    assert_eq!(p.state.artifacts[1], state.artifacts[1]);
}

#[test]
fn bundle_dependency_retirement_preserves_direct_retained_member() {
    use upskill::lifecycle::state::BundleNode;
    let mut state = prior();
    state.bundles = vec![
        BundleNode {
            id: "parent".into(),
            source: "./parent".into(),
            name: "parent".into(),
            source_revision: None,
        },
        BundleNode {
            id: "dep".into(),
            source: "./dep".into(),
            name: "dep".into(),
            source_revision: None,
        },
    ];
    state.references = vec![
        direct("b"),
        Reference::DirectBundle {
            bundle_id: "parent".into(),
        },
        Reference::BundleDependency {
            bundle_id: "parent".into(),
            dependency_id: "dep".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "dep".into(),
            installation_id: "a".into(),
        },
        Reference::BundleInstallation {
            bundle_id: "dep".into(),
            installation_id: "b".into(),
        },
    ];
    validate_state(&state).unwrap();
    let p = ready(
        plan_upsert(
            &state,
            request(
                Some(&named("parent")),
                &[set("b", "b.md", b"b")],
                &[],
                &[Reference::DirectBundle {
                    bundle_id: "parent".into(),
                }],
            ),
            &[observed("a.md", Some(b"a")), observed("b.md", Some(b"b"))],
        )
        .unwrap(),
    );
    assert_eq!(p.state.installations, vec![item("b")]);
    assert!(p.state.bundles.is_empty());
    assert_eq!(p.state.references, vec![direct("b")]);
    assert_eq!(p.state.artifacts, vec![state.artifacts[1].clone()]);
    assert_eq!(
        p.state.owner_consumers,
        vec![state.owner_consumers[1].clone()]
    );
    assert_eq!(
        p.files
            .iter()
            .map(|f| f.location.clone())
            .collect::<Vec<_>>(),
        vec![location("a.md"), location("b.md")]
    );
    assert_eq!(p.files[0].after, None);
    assert_eq!(p.files[1].before, Some(value(b"b")));
    assert_eq!(p.files[1].after, Some(value(b"b")));
}
#[test]
fn complete_mixed_set_can_become_empty_without_losing_history_or_manual_text() {
    use upskill::instructions::plan::{DesiredInstructionContribution, InstructionLocation};
    let mut s = set("a", "a.md", b"a");
    s.artifacts.push(DesiredArtifact::Instruction(
        DesiredInstructionContribution {
            location: InstructionLocation {
                root: root(),
                path: "AGENTS.md".into(),
            },
            id: "intro".into(),
            markdown: "Owned\n".into(),
            consumers: vec![TargetSurface::ClaudeCli],
        },
    ));
    let p = ready(
        plan_upsert(
            &empty(),
            request(None, &[s], &[direct("a")], &[]),
            &[
                observed("AGENTS.md", Some(b"Manual\n")),
                observed("a.md", None),
            ],
        )
        .unwrap(),
    );
    assert_eq!(p.state.artifacts.len(), 2);
    assert_eq!(p.state.owner_consumers.len(), 2);
    assert_eq!(
        p.files
            .iter()
            .map(|f| f.location.clone())
            .collect::<Vec<_>>(),
        vec![location("AGENTS.md"), location("a.md")]
    );
    assert_eq!(p.files[1].after, Some(value(b"a")));
    assert!(
        p.files[0]
            .after
            .as_ref()
            .unwrap()
            .bytes
            .starts_with(b"Manual\n")
    );
    let observations = p
        .files
        .iter()
        .map(|f| ObservedFile {
            location: f.location.clone(),
            value: f.after.clone(),
        })
        .collect::<Vec<_>>();
    let s = DesiredInstallationSet {
        installation: item("a"),
        artifacts: vec![],
        audience_excluded: vec![TargetSurface::ClaudeCli],
    };
    let next = ready(
        plan_upsert(
            &p.state,
            request(Some(&named("a")), &[s], &[], &[]),
            &observations,
        )
        .unwrap(),
    );
    assert_eq!(next.state.installations, vec![item("a")]);
    assert_eq!(next.state.references, vec![direct("a")]);
    assert!(next.state.artifacts.is_empty());
    assert!(next.state.owner_consumers.is_empty());
    assert_eq!(next.files[0].location, location("AGENTS.md"));
    assert_eq!(next.files[0].after, Some(value(b"Manual\n")));
    assert_eq!(next.files[1].location, location("a.md"));
    assert_eq!(next.files[1].after, None);
}
