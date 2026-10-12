//! Public acceptance tests for pure owned instruction planning.

use upskill::instructions::aggregate::{self, Contribution};
use upskill::instructions::plan::*;
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn location(path: &str) -> InstructionLocation {
    InstructionLocation {
        root: InstallationRoot {
            scope: OwnershipScope::Project,
            path: if cfg!(windows) {
                "C:/workspace"
            } else {
                "/workspace"
            }
            .into(),
        },
        path: path.into(),
    }
}
fn owner(id: &str) -> ArtifactOwner {
    ArtifactOwner {
        installation_id: id.into(),
        source: "local:./registry".into(),
        source_revision: None,
        kind: ItemKind::Rule,
        source_name: id.into(),
        installed_name: id.into(),
    }
}
fn set(id: &str, markdown: &str) -> InstructionInstallationChange {
    InstructionInstallationChange::Set {
        owner: owner(id),
        contributions: vec![DesiredInstructionContribution {
            location: location("AGENTS.md"),
            id: id.into(),
            markdown: markdown.into(),
            consumers: vec![TargetSurface::CodexCli],
        }],
    }
}
#[test]
fn separate_owner_removal_preserves_manual_bytes_and_later_metadata() {
    let manual = b"Manual instructions without final newline".to_vec();
    let first = plan_instructions(
        &[],
        &[set("a", "A\n"), set("b", "B\n")],
        &[ObservedInstructionFile {
            location: location("AGENTS.md"),
            content: Some(manual.clone()),
        }],
    )
    .unwrap();
    assert_eq!(first.files.len(), 1, "initial add must plan one file");
    let second = plan_instructions(
        &first.artifacts,
        &[InstructionInstallationChange::Remove {
            installation_id: "a".into(),
        }],
        &[ObservedInstructionFile {
            location: location("AGENTS.md"),
            content: Some(first.files[0].after.clone()),
        }],
    )
    .unwrap();
    assert_eq!(second.artifacts.len(), 1);
    assert_eq!(second.artifacts[0].owners, vec![owner("b")]);
    assert_eq!(second.files[0].after, b"Manual instructions without final newline\n<!-- upskill:begin b -->\nB\n<!-- upskill:end b -->\n");
    let third = plan_instructions(
        &second.artifacts,
        &[InstructionInstallationChange::Remove {
            installation_id: "b".into(),
        }],
        &[ObservedInstructionFile {
            location: location("AGENTS.md"),
            content: Some(second.files[0].after.clone()),
        }],
    )
    .unwrap();
    assert_eq!(third.files[0].after, manual);
}
#[test]
fn no_op_update_keeps_exact_before_state() {
    let edit = aggregate::reconcile("", &[], &[Contribution::new("a", "A\n")]).unwrap();
    let prior = vec![OwnedArtifact {
        root: location("AGENTS.md").root,
        path: "AGENTS.md".into(),
        content: ArtifactContent::MarkdownContribution {
            contribution: edit.owned[0].clone(),
        },
        owners: vec![owner("a")],
        consumers: vec![TargetSurface::CodexCli],
    }];
    let before = edit.content.into_bytes();
    let second = plan_instructions(
        &prior,
        &[set("a", "A\n")],
        &[ObservedInstructionFile {
            location: location("AGENTS.md"),
            content: Some(before.clone()),
        }],
    )
    .unwrap();
    assert_eq!(second.files.len(), 1);
    assert_eq!(second.files[0].before, Some(before.clone()));
    assert_eq!(second.files[0].after, before);
}
#[test]
fn verified_reader_preserves_document_order() {
    let edit = aggregate::reconcile(
        "manual",
        &[],
        &[Contribution::new("a", "A\n"), Contribution::new("b", "B\n")],
    )
    .unwrap();
    let mut records = edit.owned.clone();
    records.reverse();
    assert_eq!(
        aggregate::inspect_owned(&edit.content, &records).unwrap(),
        vec![Contribution::new("a", "A\n"), Contribution::new("b", "B\n")]
    );
}

