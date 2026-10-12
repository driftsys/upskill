use upskill::files::plan::{DesiredFile, FileLocation, FileValue, ObservedFile};
use upskill::instructions::plan::{DesiredInstructionContribution, InstructionLocation};
use upskill::operations::plan::{DesiredArtifact, InstallationChange, plan_owned_artifacts};
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn location(path: &str) -> FileLocation {
    FileLocation {
        root: InstallationRoot {
            scope: OwnershipScope::Project,
            path: std::env::temp_dir().join("upskill-mixed-plan"),
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
        source_name: "source-demo".into(),
        installed_name: "demo".into(),
    }
}

#[test]
fn creates_mixed_installation_with_exact_manual_bytes_and_modes() {
    let instructions = location("AGENTS.md");
    let resource = location("resources/data.bin");
    let changes = vec![InstallationChange::Set {
        owner: owner("install-a"),
        artifacts: vec![
            DesiredArtifact::Instruction(DesiredInstructionContribution {
                location: InstructionLocation {
                    root: instructions.root.clone(),
                    path: instructions.path.clone(),
                },
                id: "rule-a".into(),
                markdown: "A\n".into(),
                consumers: vec![TargetSurface::CodexCli],
            }),
            DesiredArtifact::File(DesiredFile {
                location: resource.clone(),
                value: FileValue {
                    bytes: vec![0, 255],
                    mode: Some(0o755),
                },
                consumers: vec![TargetSurface::ClaudeCli],
            }),
        ],
    }];
    let observations = vec![
        ObservedFile {
            location: resource.clone(),
            value: None,
        },
        ObservedFile {
            location: instructions.clone(),
            value: Some(FileValue {
                bytes: b"Manual".to_vec(),
                mode: Some(0o640),
            }),
        },
    ];
    let saved = (changes.clone(), observations.clone());
    let result = plan_owned_artifacts(&[], &changes, &observations).unwrap();
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.artifacts.len(), 2);
    assert_eq!(result.files[0].location, instructions);
    assert_eq!(result.files[0].before, observations[1].value);
    assert_eq!(
        result.files[0].after,
        Some(FileValue {
            bytes: b"Manual\n\n<!-- upskill:begin rule-a -->\nA\n<!-- upskill:end rule-a -->\n"
                .to_vec(),
            mode: Some(0o640),
        })
    );
    assert_eq!(result.files[1].location, resource);
    assert_eq!(result.files[1].before, None);
    assert_eq!(
        result.files[1].after,
        Some(FileValue {
            bytes: vec![0, 255],
            mode: Some(0o755)
        })
    );
    assert_eq!((changes, observations), saved);
}

#[test]
fn rejects_empty_complete_set() {
    let changes = vec![InstallationChange::Set {
        owner: owner("a"),
        artifacts: vec![],
    }];
    let saved = changes.clone();
    let result = plan_owned_artifacts(&[], &changes, &[]);
    assert!(result.is_err(), "empty complete set must conflict");
    assert!(format!("{:#}", result.unwrap_err()).contains("empty"));
    assert_eq!(changes, saved);
}

use upskill::files::plan::PlannedFile;
use upskill::instructions::aggregate::{self, Contribution};
use upskill::ownership::{ArtifactContent, OwnedArtifact};

fn file(path: &str, mode: Option<u32>) -> DesiredArtifact {
    DesiredArtifact::File(DesiredFile {
        location: location(path),
        value: FileValue {
            bytes: b"abc".to_vec(),
            mode,
        },
        consumers: vec![TargetSurface::ClaudeCli],
    })
}
fn instruction(path: &str, id: &str) -> DesiredArtifact {
    DesiredArtifact::Instruction(DesiredInstructionContribution {
        location: InstructionLocation {
            root: location(path).root,
            path: path.into(),
        },
        id: id.into(),
        markdown: "A\n".into(),
        consumers: vec![TargetSurface::CodexCli],
    })
}
fn set(id: &str, artifacts: Vec<DesiredArtifact>) -> InstallationChange {
    InstallationChange::Set {
        owner: owner(id),
        artifacts,
    }
}
fn remove(id: &str) -> InstallationChange {
    InstallationChange::Remove {
        installation_id: id.into(),
    }
}
fn observed(path: &str, bytes: Option<&[u8]>, mode: Option<u32>) -> ObservedFile {
    ObservedFile {
        location: location(path),
        value: bytes.map(|bytes| FileValue {
            bytes: bytes.to_vec(),
            mode,
        }),
    }
}
fn file_record(id: &str, path: &str, mode: Option<u32>) -> OwnedArtifact {
    OwnedArtifact {
        root: location(path).root,
        path: path.into(),
        content: ArtifactContent::File {
            content_sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .into(),
            mode,
        },
        owners: vec![owner(id)],
        consumers: vec![TargetSurface::ClaudeCli],
    }
}
// The existing editor supplies valid prior metadata only; expected results below
// use independent literal bytes and ownership lists.
fn instruction_record(id: &str, path: &str, selector: &str, manual: &str) -> OwnedArtifact {
    let edit = aggregate::reconcile(manual, &[], &[Contribution::new(selector, "A\n")]).unwrap();
    OwnedArtifact {
        root: location(path).root,
        path: path.into(),
        content: ArtifactContent::MarkdownContribution {
            contribution: edit.owned[0].clone(),
        },
        owners: vec![owner(id)],
        consumers: vec![TargetSurface::CodexCli],
    }
}
const AGGREGATE: &[u8] = b"Manual\n\n<!-- upskill:begin a -->\nA\n<!-- upskill:end a -->\n";
const EMPTY_MANUAL: &[u8] = b"<!-- upskill:begin a -->\nA\n<!-- upskill:end a -->\n";
fn mixed_prior() -> Vec<OwnedArtifact> {
    vec![
        instruction_record("a", "AGENTS.md", "a", "Manual"),
        file_record("a", "old-1", Some(0o755)),
        file_record("a", "old-2", None),
    ]
}
fn mixed_observations() -> Vec<ObservedFile> {
    vec![
        observed("AGENTS.md", Some(AGGREGATE), Some(0o640)),
        observed("old-1", Some(b"abc"), Some(0o755)),
        observed("old-2", Some(b"abc"), None),
    ]
}
fn rejects(
    prior: &[OwnedArtifact],
    changes: &[InstallationChange],
    observations: &[ObservedFile],
    context: &str,
) {
    let saved = (prior.to_vec(), changes.to_vec(), observations.to_vec());
    let result = plan_owned_artifacts(prior, changes, observations);
    assert!(
        result.is_err(),
        "expected conflict containing {context}, got {result:?}"
    );
    let diagnostic = format!("{:#}", result.unwrap_err());
    assert!(
        diagnostic.contains(context),
        "expected {context:?} in {diagnostic:?}"
    );
    assert_eq!(
        (prior.to_vec(), changes.to_vec(), observations.to_vec()),
        saved
    );
}
fn paths(rows: &[OwnedArtifact]) -> Vec<&str> {
    rows.iter().map(|row| row.path.as_str()).collect()
}

