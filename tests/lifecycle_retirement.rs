use sha2::{Digest, Sha256};
use upskill::files::plan::{FileLocation, FileValue, ObservedFile, PlannedFile};
use upskill::instructions::aggregate::{self, Contribution};
use upskill::lifecycle::retire::plan_reference_removal;
use upskill::lifecycle::state::{
    CombinedState, Installation, OwnerConsumers, Reference, SourceEvidence, validate_state,
};
use upskill::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn root() -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: std::env::temp_dir().join("upskill-retirement-example"),
    }
}
fn installation(id: &str, surface: TargetSurface) -> Installation {
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
        requested: vec![surface],
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
fn shared(instruction: bool) -> (CombinedState, ObservedFile) {
    let installations = vec![
        installation("a", TargetSurface::CodexCli),
        installation("b", TargetSurface::ClaudeCli),
    ];
    let (path, bytes, content) = if instruction {
        let edit =
            aggregate::reconcile("Manual\n", &[], &[Contribution::new("shared", "Shared\n")])
                .unwrap();
        (
            "AGENTS.md",
            edit.content.into_bytes(),
            ArtifactContent::MarkdownContribution {
                contribution: edit.owned[0].clone(),
            },
        )
    } else {
        (
            "resources/data.bin",
            vec![0, 255],
            ArtifactContent::File {
                content_sha256: Sha256::digest([0u8, 255])
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect(),
                mode: Some(0o640),
            },
        )
    };
    let state = CombinedState {
        schema: 2,
        references: vec![direct("a"), direct("b")],
        bundles: vec![],
        artifacts: vec![OwnedArtifact {
            root: root(),
            path: path.into(),
            content,
            owners: installations.iter().map(|i| i.owner.clone()).collect(),
            consumers: vec![TargetSurface::CodexCli, TargetSurface::ClaudeCli],
        }],
        owner_consumers: installations
            .iter()
            .map(|i| OwnerConsumers {
                root: root(),
                path: path.into(),
                contribution_id: instruction.then(|| "shared".into()),
                installation_id: i.owner.installation_id.clone(),
                consumers: i.requested.clone(),
            })
            .collect(),
        installations,
        legacy: None,
    };
    validate_state(&state).unwrap();
    (
        state,
        ObservedFile {
            location: FileLocation {
                root: root(),
                path: path.into(),
            },
            value: Some(FileValue {
                bytes,
                mode: Some(0o640),
            }),
        },
    )
}
fn reject(state: &CombinedState, remove: &[Reference], observations: &[ObservedFile]) -> String {
    let before = (state.clone(), remove.to_vec(), observations.to_vec());
    let error = plan_reference_removal(state, remove, observations).unwrap_err();
    assert_eq!(
        (state.clone(), remove.to_vec(), observations.to_vec()),
        before
    );
    format!("{error:#}")
}

#[test]
fn shared_whole_file_survives_until_final_owner_retires() {
    let (state, observed) = shared(false);
    let before = (state.clone(), observed.clone());
    let result = plan_reference_removal(&state, &[direct("a")], &[observed.clone()]).unwrap();
    validate_state(&result.state).unwrap();
    assert_eq!(
        result.state.installations,
        vec![state.installations[1].clone()]
    );
    assert_eq!(
        result.state.artifacts[0].owners,
        vec![state.installations[1].owner.clone()]
    );
    assert_eq!(
        result.state.artifacts[0].consumers,
        vec![TargetSurface::ClaudeCli]
    );
    assert_eq!(
        result.state.owner_consumers,
        vec![state.owner_consumers[1].clone()]
    );
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].before, observed.value);
    assert_eq!(result.files[0].after, observed.value);
    let final_plan =
        plan_reference_removal(&result.state, &[direct("b")], &[observed.clone()]).unwrap();
    assert!(final_plan.state.artifacts.is_empty());
    assert!(final_plan.state.owner_consumers.is_empty());
    assert_eq!(final_plan.files.len(), 1);
    assert_eq!(final_plan.files[0].before, observed.value);
    assert_eq!(final_plan.files[0].after, None);
    assert_eq!((state, observed), before);
}