use upskill::ownership::{ArtifactContent, OwnedArtifact};

fn observe(path: &str, content: Option<Vec<u8>>) -> ObservedInstructionFile {
    ObservedInstructionFile {
        location: location(path),
        content,
    }
}
fn remove(id: &str) -> InstructionInstallationChange {
    InstructionInstallationChange::Remove {
        installation_id: id.into(),
    }
}
fn desired(path: &str, id: &str, markdown: &str) -> DesiredInstructionContribution {
    DesiredInstructionContribution {
        location: location(path),
        id: id.into(),
        markdown: markdown.into(),
        consumers: vec![TargetSurface::CodexCli],
    }
}
fn fixture() -> (Vec<OwnedArtifact>, Vec<u8>) {
    let edit = aggregate::reconcile(
        "Manual",
        &[],
        &[Contribution::new("a", "A\n"), Contribution::new("b", "B\n")],
    )
    .unwrap();
    let records = edit
        .owned
        .into_iter()
        .map(|contribution| OwnedArtifact {
            root: location("AGENTS.md").root,
            path: "AGENTS.md".into(),
            owners: vec![owner(&contribution.id)],
            consumers: vec![TargetSurface::CodexCli],
            content: ArtifactContent::MarkdownContribution { contribution },
        })
        .collect();
    (records, edit.content.into_bytes())
}
fn whole_file(path: &str, id: &str) -> OwnedArtifact {
    OwnedArtifact {
        root: location(path).root,
        path: path.into(),
        content: ArtifactContent::File {
            content_sha256: "0".repeat(64),
            mode: None,
        },
        owners: vec![owner(id)],
        consumers: vec![TargetSurface::CodexCli],
    }
}
fn error(
    prior: &[OwnedArtifact],
    changes: &[InstructionInstallationChange],
    observations: &[ObservedInstructionFile],
) -> String {
    format!(
        "{:#}",
        plan_instructions(prior, changes, observations).unwrap_err()
    )
}

#[test]
fn updating_one_owner_preserves_manual_prefix_suffix_and_other_metadata() {
    let (prior, mut bytes) = fixture();
    bytes.extend_from_slice(b"\nTail\r\n");
    let result = plan_instructions(
        &prior,
        &[set("a", "New A\n")],
        &[observe("AGENTS.md", Some(bytes.clone()))],
    )
    .unwrap();
    assert_eq!(result.files[0].before, Some(bytes));
    assert_eq!(result.files[0].after, b"Manual\n\n<!-- upskill:begin a -->\nNew A\n<!-- upskill:end a -->\n\n<!-- upskill:begin b -->\nB\n<!-- upskill:end b -->\n\nTail\r\n");
    assert_eq!(result.artifacts[1], prior[1]);
    let ArtifactContent::MarkdownContribution { contribution } = &result.artifacts[0].content
    else {
        panic!("updated region must retain contribution metadata")
    };
    assert_eq!(
        contribution.content_sha256,
        "4f20c82a5fc868b914151c1e0ee4b67a2b290c3f902c835690264f0cdfaca476"
    );
    let removed = plan_instructions(
        &result.artifacts,
        &[remove("a")],
        &[observe("AGENTS.md", Some(result.files[0].after.clone()))],
    )
    .unwrap();
    assert_eq!(removed.artifacts.len(), 1);
    assert_eq!(removed.artifacts[0].owners, vec![owner("b")]);
    assert_eq!(
        removed.files[0].after,
        b"Manual\n<!-- upskill:begin b -->\nB\n<!-- upskill:end b -->\n\nTail\r\n"
    );
}

