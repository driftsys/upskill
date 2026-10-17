//! Read-only descriptions of lifecycle plans over captured evidence.
//!
//! Preview neither resolves sources nor establishes permission to apply a plan.
use anyhow::Result;

use super::inspection::inspect_state;
use super::retire::plan_reference_removal;
use super::state::{CombinedState, Reference, serialize_state};
use super::upsert::{UpsertOutcome, UpsertPlan, UpsertRequest, plan_shared_upsert, plan_upsert};
use crate::files::plan::{ObservedFile, PlannedFile};
use crate::lifecycle::selection::SelectionDiagnostic;

/// Explicit captured request; shared upserts retain their separate opt-in boundary.
#[derive(Debug, Clone, Copy)]
pub enum PreviewRequest<'a> {
    Upsert(UpsertRequest<'a>),
    SharedUpsert(UpsertRequest<'a>),
    /// Exact graph edges, not CLI names or inferred legacy ownership.
    RemoveReferences(&'a [Reference]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewOutcome {
    Ready(Box<LifecyclePreview>),
    /// Selection blocked the entire operation; no partial actions are returned.
    Blocked(Vec<SelectionDiagnostic>),
}

/// Complete proposed records and output actions, with the exact captured input bytes.
/// The output actions exclude the final state write and migration backup. Those
/// remain guarded application responsibilities, not preview execution capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecyclePreview {
    pub original_state: Vec<u8>,
    pub state: CombinedState,
    pub state_change: StateChange,
    pub files: Vec<FilePreview>,
}

/// Semantic record comparison, not a prediction of serialization-only rewrites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateChange {
    Unchanged,
    Changed,
    /// An applying transition would require the exact schema-1 backup prerequisite.
    UpgradeFromSchema1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePreview {
    pub plan: PlannedFile,
    pub change: FileChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChange {
    Create,
    Content,
    Mode,
    ContentAndMode,
    Remove,
    Unchanged,
}

fn describe(plan: PlannedFile) -> FilePreview {
    let change = match (&plan.before, &plan.after) {
        (None, Some(_)) => FileChange::Create,
        (Some(_), None) => FileChange::Remove,
        (None, None) => FileChange::Unchanged,
        (Some(before), Some(after)) => {
            match (before.bytes != after.bytes, before.mode != after.mode) {
                (true, true) => FileChange::ContentAndMode,
                (true, false) => FileChange::Content,
                (false, true) => FileChange::Mode,
                (false, false) => FileChange::Unchanged,
            }
        }
    };
    FilePreview { plan, change }
}

/// Plan from captured state, complete desired sets and supplied output observations.
///
/// This function performs no I/O, fetching, migration, locking or recovery. It
/// preserves planner action order and exact before/after bytes and modes, including
/// no-ops. Invalid evidence or graph/artifact conflicts return the planner's error
/// without a partial preview; selection blockers remain typed. Qualified freshness,
/// source authenticity, native support and application authority are not established.
pub fn preview(
    state_bytes: &[u8],
    request: PreviewRequest<'_>,
    observations: &[ObservedFile],
) -> Result<PreviewOutcome> {
    let inspection = inspect_state(state_bytes)?;
    let outcome = match request {
        PreviewRequest::Upsert(request) => plan_upsert(&inspection.state, request, observations)?,
        PreviewRequest::SharedUpsert(request) => {
            plan_shared_upsert(&inspection.state, request, observations)?
        }
        PreviewRequest::RemoveReferences(remove) => {
            let plan = plan_reference_removal(&inspection.state, remove, observations)?;
            UpsertOutcome::Ready(Box::new(UpsertPlan {
                state: plan.state,
                files: plan.files,
            }))
        }
    };
    let plan = match outcome {
        UpsertOutcome::Ready(plan) => plan,
        UpsertOutcome::Blocked(diagnostics) => return Ok(PreviewOutcome::Blocked(diagnostics)),
    };
    let UpsertPlan { state, files } = *plan;
    let state_change = if serde_json::from_slice::<serde_json::Value>(state_bytes)?["schema"] == 1 {
        StateChange::UpgradeFromSchema1
    } else if serialize_state(&state)? == serialize_state(&inspection.state)? {
        StateChange::Unchanged
    } else {
        StateChange::Changed
    };
    Ok(PreviewOutcome::Ready(Box::new(LifecyclePreview {
        original_state: inspection.original_bytes,
        state,
        state_change,
        files: files.into_iter().map(describe).collect(),
    })))
}
