#![cfg(any(target_os = "linux", target_os = "macos"))]
use upskill::lifecycle::orchestration::{TargetSelection, effective_targets};
use upskill::target::TargetSurface;

#[test]
fn target_precedence_ignores_empty_overrides() {
    let c = [TargetSurface::ClaudeCli];
    let o = [TargetSurface::OpenCodeCli];
    let p = [TargetSurface::CopilotCli];
    for (invocation, project, global, expected) in [
        (&c[..], &o[..], &p[..], &c[..]),
        (&[][..], &o[..], &p[..], &o[..]),
        (&[][..], &[][..], &p[..], &p[..]),
        (&[][..], &[][..], &[][..], &c[..]),
    ] {
        assert_eq!(
            effective_targets(
                TargetSelection {
                    invocation,
                    project,
                    global
                },
                &c
            )
            .unwrap(),
            expected
        );
    }
    assert!(effective_targets(TargetSelection::default(), &[]).is_err());
    assert!(
        effective_targets(
            TargetSelection {
                invocation: &[TargetSurface::ClaudeCli, TargetSurface::ClaudeCli],
                ..Default::default()
            },
            &[]
        )
        .is_err()
    );
}

use upskill::files::permissions::ResourceExecutionPolicy;
use upskill::filesystem::{ExecutionOutcome, QualifiedScope};
use upskill::generate::Client;
use upskill::lifecycle::desired::CompilerBinding;
use upskill::lifecycle::orchestration::{
    AddRule, CompilerContext, OrchestrationOutcome, SourceOptions, SourceRequest, with_prepared,
};
use upskill::lifecycle::selection::Selection;
use upskill::lifecycle::state::{Reference, parse_state};
use upskill::ownership::{InstallationRoot, OwnershipScope};
use upskill::source::InstallSource;
use upskill::target::TargetProfile;

fn context() -> CompilerContext {
    CompilerContext {
        bindings: vec![CompilerBinding {
            client: Client::Claude,
            consumers: vec![TargetSurface::ClaudeCli],
        }],
        profiles: vec![
            TargetProfile::new(
                TargetSurface::ClaudeCli.family(),
                TargetSurface::ClaudeCli,
                1,
            )
            .unwrap(),
        ],
        execution: ResourceExecutionPolicy::Disabled,
    }
}
fn root(path: &std::path::Path) -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: path.into(),
    }
}
fn add(path: &std::path::Path) -> SourceRequest {
    SourceRequest::Add(vec![AddRule {
        source: InstallSource::LocalPath(path.into()),
        source_name: "source".into(),
        installed_name: "alias".into(),
    }])
}
fn write_rule(path: &std::path::Path, body: &str) {
    std::fs::write(
        path.join("RULE.md"),
        format!("---\nschema: 1\nname: source\ndescription: demo\n---\n\n# {body}\n"),
    )
    .unwrap();
}
fn run(root: &InstallationRoot, request: SourceRequest) -> OrchestrationOutcome {
    let scope = QualifiedScope::open(root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    with_prepared(
        &scope,
        &guard,
        root.clone(),
        request,
        SourceOptions {
            targets: TargetSelection::default(),
            add_defaults: &[TargetSurface::ClaudeCli],
            compiler: &context(),
        },
        |prepared| prepared.apply(),
    )
    .unwrap()
}
fn state(path: &std::path::Path) -> upskill::lifecycle::state::CombinedState {
    parse_state(&std::fs::read_to_string(path.join(".upskill-lock.json")).unwrap()).unwrap()
}
#[test]
fn local_add_update_remove_retains_identity_alias_and_exact_source_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "First");
    let root = root(tmp.path());
    assert!(
        matches!(run(&root, add(source.path())), OrchestrationOutcome::Ready(report) if report.outcome == ExecutionOutcome::Complete)
    );
    let first = state(tmp.path());
    let id = first.installations[0].owner.installation_id.clone();
    assert_eq!(id.len(), 64);
    assert_eq!(first.installations[0].owner.installed_name, "alias");
    assert_eq!(
        first.references,
        vec![Reference::DirectInstallation {
            installation_id: id.clone()
        }]
    );
    let output = tmp.path().join(&first.artifacts[0].path);
    assert!(std::fs::read_to_string(&output).unwrap().contains("First"));
    write_rule(source.path(), "Repeated add refresh");
    run(&root, add(source.path()));
    let repeated = state(tmp.path());
    assert_eq!(repeated.installations.len(), 1);
    assert_eq!(repeated.installations[0].owner.installation_id, id);
    assert_eq!(repeated.references, first.references);
    assert_ne!(
        repeated.installations[0].source_evidence,
        first.installations[0].source_evidence
    );
    assert!(
        std::fs::read_to_string(&output)
            .unwrap()
            .contains("Repeated add refresh")
    );
    write_rule(source.path(), "Second");
    run(
        &root,
        SourceRequest::Update(Selection::Names(vec!["alias".into()])),
    );
    let second = state(tmp.path());
    assert_eq!(second.installations[0].owner.installation_id, id);
    assert_ne!(
        second.installations[0].source_evidence,
        first.installations[0].source_evidence
    );
    assert!(std::fs::read_to_string(&output).unwrap().contains("Second"));
    std::fs::write(output.parent().unwrap().join("manual.md"), "manual").unwrap();
    run(
        &root,
        SourceRequest::Remove(Selection::Names(vec!["alias".into()])),
    );
    assert!(!output.exists());
    assert_eq!(
        std::fs::read_to_string(output.parent().unwrap().join("manual.md")).unwrap(),
        "manual"
    );
    assert!(state(tmp.path()).installations.is_empty());
}

