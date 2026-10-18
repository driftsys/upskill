#![cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::Path;
use upskill::files::permissions::ResourceExecutionPolicy;
use upskill::filesystem::{ExecutionOutcome, QualifiedScope};
use upskill::lifecycle::desired::codex::{InstructionCompatibility, InstructionReaderRole};
use upskill::lifecycle::orchestration::{
    AddRule, CodexSourceOptions, OrchestrationOutcome, SourceInstructionReaders, SourceRequest,
    TargetSelection, with_prepared_codex,
};
use upskill::lifecycle::selection::Selection;
use upskill::lifecycle::state::{CombinedState, Reference, parse_state};
use upskill::ownership::{ArtifactContent, InstallationRoot, OwnershipScope};
use upskill::source::InstallSource;
use upskill::target::{TargetProfile, TargetSurface};

// Concurrent Git subprocess creation may briefly inherit a CLOEXEC lock before
// exec. Bound that transient lease; a persistent lock still fails the test.
fn writer(scope: &QualifiedScope) -> upskill::filesystem::ScopeWriterGuard<'_> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match scope.try_lock_writer() {
            Ok(guard) => return guard,
            Err(error) => {
                assert!(
                    error.to_string().contains("busy") && std::time::Instant::now() < deadline,
                    "writer acquisition failed: {error:#}"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

fn root(path: &Path, scope: OwnershipScope) -> InstallationRoot {
    InstallationRoot {
        scope,
        path: path.into(),
    }
}
fn write_rule(path: &Path, body: &str, audience: &str) {
    std::fs::write(
        path.join("RULE.md"),
        format!("---\nschema: 1\nname: source\ndescription: demo\n{audience}---\n\n# {body}\n"),
    )
    .unwrap();
}
fn add(path: &Path, alias: &str) -> AddRule {
    AddRule {
        source: InstallSource::LocalPath(path.into()),
        source_name: "source".into(),
        installed_name: alias.into(),
    }
}
fn options() -> CodexSourceOptions<'static> {
    CodexSourceOptions {
        targets: TargetSelection::default(),
        add_defaults: &[TargetSurface::CodexCli],
        profile: TargetSurface::CodexCli.profile(),
        execution: ResourceExecutionPolicy::Disabled,
    }
}
fn state(path: &Path) -> CombinedState {
    parse_state(&std::fs::read_to_string(path.join(".upskill-lock.json")).unwrap()).unwrap()
}
fn apply(root: &InstallationRoot, request: SourceRequest) -> Vec<SourceInstructionReaders> {
    let scope = QualifiedScope::open(root).unwrap();
    let guard = writer(&scope);
    match with_prepared_codex(
        &scope,
        &guard,
        root.clone(),
        request,
        options(),
        |prepared| {
            let reports = prepared.instruction_readers().to_vec();
            assert_eq!(prepared.apply()?.outcome, ExecutionOutcome::Complete);
            Ok(reports)
        },
    )
    .unwrap()
    {
        OrchestrationOutcome::Ready(reports) => reports,
        _ => panic!("expected ready"),
    }
}
fn assert_readers(
    reports: &[SourceInstructionReaders],
    expected: &[TargetSurface],
    compatibility: InstructionCompatibility,
) {
    for value in reports {
        assert_eq!(value.report.contract_revision, 1);
        assert_eq!(value.report.compatibility, compatibility);
        assert_eq!(
            value
                .report
                .readers
                .iter()
                .map(|r| (r.profile, r.role))
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|s| (
                    s.profile(),
                    if *s == TargetSurface::CodexCli {
                        InstructionReaderRole::Requested
                    } else {
                        InstructionReaderRole::PotentialIncidental
                    }
                ))
                .collect::<Vec<_>>()
        );
    }
}
const PROJECT_READERS: &[TargetSurface] = &[
    TargetSurface::CodexCli,
    TargetSurface::CodexIde,
    TargetSurface::CopilotCli,
    TargetSurface::CopilotVscodeLocal,
    TargetSurface::OpenCodeCli,
];