#[test]
fn shared_region_retains_content_and_empty_aggregate_stays_present() {
    let (state, observed) = shared(true);
    let result = plan_reference_removal(&state, &[direct("a")], &[observed.clone()]).unwrap();
    assert_eq!(result.files[0].after, observed.value);
    assert_eq!(
        result.state.artifacts[0].consumers,
        vec![TargetSurface::ClaudeCli]
    );
    assert_eq!(
        result.state.artifacts[0].owners,
        vec![state.installations[1].owner.clone()]
    );
    let final_plan =
        plan_reference_removal(&result.state, &[direct("b")], &[observed.clone()]).unwrap();
    assert_eq!(
        final_plan.files[0].after,
        Some(FileValue {
            bytes: b"Manual\n".to_vec(),
            mode: Some(0o640)
        })
    );
    let mut empty = observed;
    let ArtifactContent::MarkdownContribution { contribution } = &state.artifacts[0].content else {
        unreachable!()
    };
    let edit = aggregate::reconcile("", &[], &[Contribution::new("shared", "Shared\n")]).unwrap();
    assert_eq!(contribution.id, "shared");
    let mut state = state;
    state.artifacts[0].content = ArtifactContent::MarkdownContribution {
        contribution: edit.owned[0].clone(),
    };
    empty.value.as_mut().unwrap().bytes = edit.content.into_bytes();
    let result = plan_reference_removal(&state, &[direct("a"), direct("b")], &[empty]).unwrap();
    assert_eq!(
        result.files[0].after,
        Some(FileValue {
            bytes: vec![],
            mode: Some(0o640)
        })
    );
}

#[test]
fn metadata_only_retirement_requires_unchanged_owned_evidence() {
    for instruction in [false, true] {
        let (state, observed) = shared(instruction);
        let mut edited = observed.clone();
        edited.value.as_mut().unwrap().bytes[0] ^= 1;
        if instruction {
            edited.value.as_mut().unwrap().bytes =
                String::from_utf8(observed.value.as_ref().unwrap().bytes.clone())
                    .unwrap()
                    .replace("Shared", "Edited")
                    .into_bytes();
        }
        assert!(!reject(&state, &[direct("a")], &[edited]).is_empty());
        assert!(!reject(&state, &[direct("a")], &[]).is_empty());
        assert!(
            !reject(
                &state,
                &[direct("a")],
                &[observed.clone(), observed.clone()]
            )
            .is_empty()
        );
        let mut absent = observed.clone();
        absent.value = None;
        assert!(!reject(&state, &[direct("a")], &[absent]).is_empty());
        let mut wrong = observed.clone();
        wrong.location.path = "other".into();
        assert!(!reject(&state, &[direct("a")], &[wrong]).is_empty());
        let mut wrong = observed.clone();
        wrong.location.root.scope = OwnershipScope::User;
        assert!(!reject(&state, &[direct("a")], &[wrong]).is_empty());
        let mut wrong = observed.clone();
        wrong.location.root.path.push(".");
        assert!(!reject(&state, &[direct("a")], &[wrong]).is_empty());
        let mut invalid = observed.clone();
        invalid.value.as_mut().unwrap().mode = Some(0o10000);
        assert!(!reject(&state, &[direct("a")], &[invalid]).is_empty());
        if !instruction {
            let mut mode = observed;
            mode.value.as_mut().unwrap().mode = Some(0o600);
            assert!(!reject(&state, &[direct("a")], &[mode]).is_empty());
        }
    }
}

#[test]
fn empty_installations_and_noop_graph_edits_do_not_invent_file_actions() {
    let (mut state, observed) = shared(false);
    state
        .installations
        .push(installation("empty", TargetSurface::CodexCli));
    state.references.push(direct("empty"));
    let result = plan_reference_removal(&state, &[], &[]).unwrap();
    assert_eq!(result.state, state);
    assert!(result.files.is_empty());
    let result = plan_reference_removal(&state, &[direct("empty")], &[]).unwrap();
    assert_eq!(result.state.artifacts, state.artifacts);
    assert!(result.files.is_empty());
    assert_eq!(result.state.installations.len(), 2);
    assert!(!reject(&state, &[], &[observed]).is_empty());
    let mut invalid = state;
    invalid.owner_consumers.pop();
    assert!(!reject(&invalid, &[direct("empty")], &[]).is_empty());
}

