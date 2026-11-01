//! Fixed Claude metadata queries. Production profile admission stays separate.
use super::{ClaudeInventoryError, ClaudePluginInventory};
use crate::filesystem::{QualifiedScope, ScopeBinding, ScopeWriterGuard};
use crate::native_delivery::process::{
    NativeCommandSpec, NativeExecutable, NativeGroupState, NativeProcessError, NativeProcessOutput,
    NativeStopReason, run_native_process,
};
use std::fmt;
use std::sync::atomic::AtomicBool;

#[derive(Debug, thiserror::Error)]
pub enum ClaudeInventoryReadError {
    #[error("Claude inventory queries require the fixed command contract without extra arguments")]
    UnexpectedArguments,
    #[error("Claude inventory CWD binding changed or is unavailable")]
    ScopeChanged,
    #[error("Claude runtime version does not match the supported inventory contract")]
    VersionMismatch,
    #[error(
        "Claude inventory transport is incomplete, refused or unresolved; no native absence is established"
    )]
    UnresolvedTransport,
    #[error(transparent)]
    Invocation(#[from] NativeProcessError),
    #[error(transparent)]
    Inventory(#[from] ClaudeInventoryError),
}
/// Metadata from two completed local processes, bound to their captured launcher
/// and CWD. Effective configuration, delegated runtime, ownership, activation,
/// native worker quiescence and resource retention are not qualified here.
pub struct ClaudeInventoryCapture {
    pub inventory: ClaudePluginInventory,
    pub cwd: ScopeBinding,
    pub executable: NativeExecutable,
}
impl fmt::Debug for ClaudeInventoryCapture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudeInventoryCapture")
            .field("entries", &self.inventory.entries().len())
            .finish_non_exhaustive()
    }
}
fn require_complete(output: &NativeProcessOutput) -> Result<(), ClaudeInventoryReadError> {
    if output.reason != NativeStopReason::Exited
        || output.exit_code != Some(0)
        || output.group != NativeGroupState::Empty
        || !output.direct_child_reaped
        || !output.output_complete
        || !output.scope_unchanged
        || !output.executable_unchanged
    {
        return Err(ClaudeInventoryReadError::UnresolvedTransport);
    }
    Ok(())
}
/// Query only `--version`, then `plugin list --json`, with the same captured
/// launcher, explicit environment and qualified CWD. Template arguments must be
/// empty. Raw output and diagnostics are processed in memory, never persisted.
/// This candidate reader is not routed from the CLI and grants no admission;
/// production must pass the separate profile gate before calling an adapter.
pub fn read_plugin_inventory(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    template: &NativeCommandSpec,
    cancellation: &AtomicBool,
) -> Result<ClaudeInventoryCapture, ClaudeInventoryReadError> {
    if !template.arguments.is_empty() {
        return Err(ClaudeInventoryReadError::UnexpectedArguments);
    }
    let cwd = scope
        .binding()
        .map_err(|_| ClaudeInventoryReadError::ScopeChanged)?;
    let command = |arguments: &[&str], limit: usize| NativeCommandSpec {
        executable: template.executable.clone(),
        arguments: arguments.iter().map(|arg| (*arg).into()).collect(),
        environment: template.environment.clone(),
        timeout: template.timeout,
        output_limit: template.output_limit.min(limit),
    };
    let version = run_native_process(scope, guard, &command(&["--version"], 256), cancellation)?;
    require_complete(&version)?;
    if version.stdout.trim_ascii() != b"2.1.308 (Claude Code)" {
        return Err(ClaudeInventoryReadError::VersionMismatch);
    }
    let listed = run_native_process(
        scope,
        guard,
        &command(&["plugin", "list", "--json"], 1024 * 1024),
        cancellation,
    )?;
    require_complete(&listed)?;
    let inventory = ClaudePluginInventory::parse("2.1.308", &listed.stdout)?;
    if scope
        .binding()
        .map_err(|_| ClaudeInventoryReadError::ScopeChanged)?
        != cwd
    {
        return Err(ClaudeInventoryReadError::ScopeChanged);
    }
    Ok(ClaudeInventoryCapture {
        inventory,
        cwd,
        executable: template.executable.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn complete() -> NativeProcessOutput {
        NativeProcessOutput {
            reason: NativeStopReason::Exited,
            exit_code: Some(0),
            stdout: b"[]".to_vec(),
            stderr: vec![],
            group: NativeGroupState::Empty,
            direct_child_reaped: true,
            cleanup_signal_sent: true,
            output_complete: true,
            scope_unchanged: true,
            executable_unchanged: true,
        }
    }
    #[test]
    fn every_incomplete_transport_fact_refuses_metadata_independently() {
        require_complete(&complete()).unwrap();
        let mut denied_zombie_signal = complete();
        denied_zombie_signal.cleanup_signal_sent = false;
        require_complete(&denied_zombie_signal).unwrap();
        for case in 0..11 {
            let mut output = complete();
            match case {
                0 => output.reason = NativeStopReason::TimedOut,
                1 => output.reason = NativeStopReason::Cancelled,
                2 => output.reason = NativeStopReason::OutputLimit,
                3 => output.reason = NativeStopReason::OutputReadFailed,
                4 => output.exit_code = Some(17),
                5 => output.exit_code = None,
                6 => output.group = NativeGroupState::Unresolved,
                7 => output.direct_child_reaped = false,
                8 => output.output_complete = false,
                9 => output.scope_unchanged = false,
                _ => output.executable_unchanged = false,
            }
            assert!(
                matches!(
                    require_complete(&output),
                    Err(ClaudeInventoryReadError::UnresolvedTransport)
                ),
                "case {case} accepted"
            );
        }
    }
}