// C01: missing actions, altered modes or lost explicit consumers must fail.
#[test]
fn c01_creation_records_keep_literal_hashes_owners_and_consumers() {
    let result = plan_owned_artifacts(
        &[],
        &[set(
            "a",
            vec![file("data", Some(0o755)), instruction("AGENTS.md", "a")],
        )],
        &[
            observed("data", None, None),
            observed("AGENTS.md", None, None),
        ],
    )
    .unwrap();
    assert_eq!(paths(&result.artifacts), vec!["AGENTS.md", "data"]);
    assert_eq!(result.artifacts[0].owners, vec![owner("a")]);
    assert_eq!(result.artifacts[0].consumers, vec![TargetSurface::CodexCli]);
    assert_eq!(result.artifacts[1], file_record("a", "data", Some(0o755)));
    assert_eq!(
        result.files[0].after,
        Some(FileValue {
            bytes: EMPTY_MANUAL.to_vec(),
            mode: None
        })
    );
}

// C02: silently retaining omitted resources breaks the complete-set contract.
#[test]
fn c02_omitted_two_resources_are_removed_and_contribution_is_updated() {
    let prior = mixed_prior();
    let mut updated = instruction("AGENTS.md", "a");
    if let DesiredArtifact::Instruction(value) = &mut updated {
        value.markdown = "Updated\n".into();
    }
    let result = plan_owned_artifacts(
        &prior,
        &[set("a", vec![updated.clone()])],
        &mixed_observations(),
    )
    .unwrap();
    assert_eq!(
        result.artifacts,
        vec![OwnedArtifact {
            root: location("AGENTS.md").root,
            path: "AGENTS.md".into(),
            content: ArtifactContent::MarkdownContribution {
                contribution: aggregate::OwnedContribution {
                    id: "a".into(),
                    content_sha256:
                        "1e77af781a8b33e666b31dd6fc97f48d064d1eac5c021f9255c8a4b133fd7ab8".into(),
                    leading_newlines: 2,
                    preceding_newlines: 0,
                },
            },
            owners: vec![owner("a")],
            consumers: vec![TargetSurface::CodexCli],
        }]
    );
    assert_eq!(
        result.files,
        vec![
            PlannedFile {
                location: location("AGENTS.md"),
                before: Some(FileValue {
                    bytes: AGGREGATE.to_vec(),
                    mode: Some(0o640)
                }),
                after: Some(FileValue {
                    bytes: b"Manual\n\n<!-- upskill:begin a -->\nUpdated\n<!-- upskill:end a -->\n"
                        .to_vec(),
                    mode: Some(0o640)
                })
            },
            PlannedFile {
                location: location("old-1"),
                before: Some(FileValue {
                    bytes: b"abc".to_vec(),
                    mode: Some(0o755)
                }),
                after: None
            },
            PlannedFile {
                location: location("old-2"),
                before: Some(FileValue {
                    bytes: b"abc".to_vec(),
                    mode: None
                }),
                after: None
            },
        ]
    );
    let no_op = plan_owned_artifacts(
        &result.artifacts,
        &[set("a", vec![updated])],
        &[ObservedFile {
            location: location("AGENTS.md"),
            value: result.files[0].after.clone(),
        }],
    )
    .unwrap();
    assert_eq!(no_op.artifacts, result.artifacts);
    let expected = Some(FileValue {
        bytes: b"Manual\n\n<!-- upskill:begin a -->\nUpdated\n<!-- upskill:end a -->\n".to_vec(),
        mode: Some(0o640),
    });
    assert_eq!(
        no_op.files,
        vec![PlannedFile {
            location: location("AGENTS.md"),
            before: expected.clone(),
            after: expected,
        }]
    );
}
#[test]
fn c02_omitted_contribution_preserves_manual_text_and_desired_resources() {
    let prior = mixed_prior();
    let result = plan_owned_artifacts(
        &prior,
        &[set(
            "a",
            vec![file("old-1", Some(0o755)), file("old-2", None)],
        )],
        &mixed_observations(),
    )
    .unwrap();
    assert_eq!(
        result.artifacts,
        vec![
            file_record("a", "old-1", Some(0o755)),
            file_record("a", "old-2", None)
        ]
    );
    assert_eq!(result.files.len(), 3);
    assert_eq!(
        result.files[0].after,
        Some(FileValue {
            bytes: b"Manual".to_vec(),
            mode: Some(0o640)
        })
    );
    assert_eq!(result.files[1].before, result.files[1].after);
    assert_eq!(result.files[2].before, result.files[2].after);
}
#[test]
fn c02_global_removal_reaches_both_forms_retains_empty_aggregate_and_untouched_owner() {
    let mut prior = mixed_prior();
    let untouched = file_record("untouched", "keep", Some(0o600));
    prior.push(untouched.clone());
    let result = plan_owned_artifacts(&prior, &[remove("a")], &mixed_observations()).unwrap();
    assert_eq!(result.artifacts, vec![untouched]);
    assert_eq!(result.files.len(), 3);
    assert_eq!(
        result.files[0].after,
        Some(FileValue {
            bytes: b"Manual".to_vec(),
            mode: Some(0o640)
        })
    );
    assert_eq!(result.files[1].after, None);
    assert_eq!(result.files[2].after, None);
    let result = plan_owned_artifacts(
        &[instruction_record("a", "AGENTS.md", "a", "")],
        &[remove("a")],
        &[observed("AGENTS.md", Some(EMPTY_MANUAL), Some(0o640))],
    )
    .unwrap();
    assert_eq!(
        result.files[0].after,
        Some(FileValue {
            bytes: vec![],
            mode: Some(0o640)
        })
    );
}

