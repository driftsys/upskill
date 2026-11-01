//! Pending/observation metadata; effect execution and terminal state are separate.
use super::{
    NativeJournalStatus, NativeOperationJournal, NativeOperationStore, checkpoint, codec::*,
};
use crate::lifecycle::native_receipt::NativeBindingOutcome;
use crate::native_delivery::adapter::*;
use anyhow::{Context, Result, ensure};

impl<'locks, 'scope: 'locks> NativeOperationStore<'locks, 'scope> {
    /// Sync Pending before returning the recorded next step. This is metadata,
    /// not filesystem preflight or authority to replay an uncertain native call.
    /// A pending step can only be observed here, never begun a second time.
    pub fn begin_execution_step(&self, id: &str, index: usize) -> Result<NativeExecutionStep> {
        let inspection = self.inspect(id)?;
        ensure!(
            inspection.markers_complete,
            "pending checkpoint requires every root marker"
        );
        let mut journal = inspection.journal;
        let execution = journal
            .execution
            .as_ref()
            .context("native execution not planned")?;
        let step = execution
            .steps
            .get(index)
            .context("native step index out of range")?
            .clone();
        ensure!(
            journal.checkpoints[..index]
                .iter()
                .all(|row| row.phase == NativeStepPhase::ObservedAfter)
                && journal.checkpoints[index].phase == NativeStepPhase::Queued,
            "native step is not the next queued effect"
        );
        journal.checkpoints[index].phase = NativeStepPhase::Pending;
        self.save_checkpoint(id, journal)?;
        checkpoint("after_pending", index, id)?;
        Ok(step)
    }

    /// Preserve the caller's native attempt outcome without acknowledging an
    /// effect. Confirmed, rejected and unknown outcomes all need an independent
    /// observation; this method itself calls no native command.
    pub fn record_native_attempt(
        &self,
        id: &str,
        index: usize,
        outcome: NativeBindingOutcome,
        detail: Option<String>,
    ) -> Result<()> {
        let inspection = self.inspect(id)?;
        ensure!(
            inspection.markers_complete,
            "attempt checkpoint requires every root marker"
        );
        let mut journal = inspection.journal;
        let execution = journal
            .execution
            .as_ref()
            .context("native execution not planned")?;
        ensure!(
            matches!(
                execution.steps.get(index),
                Some(NativeExecutionStep::Native(_))
            ) && journal.checkpoints[index].phase == NativeStepPhase::Pending,
            "native attempt requires a pending native effect"
        );
        journal.checkpoints[index]
            .attempts
            .push(NativeAttempt { outcome, detail });
        self.save_checkpoint(id, journal)?;
        checkpoint("after_attempt", index, id)
    }

    /// Independently observe a pending step. Unknown, divergence or observer
    /// failure remains Pending with evidence retained. Native effect relation
    /// is distinct from final effective binding, which needs its own later gate.
    pub fn observe_pending_step(
        &self,
        id: &str,
        index: usize,
        adapter: &mut dyn NativeDeliveryAdapter,
    ) -> Result<bool> {
        let inspection = self.inspect(id)?;
        ensure!(
            inspection.markers_complete,
            "observation requires every root marker"
        );
        let mut journal = inspection.journal;
        let execution = journal
            .execution
            .as_ref()
            .context("native execution not planned")?;
        let step = execution
            .steps
            .get(index)
            .context("native step index out of range")?;
        ensure!(
            journal.checkpoints[index].phase == NativeStepPhase::Pending,
            "native step is not pending"
        );
        let observation = match step {
            NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file) => {
                let (scope, _) = self.roots.scope_and_guard(&file.location.root)?;
                let snapshot = scope.observe_destinations(std::slice::from_ref(&file.location))?;
                let actual = snapshot.observations()[0].clone();
                snapshot.verify_unchanged()?;
                NativeStepObservation::File(actual)
            }
            NativeExecutionStep::Native(effect) => {
                let profile =
                    adapter.profile(&effect.request.change.key, &effect.request.change.scope)?;
                ensure!(
                    profile == effect.request.profile,
                    "native adapter profile changed"
                );
                match adapter.observe(effect) {
                    Ok(observation) => {
                        validate_native(effect, &observation)?;
                        NativeStepObservation::Native(Box::new(observation))
                    }
                    Err(error) => NativeStepObservation::Failed(format!("{error:#}")),
                }
            }
        };
        let after = observed_after(step, &observation);
        let row = &mut journal.checkpoints[index];
        row.observations.push(observation);
        if after {
            row.phase = NativeStepPhase::ObservedAfter;
        }
        self.save_checkpoint(id, journal)?;
        checkpoint("after_observation", index, id)?;
        Ok(after)
    }

    fn save_checkpoint(&self, id: &str, mut journal: NativeOperationJournal) -> Result<()> {
        let before = self
            .state
            .evidence()
            .read_record(&journal_name(id))?
            .context("native journal missing")?;
        let previous: NativeOperationJournal = json(&before)?;
        let mut expected = previous.clone();
        expected.checkpoints = journal.checkpoints.clone();
        ensure!(
            expected == journal && journal.status == NativeJournalStatus::Executing,
            "native journal changed before checkpoint publication"
        );
        journal.generation = journal
            .generation
            .checked_add(1)
            .context("native journal generation overflow")?;
        validate(
            &journal,
            journal
                .execution
                .as_ref()
                .context("native execution not planned")?,
        )?;
        self.roots.verify_bindings()?;
        self.state.evidence().write_record(
            &journal_name(id),
            Some(&before),
            Some(&serde_json::to_vec(&journal)?),
        )?;
        self.inspect(id)?;
        Ok(())
    }
}

