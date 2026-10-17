#![cfg(any(target_os = "linux", target_os = "macos"))]

use upskill::lifecycle::sources::ResolvedRuleSource;
use upskill::source::InstallSource;

#[test]
fn capture_rejects_edits_and_missing_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("rule");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("RULE.md"), b"original").unwrap();
    let resolved = ResolvedRuleSource::resolve(&InstallSource::LocalPath(root.clone())).unwrap();
    let captured = resolved.capture().unwrap();
    assert_eq!(captured.snapshot().manifest().files[0].bytes, b"original");
    std::fs::write(root.join("RULE.md"), b"changed").unwrap();
    assert!(captured.verify_unchanged().is_err());
    assert!(ResolvedRuleSource::resolve(&InstallSource::LocalPath(root.join("missing"))).is_err());
}

use upskill::generate::Client;
use upskill::lifecycle::desired::CompilerBinding;
use upskill::lifecycle::sources::RuleSourceBinding;
use upskill::ownership::{InstallationRoot, OwnershipScope};
use upskill::target::{TargetProfile, TargetSurface};
const RULE: &[u8] = b"---\nschema: 1\nname: source\ndescription: demo\n---\n\n# Captured bytes\n";

fn compile(
    source: &ResolvedRuleSource,
    name: &str,
    profiles: &[TargetProfile],
) -> anyhow::Result<upskill::lifecycle::desired::ContractedDesiredSet> {
    source.capture()?.compile(RuleSourceBinding {
        installation_id: "test-installation",
        expected_source_name: name,
        installed_name: "alias",
        root: &InstallationRoot {
            scope: OwnershipScope::Project,
            path: std::env::temp_dir().join("source-output-never-created"),
        },
        requested: &[TargetSurface::ClaudeCli],
        bindings: &[CompilerBinding {
            client: Client::Claude,
            consumers: vec![TargetSurface::ClaudeCli],
        }],
        profiles,
    })
}
fn profiles() -> Vec<TargetProfile> {
    vec![
        TargetProfile::new(
            TargetSurface::ClaudeCli.family(),
            TargetSurface::ClaudeCli,
            1,
        )
        .unwrap(),
    ]
}
#[test]
fn captured_compile_binds_alias_bytes_modes_and_contracts_without_writes() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("RULE.md"), RULE).unwrap();
    let resolved =
        ResolvedRuleSource::resolve(&InstallSource::LocalPath(tmp.path().into())).unwrap();
    let result = compile(&resolved, "source", &profiles()).unwrap();
    assert_eq!(result.set.installation.owner.installed_name, "alias");
    assert_eq!(result.contracts.len(), 1);
    assert_eq!(result.set.artifacts.len(), 1);
    let upskill::operations::plan::DesiredArtifact::File(file) = &result.set.artifacts[0] else {
        panic!()
    };
    assert!(String::from_utf8_lossy(&file.value.bytes).contains("Captured bytes"));
    assert!(file.location.path.contains("alias"));
    assert_eq!(
        result.contracts[0].source_manifest_sha256,
        resolved
            .capture()
            .unwrap()
            .snapshot()
            .manifest()
            .content_sha256
    );
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 1);
    assert!(compile(&resolved, "different", &profiles()).is_err());
    assert!(compile(&resolved, "source", &[]).is_err());
    std::fs::write(tmp.path().join("RULE.md"), b"---\nmalformed").unwrap();
    assert!(compile(&resolved, "source", &profiles()).is_err());
}
#[cfg(unix)]
#[test]
fn selected_final_symlink_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let link = tmp.path().join("link");
    std::os::unix::fs::symlink(tmp.path(), &link).unwrap();
    assert!(ResolvedRuleSource::resolve(&InstallSource::LocalPath(link)).is_err());
}

