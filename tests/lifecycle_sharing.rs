use sha2::{Digest, Sha256};
use upskill::files::plan::{DesiredFile, FileLocation, FileValue, ObservedFile};
use upskill::instructions::aggregate::{self, Contribution};
use upskill::instructions::plan::{DesiredInstructionContribution, InstructionLocation};
use upskill::lifecycle::contracts::{ConsumerContract, ContractContent, OwnerContract};
use upskill::lifecycle::desired::DesiredInstallationSet;
use upskill::lifecycle::selection::Selection;
use upskill::lifecycle::state::{
    CombinedState, Installation, OwnerConsumers, Reference, SourceEvidence, parse_state,
    serialize_state, validate_state,
};
use upskill::lifecycle::upsert::{
    UpsertOutcome, UpsertPlan, UpsertRequest, plan_shared_upsert, plan_upsert,
};
use upskill::operations::plan::DesiredArtifact;
use upskill::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn hash(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn root() -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: std::env::temp_dir().join("upskill-sharing"),
    }
}
fn item(id: &str) -> Installation {
    Installation {
        owner: ArtifactOwner {
            installation_id: id.into(),
            source: format!("./{id}"),
            source_revision: None,
            kind: ItemKind::Rule,
            source_name: "source".into(),
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
fn empty() -> CombinedState {
    CombinedState {
        schema: 2,
        installations: vec![],
        bundles: vec![],
        references: vec![],
        artifacts: vec![],
        owner_consumers: vec![],
        owner_contracts: vec![],
        legacy: None,
    }
}
fn set(id: &str, b: &[u8]) -> DesiredInstallationSet {
    DesiredInstallationSet {
        installation: item(id),
        artifacts: vec![DesiredArtifact::File(DesiredFile {
            location: FileLocation {
                root: root(),
                path: "shared.md".into(),
            },
            value: FileValue {
                bytes: b.into(),
                mode: Some(0o640),
            },
            consumers: vec![TargetSurface::ClaudeCli],
        })],
        audience_excluded: vec![],
    }
}
fn instruction(id: &str, b: &str) -> DesiredInstallationSet {
    DesiredInstallationSet {
        installation: item(id),
        artifacts: vec![DesiredArtifact::Instruction(
            DesiredInstructionContribution {
                location: InstructionLocation {
                    root: root(),
                    path: "AGENTS.md".into(),
                },
                id: "shared".into(),
                markdown: b.into(),
                consumers: vec![TargetSurface::ClaudeCli],
            },
        )],
        audience_excluded: vec![],
    }
}
fn proofs(sets: &[DesiredInstallationSet]) -> Vec<OwnerContract> {
    sets.iter()
        .flat_map(|s| {
            s.artifacts.iter().map(|a| {
                let (path, selector, content, consumers) = match a {
                    DesiredArtifact::File(f) => (
                        &f.location.path,
                        None,
                        ContractContent::File {
                            content_sha256: hash(&f.value.bytes),
                            mode: f.value.mode,
                        },
                        &f.consumers,
                    ),
                    DesiredArtifact::Instruction(v) => (
                        &v.location.path,
                        Some(v.id.clone()),
                        ContractContent::Instruction {
                            content_sha256: hash(v.markdown.as_bytes()),
                        },
                        &v.consumers,
                    ),
                };
                let source = match &s.installation.source_evidence {
                    SourceEvidence::Local { manifest_sha256 }
                    | SourceEvidence::Git {
                        manifest_sha256, ..
                    } => manifest_sha256,
                };
                OwnerContract {
                    root: s.installation.root.clone(),
                    path: path.clone(),
                    contribution_id: selector,
                    installation_id: s.installation.owner.installation_id.clone(),
                    source_manifest_sha256: source.clone(),
                    content,
                    profiles: consumers
                        .iter()
                        .map(|surface| ConsumerContract {
                            surface: *surface,
                            contract_revision: 7,
                        })
                        .collect(),
                }
            })
        })
        .collect()
}
fn association(id: &str, path: &str, selector: Option<&str>) -> OwnerConsumers {
    OwnerConsumers {
        root: root(),
        path: path.into(),
        contribution_id: selector.map(str::to_owned),
        installation_id: id.into(),
        consumers: vec![TargetSurface::ClaudeCli],
    }
}
fn shared() -> CombinedState {
    let mut s = empty();
    s.installations = vec![item("a"), item("b")];
    s.references = vec![direct("a"), direct("b")];
    s.artifacts = vec![OwnedArtifact {
        root: root(),
        path: "shared.md".into(),
        content: ArtifactContent::File {
            content_sha256: hash(b"old"),
            mode: Some(0o640),
        },
        owners: vec![item("b").owner, item("a").owner],
        consumers: vec![TargetSurface::ClaudeCli],
    }];
    s.owner_consumers = vec![
        association("b", "shared.md", None),
        association("a", "shared.md", None),
    ];
    s.owner_contracts = proofs(&[set("b", b"old"), set("a", b"old")]);
    validate_state(&s).unwrap();
    s
}
fn observed(path: &str, b: Option<&[u8]>) -> ObservedFile {
    ObservedFile {
        location: FileLocation {
            root: root(),
            path: path.into(),
        },
        value: b.map(|b| FileValue {
            bytes: b.into(),
            mode: Some(0o640),
        }),
    }
}
fn request<'a>(
    sets: &'a [DesiredInstallationSet],
    contracts: &'a [OwnerContract],
    sel: Option<&'a Selection>,
    add: &'a [Reference],
    remove: &'a [Reference],
) -> UpsertRequest<'a> {
    UpsertRequest {
        selection: sel,
        sets,
        contracts,
        bundles: &[],
        add_references: add,
        remove_references: remove,
    }
}
fn ready(r: anyhow::Result<UpsertOutcome>) -> UpsertPlan {
    let UpsertOutcome::Ready(p) = r.unwrap() else {
        panic!("unexpected blockers")
    };
    *p
}
fn ids(s: &CombinedState) -> Vec<&str> {
    s.artifacts[0]
        .owners
        .iter()
        .map(|o| o.installation_id.as_str())
        .collect()
}

#[test]
fn explicit_new_sharing_has_real_owners_and_old_api_stays_restrictive() {
    let prior = empty();
    let sets = [set("b", b"old"), set("a", b"old")];
    let c = proofs(&sets);
    let refs = [direct("a"), direct("b")];
    let req = request(&sets, &c, None, &refs, &[]);
    let obs = [observed("shared.md", None)];
    let before = (prior.clone(), sets.clone(), c.clone(), obs.clone());
    assert!(plan_upsert(&prior, req, &obs).is_err());
    let p = ready(plan_shared_upsert(&prior, req, &obs));
    assert_eq!(ids(&p.state), vec!["a", "b"]);
    assert_eq!(p.state.owner_contracts.len(), 2);
    assert_eq!(p.files.len(), 1);
    assert_eq!(p.files[0].before, None);
    assert_eq!(
        p.files[0].after,
        Some(FileValue {
            bytes: b"old".to_vec(),
            mode: Some(0o640)
        })
    );
    assert_eq!(
        parse_state(&serialize_state(&p.state).unwrap())
            .unwrap()
            .owner_contracts,
        p.state.owner_contracts
    );
    assert_eq!((prior, sets, c, obs), before);
}
#[test]
fn adding_owner_preserves_retained_real_proof_and_checks_noop_bytes() {
    let s = shared();
    let sets = [set("c", b"old")];
    let c = proofs(&sets);
    let add = [direct("c")];
    let obs = observed("shared.md", Some(b"old"));
    let p = ready(plan_shared_upsert(
        &s,
        request(&sets, &c, None, &add, &[]),
        std::slice::from_ref(&obs),
    ));
    assert_eq!(ids(&p.state), vec!["b", "a", "c"]);
    assert_eq!(&p.state.owner_contracts[..2], &s.owner_contracts);
    assert_eq!(p.files[0].before, obs.value);
    assert_eq!(p.files[0].after, obs.value);
}
#[test]
fn coordinated_update_requires_every_new_contract_and_preserves_owner_order() {
    let s = shared();
    let mut sets = [set("a", b"new"), set("b", b"new")];
    for set in &mut sets {
        set.installation.source_evidence = SourceEvidence::Local {
            manifest_sha256: "b".repeat(64),
        };
    }
    let c = proofs(&sets);
    let sel = Selection::All;
    let obs = [observed("shared.md", Some(b"old"))];
    let p = ready(plan_shared_upsert(
        &s,
        request(&sets, &c, Some(&sel), &[], &[]),
        &obs,
    ));
    assert_eq!(ids(&p.state), vec!["b", "a"]);
    assert_eq!(p.files[0].after.as_ref().unwrap().bytes, b"new");
    assert!(
        p.state
            .owner_contracts
            .iter()
            .all(|c| c.source_manifest_sha256 == "b".repeat(64))
    );
    assert!(plan_shared_upsert(&s, request(&sets, &c[..1], Some(&sel), &[], &[]), &obs).is_err());
}
#[test]
fn explicit_complete_requalification_does_not_infer_unknown_retained_proof() {
    let mut s = shared();
    s.owner_contracts.clear();
    let sets = [set("a", b"old"), set("b", b"old")];
    let c = proofs(&sets);
    let all = Selection::All;
    let obs = [observed("shared.md", Some(b"old"))];
    assert_eq!(
        ready(plan_shared_upsert(
            &s,
            request(&sets, &c, Some(&all), &[], &[]),
            &obs
        ))
        .state
        .owner_contracts
        .len(),
        2
    );
    let a = Selection::Names(vec!["a".into()]);
    assert!(
        plan_shared_upsert(&s, request(&sets[..1], &c[..1], Some(&a), &[], &[]), &obs).is_err()
    );
    assert!(plan_shared_upsert(&s, request(&sets, &[], Some(&all), &[], &[]), &obs).is_err());
}
#[test]
fn partial_content_source_and_profile_updates_cannot_change_retained_owners() {
    let s = shared();
    let sel = Selection::Names(vec!["a".into()]);
    let obs = [observed("shared.md", Some(b"old"))];
    let sets = [set("a", b"new")];
    assert!(
        plan_shared_upsert(
            &s,
            request(&sets, &proofs(&sets), Some(&sel), &[], &[]),
            &obs
        )
        .is_err()
    );
    let mut sets = [set("a", b"old")];
    sets[0].installation.source_evidence = SourceEvidence::Local {
        manifest_sha256: "b".repeat(64),
    };
    assert!(
        plan_shared_upsert(
            &s,
            request(&sets, &proofs(&sets), Some(&sel), &[], &[]),
            &obs
        )
        .is_err()
    );
    let sets = [set("a", b"old")];
    let mut c = proofs(&sets);
    c[0].profiles[0].contract_revision = 8;
    assert!(plan_shared_upsert(&s, request(&sets, &c, Some(&sel), &[], &[]), &obs).is_err());
}
#[test]
fn desired_bytes_and_modes_must_agree_at_one_identity() {
    let s = empty();
    let refs = [direct("a"), direct("b")];
    let obs = [observed("shared.md", None)];
    let sets = [set("a", b"old"), set("b", b"different")];
    assert!(
        plan_shared_upsert(&s, request(&sets, &proofs(&sets), None, &refs, &[]), &obs).is_err()
    );
    let mut sets = [set("a", b"old"), set("b", b"old")];
    let DesiredArtifact::File(f) = &mut sets[1].artifacts[0] else {
        unreachable!()
    };
    f.value.mode = Some(0o600);
    assert!(
        plan_shared_upsert(&s, request(&sets, &proofs(&sets), None, &refs, &[]), &obs).is_err()
    );
}
#[test]
fn owner_release_and_final_retirement_preserve_unknown_history_safely() {
    let mut s = shared();
    s.owner_contracts.clear();
    let mut release = set("a", b"");
    release.artifacts.clear();
    release.audience_excluded = vec![TargetSurface::ClaudeCli];
    let sel = Selection::Names(vec!["a".into()]);
    let obs = observed("shared.md", Some(b"old"));
    let p = ready(plan_shared_upsert(
        &s,
        request(&[release], &[], Some(&sel), &[], &[]),
        std::slice::from_ref(&obs),
    ));
    assert_eq!(ids(&p.state), vec!["b"]);
    assert_eq!(p.files[0].after, obs.value);
    assert_eq!(p.state.installations.len(), 2);
    assert!(p.state.owner_contracts.is_empty());
    let all = Selection::All;
    let remove = [direct("a"), direct("b")];
    let p = ready(plan_shared_upsert(
        &s,
        request(&[], &[], Some(&all), &[], &remove),
        &[obs],
    ));
    assert!(p.state.artifacts.is_empty());
    assert!(p.state.installations.is_empty());
    assert_eq!(p.files[0].after, None);
}
#[test]
fn projection_cannot_bypass_immutable_real_bindings_or_unselected_proof_guard() {
    let s = shared();
    let all = Selection::All;
    let obs = [observed("shared.md", Some(b"old"))];
    for field in 0..3 {
        let mut sets = [set("a", b"old"), set("b", b"old")];
        match field {
            0 => sets[1].installation.owner.source = "./other".into(),
            1 => sets[1].installation.owner.source_name = "other".into(),
            _ => sets[1].installation.owner.installed_name = "other".into(),
        };
        assert!(
            plan_shared_upsert(
                &s,
                request(&sets, &proofs(&sets), Some(&all), &[], &[]),
                &obs
            )
            .is_err()
        );
    }
    let mut s = s;
    s.owner_contracts.retain(|c| c.installation_id != "b");
    let sets = [set("a", b"old")];
    let c = proofs(&[set("a", b"old"), set("b", b"old")]);
    let a = Selection::Names(vec!["a".into()]);
    assert!(plan_shared_upsert(&s, request(&sets, &c, Some(&a), &[], &[]), &obs).is_err());
}
#[test]
fn all_observations_are_exact_unique_and_owned_even_for_noops() {
    let s = shared();
    let sets = [set("a", b"old"), set("b", b"old")];
    let c = proofs(&sets);
    let all = Selection::All;
    let req = request(&sets, &c, Some(&all), &[], &[]);
    let good = observed("shared.md", Some(b"old"));
    let mut mode = good.clone();
    mode.value.as_mut().unwrap().mode = Some(0o600);
    let mut root_alias = good.clone();
    root_alias.location.root.path = root().path.join(".");
    for obs in [
        vec![],
        vec![good.clone(), good.clone()],
        vec![observed("shared.md", Some(b"edited"))],
        vec![observed("shared.md", None)],
        vec![mode],
        vec![root_alias],
        vec![good, observed("extra.md", None)],
    ] {
        assert!(plan_shared_upsert(&s, req, &obs).is_err());
    }
    let empty = empty();
    let refs = [direct("a"), direct("b")];
    assert!(
        plan_shared_upsert(
            &empty,
            request(&sets, &c, None, &refs, &[]),
            &[observed("shared.md", Some(b"old"))]
        )
        .is_err()
    );
}
#[test]
fn malformed_unrelated_state_and_legacy_names_block_before_ready() {
    let mut s = shared();
    s.references.push(direct("unknown"));
    let sets = [set("a", b"old"), set("b", b"old")];
    let c = proofs(&sets);
    let all = Selection::All;
    assert!(
        plan_shared_upsert(
            &s,
            request(&sets, &c, Some(&all), &[], &[]),
            &[observed("shared.md", Some(b"old"))]
        )
        .is_err()
    );
    let mut s = empty();
    let mut lock = upskill::lockfile::Lockfile::default();
    lock.items.push(upskill::lockfile::LockedItem {
        kind: ItemKind::Rule,
        name: "a".into(),
        source: "./legacy".into(),
        git_ref: None,
        hash: None,
        source_name: None,
        required_by: vec![],
        group: None,
        clients: vec!["claude".into()],
    });
    s.legacy = Some(lock);
    let refs = [direct("a"), direct("b")];
    assert!(matches!(
        plan_shared_upsert(
            &s,
            request(&sets, &c, None, &refs, &[]),
            &[observed("shared.md", None)]
        )
        .unwrap(),
        UpsertOutcome::Blocked(_)
    ));
}
fn shared_instructions() -> (CombinedState, ObservedFile) {
    let edit =
        aggregate::reconcile("Manual\n", &[], &[Contribution::new("shared", "Old\n")]).unwrap();
    let mut s = shared();
    s.artifacts[0].path = "AGENTS.md".into();
    s.artifacts[0].content = ArtifactContent::MarkdownContribution {
        contribution: edit.owned[0].clone(),
    };
    s.owner_consumers = vec![
        association("b", "AGENTS.md", Some("shared")),
        association("a", "AGENTS.md", Some("shared")),
    ];
    s.owner_contracts = proofs(&[instruction("b", "Old\n"), instruction("a", "Old\n")]);
    validate_state(&s).unwrap();
    (s, observed("AGENTS.md", Some(edit.content.as_bytes())))
}
#[test]
fn shared_instruction_update_preserves_manual_bytes_modes_and_real_owners() {
    let (s, obs) = shared_instructions();
    let sets = [instruction("a", "New\n"), instruction("b", "New\n")];
    let c = proofs(&sets);
    let all = Selection::All;
    let p = ready(plan_shared_upsert(
        &s,
        request(&sets, &c, Some(&all), &[], &[]),
        std::slice::from_ref(&obs),
    ));
    assert_eq!(ids(&p.state), vec!["b", "a"]);
    let after = p.files[0].after.as_ref().unwrap();
    assert!(after.bytes.starts_with(b"Manual\n"));
    assert_eq!(after.mode, Some(0o640));
    let records = p
        .state
        .artifacts
        .iter()
        .map(|a| {
            let ArtifactContent::MarkdownContribution { contribution } = &a.content else {
                panic!("wrong form")
            };
            contribution.clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        aggregate::inspect_owned(std::str::from_utf8(&after.bytes).unwrap(), &records).unwrap(),
        vec![Contribution::new("shared", "New\n")]
    );
    assert!(plan_upsert(&s, request(&sets, &c, Some(&all), &[], &[]), &[obs]).is_err());
}
#[test]
fn final_instruction_retirement_keeps_manual_aggregate_present() {
    let (s, obs) = shared_instructions();
    let all = Selection::All;
    let remove = [direct("a"), direct("b")];
    let p = ready(plan_shared_upsert(
        &s,
        request(&[], &[], Some(&all), &[], &remove),
        &[obs],
    ));
    assert!(p.state.artifacts.is_empty());
    assert_eq!(
        p.files[0].after,
        Some(FileValue {
            bytes: b"Manual\n".to_vec(),
            mode: Some(0o640)
        })
    );
}
#[test]
fn unchanged_shared_instruction_neighbors_keep_unknown_contracts() {
    let (mut s, _) = shared_instructions();
    let edit = aggregate::reconcile(
        "Manual\n",
        &[],
        &[
            Contribution::new("shared", "Old\n"),
            Contribution::new("neighbor", "Keep\n"),
        ],
    )
    .unwrap();
    s.artifacts[0].content = ArtifactContent::MarkdownContribution {
        contribution: edit
            .owned
            .iter()
            .find(|c| c.id == "shared")
            .unwrap()
            .clone(),
    };
    s.artifacts.push(OwnedArtifact {
        root: root(),
        path: "AGENTS.md".into(),
        content: ArtifactContent::MarkdownContribution {
            contribution: edit
                .owned
                .iter()
                .find(|c| c.id == "neighbor")
                .unwrap()
                .clone(),
        },
        owners: vec![item("d").owner, item("c").owner],
        consumers: vec![TargetSurface::ClaudeCli],
    });
    for id in ["c", "d"] {
        s.installations.push(item(id));
        s.references.push(direct(id));
        s.owner_consumers
            .push(association(id, "AGENTS.md", Some("neighbor")));
    }
    validate_state(&s).unwrap();
    let sets = [instruction("a", "New\n"), instruction("b", "New\n")];
    let c = proofs(&sets);
    let sel = Selection::Names(vec!["a".into(), "b".into()]);
    let p = ready(plan_shared_upsert(
        &s,
        request(&sets, &c, Some(&sel), &[], &[]),
        &[observed("AGENTS.md", Some(edit.content.as_bytes()))],
    ));
    assert_eq!(p.state.owner_contracts.len(), 2);
    assert_eq!(p.state.artifacts[1].owners, s.artifacts[1].owners);
    let records = p
        .state
        .artifacts
        .iter()
        .map(|a| {
            let ArtifactContent::MarkdownContribution { contribution } = &a.content else {
                unreachable!()
            };
            contribution.clone()
        })
        .collect::<Vec<_>>();
    let after = std::str::from_utf8(&p.files[0].after.as_ref().unwrap().bytes).unwrap();
    assert_eq!(
        aggregate::inspect_owned(after, &records).unwrap(),
        vec![
            Contribution::new("neighbor", "Keep\n"),
            Contribution::new("shared", "New\n")
        ]
    );
    assert_eq!(p.state.owner_consumers[2..], s.owner_consumers[2..]);
}
#[test]
fn new_shared_instruction_cannot_adopt_existing_unowned_marker() {
    let s = empty();
    let sets = [instruction("a", "New\n"), instruction("b", "New\n")];
    let c = proofs(&sets);
    let refs = [direct("a"), direct("b")];
    let existing =
        aggregate::reconcile("Manual\n", &[], &[Contribution::new("shared", "New\n")]).unwrap();
    assert!(
        plan_shared_upsert(
            &s,
            request(&sets, &c, None, &refs, &[]),
            &[observed("AGENTS.md", Some(existing.content.as_bytes()))]
        )
        .is_err()
    );
    let p = ready(plan_shared_upsert(
        &s,
        request(&sets, &c, None, &refs, &[]),
        &[observed("AGENTS.md", Some(b"Manual\n"))],
    ));
    assert!(
        p.files[0]
            .after
            .as_ref()
            .unwrap()
            .bytes
            .starts_with(b"Manual\n")
    );
}
#[test]
fn optin_single_owner_plan_matches_original_without_invented_contracts() {
    let s = empty();
    let sets = [set("a", b"old")];
    let refs = [direct("a")];
    let req = request(&sets, &[], None, &refs, &[]);
    let obs = [observed("shared.md", None)];
    assert_eq!(
        plan_shared_upsert(&s, req, &obs).unwrap(),
        plan_upsert(&s, req, &obs).unwrap()
    );
}
#[test]
fn topology_conflict_with_unselected_owned_child_has_no_partial_plan() {
    let mut s = shared();
    s.installations.push(item("c"));
    s.references.push(direct("c"));
    s.artifacts.push(OwnedArtifact {
        root: root(),
        path: "tree/child".into(),
        content: ArtifactContent::File {
            content_sha256: hash(b"keep"),
            mode: Some(0o640),
        },
        owners: vec![item("c").owner],
        consumers: vec![TargetSurface::ClaudeCli],
    });
    s.owner_consumers.push(association("c", "tree/child", None));
    validate_state(&s).unwrap();
    let mut sets = [set("a", b"new"), set("b", b"new")];
    for set in &mut sets {
        let DesiredArtifact::File(f) = &mut set.artifacts[0] else {
            unreachable!()
        };
        f.location.path = "tree".into();
    }
    let c = proofs(&sets);
    let sel = Selection::Names(vec!["a".into(), "b".into()]);
    assert!(
        plan_shared_upsert(
            &s,
            request(&sets, &c, Some(&sel), &[], &[]),
            &[observed("shared.md", Some(b"old")), observed("tree", None)]
        )
        .is_err()
    );
}
#[test]
fn known_legacy_destination_blocks_sharing_without_adoption() {
    let mut s = empty();
    let mut lock = upskill::lockfile::Lockfile::default();
    lock.items.push(upskill::lockfile::LockedItem {
        kind: ItemKind::Rule,
        name: "old".into(),
        source: "./old".into(),
        git_ref: None,
        hash: None,
        source_name: None,
        required_by: vec![],
        group: None,
        clients: vec!["claude".into()],
    });
    s.legacy = Some(lock);
    let mut sets = [set("a", b"new"), set("b", b"new")];
    for set in &mut sets {
        let DesiredArtifact::File(f) = &mut set.artifacts[0] else {
            unreachable!()
        };
        f.location.path = ".claude/rules/old.md".into();
    }
    let c = proofs(&sets);
    let refs = [direct("a"), direct("b")];
    assert!(
        plan_shared_upsert(
            &s,
            request(&sets, &c, None, &refs, &[]),
            &[observed(".claude/rules/old.md", None)]
        )
        .is_err()
    );
}
#[test]
fn last_shared_contribution_retirement_preserves_empty_file() {
    let (mut s, _) = shared_instructions();
    let edit = aggregate::reconcile("", &[], &[Contribution::new("shared", "Old\n")]).unwrap();
    s.artifacts[0].content = ArtifactContent::MarkdownContribution {
        contribution: edit.owned[0].clone(),
    };
    let all = Selection::All;
    let refs = [direct("a"), direct("b")];
    let p = ready(plan_shared_upsert(
        &s,
        request(&[], &[], Some(&all), &[], &refs),
        &[observed("AGENTS.md", Some(edit.content.as_bytes()))],
    ));
    assert_eq!(
        p.files[0].after,
        Some(FileValue {
            bytes: vec![],
            mode: Some(0o640)
        })
    );
}

#[test]
fn instruction_observation_modes_survive_and_edited_regions_reject() {
    let (s, mut obs) = shared_instructions();
    let sets = [instruction("a", "New\n"), instruction("b", "New\n")];
    let c = proofs(&sets);
    let all = Selection::All;
    obs.value.as_mut().unwrap().mode = Some(0o600);
    let p = ready(plan_shared_upsert(
        &s,
        request(&sets, &c, Some(&all), &[], &[]),
        std::slice::from_ref(&obs),
    ));
    assert_eq!(p.files[0].after.as_ref().unwrap().mode, Some(0o600));
    for bytes in [
        b"edited".to_vec(),
        vec![255],
        obs.value
            .as_ref()
            .unwrap()
            .bytes
            .iter()
            .map(|b| if *b == b'O' { b'X' } else { *b })
            .collect(),
    ] {
        let mut bad = obs.clone();
        bad.value.as_mut().unwrap().bytes = bytes;
        assert!(plan_shared_upsert(&s, request(&sets, &c, Some(&all), &[], &[]), &[bad]).is_err());
    }
}

#[test]
fn retained_instruction_release_preserves_body_and_real_evidence() {
    let (mut s, obs) = shared_instructions();
    s.owner_contracts.clear();
    let mut release = instruction("a", "");
    release.artifacts.clear();
    release.audience_excluded = vec![TargetSurface::ClaudeCli];
    let a = Selection::Names(vec!["a".into()]);
    let p = ready(plan_shared_upsert(
        &s,
        request(&[release], &[], Some(&a), &[], &[]),
        std::slice::from_ref(&obs),
    ));
    assert_eq!(ids(&p.state), vec!["b"]);
    assert_eq!(p.files[0].after, obs.value);
    assert!(p.state.owner_contracts.is_empty());
}
