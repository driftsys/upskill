//! Inactive command planning: no native executable or provider is invoked.
use std::ffi::OsString;
use upskill::compile::skill_package::SkillPackageBinding;
use upskill::lifecycle::native_receipt::{NativeSkillSelection, SkillTarget};
use upskill::native_delivery::adapter::{
    NativeActionKind, NativeDeliveryProfile, NativeExposureOrder,
};
use upskill::native_delivery::claude::{
    ClaudeCommandError, ClaudePluginAction, plan_plugin_command,
};
use upskill::ownership::{InstallationRoot, OwnershipScope};
use upskill::target::TargetSurface;

const NAME: &str = "upskill-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PAYLOAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
fn input() -> (NativeDeliveryProfile, SkillTarget, NativeSkillSelection) {
    let profile = NativeDeliveryProfile {
        id: "candidate-claude".into(),
        revision: 1,
        surface: TargetSurface::ClaudeCli,
        scope: InstallationRoot {
            scope: OwnershipScope::Project,
            path: "/fixture/project with spaces".into(),
        },
        client_version: "2.1.308".into(),
        platform: "unqualified-fixture".into(),
        exposure: NativeExposureOrder::AfterNative,
        action: NativeActionKind::Command,
    };
    let target = SkillTarget {
        installation_id: "owned-1".into(),
        surface: TargetSurface::ClaudeCli,
    };
    let selection = NativeSkillSelection {
        native_name: NAME.into(),
        payload_sha256: PAYLOAD.into(),
        package_sha256: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
        revision_root: format!(".upskill/exports/claude/owned-1/revisions/{PAYLOAD}"),
        binding: SkillPackageBinding::LocalMarketplace {
            directory: "/fixture/project with spaces/.upskill/exports/claude/owned-1".into(),
        },
    };
    (profile, target, selection)
}
#[test]
fn every_effect_has_an_exact_selector_and_explicit_scope_without_trust_flags() {
    let (mut profile, target, selection) = input();
    let catalog = "/fixture/project with spaces/.upskill/exports/claude/owned-1";
    let selector = "upskill-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb@upskill-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    for (kind, scope) in [
        (OwnershipScope::Project, "project"),
        (OwnershipScope::User, "user"),
    ] {
        profile.scope.scope = kind;
        for (action, expected) in [
            (
                ClaudePluginAction::ValidateCatalog,
                vec!["plugin", "validate", catalog],
            ),
            (
                ClaudePluginAction::RegisterMarketplace,
                vec![
                    "plugin",
                    "marketplace",
                    "add",
                    catalog,
                    "--scope",
                    scope,
                    "--json",
                ],
            ),
            (
                ClaudePluginAction::Install,
                vec!["plugin", "install", selector, "--scope", scope, "--json"],
            ),
            (
                ClaudePluginAction::Update,
                vec!["plugin", "update", selector, "--scope", scope, "--json"],
            ),
            (
                ClaudePluginAction::Uninstall,
                vec!["plugin", "uninstall", selector, "--scope", scope, "--json"],
            ),
            (
                ClaudePluginAction::UnregisterMarketplace,
                vec![
                    "plugin",
                    "marketplace",
                    "remove",
                    NAME,
                    "--scope",
                    scope,
                    "--json",
                ],
            ),
        ] {
            let command = plan_plugin_command(&profile, &target, &selection, action).unwrap();
            assert_eq!(
                command.arguments(),
                expected.iter().map(OsString::from).collect::<Vec<_>>()
            );
            assert!(!format!("{command:?}").contains(NAME));
            assert!(!format!("{command:?}").contains("fixture"));
        }
    }
}
#[test]
fn unsupported_profile_cannot_produce_any_command() {
    let (profile, target, selection) = input();
    for index in 0..4 {
        let mut profile = profile.clone();
        let mut target = target.clone();
        match index {
            0 => profile.client_version = "2.1.309-private-canary".into(),
            1 => profile.surface = TargetSurface::CopilotCli,
            2 => target.surface = TargetSurface::CodexCli,
            3 => profile.action = NativeActionKind::FileOnly,
            _ => unreachable!(),
        }
        let error = plan_plugin_command(&profile, &target, &selection, ClaudePluginAction::Install)
            .unwrap_err();
        assert_eq!(error, ClaudeCommandError::UnsupportedProfile);
        assert!(!format!("{error:?}: {error}").contains("private-canary"));
    }
}
#[test]
fn mismatched_or_unsafe_managed_selection_cannot_escape_its_owned_catalog() {
    for index in 0..12 {
        let (mut profile, mut target, mut selection) = input();
        match index {
            0 => target.installation_id = "../other".into(),
            1 => target.installation_id = "other-owner".into(),
            2 => profile.scope.path = "relative".into(),
            3 => profile.scope.path = "/fixture/../project with spaces".into(),
            4 => selection.native_name = "--yes".into(),
            5 => selection.native_name = "upskill-abc".into(),
            6 => selection.payload_sha256 = "a".repeat(63),
            7 => selection.package_sha256 = "C".repeat(64),
            8 => selection.revision_root.push_str("/../other"),
            9 => {
                selection.binding = SkillPackageBinding::ExplicitSkillSource {
                    directory: "/other".into(),
                }
            }
            10 => {
                selection.binding = SkillPackageBinding::LocalMarketplace {
                    directory: "/other".into(),
                }
            }
            11 => {
                selection.binding = SkillPackageBinding::LocalMarketplace {
                    directory:
                        "/fixture/project with spaces/.upskill/exports/claude/owned-1/../owned-1"
                            .into(),
                }
            }
            _ => unreachable!(),
        }
        assert_eq!(
            plan_plugin_command(
                &profile,
                &target,
                &selection,
                ClaudePluginAction::UnregisterMarketplace
            )
            .unwrap_err(),
            ClaudeCommandError::InvalidSelection,
            "case {index}"
        );
    }
}

