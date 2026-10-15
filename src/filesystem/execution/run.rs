use super::{
    ExecutionOutcome, ExecutionReport, ExecutionRequest, backend,
    journal::{Journal, Receipt},
    request,
};
use crate::files::plan::{FileLocation, PlannedFile};
use crate::filesystem::{QualifiedScope, ScopeWriterGuard, SourceSnapshot};
use crate::operations::recovery::{
    ActionKind, ObservedRelation, RecoveryAction, RecoveryDecision, RecoveryStep, plan_recovery,
};
use anyhow::{Context, Result, ensure};

impl QualifiedScope {
    /// Apply a caller-authorized complete file plan under its matching scope guard.
    /// This opt-in API does not establish ownership or activate CLI commands.
    pub fn apply_operation(
        &self,
        guard: &ScopeWriterGuard<'_>,
        sources: &[&SourceSnapshot<'_>],
        request: ExecutionRequest,
    ) -> Result<ExecutionReport> {
        guard.require_scope(self)?;
        request::validate(&request)?;
        for source in sources {
            source.verify_unchanged()?;
        }
        let actions: Vec<_> = request
            .files
            .iter()
            .chain(std::iter::once(&request.state))
            .collect();
        let locations: Vec<_> = actions.iter().map(|a| a.location.clone()).collect();
        let snapshot = self.observe_destinations(&locations)?;
        for action in &actions {
            let actual = snapshot
                .observations()
                .iter()
                .find(|v| v.location == action.location)
                .context("missing qualified preflight location")?;
            ensure!(
                action.before == actual.value,
                "execution precondition changed: {}",
                action.location.path
            );
        }
        for action in &actions {
            backend::qualify_metadata(self, &action.location)?;
        }
        snapshot.verify_unchanged()?;
        for source in sources {
            source.verify_unchanged()?;
        }
        guard.verify_binding()?;
        ensure!(
            Journal::load(self)?.is_none(),
            "recover existing execution before starting another operation"
        );
        drive(self, guard, Journal::create(self, &request)?)
    }
    /// Recover validated durable file receipts, preserving unexpected observations.
    pub fn recover_operation(
        &self,
        guard: &ScopeWriterGuard<'_>,
    ) -> Result<Option<ExecutionReport>> {
        guard.require_scope(self)?;
        Journal::load(self)?
            .map(|journal| drive(self, guard, journal))
            .transpose()
    }
}
fn drive(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    mut journal: Journal<'_>,
) -> Result<ExecutionReport> {
    loop {
        let recorded_prefix = journal
            .disk
            .actions
            .iter()
            .take_while(|action| action.receipt == Receipt::Done)
            .count();
        // Until the complete observation succeeds, no current post-state prefix is verified.
        let qualification_context = || {
            format!(
                "execution stopped with recorded prefix {recorded_prefix}, verified post-state prefix 0; current observations unavailable"
            )
        };
        guard
            .require_scope(scope)
            .with_context(qualification_context)?;
        let locations: Vec<_> = journal
            .disk
            .actions
            .iter()
            .map(|a| FileLocation {
                root: journal.disk.root.clone(),
                path: a.path.clone(),
            })
            .collect();
        let observations =
            backend::observe(scope, &locations).with_context(qualification_context)?;
        for location in &locations {
            backend::qualify_metadata(scope, location).with_context(qualification_context)?;
        }
        let steps: Vec<_> = journal
            .disk
            .actions
            .iter()
            .zip(&observations)
            .map(|(action, actual)| {
                let matches =
                    |expected: &Option<super::journal::Evidence>| match (expected, &actual.value) {
                        (None, None) => true,
                        (Some(e), Some(v)) => e.matches(v),
                        _ => false,
                    };
                let observed = match (matches(&action.before), matches(&action.after)) {
                    (true, true) => ObservedRelation::Both,
                    (true, false) => ObservedRelation::Before,
                    (false, true) => ObservedRelation::After,
                    (false, false) => ObservedRelation::Neither,
                };
                RecoveryAction {
                    kind: if action.state {
                        ActionKind::State
                    } else {
                        ActionKind::File
                    },
                    checkpoint: action.receipt.checkpoint(),
                    observed,
                }
            })
            .collect();
        match plan_recovery(&steps).with_context(qualification_context)? {
            RecoveryDecision::Complete { completed } => {
                journal.cleanup().with_context(|| format!("clean completed execution with recorded prefix {completed}, verified post-state prefix {completed}; retained receipts require reconciliation on failure"))?;
                return Ok(ExecutionReport {
                    recorded_prefix: completed,
                    verified_post_prefix: completed,
                    outcome: ExecutionOutcome::Complete,
                });
            }
            RecoveryDecision::Reconcile {
                index,
                recorded_prefix,
                verified_post_prefix,
            } => {
                return Ok(ExecutionReport {
                    recorded_prefix,
                    verified_post_prefix,
                    outcome: ExecutionOutcome::Reconcile { index },
                });
            }
            RecoveryDecision::Ready {
                recorded_prefix,
                verified_post_prefix,
                step,
            } => {
                let result = (|| -> Result<()> {
                    match step {
                        RecoveryStep::Apply { index } => {
                            super::checkpoint("before_pending", index)?;
                            journal.record(index, Receipt::Pending)?;
                            super::checkpoint("after_pending", index)?;
                            let action = PlannedFile {
                                location: locations[index].clone(),
                                before: observations[index].value.clone(),
                                after: journal.payload(index)?,
                            };
                            backend::apply(scope, &action, index)
                        }
                        RecoveryStep::Acknowledge { index }
                        | RecoveryStep::CheckpointOnly { index } => {
                            if matches!(step, RecoveryStep::Acknowledge { .. }) {
                                backend::sync_post(
                                    scope,
                                    &locations[index],
                                    &observations[index].value,
                                )?;
                                super::checkpoint("after_ack_sync", index)?;
                            }
                            super::checkpoint("before_done", index)?;
                            journal.record(index, Receipt::Done)?;
                            super::checkpoint("after_done", index)
                        }
                    }
                })();
                result.with_context(|| format!("execution stopped with recorded prefix {recorded_prefix}, verified post-state prefix {verified_post_prefix}"))?;
            }
        }
    }
}
#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
