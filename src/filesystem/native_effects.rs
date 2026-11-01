//! Descriptor-based effects driven by the separate native operation journal.
use super::{QualifiedScope, ScopeWriterGuard, execution::backend};
use crate::files::plan::{FileValue, PlannedFile};
use anyhow::{Result, ensure};

pub(crate) fn preflight(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    action: &PlannedFile,
) -> Result<()> {
    guard.require_scope(scope)?;
    crate::ownership::validate_relative_path(&action.location.path)?;
    for value in [&action.before, &action.after].into_iter().flatten() {
        ensure!(
            value.mode.is_some_and(|mode| mode <= 0o777),
            "native file effect requires known ordinary modes"
        );
    }
    if let Some(after) = &action.after {
        if let Some(before) = &action.before {
            ensure!(
                before.mode == after.mode,
                "native file effect must preserve existing mode"
            );
        } else {
            ensure!(
                matches!(after.mode, Some(0o600 | 0o700)),
                "new native file requires a private mode"
            );
        }
    }
    backend::qualify_metadata(scope, &action.location)?;
    guard.verify_binding()
}
pub(crate) fn observe(scope: &QualifiedScope, action: &PlannedFile) -> Result<Option<FileValue>> {
    let snapshot = scope.observe_destinations(std::slice::from_ref(&action.location))?;
    let value = snapshot.observations()[0].value.clone();
    snapshot.verify_unchanged()?;
    Ok(value)
}
pub(crate) fn apply(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    action: &PlannedFile,
    immutable: bool,
    index: usize,
) -> Result<()> {
    preflight(scope, guard, action)?;
    ensure!(
        observe(scope, action)? == action.before,
        "native file precondition changed"
    );
    if immutable {
        backend::apply_create_only(scope, action, index)?;
    } else {
        backend::apply(scope, action, index)?;
    }
    if action.before.is_some() || action.after.is_some() {
        backend::sync_post(scope, &action.location, &action.after)?;
    }
    ensure!(
        observe(scope, action)? == action.after,
        "native file effect did not reach its after-state"
    );
    guard.verify_binding()
}
pub(crate) fn sync(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    action: &PlannedFile,
) -> Result<()> {
    preflight(scope, guard, action)?;
    if action.after.is_some() || action.before.is_some() {
        backend::sync_post(scope, &action.location, &action.after)?;
    }
    ensure!(
        observe(scope, action)? == action.after,
        "native file post-state changed"
    );
    guard.verify_binding()
}
