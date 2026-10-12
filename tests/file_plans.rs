use upskill::files::plan::{
    DesiredFile, FileInstallationChange, FileLocation, FileValue, ObservedFile, plan_files,
};
use upskill::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn location(path: &str) -> FileLocation {
    FileLocation {
        root: InstallationRoot {
            scope: OwnershipScope::Project,
            path: std::env::temp_dir().join("upskill-file-plan"),
        },
        path: path.to_owned(),
    }
}
fn owner(id: &str) -> ArtifactOwner {
    ArtifactOwner {
        installation_id: id.to_owned(),
        source: "local:./content".to_owned(),
        source_revision: None,
        kind: ItemKind::Skill,
        source_name: "source-demo".to_owned(),
        installed_name: "demo".to_owned(),
    }
}
fn desired(path: &str, bytes: &[u8]) -> DesiredFile {
    DesiredFile {
        location: location(path),
        value: FileValue {
            bytes: bytes.to_vec(),
            mode: None,
        },
        consumers: vec![TargetSurface::ClaudeCli],
    }
}

#[test]
fn creates_sorted_entrypoint_and_binary_resource_with_literal_evidence() {
    let entry = desired("demo/SKILL.md", b"abc");
    let resource = desired("demo/assets/data.bin", &[0, 255]);
    let observations = vec![
        ObservedFile {
            location: resource.location.clone(),
            value: None,
        },
        ObservedFile {
            location: entry.location.clone(),
            value: None,
        },
    ];
    let changes = vec![FileInstallationChange::Set {
        owner: owner("install-a"),
        files: vec![resource.clone(), entry.clone()],
    }];
    let before = (changes.clone(), observations.clone());
    let plan = plan_files(&[], &changes, &observations).unwrap();
    assert_eq!(plan.files.len(), 2);
    assert_eq!(plan.artifacts.len(), 2);
    for (index, expected) in [entry, resource].iter().enumerate() {
        assert_eq!(plan.files[index].location, expected.location);
        assert_eq!(plan.files[index].before, None);
        assert_eq!(plan.files[index].after, Some(expected.value.clone()));
        assert_eq!(
            plan.artifacts[index].root.path.as_os_str(),
            expected.location.root.path.as_os_str()
        );
        assert_eq!(plan.artifacts[index].path, expected.location.path);
        assert_eq!(plan.artifacts[index].owners, vec![owner("install-a")]);
        assert_eq!(
            plan.artifacts[index].consumers,
            vec![TargetSurface::ClaudeCli]
        );
    }
    assert_eq!(
        plan.artifacts[0].content,
        ArtifactContent::File {
            content_sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .to_owned(),
            mode: None,
        }
    );
    assert_eq!(
        plan.artifacts[1].content,
        ArtifactContent::File {
            content_sha256: "06eb7d6a69ee19e5fbdf749018d3d2abfa04bcbd1365db312eb86dc7169389b8"
                .to_owned(),
            mode: None,
        }
    );
    assert_eq!((changes, observations), before);
}

#[test]
fn rejects_identical_unowned_file() {
    let file = desired("demo/SKILL.md", b"abc");
    let observed = ObservedFile {
        location: file.location.clone(),
        value: Some(file.value.clone()),
    };
    let changes = [FileInstallationChange::Set {
        owner: owner("install-a"),
        files: vec![file],
    }];
    let result = plan_files(&[], &changes, &[observed]);
    assert!(result.is_err(), "unowned equal-byte file must conflict");
    assert!(format!("{:#}", result.unwrap_err()).contains("unowned"));
}