#[test]
fn complete_set_omits_and_moves_old_regions_and_preserves_other_owners() {
    let (mut prior, bytes) = fixture();
    let old = aggregate::reconcile("Old", &[], &[Contribution::new("omitted", "Gone\n")]).unwrap();
    prior.push(OwnedArtifact {
        root: location("OLD.md").root,
        path: "OLD.md".into(),
        content: ArtifactContent::MarkdownContribution {
            contribution: old.owned[0].clone(),
        },
        owners: vec![owner("a")],
        consumers: vec![TargetSurface::CodexCli],
    });
    let changes = [InstructionInstallationChange::Set {
        owner: owner("a"),
        contributions: vec![desired("NEW.md", "a", "Moved\n")],
    }];
    let result = plan_instructions(
        &prior,
        &changes,
        &[
            observe("OLD.md", Some(old.content.into_bytes())),
            observe("NEW.md", None),
            observe("AGENTS.md", Some(bytes.clone())),
        ],
    )
    .unwrap();
    assert_eq!(result.artifacts.len(), 2);
    assert_eq!(result.artifacts[0].owners, vec![owner("b")]);
    assert_eq!(result.artifacts[1].path, "NEW.md");
    assert_eq!(
        result
            .files
            .iter()
            .map(|f| f.location.path.as_str())
            .collect::<Vec<_>>(),
        ["AGENTS.md", "NEW.md", "OLD.md"]
    );
    assert_eq!(result.files[0].before, Some(bytes));
    assert_eq!(
        result.files[0].after,
        b"Manual\n<!-- upskill:begin b -->\nB\n<!-- upskill:end b -->\n"
    );
    assert_eq!(result.files[1].before, None);
    assert_eq!(
        result.files[1].after,
        b"<!-- upskill:begin a -->\nMoved\n<!-- upskill:end a -->\n"
    );
    assert_eq!(
        result.files[2].before.as_deref(),
        Some(
            b"Old\n\n<!-- upskill:begin omitted -->\nGone\n<!-- upskill:end omitted -->\n"
                .as_slice()
        )
    );
    assert_eq!(result.files[2].after, b"Old");
}

#[test]
fn last_removal_retains_empty_aggregate() {
    let first = plan_instructions(&[], &[set("a", "A\n")], &[observe("AGENTS.md", None)]).unwrap();
    let result = plan_instructions(
        &first.artifacts,
        &[remove("a")],
        &[observe("AGENTS.md", Some(first.files[0].after.clone()))],
    )
    .unwrap();
    assert!(result.artifacts.is_empty());
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].after, b"");
}

#[test]
fn revision_replacement_updates_all_owner_bindings() {
    let (mut prior, bytes) = fixture();
    let extra = aggregate::reconcile("", &[], &[Contribution::new("extra", "Extra\n")]).unwrap();
    prior.push(OwnedArtifact {
        root: location("EXTRA.md").root,
        path: "EXTRA.md".into(),
        content: ArtifactContent::MarkdownContribution {
            contribution: extra.owned[0].clone(),
        },
        owners: vec![owner("a")],
        consumers: vec![TargetSurface::CodexCli],
    });
    let mut changed = owner("a");
    changed.source_revision = Some("v2".into());
    let result = plan_instructions(
        &prior,
        &[InstructionInstallationChange::Set {
            owner: changed.clone(),
            contributions: vec![
                desired("AGENTS.md", "a", "A\n"),
                desired("EXTRA.md", "extra", "Extra\n"),
            ],
        }],
        &[
            observe("AGENTS.md", Some(bytes)),
            observe("EXTRA.md", Some(extra.content.into_bytes())),
        ],
    )
    .unwrap();
    assert_eq!(result.artifacts[0].owners, vec![changed.clone()]);
    assert_eq!(result.artifacts[2].owners, vec![changed]);
}

#[test]
fn creation_preserves_distinct_source_name_and_installed_alias() {
    let binding = ArtifactOwner {
        source_name: "source-rule".into(),
        installed_name: "installed-alias".into(),
        source_revision: Some("v1".into()),
        ..owner("installation")
    };
    let plan = plan_instructions(
        &[],
        &[InstructionInstallationChange::Set {
            owner: binding.clone(),
            contributions: vec![desired("AGENTS.md", "region", "Initial\n")],
        }],
        &[observe("AGENTS.md", None)],
    )
    .unwrap();
    assert_eq!(plan.artifacts.len(), 1);
    assert_eq!(plan.artifacts[0].owners, vec![binding]);
    assert_eq!(
        plan.files[0].after,
        b"<!-- upskill:begin region -->\nInitial\n<!-- upskill:end region -->\n"
    );
}