#[test]
fn stale_source_after_preparation_rejects_without_destination_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Captured");
    let root = root(tmp.path());
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let result = with_prepared(
        &scope,
        &guard,
        root,
        add(source.path()),
        SourceOptions {
            targets: TargetSelection::default(),
            add_defaults: &[TargetSurface::ClaudeCli],
            compiler: &context(),
        },
        |prepared| {
            write_rule(source.path(), "Changed after preparation");
            prepared.apply()
        },
    );
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
}

#[test]
fn manual_output_conflict_preserves_entire_state_and_files() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Original");
    let root = root(tmp.path());
    run(&root, add(source.path()));
    let before = std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap();
    let output = tmp.path().join(&state(tmp.path()).artifacts[0].path);
    std::fs::write(&output, "Manual edit").unwrap();
    write_rule(source.path(), "New source");
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    assert!(
        with_prepared(
            &scope,
            &guard,
            root,
            SourceRequest::Update(Selection::All),
            SourceOptions {
                targets: TargetSelection::default(),
                add_defaults: &[],
                compiler: &context(),
            },
            |prepared| prepared.apply()
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        before
    );
    assert_eq!(std::fs::read_to_string(output).unwrap(), "Manual edit");
}

#[test]
fn migrated_disjoint_history_survives_and_complete_legacy_selection_blocks_before_source_work() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Owned");
    let legacy = br#"{"schema":1,"items":[{"kind":"skill","name":"legacy","source":"unresolved","required_by":[],"clients":[]}],"bundles":[]}"#;
    std::fs::write(tmp.path().join(".upskill-lock.json"), legacy).unwrap();
    let root = root(tmp.path());
    run(&root, add(source.path()));
    assert_eq!(
        state(tmp.path()).legacy.unwrap().items[0].source,
        "unresolved"
    );
    assert_eq!(
        std::fs::read(
            tmp.path()
                .join(upskill::lifecycle::application::SCHEMA1_BACKUP_NAME)
        )
        .unwrap(),
        legacy
    );
    std::fs::remove_file(source.path().join("RULE.md")).unwrap();
    let before = std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(matches!(
        run(&root, SourceRequest::Update(Selection::All)),
        OrchestrationOutcome::Blocked(_)
    ));
    assert!(matches!(
        run(
            &root,
            SourceRequest::Update(Selection::Names(vec!["alias".into(), "legacy".into()]))
        ),
        OrchestrationOutcome::Blocked(_)
    ));
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        before
    );
    run(
        &root,
        SourceRequest::Remove(Selection::Names(vec!["alias".into()])),
    );
    assert_eq!(
        state(tmp.path()).legacy.unwrap().items[0].source,
        "unresolved"
    );
}

#[test]
fn update_configured_targets_override_history_and_removal_uses_recorded_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Target");
    let root = root(tmp.path());
    run(&root, add(source.path()));
    let old_output = tmp.path().join(&state(tmp.path()).artifacts[0].path);
    let compiler = CompilerContext {
        bindings: vec![CompilerBinding {
            client: Client::OpenCode,
            consumers: vec![TargetSurface::OpenCodeCli],
        }],
        profiles: vec![
            TargetProfile::new(
                TargetSurface::OpenCodeCli.family(),
                TargetSurface::OpenCodeCli,
                1,
            )
            .unwrap(),
        ],
        execution: ResourceExecutionPolicy::Disabled,
    };
    {
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        with_prepared(
            &scope,
            &guard,
            root.clone(),
            SourceRequest::Update(Selection::All),
            SourceOptions {
                targets: TargetSelection {
                    invocation: &[],
                    project: &[TargetSurface::OpenCodeCli],
                    global: &[TargetSurface::ClaudeCli],
                },
                add_defaults: &[],
                compiler: &compiler,
            },
            |prepared| prepared.apply(),
        )
        .unwrap();
    }
    let updated = state(tmp.path());
    assert_eq!(
        updated.installations[0].requested,
        vec![TargetSurface::OpenCodeCli]
    );
    assert_eq!(
        updated.artifacts[0].consumers,
        vec![TargetSurface::OpenCodeCli]
    );
    assert!(!old_output.exists());
    let output = tmp.path().join(&updated.artifacts[0].path);
    assert!(output.exists());
    run(&root, SourceRequest::Remove(Selection::All));
    assert!(!output.exists());
}