// C03/C04: a disjoint-form conversion must still preserve global bindings.
#[test]
fn c03_distinct_location_conversions_accept_coherent_revision_changes() {
    for reverse in [false, true] {
        let (prior, desired, observations, expected_path) = if reverse {
            (
                vec![instruction_record("a", "AGENTS.md", "a", "")],
                file("data", Some(0o600)),
                vec![
                    observed("AGENTS.md", Some(EMPTY_MANUAL), None),
                    observed("data", None, None),
                ],
                "data",
            )
        } else {
            (
                vec![file_record("a", "data", Some(0o600))],
                instruction("AGENTS.md", "a"),
                vec![
                    observed("data", Some(b"abc"), Some(0o600)),
                    observed("AGENTS.md", None, None),
                ],
                "AGENTS.md",
            )
        };
        let mut binding = owner("a");
        binding.source_revision = Some("v2".into());
        let changes = vec![InstallationChange::Set {
            owner: binding.clone(),
            artifacts: vec![desired],
        }];
        let result = plan_owned_artifacts(&prior, &changes, &observations).unwrap();
        assert_eq!(paths(&result.artifacts), vec![expected_path]);
        assert_eq!(result.artifacts[0].owners, vec![binding]);
        assert_eq!(result.files.len(), 2);
        assert_eq!(
            result.files[0].after,
            Some(FileValue {
                bytes: if reverse {
                    vec![]
                } else {
                    EMPTY_MANUAL.to_vec()
                },
                mode: None
            })
        );
        assert_eq!(
            result.files[1].after,
            if reverse {
                Some(FileValue {
                    bytes: b"abc".to_vec(),
                    mode: Some(0o600),
                })
            } else {
                None
            }
        );
    }
}
#[test]
fn c04_disjoint_form_changes_reject_each_stable_binding_field() {
    for reverse in [false, true] {
        for field in ["source", "kind", "source_name", "installed_name"] {
            let mut binding = owner("a");
            match field {
                "source" => binding.source = "local:./other".into(),
                "kind" => binding.kind = ItemKind::Skill,
                "source_name" => binding.source_name = "other".into(),
                _ => binding.installed_name = "other".into(),
            }
            let (prior, desired, observations) = if reverse {
                (
                    vec![instruction_record("a", "AGENTS.md", "a", "")],
                    file("data", None),
                    vec![
                        observed("AGENTS.md", Some(EMPTY_MANUAL), None),
                        observed("data", None, None),
                    ],
                )
            } else {
                (
                    vec![file_record("a", "data", None)],
                    instruction("AGENTS.md", "a"),
                    vec![
                        observed("data", Some(b"abc"), None),
                        observed("AGENTS.md", None, None),
                    ],
                )
            };
            rejects(
                &prior,
                &[InstallationChange::Set {
                    owner: binding,
                    artifacts: vec![desired],
                }],
                &observations,
                field,
            );
        }
    }
}
#[test]
fn c04_duplicate_changes_unknown_removal_and_duplicate_desired_values_conflict() {
    let prior = mixed_prior();
    let observations = mixed_observations();
    let desired = set("a", vec![instruction("AGENTS.md", "a")]);
    for changes in [
        vec![desired.clone(), remove("a")],
        vec![desired.clone(), desired],
        vec![remove("a"), remove("a")],
    ] {
        rejects(&prior, &changes, &observations, "duplicate");
    }
    rejects(&[], &[remove("unknown")], &[], "unknown");
    rejects(&[], &[set("a", vec![])], &[], "empty");
    for artifact in [file("same", None), instruction("same", "a")] {
        rejects(
            &[],
            &[set("a", vec![artifact.clone(), artifact])],
            &[observed("same", None, None)],
            "duplicate",
        );
    }
}
#[test]
fn c04_malformed_owner_and_consumers_conflict_for_both_forms() {
    for artifact in [file("data", None), instruction("AGENTS.md", "a")] {
        for invalid in 0..7 {
            let mut binding = owner("a");
            let mut desired = artifact.clone();
            match invalid {
                0 => binding.installation_id = "bad id".into(),
                1 => binding.source.clear(),
                2 => binding.source_name.clear(),
                3 => binding.installed_name.clear(),
                4 => binding.source_revision = Some(String::new()),
                _ => {
                    let consumers = match &mut desired {
                        DesiredArtifact::File(value) => &mut value.consumers,
                        DesiredArtifact::Instruction(value) => &mut value.consumers,
                    };
                    if invalid == 5 {
                        consumers.clear();
                    } else {
                        consumers.push(consumers[0]);
                    }
                }
            }
            let path = match &desired {
                DesiredArtifact::File(value) => &value.location.path,
                DesiredArtifact::Instruction(value) => &value.location.path,
            };
            rejects(
                &[],
                &[InstallationChange::Set {
                    owner: binding,
                    artifacts: vec![desired.clone()],
                }],
                &[observed(path, None, None)],
                "installation",
            );
        }
    }
}