fn set(id: &str, files: Vec<DesiredFile>) -> FileInstallationChange {
    FileInstallationChange::Set {
        owner: owner(id),
        files,
    }
}
fn remove(id: &str) -> FileInstallationChange {
    FileInstallationChange::Remove {
        installation_id: id.into(),
    }
}
fn observe(file: &DesiredFile, present: bool) -> ObservedFile {
    ObservedFile {
        location: file.location.clone(),
        value: present.then(|| file.value.clone()),
    }
}
fn record(id: &str, file: &DesiredFile) -> OwnedArtifact {
    use sha2::{Digest, Sha256};
    OwnedArtifact {
        root: file.location.root.clone(),
        path: file.location.path.clone(),
        content: ArtifactContent::File {
            content_sha256: Sha256::digest(&file.value.bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            mode: file.value.mode,
        },
        owners: vec![owner(id)],
        consumers: file.consumers.clone(),
    }
}
fn contribution(id: &str, path: &str) -> OwnedArtifact {
    OwnedArtifact {
        content: ArtifactContent::MarkdownContribution {
            contribution: upskill::instructions::aggregate::OwnedContribution {
                id: id.into(),
                content_sha256: "0".repeat(64),
                leading_newlines: 1,
                preceding_newlines: 0,
            },
        },
        ..record(id, &desired(path, b"manual"))
    }
}
fn rejects(
    prior: &[OwnedArtifact],
    changes: &[FileInstallationChange],
    observations: &[ObservedFile],
    diagnostic: &str,
) {
    let result = plan_files(prior, changes, observations);
    assert!(
        result.is_err(),
        "expected rejection mentioning {diagnostic}, got {result:?}"
    );
    let error = format!("{:#}", result.unwrap_err());
    assert!(
        error.contains(diagnostic),
        "expected {diagnostic:?} in {error:?}"
    );
}

#[test]
fn updates_bytes_and_then_removes_only_recorded_file() {
    let mut old = desired("demo/SKILL.md", b"abc");
    old.value.mode = Some(0o640);
    let mut new = desired("demo/SKILL.md", b"changed");
    new.value.mode = Some(0o700);
    let unrelated = contribution("other", "AGENTS.md");
    let prior = vec![record("a", &old), unrelated.clone()];
    let plan = plan_files(
        &prior,
        &[set("a", vec![new.clone()])],
        &[observe(&old, true)],
    )
    .unwrap();
    assert_eq!(
        plan.files,
        vec![upskill::files::plan::PlannedFile {
            location: old.location.clone(),
            before: Some(old.value.clone()),
            after: Some(new.value.clone()),
        }]
    );
    assert_eq!(
        plan.artifacts[0].content,
        ArtifactContent::File {
            content_sha256: "d67e2e944994496c8d8ec76eed0cf9f09679448d584b532bebf941852a37f5ed"
                .into(),
            mode: Some(0o700),
        }
    );
    assert_eq!(plan.artifacts[0].owners, vec![owner("a")]);
    assert_eq!(plan.artifacts[1], unrelated);
    let removed = plan_files(&plan.artifacts, &[remove("a")], &[observe(&new, true)]).unwrap();
    assert_eq!(
        removed.files,
        vec![upskill::files::plan::PlannedFile {
            location: new.location,
            before: Some(new.value),
            after: None,
        }]
    );
    assert_eq!(removed.artifacts, vec![unrelated]);
}

#[test]
fn revision_only_update_preserves_exact_values_and_changes_all_bindings() {
    let mut first = desired("z", b"abc");
    first.value.mode = Some(0);
    let second = desired("a", &[0, 255]);
    let prior = vec![record("a", &first), record("a", &second)];
    let mut binding = owner("a");
    binding.source_revision = Some("revision-2".into());
    let changes = vec![FileInstallationChange::Set {
        owner: binding.clone(),
        files: vec![first.clone(), second.clone()],
    }];
    let observations = vec![observe(&first, true), observe(&second, true)];
    let snapshot = (prior.clone(), changes.clone(), observations.clone());
    let plan = plan_files(&prior, &changes, &observations).unwrap();
    assert_eq!(plan.files.len(), 2);
    for (action, file) in plan.files.iter().zip([&second, &first]) {
        assert_eq!(action.before, Some(file.value.clone()));
        assert_eq!(action.after, Some(file.value.clone()));
    }
    for row in &plan.artifacts {
        assert_eq!(row.owners, vec![binding.clone()]);
    }
    assert_eq!(binding.source, "local:./content");
    assert_eq!(binding.source_name, "source-demo");
    assert_eq!(binding.installed_name, "demo");
    assert_eq!(binding.kind, ItemKind::Skill);
    assert_eq!((prior, changes, observations), snapshot);
}

#[test]
fn stable_installation_fields_each_reject_changes_but_new_identity_may_differ() {
    let file = desired("old", b"abc");
    let prior = [record("a", &file)];
    for (index, diagnostic) in ["source", "kind", "source_name", "installed_name"]
        .iter()
        .enumerate()
    {
        let mut binding = owner("a");
        match index {
            0 => binding.source = "local:./other".into(),
            1 => binding.kind = ItemKind::Rule,
            2 => binding.source_name = "other-source".into(),
            3 => binding.installed_name = "other-alias".into(),
            _ => unreachable!(),
        }
        rejects(
            &prior,
            &[FileInstallationChange::Set {
                owner: binding,
                files: vec![file.clone()],
            }],
            &[observe(&file, true)],
            diagnostic,
        );
    }
    let new = desired("new", b"abc");
    let mut binding = owner("b");
    binding.source_name = "new-source".into();
    binding.installed_name = "new-alias".into();
    let plan = plan_files(
        &prior,
        &[FileInstallationChange::Set {
            owner: binding.clone(),
            files: vec![new.clone()],
        }],
        &[observe(&new, false)],
    )
    .unwrap();
    assert_eq!(plan.artifacts[0], prior[0]);
    assert_eq!(plan.artifacts[1].owners, vec![binding]);
}

#[test]
fn complete_set_replacement_preserves_untouched_records_and_order() {
    let old = desired("old", b"abc");
    let new = desired("new", &[0, 255]);
    let mut shared = record("b", &desired("z", b"shared"));
    shared.owners.push(owner("c"));
    shared.consumers = vec![TargetSurface::CopilotCli, TargetSurface::ClaudeCli];
    let manual = contribution("manual", "AGENTS.md");
    let other = record("d", &desired("a", b"other"));
    let prior = vec![
        shared.clone(),
        record("a", &old),
        manual.clone(),
        other.clone(),
    ];
    let plan = plan_files(
        &prior,
        &[set("a", vec![new.clone()])],
        &[observe(&old, true), observe(&new, false)],
    )
    .unwrap();
    assert_eq!(
        plan.artifacts,
        vec![shared, manual, other, record("a", &new)]
    );
    assert_eq!(
        plan.files,
        vec![
            upskill::files::plan::PlannedFile {
                location: new.location.clone(),
                before: None,
                after: Some(new.value.clone())
            },
            upskill::files::plan::PlannedFile {
                location: old.location,
                before: Some(old.value),
                after: None
            },
        ]
    );
}

#[test]
fn same_location_replacement_keeps_prior_position_and_explicit_consumers() {
    let old = desired("z", b"abc");
    let mut new = old.clone();
    new.consumers = vec![TargetSurface::CopilotCli, TargetSurface::CodexCli];
    let prior = vec![record("a", &old), record("b", &desired("a", b"untouched"))];
    let plan = plan_files(
        &prior,
        &[set("a", vec![new.clone()])],
        &[observe(&old, true)],
    )
    .unwrap();
    assert_eq!(plan.artifacts, vec![record("a", &new), prior[1].clone()]);
    assert_eq!(plan.files[0].before, Some(old.value.clone()));
    assert_eq!(plan.files[0].after, Some(old.value));
}

#[test]
fn affected_shared_files_and_contributions_reject_set_and_removal() {
    let file = desired("shared", b"abc");
    let mut shared = record("a", &file);
    shared.owners.push(owner("b"));
    let manual = contribution("a", "shared");
    for (prior, diagnostic) in [(shared, "multi-owner"), (manual, "contribution")] {
        for change in [set("a", vec![file.clone()]), remove("a")] {
            rejects(
                std::slice::from_ref(&prior),
                &[change],
                &[observe(&file, true)],
                diagnostic,
            );
        }
        rejects(
            std::slice::from_ref(&prior),
            &[set("c", vec![file.clone()])],
            &[observe(&file, true)],
            diagnostic,
        );
    }
}

#[test]
fn ownership_handoff_requires_selected_old_owner_and_verified_before() {
    let old = desired("shared", b"abc");
    let new = desired("shared", b"changed");
    let prior = [record("a", &old)];
    rejects(
        &prior,
        &[set("b", vec![new.clone()])],
        &[observe(&old, true)],
        "untouched owner",
    );
    let changes = [remove("a"), set("b", vec![new.clone()])];
    let plan = plan_files(&prior, &changes, &[observe(&old, true)]).unwrap();
    assert_eq!(plan.artifacts, vec![record("b", &new)]);
    assert_eq!(plan.files[0].before, Some(old.value));
    assert_eq!(plan.files[0].after, Some(new.value.clone()));
    rejects(&prior, &changes, &[observe(&new, true)], "bytes");
}

#[test]
fn rejects_different_unowned_bytes_even_with_matching_mode() {
    let mut file = desired("new", b"abc");
    file.value.mode = Some(0o640);
    let mut observed = observe(&file, true);
    observed.value.as_mut().unwrap().bytes = b"manual".to_vec();
    rejects(&[], &[set("a", vec![file])], &[observed], "unowned");
}

#[test]
fn edited_owned_bytes_reject_update_remove_and_noop() {
    let old = desired("owned", b"abc");
    let new = desired("owned", b"changed");
    let prior = [record("a", &old)];
    for change in [
        set("a", vec![new.clone()]),
        remove("a"),
        set("a", vec![old.clone()]),
    ] {
        rejects(&prior, &[change], &[observe(&new, true)], "bytes");
    }
}

#[test]
fn missing_owned_bytes_reject_update_remove_and_noop() {
    let old = desired("owned", b"abc");
    let new = desired("owned", b"changed");
    let prior = [record("a", &old)];
    for change in [
        set("a", vec![new]),
        remove("a"),
        set("a", vec![old.clone()]),
    ] {
        rejects(
            &prior,
            &[change],
            &[observe(&old, false)],
            "missing owned file",
        );
    }
}

fn rejects_mode_evidence(recorded: Option<u32>, observed: Option<u32>) {
    let mut old = desired("mode", b"abc");
    old.value.mode = recorded;
    let prior = [record("a", &old)];
    let mut observation = observe(&old, true);
    observation.value.as_mut().unwrap().mode = observed;
    for change in [set("a", vec![old.clone()]), remove("a")] {
        rejects(
            &prior,
            &[change],
            std::slice::from_ref(&observation),
            "mode",
        );
    }
}
#[test]
fn missing_recorded_mode_differs_from_observed_zero() {
    rejects_mode_evidence(None, Some(0));
}
#[test]
fn recorded_zero_differs_from_missing_observed_mode() {
    rejects_mode_evidence(Some(0), None);
}
#[test]
fn changed_observed_mode_rejects() {
    rejects_mode_evidence(Some(0o640), Some(0o600));
}

#[test]
fn permission_boundary_modes_roundtrip_exactly() {
    for mode in [0, 0o7777] {
        let mut file = desired("mode", b"abc");
        file.value.mode = Some(mode);
        let created = plan_files(
            &[],
            &[set("a", vec![file.clone()])],
            &[observe(&file, false)],
        )
        .unwrap();
        assert_eq!(created.artifacts, vec![record("a", &file)]);
        assert_eq!(created.files[0].after, Some(file.value.clone()));
        let noop = plan_files(
            &created.artifacts,
            &[set("a", vec![file.clone()])],
            &[observe(&file, true)],
        )
        .unwrap();
        assert_eq!(noop.files[0].before, Some(file.value.clone()));
        assert_eq!(noop.files[0].after, Some(file.value));
    }
}

#[test]
fn empty_binary_and_unicode_values_remain_exact() {
    let files = vec![
        desired("é/技能.md", b""),
        desired("binary", &[0, 255, 128, 13, 10]),
    ];
    let plan = plan_files(
        &[],
        &[set("a", files.clone())],
        &files.iter().map(|f| observe(f, false)).collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(plan.files[1].location.path, "é/技能.md");
    assert_eq!(plan.files[1].after, Some(files[0].value.clone()));
    assert_eq!(plan.files[0].after, Some(files[1].value.clone()));
    assert_eq!(
        plan.artifacts[1].content,
        ArtifactContent::File {
            content_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .into(),
            mode: None,
        }
    );
}

#[test]
fn observations_are_required_unique_and_exact_even_for_noop() {
    let file = desired("owned", b"abc");
    let prior = [record("a", &file)];
    let changes = [set("a", vec![file.clone()])];
    rejects(&prior, &changes, &[], "missing observation");
    let observation = observe(&file, true);
    rejects(
        &prior,
        &changes,
        &[observation.clone(), observation.clone()],
        "duplicate observation",
    );
    rejects(
        &prior,
        &changes,
        &[observation, observe(&desired("extra", b""), false)],
        "extraneous observation",
    );
    rejects(&[], &[], &[observe(&file, false)], "extraneous observation");
}

#[test]
fn invalid_observed_locations_and_modes_precede_membership_errors() {
    for path in [
        "",
        "../bad",
        "a/./b",
        "a//b",
        "/absolute",
        "a\\b",
        "a:b",
        "a\n",
    ] {
        let invalid = desired(path, b"");
        rejects(&[], &[], &[observe(&invalid, false)], "portable");
    }
    for root in ["", "relative"] {
        let mut invalid = desired("file", b"");
        invalid.location.root.path = root.into();
        rejects(&[], &[], &[observe(&invalid, false)], "absolute");
    }
    let mut invalid = desired("file", b"");
    invalid.value.mode = Some(0o10000);
    rejects(&[], &[], &[observe(&invalid, true)], "permission bits");
}

#[test]
fn invalid_desired_metadata_locations_and_modes_reject_before_missing_observation() {
    for (case, diagnostic) in [
        (0, "consumer"),
        (1, "duplicate consumer"),
        (2, "installation id"),
        (3, "source"),
        (4, "source"),
        (5, "source"),
        (6, "revision"),
        (7, "permission bits"),
        (8, "portable"),
        (9, "absolute"),
    ] {
        let mut binding = owner("a");
        let mut file = desired("file", b"abc");
        match case {
            0 => file.consumers.clear(),
            1 => file.consumers.push(TargetSurface::ClaudeCli),
            2 => binding.installation_id = "bad id".into(),
            3 => binding.source.clear(),
            4 => binding.source_name.clear(),
            5 => binding.installed_name.clear(),
            6 => binding.source_revision = Some(String::new()),
            7 => file.value.mode = Some(0o10000),
            8 => file.location.path = "../bad".into(),
            9 => file.location.root.path = "relative".into(),
            _ => unreachable!(),
        }
        rejects(
            &[],
            &[FileInstallationChange::Set {
                owner: binding,
                files: vec![file],
            }],
            &[],
            diagnostic,
        );
    }
}

#[test]
fn invalid_prior_hash_or_conflicting_revision_rejects_without_changes() {
    let file = desired("file", b"abc");
    let mut invalid = record("a", &file);
    invalid.content = ArtifactContent::File {
        content_sha256: "invalid".into(),
        mode: None,
    };
    rejects(&[invalid], &[], &[], "SHA-256");
    let mut other = record("a", &desired("other", b"abc"));
    other.owners[0].source_revision = Some("revision-2".into());
    rejects(&[record("a", &file), other], &[], &[], "binding");
}

#[test]
fn duplicate_locations_changes_empty_sets_and_unknown_removals_reject() {
    let file = desired("file", b"abc");
    rejects(
        &[],
        &[set("a", vec![file.clone(), file.clone()])],
        &[],
        "duplicate desired location",
    );
    rejects(
        &[],
        &[set("a", vec![file.clone()]), set("b", vec![file.clone()])],
        &[],
        "duplicate desired location",
    );
    rejects(
        &[],
        &[
            set("a", vec![file.clone()]),
            set("a", vec![desired("other", b"abc")]),
        ],
        &[],
        "duplicate file change",
    );
    rejects(
        &[record("a", &file)],
        &[set("a", vec![file.clone()]), remove("a")],
        &[],
        "duplicate file change",
    );
    rejects(&[], &[set("a", vec![])], &[], "empty");
    rejects(&[], &[remove("unknown")], &[], "unknown installation");
}

#[test]
fn raw_root_spellings_distinct_roots_and_scopes_remain_distinct() {
    let normal = desired("file", b"abc");
    let mut lexical = normal.clone();
    let mut raw = normal.location.root.path.as_os_str().to_owned();
    raw.push("/.");
    lexical.location.root.path = raw.into();
    assert_ne!(
        normal.location.root.path.as_os_str(),
        lexical.location.root.path.as_os_str()
    );
    let mut distinct = normal.clone();
    distinct.location.root.path.push("other");
    let mut user = normal.clone();
    user.location.root.scope = OwnershipScope::User;
    let files = vec![
        user.clone(),
        lexical.clone(),
        distinct.clone(),
        normal.clone(),
    ];
    let plan = plan_files(
        &[],
        &[set("a", files.clone())],
        &files.iter().map(|f| observe(f, false)).collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(plan.files.len(), 4);
    for (action, expected) in plan.files.iter().zip([normal, lexical, distinct, user]) {
        assert_eq!(
            action.location.root.path.as_os_str(),
            expected.location.root.path.as_os_str()
        );
        assert_eq!(action.location.root.scope, expected.location.root.scope);
        assert_eq!(action.after, Some(expected.value));
    }
}

#[test]
fn ancestor_conflicts_include_prior_desired_contributions_and_removal_transitions() {
    for (prior_path, desired_path) in [("a", "a/b"), ("a/b", "a")] {
        let old = desired(prior_path, b"abc");
        let new = desired(desired_path, b"abc");
        rejects(
            &[record("a", &old)],
            &[set("b", vec![new.clone()])],
            &[observe(&new, false)],
            "topology",
        );
        rejects(
            &[record("a", &old)],
            &[remove("a"), set("b", vec![new.clone()])],
            &[observe(&old, true), observe(&new, false)],
            "topology",
        );
    }
    let new = desired("a", b"abc");
    rejects(
        &[contribution("manual", "a/b")],
        &[set("a", vec![new.clone()])],
        &[observe(&new, false)],
        "topology",
    );
    let files = vec![desired("a", b"abc"), desired("a/b", b"abc")];
    rejects(
        &[],
        &[set("a", files.clone())],
        &files.iter().map(|f| observe(f, false)).collect::<Vec<_>>(),
        "topology",
    );
}

#[test]
fn component_prefix_siblings_and_disjoint_untouched_topology_are_allowed() {
    let first = desired("a", b"abc");
    let second = desired("ab", b"abc");
    let prior = vec![
        record("a", &first),
        record("x", &desired("old", b"abc")),
        contribution("manual", "old/child"),
    ];
    let plan = plan_files(
        &prior,
        &[set("b", vec![second.clone()])],
        &[observe(&second, false)],
    )
    .unwrap();
    assert_eq!(&plan.artifacts[..3], &prior);
    assert_eq!(plan.artifacts[3], record("b", &second));
}

#[test]
fn reversing_changes_files_and_observations_keeps_deterministic_output_and_inputs() {
    let old = desired("existing", b"abc");
    let prior = vec![
        record("z", &desired("z", b"other")),
        record("a", &old),
        contribution("manual", "AGENTS.md"),
    ];
    let new = desired("new", b"changed");
    let changes = vec![
        set("a", vec![old.clone(), new.clone()]),
        set("b", vec![desired("first", &[0, 255]), desired("last", b"")]),
    ];
    let observations = vec![
        observe(&old, true),
        observe(&new, false),
        observe(&desired("first", &[0, 255]), false),
        observe(&desired("last", b""), false),
    ];
    let snapshot = (prior.clone(), changes.clone(), observations.clone());
    let first = plan_files(&prior, &changes, &observations).unwrap();
    let mut reverse_changes = changes.clone();
    reverse_changes.reverse();
    for change in &mut reverse_changes {
        if let FileInstallationChange::Set { files, .. } = change {
            files.reverse();
        }
    }
    let mut reverse_observations = observations.clone();
    reverse_observations.reverse();
    assert_eq!(
        first,
        plan_files(&prior, &reverse_changes, &reverse_observations).unwrap()
    );
    assert_eq!(&first.artifacts[..3], &prior);
    assert_eq!(
        first.artifacts[3..]
            .iter()
            .map(|r| r.path.as_str())
            .collect::<Vec<_>>(),
        ["first", "last", "new"]
    );
    assert_eq!(
        first
            .files
            .iter()
            .map(|f| f.location.path.as_str())
            .collect::<Vec<_>>(),
        ["existing", "first", "last", "new"]
    );
    assert_eq!((prior, changes, observations), snapshot);
}

#[test]
fn empty_operation_preserves_exact_prior_records_and_order() {
    let mut shared = record("b", &desired("z", b"abc"));
    shared.owners.push(owner("c"));
    let prior = vec![
        shared,
        contribution("manual", "AGENTS.md"),
        record("a", &desired("a", b"abc")),
    ];
    let snapshot = prior.clone();
    let plan = plan_files(&prior, &[], &[]).unwrap();
    assert_eq!(plan.artifacts, snapshot);
    assert!(plan.files.is_empty());
    assert_eq!(prior, snapshot);
    rejects(
        &prior,
        &[],
        &[observe(&desired("extra", b""), false)],
        "extraneous observation",
    );
}

fn assert_isolated_ancestor_plan(child_root: InstallationRoot) {
    let mut parent = desired("a", b"abc");
    parent.value.mode = Some(0o640);
    let mut child = desired("a/b", b"changed");
    child.location.root = child_root;
    child.value.mode = Some(0o700);
    child.consumers = vec![TargetSurface::CodexCli, TargetSurface::ClaudeCli];
    let prior = vec![record("a", &parent)];
    let changes = vec![set("b", vec![child.clone()])];
    let observations = vec![observe(&child, false)];
    let snapshot = (prior.clone(), changes.clone(), observations.clone());
    let result = plan_files(&prior, &changes, &observations);
    assert!(
        result.is_ok(),
        "ancestor paths in distinct locations must remain independent: {result:?}"
    );
    let plan = result.unwrap();
    assert_eq!(
        plan,
        upskill::files::plan::FileOperationPlan {
            artifacts: vec![
                prior[0].clone(),
                OwnedArtifact {
                    root: child.location.root.clone(),
                    path: "a/b".into(),
                    content: ArtifactContent::File {
                        content_sha256:
                            "d67e2e944994496c8d8ec76eed0cf9f09679448d584b532bebf941852a37f5ed"
                                .into(),
                        mode: Some(0o700),
                    },
                    owners: vec![owner("b")],
                    consumers: vec![TargetSurface::CodexCli, TargetSurface::ClaudeCli],
                }
            ],
            files: vec![upskill::files::plan::PlannedFile {
                location: child.location.clone(),
                before: None,
                after: Some(child.value.clone()),
            }],
        }
    );
    assert_eq!(
        plan.artifacts[0].root.path.as_os_str(),
        parent.location.root.path.as_os_str()
    );
    assert_eq!(
        plan.artifacts[1].root.path.as_os_str(),
        child.location.root.path.as_os_str()
    );
    assert_eq!(
        plan.files[0].location.root.path.as_os_str(),
        child.location.root.path.as_os_str()
    );
    assert_eq!((prior, changes, observations), snapshot);
}

#[test]
fn isolated_ancestor_paths_in_distinct_roots_preserve_exact_plan() {
    let mut root = location("a/b").root;
    root.path.push("other");
    assert_ne!(root.path.as_os_str(), location("a").root.path.as_os_str());
    assert_isolated_ancestor_plan(root);
}

#[test]
fn isolated_ancestor_paths_in_distinct_scopes_preserve_exact_plan() {
    let mut root = location("a/b").root;
    root.scope = OwnershipScope::User;
    assert_isolated_ancestor_plan(root);
}

#[test]
fn isolated_ancestor_paths_in_lexical_dot_roots_preserve_exact_plan() {
    let mut root = location("a/b").root;
    let mut raw = root.path.as_os_str().to_owned();
    raw.push("/.");
    root.path = raw.into();
    assert_ne!(root.path.as_os_str(), location("a").root.path.as_os_str());
    assert_isolated_ancestor_plan(root);
}

#[test]
fn shared_file_rejects_removal_when_all_owners_are_selected() {
    let file = desired("shared", b"abc");
    let mut shared = record("a", &file);
    shared.owners.push(owner("b"));
    rejects(
        &[shared],
        &[remove("a"), remove("b")],
        &[observe(&file, true)],
        "multi-owner",
    );
}

#[test]
fn shared_file_rejects_replacement_when_all_owners_are_selected() {
    let old = desired("shared", b"abc");
    let new = desired("shared", b"changed");
    let mut shared = record("a", &old);
    shared.owners.push(owner("b"));
    rejects(
        &[shared],
        &[remove("a"), set("b", vec![new])],
        &[observe(&old, true)],
        "multi-owner",
    );
}

fn multiple_old_files() -> (DesiredFile, DesiredFile, OwnedArtifact) {
    let mut entry = desired("demo/SKILL.md", b"abc");
    entry.value.mode = Some(0o640);
    let mut resource = desired("demo/assets/data.bin", &[0, 255]);
    resource.value.mode = Some(0o600);
    let mut unrelated = record("other", &desired("untouched.bin", b"keep"));
    unrelated.owners.push(owner("shared"));
    unrelated.consumers = vec![TargetSurface::CodexCli, TargetSurface::CopilotCli];
    (entry, resource, unrelated)
}

#[test]
fn complete_multi_file_removal_deletes_every_old_location() {
    let (entry, resource, unrelated) = multiple_old_files();
    let prior = vec![
        record("install-a", &resource),
        unrelated.clone(),
        record("install-a", &entry),
    ];
    let changes = vec![remove("install-a")];
    let observations = vec![observe(&resource, true), observe(&entry, true)];
    let snapshot = (prior.clone(), changes.clone(), observations.clone());
    let result = plan_files(&prior, &changes, &observations);
    assert!(
        result.is_ok(),
        "complete removal must account for every prior file: {result:?}"
    );
    assert_eq!(
        result.unwrap(),
        upskill::files::plan::FileOperationPlan {
            artifacts: vec![unrelated],
            files: vec![
                upskill::files::plan::PlannedFile {
                    location: entry.location,
                    before: Some(entry.value),
                    after: None
                },
                upskill::files::plan::PlannedFile {
                    location: resource.location,
                    before: Some(resource.value),
                    after: None
                },
            ],
        }
    );
    assert_eq!((prior, changes, observations), snapshot);
}

#[test]
fn complete_multi_file_replacement_deletes_every_omitted_location() {
    let (entry, resource, unrelated) = multiple_old_files();
    let prior = vec![
        record("install-a", &resource),
        unrelated.clone(),
        record("install-a", &entry),
    ];
    let mut new = desired("replacement/SKILL.md", b"changed");
    new.value.mode = Some(0o700);
    new.consumers = vec![TargetSurface::CopilotCli, TargetSurface::ClaudeCli];
    let mut binding = owner("install-a");
    binding.source_revision = Some("revision-2".into());
    let changes = vec![FileInstallationChange::Set {
        owner: binding.clone(),
        files: vec![new.clone()],
    }];
    let observations = vec![
        observe(&new, false),
        observe(&resource, true),
        observe(&entry, true),
    ];
    let snapshot = (prior.clone(), changes.clone(), observations.clone());
    let result = plan_files(&prior, &changes, &observations);
    assert!(
        result.is_ok(),
        "complete replacement must account for every omitted file: {result:?}"
    );
    assert_eq!(
        result.unwrap(),
        upskill::files::plan::FileOperationPlan {
            artifacts: vec![
                unrelated,
                OwnedArtifact {
                    root: new.location.root.clone(),
                    path: "replacement/SKILL.md".into(),
                    content: ArtifactContent::File {
                        content_sha256:
                            "d67e2e944994496c8d8ec76eed0cf9f09679448d584b532bebf941852a37f5ed"
                                .into(),
                        mode: Some(0o700),
                    },
                    owners: vec![binding],
                    consumers: vec![TargetSurface::CopilotCli, TargetSurface::ClaudeCli],
                }
            ],
            files: vec![
                upskill::files::plan::PlannedFile {
                    location: entry.location,
                    before: Some(entry.value),
                    after: None
                },
                upskill::files::plan::PlannedFile {
                    location: resource.location,
                    before: Some(resource.value),
                    after: None
                },
                upskill::files::plan::PlannedFile {
                    location: new.location,
                    before: None,
                    after: Some(new.value)
                },
            ],
        }
    );
    assert_eq!((prior, changes, observations), snapshot);
}

#[test]
fn removal_only_topology_rejects_either_owned_endpoint() {
    let parent = desired("a", b"abc");
    let child = desired("a/b", b"changed");
    let prior = vec![record("parent", &parent), record("child", &child)];
    for (id, file) in [("parent", &parent), ("child", &child)] {
        rejects(&prior, &[remove(id)], &[observe(file, true)], "topology");
    }
}

#[test]
fn removal_only_topology_rejects_untouched_contribution_endpoint() {
    let parent = desired("a", b"abc");
    let child = desired("a/b", b"changed");
    let child_contribution = vec![record("parent", &parent), contribution("manual", "a/b")];
    rejects(
        &child_contribution,
        &[remove("parent")],
        &[observe(&parent, true)],
        "topology",
    );
    let parent_contribution = vec![contribution("manual", "a"), record("child", &child)];
    rejects(
        &parent_contribution,
        &[remove("child")],
        &[observe(&child, true)],
        "topology",
    );
}