fn clear_git_environment(command: &mut std::process::Command) {
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
}

fn git(path: &std::path::Path, args: &[&str]) -> String {
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
#[test]
fn git_branch_advancement_updates_actual_revision_without_changing_id_or_requested_binding() {
    const CHILD: &str = "UPSKILL_ORCHESTRATION_GIT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        git(repo.path(), &["init", "--quiet", "--initial-branch=main"]);
        std::fs::create_dir(repo.path().join("item")).unwrap();
        write_rule(&repo.path().join("item"), "First commit");
        git(repo.path(), &["add", "."]);
        git(
            repo.path(),
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
        std::fs::write(
            home.path().join(".gitconfig"),
            format!(
                "[url \"file://{}\"]\n    insteadOf = https://example.invalid/rules\n",
                repo.path().display()
            ),
        )
        .unwrap();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        clear_git_environment(&mut command);
        let output = command.args(["--exact", "git_branch_advancement_updates_actual_revision_without_changing_id_or_requested_binding", "--nocapture"])
            .env(CHILD, repo.path()).env("HOME", home.path()).env("USERPROFILE", home.path())
            .env("GIT_CONFIG_GLOBAL", home.path().join(".gitconfig")).env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TRACE2_EVENT", home.path().join("source-trace.json"))
            .env_remove("GIT_CONFIG_COUNT").env_remove("GIT_CONFIG_PARAMETERS").env_remove("GIT_DIR").env_remove("GIT_WORK_TREE")
            .output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let repo = std::path::PathBuf::from(std::env::var_os(CHILD).unwrap());
    let tmp = tempfile::tempdir().unwrap();
    let root = root(tmp.path());
    let source = InstallSource::Git(upskill::source::GitRepo {
        url: "https://example.invalid/rules".into(),
        git_ref: Some("main".into()),
        subfolder: Some("item".into()),
    });
    let label = source.to_string();
    run(
        &root,
        SourceRequest::Add(vec![AddRule {
            source,
            source_name: "source".into(),
            installed_name: "alias".into(),
        }]),
    );
    let first = state(tmp.path()).installations.remove(0);
    assert_eq!(
        first.owner.source_revision.as_deref(),
        Some(git(&repo, &["rev-parse", "HEAD"]).as_str())
    );
    assert_eq!(first.owner.source, label);
    write_rule(&repo.join("item"), "Second commit");
    git(&repo, &["add", "."]);
    git(
        &repo,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "advance",
        ],
    );
    run(&root, SourceRequest::Update(Selection::All));
    let second_state = state(tmp.path());
    let second = &second_state.installations[0];
    assert_eq!(second.owner.installation_id, first.owner.installation_id);
    assert_eq!(second.owner.source, label);
    assert_eq!(
        second.owner.source_revision.as_deref(),
        Some(git(&repo, &["rev-parse", "HEAD"]).as_str())
    );
    assert_ne!(second.owner.source_revision, first.owner.source_revision);
    let upskill::lifecycle::state::SourceEvidence::Git {
        resolved_revision,
        manifest_sha256,
    } = &second.source_evidence
    else {
        panic!()
    };
    assert_eq!(
        Some(resolved_revision),
        second.owner.source_revision.as_ref()
    );
    assert_eq!(manifest_sha256.len(), 64);
    let output = tmp.path().join(&second_state.artifacts[0].path);
    assert!(
        std::fs::read_to_string(&output)
            .unwrap()
            .contains("Second commit")
    );
    let state_before = std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap();
    let output_before = std::fs::read(&output).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let result = with_prepared(
        &scope,
        &guard,
        root.clone(),
        SourceRequest::Update(Selection::All),
        SourceOptions {
            targets: TargetSelection::default(),
            add_defaults: &[],
            compiler: &context(),
        },
        |prepared| {
            let trace =
                std::fs::read_to_string(std::env::var_os("GIT_TRACE2_EVENT").unwrap()).unwrap();
            let metadata = trace
                .lines()
                .filter_map(|line| {
                    let event: serde_json::Value = serde_json::from_str(line).ok()?;
                    let argv = event.get("argv")?.as_array()?;
                    let at = argv
                        .iter()
                        .position(|arg| arg.as_str() == Some("--git-dir"))?;
                    argv.get(at + 1)?.as_str().map(str::to_owned)
                })
                .next_back()
                .expect("retained checkout Git metadata in trace");
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
                    "same tree, different identity",
                ],
            );
            assert_ne!(old, new);
            git(&repo, &["--git-dir", &metadata, "update-ref", "HEAD", &new]);
            prepared.apply()
        },
    );
    assert!(
        format!(
            "{:#}",
            result.err().expect("changed checkout identity must reject")
        )
        .contains("source checkout revision changed")
    );
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        state_before
    );
    assert_eq!(std::fs::read(&output).unwrap(), output_before);
    assert!(!scope.has_pending_operation().unwrap());
    drop(guard);
    drop(scope);
    run(&root, SourceRequest::Remove(Selection::All));
    assert!(!output.exists());
}

