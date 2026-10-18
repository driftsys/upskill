#![cfg(any(target_os = "linux", target_os = "macos"))]

use sha2::{Digest, Sha256};
use std::path::Path;
use upskill::generate::Client;
use upskill::instructions::plan::{DesiredInstructionContribution, InstructionLocation};
use upskill::lifecycle::contracts::{ConsumerContract, ContractContent};
use upskill::lifecycle::desired::CompilerBinding;
use upskill::lifecycle::desired::codex::{
    CompiledCodexInstructions, InstructionCompatibility, InstructionReaderRole,
};
use upskill::lifecycle::sources::{
    CapturedRuleSource, CodexSourceBinding, ResolvedRuleSource, RuleSourceBinding,
};
use upskill::lifecycle::state::SourceEvidence;
use upskill::operations::plan::DesiredArtifact;
use upskill::ownership::{InstallationRoot, OwnershipScope};
use upskill::source::{GitRepo, InstallSource};
use upskill::target::{CapabilityError, TargetFamily, TargetProfile, TargetSurface};

const CONTRIBUTION_ID: &str = "5e4c87efab6d80cf6ea17c6622a1cd76c6b91c7f883639054253f101c43d5da4";

fn write_rule(root: &Path, fields: &str, body: &str) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("RULE.md"),
        format!("---\nschema: 1\nname: source\ndescription: demo\n{fields}---\n{body}"),
    )
    .unwrap();
}

fn root(path: &Path, scope: OwnershipScope) -> InstallationRoot {
    InstallationRoot {
        scope,
        path: path.join("never-created-output"),
    }
}

fn location(root: &InstallationRoot) -> InstructionLocation {
    InstructionLocation {
        root: root.clone(),
        path: match root.scope {
            OwnershipScope::Project => "AGENTS.md",
            OwnershipScope::User => ".codex/AGENTS.md",
        }
        .into(),
    }
}

fn binding<'a>(
    root: &'a InstallationRoot,
    location: &'a InstructionLocation,
) -> CodexSourceBinding<'a> {
    CodexSourceBinding {
        installation_id: "test-installation",
        expected_source_name: "source",
        installed_name: "alias",
        root,
        requested: &[TargetSurface::CodexCli],
        profile: TargetSurface::CodexCli.profile(),
        location,
    }
}

fn contribution(result: &CompiledCodexInstructions) -> &DesiredInstructionContribution {
    let [DesiredArtifact::Instruction(value)] = result.desired.set.artifacts.as_slice() else {
        panic!("expected one instruction contribution")
    };
    value
}

fn readers(result: &CompiledCodexInstructions) -> Vec<(&str, u32, InstructionReaderRole)> {
    result
        .readers
        .readers
        .iter()
        .map(|reader| {
            (
                reader.profile.surface.name(),
                reader.profile.contract_revision,
                reader.role,
            )
        })
        .collect()
}

fn project_readers() -> Vec<(&'static str, u32, InstructionReaderRole)> {
    vec![
        ("codex.cli", 1, InstructionReaderRole::Requested),
        ("codex.ide", 1, InstructionReaderRole::PotentialIncidental),
        ("copilot.cli", 1, InstructionReaderRole::PotentialIncidental),
        (
            "copilot.vscode-local",
            1,
            InstructionReaderRole::PotentialIncidental,
        ),
        (
            "opencode.cli",
            1,
            InstructionReaderRole::PotentialIncidental,
        ),
    ]
}

