//! Pure candidate argv planning for the retained Claude 2.1.308 interface.
use super::super::adapter::{NativeActionKind, NativeDeliveryProfile};
use crate::compile::skill_package::SkillPackageBinding;
use crate::lifecycle::native_receipt::{NativeSkillSelection, SkillTarget};
use crate::ownership::OwnershipScope;
use crate::target::TargetSurface;
use std::ffi::OsString;
use std::fmt;
use std::path::{Component, Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudePluginAction {
    ValidateCatalog,
    RegisterMarketplace,
    Install,
    Update,
    Uninstall,
    UnregisterMarketplace,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ClaudeCommandError {
    #[error("Claude command profile is unsupported; review the exact native version and route")]
    UnsupportedProfile,
    #[error("Claude command selection is inconsistent with its managed scope and package")]
    InvalidSelection,
}
/// Fixed argument vector only. This has no execution, ownership, admission or
/// successful-effect authority and deliberately has no serde representation.
pub struct ClaudePluginCommand {
    arguments: Vec<OsString>,
}
impl ClaudePluginCommand {
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}
impl fmt::Debug for ClaudePluginCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudePluginCommand")
            .field("argument_count", &self.arguments.len())
            .finish_non_exhaustive()
    }
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
fn absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.to_str().is_some_and(|text| !text.contains('\0'))
        && !path.components().any(|part| part == Component::ParentDir)
}
/// Validate declared package/target consistency and produce exact scoped argv.
/// Roots, file contents, catalog sources, ownership, executable identity, trust,
/// competing readers and native qualification must be established separately.
/// Each mutation still needs its durable invocation checkpoint and independent
/// observation. In particular, update syntax does not establish cache retention
/// or restart/session behavior. No trust, prune or default-scope flags are added.
pub fn plan_plugin_command(
    profile: &NativeDeliveryProfile,
    target: &SkillTarget,
    selection: &NativeSkillSelection,
    action: ClaudePluginAction,
) -> Result<ClaudePluginCommand, ClaudeCommandError> {
    if profile.surface != TargetSurface::ClaudeCli
        || target.surface != TargetSurface::ClaudeCli
        || profile.client_version != "2.1.308"
        || profile.action != NativeActionKind::Command
    {
        return Err(ClaudeCommandError::UnsupportedProfile);
    }
    let item = format!(".upskill/exports/claude/{}", target.installation_id);
    let SkillPackageBinding::LocalMarketplace { directory } = &selection.binding else {
        return Err(ClaudeCommandError::InvalidSelection);
    };
    if target.installation_id.is_empty()
        || !target
            .installation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        || !absolute(&profile.scope.path)
        || !absolute(directory)
        || directory != &profile.scope.path.join(&item)
        || !selection
            .native_name
            .strip_prefix("upskill-")
            .is_some_and(digest)
        || !digest(&selection.payload_sha256)
        || !digest(&selection.package_sha256)
        || selection.revision_root != format!("{item}/revisions/{}", selection.payload_sha256)
    {
        return Err(ClaudeCommandError::InvalidSelection);
    }
    let scope = match profile.scope.scope {
        OwnershipScope::Project => "project",
        OwnershipScope::User => "user",
    };
    let selector = format!("{}@{}", selection.native_name, selection.native_name);
    let mut arguments: Vec<OsString> = match action {
        ClaudePluginAction::ValidateCatalog => {
            vec!["plugin".into(), "validate".into(), directory.into()]
        }
        ClaudePluginAction::RegisterMarketplace => vec![
            "plugin".into(),
            "marketplace".into(),
            "add".into(),
            directory.into(),
        ],
        ClaudePluginAction::UnregisterMarketplace => vec![
            "plugin".into(),
            "marketplace".into(),
            "remove".into(),
            selection.native_name.clone().into(),
        ],
        ClaudePluginAction::Install
        | ClaudePluginAction::Update
        | ClaudePluginAction::Uninstall => {
            let verb = match action {
                ClaudePluginAction::Install => "install",
                ClaudePluginAction::Update => "update",
                ClaudePluginAction::Uninstall => "uninstall",
                _ => unreachable!(),
            };
            vec!["plugin".into(), verb.into(), selector.into()]
        }
    };
    if action != ClaudePluginAction::ValidateCatalog {
        arguments.extend(["--scope".into(), scope.into(), "--json".into()]);
    }
    Ok(ClaudePluginCommand { arguments })
}