#[test]
fn audience_filter_retains_requested_targets_and_empty_output_history() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(
        source.path().join("RULE.md"),
        "---\nschema: 1\nname: source\ndescription: demo\naudience: [copilot]\n---\n\n# Filtered\n",
    )
    .unwrap();
    let root = root(tmp.path());
    run(&root, add(source.path()));
    let first = state(tmp.path());
    assert_eq!(
        first.installations[0].requested,
        vec![TargetSurface::ClaudeCli]
    );
    assert!(first.artifacts.is_empty());
    assert_eq!(first.references.len(), 1);
    run(&root, SourceRequest::Update(Selection::All));
    let second = state(tmp.path());
    assert_eq!(
        second.installations[0].owner.installation_id,
        first.installations[0].owner.installation_id
    );
    assert!(second.artifacts.is_empty());
    run(&root, SourceRequest::Remove(Selection::All));
    assert!(state(tmp.path()).installations.is_empty());
}

#[test]
fn resource_policy_is_applied_to_output_and_contract_and_manual_neighbors_survive() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Resource");
    std::fs::create_dir(source.path().join("scripts")).unwrap();
    std::fs::write(
        source.path().join("scripts/check.sh"),
        b"#!/bin/sh\nexit 0\n",
    )
    .unwrap();
    std::fs::set_permissions(
        source.path().join("scripts/check.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let root = root(tmp.path());
    {
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let compiler = CompilerContext {
            execution: ResourceExecutionPolicy::OwnerExecutable,
            ..context()
        };
        with_prepared(
            &scope,
            &guard,
            root.clone(),
            add(source.path()),
            SourceOptions {
                targets: TargetSelection::default(),
                add_defaults: &[TargetSurface::ClaudeCli],
                compiler: &compiler,
            },
            |prepared| prepared.apply(),
        )
        .unwrap();
    }
    let installed = state(tmp.path());
    let artifact = installed
        .artifacts
        .iter()
        .find(|artifact| artifact.path.ends_with("scripts/check.sh"))
        .unwrap();
    let output = tmp.path().join(&artifact.path);
    assert_eq!(std::fs::read(&output).unwrap(), b"#!/bin/sh\nexit 0\n");
    assert_eq!(
        std::fs::metadata(&output).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let contract = installed
        .owner_contracts
        .iter()
        .find(|contract| contract.path == artifact.path)
        .unwrap();
    assert!(matches!(
        contract.content,
        upskill::lifecycle::contracts::ContractContent::File {
            mode: Some(0o700),
            ..
        }
    ));
    let manual = output.parent().unwrap().join("manual.sh");
    std::fs::write(&manual, "manual resource").unwrap();
    run(&root, SourceRequest::Remove(Selection::All));
    assert!(!output.exists());
    assert_eq!(std::fs::read_to_string(manual).unwrap(), "manual resource");
}

#[test]
fn different_source_alias_reuse_rejects_before_resolver_fetch() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Owned");
    let root = root(tmp.path());
    run(&root, add(source.path()));
    let before = std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let request = SourceRequest::Add(vec![AddRule {
        source: InstallSource::Git(upskill::source::GitRepo {
            url: "https://example.invalid/never-fetch".into(),
            git_ref: None,
            subfolder: None,
        }),
        source_name: "source".into(),
        installed_name: "alias".into(),
    }]);
    let error = with_prepared(
        &scope,
        &guard,
        root,
        request,
        SourceOptions {
            targets: TargetSelection::default(),
            add_defaults: &[TargetSurface::ClaudeCli],
            compiler: &context(),
        },
        |prepared| prepared.apply(),
    )
    .err()
    .unwrap();
    assert!(
        error.to_string().contains("alias already belongs"),
        "{error:#}"
    );
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        before
    );
}

