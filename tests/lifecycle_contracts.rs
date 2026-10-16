use sha2::{Digest, Sha256};
use upskill::files::plan::{DesiredFile, FileLocation, FileValue, ObservedFile};
use upskill::lifecycle::contracts::{
    ConsumerContract, ContractAgreement, ContractContent, ContractIssue, OwnerContract,
    check_contract_agreement,
};
use upskill::lifecycle::desired::DesiredInstallationSet;
use upskill::lifecycle::retire::plan_reference_removal;
use upskill::lifecycle::selection::Selection;
use upskill::lifecycle::state::{
    CombinedState, Installation, OwnerConsumers, Reference, SourceEvidence, parse_state,
    project_legacy, serialize_state, validate_state,
};
use upskill::lifecycle::upsert::{UpsertOutcome, UpsertRequest, plan_upsert};
use upskill::operations::plan::DesiredArtifact;
use upskill::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn root() -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: std::env::temp_dir().join("upskill-contract-example"),
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
fn profile(surface: TargetSurface, revision: u32) -> ConsumerContract {
    ConsumerContract {
        surface,
        contract_revision: revision,
    }
}
fn contract(id: &str, path: &str, bytes: &[u8]) -> OwnerContract {
    OwnerContract {
        root: root(),
        path: path.into(),
        contribution_id: None,
        installation_id: id.into(),
        source_manifest_sha256: "a".repeat(64),
        content: ContractContent::File {
            content_sha256: hash(bytes),
            mode: Some(0o640),
        },
        profiles: vec![profile(TargetSurface::ClaudeCli, 1)],
    }
}
fn state() -> CombinedState {
    let installations = vec![item("a"), item("b")];
    CombinedState {
        schema: 2,
        references: vec![direct("a"), direct("b")],
        bundles: vec![],
        artifacts: vec![OwnedArtifact {
            root: root(),
            path: "same.md".into(),
            content: ArtifactContent::File {
                content_sha256: hash(b"body"),
                mode: Some(0o640),
            },
            owners: installations.iter().map(|i| i.owner.clone()).collect(),
            consumers: vec![TargetSurface::ClaudeCli],
        }],
        owner_consumers: ["a", "b"]
            .iter()
            .map(|id| OwnerConsumers {
                root: root(),
                path: "same.md".into(),
                contribution_id: None,
                installation_id: (*id).into(),
                consumers: vec![TargetSurface::ClaudeCli],
            })
            .collect(),
        owner_contracts: vec![],
        installations,
        legacy: None,
    }
}
fn evidenced() -> CombinedState {
    let mut s = state();
    s.owner_contracts = vec![
        contract("a", "same.md", b"body"),
        contract("b", "same.md", b"body"),
    ];
    s
}
fn check(s: &CombinedState) -> ContractAgreement {
    check_contract_agreement(s, &root(), "same.md", None).unwrap()
}
fn single() -> CombinedState {
    let mut s = state();
    s.artifacts.clear();
    s.owner_consumers.clear();
    for id in ["a", "b"] {
        s.artifacts.push(OwnedArtifact {
            root: root(),
            path: format!("{id}.md"),
            content: ArtifactContent::File {
                content_sha256: hash(id.as_bytes()),
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
        s.owner_contracts
            .push(contract(id, &format!("{id}.md"), id.as_bytes()));
    }
    s
}
fn desired(bytes: &[u8]) -> DesiredInstallationSet {
    DesiredInstallationSet {
        installation: item("a"),
        artifacts: vec![DesiredArtifact::File(DesiredFile {
            location: FileLocation {
                root: root(),
                path: "a.md".into(),
            },
            value: FileValue {
                bytes: bytes.to_vec(),
                mode: Some(0o640),
            },
            consumers: vec![TargetSurface::ClaudeCli],
        })],
        audience_excluded: vec![],
    }
}
fn observation(path: &str, bytes: &[u8]) -> ObservedFile {
    ObservedFile {
        location: FileLocation {
            root: root(),
            path: path.into(),
        },
        value: Some(FileValue {
            bytes: bytes.to_vec(),
            mode: Some(0o640),
        }),
    }
}

#[test]
fn absent_historical_evidence_is_not_inferred_or_serialized() {
    let s = state();
    let encoded = serialize_state(&s).unwrap();
    assert!(!encoded.contains("owner_contracts"));
    assert!(parse_state(&encoded).unwrap().owner_contracts.is_empty());
    assert!(
        project_legacy(&Default::default())
            .unwrap()
            .owner_contracts
            .is_empty()
    );
    let ContractAgreement::Blocked(rows) = check(&s) else {
        panic!("missing contracts accepted")
    };
    assert_eq!(
        rows.iter()
            .map(|r| (r.installation_id.as_str(), &r.issue))
            .collect::<Vec<_>>(),
        vec![
            ("a", &ContractIssue::MissingEvidence),
            ("b", &ContractIssue::MissingEvidence)
        ]
    );
}
#[test]
fn matching_explicit_contracts_agree_without_mutating_state() {
    let s = evidenced();
    let before = s.clone();
    validate_state(&s).unwrap();
    assert_eq!(check(&s), ContractAgreement::Matching);
    assert_eq!(s, before);
}
#[test]
fn same_output_does_not_hide_source_or_profile_disagreement() {
    let mut s = evidenced();
    s.installations[1].source_evidence = SourceEvidence::Local {
        manifest_sha256: "b".repeat(64),
    };
    s.owner_contracts[1].source_manifest_sha256 = "b".repeat(64);
    let ContractAgreement::Blocked(rows) = check(&s) else {
        panic!("different source accepted")
    };
    assert_eq!(rows[0].installation_id, "b");
    assert!(matches!(
        rows[0].issue,
        ContractIssue::SourceManifestMismatch { .. }
    ));
    let mut s = evidenced();
    s.owner_contracts[1].profiles[0].contract_revision = 2;
    let ContractAgreement::Blocked(rows) = check(&s) else {
        panic!("different profile accepted")
    };
    assert!(matches!(
        rows[0].issue,
        ContractIssue::TargetProfilesMismatch { .. }
    ));
}
#[test]
fn different_consumer_sets_remain_conservatively_blocked() {
    let mut s = evidenced();
    s.installations[1].requested = vec![TargetSurface::CopilotCli];
    s.owner_consumers[1].consumers = vec![TargetSurface::CopilotCli];
    s.owner_contracts[1].profiles = vec![profile(TargetSurface::CopilotCli, 1)];
    s.artifacts[0].consumers.push(TargetSurface::CopilotCli);
    assert!(matches!(check(&s), ContractAgreement::Blocked(_)));
}
#[test]
fn present_contracts_require_exact_binding_source_content_and_profiles() {
    let original = evidenced();
    let mut variants = vec![];
    for edit in 0..13 {
        let mut s = original.clone();
        let c = &mut s.owner_contracts[0];
        match edit {
            0 => c.root.path = root().path.join("."),
            1 => c.root.scope = OwnershipScope::User,
            2 => c.path = "other.md".into(),
            3 => c.contribution_id = Some("other".into()),
            4 => c.installation_id = "unknown".into(),
            5 => c.source_manifest_sha256 = "b".repeat(64),
            6 => {
                c.content = ContractContent::File {
                    content_sha256: hash(b"other"),
                    mode: Some(0o640),
                }
            }
            7 => {
                c.content = ContractContent::Instruction {
                    content_sha256: hash(b"body"),
                }
            }
            8 => {
                c.content = ContractContent::File {
                    content_sha256: hash(b"body"),
                    mode: Some(0o600),
                }
            }
            9 => c.profiles.clear(),
            10 => c.profiles.push(profile(TargetSurface::ClaudeCli, 1)),
            11 => c.profiles[0].contract_revision = 0,
            _ => c.profiles[0].surface = TargetSurface::CopilotCli,
        };
        variants.push(s);
    }
    let mut s = original.clone();
    s.owner_contracts.push(s.owner_contracts[0].clone());
    variants.push(s);
    let mut s = original;
    s.owner_contracts[0]
        .profiles
        .push(profile(TargetSurface::CopilotCli, 1));
    variants.push(s);
    for s in variants {
        assert!(validate_state(&s).is_err());
        assert!(serialize_state(&s).is_err());
    }
}
#[test]
fn deterministic_codec_sorts_contracts_and_profiles_without_inference() {
    let mut s = evidenced();
    for i in &mut s.installations {
        i.requested.push(TargetSurface::CopilotCli);
    }
    for a in &mut s.owner_consumers {
        a.consumers.push(TargetSurface::CopilotCli);
    }
    s.artifacts[0].consumers.push(TargetSurface::CopilotCli);
    for c in &mut s.owner_contracts {
        c.profiles.insert(0, profile(TargetSurface::CopilotCli, 7));
    }
    s.owner_contracts.reverse();
    let before = s.clone();
    assert_eq!(check(&s), ContractAgreement::Matching);
    let encoded = serialize_state(&s).unwrap();
    let v: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(v["owner_contracts"][0]["installation_id"], "a");
    assert_eq!(
        v["owner_contracts"][0]["profiles"][0]["surface"],
        "claude.cli"
    );
    assert_eq!(
        v["owner_contracts"][0]["profiles"][1]["contract_revision"],
        7
    );
    assert_eq!(s, before);
    assert_eq!(
        serialize_state(&parse_state(&encoded).unwrap()).unwrap(),
        encoded
    );
    let mut bad = v;
    bad["owner_contracts"][0]["invented"] = true.into();
    assert!(parse_state(&bad.to_string()).is_err());
}
#[test]
fn unknown_artifact_or_selector_is_an_error() {
    let s = evidenced();
    assert!(check_contract_agreement(&s, &root(), "absent.md", None).is_err());
    assert!(check_contract_agreement(&s, &root(), "same.md", Some("a")).is_err());
}
#[test]
fn upsert_replaces_explicit_proof_and_preserves_untouched_contracts() {
    let s = single();
    let sets = [desired(b"next")];
    let mut c = contract("a", "a.md", b"next");
    c.profiles[0].contract_revision = 2;
    let contracts = [c.clone()];
    let sel = Selection::Names(vec!["a".into()]);
    let request = UpsertRequest {
        selection: Some(&sel),
        sets: &sets,
        bundles: &[],
        add_references: &[],
        remove_references: &[],
        contracts: &contracts,
    };
    let UpsertOutcome::Ready(p) = plan_upsert(&s, request, &[observation("a.md", b"a")]).unwrap()
    else {
        panic!("unexpected blocker")
    };
    assert_eq!(
        p.state.owner_contracts,
        vec![c, s.owner_contracts[1].clone()]
    );
    let UpsertOutcome::Ready(p) = plan_upsert(
        &s,
        UpsertRequest {
            contracts: &[],
            ..request
        },
        &[observation("a.md", b"a")],
    )
    .unwrap() else {
        panic!("unexpected blocker")
    };
    assert_eq!(p.state.owner_contracts, vec![s.owner_contracts[1].clone()]);
}
#[test]
fn upsert_rejects_unselected_duplicate_or_stale_supplied_proof() {
    let s = single();
    let sets = [desired(b"next")];
    let sel = Selection::Names(vec!["a".into()]);
    for contracts in [
        vec![s.owner_contracts[1].clone()],
        vec![s.owner_contracts[0].clone()],
        vec![
            contract("a", "a.md", b"next"),
            contract("a", "a.md", b"next"),
        ],
    ] {
        let request = UpsertRequest {
            selection: Some(&sel),
            sets: &sets,
            bundles: &[],
            add_references: &[],
            remove_references: &[],
            contracts: &contracts,
        };
        assert!(plan_upsert(&s, request, &[observation("a.md", b"a")]).is_err());
    }
}
#[test]
fn shared_retirement_keeps_survivor_proof_until_final_owner_retires() {
    let s = evidenced();
    let obs = observation("same.md", b"body");
    let p = plan_reference_removal(&s, &[direct("a")], std::slice::from_ref(&obs)).unwrap();
    assert_eq!(p.state.owner_contracts, vec![s.owner_contracts[1].clone()]);
    assert_eq!(check(&p.state), ContractAgreement::Matching);
    let p = plan_reference_removal(&p.state, &[direct("b")], &[obs]).unwrap();
    assert!(p.state.owner_contracts.is_empty());
    assert!(p.state.artifacts.is_empty());
}

#[test]
fn instruction_contract_binds_body_and_selector_without_layout_metadata() {
    use upskill::instructions::aggregate::{self, Contribution};
    let edit =
        aggregate::reconcile("Manual\n", &[], &[Contribution::new("shared", "Shared\n")]).unwrap();
    let mut s = evidenced();
    s.artifacts[0].path = "AGENTS.md".into();
    s.artifacts[0].content = ArtifactContent::MarkdownContribution {
        contribution: edit.owned[0].clone(),
    };
    for row in &mut s.owner_consumers {
        row.path = "AGENTS.md".into();
        row.contribution_id = Some("shared".into());
    }
    for row in &mut s.owner_contracts {
        row.path = "AGENTS.md".into();
        row.contribution_id = Some("shared".into());
        row.content = ContractContent::Instruction {
            content_sha256: edit.owned[0].content_sha256.clone(),
        };
    }
    assert_eq!(
        check_contract_agreement(&s, &root(), "AGENTS.md", Some("shared")).unwrap(),
        ContractAgreement::Matching
    );
    assert!(check_contract_agreement(&s, &root(), "AGENTS.md", None).is_err());
    let relaid = aggregate::reconcile(
        "Manual\n\n\n",
        &[],
        &[Contribution::new("shared", "Shared\n")],
    )
    .unwrap();
    assert_ne!(
        relaid.owned[0].preceding_newlines,
        edit.owned[0].preceding_newlines
    );
    s.artifacts[0].content = ArtifactContent::MarkdownContribution {
        contribution: relaid.owned[0].clone(),
    };
    assert_eq!(
        check_contract_agreement(&s, &root(), "AGENTS.md", Some("shared")).unwrap(),
        ContractAgreement::Matching
    );
    let observed = observation("AGENTS.md", relaid.content.as_bytes());
    let first =
        plan_reference_removal(&s, &[direct("a")], std::slice::from_ref(&observed)).unwrap();
    assert_eq!(
        first.state.owner_contracts,
        vec![s.owner_contracts[1].clone()]
    );
    let last = plan_reference_removal(&first.state, &[direct("b")], &[observed]).unwrap();
    assert!(last.state.owner_contracts.is_empty());
    assert_eq!(
        last.files[0].after,
        Some(FileValue {
            bytes: b"Manual\n\n\n".to_vec(),
            mode: Some(0o640)
        })
    );
    let mut changed = s;
    changed.owner_contracts[0].content = ContractContent::Instruction {
        content_sha256: hash(b"different"),
    };
    assert!(validate_state(&changed).is_err());
}

#[test]
fn empty_selected_outputs_clear_proof_without_retiring_referenced_owner() {
    let s = single();
    let mut set = desired(b"unused");
    set.artifacts.clear();
    set.audience_excluded = vec![TargetSurface::ClaudeCli];
    let sets = [set];
    let sel = Selection::Names(vec!["a".into()]);
    let request = UpsertRequest {
        selection: Some(&sel),
        sets: &sets,
        bundles: &[],
        add_references: &[],
        remove_references: &[],
        contracts: &[],
    };
    let UpsertOutcome::Ready(p) = plan_upsert(&s, request, &[observation("a.md", b"a")]).unwrap()
    else {
        panic!("unexpected blocker")
    };
    assert_eq!(p.state.owner_contracts, vec![s.owner_contracts[1].clone()]);
    assert!(
        p.state
            .installations
            .iter()
            .any(|i| i.owner.installation_id == "a")
    );
    assert!(!p.state.artifacts.iter().any(|a| a.path == "a.md"));
}

#[test]
fn profile_order_is_irrelevant_and_all_disagreements_are_reported() {
    let mut s = evidenced();
    for i in &mut s.installations {
        i.requested.push(TargetSurface::CopilotCli);
    }
    for a in &mut s.owner_consumers {
        a.consumers.push(TargetSurface::CopilotCli);
    }
    s.artifacts[0].consumers.push(TargetSurface::CopilotCli);
    for c in &mut s.owner_contracts {
        c.profiles.push(profile(TargetSurface::CopilotCli, 7));
    }
    s.owner_contracts[1].profiles.reverse();
    assert_eq!(check(&s), ContractAgreement::Matching);
    s.installations[1].source_evidence = SourceEvidence::Local {
        manifest_sha256: "b".repeat(64),
    };
    s.owner_contracts[1].source_manifest_sha256 = "b".repeat(64);
    s.owner_contracts[1].profiles[0].contract_revision = 8;
    let ContractAgreement::Blocked(rows) = check(&s) else {
        panic!("disagreements accepted")
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].installation_id, "b");
    assert_eq!(
        rows[0].issue,
        ContractIssue::SourceManifestMismatch {
            reference_installation_id: "a".into()
        }
    );
    assert_eq!(
        rows[1].issue,
        ContractIssue::TargetProfilesMismatch {
            reference_installation_id: "a".into()
        }
    );
}

#[test]
fn missing_later_owner_blocks_even_after_recorded_baseline() {
    let mut s = evidenced();
    s.owner_contracts.truncate(1);
    assert_eq!(
        check(&s),
        ContractAgreement::Blocked(vec![upskill::lifecycle::contracts::ContractConflict {
            installation_id: "b".into(),
            issue: ContractIssue::MissingEvidence
        }])
    );
}

#[test]
fn non_rule_owners_remain_representable_without_contract_evidence() {
    for kind in [ItemKind::Skill, ItemKind::Agent] {
        let mut s = evidenced();
        s.installations[0].owner.kind = kind;
        s.artifacts[0].owners[0].kind = kind;
        assert!(validate_state(&s).is_err());
        s.owner_contracts.remove(0);
        validate_state(&s).unwrap();
    }
}

#[test]
fn replacement_preserves_prior_contract_positions() {
    let mut s = single();
    s.owner_contracts.reverse();
    let sets = [desired(b"next")];
    let sel = Selection::Names(vec!["a".into()]);
    let c = contract("a", "a.md", b"next");
    let contracts = [c.clone()];
    let request = UpsertRequest {
        selection: Some(&sel),
        sets: &sets,
        bundles: &[],
        add_references: &[],
        remove_references: &[],
        contracts: &contracts,
    };
    let UpsertOutcome::Ready(p) = plan_upsert(&s, request, &[observation("a.md", b"a")]).unwrap()
    else {
        panic!("unexpected blocker")
    };
    assert_eq!(
        p.state.owner_contracts,
        vec![s.owner_contracts[0].clone(), c]
    );
}

#[test]
fn unchanged_owner_cannot_gain_previously_absent_proof() {
    let mut s = single();
    let b = s.owner_contracts.pop().unwrap();
    let sets = [desired(b"a")];
    let sel = Selection::Names(vec!["a".into()]);
    let contracts = [b];
    let request = UpsertRequest {
        selection: Some(&sel),
        sets: &sets,
        bundles: &[],
        add_references: &[],
        remove_references: &[],
        contracts: &contracts,
    };
    assert!(plan_upsert(&s, request, &[observation("a.md", b"a")]).is_err());
}

#[test]
fn no_op_selected_update_clears_omitted_proof() {
    let s = single();
    let sets = [desired(b"a")];
    let sel = Selection::Names(vec!["a".into()]);
    let request = UpsertRequest {
        selection: Some(&sel),
        sets: &sets,
        bundles: &[],
        add_references: &[],
        remove_references: &[],
        contracts: &[],
    };
    let UpsertOutcome::Ready(p) = plan_upsert(&s, request, &[observation("a.md", b"a")]).unwrap()
    else {
        panic!("unexpected blocker")
    };
    assert_eq!(p.state.owner_contracts, vec![s.owner_contracts[1].clone()]);
    assert_eq!(p.state.artifacts, s.artifacts);
}

#[test]
fn agreement_reports_recorded_owner_order_and_validates_whole_state() {
    let mut s = evidenced();
    s.artifacts[0].owners.reverse();
    s.owner_contracts.clear();
    let ContractAgreement::Blocked(rows) = check(&s) else {
        panic!("missing proof accepted")
    };
    assert_eq!(
        rows.iter()
            .map(|r| r.installation_id.as_str())
            .collect::<Vec<_>>(),
        vec!["b", "a"]
    );
    s = evidenced();
    s.artifacts[0].owners.reverse();
    s.owner_contracts[0].profiles[0].contract_revision = 2;
    assert_eq!(
        check(&s),
        ContractAgreement::Blocked(vec![upskill::lifecycle::contracts::ContractConflict {
            installation_id: "a".into(),
            issue: ContractIssue::TargetProfilesMismatch {
                reference_installation_id: "b".into()
            }
        }])
    );
    s.owner_contracts[0].content = ContractContent::File {
        content_sha256: hash(b"invalid"),
        mode: Some(0o640),
    };
    assert!(check_contract_agreement(&s, &root(), "same.md", None).is_err());
    s = evidenced();
    s.references.push(direct("unknown"));
    assert!(check_contract_agreement(&s, &root(), "same.md", None).is_err());
}

#[test]
fn agreement_query_requires_exact_scope_and_raw_root() {
    let s = evidenced();
    let mut wrong = root();
    wrong.scope = OwnershipScope::User;
    assert!(check_contract_agreement(&s, &wrong, "same.md", None).is_err());
    wrong = root();
    wrong.path = wrong.path.join(".");
    assert!(check_contract_agreement(&s, &wrong, "same.md", None).is_err());
}

#[test]
fn previously_absent_evidence_appends_in_full_identity_order() {
    let mut s = single();
    s.owner_contracts.remove(0);
    let mut set = desired(b"a");
    let DesiredArtifact::File(template) = set.artifacts[0].clone() else {
        unreachable!()
    };
    for path in ["z.md", "c.md"] {
        let mut file = template.clone();
        file.location.path = path.into();
        set.artifacts.push(DesiredArtifact::File(file));
    }
    let sets = [set];
    let contracts = [
        contract("a", "z.md", b"a"),
        contract("a", "c.md", b"a"),
        contract("a", "a.md", b"a"),
    ];
    let sel = Selection::Names(vec!["a".into()]);
    let request = UpsertRequest {
        selection: Some(&sel),
        sets: &sets,
        bundles: &[],
        add_references: &[],
        remove_references: &[],
        contracts: &contracts,
    };
    let observations = [
        observation("a.md", b"a"),
        ObservedFile {
            location: FileLocation {
                root: root(),
                path: "z.md".into(),
            },
            value: None,
        },
        ObservedFile {
            location: FileLocation {
                root: root(),
                path: "c.md".into(),
            },
            value: None,
        },
    ];
    let UpsertOutcome::Ready(p) = plan_upsert(&s, request, &observations).unwrap() else {
        panic!("unexpected blocker")
    };
    assert_eq!(
        p.state.owner_contracts,
        vec![
            s.owner_contracts[0].clone(),
            contracts[2].clone(),
            contracts[1].clone(),
            contracts[0].clone()
        ]
    );
}

#[test]
fn graph_retirement_clears_proof_without_a_desired_set() {
    let s = single();
    let sel = Selection::Names(vec!["a".into()]);
    let remove = [direct("a")];
    let request = UpsertRequest {
        selection: Some(&sel),
        sets: &[],
        bundles: &[],
        add_references: &[],
        remove_references: &remove,
        contracts: &[],
    };
    let UpsertOutcome::Ready(p) = plan_upsert(&s, request, &[observation("a.md", b"a")]).unwrap()
    else {
        panic!("unexpected blocker")
    };
    assert_eq!(p.state.owner_contracts, vec![s.owner_contracts[1].clone()]);
}

#[test]
fn nested_contract_fields_reject_unknown_keys() {
    let json = serialize_state(&evidenced()).unwrap();
    for profile in [false, true] {
        let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
        if profile {
            v["owner_contracts"][0]["profiles"][0]["unknown"] = true.into();
        } else {
            v["owner_contracts"][0]["content"]["unknown"] = true.into();
        }
        assert!(parse_state(&v.to_string()).is_err());
    }
}

#[test]
fn canonical_contract_order_uses_path_before_owner() {
    let mut s = single();
    s.artifacts[0].path = "z.md".into();
    s.owner_consumers[0].path = "z.md".into();
    s.owner_contracts[0].path = "z.md".into();
    let v: serde_json::Value = serde_json::from_str(&serialize_state(&s).unwrap()).unwrap();
    assert_eq!(v["owner_contracts"][0]["installation_id"], "b");
    assert_eq!(v["owner_contracts"][0]["path"], "b.md");
}

#[test]
fn codec_preserves_every_contract_and_profile() {
    let mut s = evidenced();
    for i in &mut s.installations {
        i.requested.push(TargetSurface::CopilotCli);
    }
    for a in &mut s.owner_consumers {
        a.consumers.push(TargetSurface::CopilotCli);
    }
    s.artifacts[0].consumers.push(TargetSurface::CopilotCli);
    for c in &mut s.owner_contracts {
        c.profiles.push(profile(TargetSurface::CopilotCli, 9));
    }
    s.owner_contracts[1].profiles[1].contract_revision = 17;
    s.installations[1].source_evidence = SourceEvidence::Local {
        manifest_sha256: "b".repeat(64),
    };
    s.owner_contracts[1].source_manifest_sha256 = "b".repeat(64);
    let expected_agreement = check(&s);
    assert!(matches!(expected_agreement, ContractAgreement::Blocked(_)));
    let decoded = parse_state(&serialize_state(&s).unwrap()).unwrap();
    assert_eq!(decoded.owner_contracts, s.owner_contracts);
    assert_eq!(check(&decoded), expected_agreement);
}

#[test]
fn git_capture_digest_binds_contract_without_implying_authenticity() {
    let mut s = evidenced();
    let revision = "1".repeat(40);
    s.installations[0].owner.source_revision = Some(revision.clone());
    s.installations[0].source_evidence = SourceEvidence::Git {
        resolved_revision: revision,
        manifest_sha256: "a".repeat(64),
    };
    s.artifacts[0].owners[0] = s.installations[0].owner.clone();
    assert_eq!(check(&s), ContractAgreement::Matching);
    s.owner_contracts[0].source_manifest_sha256 = "b".repeat(64);
    assert!(check_contract_agreement(&s, &root(), "same.md", None).is_err());
}

#[test]
fn missing_first_owner_does_not_hide_later_source_and_profile_conflicts() {
    let mut s = evidenced();
    s.installations.push(item("c"));
    s.references.push(direct("c"));
    s.artifacts[0].owners.insert(0, item("c").owner);
    let mut row = s.owner_consumers[0].clone();
    row.installation_id = "c".into();
    s.owner_consumers.push(row);
    s.installations[1].source_evidence = SourceEvidence::Local {
        manifest_sha256: "b".repeat(64),
    };
    s.owner_contracts[1].source_manifest_sha256 = "b".repeat(64);
    s.owner_contracts[1].profiles[0].contract_revision = 2;
    let ContractAgreement::Blocked(rows) = check(&s) else {
        panic!("partial diagnostics accepted")
    };
    assert_eq!(
        rows,
        vec![
            upskill::lifecycle::contracts::ContractConflict {
                installation_id: "c".into(),
                issue: ContractIssue::MissingEvidence
            },
            upskill::lifecycle::contracts::ContractConflict {
                installation_id: "b".into(),
                issue: ContractIssue::SourceManifestMismatch {
                    reference_installation_id: "a".into()
                }
            },
            upskill::lifecycle::contracts::ContractConflict {
                installation_id: "b".into(),
                issue: ContractIssue::TargetProfilesMismatch {
                    reference_installation_id: "a".into()
                }
            },
        ]
    );
}

#[test]
fn agreement_rejects_invalid_evidence_on_unrelated_artifact() {
    let mut s = single();
    s.owner_contracts[1].content = ContractContent::File {
        content_sha256: hash(b"invalid"),
        mode: Some(0o640),
    };
    assert!(check_contract_agreement(&s, &root(), "a.md", None).is_err());
}

#[test]
fn instruction_contracts_roundtrip_completely_and_reject_unknown_content() {
    use upskill::instructions::aggregate::{self, Contribution};
    let edit = aggregate::reconcile("", &[], &[Contribution::new("shared", "Shared\n")]).unwrap();
    let mut s = evidenced();
    s.artifacts[0].path = "AGENTS.md".into();
    s.artifacts[0].content = ArtifactContent::MarkdownContribution {
        contribution: edit.owned[0].clone(),
    };
    for a in &mut s.owner_consumers {
        a.path = "AGENTS.md".into();
        a.contribution_id = Some("shared".into());
    }
    for c in &mut s.owner_contracts {
        c.path = "AGENTS.md".into();
        c.contribution_id = Some("shared".into());
        c.content = ContractContent::Instruction {
            content_sha256: edit.owned[0].content_sha256.clone(),
        };
    }
    let encoded = serialize_state(&s).unwrap();
    assert_eq!(
        parse_state(&encoded).unwrap().owner_contracts,
        s.owner_contracts
    );
    let mut v: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    v["owner_contracts"][0]["content"]["unknown"] = true.into();
    assert!(parse_state(&v.to_string()).is_err());
}

#[test]
fn canonical_contract_order_covers_scope_root_path_selector_and_owner() {
    use upskill::instructions::aggregate::OwnedContribution;
    let mut s = state();
    s.installations.clear();
    s.references.clear();
    s.artifacts.clear();
    s.owner_consumers.clear();
    let cases = [
        ("a", OwnershipScope::User, "a", "A.md", None),
        ("b", OwnershipScope::Project, "z", "A.md", None),
        ("c", OwnershipScope::Project, "a", "Z.md", None),
        ("d", OwnershipScope::Project, "a", "A.md", Some("z")),
        ("e", OwnershipScope::Project, "a", "A.md", Some("a")),
        ("f", OwnershipScope::Project, "a", "A.md", Some("a")),
    ];
    for (id, scope, suffix, path, selector) in cases {
        let location = InstallationRoot {
            scope,
            path: root().path.join(suffix),
        };
        let mut i = item(id);
        i.root = location.clone();
        let content = match selector {
            None => ArtifactContent::File {
                content_sha256: hash(b"body"),
                mode: Some(0o640),
            },
            Some(id) => ArtifactContent::MarkdownContribution {
                contribution: OwnedContribution {
                    id: id.into(),
                    content_sha256: hash(b"body"),
                    leading_newlines: 0,
                    preceding_newlines: 0,
                },
            },
        };
        if let Some(a) = s
            .artifacts
            .iter_mut()
            .find(|a| a.root == location && a.path == path && a.content == content)
        {
            a.owners.push(i.owner.clone());
        } else {
            s.artifacts.push(OwnedArtifact {
                root: location.clone(),
                path: path.into(),
                content,
                owners: vec![i.owner.clone()],
                consumers: vec![TargetSurface::ClaudeCli],
            });
        }
        s.owner_consumers.push(OwnerConsumers {
            root: location.clone(),
            path: path.into(),
            contribution_id: selector.map(str::to_owned),
            installation_id: id.into(),
            consumers: vec![TargetSurface::ClaudeCli],
        });
        let mut c = contract(id, path, b"body");
        c.root = location;
        c.contribution_id = selector.map(str::to_owned);
        if selector.is_some() {
            c.content = ContractContent::Instruction {
                content_sha256: hash(b"body"),
            };
        }
        s.owner_contracts.push(c);
        s.references.push(direct(id));
        s.installations.push(i);
    }
    s.owner_contracts.reverse();
    let encoded = serialize_state(&s).unwrap();
    let decoded = parse_state(&encoded).unwrap();
    assert_eq!(
        decoded
            .owner_contracts
            .iter()
            .map(|c| c.installation_id.as_str())
            .collect::<Vec<_>>(),
        vec!["e", "f", "d", "c", "b", "a"]
    );
}

#[test]
fn removal_preserves_noncanonical_survivor_and_replacement_order() {
    let mut s = single();
    let c = item("c");
    s.installations.push(c.clone());
    s.references.push(direct("c"));
    s.artifacts.push(OwnedArtifact {
        root: root(),
        path: "c.md".into(),
        content: ArtifactContent::File {
            content_sha256: hash(b"c"),
            mode: Some(0o640),
        },
        owners: vec![c.owner],
        consumers: vec![TargetSurface::ClaudeCli],
    });
    s.owner_consumers.push(OwnerConsumers {
        root: root(),
        path: "c.md".into(),
        contribution_id: None,
        installation_id: "c".into(),
        consumers: vec![TargetSurface::ClaudeCli],
    });
    s.owner_contracts = vec![
        contract("c", "c.md", b"c"),
        s.owner_contracts[1].clone(),
        s.owner_contracts[0].clone(),
    ];
    let sel = Selection::Names(vec!["a".into(), "c".into()]);
    let sets = [desired(b"next")];
    let contracts = [contract("a", "a.md", b"next")];
    let remove = [direct("c")];
    let request = UpsertRequest {
        selection: Some(&sel),
        sets: &sets,
        bundles: &[],
        add_references: &[],
        remove_references: &remove,
        contracts: &contracts,
    };
    let UpsertOutcome::Ready(p) = plan_upsert(
        &s,
        request,
        &[observation("a.md", b"a"), observation("c.md", b"c")],
    )
    .unwrap() else {
        panic!("unexpected blocker")
    };
    assert_eq!(
        p.state.owner_contracts,
        vec![s.owner_contracts[1].clone(), contracts[0].clone()]
    );
}