// C05: even explicit removal does not confer aggregate ownership.
#[test]
fn c05_desired_cross_form_location_collision_conflicts() {
    rejects(
        &[],
        &[set("a", vec![file("same", None), instruction("same", "a")])],
        &[observed("same", None, None)],
        "cross-form",
    );
}
#[test]
fn c05_cross_form_takeovers_and_explicit_handoffs_conflict_in_both_directions() {
    for reverse in [false, true] {
        let (prior, desired, observation) = if reverse {
            (
                instruction_record("a", "same", "a", ""),
                file("same", None),
                observed("same", Some(EMPTY_MANUAL), None),
            )
        } else {
            (
                file_record("a", "same", None),
                instruction("same", "a"),
                observed("same", Some(b"abc"), None),
            )
        };
        for changes in [
            vec![set("a", vec![desired.clone()])],
            vec![remove("a"), set("b", vec![desired.clone()])],
            vec![set("b", vec![desired])],
        ] {
            rejects(
                std::slice::from_ref(&prior),
                &changes,
                std::slice::from_ref(&observation),
                "cross-form",
            );
        }
    }
}

// C06: component-boundary topology includes old paths and removal-only requests.
#[test]
fn c06_cross_form_ancestor_topology_conflicts_in_both_directions() {
    for artifacts in [
        vec![instruction("a", "a"), file("a/b", None)],
        vec![file("a", None), instruction("a/b", "a")],
        vec![
            instruction("a", "a"),
            file("a-keep", None),
            file("a/b", None),
        ],
    ] {
        let observations: Vec<_> = artifacts
            .iter()
            .map(|value| match value {
                DesiredArtifact::File(value) => observed(&value.location.path, None, None),
                DesiredArtifact::Instruction(value) => observed(&value.location.path, None, None),
            })
            .collect();
        rejects(&[], &[set("a", artifacts)], &observations, "topology");
    }
}
#[test]
fn c06_removal_only_and_old_new_topology_conflict_but_historical_pairs_survive() {
    let prior = vec![
        instruction_record("a", "a", "a", ""),
        file_record("b", "a/b", None),
    ];
    rejects(
        &prior,
        &[remove("a")],
        &[observed("a", Some(EMPTY_MANUAL), None)],
        "topology",
    );
    rejects(
        &prior,
        &[remove("b")],
        &[observed("a/b", Some(b"abc"), None)],
        "topology",
    );
    let result = plan_owned_artifacts(
        &prior,
        &[set("c", vec![file("z", None)])],
        &[observed("z", None, None)],
    )
    .unwrap();
    assert_eq!(
        result.artifacts,
        vec![
            prior[0].clone(),
            prior[1].clone(),
            file_record("c", "z", None)
        ]
    );
    rejects(
        &prior[..1],
        &[set("a", vec![file("a/b", None)])],
        &[
            observed("a", Some(EMPTY_MANUAL), None),
            observed("a/b", None, None),
        ],
        "topology",
    );
    rejects(
        &[file_record("a", "a/b", None)],
        &[set("a", vec![instruction("a", "a")])],
        &[
            observed("a/b", Some(b"abc"), None),
            observed("a", None, None),
        ],
        "topology",
    );
}
#[test]
fn c06_siblings_with_shared_text_prefix_are_distinct() {
    let result = plan_owned_artifacts(
        &[],
        &[set("a", vec![file("ab", None), instruction("a/b", "a")])],
        &[observed("a/b", None, None), observed("ab", None, None)],
    )
    .unwrap();
    assert_eq!(paths(&result.artifacts), vec!["a/b", "ab"]);
    assert_eq!(result.files.len(), 2);
}