#[test]
fn fixture_git_commands_ignore_inherited_repository_overrides() {
    const CHILD: &str = "UPSKILL_ORCHESTRATION_INDEX_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let external = tmp.path().join("external-index");
        std::fs::write(&external, b"external index sentinel").unwrap();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        clear_git_environment(&mut command);
        let output = command
            .args([
                "--exact",
                "fixture_git_commands_ignore_inherited_repository_overrides",
                "--nocapture",
            ])
            .env(CHILD, &external)
            .env("GIT_INDEX_FILE", &external)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(std::fs::read(external).unwrap(), b"external index sentinel");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    git(tmp.path(), &["init", "--quiet"]);
    write_rule(tmp.path(), "Fixture index confinement");
    git(tmp.path(), &["add", "."]);
    git(
        tmp.path(),
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
    assert_eq!(
        std::fs::read(std::env::var_os(CHILD).unwrap()).unwrap(),
        b"external index sentinel"
    );
}

#[test]
fn complete_multi_source_add_update_and_remove_retains_each_installation() {
    let destination = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    write_rule(first.path(), "First source");
    write_rule(second.path(), "Second source");
    let root = root(destination.path());
    run(
        &root,
        SourceRequest::Add(vec![
            AddRule {
                source: InstallSource::LocalPath(first.path().into()),
                source_name: "source".into(),
                installed_name: "first".into(),
            },
            AddRule {
                source: InstallSource::LocalPath(second.path().into()),
                source_name: "source".into(),
                installed_name: "second".into(),
            },
        ]),
    );
    let initial = state(destination.path());
    assert_eq!(initial.installations.len(), 2);
    assert_eq!(initial.artifacts.len(), 2);
    assert_eq!(initial.references.len(), 2);
    assert_eq!(initial.owner_contracts.len(), 2);
    assert_ne!(
        initial.installations[0].owner.installation_id,
        initial.installations[1].owner.installation_id
    );
    let paths = initial
        .artifacts
        .iter()
        .map(|artifact| destination.path().join(&artifact.path))
        .collect::<Vec<_>>();
    for (alias, source, body) in [
        ("first", first.path(), "First source"),
        ("second", second.path(), "Second source"),
    ] {
        let item = initial
            .installations
            .iter()
            .find(|item| item.owner.installed_name == alias)
            .unwrap();
        assert_eq!(
            item.owner.source,
            format!("local:{}", source.canonicalize().unwrap().display())
        );
        assert_eq!(item.owner.source_name, "source");
        assert_eq!(item.root, root);
        let artifact = initial
            .artifacts
            .iter()
            .find(|artifact| artifact.owners.contains(&item.owner))
            .unwrap();
        assert_eq!(artifact.path, format!(".claude/rules/{alias}.md"));
        assert!(
            std::fs::read_to_string(destination.path().join(&artifact.path))
                .unwrap()
                .contains(body)
        );
        assert!(initial.references.contains(&Reference::DirectInstallation {
            installation_id: item.owner.installation_id.clone()
        }));
        let contract = initial
            .owner_contracts
            .iter()
            .find(|contract| {
                contract.installation_id == item.owner.installation_id
                    && contract.path == artifact.path
            })
            .unwrap();
        assert_eq!(contract.root, root);
        assert_eq!(contract.contribution_id, None);
        let upskill::lifecycle::state::SourceEvidence::Local { manifest_sha256 } =
            &item.source_evidence
        else {
            panic!()
        };
        assert_eq!(&contract.source_manifest_sha256, manifest_sha256);
        let upskill::ownership::ArtifactContent::File {
            content_sha256,
            mode,
        } = &artifact.content
        else {
            panic!()
        };
        assert_eq!(
            contract.content,
            upskill::lifecycle::contracts::ContractContent::File {
                content_sha256: content_sha256.clone(),
                mode: *mode
            }
        );
        assert_eq!(
            contract
                .profiles
                .iter()
                .map(|profile| profile.surface)
                .collect::<Vec<_>>(),
            vec![TargetSurface::ClaudeCli]
        );
    }
    write_rule(first.path(), "First refreshed");
    run(
        &root,
        SourceRequest::Update(Selection::Names(vec!["first".into()])),
    );
    let updated = state(destination.path());
    assert_eq!(updated.references, initial.references);
    assert_eq!(updated.installations.len(), 2);
    for prior in &initial.installations {
        let current = updated
            .installations
            .iter()
            .find(|item| item.owner.installation_id == prior.owner.installation_id)
            .unwrap();
        assert_eq!(current.owner, prior.owner);
        if prior.owner.installed_name == "first" {
            assert_ne!(current.source_evidence, prior.source_evidence);
        } else {
            assert_eq!(current, prior);
            assert_eq!(
                updated
                    .artifacts
                    .iter()
                    .find(|artifact| artifact.owners.contains(&current.owner)),
                initial
                    .artifacts
                    .iter()
                    .find(|artifact| artifact.owners.contains(&prior.owner))
            );
            assert_eq!(
                updated
                    .owner_contracts
                    .iter()
                    .filter(|contract| contract.installation_id == current.owner.installation_id)
                    .collect::<Vec<_>>(),
                initial
                    .owner_contracts
                    .iter()
                    .filter(|contract| contract.installation_id == prior.owner.installation_id)
                    .collect::<Vec<_>>()
            );
        }
    }
    assert!(
        std::fs::read_to_string(destination.path().join(".claude/rules/first.md"))
            .unwrap()
            .contains("First refreshed")
    );
    assert!(
        std::fs::read_to_string(destination.path().join(".claude/rules/second.md"))
            .unwrap()
            .contains("Second source")
    );
    run(&root, SourceRequest::Remove(Selection::All));
    assert!(state(destination.path()).installations.is_empty());
    assert!(paths.iter().all(|path| !path.exists()));
}

fn install_captured_instruction(root: &InstallationRoot, source: &std::path::Path, bundle: bool) {
    use upskill::instructions::plan::{DesiredInstructionContribution, InstructionLocation};
    use upskill::lifecycle::commands::{
        CapturedUpsert, LifecycleSession, PrepareOutcome, SessionOutcome,
    };
    use upskill::lifecycle::sources::{ResolvedRuleSource, RuleSourceBinding};
    use upskill::operations::plan::DesiredArtifact;
    let resolved = ResolvedRuleSource::resolve(&InstallSource::LocalPath(source.into())).unwrap();
    let capture = resolved.capture().unwrap();
    let compiler = context();
    let compiled = capture
        .compile(RuleSourceBinding {
            installation_id: "captured-owner",
            expected_source_name: "source",
            installed_name: "alias",
            root,
            requested: &[TargetSurface::ClaudeCli],
            bindings: &compiler.bindings,
            profiles: &compiler.profiles,
        })
        .unwrap();
    let mut set = compiled.set;
    set.artifacts = vec![DesiredArtifact::Instruction(
        DesiredInstructionContribution {
            location: InstructionLocation {
                root: root.clone(),
                path: "AGENTS.md".into(),
            },
            id: "selected".into(),
            markdown: "Captured instruction\n".into(),
            consumers: vec![TargetSurface::ClaudeCli],
        },
    )];
    let scope = QualifiedScope::open(root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let SessionOutcome::Ready(session) =
        LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
    else {
        panic!()
    };
    let PrepareOutcome::Ready(prepared) = session
        .prepare_upsert(
            None,
            &[capture.snapshot()],
            ResourceExecutionPolicy::Disabled,
            |_| {
                let mut request = CapturedUpsert {
                    sets: vec![set],
                    add_references: vec![Reference::DirectInstallation {
                        installation_id: "captured-owner".into(),
                    }],
                    ..Default::default()
                };
                if bundle {
                    request.bundles.push(upskill::lifecycle::state::BundleNode {
                        id: "bundle-id".into(),
                        source: "local:bundle".into(),
                        name: "bundle-alias".into(),
                        source_revision: None,
                    });
                    request.add_references.push(Reference::DirectBundle {
                        bundle_id: "bundle-id".into(),
                    });
                }
                Ok(request)
            },
        )
        .unwrap()
    else {
        panic!()
    };
    prepared.apply().unwrap();
}

#[test]
fn source_upserts_reject_captured_instruction_owners_before_source_work_but_remove_is_allowed() {
    for (repeated_add, unavailable) in [(false, false), (true, false), (false, true), (true, true)]
    {
        let destination = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        write_rule(source.path(), "Captured");
        let root = root(destination.path());
        install_captured_instruction(&root, source.path(), false);
        let before = std::fs::read(destination.path().join(".upskill-lock.json")).unwrap();
        let instructions = std::fs::read(destination.path().join("AGENTS.md")).unwrap();
        // An absent source makes source-resolution ordering observable.
        if unavailable {
            std::fs::remove_file(source.path().join("RULE.md")).unwrap();
        }
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let request = if repeated_add {
            add(source.path())
        } else {
            SourceRequest::Update(Selection::All)
        };
        let result: anyhow::Result<OrchestrationOutcome<()>> = with_prepared(
            &scope,
            &guard,
            root.clone(),
            request,
            SourceOptions {
                targets: TargetSelection::default(),
                add_defaults: &[TargetSurface::ClaudeCli],
                compiler: &context(),
            },
            |_| panic!("instruction owner must reject before callback"),
        );
        let error = result
            .err()
            .expect("instruction owner must be rejected")
            .to_string();
        assert!(error.contains("captured instruction adapter"), "{error}");
        assert_eq!(
            std::fs::read(destination.path().join(".upskill-lock.json")).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read(destination.path().join("AGENTS.md")).unwrap(),
            instructions
        );
        assert!(!destination.path().join(".claude").exists());
        drop(guard);
        assert!(
            matches!(run(&root, SourceRequest::Remove(Selection::All)), OrchestrationOutcome::Ready(report) if report.outcome == ExecutionOutcome::Complete)
        );
        assert!(state(destination.path()).installations.is_empty());
    }
}

#[test]
fn add_rejects_owned_bundle_alias_before_source_work() {
    let destination = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Captured");
    let root = root(destination.path());
    install_captured_instruction(&root, source.path(), true);
    let before = std::fs::read(destination.path().join(".upskill-lock.json")).unwrap();
    let instructions = std::fs::read(destination.path().join("AGENTS.md")).unwrap();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let result: anyhow::Result<OrchestrationOutcome<()>> = with_prepared(
        &scope,
        &guard,
        root.clone(),
        SourceRequest::Add(vec![AddRule {
            source: InstallSource::LocalPath(destination.path().join("missing-source")),
            source_name: "source".into(),
            installed_name: "bundle-alias".into(),
        }]),
        SourceOptions {
            targets: TargetSelection::default(),
            add_defaults: &[TargetSurface::ClaudeCli],
            compiler: &context(),
        },
        |_| panic!("bundle alias must reject before callback"),
    );
    let error = result
        .err()
        .expect("bundle collision must reject")
        .to_string();
    assert!(error.contains("owned bundle"), "{error}");
    assert_eq!(
        std::fs::read(destination.path().join(".upskill-lock.json")).unwrap(),
        before
    );
    assert_eq!(
        std::fs::read(destination.path().join("AGENTS.md")).unwrap(),
        instructions
    );
    assert!(!destination.path().join(".claude").exists());
}

#[test]
fn same_source_distinct_aliases_have_stable_independent_lifecycles() {
    let destination = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    write_rule(source.path(), "Shared initial");
    let root = root(destination.path());
    let request = || {
        SourceRequest::Add(
            ["first", "second"]
                .into_iter()
                .map(|alias| AddRule {
                    source: InstallSource::LocalPath(source.path().into()),
                    source_name: "source".into(),
                    installed_name: alias.into(),
                })
                .collect(),
        )
    };
    run(&root, request());
    let initial = state(destination.path());
    assert_eq!(initial.installations.len(), 2);
    assert_ne!(
        initial.installations[0].owner.installation_id,
        initial.installations[1].owner.installation_id
    );
    run(&root, request());
    assert_eq!(state(destination.path()), initial);
    write_rule(source.path(), "Selected refresh");
    run(
        &root,
        SourceRequest::Update(Selection::Names(vec!["first".into()])),
    );
    let updated = state(destination.path());
    for prior in &initial.installations {
        let current = updated
            .installations
            .iter()
            .find(|item| item.owner.installation_id == prior.owner.installation_id)
            .unwrap();
        if prior.owner.installed_name == "first" {
            assert_ne!(current.source_evidence, prior.source_evidence);
        } else {
            assert_eq!(current, prior);
        }
    }
    assert!(
        std::fs::read_to_string(destination.path().join(".claude/rules/first.md"))
            .unwrap()
            .contains("Selected refresh")
    );
    assert!(
        std::fs::read_to_string(destination.path().join(".claude/rules/second.md"))
            .unwrap()
            .contains("Shared initial")
    );
    run(
        &root,
        SourceRequest::Remove(Selection::Names(vec!["first".into()])),
    );
    let removed = state(destination.path());
    assert_eq!(
        removed.installations,
        initial
            .installations
            .into_iter()
            .filter(|item| item.owner.installed_name == "second")
            .collect::<Vec<_>>()
    );
    assert!(!destination.path().join(".claude/rules/first.md").exists());
    assert!(
        std::fs::read_to_string(destination.path().join(".claude/rules/second.md"))
            .unwrap()
            .contains("Shared initial")
    );
    assert_eq!(
        removed.references,
        vec![Reference::DirectInstallation {
            installation_id: removed.installations[0].owner.installation_id.clone()
        }]
    );
}

#[test]
fn add_applies_invocation_project_global_and_default_target_precedence() {
    let c = [TargetSurface::ClaudeCli];
    let o = [TargetSurface::OpenCodeCli];
    let p = [TargetSurface::CopilotCli];
    let defaults = [TargetSurface::CopilotVscodeLocal];
    let compiler = CompilerContext {
        bindings: vec![
            CompilerBinding {
                client: Client::Claude,
                consumers: c.to_vec(),
            },
            CompilerBinding {
                client: Client::OpenCode,
                consumers: o.to_vec(),
            },
            CompilerBinding {
                client: Client::Copilot,
                consumers: vec![p[0], defaults[0]],
            },
        ],
        profiles: [c[0], o[0], p[0], defaults[0]]
            .into_iter()
            .map(|surface| TargetProfile::new(surface.family(), surface, 1).unwrap())
            .collect(),
        execution: ResourceExecutionPolicy::Disabled,
    };
    for (invocation, project, global, expected, route) in [
        (&c[..], &o[..], &p[..], &c[..], ".claude/rules/alias.md"),
        (
            &[][..],
            &o[..],
            &p[..],
            &o[..],
            ".agents/rules/alias/RULE.md",
        ),
        (
            &[][..],
            &[][..],
            &p[..],
            &p[..],
            ".github/instructions/alias.instructions.md",
        ),
        (
            &[][..],
            &[][..],
            &[][..],
            &defaults[..],
            ".github/instructions/alias.instructions.md",
        ),
    ] {
        let compiler = CompilerContext {
            bindings: compiler.bindings.clone(),
            profiles: expected
                .iter()
                .map(|surface| TargetProfile::new(surface.family(), *surface, 1).unwrap())
                .collect(),
            execution: compiler.execution,
        };
        let destination = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        write_rule(source.path(), "Target precedence");
        let root = root(destination.path());
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        assert!(
            matches!(with_prepared(&scope, &guard, root.clone(), add(source.path()), SourceOptions { targets: TargetSelection { invocation, project, global }, add_defaults: &defaults, compiler: &compiler }, |prepared| prepared.apply()).unwrap(), OrchestrationOutcome::Ready(report) if report.outcome == ExecutionOutcome::Complete)
        );
        let installed = state(destination.path());
        assert_eq!(installed.installations[0].requested, expected);
        assert_eq!(installed.owner_consumers[0].consumers, expected);
        assert_eq!(installed.artifacts[0].path, route);
        assert!(
            std::fs::read_to_string(destination.path().join(route))
                .unwrap()
                .contains("Target precedence")
        );
        assert_eq!(
            installed.owner_contracts[0]
                .profiles
                .iter()
                .map(|profile| profile.surface)
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn add_preflights_later_instruction_and_bundle_aliases_before_resolving_any_source() {
    for (alias, diagnostic) in [
        ("alias", "captured instruction adapter"),
        ("bundle-alias", "owned bundle"),
    ] {
        let destination = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        write_rule(source.path(), "Captured");
        let root = root(destination.path());
        install_captured_instruction(&root, source.path(), true);
        let before = state(destination.path());
        let bytes = std::fs::read(destination.path().join(".upskill-lock.json")).unwrap();
        let instructions = std::fs::read(destination.path().join("AGENTS.md")).unwrap();
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let result: anyhow::Result<OrchestrationOutcome<()>> = with_prepared(
            &scope,
            &guard,
            root.clone(),
            SourceRequest::Add(vec![
                AddRule {
                    source: InstallSource::LocalPath(
                        destination.path().join("missing-first-source"),
                    ),
                    source_name: "source".into(),
                    installed_name: "free-alias".into(),
                },
                AddRule {
                    source: InstallSource::LocalPath(source.path().into()),
                    source_name: "source".into(),
                    installed_name: alias.into(),
                },
            ]),
            SourceOptions {
                targets: TargetSelection::default(),
                add_defaults: &[TargetSurface::ClaudeCli],
                compiler: &context(),
            },
            |_| panic!("complete namespace preflight must reject before callback"),
        );
        let error = result
            .err()
            .expect("later namespace collision must reject")
            .to_string();
        assert!(error.contains(diagnostic), "{error}");
        assert_eq!(state(destination.path()), before);
        assert_eq!(
            std::fs::read(destination.path().join(".upskill-lock.json")).unwrap(),
            bytes
        );
        assert_eq!(
            std::fs::read(destination.path().join("AGENTS.md")).unwrap(),
            instructions
        );
        assert!(!destination.path().join(".claude").exists());
        assert!(!scope.has_pending_operation().unwrap());
    }
}

#[test]
fn source_rule_updates_beside_an_unselected_instruction_preserve_its_complete_evidence() {
    let destination = tempfile::tempdir().unwrap();
    let instruction_source = tempfile::tempdir().unwrap();
    let rule_source = tempfile::tempdir().unwrap();
    write_rule(instruction_source.path(), "Captured instruction source");
    write_rule(rule_source.path(), "First rule");
    let root = root(destination.path());
    install_captured_instruction(&root, instruction_source.path(), false);
    let before = state(destination.path());
    let instructions = std::fs::read(destination.path().join("AGENTS.md")).unwrap();
    let rule = || {
        SourceRequest::Add(vec![AddRule {
            source: InstallSource::LocalPath(rule_source.path().into()),
            source_name: "source".into(),
            installed_name: "separate-rule".into(),
        }])
    };
    for operation in ["add", "repeated-add", "update"] {
        write_rule(rule_source.path(), operation);
        let request = if operation == "update" {
            SourceRequest::Update(Selection::Names(vec!["separate-rule".into()]))
        } else {
            rule()
        };
        assert!(
            matches!(run(&root, request), OrchestrationOutcome::Ready(report) if report.outcome == ExecutionOutcome::Complete)
        );
        let current = state(destination.path());
        assert_eq!(
            std::fs::read(destination.path().join("AGENTS.md")).unwrap(),
            instructions
        );
        assert_eq!(
            current
                .installations
                .iter()
                .filter(|item| item.owner.installation_id == "captured-owner")
                .collect::<Vec<_>>(),
            before.installations.iter().collect::<Vec<_>>()
        );
        assert_eq!(
            current
                .artifacts
                .iter()
                .filter(|artifact| artifact
                    .owners
                    .iter()
                    .any(|owner| owner.installation_id == "captured-owner"))
                .collect::<Vec<_>>(),
            before.artifacts.iter().collect::<Vec<_>>()
        );
        assert_eq!(
            current
                .owner_consumers
                .iter()
                .filter(|value| value.installation_id == "captured-owner")
                .collect::<Vec<_>>(),
            before.owner_consumers.iter().collect::<Vec<_>>()
        );
        assert_eq!(current.references.iter().filter(|reference| matches!(reference, Reference::DirectInstallation { installation_id } if installation_id == "captured-owner")).collect::<Vec<_>>(), before.references.iter().collect::<Vec<_>>());
        assert_eq!(
            current
                .owner_contracts
                .iter()
                .filter(|value| value.installation_id == "captured-owner")
                .collect::<Vec<_>>(),
            before.owner_contracts.iter().collect::<Vec<_>>()
        );
        assert!(
            std::fs::read_to_string(destination.path().join(".claude/rules/separate-rule.md"))
                .unwrap()
                .contains(operation)
        );
    }
}
