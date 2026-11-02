//! Candidate Copilot 1.0.105 argv. No execution or native name acceptance proof.
use super::adapter::NativeDeliveryProfile;
use super::user_plugin_command::validate;
pub use super::user_plugin_command::{
    UserPluginAction as CopilotPluginAction, UserPluginCommandError as CopilotCommandError,
};
use crate::lifecycle::native_receipt::{NativeSkillSelection, SkillTarget};
use crate::target::TargetSurface;
use std::{ffi::OsString, fmt};

/// Fixed argv only. The executor must separately bind HOME/configuration,
/// runtime identity, ownership, durable intent, quiescence and observation.
pub struct CopilotPluginCommand {
    arguments: Vec<OsString>,
}
impl CopilotPluginCommand {
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}
impl fmt::Debug for CopilotPluginCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CopilotPluginCommand")
            .field("argument_count", &self.arguments.len())
            .finish_non_exhaustive()
    }
}
/// Plan only the captured user-context command family. Full digest names are
/// preserved; their native acceptance remains unqualified. Never infer project
/// support or use update-all, force removal, automatic trust, or JSON flags.
pub fn plan_plugin_command(
    profile: &NativeDeliveryProfile,
    target: &SkillTarget,
    selection: &NativeSkillSelection,
    action: CopilotPluginAction,
) -> Result<CopilotPluginCommand, CopilotCommandError> {
    let directory = validate(
        profile,
        target,
        selection,
        TargetSurface::CopilotCli,
        "1.0.105",
    )?;
    let selector = format!("{}@{}", selection.native_name, selection.native_name);
    let mut arguments: Vec<OsString> = vec!["--no-auto-update".into(), "plugin".into()];
    match action {
        CopilotPluginAction::RegisterMarketplace => {
            arguments.extend(["marketplace".into(), "add".into(), directory.into()])
        }
        CopilotPluginAction::UnregisterMarketplace => arguments.extend([
            "marketplace".into(),
            "remove".into(),
            selection.native_name.clone().into(),
        ]),
        CopilotPluginAction::Install => arguments.extend(["install".into(), selector.into()]),
        CopilotPluginAction::Update => arguments.extend(["update".into(), selector.into()]),
        CopilotPluginAction::Uninstall => arguments.extend(["uninstall".into(), selector.into()]),
    }
    Ok(CopilotPluginCommand { arguments })
}

pub mod probe;