// C07: exact evidence remains mandatory, including every byte-identical no-op.
#[test]
fn c07_missing_duplicate_and_extra_observations_conflict() {
    let prior = mixed_prior();
    let changes = vec![remove("a")];
    let observations = mixed_observations();
    for missing in 0..observations.len() {
        let mut changed = observations.clone();
        changed.remove(missing);
        rejects(&prior, &changes, &changed, "missing observation");
    }
    let mut duplicate = observations.clone();
    duplicate.push(observations[0].clone());
    rejects(&prior, &changes, &duplicate, "duplicate observation");
    let mut extra = observations.clone();
    extra.push(observed("extra", None, None));
    rejects(&prior, &changes, &extra, "extraneous observation");
}
#[test]
fn c07_invalid_observation_roots_paths_and_modes_conflict_before_extras() {
    for invalid in 0..6 {
        let mut observation = observed("data", None, None);
        let context = match invalid {
            0 => {
                observation.location.root.path = "relative".into();
                "root"
            }
            1 => {
                observation.location.root.path = "".into();
                "root"
            }
            2 => {
                observation.location.path = "../bad".into();
                "path"
            }
            3 => {
                observation.location.path = "".into();
                "path"
            }
            4 => {
                observation.location.path = "a\\b".into();
                "path"
            }
            _ => {
                observation.value = Some(FileValue {
                    bytes: vec![],
                    mode: Some(0o10000),
                });
                "mode"
            }
        };
        rejects(&[], &[], &[observation], context);
    }
}
#[test]
fn c07_invalid_desired_locations_modes_and_prior_rows_conflict() {
    for instruction_form in [false, true] {
        for invalid in 0..5 {
            let mut desired = if instruction_form {
                instruction("data", "a")
            } else {
                file("data", None)
            };
            let (root, path) = match &mut desired {
                DesiredArtifact::File(value) => {
                    (&mut value.location.root, &mut value.location.path)
                }
                DesiredArtifact::Instruction(value) => {
                    (&mut value.location.root, &mut value.location.path)
                }
            };
            let context = match invalid {
                0 => {
                    root.path = "relative".into();
                    "root"
                }
                1 => {
                    root.path = "".into();
                    "root"
                }
                2 => {
                    *path = "../bad".into();
                    "path"
                }
                3 => {
                    *path = "".into();
                    "path"
                }
                _ => {
                    *path = "a:b".into();
                    "path"
                }
            };
            rejects(&[], &[set("a", vec![desired])], &[], context);
        }
    }
    let invalid_file = file("data", Some(0o10000));
    rejects(
        &[],
        &[set("a", vec![invalid_file.clone()])],
        &[observed("data", None, None)],
        "mode",
    );
    let mut invalid_prior = file_record("a", "data", None);
    invalid_prior.consumers.clear();
    rejects(&[invalid_prior], &[], &[], "prior");
}

// C08: no-op actions cannot discard modes or bypass existing before checks.
#[test]
fn c08_aggregate_and_dedicated_noops_preserve_exact_optional_modes() {
    let prior = mixed_prior();
    let changes = vec![set(
        "a",
        vec![
            instruction("AGENTS.md", "a"),
            file("old-1", Some(0o755)),
            file("old-2", None),
        ],
    )];
    let observations = mixed_observations();
    let result = plan_owned_artifacts(&prior, &changes, &observations).unwrap();
    assert_eq!(result.artifacts, prior);
    assert_eq!(result.files.len(), 3);
    for (action, before) in result.files.iter().zip(&observations) {
        assert_eq!(action.before, before.value);
        assert_eq!(action.after, before.value);
    }
    for mode in [Some(0o755), Some(0o600), None] {
        let result = plan_owned_artifacts(
            &[file_record("a", "data", mode)],
            &[set("a", vec![file("data", mode)])],
            &[observed("data", Some(b"abc"), mode)],
        )
        .unwrap();
        assert_eq!(
            result.files,
            vec![PlannedFile {
                location: location("data"),
                before: Some(FileValue {
                    bytes: b"abc".to_vec(),
                    mode
                }),
                after: Some(FileValue {
                    bytes: b"abc".to_vec(),
                    mode
                })
            }]
        );
        rejects(
            &[file_record("a", "data", mode)],
            &[set("a", vec![file("data", mode)])],
            &[],
            "missing observation",
        );
    }
}
#[test]
fn c08_dedicated_missing_edited_bytes_or_changed_modes_reject_all_operations() {
    let prior = vec![file_record("a", "data", Some(0o755))];
    for change in [
        remove("a"),
        set("a", vec![file("data", Some(0o755))]),
        set("a", vec![file("data", Some(0o600))]),
    ] {
        for (observation, context) in [
            (observed("data", None, None), "missing owned"),
            (
                observed("data", Some(b"edited"), Some(0o755)),
                "bytes changed",
            ),
            (observed("data", Some(b"abc"), Some(0o600)), "mode evidence"),
            (observed("data", Some(b"abc"), None), "mode evidence"),
        ] {
            rejects(
                &prior,
                std::slice::from_ref(&change),
                &[observation],
                context,
            );
        }
    }
    rejects(
        &[],
        &[set("a", vec![file("data", Some(0o755))])],
        &[observed("data", Some(b"abc"), Some(0o755))],
        "unowned",
    );
}
#[test]
fn c08_invalid_aggregate_utf8_edited_or_unrecorded_markers_conflict() {
    let prior = vec![instruction_record("a", "AGENTS.md", "a", "Manual")];
    for (bytes, context) in [
        (vec![255], "UTF-8"),
        (b"Manual".to_vec(), "instruction"),
        (
            b"Manual\n\n<!-- upskill:begin a -->\nEdited\n<!-- upskill:end a -->\n".to_vec(),
            "instruction",
        ),
        (
            [
                AGGREGATE,
                b"<!-- upskill:begin rogue -->\nR\n<!-- upskill:end rogue -->\n",
            ]
            .concat(),
            "instruction",
        ),
    ] {
        for change in [remove("a"), set("a", vec![instruction("AGENTS.md", "a")])] {
            rejects(
                &prior,
                &[change],
                &[observed("AGENTS.md", Some(&bytes), Some(0o640))],
                context,
            );
        }
    }
    rejects(
        &prior,
        &[remove("a")],
        &[observed("AGENTS.md", None, None)],
        "absent",
    );
}