#[test]
fn revision_replacement_preserves_distinct_source_name_and_installed_alias() {
    let binding = ArtifactOwner {
        source_name: "source-rule".into(),
        installed_name: "installed-alias".into(),
        source_revision: Some("v1".into()),
        ..owner("installation")
    };
    let edit = aggregate::reconcile("", &[], &[Contribution::new("region", "Initial\n")]).unwrap();
    let prior = vec![OwnedArtifact {
        root: location("AGENTS.md").root,
        path: "AGENTS.md".into(),
        content: ArtifactContent::MarkdownContribution {
            contribution: edit.owned[0].clone(),
        },
        owners: vec![binding.clone()],
        consumers: vec![TargetSurface::CodexCli],
    }];
    let replacement = ArtifactOwner {
        source_revision: Some("v2".into()),
        ..binding
    };
    let plan = plan_instructions(
        &prior,
        &[InstructionInstallationChange::Set {
            owner: replacement.clone(),
            contributions: vec![desired("AGENTS.md", "region", "Replacement\n")],
        }],
        &[observe("AGENTS.md", Some(edit.content.into_bytes()))],
    )
    .unwrap();
    assert_eq!(plan.artifacts.len(), 1);
    assert_eq!(plan.artifacts[0].owners, vec![replacement]);
    assert_eq!(
        plan.files[0].after,
        b"<!-- upskill:begin region -->\nReplacement\n<!-- upskill:end region -->\n"
    );
    assert_eq!(prior[0].owners[0].source_revision.as_deref(), Some("v1"));
}

#[test]
fn existing_binding_cannot_be_reinterpreted() {
    let (prior, bytes) = fixture();
    for field in 0..4 {
        let mut changed = owner("a");
        match field {
            0 => changed.source = "local:./other".into(),
            1 => changed.kind = ItemKind::Skill,
            2 => changed.source_name = "other".into(),
            _ => changed.installed_name = "alias".into(),
        }
        let changes = [InstructionInstallationChange::Set {
            owner: changed,
            contributions: vec![desired("AGENTS.md", "a", "A\n")],
        }];
        assert!(
            error(
                &prior,
                &changes,
                &[observe("AGENTS.md", Some(bytes.clone()))]
            )
            .contains("a")
        );
    }
}

#[test]
fn untouched_selector_cannot_be_taken_over_even_with_identical_body() {
    let (prior, bytes) = fixture();
    for body in ["B\n", "Replacement\n"] {
        let change = InstructionInstallationChange::Set {
            owner: owner("a"),
            contributions: vec![desired("AGENTS.md", "b", body)],
        };
        assert!(
            error(
                &prior,
                &[change],
                &[observe("AGENTS.md", Some(bytes.clone()))]
            )
            .contains("b")
        );
    }
}

#[test]
fn setting_first_shared_owner_rejects_multi_owner_region() {
    assert_shared_owner_change_rejected(set("a", "Replacement\n"));
}

#[test]
fn removing_first_shared_owner_rejects_multi_owner_region() {
    assert_shared_owner_change_rejected(remove("a"));
}

#[test]
fn setting_later_shared_owner_rejects_multi_owner_region() {
    assert_shared_owner_change_rejected(set("c", "Replacement\n"));
}

#[test]
fn removing_later_shared_owner_rejects_multi_owner_region() {
    assert_shared_owner_change_rejected(remove("c"));
}

fn assert_shared_owner_change_rejected(change: InstructionInstallationChange) {
    let (mut prior, bytes) = fixture();
    prior[0].owners.push(owner("c"));
    let snapshot = prior.clone();
    assert_rejected_with(
        plan_instructions(&prior, &[change], &[observe("AGENTS.md", Some(bytes))]),
        "multi-owner instruction region",
    );
    assert_eq!(prior, snapshot);
}