// Catches missing reports, incidental ownership, unstable IDs and destructive aggregate updates.
#[test]
fn two_local_rules_keep_manual_bytes_modes_and_other_owner_through_update_remove() {
    use std::os::unix::fs::PermissionsExt;
    let dest = tempfile::tempdir().unwrap();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write_rule(a.path(), "Alpha", "");
    write_rule(b.path(), "Beta", "");
    let path = dest.path().join("AGENTS.md");
    let manual = b"Manual instructions.\n\n";
    std::fs::write(&path, manual).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let root = root(dest.path(), OwnershipScope::Project);
    let reports = apply(
        &root,
        SourceRequest::Add(vec![add(a.path(), "alpha"), add(b.path(), "beta")]),
    );
    assert_eq!(reports.len(), 2);
    assert!(reports[0].installation_id < reports[1].installation_id);
    assert_readers(
        &reports,
        PROJECT_READERS,
        InstructionCompatibility::UnconditionalMarkdown,
    );
    for r in &reports {
        assert_eq!(r.report.location.root, root);
        assert_eq!(r.report.location.path, "AGENTS.md");
    }
    let first = state(dest.path());
    assert_eq!(first.installations.len(), 2);
    assert_eq!(first.artifacts.len(), 2);
    assert_eq!(first.owner_contracts.len(), 2);
    for item in &first.installations {
        assert_eq!(item.requested, [TargetSurface::CodexCli]);
        assert!(first.references.contains(&Reference::DirectInstallation {
            installation_id: item.owner.installation_id.clone()
        }));
    }
    for artifact in &first.artifacts {
        assert_eq!(artifact.owners.len(), 1);
        assert_eq!(artifact.consumers, [TargetSurface::CodexCli]);
        assert_eq!(artifact.path, "AGENTS.md");
    }
    for association in &first.owner_consumers {
        assert_eq!(association.consumers, [TargetSurface::CodexCli]);
    }
    for contract in &first.owner_contracts {
        assert_eq!(contract.path, "AGENTS.md");
        assert_eq!(contract.profiles.len(), 1);
        assert_eq!(contract.profiles[0].surface, TargetSurface::CodexCli);
        assert_eq!(contract.profiles[0].contract_revision, 1);
    }
    let beta = first
        .artifacts
        .iter()
        .find(|x| x.owners[0].installed_name == "beta")
        .unwrap()
        .clone();
    let before = std::fs::read_to_string(&path).unwrap();
    write_rule(a.path(), "Alpha updated", "");
    apply(
        &root,
        SourceRequest::Update(Selection::Names(vec!["alpha".into()])),
    );
    let second = state(dest.path());
    assert_eq!(
        second
            .installations
            .iter()
            .map(|i| &i.owner.installation_id)
            .collect::<Vec<_>>(),
        first
            .installations
            .iter()
            .map(|i| &i.owner.installation_id)
            .collect::<Vec<_>>()
    );
    assert!(second.artifacts.contains(&beta));
    assert_eq!(second.references, first.references);
    let updated = std::fs::read_to_string(&path).unwrap();
    assert_eq!(updated, before.replace("# Alpha\n", "# Alpha updated\n"));
    assert!(
        apply(
            &root,
            SourceRequest::Remove(Selection::Names(vec!["alpha".into()]))
        )
        .is_empty()
    );
    let surviving = state(dest.path());
    assert_eq!(surviving.artifacts.len(), 1);
    let remaining = &surviving.artifacts[0];
    assert_eq!(remaining.owners, beta.owners);
    assert_eq!(remaining.consumers, [TargetSurface::CodexCli]);
    assert_eq!(remaining.path, "AGENTS.md");
    assert_eq!(remaining.root, root);
    let (
        ArtifactContent::MarkdownContribution { contribution: old },
        ArtifactContent::MarkdownContribution { contribution: new },
    ) = (&beta.content, &remaining.content)
    else {
        panic!("expected contribution");
    };
    assert_eq!(new.id, old.id);
    assert_eq!(new.content_sha256, old.content_sha256);
    assert_eq!(
        surviving.owner_contracts,
        first
            .owner_contracts
            .iter()
            .filter(|c| c.installation_id == beta.owners[0].installation_id)
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        surviving.references,
        [Reference::DirectInstallation {
            installation_id: beta.owners[0].installation_id.clone()
        }]
    );
    assert_eq!(surviving.installations.len(), 1);
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, format!("Manual instructions.\n\n\n<!-- upskill:begin {} -->\n# Beta\n<!-- upskill:end {} -->\n", old.id, old.id).as_bytes());
    assert!(bytes.starts_with(manual));
    assert!(!String::from_utf8_lossy(&bytes).contains("Alpha"));
    assert!(String::from_utf8_lossy(&bytes).contains("Beta"));
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    apply(&root, SourceRequest::Remove(Selection::All));
    assert_eq!(std::fs::read(&path).unwrap(), manual);
}

// Catches incorrect user destinations and missing global reports/contracts.
#[test]
fn user_root_codex_only_rule_uses_dot_codex_agents_and_two_readers() {
    let dest = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Global", "audience: [codex]\n");
    let root = root(dest.path(), OwnershipScope::User);
    let reports = apply(
        &root,
        SourceRequest::Add(vec![add(source.path(), "global")]),
    );
    assert_eq!(reports.len(), 1);
    assert_readers(
        &reports,
        &[TargetSurface::CodexCli, TargetSurface::CodexIde],
        InstructionCompatibility::UnconditionalMarkdown,
    );
    assert_eq!(reports[0].report.location.path, ".codex/AGENTS.md");
    assert_eq!(reports[0].report.location.root, root);
    let installed = state(dest.path());
    assert_eq!(installed.artifacts.len(), 1);
    assert_eq!(installed.owner_contracts.len(), 1);
    assert_eq!(installed.artifacts[0].root, root);
    assert_eq!(installed.artifacts[0].path, ".codex/AGENTS.md");
    assert_eq!(installed.owner_contracts[0].root, root);
    assert_eq!(installed.owner_contracts[0].path, ".codex/AGENTS.md");
    assert!(
        std::fs::read_to_string(dest.path().join(".codex/AGENTS.md"))
            .unwrap()
            .contains("Global")
    );
    assert!(!dest.path().join("AGENTS.md").exists());
}

// Catches dropped excluded reports, loss of retained identity, and historical adapter conversion.
#[test]
fn exclusion_reinclusion_and_historical_rejection_keep_route_identity() {
    use upskill::lifecycle::orchestration::{CompilerContext, SourceOptions, with_prepared};
    for scope_kind in [OwnershipScope::Project, OwnershipScope::User] {
        let dest = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        write_rule(source.path(), "Excluded", "audience: [claude]\n");
        let root = root(dest.path(), scope_kind);
        let readers: &[TargetSurface] = if scope_kind == OwnershipScope::Project {
            PROJECT_READERS
        } else {
            &[TargetSurface::CodexCli, TargetSurface::CodexIde]
        };
        let reports = apply(&root, SourceRequest::Add(vec![add(source.path(), "alias")]));
        assert_eq!(reports.len(), 1);
        assert_readers(&reports, readers, InstructionCompatibility::NotEvaluated);
        let excluded = state(dest.path());
        assert!(excluded.artifacts.is_empty());
        assert!(excluded.owner_consumers.is_empty());
        assert!(excluded.owner_contracts.is_empty());
        assert_eq!(
            excluded.installations[0].requested,
            [TargetSurface::CodexCli]
        );
        let bytes = std::fs::read(dest.path().join(".upskill-lock.json")).unwrap();
        for request in [
            SourceRequest::Update(Selection::All),
            SourceRequest::Add(vec![add(source.path(), "alias")]),
        ] {
            let scope = QualifiedScope::open(&root).unwrap();
            let guard = writer(&scope);
            let compiler = CompilerContext {
                bindings: vec![],
                profiles: vec![],
                execution: ResourceExecutionPolicy::Disabled,
            };
            let error = with_prepared(
                &scope,
                &guard,
                root.clone(),
                request,
                SourceOptions {
                    targets: TargetSelection::default(),
                    add_defaults: &[TargetSurface::CodexCli],
                    compiler: &compiler,
                },
                |_| -> anyhow::Result<()> {
                    panic!("historical route must reject excluded Codex owner")
                },
            )
            .err()
            .unwrap();
            assert!(error.to_string().contains("captured instruction adapter"));
            assert_eq!(
                std::fs::read(dest.path().join(".upskill-lock.json")).unwrap(),
                bytes
            );
            assert!(!scope.has_pending_operation().unwrap());
        }
        write_rule(source.path(), "Included", "");
        apply(&root, SourceRequest::Update(Selection::All));
        let included = state(dest.path());
        assert_eq!(
            included.installations[0].owner.installation_id,
            excluded.installations[0].owner.installation_id
        );
        let contribution = included.artifacts[0].content.clone();
        write_rule(source.path(), "Excluded again", "audience: [claude]\n");
        let reports = apply(&root, SourceRequest::Update(Selection::All));
        assert_eq!(reports.len(), 1);
        assert_readers(&reports, readers, InstructionCompatibility::NotEvaluated);
        assert!(state(dest.path()).artifacts.is_empty());
        write_rule(source.path(), "Included", "");
        apply(&root, SourceRequest::Add(vec![add(source.path(), "alias")]));
        assert_eq!(state(dest.path()).artifacts[0].content, contribution);
    }
}