#[test]
fn unsupported_owned_kind_blocks_retirement_but_survives_disjoint_work() {
    for kind in [ItemKind::Skill, ItemKind::Agent] {
        let (mut state, observed) = shared(false);
        state
            .installations
            .push(installation("skill", TargetSurface::CodexCli));
        state.installations.last_mut().unwrap().owner.kind = kind;
        state.references.push(direct("skill"));
        validate_state(&state).unwrap();
        assert!(reject(&state, &[direct("skill")], &[]).contains("unsupported"));
        let result = plan_reference_removal(&state, &[direct("a")], &[observed.clone()]).unwrap();
        assert!(result.state.installations.contains(&state.installations[2]));
    }
}

#[test]
fn multiple_regions_retire_in_one_file_without_touching_manual_or_retained_region() {
    let (mut state, mut observed) = shared(true);
    state
        .installations
        .push(installation("c", TargetSurface::CodexCli));
    state.references.push(direct("c"));
    let edit = aggregate::reconcile(
        "Manual prefix\n",
        &[],
        &[
            Contribution::new("a", "A\n"),
            Contribution::new("b", "B\n"),
            Contribution::new("c", "C\n"),
        ],
    )
    .unwrap();
    observed.value.as_mut().unwrap().bytes =
        format!("{}Manual suffix\n", edit.content).into_bytes();
    state.artifacts.clear();
    state.owner_consumers.clear();
    for (index, id) in ["a", "b", "c"].iter().enumerate() {
        let owner = state.installations[index].owner.clone();
        let consumers = state.installations[index].requested.clone();
        state.artifacts.push(OwnedArtifact {
            root: root(),
            path: "AGENTS.md".into(),
            content: ArtifactContent::MarkdownContribution {
                contribution: edit.owned.iter().find(|row| &row.id == id).unwrap().clone(),
            },
            owners: vec![owner],
            consumers: consumers.clone(),
        });
        state.owner_consumers.push(OwnerConsumers {
            root: root(),
            path: "AGENTS.md".into(),
            contribution_id: Some((*id).into()),
            installation_id: (*id).into(),
            consumers,
        });
    }
    validate_state(&state).unwrap();
    let result =
        plan_reference_removal(&state, &[direct("b"), direct("a")], &[observed.clone()]).unwrap();
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].before, observed.value);
    assert_eq!(result.files[0].after, Some(FileValue { bytes: b"Manual prefix\n\n<!-- upskill:begin c -->\nC\n<!-- upskill:end c -->\nManual suffix\n".to_vec(), mode: Some(0o640) }));
    assert_eq!(result.state.artifacts.len(), 1);
    assert_eq!(
        result.state.artifacts[0].owners,
        vec![state.installations[2].owner.clone()]
    );
    let after = result.files[0].after.as_ref().unwrap();
    let ArtifactContent::MarkdownContribution { contribution } = &result.state.artifacts[0].content
    else {
        unreachable!()
    };
    assert_eq!(
        aggregate::inspect_owned(
            std::str::from_utf8(&after.bytes).unwrap(),
            &[contribution.clone()]
        )
        .unwrap(),
        vec![Contribution::new("c", "C\n")]
    );
    let mut edited = observed;
    edited.value.as_mut().unwrap().bytes =
        String::from_utf8(edited.value.as_ref().unwrap().bytes.clone())
            .unwrap()
            .replace("C\n", "Changed\n")
            .into_bytes();
    assert!(!reject(&state, &[direct("a")], &[edited]).is_empty());
}

#[test]
fn retained_bundle_path_delays_retirement_and_preserves_legacy_rows() {
    use upskill::lifecycle::state::BundleNode;
    let (mut state, observed) = shared(false);
    state.bundles = ["left", "right"]
        .iter()
        .map(|id| BundleNode {
            id: (*id).into(),
            source: format!("./{id}"),
            name: (*id).into(),
            source_revision: None,
        })
        .collect();
    state.references = vec![direct("b")];
    for id in ["left", "right"] {
        state.references.push(Reference::DirectBundle {
            bundle_id: id.into(),
        });
        state.references.push(Reference::BundleInstallation {
            bundle_id: id.into(),
            installation_id: "a".into(),
        });
    }
    state.legacy = Some(upskill::lockfile::Lockfile {
        schema: 1,
        items: vec![],
        bundles: vec![],
        plugins: vec![],
        mcps: vec![],
    });
    let legacy_json = serde_json::json!({"kind":"rule","name":"old","source":"malformed old source label","required_by":[]});
    let row: upskill::lockfile::LockedItem = serde_json::from_value(legacy_json).unwrap();
    let mut other = row.clone();
    other.name = "different legacy item".into();
    other.source = "another untouched source".into();
    state.legacy.as_mut().unwrap().items = vec![row.clone(), row, other];
    let left = Reference::DirectBundle {
        bundle_id: "left".into(),
    };
    let right = Reference::DirectBundle {
        bundle_id: "right".into(),
    };
    let first = plan_reference_removal(&state, &[left], &[]).unwrap();
    assert_eq!(first.state.artifacts, state.artifacts);
    assert_eq!(first.state.owner_consumers, state.owner_consumers);
    assert_eq!(first.state.legacy, state.legacy);
    assert!(first.files.is_empty());
    let second = plan_reference_removal(&first.state, &[right], &[observed.clone()]).unwrap();
    assert_eq!(second.state.legacy, state.legacy);
    assert_eq!(
        second.state.artifacts[0].owners,
        vec![state.installations[1].owner.clone()]
    );
    assert_eq!(second.files[0].after, observed.value);
}