#[test]
fn affected_whole_file_and_shared_region_conflict_without_mutation() {
    let (mut shared, bytes) = fixture();
    shared[0].owners.push(owner("c"));
    let snapshot = shared.clone();
    assert!(
        error(
            &shared,
            &[remove("a")],
            &[observe("AGENTS.md", Some(bytes.clone()))]
        )
        .contains("a")
    );
    assert_eq!(shared, snapshot);
    let file = vec![whole_file("AGENTS.md", "a")];
    assert!(error(&file, &[remove("a")], &[observe("AGENTS.md", None)]).contains("AGENTS.md"));
    assert!(
        error(
            &[whole_file("AGENTS.md", "z")],
            &[set("a", "A\n")],
            &[observe("AGENTS.md", None)]
        )
        .contains("AGENTS.md")
    );
}

#[test]
fn unrelated_whole_file_and_shared_regions_survive() {
    let (mut prior, bytes) = fixture();
    prior[1].owners.push(owner("c"));
    prior[1].consumers.push(TargetSurface::ClaudeCli);
    let untouched_shared = OwnedArtifact {
        path: "OTHER.md".into(),
        ..prior[1].clone()
    };
    let unrelated_file = whole_file("RULE.md", "z");
    prior.extend([untouched_shared.clone(), unrelated_file.clone()]);
    let result =
        plan_instructions(&prior, &[remove("a")], &[observe("AGENTS.md", Some(bytes))]).unwrap();
    assert_eq!(result.artifacts.len(), 3);
    assert_eq!(result.artifacts[0].owners, vec![owner("b"), owner("c")]);
    assert_eq!(
        result.artifacts[0].consumers,
        vec![TargetSurface::CodexCli, TargetSurface::ClaudeCli]
    );
    assert_eq!(result.artifacts[1], untouched_shared);
    assert_eq!(result.artifacts[2], unrelated_file);
    let ArtifactContent::MarkdownContribution { contribution } = &result.artifacts[0].content
    else {
        panic!()
    };
    assert_eq!(contribution.preceding_newlines, 0);
    assert_eq!(contribution.leading_newlines, 1);
    assert_eq!(
        aggregate::inspect_owned(
            std::str::from_utf8(&result.files[0].after).unwrap(),
            std::slice::from_ref(contribution)
        )
        .unwrap(),
        vec![Contribution::new("b", "B\n")]
    );
}

fn assert_rejected_with(result: anyhow::Result<InstructionOperationPlan>, diagnostic: &str) {
    assert_eq!(
        result
            .err()
            .map(|error| format!("{error:#}").contains(diagnostic)),
        Some(true),
        "expected rejection mentioning {diagnostic}"
    );
}

#[test]
fn duplicate_set_changes_are_rejected_with_disjoint_selectors() {
    let changes = [
        InstructionInstallationChange::Set {
            owner: owner("a"),
            contributions: vec![desired("AGENTS.md", "left", "Left\n")],
        },
        InstructionInstallationChange::Set {
            owner: owner("a"),
            contributions: vec![desired("AGENTS.md", "right", "Right\n")],
        },
    ];
    assert_rejected_with(
        plan_instructions(&[], &changes, &[observe("AGENTS.md", None)]),
        "duplicate instruction change for installation a",
    );
}

#[test]
fn duplicate_set_remove_changes_are_rejected_for_known_installation() {
    let (prior, bytes) = fixture();
    assert_rejected_with(
        plan_instructions(
            &prior,
            &[set("a", "A\n"), remove("a")],
            &[observe("AGENTS.md", Some(bytes))],
        ),
        "duplicate instruction change for installation a",
    );
}

#[test]
fn empty_set_is_rejected_without_observations() {
    assert_rejected_with(
        plan_instructions(
            &[],
            &[InstructionInstallationChange::Set {
                owner: owner("a"),
                contributions: vec![],
            }],
            &[],
        ),
        "installation a has an empty contribution set",
    );
}

#[test]
fn unknown_removal_is_rejected_without_observations() {
    assert_rejected_with(
        plan_instructions(&[], &[remove("unknown")], &[]),
        "unknown installation unknown for removal",
    );
}