// Catches publication during preparation and loss of final source byte validation.
#[test]
fn discard_and_local_source_change_after_preparation_have_no_effects() {
    let dest = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Captured", "");
    let root = root(dest.path(), OwnershipScope::Project);
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = writer(&scope);
    with_prepared_codex(
        &scope,
        &guard,
        root.clone(),
        SourceRequest::Add(vec![add(source.path(), "alias")]),
        options(),
        |prepared| {
            assert_eq!(prepared.instruction_readers().len(), 1);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
    let result = with_prepared_codex(
        &scope,
        &guard,
        root.clone(),
        SourceRequest::Add(vec![add(source.path(), "alias")]),
        options(),
        |prepared| {
            write_rule(source.path(), "Changed", "");
            prepared.apply()
        },
    );
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
    assert!(!scope.has_pending_operation().unwrap());
}

// Catches validation before removal and accidental dependency on retained source availability.
#[test]
fn remove_deleted_source_ignores_invalid_options_on_both_routes() {
    use upskill::lifecycle::orchestration::{CompilerContext, SourceOptions, with_prepared};
    for codex_route in [true, false] {
        let dest = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        write_rule(source.path(), "Owned", "");
        let root = root(dest.path(), OwnershipScope::Project);
        apply(&root, SourceRequest::Add(vec![add(source.path(), "alias")]));
        std::fs::remove_file(source.path().join("RULE.md")).unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = writer(&scope);
        let callback = |prepared: upskill::lifecycle::orchestration::SourcePrepared<'_>| {
            assert!(prepared.instruction_readers().is_empty());
            prepared.apply()
        };
        let outcome = if codex_route {
            with_prepared_codex(
                &scope,
                &guard,
                root.clone(),
                SourceRequest::Remove(Selection::All),
                CodexSourceOptions {
                    targets: TargetSelection {
                        invocation: &[TargetSurface::ClaudeCli, TargetSurface::CodexIde],
                        ..Default::default()
                    },
                    profile: TargetProfile {
                        contract_revision: 0,
                        ..TargetSurface::ClaudeCli.profile()
                    },
                    ..options()
                },
                callback,
            )
        } else {
            let compiler = CompilerContext {
                bindings: vec![],
                profiles: vec![],
                execution: ResourceExecutionPolicy::Disabled,
            };
            with_prepared(
                &scope,
                &guard,
                root.clone(),
                SourceRequest::Remove(Selection::All),
                SourceOptions {
                    targets: TargetSelection::default(),
                    add_defaults: &[],
                    compiler: &compiler,
                },
                callback,
            )
        }
        .unwrap();
        assert!(
            matches!(outcome, OrchestrationOutcome::Ready(report) if report.outcome == ExecutionOutcome::Complete)
        );
        assert!(state(dest.path()).installations.is_empty());
        assert_eq!(std::fs::read(dest.path().join("AGENTS.md")).unwrap(), b"");
    }
}

// Catches target precedence regression and premature source resolution for unsupported intent.
#[test]
fn effective_intent_and_profile_fail_with_typed_instructions_error_before_resolution() {
    use upskill::target::{CapabilityCategory, CapabilityError};
    let dest = tempfile::tempdir().unwrap();
    let root = root(dest.path(), OwnershipScope::Project);
    let missing = dest.path().join("missing-source");
    for (targets, profile, key) in [
        (
            TargetSelection {
                invocation: &[TargetSurface::CodexCli, TargetSurface::CodexIde],
                ..Default::default()
            },
            TargetSurface::CodexCli.profile(),
            "selection",
        ),
        (
            TargetSelection {
                project: &[TargetSurface::ClaudeCli],
                global: &[TargetSurface::CodexCli],
                ..Default::default()
            },
            TargetSurface::CodexCli.profile(),
            "selection",
        ),
        (
            TargetSelection::default(),
            TargetProfile {
                contract_revision: 2,
                ..TargetSurface::CodexCli.profile()
            },
            "profile",
        ),
        (
            TargetSelection::default(),
            TargetSurface::CodexIde.profile(),
            "profile",
        ),
    ] {
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = writer(&scope);
        let error = with_prepared_codex(
            &scope,
            &guard,
            root.clone(),
            SourceRequest::Add(vec![add(&missing, "alias")]),
            CodexSourceOptions {
                targets,
                profile,
                ..options()
            },
            |_| -> anyhow::Result<()> { panic!("invalid options must reject before callback") },
        )
        .err()
        .unwrap();
        let typed = error
            .downcast_ref::<CapabilityError>()
            .expect("typed capability error");
        assert_eq!(typed.key.category, CapabilityCategory::Instructions);
        assert_eq!(typed.key.name, key);
        assert!(!scope.has_pending_operation().unwrap());
    }
    for invocation in [
        &[][..],
        &[TargetSurface::CodexCli, TargetSurface::CodexCli][..],
        &[TargetSurface::ClaudeCli][..],
    ] {
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = writer(&scope);
        let error = with_prepared_codex(
            &scope,
            &guard,
            root.clone(),
            SourceRequest::Add(vec![add(&missing, "alias")]),
            CodexSourceOptions {
                targets: TargetSelection {
                    invocation,
                    ..Default::default()
                },
                add_defaults: &[],
                ..options()
            },
            |_| -> anyhow::Result<()> {
                panic!("absent, duplicate and foreign intent must reject before source work")
            },
        )
        .err()
        .unwrap();
        let typed = error
            .downcast_ref::<CapabilityError>()
            .expect("typed absent/duplicate/foreign selection error");
        assert_eq!(typed.key.category, CapabilityCategory::Instructions);
        assert_eq!(typed.key.name, "selection");
        assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
        assert!(!scope.has_pending_operation().unwrap());
    }
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Precedence", "");
    for targets in [
        TargetSelection {
            invocation: &[TargetSurface::CodexCli],
            project: &[TargetSurface::ClaudeCli],
            global: &[TargetSurface::OpenCodeCli],
        },
        TargetSelection {
            project: &[TargetSurface::CodexCli],
            global: &[TargetSurface::ClaudeCli],
            ..Default::default()
        },
        TargetSelection {
            global: &[TargetSurface::CodexCli],
            ..Default::default()
        },
    ] {
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = writer(&scope);
        with_prepared_codex(
            &scope,
            &guard,
            root.clone(),
            SourceRequest::Add(vec![add(source.path(), "alias")]),
            CodexSourceOptions {
                targets,
                add_defaults: &[TargetSurface::ClaudeCli],
                ..options()
            },
            |prepared| prepared.apply(),
        )
        .unwrap();
    }
    assert_eq!(
        state(dest.path()).installations[0].requested,
        [TargetSurface::CodexCli]
    );
}

// Catches partial application when a later captured source conflicts with incidental readers.
#[test]
fn later_incompatible_source_prevents_callback_and_all_writes() {
    for audience in ["audience: [codex]\n", ""] {
        let dest = tempfile::tempdir().unwrap();
        let first = tempfile::tempdir().unwrap();
        let later = tempfile::tempdir().unwrap();
        write_rule(first.path(), "Valid", "");
        if audience.is_empty() {
            write_rule(
                later.path(),
                "Common\n<!-- @client:codex -->\nDifferent\n<!-- @endclient -->",
                audience,
            );
        } else {
            write_rule(later.path(), "Codex only", audience);
        }
        let root = root(dest.path(), OwnershipScope::Project);
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = writer(&scope);
        let error = with_prepared_codex(
            &scope,
            &guard,
            root,
            SourceRequest::Add(vec![add(first.path(), "first"), add(later.path(), "later")]),
            options(),
            |_| -> anyhow::Result<()> { panic!("later source must reject before callback") },
        )
        .err()
        .expect("incompatible source must reject");
        let typed = error
            .downcast_ref::<upskill::target::CapabilityError>()
            .expect("reader conflict capability error");
        assert_eq!(
            typed.key.name,
            if audience.is_empty() {
                "directives"
            } else {
                "audience"
            }
        );
        assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
        assert!(!scope.has_pending_operation().unwrap());
    }
}

fn write_state(path: &Path, value: &CombinedState) {
    std::fs::write(
        path.join(".upskill-lock.json"),
        upskill::lifecycle::state::serialize_state(value).unwrap(),
    )
    .unwrap();
}

// Catches incomplete selection preflight, permissive migration and noncanonical retained ownership.
#[test]
fn all_retained_owners_are_preflighted_before_any_source_resolution() {
    use upskill::lifecycle::contracts::{ConsumerContract, ContractContent};
    for defect in [
        "missing-contract",
        "targets",
        "nonrule-excluded",
        "contract",
        "destination",
        "arbitrary-id",
        "multiple",
        "shared",
        "whole-file",
    ] {
        for same_alias_add in [false, true] {
            let dest = tempfile::tempdir().unwrap();
            let source = tempfile::tempdir().unwrap();
            write_rule(source.path(), "Owned", "");
            let root = root(dest.path(), OwnershipScope::Project);
            apply(
                &root,
                SourceRequest::Add(vec![
                    add(source.path(), "first"),
                    add(source.path(), "later"),
                ]),
            );
            let mut value = state(dest.path());
            // Serialization orders owners by identity; invalidate the last selected
            // owner to pin complete preflight rather than first-owner validation.
            let valid_alias = value
                .installations
                .first()
                .unwrap()
                .owner
                .installed_name
                .clone();
            let owner = value.installations.last().unwrap().owner.clone();
            let invalid_alias = owner.installed_name.clone();
            let id = owner.installation_id.clone();
            let ai = value
                .artifacts
                .iter()
                .position(|a| a.owners[0].installation_id == id)
                .unwrap();
            let ci = value
                .owner_contracts
                .iter()
                .position(|c| c.installation_id == id)
                .unwrap();
            let oi = value
                .owner_consumers
                .iter()
                .position(|c| c.installation_id == id)
                .unwrap();
            let expected = match defect {
                "targets" => {
                    value
                        .installations
                        .iter_mut()
                        .find(|i| i.owner.installation_id == id)
                        .unwrap()
                        .requested
                        .push(TargetSurface::CodexIde);
                    "exact Codex CLI rule binding"
                }
                "nonrule-excluded" => {
                    value.artifacts.remove(ai);
                    value.owner_consumers.remove(oi);
                    value.owner_contracts.remove(ci);
                    value
                        .installations
                        .iter_mut()
                        .find(|i| i.owner.installation_id == id)
                        .unwrap()
                        .owner
                        .kind = upskill::pipeline::ItemKind::Skill;
                    "exact Codex CLI rule binding"
                }
                "contract" => {
                    value.owner_contracts[ci].profiles[0].contract_revision = 2;
                    "owner contract must match revision 1"
                }
                "missing-contract" => {
                    value.owner_contracts.remove(ci);
                    "missing its owner contract"
                }
                "destination" => {
                    value.artifacts[ai].path = "nested/AGENTS.md".into();
                    value.owner_contracts[ci].path = "nested/AGENTS.md".into();
                    value.owner_consumers[oi].path = "nested/AGENTS.md".into();
                    "canonical destination"
                }
                "arbitrary-id" => {
                    if let ArtifactContent::MarkdownContribution { contribution } =
                        &mut value.artifacts[ai].content
                    {
                        contribution.id = "arbitrary".into();
                    }
                    value.owner_consumers[oi].contribution_id = Some("arbitrary".into());
                    value.owner_contracts[ci].contribution_id = Some("arbitrary".into());
                    "producer identity and sole installation ownership"
                }
                "multiple" => {
                    let mut artifact = value.artifacts[ai].clone();
                    if let ArtifactContent::MarkdownContribution { contribution } =
                        &mut artifact.content
                    {
                        contribution.id = "second".into();
                    }
                    value.artifacts.push(artifact);
                    let mut association = value.owner_consumers[oi].clone();
                    association.contribution_id = Some("second".into());
                    value.owner_consumers.push(association);
                    let mut contract = value.owner_contracts[ci].clone();
                    contract.contribution_id = Some("second".into());
                    value.owner_contracts.push(contract);
                    "zero or one canonical contribution"
                }
                "shared" => {
                    let mut extra = value
                        .installations
                        .iter()
                        .find(|i| i.owner.installation_id == id)
                        .unwrap()
                        .clone();
                    extra.owner.installation_id = "unselected-shared-owner".into();
                    extra.owner.installed_name = "shared".into();
                    value.artifacts[ai].owners.push(extra.owner.clone());
                    let mut association = value.owner_consumers[oi].clone();
                    association.installation_id = extra.owner.installation_id.clone();
                    value.owner_consumers.push(association);
                    let mut contract = value.owner_contracts[ci].clone();
                    contract.installation_id = extra.owner.installation_id.clone();
                    contract.profiles = vec![ConsumerContract {
                        surface: TargetSurface::CodexCli,
                        contract_revision: 1,
                    }];
                    value.owner_contracts.push(contract);
                    value.references.push(Reference::DirectInstallation {
                        installation_id: extra.owner.installation_id.clone(),
                    });
                    value.installations.push(extra);
                    "producer identity and sole installation ownership"
                }
                "whole-file" => {
                    let hash = "0".repeat(64);
                    value.artifacts[ai].content = ArtifactContent::File {
                        content_sha256: hash.clone(),
                        mode: None,
                    };
                    value.owner_consumers[oi].contribution_id = None;
                    value.owner_contracts[ci].contribution_id = None;
                    value.owner_contracts[ci].content = ContractContent::File {
                        content_sha256: hash,
                        mode: None,
                    };
                    value.artifacts[ai].path = "RULE.md".into();
                    value.owner_consumers[oi].path = "RULE.md".into();
                    value.owner_contracts[ci].path = "RULE.md".into();
                    "cannot convert whole-file ownership"
                }
                _ => unreachable!(),
            };
            write_state(dest.path(), &value);
            let bytes = std::fs::read(dest.path().join(".upskill-lock.json")).unwrap();
            let instructions = std::fs::read(dest.path().join("AGENTS.md")).unwrap();
            std::fs::remove_dir_all(source.path()).unwrap();
            let scope = QualifiedScope::open(&root).unwrap();
            let guard = writer(&scope);
            let request = if same_alias_add {
                SourceRequest::Add(vec![
                    add(source.path(), &valid_alias),
                    add(source.path(), &invalid_alias),
                ])
            } else {
                SourceRequest::Update(Selection::Names(vec![valid_alias, invalid_alias]))
            };
            let result = with_prepared_codex(
                &scope,
                &guard,
                root.clone(),
                request,
                options(),
                |_| -> anyhow::Result<()> {
                    panic!("complete preflight must reject before callback")
                },
            );
            if defect == "nonrule-excluded" && !same_alias_add {
                let Ok(OrchestrationOutcome::Blocked(diagnostics)) = result else {
                    panic!("unsupported non-rule selection must block");
                };
                assert!(
                    matches!(&diagnostics[..], [upskill::lifecycle::commands::CommandDiagnostic::Selection(upskill::lifecycle::selection::SelectionDiagnostic::UnsupportedInstallation { installation_id, kind: upskill::pipeline::ItemKind::Skill })] if installation_id == &id)
                );
            } else {
                let error = result.err().expect("retained representation must reject");
                assert!(error.to_string().contains(expected), "{defect}: {error:#}");
            }
            assert_eq!(
                std::fs::read(dest.path().join(".upskill-lock.json")).unwrap(),
                bytes
            );
            assert_eq!(
                std::fs::read(dest.path().join("AGENTS.md")).unwrap(),
                instructions
            );
            assert!(!scope.has_pending_operation().unwrap());
        }
    }
}

fn clear_git_environment(command: &mut std::process::Command) {
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
}
fn git(path: &Path, args: &[&str]) -> String {
    let mut command = std::process::Command::new("git");
    clear_git_environment(&mut command);
    let output = command
        .current_dir(path)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn commit(repo: &Path) {
    git(repo, &["add", "."]);
    git(
        repo,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    );
}

// Catches lost actual Git evidence, excluded-report loss and final revision-only revalidation removal.
#[test]
fn git_updates_exclusions_and_revision_only_change_rejects_before_apply() {
    const CHILD: &str = "UPSKILL_CODEX_ORCHESTRATION_GIT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        git(repo.path(), &["init", "--quiet", "--initial-branch=main"]);
        std::fs::create_dir(repo.path().join("item")).unwrap();
        write_rule(&repo.path().join("item"), "Initial", "");
        commit(repo.path());
        std::fs::write(
            home.path().join(".gitconfig"),
            format!(
                "[url \"file://{}\"]\n    insteadOf = https://example.invalid/codex-rules\n",
                repo.path().display()
            ),
        )
        .unwrap();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        clear_git_environment(&mut command);
        let output = command
            .args([
                "--exact",
                "git_updates_exclusions_and_revision_only_change_rejects_before_apply",
                "--nocapture",
            ])
            .env(CHILD, repo.path())
            .env("HOME", home.path())
            .env("USERPROFILE", home.path())
            .env("GIT_CONFIG_GLOBAL", home.path().join(".gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TRACE2_EVENT", home.path().join("source-trace.json"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let repo = std::path::PathBuf::from(std::env::var_os(CHILD).unwrap());
    for scope_kind in [OwnershipScope::Project, OwnershipScope::User] {
        let dest = tempfile::tempdir().unwrap();
        let root = root(dest.path(), scope_kind);
        let source = InstallSource::Git(upskill::source::GitRepo {
            url: "https://example.invalid/codex-rules".into(),
            git_ref: Some("main".into()),
            subfolder: Some("item".into()),
        });
        let label = source.to_string();
        let audience = if scope_kind == OwnershipScope::User {
            "audience: [codex]\n"
        } else {
            ""
        };
        write_rule(&repo.join("item"), "Included", audience);
        commit(&repo);
        let reports = apply(
            &root,
            SourceRequest::Add(vec![AddRule {
                source,
                source_name: "source".into(),
                installed_name: "alias".into(),
            }]),
        );
        assert_eq!(reports.len(), 1);
        let first = state(dest.path());
        let id = first.installations[0].owner.installation_id.clone();
        let contribution = first.artifacts[0].content.clone();
        let output_path = dest.path().join(&first.artifacts[0].path);
        assert_eq!(first.installations[0].owner.source, label);
        assert_eq!(
            first.installations[0].owner.source_revision.as_deref(),
            Some(git(&repo, &["rev-parse", "HEAD"]).as_str())
        );
        write_rule(&repo.join("item"), "Excluded", "audience: [claude]\n");
        commit(&repo);
        let reports = apply(&root, SourceRequest::Update(Selection::All));
        assert_eq!(reports.len(), 1);
        assert_readers(
            &reports,
            if scope_kind == OwnershipScope::Project {
                PROJECT_READERS
            } else {
                &[TargetSurface::CodexCli, TargetSurface::CodexIde]
            },
            InstructionCompatibility::NotEvaluated,
        );
        let excluded = state(dest.path());
        assert!(excluded.artifacts.is_empty());
        assert!(excluded.owner_contracts.is_empty());
        assert_eq!(excluded.installations[0].owner.installation_id, id);
        assert_ne!(
            excluded.installations[0].owner.source_revision,
            first.installations[0].owner.source_revision
        );
        write_rule(&repo.join("item"), "Included", audience);
        commit(&repo);
        apply(&root, SourceRequest::Update(Selection::All));
        let included = state(dest.path());
        assert_eq!(included.installations[0].owner.installation_id, id);
        assert_eq!(included.artifacts[0].content, contribution);
        let upskill::lifecycle::state::SourceEvidence::Git {
            resolved_revision,
            manifest_sha256,
        } = &included.installations[0].source_evidence
        else {
            panic!("expected Git evidence")
        };
        assert_eq!(
            Some(resolved_revision),
            included.installations[0].owner.source_revision.as_ref()
        );
        assert_eq!(resolved_revision, &git(&repo, &["rev-parse", "HEAD"]));
        assert_eq!(manifest_sha256.len(), 64);
        let bytes = std::fs::read(dest.path().join(".upskill-lock.json")).unwrap();
        let output = std::fs::read(&output_path).unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = writer(&scope);
        let result = with_prepared_codex(
            &scope,
            &guard,
            root.clone(),
            SourceRequest::Update(Selection::All),
            options(),
            |prepared| {
                // Git trace records the real private checkout commands; mutate only HEAD,
                // leaving captured tree bytes unchanged so lower snapshot checks cannot catch it.
                let trace =
                    std::fs::read_to_string(std::env::var_os("GIT_TRACE2_EVENT").unwrap()).unwrap();
                let (metadata, tree) = trace
                    .lines()
                    .filter_map(|line| {
                        let event: serde_json::Value = serde_json::from_str(line).ok()?;
                        let argv = event.get("argv")?.as_array()?;
                        let at = argv.iter().position(|a| a.as_str() == Some("--git-dir"))?;
                        let wt = argv
                            .iter()
                            .position(|a| a.as_str() == Some("--work-tree"))?;
                        Some((
                            argv.get(at + 1)?.as_str()?.to_owned(),
                            argv.get(wt + 1)?.as_str()?.to_owned(),
                        ))
                    })
                    .next_back()
                    .expect("recorded private Git metadata and work tree");
                let tree_before = std::fs::read(Path::new(&tree).join("item/RULE.md")).unwrap();
                let old = git(&repo, &["--git-dir", &metadata, "rev-parse", "HEAD"]);
                let new = git(
                    &repo,
                    &[
                        "--git-dir",
                        &metadata,
                        "-c",
                        "user.name=Test",
                        "-c",
                        "user.email=test@example.invalid",
                        "commit-tree",
                        "HEAD^{tree}",
                        "-p",
                        "HEAD",
                        "-m",
                        "same tree different revision",
                    ],
                );
                assert_ne!(old, new);
                git(&repo, &["--git-dir", &metadata, "update-ref", "HEAD", &new]);
                assert_eq!(
                    std::fs::read(Path::new(&tree).join("item/RULE.md")).unwrap(),
                    tree_before
                );
                prepared.apply()
            },
        );
        assert!(
            format!(
                "{:#}",
                result.err().expect("changed private revision must reject")
            )
            .contains("source checkout revision changed")
        );
        assert_eq!(
            std::fs::read(dest.path().join(".upskill-lock.json")).unwrap(),
            bytes
        );
        assert_eq!(std::fs::read(&output_path).unwrap(), output);
        assert!(!scope.has_pending_operation().unwrap());
    }
}

// Catches invented reports for historical compilation and cross-adapter conversion.
#[test]
fn historical_preparations_report_no_new_readers_and_cannot_convert_to_codex() {
    use upskill::generate::Client;
    use upskill::lifecycle::desired::CompilerBinding;
    use upskill::lifecycle::orchestration::{CompilerContext, SourceOptions, with_prepared};
    let dest = tempfile::tempdir().unwrap();
    let codex = tempfile::tempdir().unwrap();
    let historical = tempfile::tempdir().unwrap();
    write_rule(codex.path(), "Codex", "");
    write_rule(historical.path(), "Historical", "");
    let root = root(dest.path(), OwnershipScope::Project);
    apply(&root, SourceRequest::Add(vec![add(codex.path(), "codex")]));
    let instructions = std::fs::read(dest.path().join("AGENTS.md")).unwrap();
    let compiler = CompilerContext {
        bindings: vec![CompilerBinding {
            client: Client::Claude,
            consumers: vec![TargetSurface::ClaudeCli],
        }],
        profiles: vec![TargetSurface::ClaudeCli.profile()],
        execution: ResourceExecutionPolicy::Disabled,
    };
    for request in [
        SourceRequest::Add(vec![add(historical.path(), "historical")]),
        SourceRequest::Update(Selection::Names(vec!["historical".into()])),
    ] {
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = writer(&scope);
        with_prepared(
            &scope,
            &guard,
            root.clone(),
            request,
            SourceOptions {
                targets: TargetSelection::default(),
                add_defaults: &[TargetSurface::ClaudeCli],
                compiler: &compiler,
            },
            |prepared| {
                assert!(prepared.instruction_readers().is_empty());
                prepared.apply()
            },
        )
        .unwrap();
        assert_eq!(
            std::fs::read(dest.path().join("AGENTS.md")).unwrap(),
            instructions
        );
    }
    std::fs::remove_dir_all(historical.path()).unwrap();
    let before = std::fs::read(dest.path().join(".upskill-lock.json")).unwrap();
    for request in [
        SourceRequest::Add(vec![add(historical.path(), "historical")]),
        SourceRequest::Update(Selection::Names(vec!["historical".into()])),
    ] {
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = writer(&scope);
        let error = with_prepared_codex(
            &scope,
            &guard,
            root.clone(),
            request,
            options(),
            |_| -> anyhow::Result<()> {
                panic!("historical conversion must reject before source resolution")
            },
        )
        .err()
        .unwrap();
        assert!(error.to_string().contains("exact Codex CLI rule binding"));
        assert_eq!(
            std::fs::read(dest.path().join(".upskill-lock.json")).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read(dest.path().join("AGENTS.md")).unwrap(),
            instructions
        );
        assert!(!scope.has_pending_operation().unwrap());
    }
}

// Catches alias-substituted report IDs and missing sorting independently of random paths.
#[test]
fn sorted_mixed_reports_identify_installations_on_reverse_order_refresh() {
    let dest = tempfile::tempdir().unwrap();
    let included = tempfile::tempdir().unwrap();
    let excluded = tempfile::tempdir().unwrap();
    write_rule(included.path(), "Included", "");
    write_rule(excluded.path(), "Excluded", "audience: [claude]\n");
    let root = root(dest.path(), OwnershipScope::Project);
    apply(
        &root,
        SourceRequest::Add(vec![
            add(included.path(), "included"),
            add(excluded.path(), "excluded"),
        ]),
    );
    let installed = state(dest.path());
    let mut expected_ids = installed
        .installations
        .iter()
        .map(|i| i.owner.installation_id.clone())
        .collect::<Vec<_>>();
    expected_ids.sort();
    assert_eq!(expected_ids.len(), 2);
    let requests = expected_ids
        .iter()
        .rev()
        .map(|id| {
            let item = installed
                .installations
                .iter()
                .find(|i| &i.owner.installation_id == id)
                .unwrap();
            let path = match item.owner.installed_name.as_str() {
                "included" => included.path(),
                "excluded" => excluded.path(),
                _ => panic!("unexpected alias"),
            };
            add(path, &item.owner.installed_name)
        })
        .collect();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = writer(&scope);
    with_prepared_codex(
        &scope,
        &guard,
        root.clone(),
        SourceRequest::Add(requests),
        options(),
        |prepared| {
            let reports = prepared.instruction_readers();
            assert_eq!(
                reports
                    .iter()
                    .map(|r| r.installation_id.clone())
                    .collect::<Vec<_>>(),
                expected_ids
            );
            for item in &installed.installations {
                let report = reports
                    .iter()
                    .find(|r| r.installation_id == item.owner.installation_id)
                    .expect("each installed owner has its own report");
                let compatibility = match item.owner.installed_name.as_str() {
                    "included" => InstructionCompatibility::UnconditionalMarkdown,
                    "excluded" => InstructionCompatibility::NotEvaluated,
                    _ => panic!("unexpected alias"),
                };
                assert_readers(std::slice::from_ref(report), PROJECT_READERS, compatibility);
                assert_eq!(report.report.location.root, root);
                assert_eq!(report.report.location.path, "AGENTS.md");
            }
            prepared.apply()
        },
    )
    .unwrap();
    assert_eq!(state(dest.path()), installed);
}

fn retained_root_mismatch_preflight(defects: &[&str]) {
    for defect in defects {
        for excluded in [true, false] {
            for same_alias_add in [false, true] {
                let dest = tempfile::tempdir().unwrap();
                let other = tempfile::tempdir().unwrap();
                let a = tempfile::tempdir().unwrap();
                let b = tempfile::tempdir().unwrap();
                let audience = if excluded { "audience: [claude]\n" } else { "" };
                write_rule(a.path(), "Alpha", audience);
                write_rule(b.path(), "Beta", audience);
                let root = root(dest.path(), OwnershipScope::Project);
                apply(
                    &root,
                    SourceRequest::Add(vec![add(a.path(), "alpha"), add(b.path(), "beta")]),
                );
                let mut retained = state(dest.path());
                let first = retained.installations.first().unwrap().owner.clone();
                let last = retained.installations.last().unwrap().owner.clone();
                let source = |alias: &str| match alias {
                    "alpha" => a.path(),
                    "beta" => b.path(),
                    _ => panic!("unexpected alias"),
                };
                let foreign = match *defect {
                    "path" => InstallationRoot {
                        path: other.path().into(),
                        ..root.clone()
                    },
                    "raw-path" => InstallationRoot {
                        path: root.path.join("."),
                        ..root.clone()
                    },
                    "scope" => InstallationRoot {
                        scope: OwnershipScope::User,
                        ..root.clone()
                    },
                    _ => panic!("unexpected defect"),
                };
                retained.installations.last_mut().unwrap().root = foreign.clone();
                let path = if foreign.scope == OwnershipScope::User {
                    ".codex/AGENTS.md"
                } else {
                    "AGENTS.md"
                };
                for artifact in &mut retained.artifacts {
                    if artifact
                        .owners
                        .iter()
                        .any(|owner| owner.installation_id == last.installation_id)
                    {
                        artifact.root = foreign.clone();
                        artifact.path = path.into();
                    }
                }
                for association in &mut retained.owner_consumers {
                    if association.installation_id == last.installation_id {
                        association.root = foreign.clone();
                        association.path = path.into();
                    }
                }
                for contract in &mut retained.owner_contracts {
                    if contract.installation_id == last.installation_id {
                        contract.root = foreign.clone();
                        contract.path = path.into();
                    }
                }
                // All retained associations/contracts align with the foreign root.
                // Delete the first selected owner's source to expose premature resolution.
                write_state(dest.path(), &retained);
                std::fs::remove_dir_all(source(&first.installed_name)).unwrap();
                let before = std::fs::read(dest.path().join(".upskill-lock.json")).unwrap();
                let instructions = std::fs::read(dest.path().join("AGENTS.md")).ok();
                let request = if same_alias_add {
                    SourceRequest::Add(vec![
                        add(source(&first.installed_name), &first.installed_name),
                        add(source(&last.installed_name), &last.installed_name),
                    ])
                } else {
                    SourceRequest::Update(Selection::Names(vec![
                        first.installed_name,
                        last.installed_name,
                    ]))
                };
                let scope = QualifiedScope::open(&root).unwrap();
                let guard = writer(&scope);
                let error = with_prepared_codex(
                    &scope,
                    &guard,
                    root.clone(),
                    request,
                    options(),
                    |_| -> anyhow::Result<()> {
                        panic!("foreign retained root must reject before callback")
                    },
                )
                .err()
                .expect("foreign retained root must reject");
                assert!(
                    error.to_string().contains(
                        "retained Codex installation root differs from declared session root"
                    ),
                    "{defect}, excluded={excluded}, add={same_alias_add}: {error:#}"
                );
                assert_eq!(
                    std::fs::read(dest.path().join(".upskill-lock.json")).unwrap(),
                    before
                );
                assert_eq!(
                    std::fs::read(dest.path().join("AGENTS.md")).ok(),
                    instructions
                );
                assert_eq!(std::fs::read_dir(other.path()).unwrap().count(), 0);
                assert!(!dest.path().join(".codex").exists());
                assert!(!scope.has_pending_operation().unwrap());
            }
        }
    }
}

// Catches root checks against recorded state and normalized rather than raw root paths.
#[test]
fn retained_path_mismatch_preflights_complete_selection_before_source_resolution() {
    retained_root_mismatch_preflight(&["path", "raw-path"]);
}

// Catches missing scope authority even when the foreign relative destination is canonical.
#[test]
fn retained_scope_mismatch_preflights_complete_selection_before_source_resolution() {
    retained_root_mismatch_preflight(&["scope"]);
}

// Catches excluded root mismatches reaching compilation/callback on reinclusion.
#[test]
fn excluded_scope_mismatch_rejects_before_reinclusion_callback() {
    for same_alias_add in [false, true] {
        let dest = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        write_rule(source.path(), "Excluded", "audience: [claude]\n");
        let root = root(dest.path(), OwnershipScope::Project);
        apply(&root, SourceRequest::Add(vec![add(source.path(), "alias")]));
        let mut retained = state(dest.path());
        assert!(retained.artifacts.is_empty());
        retained.installations[0].root.scope = OwnershipScope::User;
        write_state(dest.path(), &retained);
        write_rule(source.path(), "Reincluded", "audience: [codex]\n");
        let before = std::fs::read(dest.path().join(".upskill-lock.json")).unwrap();
        let request = if same_alias_add {
            SourceRequest::Add(vec![add(source.path(), "alias")])
        } else {
            SourceRequest::Update(Selection::All)
        };
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = writer(&scope);
        let error = with_prepared_codex(
            &scope,
            &guard,
            root.clone(),
            request,
            options(),
            |_| -> anyhow::Result<()> {
                panic!("scope-mismatched excluded owner reached callback")
            },
        )
        .err()
        .expect("scope mismatch must reject");
        assert!(
            error
                .to_string()
                .contains("retained Codex installation root differs from declared session root"),
            "{error:#}"
        );
        assert_eq!(
            std::fs::read(dest.path().join(".upskill-lock.json")).unwrap(),
            before
        );
        assert!(!dest.path().join("AGENTS.md").exists());
        assert!(!dest.path().join(".codex").exists());
        assert!(!scope.has_pending_operation().unwrap());
    }
}
