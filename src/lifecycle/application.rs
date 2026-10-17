//! Guarded application of a first combined-state lifecycle operation.
//!
//! This boundary binds the complete owned-artifact plan to the state committed by
//! the same executor journal. The schema-1 backup and all output/state actions are
//! applied under the matching qualified scope writer guard.

use anyhow::{Context, Result, ensure};

use crate::files::plan::{FileLocation, FileOperationPlan, FileValue, PlannedFile};
use crate::filesystem::{
    ExecutionReport, QualifiedScope, ScopeWriterGuard, SourceSnapshot, execution::ExecutionRequest,
};
use crate::lifecycle::state::{CombinedState, serialize_state};

/// Fully captured inputs for a first schema-2 lifecycle transition.
pub struct FirstSchema2Write<'state, 'source> {
    pub plan: FileOperationPlan,
    pub state_before: FileValue,
    pub state_location: FileLocation,
    pub backup_location: FileLocation,
    pub state_after: &'state CombinedState,
    pub sources: &'source [&'source SourceSnapshot<'source>],
}

/// Fully captured inputs for a later schema-2 lifecycle transition.
pub struct ExistingSchema2Write<'state, 'source> {
    pub plan: FileOperationPlan,
    pub state_before: FileValue,
    pub state_location: FileLocation,
    pub backup_original: FileValue,
    pub backup_location: FileLocation,
    pub state_after: &'state CombinedState,
    pub sources: &'source [&'source SourceSnapshot<'source>],
}

/// Apply the first schema-2 transition after confirming it is backed by schema 1.
///
/// The resulting state must contain exactly the artifacts produced by the supplied
/// complete plan. The executor requalifies every before value, writes a private
/// exact-byte schema-1 backup before other files, checkpoints the final state, and
/// uses its durable journal for interruption recovery. This API does not activate
/// production commands or infer ownership from the schema-1 snapshot.
pub fn apply_first_schema2_write(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    operation: FirstSchema2Write<'_, '_>,
) -> Result<ExecutionReport> {
    let FirstSchema2Write {
        plan,
        state_before,
        state_location,
        backup_location,
        state_after,
        sources,
    } = operation;
    ensure!(state_after.schema == 2, "lifecycle state must use schema 2");
    let legacy: serde_json::Value =
        serde_json::from_slice(&state_before.bytes).context("parse observed schema-1 state")?;
    ensure!(
        legacy.get("schema").and_then(serde_json::Value::as_u64) == Some(1),
        "first schema-2 write requires an observed schema-1 state"
    );
    ensure!(
        plan.artifacts == state_after.artifacts,
        "combined state artifacts differ from the complete owned-artifact plan"
    );
    let state_bytes = serialize_state(state_after)?.into_bytes();
    let state_after_value = FileValue {
        bytes: state_bytes,
        mode: Some(
            state_before
                .mode
                .context("schema-1 state mode is unknown")?,
        ),
    };
    let request = ExecutionRequest {
        files: plan.files,
        state: PlannedFile {
            location: state_location,
            before: Some(state_before),
            after: Some(state_after_value),
        },
    };
    request.validate()?;
    scope.apply_operation_with_backup(guard, sources, request, backup_location)
}

/// Apply a later schema-2 transition while rechecking the immutable schema-1 backup.
///
/// The backup is included as a journaled no-op action, so its exact bytes and mode
/// are requalified with all other preconditions and remain represented during
/// interruption recovery.
pub fn apply_schema2_write(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    operation: ExistingSchema2Write<'_, '_>,
) -> Result<ExecutionReport> {
    let ExistingSchema2Write {
        plan,
        state_before,
        state_location,
        backup_original,
        backup_location,
        state_after,
        sources,
    } = operation;
    ensure!(state_after.schema == 2, "lifecycle state must use schema 2");
    let prior: serde_json::Value =
        serde_json::from_slice(&state_before.bytes).context("parse prior schema-2 state")?;
    ensure!(
        prior.get("schema").and_then(serde_json::Value::as_u64) == Some(2),
        "later schema-2 write requires an observed schema-2 state"
    );
    let original: serde_json::Value =
        serde_json::from_slice(&backup_original.bytes).context("parse exact schema-1 backup")?;
    ensure!(
        original.get("schema").and_then(serde_json::Value::as_u64) == Some(1),
        "retained backup must contain schema-1 state"
    );
    ensure!(
        plan.artifacts == state_after.artifacts,
        "combined state artifacts differ from the complete owned-artifact plan"
    );
    let state_after_value = FileValue {
        bytes: serialize_state(state_after)?.into_bytes(),
        mode: Some(
            state_before
                .mode
                .context("schema-2 state mode is unknown")?,
        ),
    };
    let mut files = plan.files;
    files.push(PlannedFile {
        location: backup_location,
        before: Some(backup_original.clone()),
        after: Some(backup_original),
    });
    let request = ExecutionRequest {
        files,
        state: PlannedFile {
            location: state_location,
            before: Some(state_before),
            after: Some(state_after_value),
        },
    };
    request.validate()?;
    scope.apply_operation(guard, sources, request)
}

/// Recover an interrupted lifecycle application while holding the same scope lock.
///
/// Recovery outcomes remain those of the qualified executor; a reconciliation
/// result is surfaced without attempting a separate lifecycle rewrite.
pub fn recover(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
) -> Result<Option<ExecutionReport>> {
    scope.recover_operation(guard)
}
