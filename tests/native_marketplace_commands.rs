//! Candidate argv only; no native client is invoked or qualified.
#![cfg(unix)]
use copilot::{CopilotCommandError as Error, CopilotPluginAction as Action};
use std::ffi::OsString;
use upskill::compile::skill_package::SkillPackageBinding;
use upskill::lifecycle::native_receipt::{NativeSkillSelection, SkillTarget};
use upskill::native_delivery::adapter::{
    NativeActionKind, NativeDeliveryProfile, NativeExposureOrder,
};
use upskill::native_delivery::{codex, copilot};
use upskill::ownership::{InstallationRoot, OwnershipScope};
use upskill::target::TargetSurface;
const NAME: &str = "upskill-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SELECTOR: &str = "upskill-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb@upskill-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PAYLOAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ACTIONS: [Action; 5] = [
    Action::RegisterMarketplace,
    Action::Install,
    Action::Update,
    Action::Uninstall,
    Action::UnregisterMarketplace,
];
fn input(surface: TargetSurface) -> (NativeDeliveryProfile, SkillTarget, NativeSkillSelection) {
    let (namespace, version) = match surface {
        TargetSurface::CopilotCli => ("copilot", "1.0.105"),
        TargetSurface::CodexCli => ("codex", "0.166.0"),
        _ => unreachable!(),
    };
    let profile = NativeDeliveryProfile {
        id: "candidate-only".into(),
        revision: 1,
        surface,
        scope: InstallationRoot {
            scope: OwnershipScope::User,
            path: "/fixture/user home".into(),
        },
        client_version: version.into(),
        platform: "unqualified-fixture".into(),
        exposure: NativeExposureOrder::AfterNative,
        action: NativeActionKind::Command,
    };
    let target = SkillTarget {
        installation_id: "owned-1".into(),
        surface,
    };
    let item = format!(".upskill/exports/{namespace}/owned-1");
    let selection = NativeSkillSelection {
        native_name: NAME.into(),
        payload_sha256: PAYLOAD.into(),
        package_sha256: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
        revision_root: format!("{item}/revisions/{PAYLOAD}"),
        binding: SkillPackageBinding::LocalMarketplace {
            directory: profile.scope.path.join(item),
        },
    };
    (profile, target, selection)
}
fn plan(
    surface: TargetSurface,
    profile: &NativeDeliveryProfile,
    target: &SkillTarget,
    selection: &NativeSkillSelection,
    action: Action,
) -> Result<Vec<OsString>, Error> {
    match surface {
        TargetSurface::CopilotCli => {
            copilot::plan_plugin_command(profile, target, selection, action)
                .map(|command| command.arguments().to_vec())
        }
        TargetSurface::CodexCli => codex::plan_plugin_command(profile, target, selection, action)
            .map(|command| command.arguments().to_vec()),
        _ => unreachable!(),
    }
}
#[test]
fn copilot_uses_exact_local_catalog_and_selectors_without_scope_or_trust_invention() {
    let (profile, target, selection) = input(TargetSurface::CopilotCli);
    for (action, expected) in [
        (
            Action::RegisterMarketplace,
            vec![
                "--no-auto-update",
                "plugin",
                "marketplace",
                "add",
                "/fixture/user home/.upskill/exports/copilot/owned-1",
            ],
        ),
        (
            Action::Install,
            vec!["--no-auto-update", "plugin", "install", SELECTOR],
        ),
        (
            Action::Update,
            vec!["--no-auto-update", "plugin", "update", SELECTOR],
        ),
        (
            Action::Uninstall,
            vec!["--no-auto-update", "plugin", "uninstall", SELECTOR],
        ),
        (
            Action::UnregisterMarketplace,
            vec!["--no-auto-update", "plugin", "marketplace", "remove", NAME],
        ),
    ] {
        let command = copilot::plan_plugin_command(&profile, &target, &selection, action).unwrap();
        assert_eq!(
            command.arguments(),
            expected.iter().map(OsString::from).collect::<Vec<_>>()
        );
        assert!(!format!("{command:?}").contains(NAME));
        assert!(!format!("{command:?}").contains("fixture"));
    }
}
#[test]
fn codex_uses_json_and_never_substitutes_git_upgrade_or_remove_add_for_local_refresh() {
    let (profile, target, selection) = input(TargetSurface::CodexCli);
    for (action, expected) in [
        (
            Action::RegisterMarketplace,
            vec![
                "plugin",
                "marketplace",
                "add",
                "/fixture/user home/.upskill/exports/codex/owned-1",
                "--json",
            ],
        ),
        (Action::Install, vec!["plugin", "add", SELECTOR, "--json"]),
        (
            Action::Uninstall,
            vec!["plugin", "remove", SELECTOR, "--json"],
        ),
        (
            Action::UnregisterMarketplace,
            vec!["plugin", "marketplace", "remove", NAME, "--json"],
        ),
    ] {
        let command = codex::plan_plugin_command(&profile, &target, &selection, action).unwrap();
        assert_eq!(
            command.arguments(),
            expected.iter().map(OsString::from).collect::<Vec<_>>()
        );
        assert!(!format!("{command:?}").contains(NAME));
        assert!(!format!("{command:?}").contains("fixture"));
    }
    assert_eq!(
        codex::plan_plugin_command(&profile, &target, &selection, Action::Update).unwrap_err(),
        Error::UnsupportedAction
    );
}
#[test]
fn every_project_operation_is_rejected_before_a_user_scope_command_can_be_produced() {
    for surface in [TargetSurface::CopilotCli, TargetSurface::CodexCli] {
        let (mut profile, target, selection) = input(surface);
        profile.scope.scope = OwnershipScope::Project;
        for action in ACTIONS {
            assert_eq!(
                plan(surface, &profile, &target, &selection, action).unwrap_err(),
                Error::UnsupportedProjectScope
            );
        }
    }
}
#[test]
fn wrong_version_surface_and_action_kind_cannot_select_a_command_family() {
    for surface in [TargetSurface::CopilotCli, TargetSurface::CodexCli] {
        for index in 0..4 {
            let (mut profile, mut target, selection) = input(surface);
            match index {
                0 => profile.client_version = "private-canary-pin".into(),
                1 => profile.surface = TargetSurface::OpenCodeCli,
                2 => {
                    target.surface = if surface == TargetSurface::CopilotCli {
                        TargetSurface::CodexCli
                    } else {
                        TargetSurface::CopilotCli
                    }
                }
                3 => profile.action = NativeActionKind::FileOnly,
                _ => unreachable!(),
            }
            let error = plan(surface, &profile, &target, &selection, Action::Install).unwrap_err();
            assert_eq!(error, Error::UnsupportedProfile);
            assert!(!format!("{error:?}: {error}").contains("private-canary"));
        }
    }
}
#[test]
fn invalid_selection_controls_keep_other_catalog_and_revision_fields_consistent() {
    for surface in [TargetSurface::CopilotCli, TargetSurface::CodexCli] {
        for index in 0..12 {
            let (mut profile, mut target, mut selection) = input(surface);
            let namespace = if surface == TargetSurface::CopilotCli {
                "copilot"
            } else {
                "codex"
            };
            match index {
                0 => {
                    target.installation_id = String::new();
                    let item = format!(".upskill/exports/{namespace}/");
                    selection.binding = SkillPackageBinding::LocalMarketplace {
                        directory: profile.scope.path.join(&item),
                    };
                    selection.revision_root = format!("{item}/revisions/{PAYLOAD}");
                }
                1 => {
                    target.installation_id = "bad.name".into();
                    let item = format!(".upskill/exports/{namespace}/bad.name");
                    selection.binding = SkillPackageBinding::LocalMarketplace {
                        directory: profile.scope.path.join(&item),
                    };
                    selection.revision_root = format!("{item}/revisions/{PAYLOAD}");
                }
                2 => selection.native_name = format!("upskill-{}", "B".repeat(64)),
                3 => {
                    selection.payload_sha256 = "a".repeat(63);
                    selection.revision_root = format!(
                        ".upskill/exports/{namespace}/owned-1/revisions/{}",
                        selection.payload_sha256
                    );
                }
                4 => selection.package_sha256 = "C".repeat(64),
                5 => selection.revision_root = "foreign-revision".into(),
                6 => {
                    selection.binding = SkillPackageBinding::LocalMarketplace {
                        directory: "/foreign".into(),
                    }
                }
                7..=9 => {
                    profile.scope.path = match index {
                        7 => "relative",
                        8 => "/fixture/../home",
                        9 => "/fixture/private\0home",
                        _ => unreachable!(),
                    }
                    .into();
                    selection.binding = SkillPackageBinding::LocalMarketplace {
                        directory: profile
                            .scope
                            .path
                            .join(format!(".upskill/exports/{namespace}/owned-1")),
                    };
                }
                10 => {
                    selection.binding = SkillPackageBinding::ExplicitSkillSource {
                        directory: profile
                            .scope
                            .path
                            .join(format!(".upskill/exports/{namespace}/owned-1")),
                    }
                }
                11 => {
                    let other = if surface == TargetSurface::CopilotCli {
                        "codex"
                    } else {
                        "copilot"
                    };
                    let item = format!(".upskill/exports/{other}/owned-1");
                    selection.binding = SkillPackageBinding::LocalMarketplace {
                        directory: profile.scope.path.join(&item),
                    };
                    selection.revision_root = format!("{item}/revisions/{PAYLOAD}");
                }
                _ => unreachable!(),
            }
            assert_eq!(
                plan(surface, &profile, &target, &selection, Action::Install).unwrap_err(),
                Error::InvalidSelection,
                "{surface:?} case {index}"
            );
        }
    }
}
