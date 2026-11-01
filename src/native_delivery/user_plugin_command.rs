//! Shared consistency checks for candidate user-only marketplace commands.
use super::adapter::{NativeActionKind, NativeDeliveryProfile};
use crate::compile::skill_package::SkillPackageBinding;
use crate::lifecycle::native_receipt::{NativeSkillSelection, SkillTarget};
use crate::ownership::OwnershipScope;
use crate::target::TargetSurface;
use std::path::{Component, Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserPluginAction {
    RegisterMarketplace,
    Install,
    Update,
    Uninstall,
    UnregisterMarketplace,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UserPluginCommandError {
    #[error("native command profile is unsupported; review the exact client version and route")]
    UnsupportedProfile,
    #[error(
        "the captured native command contract has no project-only operation; do not install in user scope as a fallback"
    )]
    UnsupportedProjectScope,
    #[error("native command selection does not match its managed scope and package")]
    InvalidSelection,
    #[error(
        "the requested operation has no captured local-marketplace command; do not substitute a broader operation"
    )]
    UnsupportedAction,
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
/// This validates declared records, not ownership, HOME, effective settings,
/// catalog contents, native name acceptance, trust, or qualification.
pub(super) fn validate<'a>(
    profile: &NativeDeliveryProfile,
    target: &SkillTarget,
    selection: &'a NativeSkillSelection,
    surface: TargetSurface,
    version: &str,
) -> Result<&'a Path, UserPluginCommandError> {
    if profile.surface != surface
        || target.surface != surface
        || profile.client_version != version
        || profile.action != NativeActionKind::Command
    {
        return Err(UserPluginCommandError::UnsupportedProfile);
    }
    if profile.scope.scope != OwnershipScope::User {
        return Err(UserPluginCommandError::UnsupportedProjectScope);
    }
    let namespace = match surface {
        TargetSurface::CopilotCli => "copilot",
        TargetSurface::CodexCli => "codex",
        _ => return Err(UserPluginCommandError::UnsupportedProfile),
    };
    let SkillPackageBinding::LocalMarketplace { directory } = &selection.binding else {
        return Err(UserPluginCommandError::InvalidSelection);
    };
    let item = format!(".upskill/exports/{namespace}/{}", target.installation_id);
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
        return Err(UserPluginCommandError::InvalidSelection);
    }
    Ok(directory)
}