pub(super) fn validate(
    journal: &NativeOperationJournal,
    execution: &NativeExecutionPlan,
) -> Result<()> {
    ensure!(
        journal.checkpoints.len() == execution.steps.len(),
        "native checkpoint coverage differs"
    );
    let mut unfinished = false;
    for (step, row) in execution.steps.iter().zip(&journal.checkpoints) {
        if unfinished {
            ensure!(
                row.phase == NativeStepPhase::Queued,
                "native checkpoints are out of order"
            );
        }
        if row.phase != NativeStepPhase::ObservedAfter {
            unfinished = true;
        }
        ensure!(
            row.phase != NativeStepPhase::Queued
                || (row.attempts.is_empty() && row.observations.is_empty()),
            "queued native step has effect evidence"
        );
        ensure!(
            matches!(step, NativeExecutionStep::Native(_)) || row.attempts.is_empty(),
            "file step has native attempts"
        );
        for observation in &row.observations {
            match (step, observation) {
                (
                    NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file),
                    NativeStepObservation::File(actual),
                ) => ensure!(
                    file.location == actual.location,
                    "file observation belongs to another step"
                ),
                (
                    NativeExecutionStep::Native(effect),
                    NativeStepObservation::Native(observation),
                ) => validate_native(effect, observation)?,
                (NativeExecutionStep::Native(_), NativeStepObservation::Failed(message)) => {
                    ensure!(!message.is_empty(), "empty native observer failure")
                }
                _ => anyhow::bail!("native observation kind differs from step"),
            }
        }
        ensure!(
            row.phase != NativeStepPhase::ObservedAfter
                || row
                    .observations
                    .last()
                    .is_some_and(|observation| observed_after(step, observation)),
            "native step acknowledged without observed after-state"
        );
    }
    Ok(())
}
fn observed_after(step: &NativeExecutionStep, observation: &NativeStepObservation) -> bool {
    match (step, observation) {
        (
            NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file),
            NativeStepObservation::File(actual),
        ) => actual.location == file.location && actual.value == file.after,
        (NativeExecutionStep::Native(effect), NativeStepObservation::Native(observation)) => {
            observation.effect == NativeEffectRelation::After
                && known_binding(effect, &observation.binding)
        }
        _ => false,
    }
}
pub(super) fn validate_native(
    effect: &PlannedNativeEffect,
    observation: &NativeEffectObservation,
) -> Result<()> {
    let request = &effect.request;
    let binding = &observation.binding;
    ensure!(
        observation.operation_id == request.operation_id
            && observation.profile_id == request.profile.id
            && observation.profile_revision == request.profile.revision
            && binding.key == request.change.key
            && binding.scope == request.change.scope
            && binding.client_version == request.profile.client_version
            && binding.platform == request.profile.platform
            && !binding.observation_id.trim().is_empty(),
        "native observation attribution differs from recorded effect"
    );
    Ok(())
}

pub(super) fn known_binding(
    effect: &PlannedNativeEffect,
    binding: &crate::lifecycle::native_receipt::NativeSkillObservation,
) -> bool {
    use crate::lifecycle::native_receipt::NativeObservationState;
    match &binding.state {
        NativeObservationState::Absent => true,
        NativeObservationState::Present(selection) => {
            effect.request.change.before.as_ref() == Some(selection)
                || effect.request.change.after.as_ref() == Some(selection)
        }
        NativeObservationState::Unknown => false,
    }
}