#[test]
fn affected_topology_conflicts_reject_before_planning() {
    let (mut state, observed) = shared(false);
    let mut child = state.artifacts[0].clone();
    child.path.push_str("/child");
    state.artifacts.push(child);
    let mut associations = state.owner_consumers.clone();
    for association in &mut associations {
        association.path.push_str("/child");
    }
    state.owner_consumers.extend(associations);
    validate_state(&state).unwrap();
    let mut child_observation = observed.clone();
    child_observation.location.path.push_str("/child");
    assert!(reject(&state, &[direct("a")], &[observed, child_observation]).contains("topology"));
}

#[test]
fn manual_edits_and_observed_aggregate_mode_are_preserved() {
    let (state, mut observed) = shared(true);
    let text = String::from_utf8(observed.value.as_ref().unwrap().bytes.clone()).unwrap();
    observed.value.as_mut().unwrap().bytes = text
        .replace("Manual", "Locally edited manual text")
        .into_bytes();
    observed.value.as_mut().unwrap().mode = Some(0o600);
    let result = plan_reference_removal(&state, &[direct("a")], &[observed.clone()]).unwrap();
    assert_eq!(result.files[0].after, observed.value);
    let mut malformed = observed;
    malformed
        .value
        .as_mut()
        .unwrap()
        .bytes
        .extend_from_slice(b"<!-- upskill:begin stranger -->\nX\n<!-- upskill:end stranger -->\n");
    assert!(!reject(&state, &[direct("a")], &[malformed]).is_empty());
}

#[test]
fn overlapping_consumer_associations_and_absent_mode_remain_precise() {
    let (mut state, mut observed) = shared(false);
    state
        .installations
        .push(installation("c", TargetSurface::CodexCli));
    state.references.push(direct("c"));
    state.artifacts[0]
        .owners
        .push(state.installations[2].owner.clone());
    let mut association = state.owner_consumers[0].clone();
    association.installation_id = "c".into();
    state.owner_consumers.push(association);
    let ArtifactContent::File { mode, .. } = &mut state.artifacts[0].content else {
        unreachable!()
    };
    *mode = None;
    observed.value.as_mut().unwrap().mode = None;
    validate_state(&state).unwrap();
    let result = plan_reference_removal(&state, &[direct("a")], &[observed.clone()]).unwrap();
    assert_eq!(
        result.state.artifacts[0].consumers,
        state.artifacts[0].consumers
    );
    assert_eq!(
        result.state.artifacts[0].owners,
        state.artifacts[0].owners[1..]
    );
    assert_eq!(result.files[0].after, observed.value);
    let mut changed = observed;
    changed.value.as_mut().unwrap().mode = Some(0o600);
    assert!(reject(&state, &[direct("a")], &[changed]).contains("mode"));
}

#[test]
fn later_file_conflict_returns_no_partial_plan() {
    let (mut state, observed) = shared(false);
    let mut other = state.artifacts[0].clone();
    other.path = "z.bin".into();
    state.artifacts.push(other);
    let mut associations = state.owner_consumers.clone();
    for association in &mut associations {
        association.path = "z.bin".into();
    }
    state.owner_consumers.extend(associations);
    let mut other = observed.clone();
    other.location.path = "z.bin".into();
    other.value.as_mut().unwrap().bytes.push(1);
    assert!(reject(&state, &[direct("a"), direct("b")], &[observed, other]).contains("bytes"));
}

