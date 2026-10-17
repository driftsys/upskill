//! Application with durable initial-state provenance, using the existing executor.
use anyhow::{Context, Result, ensure};

use super::{
    ExistingSchema2Write, FirstSchema2Write, apply_first_schema2_write, apply_schema2_write,
};
use crate::files::plan::{FileLocation, FileOperationPlan, FileValue, PlannedFile};
use crate::filesystem::{
    ExecutionReport, ExecutionRequest, QualifiedScope, ScopeWriterGuard, SourceSnapshot,
};
use crate::lifecycle::state::{
    CombinedState, StateOrigin, parse_state, serialize_state, validate_state,
};

pub const SCHEMA1_BACKUP_NAME: &str = ".upskill-lock.json.schema1.bak";

/// Captured inputs for fresh, migrated or reopened schema-2 application.
/// Origin and retained legacy history cannot be changed by later writes.
pub struct BoundSchema2Write<'state, 'source> {
    pub plan: FileOperationPlan,
    pub state_before: Option<FileValue>,
    pub backup_before: Option<FileValue>,
    pub state_location: FileLocation,
    pub backup_location: FileLocation,
    pub state_after: &'state CombinedState,
    pub sources: &'source [&'source SourceSnapshot<'source>],
}

/// Apply a complete plan after binding original state and backup evidence.
///
/// Fresh state requires absent state and backup, creates a private state file,
/// and journals backup absence without inventing original bytes. Migrated state
/// uses the existing exact-backup protocol. Reopened migrated state verifies its
/// backup against the immutable digest/length recorded at the first transition.
/// Unbound historical schema-2 state is inspectable but requires reconciliation
/// before this applying boundary can use it. This API does not activate commands.
pub fn apply_bound_schema2_write(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    operation: BoundSchema2Write<'_, '_>,
) -> Result<ExecutionReport> {
    let BoundSchema2Write {
        plan,
        state_before,
        backup_before,
        state_location,
        backup_location,
        state_after,
        sources,
    } = operation;
    validate_state(state_after)?;
    ensure!(
        state_location.path == crate::lockfile::LOCKFILE_NAME
            && backup_location.path == SCHEMA1_BACKUP_NAME,
        "bound lifecycle requires canonical state and backup paths"
    );
    ensure!(
        plan.artifacts == state_after.artifacts,
        "combined state artifacts differ from the complete owned-artifact plan"
    );
    let origin = state_after
        .origin
        .as_ref()
        .context("applying state requires bound origin")?;
    if let Some(before) = &state_before {
        let json: serde_json::Value =
            serde_json::from_slice(&before.bytes).context("parse observed state")?;
        match json.get("schema").and_then(serde_json::Value::as_u64) {
            Some(1) => {
                ensure!(
                    origin == &StateOrigin::from_schema1_bytes(&before.bytes)?,
                    "schema-1 backup origin differs from original state"
                );
                let legacy: crate::lockfile::Lockfile = serde_json::from_slice(&before.bytes)?;
                ensure!(
                    state_after.legacy.as_ref() == Some(&legacy),
                    "first transition must retain exact typed legacy history"
                );
                if let Some(backup) = &backup_before {
                    ensure!(
                        backup.bytes == before.bytes && backup.mode == Some(0o600),
                        "observed backup differs from original schema-1 state"
                    );
                }
                return apply_first_schema2_write(
                    scope,
                    guard,
                    FirstSchema2Write {
                        plan,
                        state_before: before.clone(),
                        state_location,
                        backup_location,
                        state_after,
                        sources,
                    },
                );
            }
            Some(2) => {
                let prior = parse_state(std::str::from_utf8(&before.bytes)?)?;
                ensure!(
                    prior.origin.as_ref() == Some(origin),
                    "state origin is unknown or changed; reconcile before applying"
                );
                ensure!(
                    prior.legacy == state_after.legacy,
                    "later writes must retain legacy history"
                );
            }
            _ => anyhow::bail!("unsupported observed lifecycle state schema"),
        }
    } else {
        ensure!(
            origin == &StateOrigin::Fresh {},
            "absent state requires fresh origin"
        );
    }
    match origin {
        StateOrigin::Migrated { .. } => {
            let backup = backup_before.context("original schema-1 backup is missing")?;
            ensure!(
                backup.mode == Some(0o600) && origin.matches_backup(&backup.bytes),
                "original schema-1 backup binding changed"
            );
            let original: crate::lockfile::Lockfile = serde_json::from_slice(&backup.bytes)?;
            ensure!(
                state_after.legacy.as_ref() == Some(&original),
                "retained legacy history differs from original backup"
            );
            apply_schema2_write(
                scope,
                guard,
                ExistingSchema2Write {
                    plan,
                    state_before: state_before.context("migrated state is absent")?,
                    state_location,
                    backup_original: backup,
                    backup_location,
                    state_after,
                    sources,
                },
            )
        }
        StateOrigin::Fresh {} => {
            ensure!(
                backup_before.is_none(),
                "fresh state cannot have a historical backup"
            );
            let mode = match &state_before {
                Some(before) => before.mode.context("state mode is unknown")?,
                None => 0o600,
            };
            let mut files = plan.files;
            files.push(PlannedFile {
                location: backup_location,
                before: None,
                after: None,
            });
            let request = ExecutionRequest {
                files,
                state: PlannedFile {
                    location: state_location,
                    before: state_before,
                    after: Some(FileValue {
                        bytes: serialize_state(state_after)?.into_bytes(),
                        mode: Some(mode),
                    }),
                },
            };
            request.validate()?;
            scope.apply_operation(guard, sources, request)
        }
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use super::*;
    use crate::filesystem::execution::{ExecutionOutcome, TEST_HOOK};
    use crate::ownership::{InstallationRoot, OwnershipScope};

    fn empty_fresh() -> CombinedState {
        CombinedState {
            schema: 2,
            origin: Some(StateOrigin::Fresh {}),
            installations: vec![],
            bundles: vec![],
            references: vec![],
            artifacts: vec![],
            owner_consumers: vec![],
            owner_contracts: vec![],
            legacy: None,
        }
    }

    #[test]
    fn interrupted_fresh_request_recovers_origin_or_preserves_late_backup_arrival() {
        for late_backup in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            let root = InstallationRoot {
                scope: OwnershipScope::Project,
                path: tmp.path().into(),
            };
            let scope = QualifiedScope::open(&root).unwrap();
            let guard = scope.try_lock_writer().unwrap();
            let location = |path: &str| FileLocation {
                root: root.clone(),
                path: path.into(),
            };
            let plan = FileOperationPlan {
                artifacts: vec![],
                files: ["one.md", "two.md"]
                    .into_iter()
                    .map(|path| PlannedFile {
                        location: location(path),
                        before: None,
                        after: Some(FileValue {
                            bytes: path.as_bytes().into(),
                            mode: Some(0o600),
                        }),
                    })
                    .collect(),
            };
            let backup_path = tmp.path().join(SCHEMA1_BACKUP_NAME);
            let arrival = backup_path.clone();
            TEST_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |point, _| {
                    if point == "after_initial_journal" {
                        if late_backup {
                            std::fs::write(&arrival, b"unexpected historical backup")?;
                        }
                        anyhow::bail!("interrupt published fresh operation");
                    }
                    Ok(())
                }))
            });
            let state = empty_fresh();
            let interrupted = apply_bound_schema2_write(
                &scope,
                &guard,
                BoundSchema2Write {
                    plan,
                    state_before: None,
                    backup_before: None,
                    state_location: location(crate::lockfile::LOCKFILE_NAME),
                    backup_location: location(SCHEMA1_BACKUP_NAME),
                    state_after: &state,
                    sources: &[],
                },
            );
            TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
            assert!(interrupted.is_err());
            assert!(!tmp.path().join(crate::lockfile::LOCKFILE_NAME).exists());
            let result = super::super::recover(&scope, &guard).unwrap().unwrap();
            if late_backup {
                assert_eq!(result.outcome, ExecutionOutcome::Reconcile { index: 2 });
                assert_eq!(
                    std::fs::read(&backup_path).unwrap(),
                    b"unexpected historical backup"
                );
                assert!(!tmp.path().join("one.md").exists());
                assert!(!tmp.path().join("two.md").exists());
                assert!(!tmp.path().join(crate::lockfile::LOCKFILE_NAME).exists());
                assert!(scope.has_pending_operation().unwrap());
            } else {
                assert_eq!(result.outcome, ExecutionOutcome::Complete);
                assert_eq!(
                    parse_state(
                        &std::fs::read_to_string(tmp.path().join(crate::lockfile::LOCKFILE_NAME))
                            .unwrap()
                    )
                    .unwrap(),
                    state
                );
                assert_eq!(std::fs::read(tmp.path().join("one.md")).unwrap(), b"one.md");
                assert_eq!(std::fs::read(tmp.path().join("two.md")).unwrap(), b"two.md");
                assert!(!backup_path.exists());
                assert!(!scope.has_pending_operation().unwrap());
            }
        }
    }
}
