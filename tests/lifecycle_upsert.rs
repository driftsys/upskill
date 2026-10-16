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