#[test]
fn affected_paths_conflict_with_untouched_ancestors_and_descendants() {
    for retire_child in [false, true] {
        let (mut state, mut observed) = shared(false);
        state.artifacts[0].owners = vec![state.installations[0].owner.clone()];
        state.artifacts[0].consumers = vec![TargetSurface::CodexCli];
        state
            .owner_consumers
            .retain(|row| row.installation_id == "a");
        let mut child = state.artifacts[0].clone();
        child.path.push_str("/child");
        child.owners = vec![state.installations[1].owner.clone()];
        child.consumers = vec![TargetSurface::ClaudeCli];
        state.artifacts.push(child.clone());
        state.owner_consumers.push(OwnerConsumers {
            root: root(),
            path: child.path.clone(),
            contribution_id: None,
            installation_id: "b".into(),
            consumers: vec![TargetSurface::ClaudeCli],
        });
        validate_state(&state).unwrap();
        if retire_child {
            observed.location.path = child.path;
        }
        let id = if retire_child { "b" } else { "a" };
        assert!(reject(&state, &[direct(id)], &[observed]).contains("topology"));
    }
}

#[test]
fn complete_actions_keep_destinations_and_retained_artifact_order() {
    let (mut state, prototype) = shared(false);
    let original = state.artifacts[0].clone();
    let original_associations = state.owner_consumers.clone();
    state.artifacts.clear();
    state.owner_consumers.clear();
    let mut observations = Vec::new();
    for path in ["z.bin", "a.bin", "m.bin"] {
        let mut row = original.clone();
        row.path = path.into();
        state.artifacts.push(row);
        for association in &original_associations {
            let mut association = association.clone();
            association.path = path.into();
            state.owner_consumers.push(association);
        }
        let mut observation = prototype.clone();
        observation.location.path = path.into();
        observations.push(observation);
    }
    let partial = plan_reference_removal(&state, &[direct("a")], &observations).unwrap();
    let mut expected_artifacts = state.artifacts.clone();
    for row in &mut expected_artifacts {
        row.owners.remove(0);
        row.consumers = vec![TargetSurface::ClaudeCli];
    }
    assert_eq!(partial.state.artifacts, expected_artifacts);
    let final_plan =
        plan_reference_removal(&state, &[direct("a"), direct("b")], &observations).unwrap();
    let expected: Vec<_> = ["a.bin", "m.bin", "z.bin"]
        .into_iter()
        .map(|path| {
            let observation = observations
                .iter()
                .find(|o| o.location.path == path)
                .unwrap();
            PlannedFile {
                location: observation.location.clone(),
                before: observation.value.clone(),
                after: None,
            }
        })
        .collect();
    assert_eq!(final_plan.files, expected);
    let expected: Vec<_> = expected
        .into_iter()
        .map(|mut file| {
            file.after = file.before.clone();
            file
        })
        .collect();
    assert_eq!(partial.files, expected);
}

#[test]
fn same_named_regions_at_other_locations_do_not_retain_retired_content() {
    let (mut state, observed) = shared(true);
    state.artifacts[0].owners = vec![state.installations[0].owner.clone()];
    state.artifacts[0].consumers = vec![TargetSurface::CodexCli];
    state
        .owner_consumers
        .retain(|row| row.installation_id == "a");
    let edit = aggregate::reconcile(
        "Other manual\n",
        &[],
        &[Contribution::new("shared", "Other body\n")],
    )
    .unwrap();
    let other = OwnedArtifact {
        root: root(),
        path: "other.md".into(),
        content: ArtifactContent::MarkdownContribution {
            contribution: edit.owned[0].clone(),
        },
        owners: vec![state.installations[1].owner.clone()],
        consumers: vec![TargetSurface::ClaudeCli],
    };
    state.artifacts.push(other.clone());
    state.owner_consumers.push(OwnerConsumers {
        root: root(),
        path: "other.md".into(),
        contribution_id: Some("shared".into()),
        installation_id: "b".into(),
        consumers: vec![TargetSurface::ClaudeCli],
    });
    validate_state(&state).unwrap();
    let result = plan_reference_removal(&state, &[direct("a")], &[observed.clone()]).unwrap();
    assert_eq!(result.state.artifacts, vec![other]);
    assert_eq!(
        result.files,
        vec![PlannedFile {
            location: observed.location,
            before: observed.value,
            after: Some(FileValue {
                bytes: b"Manual\n".to_vec(),
                mode: Some(0o640)
            })
        }]
    );
}