// C09: shared ownership and physical identities must survive partitioning.
#[test]
fn c09_affected_shared_rows_conflict_untouched_shared_rows_survive() {
    for mut shared in [
        file_record("a", "shared", None),
        instruction_record("a", "shared", "a", ""),
    ] {
        shared.owners.push(owner("b"));
        rejects(
            std::slice::from_ref(&shared),
            &[remove("a")],
            &[observed("shared", None, None)],
            "multi-owner",
        );
        let same_form = match shared.content {
            ArtifactContent::File { .. } => file("shared", None),
            _ => instruction("shared", "other"),
        };
        rejects(
            std::slice::from_ref(&shared),
            &[set("c", vec![same_form])],
            &[observed("shared", None, None)],
            "multi-owner",
        );
        let result = plan_owned_artifacts(
            std::slice::from_ref(&shared),
            &[set("c", vec![file("new", None)])],
            &[observed("new", None, None)],
        )
        .unwrap();
        assert_eq!(
            result.artifacts,
            vec![shared, file_record("c", "new", None)]
        );
    }
}
#[test]
fn c09_same_form_file_and_selector_handoffs_preserve_existing_semantics() {
    let result = plan_owned_artifacts(
        &[file_record("a", "same", None)],
        &[remove("a"), set("b", vec![file("same", Some(0o600))])],
        &[observed("same", Some(b"abc"), None)],
    )
    .unwrap();
    assert_eq!(
        result.artifacts,
        vec![file_record("b", "same", Some(0o600))]
    );
    assert_eq!(
        result.files[0].after,
        Some(FileValue {
            bytes: b"abc".to_vec(),
            mode: Some(0o600)
        })
    );
    rejects(
        &[file_record("a", "same", None)],
        &[set("b", vec![file("same", None)])],
        &[observed("same", Some(b"abc"), None)],
        "untouched",
    );
    let prior = vec![instruction_record("a", "same", "a", "")];
    let result = plan_owned_artifacts(
        &prior,
        &[remove("a"), set("b", vec![instruction("same", "a")])],
        &[observed("same", Some(EMPTY_MANUAL), Some(0o640))],
    )
    .unwrap();
    assert_eq!(result.artifacts[0].owners, vec![owner("b")]);
    assert_eq!(result.files[0].before, result.files[0].after);
    rejects(
        &prior,
        &[set("b", vec![instruction("same", "a")])],
        &[observed("same", Some(EMPTY_MANUAL), None)],
        "untouched",
    );
}
#[test]
fn c09_raw_dot_roots_and_scopes_distinguish_cross_form_locations() {
    let plain = location("same");
    let mut lexical = location("same");
    let mut raw = lexical.root.path.as_os_str().to_os_string();
    raw.push("/.");
    lexical.root.path = raw.into();
    let mut user = location("same");
    user.root.scope = OwnershipScope::User;
    let mut contribution = instruction("same", "a");
    if let DesiredArtifact::Instruction(value) = &mut contribution {
        value.location.root = lexical.root.clone();
    }
    let mut user_file = file("same", Some(0o600));
    if let DesiredArtifact::File(value) = &mut user_file {
        value.location.root = user.root.clone();
    }
    let result = plan_owned_artifacts(
        &[],
        &[set("a", vec![user_file, contribution, file("same", None)])],
        &[
            ObservedFile {
                location: user.clone(),
                value: None,
            },
            ObservedFile {
                location: lexical.clone(),
                value: None,
            },
            ObservedFile {
                location: plain.clone(),
                value: None,
            },
        ],
    )
    .unwrap();
    assert_eq!(result.files.len(), 3);
    assert_eq!(
        result.files[0].location.root.path.as_os_str(),
        plain.root.path.as_os_str()
    );
    assert_eq!(
        result.files[1].location.root.path.as_os_str(),
        lexical.root.path.as_os_str()
    );
    assert_eq!(result.files[2].location.root.scope, OwnershipScope::User);
    assert_eq!(result.files[2].after.as_ref().unwrap().mode, Some(0o600));
}