#[test]
fn invalid_desired_records_are_rejected_even_while_removing() {
    let (prior, bytes) = fixture();
    for invalid in [0, 1, 2, 3, 4, 5, 6, 9, 10, 11] {
        let mut binding = owner("new");
        let mut contribution = desired("AGENTS.md", "new", "New\n");
        match invalid {
            0 => binding.installation_id = "bad id".into(),
            1 => binding.source.clear(),
            2 => binding.source_name.clear(),
            3 => binding.installed_name.clear(),
            4 => binding.source_revision = Some(String::new()),
            5 => contribution.consumers.clear(),
            6 => contribution.consumers.push(TargetSurface::CodexCli),
            9 => contribution.id = "bad id".into(),
            10 => contribution.markdown = "no newline".into(),
            11 => contribution.markdown = "<!-- upskill:begin rogue -->\n".into(),
            _ => unreachable!("all invalid record cases are listed"),
        }
        let changes = [
            remove("a"),
            InstructionInstallationChange::Set {
                owner: binding,
                contributions: vec![contribution],
            },
        ];
        assert!(
            !error(
                &prior,
                &changes,
                &[observe("AGENTS.md", Some(bytes.clone()))]
            )
            .is_empty(),
            "case {invalid}"
        );
    }
    let changes = [InstructionInstallationChange::Set {
        owner: owner("a"),
        contributions: vec![
            desired("AGENTS.md", "a", "A\n"),
            desired("AGENTS.md", "a", "A\n"),
        ],
    }];
    assert!(
        error(
            &prior,
            &changes,
            &[observe("AGENTS.md", Some(bytes.clone()))]
        )
        .contains("a")
    );
    let cross_owner = [
        set("a", "A\n"),
        InstructionInstallationChange::Set {
            owner: owner("c"),
            contributions: vec![desired("AGENTS.md", "a", "A\n")],
        },
    ];
    assert!(error(&prior, &cross_owner, &[observe("AGENTS.md", Some(bytes))]).contains("a"));
}

fn assert_invalid_desired_location(location: InstructionLocation, diagnostic: &str) {
    let (prior, bytes) = fixture();
    let observations = [
        observe("AGENTS.md", Some(bytes)),
        ObservedInstructionFile {
            location: location.clone(),
            content: None,
        },
    ];
    let changes = [
        remove("a"),
        InstructionInstallationChange::Set {
            owner: owner("new"),
            contributions: vec![DesiredInstructionContribution {
                location,
                ..desired("AGENTS.md", "new", "New\n")
            }],
        },
    ];
    assert_rejected_with(
        plan_instructions(&prior, &changes, &observations),
        diagnostic,
    );
}

#[test]
fn parent_component_desired_path_reports_path_validation_while_removing() {
    assert_invalid_desired_location(
        location("../bad"),
        "artifact path must contain portable forward-slash relative components",
    );
}

#[test]
fn empty_desired_path_reports_path_validation_while_removing() {
    assert_invalid_desired_location(
        location(""),
        "artifact path must contain portable forward-slash relative components",
    );
}

#[test]
fn relative_desired_root_reports_root_validation_while_removing() {
    let mut location = location("AGENTS.md");
    location.root.path = "relative".into();
    assert_invalid_desired_location(
        location,
        "installation root must be a nonempty absolute path",
    );
}

#[test]
fn empty_desired_root_reports_root_validation_while_removing() {
    let mut location = location("AGENTS.md");
    location.root.path = "".into();
    assert_invalid_desired_location(
        location,
        "installation root must be a nonempty absolute path",
    );
}

#[test]
fn observations_are_exactly_required_even_for_no_op_updates() {
    let (prior, bytes) = fixture();
    assert!(!error(&prior, &[set("a", "A\n")], &[]).is_empty());
    assert!(
        !error(
            &prior,
            &[set("a", "A\n")],
            &[
                observe("AGENTS.md", Some(bytes.clone())),
                observe("AGENTS.md", Some(bytes.clone()))
            ]
        )
        .is_empty()
    );
    assert!(
        !error(
            &prior,
            &[set("a", "A\n")],
            &[observe("AGENTS.md", Some(bytes)), observe("EXTRA.md", None)]
        )
        .is_empty()
    );
    assert!(!error(&[], &[], &[observe("AGENTS.md", None)]).is_empty());
}