fn user_readers() -> Vec<(&'static str, u32, InstructionReaderRole)> {
    vec![
        ("codex.cli", 1, InstructionReaderRole::Requested),
        ("codex.ide", 1, InstructionReaderRole::PotentialIncidental),
    ]
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn local_project_binds_actual_capture_and_alias_with_five_readers_and_sole_codex_owner() {
    let tmp = tempfile::tempdir().unwrap();
    let selected = tmp.path().join("actual-folder");
    write_rule(&selected, "", "# Captured\nUse clear names.\n");
    let source = ResolvedRuleSource::resolve(&InstallSource::LocalPath(selected.clone())).unwrap();
    let captured = source.capture().unwrap();
    let root = root(tmp.path(), OwnershipScope::Project);
    let location = location(&root);
    let result = captured.compile_codex(binding(&root, &location)).unwrap();
    let item = &result.desired.set.installation;
    assert_eq!(item.owner.installation_id, "test-installation");
    assert_eq!(item.owner.source_name, "source");
    assert_eq!(item.owner.installed_name, "alias");
    assert_eq!(
        item.owner.source,
        InstallSource::LocalPath(selected.canonicalize().unwrap()).to_string()
    );
    assert_eq!(item.owner.source_revision, None);
    assert_eq!(item.requested, vec![TargetSurface::CodexCli]);
    assert_eq!(
        item.source_evidence,
        SourceEvidence::Local {
            manifest_sha256: captured.snapshot().manifest().content_sha256.clone(),
        }
    );
    assert_eq!(item.root, root);
    let output = contribution(&result);
    assert_eq!(output.id, CONTRIBUTION_ID);
    assert_eq!(output.location.root, root);
    assert_eq!(output.location.path, "AGENTS.md");
    assert_eq!(output.markdown, "# Captured\n\nUse clear names.\n");
    assert_eq!(output.consumers, vec![TargetSurface::CodexCli]);
    assert_eq!(readers(&result), project_readers());
    assert_eq!(result.readers.location, location);
    assert_eq!(result.readers.contract_revision, 1);
    assert_eq!(
        result.readers.compatibility,
        InstructionCompatibility::UnconditionalMarkdown
    );
    assert!(result.desired.set.audience_excluded.is_empty());
    assert_contract(&result, "AGENTS.md");
    assert!(!root.path.exists());
    assert_eq!(std::fs::read_dir(&selected).unwrap().count(), 1);

    // Both compiler paths must retain the same resolver-derived identity and evidence.
    let historical = captured
        .compile(RuleSourceBinding {
            installation_id: "test-installation",
            expected_source_name: "source",
            installed_name: "alias",
            root: &root,
            requested: &[TargetSurface::ClaudeCli],
            bindings: &[CompilerBinding {
                client: Client::Claude,
                consumers: vec![TargetSurface::ClaudeCli],
            }],
            profiles: &[TargetSurface::ClaudeCli.profile()],
        })
        .unwrap();
    assert_eq!(historical.set.installation.owner, item.owner);
    assert_eq!(
        historical.set.installation.source_evidence,
        item.source_evidence
    );
    assert_eq!(
        historical.contracts[0].source_manifest_sha256,
        result.desired.contracts[0].source_manifest_sha256
    );
}

fn assert_contract(result: &CompiledCodexInstructions, path: &str) {
    let [contract] = result.desired.contracts.as_slice() else {
        panic!("one owner contract")
    };
    assert_eq!(contract.installation_id, "test-installation");
    assert_eq!(contract.root, result.desired.set.installation.root);
    assert_eq!(contract.path, path);
    assert_eq!(contract.contribution_id.as_deref(), Some(CONTRIBUTION_ID));
    let manifest = match &result.desired.set.installation.source_evidence {
        SourceEvidence::Local { manifest_sha256 }
        | SourceEvidence::Git {
            manifest_sha256, ..
        } => manifest_sha256,
    };
    assert_eq!(&contract.source_manifest_sha256, manifest);
    assert_eq!(
        contract.content,
        ContractContent::Instruction {
            content_sha256: hash(contribution(result).markdown.as_bytes())
        }
    );
    assert_eq!(
        contract.profiles,
        vec![ConsumerContract {
            surface: TargetSurface::CodexCli,
            contract_revision: 1
        }]
    );
}

#[test]
fn local_user_codex_audience_binds_exact_global_destination_and_two_readers() {
    let tmp = tempfile::tempdir().unwrap();
    let selected = tmp.path().join("rule");
    write_rule(&selected, "audience: [codex]\n", "Global Codex body\n");
    let source = ResolvedRuleSource::resolve(&InstallSource::LocalPath(selected)).unwrap();
    let captured = source.capture().unwrap();
    let root = root(tmp.path(), OwnershipScope::User);
    let result = captured
        .compile_codex(binding(&root, &location(&root)))
        .unwrap();
    assert_eq!(contribution(&result).location.path, ".codex/AGENTS.md");
    assert_eq!(contribution(&result).location.root, root);
    assert_eq!(
        contribution(&result).consumers,
        vec![TargetSurface::CodexCli]
    );
    assert_eq!(readers(&result), user_readers());
    assert_contract(&result, ".codex/AGENTS.md");
    assert!(!root.path.exists());
}

#[test]
fn audience_exclusion_keeps_complete_project_and_user_reports_without_owned_content() {
    let tmp = tempfile::tempdir().unwrap();
    let selected = tmp.path().join("rule");
    write_rule(&selected, "audience: [claude]\n", "Excluded body\n");
    let source = ResolvedRuleSource::resolve(&InstallSource::LocalPath(selected)).unwrap();
    let captured = source.capture().unwrap();
    for scope in [OwnershipScope::Project, OwnershipScope::User] {
        let root = root(tmp.path(), scope);
        let location = location(&root);
        let result = captured.compile_codex(binding(&root, &location)).unwrap();
        assert_eq!(
            result.desired.set.audience_excluded,
            vec![TargetSurface::CodexCli]
        );
        assert!(result.desired.set.artifacts.is_empty());
        assert!(result.desired.contracts.is_empty());
        assert_eq!(result.desired.set.installation.owner.source_name, "source");
        assert_eq!(result.readers.location, location);
        assert_eq!(result.readers.contract_revision, 1);
        assert_eq!(
            result.readers.compatibility,
            InstructionCompatibility::NotEvaluated
        );
        assert_eq!(
            readers(&result),
            if scope == OwnershipScope::Project {
                project_readers()
            } else {
                user_readers()
            }
        );
        assert!(!root.path.exists());
    }
}

#[test]
fn absent_source_name_uses_selected_folder_and_rejects_different_expected_name() {
    let tmp = tempfile::tempdir().unwrap();
    let selected = tmp.path().join("actual-folder");
    std::fs::create_dir(&selected).unwrap();
    std::fs::write(
        selected.join("RULE.md"),
        "---\nschema: 1\ndescription: demo\n---\nFallback body\n",
    )
    .unwrap();
    let source =
        ResolvedRuleSource::resolve(&InstallSource::LocalPath(selected.join("."))).unwrap();
    let captured = source.capture().unwrap();
    let root = root(tmp.path(), OwnershipScope::Project);
    let location = location(&root);
    let mut valid = binding(&root, &location);
    valid.expected_source_name = "actual-folder";
    let result = captured.compile_codex(valid).unwrap();
    assert_eq!(
        result.desired.set.installation.owner.source_name,
        "actual-folder"
    );
    assert_eq!(
        result.desired.set.installation.owner.installed_name,
        "alias"
    );
    let error = captured
        .compile_codex(binding(&root, &location))
        .unwrap_err();
    assert!(format!("{error:#}").contains("source name differs from installation binding"));
}

#[test]
fn rejects_invalid_binding_profile_targets_locations_and_project_reader_conflicts() {
    let tmp = tempfile::tempdir().unwrap();
    let selected = tmp.path().join("rule");
    write_rule(&selected, "", "Body\n");
    let source = ResolvedRuleSource::resolve(&InstallSource::LocalPath(selected.clone())).unwrap();
    let captured = source.capture().unwrap();
    let root = root(tmp.path(), OwnershipScope::Project);
    let location = location(&root);
    for (name, alias) in [("wrong", "alias"), ("source", "../escape")] {
        let mut invalid = binding(&root, &location);
        invalid.expected_source_name = name;
        invalid.installed_name = alias;
        assert!(captured.compile_codex(invalid).is_err());
    }
    for profile in [
        TargetProfile {
            family: TargetFamily::Claude,
            surface: TargetSurface::CodexCli,
            contract_revision: 1,
        },
        TargetProfile {
            contract_revision: 2,
            ..TargetSurface::CodexCli.profile()
        },
        TargetSurface::CodexIde.profile(),
    ] {
        let mut invalid = binding(&root, &location);
        invalid.profile = profile;
        assert!(captured.compile_codex(invalid).is_err());
    }
    for requested in [
        vec![],
        vec![TargetSurface::CodexIde],
        vec![TargetSurface::CodexCli, TargetSurface::ClaudeCli],
    ] {
        let mut invalid = binding(&root, &location);
        invalid.requested = &requested;
        assert!(captured.compile_codex(invalid).is_err());
    }
    for wrong in [
        InstructionLocation {
            path: ".codex/AGENTS.md".into(),
            ..location.clone()
        },
        InstructionLocation {
            root: InstallationRoot {
                scope: OwnershipScope::User,
                ..root.clone()
            },
            ..location.clone()
        },
        InstructionLocation {
            root: InstallationRoot {
                path: root.path.join("other"),
                ..root.clone()
            },
            ..location.clone()
        },
    ] {
        assert!(captured.compile_codex(binding(&root, &wrong)).is_err());
    }
    drop(captured);
    for fields in ["audience: [codex]\n", "audience: [codex, copilot]\n"] {
        write_rule(&selected, fields, "Equal text\n");
        let captured = source.capture().unwrap();
        let error = captured
            .compile_codex(binding(&root, &location))
            .unwrap_err();
        assert_eq!(
            error.downcast_ref::<CapabilityError>().unwrap().key.name,
            "audience"
        );
    }
    assert!(!root.path.exists());
}

#[test]
fn source_edit_after_capture_rejects_instead_of_refreshing_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let selected = tmp.path().join("rule");
    write_rule(&selected, "", "Original\n");
    let source = ResolvedRuleSource::resolve(&InstallSource::LocalPath(selected.clone())).unwrap();
    let captured = source.capture().unwrap();
    let root = root(tmp.path(), OwnershipScope::Project);
    let location = location(&root);
    captured.compile_codex(binding(&root, &location)).unwrap();
    let original = captured.snapshot().manifest().clone();
    write_rule(&selected, "", "Changed\n");
    assert!(captured.compile_codex(binding(&root, &location)).is_err());
    assert_eq!(captured.snapshot().manifest(), &original);
    assert!(!root.path.exists());
}