// C10: sorting partitions rather than reconstructing original identities loses order.
#[test]
fn c10_interleaved_prior_order_and_sorted_new_rows_are_deterministic() {
    let prior = vec![
        file_record("a", "old-1", None),
        instruction_record("b", "B.md", "a", ""),
        file_record("a", "keep", Some(0o755)),
        instruction_record("a", "A.md", "a", ""),
        file_record("a", "old-2", Some(0o600)),
        file_record("b", "untouched", None),
    ];
    let changes = vec![
        set(
            "a",
            vec![
                instruction("A.md", "a"),
                file("keep", Some(0o600)),
                file("z-new", None),
                instruction("C.md", "z"),
                instruction("C.md", "m"),
            ],
        ),
        set("c", vec![file("d-new", Some(0o755))]),
    ];
    let observations = vec![
        observed("old-1", Some(b"abc"), None),
        observed("keep", Some(b"abc"), Some(0o755)),
        observed("A.md", Some(EMPTY_MANUAL), Some(0o640)),
        observed("old-2", Some(b"abc"), Some(0o600)),
        observed("z-new", None, None),
        observed("C.md", None, None),
        observed("d-new", None, None),
    ];
    let saved = (prior.clone(), changes.clone(), observations.clone());
    let result = plan_owned_artifacts(&prior, &changes, &observations).unwrap();
    assert_eq!(
        paths(&result.artifacts),
        vec![
            "B.md",
            "keep",
            "A.md",
            "untouched",
            "C.md",
            "C.md",
            "d-new",
            "z-new"
        ]
    );
    assert_eq!(result.artifacts[0], prior[1]);
    assert_eq!(result.artifacts[1], file_record("a", "keep", Some(0o600)));
    assert_eq!(result.artifacts[2], prior[3]);
    assert_eq!(result.artifacts[3], prior[5]);
    assert_eq!(
        result
            .artifacts
            .iter()
            .map(|row| row.owners.clone())
            .collect::<Vec<_>>(),
        vec![
            vec![owner("b")],
            vec![owner("a")],
            vec![owner("a")],
            vec![owner("b")],
            vec![owner("a")],
            vec![owner("a")],
            vec![owner("c")],
            vec![owner("a")]
        ]
    );
    let new_selectors: Vec<_> = result.artifacts[4..6]
        .iter()
        .map(|row| match &row.content {
            ArtifactContent::MarkdownContribution { contribution } => contribution.id.as_str(),
            _ => panic!("expected contribution"),
        })
        .collect();
    assert_eq!(new_selectors, vec!["m", "z"]);
    assert_eq!(
        result
            .files
            .iter()
            .map(|action| action.location.path.as_str())
            .collect::<Vec<_>>(),
        vec!["A.md", "C.md", "d-new", "keep", "old-1", "old-2", "z-new"]
    );
    assert_eq!(
        result.files[0].after,
        Some(FileValue {
            bytes: EMPTY_MANUAL.to_vec(),
            mode: Some(0o640)
        })
    );
    assert_eq!(result.files[1].after, Some(FileValue { bytes: b"<!-- upskill:begin m -->\nA\n<!-- upskill:end m -->\n\n<!-- upskill:begin z -->\nA\n<!-- upskill:end z -->\n".to_vec(), mode: None }));
    assert_eq!(result.files[4].after, None);
    assert_eq!(result.files[5].after, None);
    let mut reversed_changes = changes.clone();
    reversed_changes.reverse();
    for change in &mut reversed_changes {
        if let InstallationChange::Set { artifacts, .. } = change {
            artifacts.reverse();
        }
    }
    let mut reversed_observations = observations.clone();
    reversed_observations.reverse();
    assert_eq!(
        plan_owned_artifacts(&prior, &reversed_changes, &reversed_observations).unwrap(),
        result
    );
    assert_eq!((prior, changes, observations), saved);
}
#[test]
fn c10_no_changes_preserve_all_interleaved_rows_and_require_no_observations() {
    let prior = vec![
        file_record("a", "z", None),
        instruction_record("b", "A.md", "a", ""),
        file_record("b", "c", None),
    ];
    let saved = prior.clone();
    let result = plan_owned_artifacts(&prior, &[], &[]).unwrap();
    assert_eq!(result.artifacts, saved);
    assert!(result.files.is_empty());
    assert_eq!(prior, saved);
}

// A revision update must reach every retained artifact, including both forms.
#[test]
fn c03_retained_mixed_forms_receive_coherent_revision_update() {
    let prior = mixed_prior();
    let mut binding = owner("a");
    binding.source_revision = Some("v2".into());
    let changes = vec![InstallationChange::Set {
        owner: binding.clone(),
        artifacts: vec![
            instruction("AGENTS.md", "a"),
            file("old-1", Some(0o755)),
            file("old-2", None),
        ],
    }];
    let observations = mixed_observations();
    let saved = (prior.clone(), changes.clone(), observations.clone());
    let result = plan_owned_artifacts(&prior, &changes, &observations).unwrap();
    assert_eq!(
        paths(&result.artifacts),
        vec!["AGENTS.md", "old-1", "old-2"]
    );
    for row in &result.artifacts {
        assert_eq!(row.owners, vec![binding.clone()]);
        assert_eq!(row.owners[0].source_revision.as_deref(), Some("v2"));
    }
    assert_eq!(result.files.len(), 3);
    for (action, observation) in result.files.iter().zip(&observations) {
        assert_eq!(action.before, observation.value);
        assert_eq!(action.after, observation.value);
    }
    assert_eq!((prior, changes, observations), saved);
}

// Removing every row cannot conceal a conflicting binding between the forms.
#[test]
fn c04_conflicting_prior_mixed_revisions_reject_global_removal() {
    let mut prior = vec![
        instruction_record("a", "AGENTS.md", "a", "Manual"),
        file_record("a", "data", None),
    ];
    prior[0].owners[0].source_revision = Some("v1".into());
    prior[1].owners[0].source_revision = Some("v2".into());
    rejects(
        &prior,
        &[remove("a")],
        &[
            observed("AGENTS.md", Some(AGGREGATE), Some(0o640)),
            observed("data", Some(b"abc"), None),
        ],
        "validate prior owned-artifact records",
    );
}