#[test]
fn absent_owned_files_and_invalid_bytes_or_markers_conflict() {
    let (prior, bytes) = fixture();
    assert!(error(&prior, &[remove("a")], &[observe("AGENTS.md", None)]).contains("AGENTS.md"));
    for content in [
        vec![0xff],
        bytes
            .clone()
            .into_iter()
            .map(|b| if b == b'A' { b'Z' } else { b })
            .collect(),
        b"Manual".to_vec(),
        [bytes.clone(), b"<!-- upskill:unknown -->\n".to_vec()].concat(),
        [bytes.clone(), bytes.clone()].concat(),
    ] {
        assert!(
            error(
                &prior,
                &[remove("a")],
                &[observe("AGENTS.md", Some(content))]
            )
            .contains("AGENTS.md")
        );
    }
}

#[test]
fn invalid_prior_is_rejected_without_changes() {
    let (mut prior, _) = fixture();
    prior[0].consumers.clear();
    assert!(plan_instructions(&prior, &[], &[]).is_err());
}

#[test]
fn empty_changes_preserve_complete_prior_order_and_input_vectors() {
    let (mut prior, _) = fixture();
    prior.reverse();
    prior.push(whole_file("OTHER.md", "z"));
    let snapshot = prior.clone();
    let result = plan_instructions(&prior, &[], &[]).unwrap();
    assert_eq!(result.artifacts, snapshot);
    assert!(result.files.is_empty());
    assert_eq!(prior, snapshot);
}

#[test]
fn raw_lexical_roots_are_distinct_and_new_outputs_sort_deterministically() {
    let mut lexical = location("AGENTS.md");
    lexical.root.path.push("./nested/..");
    let normal = location("AGENTS.md");
    let changes = vec![
        InstructionInstallationChange::Set {
            owner: owner("z"),
            contributions: vec![DesiredInstructionContribution {
                location: lexical.clone(),
                ..desired("AGENTS.md", "z", "Z\n")
            }],
        },
        set("a", "A\n"),
    ];
    let observations = vec![
        ObservedInstructionFile {
            location: lexical.clone(),
            content: Some(vec![]),
        },
        observe("AGENTS.md", None),
    ];
    let original = (changes.clone(), observations.clone());
    let first = plan_instructions(&[], &changes, &observations).unwrap();
    let mut reversed = changes.clone();
    reversed.reverse();
    assert_eq!(
        first,
        plan_instructions(&[], &reversed, &observations).unwrap()
    );
    assert_eq!(first.files.len(), 2);
    assert_eq!(
        first.files[0].location.root.path.as_os_str(),
        normal.root.path.as_os_str()
    );
    assert_eq!(
        first.files[1].location.root.path.as_os_str(),
        lexical.root.path.as_os_str()
    );
    assert_eq!(first.files[0].before, None);
    assert_eq!(first.files[1].before, Some(vec![]));
    assert_eq!(first.artifacts[0].owners, vec![owner("a")]);
    assert_eq!(first.artifacts[1].owners, vec![owner("z")]);
    assert_eq!((changes, observations), original);
}

