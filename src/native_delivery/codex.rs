//! Candidate Codex 0.166.0 argv. No execution or project admission.
use super::adapter::NativeDeliveryProfile;
use super::user_plugin_command::validate;
pub use super::user_plugin_command::{
    UserPluginAction as CodexPluginAction, UserPluginCommandError as CodexCommandError,
};
use crate::lifecycle::native_receipt::{NativeSkillSelection, SkillTarget};
use crate::target::TargetSurface;
use std::{ffi::OsString, fmt};

/// Fixed argv only. Scoped configuration, runtime identity, ownership, durable
/// intent, quiescence, retention and independent observation remain separate.
pub struct CodexPluginCommand {
    arguments: Vec<OsString>,
}
impl CodexPluginCommand {
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}
impl fmt::Debug for CodexPluginCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CodexPluginCommand")
            .field("argument_count", &self.arguments.len())
            .finish_non_exhaustive()
    }
}
/// Plan only captured user-context operations. A local-marketplace update has
/// no captured refresh command: Git marketplace upgrade and remove/re-add are
/// not substitutes. Removal invalidates native cache; planning grants no
/// retention witness. No feature, config override, trust, or scope flags emit.
pub fn plan_plugin_command(
    profile: &NativeDeliveryProfile,
    target: &SkillTarget,
    selection: &NativeSkillSelection,
    action: CodexPluginAction,
) -> Result<CodexPluginCommand, CodexCommandError> {
    let directory = validate(
        profile,
        target,
        selection,
        TargetSurface::CodexCli,
        "0.166.0",
    )?;
    let selector = format!("{}@{}", selection.native_name, selection.native_name);
    let mut arguments: Vec<OsString> = match action {
        CodexPluginAction::RegisterMarketplace => vec![
            "plugin".into(),
            "marketplace".into(),
            "add".into(),
            directory.into(),
        ],
        CodexPluginAction::UnregisterMarketplace => vec![
            "plugin".into(),
            "marketplace".into(),
            "remove".into(),
            selection.native_name.clone().into(),
        ],
        CodexPluginAction::Install => vec!["plugin".into(), "add".into(), selector.into()],
        CodexPluginAction::Uninstall => vec!["plugin".into(), "remove".into(), selector.into()],
        CodexPluginAction::Update => return Err(CodexCommandError::UnsupportedAction),
    };
    arguments.push("--json".into());
    Ok(CodexPluginCommand { arguments })
}