// Distinct payloads expose accidental reuse between artifacts in the same Set.
#[test]
fn c01_mixed_set_preserves_distinct_resource_and_selector_payloads() {
    let changes = vec![set(
        "multi",
        vec![
            DesiredArtifact::File(DesiredFile {
                location: location("resources/zeta.bin"),
                value: FileValue {
                    bytes: vec![128, 0, 254, 2],
                    mode: Some(0o600),
                },
                consumers: vec![TargetSurface::CopilotCli],
            }),
            DesiredArtifact::Instruction(DesiredInstructionContribution {
                location: InstructionLocation {
                    root: location("AGENTS.md").root,
                    path: "AGENTS.md".into(),
                },
                id: "zeta".into(),
                markdown: "Second rule\n".into(),
                consumers: vec![TargetSurface::ClaudeCli, TargetSurface::CopilotCli],
            }),
            DesiredArtifact::File(DesiredFile {
                location: location("resources/alpha.bin"),
                value: FileValue {
                    bytes: vec![0, 255, 1],
                    mode: Some(0o755),
                },
                consumers: vec![TargetSurface::ClaudeCli],
            }),
            DesiredArtifact::Instruction(DesiredInstructionContribution {
                location: InstructionLocation {
                    root: location("AGENTS.md").root,
                    path: "AGENTS.md".into(),
                },
                id: "alpha".into(),
                markdown: "First rule\n".into(),
                consumers: vec![TargetSurface::CodexCli],
            }),
        ],
    )];
    let observations = vec![
        observed("resources/zeta.bin", None, None),
        observed("AGENTS.md", None, None),
        observed("resources/alpha.bin", None, None),
    ];
    let saved = (changes.clone(), observations.clone());
    let result = plan_owned_artifacts(&[], &changes, &observations).unwrap();
    assert_eq!(result.files, vec![
        PlannedFile {
            location: location("AGENTS.md"), before: None,
            after: Some(FileValue {
                bytes: b"<!-- upskill:begin alpha -->\nFirst rule\n<!-- upskill:end alpha -->\n\n<!-- upskill:begin zeta -->\nSecond rule\n<!-- upskill:end zeta -->\n".to_vec(),
                mode: None,
            }),
        },
        PlannedFile {
            location: location("resources/alpha.bin"), before: None,
            after: Some(FileValue { bytes: vec![0, 255, 1], mode: Some(0o755) }),
        },
        PlannedFile {
            location: location("resources/zeta.bin"), before: None,
            after: Some(FileValue { bytes: vec![128, 0, 254, 2], mode: Some(0o600) }),
        },
    ]);
    assert_eq!(
        result.artifacts,
        vec![
            OwnedArtifact {
                root: location("AGENTS.md").root,
                path: "AGENTS.md".into(),
                content: ArtifactContent::MarkdownContribution {
                    contribution: aggregate::OwnedContribution {
                        id: "alpha".into(),
                        content_sha256:
                            "fecc2d0efc31c17c0910c49a258c6d81a7579ba289a6aab24554b949db153914"
                                .into(),
                        leading_newlines: 0,
                        preceding_newlines: 0,
                    },
                },
                owners: vec![owner("multi")],
                consumers: vec![TargetSurface::CodexCli],
            },
            OwnedArtifact {
                root: location("AGENTS.md").root,
                path: "AGENTS.md".into(),
                content: ArtifactContent::MarkdownContribution {
                    contribution: aggregate::OwnedContribution {
                        id: "zeta".into(),
                        content_sha256:
                            "c12055f0d3b9b5fd4ef73ac42b5737c0b83949ef79d23b3caa56aa0ea82ee123"
                                .into(),
                        leading_newlines: 1,
                        preceding_newlines: 1,
                    },
                },
                owners: vec![owner("multi")],
                consumers: vec![TargetSurface::ClaudeCli, TargetSurface::CopilotCli],
            },
            OwnedArtifact {
                root: location("resources/alpha.bin").root,
                path: "resources/alpha.bin".into(),
                content: ArtifactContent::File {
                    content_sha256:
                        "47ffa3ea45a70b8a41c2c0825df323c00a8b7a01c1ea06083cc41dddcc001123".into(),
                    mode: Some(0o755),
                },
                owners: vec![owner("multi")],
                consumers: vec![TargetSurface::ClaudeCli],
            },
            OwnedArtifact {
                root: location("resources/zeta.bin").root,
                path: "resources/zeta.bin".into(),
                content: ArtifactContent::File {
                    content_sha256:
                        "c09d2ee343e046f4abad68e24d5eb7b4578b2825ce4b9b57f73c55d45761da61".into(),
                    mode: Some(0o600),
                },
                owners: vec![owner("multi")],
                consumers: vec![TargetSurface::CopilotCli],
            },
        ]
    );
    let mut reversed = changes.clone();
    let InstallationChange::Set { artifacts, .. } = &mut reversed[0] else {
        unreachable!()
    };
    artifacts.reverse();
    assert_eq!(
        plan_owned_artifacts(&[], &reversed, &observations).unwrap(),
        result
    );
    assert_eq!((changes, observations), saved);
}
