use upskill::files::plan::FileLocation;
use upskill::lifecycle::preflight::{
    PreflightCategory, PreflightDiagnostic, PreflightReport, normalize_report,
};
use upskill::ownership::{InstallationRoot, OwnershipScope};

fn diagnostic(ids: &[&str], category: PreflightCategory, path: &str) -> PreflightDiagnostic {
    PreflightDiagnostic {
        installation_ids: ids.iter().map(|id| (*id).into()).collect(),
        category,
        location: Some(FileLocation {
            root: InstallationRoot {
                scope: OwnershipScope::Project,
                path: "/scope".into(),
            },
            path: path.into(),
        }),
        message: "blocked".into(),
    }
}

// Removing sorting or merging distinct categories/locations breaks this report contract.
#[test]
fn report_order_is_input_independent() {
    let entries = vec![
        diagnostic(&["b", "a", "b"], PreflightCategory::Destination, "two"),
        diagnostic(&["a"], PreflightCategory::Compilation, "one"),
        diagnostic(&["a"], PreflightCategory::Resolution, "one"),
        diagnostic(&["a"], PreflightCategory::Resolution, "other"),
        diagnostic(&["a", "b"], PreflightCategory::Destination, "two"),
    ];
    let mut left = PreflightReport {
        diagnostics: entries.clone(),
        instruction_readers: vec![],
    };
    let mut right = PreflightReport {
        diagnostics: entries.into_iter().rev().collect(),
        instruction_readers: vec![],
    };
    normalize_report(&mut left);
    normalize_report(&mut right);
    assert_eq!(left, right);
    assert_eq!(left.diagnostics.len(), 4);
    assert_eq!(left.diagnostics[0].category, PreflightCategory::Resolution);
    assert_eq!(left.diagnostics[0].location.as_ref().unwrap().path, "one");
    assert_eq!(left.diagnostics[1].location.as_ref().unwrap().path, "other");
    assert_eq!(left.diagnostics[2].category, PreflightCategory::Compilation);
    assert_eq!(left.diagnostics[3].installation_ids, ["a", "b"]);
}

// Canonicalizing or lossily comparing diagnostic roots would merge these locations.
#[test]
fn report_preserves_raw_root_identity() {
    let first = diagnostic(&["a"], PreflightCategory::Destination, "RULE.md");
    let mut second = first.clone();
    second.location.as_mut().unwrap().root.path = "/scope/.".into();
    let mut report = PreflightReport {
        diagnostics: vec![second, first],
        instruction_readers: vec![],
    };
    normalize_report(&mut report);
    assert_eq!(report.diagnostics.len(), 2);
    assert_ne!(
        report.diagnostics[0]
            .location
            .as_ref()
            .unwrap()
            .root
            .path
            .as_os_str(),
        report.diagnostics[1]
            .location
            .as_ref()
            .unwrap()
            .root
            .path
            .as_os_str()
    );
}

use upskill::files::plan::{DesiredFile, FileValue, ObservedFile};
use upskill::lifecycle::commands::CapturedUpsert;
use upskill::lifecycle::desired::DesiredInstallationSet;
use upskill::lifecycle::state::{CombinedState, Installation, Reference, SourceEvidence};
use upskill::operations::plan::DesiredArtifact;
use upskill::ownership::ArtifactOwner;
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn empty_state() -> CombinedState {
    CombinedState {
        schema: 2,
        origin: None,
        installations: vec![],
        bundles: vec![],
        references: vec![],
        artifacts: vec![],
        owner_consumers: vec![],
        owner_contracts: vec![],
        legacy: None,
    }
}
fn file_set(id: &str, path: &str, bytes: &[u8]) -> DesiredInstallationSet {
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: std::env::temp_dir().join("upskill-preflight"),
    };
    DesiredInstallationSet {
        installation: Installation {
            owner: ArtifactOwner {
                installation_id: id.into(),
                source: format!("./{id}"),
                source_revision: None,
                kind: ItemKind::Rule,
                source_name: id.into(),
                installed_name: id.into(),
            },
            root: root.clone(),
            requested: vec![TargetSurface::ClaudeCli],
            source_evidence: SourceEvidence::Local {
                manifest_sha256: "a".repeat(64),
            },
        },
        artifacts: vec![DesiredArtifact::File(DesiredFile {
            location: FileLocation {
                root,
                path: path.into(),
            },
            value: FileValue {
                bytes: bytes.into(),
                mode: Some(0o640),
            },
            consumers: vec![TargetSurface::ClaudeCli],
        })],
        audience_excluded: vec![],
    }
}
fn captured(sets: Vec<DesiredInstallationSet>) -> CapturedUpsert {
    CapturedUpsert {
        add_references: sets
            .iter()
            .map(|s| Reference::DirectInstallation {
                installation_id: s.installation.owner.installation_id.clone(),
            })
            .collect(),
        sets,
        ..Default::default()
    }
}
fn absent(path: &str) -> ObservedFile {
    ObservedFile {
        location: FileLocation {
            root: InstallationRoot {
                scope: OwnershipScope::Project,
                path: std::env::temp_dir().join("upskill-preflight"),
            },
            path: path.into(),
        },
        value: None,
    }
}