fn fixture_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    command
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = fixture_command("git")
        .current_dir(root)
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
fn moving_git_branch_refreshes_actual_revision_and_digest_with_stable_contribution_identity() {
    if let Some(repository) = std::env::var_os("UPSKILL_CODEX_SOURCE_REPOSITORY") {
        let repository = std::path::PathBuf::from(repository);
        let source = InstallSource::Git(GitRepo {
            url: "https://codex-source-fixture.invalid/repository".into(),
            git_ref: Some("main".into()),
            subfolder: Some("item".into()),
        });
        let temp = tempfile::tempdir().unwrap();
        let root = root(temp.path(), OwnershipScope::Project);
        let location = location(&root);
        let first = ResolvedRuleSource::resolve(&source).unwrap();
        let captured = first.capture().unwrap();
        let a = captured.compile_codex(binding(&root, &location)).unwrap();
        let first_revision = git(&repository, &["rev-parse", "HEAD"]);
        assert_git_evidence(&captured, &a, &source, &first_revision);
        write_rule(&repository.join("item"), "", "Second Git body\n");
        git(
            &repository,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--quiet",
                "-am",
                "second fixture",
            ],
        );
        let second_revision = git(&repository, &["rev-parse", "HEAD"]);
        assert_ne!(first_revision, second_revision);
        let second = ResolvedRuleSource::resolve(&source).unwrap();
        let next = second.capture().unwrap();
        let b = next.compile_codex(binding(&root, &location)).unwrap();
        assert_git_evidence(&next, &b, &source, &second_revision);
        assert_eq!(contribution(&a).id, CONTRIBUTION_ID);
        assert_eq!(contribution(&b).id, CONTRIBUTION_ID);
        assert_eq!(
            a.desired.set.installation.owner.installation_id,
            b.desired.set.installation.owner.installation_id
        );
        assert_eq!(contribution(&a).markdown, "First Git body\n");
        assert_eq!(contribution(&b).markdown, "Second Git body\n");
        assert_ne!(
            a.desired.set.installation.source_evidence,
            b.desired.set.installation.source_evidence
        );
        captured.verify_unchanged().unwrap();
        assert_eq!(
            captured.compile_codex(binding(&root, &location)).unwrap(),
            a
        );
        assert!(!root.path.exists());
        return;
    }
    let fixture = tempfile::tempdir().unwrap();
    let repository = fixture.path().join("repository");
    write_rule(&repository.join("item"), "", "First Git body\n");
    git(&repository, &["init", "--quiet", "--initial-branch=main"]);
    git(&repository, &["add", "."]);
    git(
        &repository,
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
    let config = fixture.path().join("gitconfig");
    std::fs::write(&config, format!("[url \"file://{}\"]\n    insteadOf = https://codex-source-fixture.invalid/repository\n", repository.display())).unwrap();
    let output = fixture_command(std::env::current_exe().unwrap())
        .args(["--exact", "moving_git_branch_refreshes_actual_revision_and_digest_with_stable_contribution_identity", "--nocapture"])
        .env("UPSKILL_CODEX_SOURCE_REPOSITORY", &repository)
        .env("HOME", fixture.path()).env("USERPROFILE", fixture.path())
        .env("GIT_CONFIG_NOSYSTEM", "1").env("GIT_CONFIG_GLOBAL", &config)
        .env("GIT_ALLOW_PROTOCOL", "file").output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_git_evidence(
    captured: &CapturedRuleSource<'_>,
    result: &CompiledCodexInstructions,
    source: &InstallSource,
    revision: &str,
) {
    let item = &result.desired.set.installation;
    assert_eq!(item.owner.source, source.to_string());
    assert_eq!(item.owner.source_revision.as_deref(), Some(revision));
    assert_eq!(
        item.source_evidence,
        SourceEvidence::Git {
            resolved_revision: revision.into(),
            manifest_sha256: captured.snapshot().manifest().content_sha256.clone(),
        }
    );
    assert_contract(result, "AGENTS.md");
    assert_eq!(readers(result), project_readers());
    let historical = captured
        .compile(RuleSourceBinding {
            installation_id: "test-installation",
            expected_source_name: "source",
            installed_name: "alias",
            root: &item.root,
            requested: &[TargetSurface::ClaudeCli],
            bindings: &[CompilerBinding {
                client: Client::Claude,
                consumers: vec![TargetSurface::ClaudeCli],
            }],
            profiles: &[TargetSurface::ClaudeCli.profile()],
        })
        .unwrap();
    assert_eq!(historical.set.installation.owner, item.owner);
    assert_eq!(
        historical.set.installation.source_evidence,
        item.source_evidence
    );
}