#[test]
fn malformed_payload_is_rejected_even_when_the_revision_path_matches_it() {
    for payload in [String::new(), "a".repeat(63), "A".repeat(64), "a\0".into()] {
        let (profile, target, mut selection) = input();
        selection.revision_root = format!(".upskill/exports/claude/owned-1/revisions/{payload}");
        selection.payload_sha256 = payload;
        assert_eq!(
            plan_plugin_command(&profile, &target, &selection, ClaudePluginAction::Install)
                .unwrap_err(),
            ClaudeCommandError::InvalidSelection
        );
    }
}
#[test]
fn coherent_unsafe_scope_and_catalog_paths_are_rejected() {
    for path in [
        "relative",
        "/fixture/../project with spaces",
        "/fixture/private\0scope",
    ] {
        let (mut profile, target, mut selection) = input();
        profile.scope.path = path.into();
        selection.binding = SkillPackageBinding::LocalMarketplace {
            directory: profile.scope.path.join(".upskill/exports/claude/owned-1"),
        };
        assert_eq!(
            plan_plugin_command(&profile, &target, &selection, ClaudePluginAction::Install)
                .unwrap_err(),
            ClaudeCommandError::InvalidSelection
        );
    }
}
#[test]
fn invalid_installation_ids_are_rejected_even_with_matching_catalog_and_revision() {
    for id in ["", "bad.name", "bad name", "badé"] {
        let (profile, mut target, mut selection) = input();
        target.installation_id = id.into();
        let item = format!(".upskill/exports/claude/{id}");
        selection.binding = SkillPackageBinding::LocalMarketplace {
            directory: profile.scope.path.join(&item),
        };
        selection.revision_root = format!("{item}/revisions/{PAYLOAD}");
        assert_eq!(
            plan_plugin_command(&profile, &target, &selection, ClaudePluginAction::Install)
                .unwrap_err(),
            ClaudeCommandError::InvalidSelection
        );
    }
}