fn git(root: &std::path::Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
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
fn git_branch_tag_and_full_sha_capture_actual_revision_and_retain_checkout() {
    if isolated_git_test("git_branch_tag_and_full_sha_capture_actual_revision_and_retain_checkout")
    {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    git(tmp.path(), &["init", "--quiet", "--initial-branch=main"]);
    std::fs::write(tmp.path().join("RULE.md"), RULE).unwrap();
    std::fs::create_dir(tmp.path().join("item")).unwrap();
    std::fs::write(tmp.path().join("item/RULE.md"), RULE).unwrap();
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
    git(tmp.path(), &["tag", "v1"]);
    let sha = git(tmp.path(), &["rev-parse", "HEAD"]);
    for reference in ["main", "v1", sha.as_str()] {
        let source = InstallSource::Git(upskill::source::GitRepo {
            url: format!("file://{}", tmp.path().display()),
            git_ref: Some(reference.into()),
            subfolder: Some("item".into()),
        });
        let resolved = ResolvedRuleSource::resolve(&source).unwrap();
        let captured = resolved.capture().unwrap();
        let result = compile(&resolved, "source", &profiles()).unwrap();
        assert_eq!(
            result.set.installation.owner.source_revision.as_deref(),
            Some(sha.as_str())
        );
        captured.verify_unchanged().unwrap();
        assert_eq!(captured.snapshot().manifest().files.len(), 1);
    }
    let escape = InstallSource::Git(upskill::source::GitRepo {
        url: format!("file://{}", tmp.path().display()),
        git_ref: None,
        subfolder: Some("../escape".into()),
    });
    assert!(ResolvedRuleSource::resolve(&escape).is_err());
}

#[test]
fn git_repository_root_capture_excludes_private_metadata() {
    if isolated_git_test("git_repository_root_capture_excludes_private_metadata") {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    git(tmp.path(), &["init", "--quiet"]);
    std::fs::write(tmp.path().join("RULE.md"), RULE).unwrap();
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
    let source = InstallSource::Git(upskill::source::GitRepo {
        url: format!("file://{}", tmp.path().display()),
        git_ref: None,
        subfolder: None,
    });
    let resolved = ResolvedRuleSource::resolve(&source).unwrap();
    assert_eq!(
        resolved
            .capture()
            .unwrap()
            .snapshot()
            .manifest()
            .files
            .len(),
        1
    );
}

// Run resolver subprocesses with private Git configuration, without process-wide env edits.
fn isolated_git_test(name: &str) -> bool {
    if std::env::var_os("UPSKILL_SOURCE_TEST_CHILD").is_some() {
        return false;
    }
    let home = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("UPSKILL_SOURCE_TEST_CHILD", "1")
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

#[test]
fn local_dot_selection_preserves_real_folder_and_rejects_final_symlink() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("RULE.md"), RULE).unwrap();
    let selected = tmp.path().join(".");
    let resolved = ResolvedRuleSource::resolve(&InstallSource::LocalPath(selected)).unwrap();
    assert!(compile(&resolved, "source", &profiles()).is_ok());
}

#[test]
fn resource_bytes_and_executable_mode_survive_compilation() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("RULE.md"), RULE).unwrap();
    std::fs::create_dir(tmp.path().join("scripts")).unwrap();
    let script = tmp.path().join("scripts/check.sh");
    std::fs::write(&script, b"#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let source = ResolvedRuleSource::resolve(&InstallSource::LocalPath(tmp.path().into())).unwrap();
    let desired = compile(&source, "source", &profiles()).unwrap();
    let file = desired
        .set
        .artifacts
        .iter()
        .find_map(|artifact| {
            let upskill::operations::plan::DesiredArtifact::File(file) = artifact else {
                return None;
            };
            file.location
                .path
                .ends_with("scripts/check.sh")
                .then_some(file)
        })
        .expect("resource output");
    assert_eq!(file.value.bytes, b"#!/bin/sh\nexit 0\n");
    assert_eq!(file.value.mode, Some(0o755));
    assert!(desired.contracts.iter().any(|contract| matches!(
        contract.content,
        upskill::lifecycle::contracts::ContractContent::File {
            mode: Some(0o755),
            ..
        }
    )));
    assert!(!desired.set.installation.root.path.exists());
}

#[test]
fn codex_binding_is_rejected_before_any_destination_effect() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("RULE.md"), RULE).unwrap();
    let source = ResolvedRuleSource::resolve(&InstallSource::LocalPath(tmp.path().into())).unwrap();
    let destination = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().join("never-created"),
    };
    let result = source.capture().unwrap().compile(RuleSourceBinding {
        installation_id: "codex-test",
        expected_source_name: "source",
        installed_name: "alias",
        root: &destination,
        requested: &[TargetSurface::CodexCli],
        bindings: &[CompilerBinding {
            client: Client::Claude,
            consumers: vec![TargetSurface::CodexCli],
        }],
        profiles: &[TargetProfile::new(
            TargetSurface::CodexCli.family(),
            TargetSurface::CodexCli,
            1,
        )
        .unwrap()],
    });
    assert!(format!("{:#}", result.unwrap_err()).contains("unsupported compiler surface"));
    assert!(!destination.path.exists());
}

#[test]
fn inherited_git_index_cannot_escape_private_checkout() {
    if std::env::var_os("UPSKILL_INDEX_FIXTURE_REPO").is_some() {
        let source = InstallSource::Git(upskill::source::GitRepo {
            url: "https://source-index-fixture.invalid/repository".into(),
            git_ref: Some("main".into()),
            subfolder: None,
        });
        let resolved = ResolvedRuleSource::resolve(&source).unwrap();
        let desired = compile(&resolved, "source", &profiles()).unwrap();
        assert_eq!(
            desired.set.installation.owner.source_revision.as_deref(),
            std::env::var("UPSKILL_INDEX_FIXTURE_REVISION")
                .ok()
                .as_deref()
        );
        resolved.capture().unwrap().verify_unchanged().unwrap();
        return;
    }
    let fixture = tempfile::tempdir().unwrap();
    let repository = fixture.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    git(&repository, &["init", "--quiet", "--initial-branch=main"]);
    std::fs::write(repository.join("RULE.md"), RULE).unwrap();
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
    let revision = git(&repository, &["rev-parse", "HEAD"]);
    let configuration = fixture.path().join("gitconfig");
    std::fs::write(&configuration, format!("[url \"file://{}\"]\n    insteadOf = https://source-index-fixture.invalid/repository\n", repository.display())).unwrap();
    let external_index = fixture.path().join("external-index");
    for existing in [false, true] {
        if existing {
            std::fs::copy(repository.join(".git/index"), &external_index).unwrap();
        }
        let before = std::fs::read(&external_index).ok();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "inherited_git_index_cannot_escape_private_checkout",
                "--nocapture",
            ])
            .env("UPSKILL_INDEX_FIXTURE_REPO", &repository)
            .env("UPSKILL_INDEX_FIXTURE_REVISION", &revision)
            .env("HOME", fixture.path())
            .env("USERPROFILE", fixture.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &configuration)
            .env("GIT_ALLOW_PROTOCOL", "file")
            .env("GIT_INDEX_FILE", &external_index)
            .env_remove("GIT_CONFIG_COUNT")
            .env_remove("GIT_CONFIG_PARAMETERS")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            std::fs::read(&external_index).ok(),
            before,
            "source resolver touched inherited external index"
        );
    }
}