// First-error whole-batch planning loses the second disjoint incompatible destination.
#[test]
fn complete_conflict_inventory() {
    use upskill::lifecycle::preflight::collect_conflicts;
    let request = captured(vec![
        file_set("a", "one.md", b"left"),
        file_set("b", "one.md", b"right"),
        file_set("c", "two.md", b"left"),
        file_set("d", "two.md", b"right"),
    ]);
    let diagnostics = collect_conflicts(
        &empty_state(),
        &request,
        &[absent("one.md"), absent("two.md")],
    )
    .unwrap();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(diagnostics[0].installation_ids, ["a", "b"]);
    assert_eq!(diagnostics[1].installation_ids, ["c", "d"]);
    assert_eq!(diagnostics[0].category, PreflightCategory::Ownership);
    assert_eq!(diagnostics[1].category, PreflightCategory::Ownership);
    assert_eq!(diagnostics[0].location.as_ref().unwrap().path, "one.md");
    assert_eq!(diagnostics[1].location.as_ref().unwrap().path, "two.md");
}

// Prefix conflicts must not require observing a directory as a file or hide a disjoint manual file.
#[test]
fn structural_conflicts_do_not_fabricate_content_errors() {
    use upskill::lifecycle::preflight::collect_conflicts;
    let request = captured(vec![
        file_set("a", "nested", b"a"),
        file_set("b", "nested/file", b"b"),
        file_set("c", "manual.md", b"c"),
    ]);
    let mut manual = absent("manual.md");
    manual.value = Some(FileValue {
        bytes: b"Manual".to_vec(),
        mode: Some(0o640),
    });
    let diagnostics = collect_conflicts(&empty_state(), &request, &[manual]).unwrap();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(diagnostics[0].installation_ids, ["a", "b"]);
    assert_eq!(diagnostics[1].installation_ids, ["c"]);
}

#[cfg(unix)]
#[test]
fn non_utf8_roots_do_not_merge_through_lossy_display() {
    use std::os::unix::ffi::OsStringExt;
    let mut first = diagnostic(&["a"], PreflightCategory::Destination, "file");
    let mut second = first.clone();
    first.location.as_mut().unwrap().root.path =
        std::ffi::OsString::from_vec(b"/scope/\xfe".to_vec()).into();
    second.location.as_mut().unwrap().root.path =
        std::ffi::OsString::from_vec(b"/scope/\xff".to_vec()).into();
    let mut report = PreflightReport {
        diagnostics: vec![second, first],
        ..Default::default()
    };
    normalize_report(&mut report);
    assert_eq!(report.diagnostics.len(), 2);
    assert_eq!(
        report.diagnostics[0]
            .location
            .as_ref()
            .unwrap()
            .root
            .path
            .as_os_str()
            .as_encoded_bytes(),
        b"/scope/\xfe"
    );
    assert_eq!(
        report.diagnostics[1]
            .location
            .as_ref()
            .unwrap()
            .root
            .path
            .as_os_str()
            .as_encoded_bytes(),
        b"/scope/\xff"
    );
}

#[test]
fn identity_precedes_category_and_scope_message_remain_distinct() {
    let mut first = diagnostic(&["a"], PreflightCategory::Ownership, "same");
    first.message = "z".into();
    let mut second = first.clone();
    second.message = "a".into();
    let mut user = second.clone();
    user.location.as_mut().unwrap().root.scope = OwnershipScope::User;
    let mut missing = second.clone();
    missing.location = None;
    let mut report = PreflightReport {
        diagnostics: vec![
            diagnostic(&["b"], PreflightCategory::Binding, "same"),
            first,
            user,
            missing,
            second,
        ],
        ..Default::default()
    };
    normalize_report(&mut report);
    assert_eq!(report.diagnostics.len(), 5);
    assert!(report.diagnostics[0].location.is_none());
    assert_eq!(report.diagnostics[1].message, "a");
    assert_eq!(report.diagnostics[2].message, "z");
    assert_eq!(
        report.diagnostics[3].location.as_ref().unwrap().root.scope,
        OwnershipScope::User
    );
    assert_eq!(report.diagnostics[4].installation_ids, ["b"]);
}