#[test]
fn desired_consumers_survive_creation_and_consumer_only_update() {
    let initial_change = InstructionInstallationChange::Set {
        owner: owner("a"),
        contributions: vec![DesiredInstructionContribution {
            consumers: vec![TargetSurface::ClaudeCli, TargetSurface::CopilotCli],
            ..desired("AGENTS.md", "a", "A\n")
        }],
    };
    let first = plan_instructions(&[], &[initial_change], &[observe("AGENTS.md", None)]).unwrap();
    assert_eq!(
        first.artifacts[0].consumers,
        vec![TargetSurface::ClaudeCli, TargetSurface::CopilotCli]
    );
    let before = first.files[0].after.clone();
    let replacement = InstructionInstallationChange::Set {
        owner: owner("a"),
        contributions: vec![DesiredInstructionContribution {
            consumers: vec![TargetSurface::CopilotCli, TargetSurface::CodexCli],
            ..desired("AGENTS.md", "a", "A\n")
        }],
    };
    let second = plan_instructions(
        &first.artifacts,
        &[replacement],
        &[observe("AGENTS.md", Some(before.clone()))],
    )
    .unwrap();
    assert_eq!(
        second.artifacts[0].consumers,
        vec![TargetSurface::CopilotCli, TargetSurface::CodexCli]
    );
    assert_eq!(second.artifacts[0].owners, vec![owner("a")]);
    assert_eq!(second.files[0].before, Some(before.clone()));
    assert_eq!(second.files[0].after, before);
}

#[test]
fn scopes_distinguish_locations_and_sort_project_before_user() {
    let mut user = location("AGENTS.md");
    user.root.scope = OwnershipScope::User;
    let changes = [
        InstructionInstallationChange::Set {
            owner: owner("user"),
            contributions: vec![DesiredInstructionContribution {
                location: user.clone(),
                ..desired("AGENTS.md", "user", "User\n")
            }],
        },
        set("project", "Project\n"),
    ];
    let observations = [
        ObservedInstructionFile {
            location: user,
            content: None,
        },
        observe("AGENTS.md", None),
    ];
    let plan = plan_instructions(&[], &changes, &observations).unwrap();
    assert_eq!(plan.files.len(), 2);
    assert_eq!(plan.files[0].location.root.scope, OwnershipScope::Project);
    assert_eq!(plan.files[1].location.root.scope, OwnershipScope::User);
    assert_eq!(
        plan.files[0].after,
        b"<!-- upskill:begin project -->\nProject\n<!-- upskill:end project -->\n"
    );
    assert_eq!(
        plan.files[1].after,
        b"<!-- upskill:begin user -->\nUser\n<!-- upskill:end user -->\n"
    );
    assert_eq!(plan.artifacts[0].owners, vec![owner("project")]);
    assert_eq!(plan.artifacts[1].owners, vec![owner("user")]);
}

#[test]
fn new_selectors_at_one_location_append_in_ascending_id_order() {
    let plan = plan_instructions(
        &[],
        &[set("z", "Z\n"), set("m", "M\n"), set("c", "C\n")],
        &[observe("AGENTS.md", None)],
    )
    .unwrap();
    assert_eq!(
        plan.artifacts
            .iter()
            .map(|row| match &row.content {
                ArtifactContent::MarkdownContribution { contribution } => contribution.id.as_str(),
                _ => panic!("expected contribution metadata"),
            })
            .collect::<Vec<_>>(),
        vec!["c", "m", "z"]
    );
}

#[test]
fn nonempty_changes_preserve_surviving_prior_row_order() {
    let (mut prior, bytes) = fixture();
    prior.reverse();
    let original = prior.clone();
    let plan = plan_instructions(
        &prior,
        &[
            set("z", "Z\n"),
            set("a", "Updated A\n"),
            set("m", "M\n"),
            set("c", "C\n"),
        ],
        &[observe("AGENTS.md", Some(bytes))],
    )
    .unwrap();
    assert_eq!(
        plan.artifacts
            .iter()
            .map(|row| match &row.content {
                ArtifactContent::MarkdownContribution { contribution } => contribution.id.as_str(),
                _ => panic!("expected contribution metadata"),
            })
            .collect::<Vec<_>>(),
        vec!["b", "a", "c", "m", "z"]
    );
    assert_eq!(
        plan.artifacts
            .iter()
            .map(|row| row.owners.clone())
            .collect::<Vec<_>>(),
        vec![
            vec![owner("b")],
            vec![owner("a")],
            vec![owner("c")],
            vec![owner("m")],
            vec![owner("z")],
        ]
    );
    assert_eq!(plan.artifacts[0], original[0]);
    assert_eq!(prior, original);
}