fn proof_request(mut request: CapturedUpsert) -> CapturedUpsert {
    use sha2::{Digest, Sha256};
    use upskill::lifecycle::contracts::{ConsumerContract, ContractContent, OwnerContract};
    let hash = |bytes: &[u8]| {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    for set in &request.sets {
        for artifact in &set.artifacts {
            let (path, contribution_id, content) = match artifact {
                DesiredArtifact::File(f) => (
                    f.location.path.clone(),
                    None,
                    ContractContent::File {
                        content_sha256: hash(&f.value.bytes),
                        mode: f.value.mode,
                    },
                ),
                DesiredArtifact::Instruction(i) => (
                    i.location.path.clone(),
                    Some(i.id.clone()),
                    ContractContent::Instruction {
                        content_sha256: hash(i.markdown.as_bytes()),
                    },
                ),
            };
            request.contracts.push(OwnerContract {
                root: set.installation.root.clone(),
                path,
                contribution_id,
                installation_id: set.installation.owner.installation_id.clone(),
                source_manifest_sha256: "a".repeat(64),
                content,
                profiles: vec![ConsumerContract {
                    surface: TargetSurface::ClaudeCli,
                    contract_revision: 1,
                }],
            });
        }
    }
    request
}
fn instruction_set(id: &str, selector: &str, body: &str) -> DesiredInstallationSet {
    let mut set = file_set(id, "AGENTS.md", b"");
    set.artifacts = vec![DesiredArtifact::Instruction(
        upskill::instructions::plan::DesiredInstructionContribution {
            location: upskill::instructions::plan::InstructionLocation {
                root: set.installation.root.clone(),
                path: "AGENTS.md".into(),
            },
            id: selector.into(),
            markdown: body.into(),
            consumers: vec![TargetSurface::ClaudeCli],
        },
    )];
    set
}

// Rejecting all same-path outputs would break legal sharing and canonical instructions.
#[test]
fn compatible_shared_files_and_distinct_contributions_have_no_conflicts() {
    use upskill::lifecycle::preflight::collect_conflicts;
    for (sets, path) in [
        (
            vec![
                file_set("a", "same.md", b"same"),
                file_set("b", "same.md", b"same"),
            ],
            "same.md",
        ),
        (
            vec![
                instruction_set("a", "first", "First\n"),
                instruction_set("b", "second", "Second\n"),
            ],
            "AGENTS.md",
        ),
    ] {
        let request = proof_request(captured(sets));
        let report = collect_conflicts(&empty_state(), &request, &[absent(path)]).unwrap();
        assert!(report.is_empty(), "{report:?}");
    }
}

// Independent type, selector, mode and contract conflicts must not collapse to a single error.
#[test]
fn mixed_artifact_and_sharing_conflict_matrix() {
    use upskill::lifecycle::preflight::collect_conflicts;
    let cases = [
        vec![
            instruction_set("a", "first", "First\n"),
            file_set("b", "AGENTS.md", b"Whole file"),
        ],
        vec![
            instruction_set("a", "same", "First\n"),
            instruction_set("b", "same", "Different\n"),
        ],
        {
            let left = file_set("a", "same.md", b"same");
            let mut right = file_set("b", "same.md", b"same");
            let DesiredArtifact::File(f) = &mut right.artifacts[0] else {
                unreachable!()
            };
            f.value.mode = Some(0o600);
            vec![left, right]
        },
    ];
    for mut sets in cases {
        sets.push(file_set("c", "independent.md", b"left"));
        sets.push(file_set("d", "independent.md", b"right"));
        let request = proof_request(captured(sets));
        let diagnostics = collect_conflicts(
            &empty_state(),
            &request,
            &[
                absent("AGENTS.md"),
                absent("same.md"),
                absent("independent.md"),
            ],
        )
        .unwrap();
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
        assert_eq!(diagnostics[0].installation_ids, ["a", "b"]);
        assert_eq!(diagnostics[1].installation_ids, ["c", "d"]);
    }
    let mut request = proof_request(captured(vec![
        file_set("a", "same.md", b"same"),
        file_set("b", "same.md", b"same"),
    ]));
    request.contracts[1].profiles[0].contract_revision = 2;
    assert_eq!(
        collect_conflicts(&empty_state(), &request, &[absent("same.md")])
            .unwrap()
            .len(),
        1
    );
}

// A retained owner's content cannot be changed by a successful subset of selected sources.
#[test]
fn retained_owner_conflicts_are_reported_without_releasing_authority() {
    use upskill::lifecycle::preflight::collect_conflicts;
    use upskill::lifecycle::upsert::{UpsertOutcome, UpsertRequest, plan_shared_upsert};
    let request = proof_request(captured(vec![
        file_set("a", "one.md", b"old one"),
        file_set("b", "two.md", b"old two"),
    ]));
    let UpsertOutcome::Ready(installed) = plan_shared_upsert(
        &empty_state(),
        UpsertRequest {
            selection: None,
            sets: &request.sets,
            contracts: &request.contracts,
            bundles: &[],
            add_references: &request.add_references,
            remove_references: &[],
        },
        &[absent("one.md"), absent("two.md")],
    )
    .unwrap() else {
        panic!("fixture installation")
    };
    let prior = installed.state;
    let baseline = prior.clone();
    let new = proof_request(captured(vec![
        file_set("c", "one.md", b"changed one"),
        file_set("d", "two.md", b"changed two"),
    ]));
    let observed = [
        ObservedFile {
            location: absent("one.md").location,
            value: Some(FileValue {
                bytes: b"old one".to_vec(),
                mode: Some(0o640),
            }),
        },
        ObservedFile {
            location: absent("two.md").location,
            value: Some(FileValue {
                bytes: b"old two".to_vec(),
                mode: Some(0o640),
            }),
        },
    ];
    let diagnostics = collect_conflicts(&prior, &new, &observed).unwrap();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(diagnostics[0].installation_ids, ["a", "c"]);
    assert_eq!(diagnostics[1].installation_ids, ["b", "d"]);
    assert_eq!(prior, baseline);
}

// A duplicate selected identity must not suppress conflicts in other selected items.
#[test]
fn duplicate_binding_does_not_hide_disjoint_conflicts() {
    use upskill::lifecycle::preflight::collect_conflicts;
    let request = captured(vec![
        file_set("a", "duplicate.md", b"one"),
        file_set("a", "duplicate.md", b"two"),
        file_set("c", "independent.md", b"left"),
        file_set("d", "independent.md", b"right"),
    ]);
    let diagnostics =
        collect_conflicts(&empty_state(), &request, &[absent("independent.md")]).unwrap();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(diagnostics[0].category, PreflightCategory::Binding);
    assert_eq!(diagnostics[1].installation_ids, ["c", "d"]);
}

// All grouping, not only report sorting, must preserve exact raw root spellings.
#[test]
fn independent_raw_roots_do_not_fabricate_shared_conflicts() {
    let first = file_set("a", "same.md", b"one");
    let mut second = file_set("b", "same.md", b"two");
    second.installation.root.path = first.installation.root.path.join(".");
    let DesiredArtifact::File(file) = &mut second.artifacts[0] else {
        panic!()
    };
    file.location.root = second.installation.root.clone();
    let observations = vec![
        ObservedFile {
            location: FileLocation {
                root: first.installation.root.clone(),
                path: "same.md".into(),
            },
            value: None,
        },
        ObservedFile {
            location: FileLocation {
                root: second.installation.root.clone(),
                path: "same.md".into(),
            },
            value: None,
        },
    ];
    let diagnostics = upskill::lifecycle::preflight::collect_conflicts(
        &empty_state(),
        &captured(vec![first, second]),
        &observations,
    )
    .unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

// Legacy reservations remain blockers even when a separate observed file also
// conflicts, rather than disappearing behind the first reservation error.
#[test]
fn legacy_reservation_and_independent_destination_are_both_reported() {
    let mut prior = empty_state();
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
    prior.legacy = Some(lock);
    let request = captured(vec![
        file_set("a", ".claude/rules/old.md", b"selected"),
        file_set("b", "manual.md", b"selected"),
    ]);
    let mut manual = absent("manual.md");
    manual.value = Some(FileValue {
        bytes: b"Manual\n".to_vec(),
        mode: Some(0o640),
    });
    let diagnostics =
        upskill::lifecycle::preflight::collect_conflicts(&prior, &request, &[manual]).unwrap();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(diagnostics[0].installation_ids, ["a"]);
    assert_eq!(diagnostics[0].category, PreflightCategory::Ownership);
    assert_eq!(
        diagnostics[0].location.as_ref().unwrap().path,
        ".claude/rules/old.md"
    );
    assert_eq!(diagnostics[1].installation_ids, ["b"]);
    assert_eq!(diagnostics[1].category, PreflightCategory::Destination);
    assert_eq!(diagnostics[1].location.as_ref().unwrap().path, "manual.md");
}
