//! Inactive coordination over fake or separately qualified native adapters.
use super::*;
use crate::files::plan::{FileLocation, PlannedFile};
use crate::filesystem::native_effects as io;
use crate::lifecycle::native_receipt::*;
use crate::native_delivery::adapter::*;
use crate::native_delivery::state::NativeDeliveryState;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
#[cfg(test)]
mod process_tests;
mod restore;
mod state;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryDisposition {
    ObserveOnly,
    Forward,
    Restore,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDeliveryOutcome {
    Complete {
        operation_id: String,
    },
    Restored {
        operation_id: String,
    },
    RecoveryRequired {
        operation_id: Option<String>,
        reason: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum NativeCallDirection {
    Forward,
    Restore,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCallRecord {
    pub direction: NativeCallDirection,
    pub index: usize,
    pub precondition: NativeEffectObservation,
    pub retention_capture: NativeSkillObservation,
    /// None is durable call intent, not a claim that the command ran.
    pub outcome: Option<NativeBindingOutcome>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRuntime {
    pub calls: Vec<NativeCallRecord>,
    pub final_observations: Vec<NativeSkillObservation>,
    pub state_action: Option<PlannedFile>,
    pub state_phase: NativeStepPhase,
    pub restore: Option<restore::NativeRestoreRuntime>,
}
pub struct NativeDeliveryCoordinator;
impl NativeDeliveryCoordinator {
    pub fn apply(
        store: &NativeOperationStore<'_, '_>,
        adapter: &mut dyn NativeDeliveryAdapter,
        capture: NativeOperationCapture,
    ) -> Result<NativeDeliveryOutcome> {
        let _operation = match store.roots.try_operation() {
            Ok(guard) => guard,
            Err(error) => return Ok(required(None, error)),
        };
        Ok(
            Self::apply_inner(store, adapter, capture)
                .unwrap_or_else(|error| required(None, error)),
        )
    }
    fn apply_inner(
        store: &NativeOperationStore<'_, '_>,
        adapter: &mut dyn NativeDeliveryAdapter,
        mut capture: NativeOperationCapture,
    ) -> Result<NativeDeliveryOutcome> {
        if let Err(error) = store.require_no_pending() {
            let pending_id = error
                .downcast_ref::<NativeRecoveryRequired>()
                .map(|error| error.operation_id.clone());
            return Ok(required(pending_id.as_deref(), error));
        }
        let snapshot = store.state.snapshot()?;
        let prior = store.prior(&snapshot.observations()[0].value)?;
        // Untouched owned configuration still needs a closed file/layer
        // precondition. A caller can supply a fresh capture to preserve unrelated
        // manual edits; omission cannot bypass checks on the prior document.
        for config in &prior.configurations {
            if capture.configurations.iter().any(|current| {
                current.before.target.file.location == config.capture.target.file.location
                    && current.before.surface == config.capture.surface
            }) {
                continue;
            }
            let entry = config
                .capture
                .ownership
                .as_ref()
                .and_then(|ownership| ownership.entries.first())
                .context("prior configuration has no owned contribution")?;
            capture.configurations.push(NativeConfigurationCapture {
                before: config.capture.clone(),
                contribution: crate::native_config::ConfigContribution {
                    id: entry.id.clone(),
                    selector: entry.selector.clone(),
                    expected: Some(entry.clone()),
                    desired: Some(entry.value.clone()),
                },
                additional_contributions: vec![],
                owners_after: config.owners.clone(),
            });
        }
        let graph = crate::lifecycle::reconcile::reconcile_references(
            &prior.skills.graph,
            &capture.graph_changes,
        )?;
        let mut keys = std::collections::BTreeMap::new();
        for installation in prior
            .skills
            .graph
            .installations
            .iter()
            .chain(&graph.installations)
            .filter(|installation| installation.owner.kind == crate::pipeline::ItemKind::Skill)
        {
            for surface in &installation.requested {
                let key = SkillTarget {
                    installation_id: installation.owner.installation_id.clone(),
                    surface: *surface,
                };
                keys.insert(key.index_key(), key);
            }
        }
        for revision in prior
            .skills
            .revisions
            .iter()
            .chain(prior.skills.staged.iter().map(|staged| &staged.revision))
        {
            let key = SkillTarget {
                installation_id: revision.installation.owner.installation_id.clone(),
                surface: revision.selected.surface,
            };
            keys.insert(key.index_key(), key);
        }
        capture.native_observations.clear();
        let mut profiles = Vec::new();
        for key in keys.into_values() {
            let profile = adapter.profile(&key, &store.context().scope)?;
            ensure!(
                profile.surface == key.surface
                    && profile.scope == store.context().scope
                    && !profile.id.trim().is_empty()
                    && profile.revision > 0
                    && !profile.client_version.trim().is_empty()
                    && !profile.platform.trim().is_empty(),
                "native profile attribution is incomplete before capture"
            );
            let actual = adapter.capture(&key, &store.context().scope)?;
            ensure!(
                actual.client_version == profile.client_version
                    && actual.platform == profile.platform,
                "native profile changed during prestate capture"
            );
            profiles.push(NativeTargetProfile {
                key: key.clone(),
                profile,
            });
            ensure!(
                actual.key == key
                    && actual.scope == store.context().scope
                    && !actual.observation_id.trim().is_empty()
                    && !actual.client_version.trim().is_empty()
                    && !actual.platform.trim().is_empty(),
                "fresh native capture attribution differs"
            );
            capture.native_observations.push(actual);
        }
        // Replan with fresh claims so a caller-supplied absence cannot be adopted.
        let (plan, _) = store.plan(&prior, &capture)?;
        state::graph(&prior, &plan)?;
        snapshot.verify_unchanged()?;
        let prepared = match store.prepare(&capture) {
            Ok(prepared) => prepared,
            Err(error) => return Ok(required(None, error)),
        };
        let id = prepared.id();
        let result = (|| {
            store.prepare_execution(id, adapter)?;
            ensure!(
                store
                    .inspect(id)?
                    .journal
                    .execution
                    .as_ref()
                    .context("native execution missing")?
                    .profiles
                    == profiles,
                "native profile changed during durable preparation"
            );
            initialize_runtime(store, id)?;
            drive(store, adapter, id, true)
        })();
        Ok(result.unwrap_or_else(|error| required(Some(id), error)))
    }
    pub fn recover(
        store: &NativeOperationStore<'_, '_>,
        adapter: &mut dyn NativeDeliveryAdapter,
        id: &str,
        disposition: RecoveryDisposition,
    ) -> Result<NativeDeliveryOutcome> {
        let result = (|| {
            let _operation = store.roots.try_operation()?;
            let mut inspection = store.inspect(id)?;
            if disposition == RecoveryDisposition::Restore
                || matches!(
                    inspection.journal.status,
                    NativeJournalStatus::Restoring
                        | NativeJournalStatus::RetiringRestore
                        | NativeJournalStatus::Restored
                )
            {
                ensure!(
                    disposition != RecoveryDisposition::Forward,
                    "a durable restore decision cannot switch to Forward"
                );
                return restore::recover(
                    store,
                    adapter,
                    id,
                    disposition == RecoveryDisposition::Restore,
                );
            }
            if matches!(
                inspection.journal.status,
                NativeJournalStatus::Retiring | NativeJournalStatus::Complete
            ) {
                return retire(store, adapter, id);
            }
            if !inspection.markers_complete {
                ensure!(
                    disposition == RecoveryDisposition::Forward,
                    "partial preparation requires explicit Forward or Restore"
                );
                restore::repair_markers(store, &inspection)?;
                inspection = store.inspect(id)?;
            }
            if inspection.journal.execution.is_none() {
                ensure!(
                    disposition == RecoveryDisposition::Forward,
                    "prepared native operation requires explicit Forward or Restore"
                );
                store.prepare_execution(id, adapter)?;
            }
            initialize_runtime(store, id)?;
            drive(
                store,
                adapter,
                id,
                disposition == RecoveryDisposition::Forward,
            )
        })();
        Ok(result.unwrap_or_else(|error| required(Some(id), error)))
    }
}
fn required(id: Option<&str>, error: anyhow::Error) -> NativeDeliveryOutcome {
    NativeDeliveryOutcome::RecoveryRequired {
        operation_id: id.map(str::to_owned),
        reason: format!("{error:#}"),
    }
}
fn initialize_runtime(store: &NativeOperationStore<'_, '_>, id: &str) -> Result<()> {
    let inspection = store.inspect(id)?;
    if inspection.journal.runtime.is_some() {
        return Ok(());
    }
    let mut journal = inspection.journal.clone();
    journal.runtime = Some(NativeRuntime {
        calls: vec![],
        final_observations: vec![],
        state_action: None,
        state_phase: NativeStepPhase::Queued,
        restore: None,
    });
    persist(store, &inspection.journal, journal)
}
fn persist(
    store: &NativeOperationStore<'_, '_>,
    before: &NativeOperationJournal,
    mut after: NativeOperationJournal,
) -> Result<()> {
    ensure!(
        before.schema == after.schema
            && after.schema == 1
            && before.execution == after.execution
            && before.operation_id == after.operation_id
            && before.intent_sha256 == after.intent_sha256
            && before.generation == after.generation,
        "coordinator cannot replace immutable execution recipe"
    );
    let id = &before.operation_id;
    let bytes = store
        .state
        .evidence()
        .read_record(&journal_name(id))?
        .context("native journal missing")?;
    ensure!(
        json::<NativeOperationJournal>(&bytes)? == *before,
        "native journal changed before coordinator checkpoint"
    );
    after.generation = before
        .generation
        .checked_add(1)
        .context("native journal generation overflow")?;
    let intent_bytes = store
        .state
        .evidence()
        .read_record(&intent_name(id))?
        .context("native intent missing before checkpoint")?;
    ensure!(
        intent_digest(&intent_bytes) == before.intent_sha256,
        "native intent changed before checkpoint"
    );
    let intent: NativeOperationIntent = yaml(&intent_bytes)?;
    let execution = after
        .execution
        .as_ref()
        .context("native checkpoint has no recipe")?;
    super::checkpoints::validate(&after, execution)?;
    validate_runtime(store, &intent, &after)?;
    store.roots.verify_bindings()?;
    store.state.evidence().write_record(
        &journal_name(id),
        Some(&bytes),
        Some(&serde_json::to_vec(&after)?),
    )?;
    store.inspect(id)?;
    Ok(())
}

fn drive(
    store: &NativeOperationStore<'_, '_>,
    adapter: &mut dyn NativeDeliveryAdapter,
    id: &str,
    allow_native: bool,
) -> Result<NativeDeliveryOutcome> {
    let inspection = store.inspect(id)?;
    let execution = inspection
        .journal
        .execution
        .as_ref()
        .context("native execution missing")?;
    validate_profiles(adapter, execution)?;
    preflight(store, &inspection)?;
    // Observe every native effect before advancing any recorded sequence.
    for (index, step) in execution.steps.iter().enumerate() {
        if let NativeExecutionStep::Native(effect) = step {
            let observation = adapter.observe(effect)?;
            if effect.request.profile.action == NativeActionKind::Command
                && inspection.journal.checkpoints[index].phase == NativeStepPhase::Queued
            {
                ensure!(
                    observation.effect == NativeEffectRelation::Before,
                    "unstarted native command cannot adopt a manual binding"
                );
            }
            super::checkpoints::validate_native(effect, &observation)?;
            ensure!(
                super::checkpoints::known_binding(effect, &observation.binding),
                "native effective binding is unknown or foreign"
            );
            ensure!(
                !matches!(
                    observation.effect,
                    NativeEffectRelation::Unknown | NativeEffectRelation::Diverged
                ),
                "native recovery cannot establish an owned effect state"
            );
            ensure!(
                allow_native || observation.effect == NativeEffectRelation::After,
                "observed native before-state requires explicit Forward before advancing files"
            );
        }
    }
    for index in 0..execution.steps.len() {
        let inspection = store.inspect(id)?;
        let step = inspection.journal.execution.as_ref().unwrap().steps[index].clone();
        let phase = inspection.journal.checkpoints[index].phase;
        if phase == NativeStepPhase::ObservedAfter {
            continue;
        }
        // Revalidate all file/configuration layers at each activation boundary.
        preflight(store, &inspection)?;
        if phase == NativeStepPhase::Queued {
            store.begin_execution_step(id, index)?;
        }
        match &step {
            NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file) => {
                let (scope, guard) = store.roots.scope_and_guard(&file.location.root)?;
                let actual = io::observe(scope, file)?;
                if actual == file.after {
                    io::sync(scope, guard, file)?;
                } else {
                    ensure!(
                        actual == file.before,
                        "native file diverged from both recorded states"
                    );
                    if matches!(step, NativeExecutionStep::Exposure(_)) {
                        guard_exposure(store, adapter, &inspection)?;
                    }
                    io::apply(
                        scope,
                        guard,
                        file,
                        matches!(step, NativeExecutionStep::Stage(_)),
                        index,
                    )?;
                    checkpoint("after_file_effect", index, id)?;
                }
            }
            NativeExecutionStep::Native(effect) => {
                let profile =
                    adapter.profile(&effect.request.change.key, &effect.request.change.scope)?;
                ensure!(
                    profile == effect.request.profile,
                    "native adapter profile changed"
                );
                let observation = adapter.observe(effect)?;
                super::checkpoints::validate_native(effect, &observation)?;
                ensure!(
                    super::checkpoints::known_binding(effect, &observation.binding),
                    "native effective binding is unknown or foreign"
                );
                match observation.effect {
                    NativeEffectRelation::After => {}
                    NativeEffectRelation::Before => {
                        ensure!(
                            allow_native,
                            "native before-state requires explicit Forward or Restore"
                        );
                        ensure!(
                            profile.action == NativeActionKind::Command,
                            "file-only native effect is not observed after exposure"
                        );
                        let capture = adapter
                            .capture(&effect.request.change.key, &effect.request.change.scope)?;
                        let expected = &inspection
                            .intent
                            .capture
                            .native_observations
                            .iter()
                            .find(|row| row.key == capture.key)
                            .context("native capture target differs")?;
                        validate_binding_attribution(&capture, expected)?;
                        require_retention(store, &inspection.intent, &capture)?;
                        begin_call(
                            store,
                            id,
                            index,
                            NativeCallDirection::Forward,
                            observation.clone(),
                            capture,
                        )?;
                        let result = adapter.apply(effect);
                        checkpoint("after_native_effect", index, id)?;
                        let (outcome, detail) = match result {
                            Ok(value) => (value, None),
                            Err(error) => {
                                (NativeBindingOutcome::Unknown, Some(format!("{error:#}")))
                            }
                        };
                        finish_call(store, id, index, outcome, detail.clone())?;
                        store.record_native_attempt(id, index, outcome, detail)?;
                        ensure!(
                            outcome == NativeBindingOutcome::Confirmed,
                            "native outcome requires observation-first recovery"
                        );
                    }
                    NativeEffectRelation::Unknown | NativeEffectRelation::Diverged => {
                        bail!("native effect is unknown or manually diverged")
                    }
                }
            }
        }
        ensure!(
            store.observe_pending_step(id, index, adapter)?,
            "native effect has not been independently observed after"
        );
    }
    commit_state(store, adapter, id)?;
    retire(store, adapter, id)
}
fn begin_call(
    store: &NativeOperationStore<'_, '_>,
    id: &str,
    index: usize,
    direction: NativeCallDirection,
    precondition: NativeEffectObservation,
    retention_capture: NativeSkillObservation,
) -> Result<()> {
    let inspection = store.inspect(id)?;
    let mut journal = inspection.journal.clone();
    let inverse = direction == NativeCallDirection::Restore;
    journal
        .runtime
        .as_mut()
        .unwrap()
        .calls
        .push(NativeCallRecord {
            direction,
            index,
            precondition,
            retention_capture,
            outcome: None,
            error: None,
        });
    persist(store, &inspection.journal, journal)?;
    checkpoint(
        if inverse {
            "after_inverse_call_pending"
        } else {
            "after_call_pending"
        },
        index,
        id,
    )
}
fn finish_call(
    store: &NativeOperationStore<'_, '_>,
    id: &str,
    index: usize,
    outcome: NativeBindingOutcome,
    error: Option<String>,
) -> Result<()> {
    let inspection = store.inspect(id)?;
    let mut journal = inspection.journal.clone();
    let call = journal
        .runtime
        .as_mut()
        .unwrap()
        .calls
        .last_mut()
        .context("native call intent missing")?;
    ensure!(
        call.index == index && call.outcome.is_none(),
        "native call result differs from pending call"
    );
    call.outcome = Some(outcome);
    call.error = error;
    persist(store, &inspection.journal, journal)
}
fn validate_profiles(
    adapter: &mut dyn NativeDeliveryAdapter,
    execution: &NativeExecutionPlan,
) -> Result<()> {
    for binding in &execution.profiles {
        ensure!(
            adapter.profile(&binding.key, &binding.profile.scope)? == binding.profile,
            "native adapter profile changed"
        );
    }
    Ok(())
}
fn validate_binding_attribution(
    actual: &NativeSkillObservation,
    expected: &NativeSkillObservation,
) -> Result<()> {
    ensure!(
        actual.key == expected.key
            && actual.scope == expected.scope
            && actual.client_version == expected.client_version
            && actual.platform == expected.platform
            && !actual.observation_id.trim().is_empty(),
        "fresh native capture attribution differs"
    );
    Ok(())
}
fn validate_capture_state(
    intent: &NativeOperationIntent,
    actual: &NativeSkillObservation,
) -> Result<()> {
    let expected = intent
        .capture
        .native_observations
        .iter()
        .find(|row| row.key == actual.key)
        .context("native capture target differs from intent")?;
    validate_binding_attribution(actual, expected)?;
    if let Some(current) = selection(actual)? {
        ensure!(
            selection(expected)?.as_ref() == Some(&current)
                || intended(intent, expected)?.as_ref() == Some(&current),
            "native effective binding is outside the recorded transition"
        );
    }
    Ok(())
}
fn require_retention(
    store: &NativeOperationStore<'_, '_>,
    intent: &NativeOperationIntent,
    observation: &NativeSkillObservation,
) -> Result<()> {
    validate_capture_state(intent, observation)?;
    let prior = store.prior(&intent.prior_state)?;
    let relevant = intent.plan.native_changes.iter().any(|change| {
        change.key == observation.key
            && (change.before.is_some()
                || prior
                    .skills
                    .revisions
                    .iter()
                    .chain(prior.skills.staged.iter().map(|staged| &staged.revision))
                    .any(|revision| {
                        revision.installation.owner.installation_id == change.key.installation_id
                            && revision.selected.surface == change.key.surface
                    }))
    });
    if relevant {
        ensure!(
            matches!(&observation.retention, NativeResourceRetention::CachePreserved{evidence_id} | NativeResourceRetention::SessionsCleared{evidence_id} if !evidence_id.trim().is_empty()),
            "native cache retention or session clearance is not verified"
        );
    }
    Ok(())
}
fn preflight(
    store: &NativeOperationStore<'_, '_>,
    inspection: &NativeOperationInspection,
) -> Result<()> {
    ensure!(
        inspection.markers_complete,
        "native execution requires every root marker"
    );
    let execution = inspection
        .journal
        .execution
        .as_ref()
        .context("native execution missing")?;
    let prior = store.prior(&inspection.intent.prior_state)?;
    state::graph(&prior, &inspection.intent.plan)?;
    for root in store.roots.bindings() {
        let (scope, _) = store.roots.scope_and_guard(&root.root)?;
        ensure!(
            !scope.has_pending_operation()?,
            "recover existing file operation before native effects"
        );
    }
    for (step, progress) in execution.steps.iter().zip(&inspection.journal.checkpoints) {
        if let NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file) = step {
            let (scope, guard) = store.roots.scope_and_guard(&file.location.root)?;
            io::preflight(scope, guard, file)?;
            let actual = io::observe(scope, file)?;
            ensure!(
                match progress.phase {
                    NativeStepPhase::Queued => actual == file.before,
                    NativeStepPhase::Pending => actual == file.before || actual == file.after,
                    NativeStepPhase::ObservedAfter => actual == file.after,
                },
                "native file changed outside its recorded checkpoint"
            );
        }
    }
    for config in &inspection.intent.capture.configurations {
        for layer in config
            .before
            .lower_layers
            .iter()
            .chain(&config.before.higher_layers)
        {
            let (scope, _) = store.roots.scope_and_guard(&layer.file.location.root)?;
            let snapshot =
                scope.observe_destinations(std::slice::from_ref(&layer.file.location))?;
            ensure!(
                snapshot.observations()[0] == layer.file,
                "native configuration layer changed during operation"
            );
            snapshot.verify_unchanged()?;
        }
    }
    let runtime = inspection
        .journal
        .runtime
        .as_ref()
        .context("native runtime missing")?;
    let state = store.state.snapshot()?;
    let value = &state.observations()[0].value;
    ensure!(
        value == &inspection.intent.prior_state
            || runtime
                .state_action
                .as_ref()
                .is_some_and(|action| value == &action.after),
        "authoritative native state changed outside operation"
    );
    let action = runtime.state_action.clone().unwrap_or(PlannedFile {
        location: FileLocation {
            root: inspection.intent.context.scope.clone(),
            path: ".upskill-lock.json".into(),
        },
        before: inspection.intent.prior_state.clone(),
        after: inspection.intent.prior_state.clone(),
    });
    let (scope, guard) = store.roots.scope_and_guard(&action.location.root)?;
    io::preflight(scope, guard, &action)?;
    state.verify_unchanged()?;
    store.roots.verify_bindings()
}
fn guard_exposure(
    store: &NativeOperationStore<'_, '_>,
    adapter: &mut dyn NativeDeliveryAdapter,
    inspection: &NativeOperationInspection,
) -> Result<()> {
    validate_profiles(
        adapter,
        inspection
            .journal
            .execution
            .as_ref()
            .context("native execution missing")?,
    )?;
    for change in &inspection.intent.plan.native_changes {
        let actual = adapter.capture(&change.key, &change.scope)?;
        let expected = inspection
            .intent
            .capture
            .native_observations
            .iter()
            .find(|row| row.key == change.key)
            .context("native change observation missing")?;
        validate_binding_attribution(&actual, expected)?;
        require_retention(store, &inspection.intent, &actual)?;
    }
    Ok(())
}
fn verify_layers(
    store: &NativeOperationStore<'_, '_>,
    intent: &NativeOperationIntent,
) -> Result<()> {
    for config in &intent.capture.configurations {
        for layer in config
            .before
            .lower_layers
            .iter()
            .chain(&config.before.higher_layers)
        {
            let (scope, _) = store.roots.scope_and_guard(&layer.file.location.root)?;
            let snapshot =
                scope.observe_destinations(std::slice::from_ref(&layer.file.location))?;
            ensure!(
                snapshot.observations()[0] == layer.file,
                "native configuration layer changed during retirement"
            );
            snapshot.verify_unchanged()?;
        }
    }
    Ok(())
}
fn final_observations(
    store: &NativeOperationStore<'_, '_>,
    adapter: &mut dyn NativeDeliveryAdapter,
    inspection: &NativeOperationInspection,
) -> Result<Vec<NativeSkillObservation>> {
    validate_profiles(
        adapter,
        inspection
            .journal
            .execution
            .as_ref()
            .context("native execution missing")?,
    )?;
    let mut observations = Vec::new();
    for expected in &inspection.intent.capture.native_observations {
        let actual = adapter.capture(&expected.key, &expected.scope)?;
        validate_binding_attribution(&actual, expected)?;
        ensure!(
            selection(&actual)? == intended(&inspection.intent, expected)?,
            "final effective native binding differs from intent"
        );
        require_retention(store, &inspection.intent, &actual)?;
        observations.push(actual);
    }
    Ok(observations)
}
fn selection(observation: &NativeSkillObservation) -> Result<Option<NativeSkillSelection>> {
    match &observation.state {
        NativeObservationState::Absent => Ok(None),
        NativeObservationState::Present(value) => Ok(Some(value.clone())),
        NativeObservationState::Unknown => bail!("native effective binding is unknown"),
    }
}
fn intended(
    intent: &NativeOperationIntent,
    before: &NativeSkillObservation,
) -> Result<Option<NativeSkillSelection>> {
    match intent
        .plan
        .native_changes
        .iter()
        .find(|change| change.key == before.key)
    {
        Some(change) => Ok(change.after.clone()),
        None => selection(before),
    }
}
fn commit_state(
    store: &NativeOperationStore<'_, '_>,
    adapter: &mut dyn NativeDeliveryAdapter,
    id: &str,
) -> Result<()> {
    let mut inspection = store.inspect(id)?;
    preflight(store, &inspection)?;
    ensure!(
        inspection
            .journal
            .checkpoints
            .iter()
            .all(|row| row.phase == NativeStepPhase::ObservedAfter),
        "native effects are not observed after"
    );
    let observations = final_observations(store, adapter, &inspection)?;
    if inspection
        .journal
        .runtime
        .as_ref()
        .unwrap()
        .state_action
        .is_none()
    {
        let candidate = state::completed(store, &inspection.intent, &observations)?;
        let action = store.state.prepare_state_file(&candidate)?;
        ensure!(
            action.before == inspection.intent.prior_state,
            "native state changed before candidate publication"
        );
        let mut journal = inspection.journal.clone();
        let runtime = journal.runtime.as_mut().unwrap();
        runtime.final_observations = observations;
        runtime.state_action = Some(action);
        runtime.state_phase = NativeStepPhase::Pending;
        persist(store, &inspection.journal, journal)?;
        checkpoint("after_state_pending", 0, id)?;
        inspection = store.inspect(id)?;
    }
    let action = inspection
        .journal
        .runtime
        .as_ref()
        .unwrap()
        .state_action
        .as_ref()
        .unwrap();
    let (scope, guard) = store.roots.scope_and_guard(&action.location.root)?;
    let actual = io::observe(scope, action)?;
    if actual == action.after {
        io::sync(scope, guard, action)?;
    } else {
        ensure!(
            actual == action.before,
            "native state diverged before publication"
        );
        io::apply(scope, guard, action, false, 0)?;
        checkpoint("after_state_effect", 0, id)?;
    }
    preflight(store, &store.inspect(id)?)?;
    final_observations(store, adapter, &inspection)?;
    let loaded = store
        .state
        .load()?
        .context("native state missing after publication")?;
    ensure!(
        loaded
            == state::completed(
                store,
                &inspection.intent,
                &inspection
                    .journal
                    .runtime
                    .as_ref()
                    .unwrap()
                    .final_observations
            )?,
        "published native state differs from completed observations"
    );
    let current = store.inspect(id)?;
    let mut journal = current.journal.clone();
    journal.runtime.as_mut().unwrap().state_phase = NativeStepPhase::ObservedAfter;
    journal.status = NativeJournalStatus::Retiring;
    persist(store, &current.journal, journal)?;
    checkpoint("after_terminal_pending", 0, id)
}
fn retire(
    store: &NativeOperationStore<'_, '_>,
    adapter: &mut dyn NativeDeliveryAdapter,
    id: &str,
) -> Result<NativeDeliveryOutcome> {
    let inspection = store.inspect(id)?;
    ensure!(
        matches!(
            inspection.journal.status,
            NativeJournalStatus::Retiring | NativeJournalStatus::Complete
        ),
        "native operation is not ready for retirement"
    );
    final_observations(store, adapter, &inspection)?;
    verify_layers(store, &inspection.intent)?;
    let action = inspection
        .journal
        .runtime
        .as_ref()
        .unwrap()
        .state_action
        .as_ref()
        .context("native terminal state missing")?;
    let (scope, guard) = store.roots.scope_and_guard(&action.location.root)?;
    io::sync(scope, guard, action)?;
    for step in &inspection.journal.execution.as_ref().unwrap().steps {
        if let NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file) = step {
            let (scope, guard) = store.roots.scope_and_guard(&file.location.root)?;
            io::sync(scope, guard, file)?;
        }
    }
    for (index, marker) in inspection.intent.marker_namespaces.iter().enumerate() {
        let storage = store
            .marker_store(&marker.root.root, false)?
            .context("native marker storage missing")?;
        if let Some(bytes) = storage.read_record("pending.json")? {
            ensure!(
                json::<PendingMarker>(&bytes)?
                    == pending(&inspection.intent, &inspection.journal, marker),
                "native marker changed during retirement"
            );
            storage.write_record("pending.json", Some(&bytes), None)?;
            checkpoint("after_retired_marker", index, id)?;
        }
    }
    let current = store.inspect(id)?;
    if current.journal.status != NativeJournalStatus::Complete {
        let mut journal = current.journal.clone();
        journal.status = NativeJournalStatus::Complete;
        persist(store, &current.journal, journal)?;
    }
    for marker in &inspection.intent.marker_namespaces {
        ensure!(
            store
                .marker_store(&marker.root.root, false)?
                .context("native marker storage missing")?
                .read_record("pending.json")?
                .is_none(),
            "native markers were not retired"
        );
    }
    Ok(NativeDeliveryOutcome::Complete {
        operation_id: id.into(),
    })
}

pub(super) fn validate_runtime(
    store: &NativeOperationStore<'_, '_>,
    intent: &NativeOperationIntent,
    journal: &NativeOperationJournal,
) -> Result<()> {
    let Some(runtime) = &journal.runtime else {
        ensure!(
            journal.status == NativeJournalStatus::Executing,
            "terminal native journal has no runtime"
        );
        return Ok(());
    };
    let execution = journal
        .execution
        .as_ref()
        .context("native runtime has no recipe")?;
    if matches!(
        journal.status,
        NativeJournalStatus::Restoring
            | NativeJournalStatus::RetiringRestore
            | NativeJournalStatus::Restored
    ) {
        restore::validate(store, intent, journal)?;
    } else {
        ensure!(
            runtime.restore.is_none(),
            "forward journal contains restore checkpoints"
        );
    }
    for call in &runtime.calls {
        let Some(NativeExecutionStep::Native(effect)) = execution.steps.get(call.index) else {
            bail!("native call is not bound to a native effect")
        };
        ensure!(
            effect.request.profile.action == NativeActionKind::Command,
            "file-only profile has a native command call"
        );
        super::checkpoints::validate_native(effect, &call.precondition)?;
        ensure!(
            super::checkpoints::known_binding(effect, &call.precondition.binding),
            "native call has an unknown or foreign precondition binding"
        );
        ensure!(
            call.precondition.effect
                == if call.direction == NativeCallDirection::Forward {
                    NativeEffectRelation::Before
                } else {
                    NativeEffectRelation::After
                },
            "native call precondition relation differs from its direction"
        );
        require_retention(store, intent, &call.retention_capture)?;
        if call.direction == NativeCallDirection::Restore {
            ensure!(
                matches!(&call.retention_capture.retention, NativeResourceRetention::CachePreserved{evidence_id} | NativeResourceRetention::SessionsCleared{evidence_id} if !evidence_id.trim().is_empty()),
                "native inverse call lacks cache/session proof"
            );
        }
        ensure!(
            matches!(
                execution.steps.get(call.index),
                Some(NativeExecutionStep::Native(_))
            ) && journal.checkpoints[call.index].phase != NativeStepPhase::Queued,
            "native call is not bound to a pending effect"
        );
        ensure!(
            call.error.is_none() || call.outcome == Some(NativeBindingOutcome::Unknown),
            "native error requires an unknown call outcome"
        );
    }
    match &runtime.state_action {
        None => ensure!(
            runtime.state_phase == NativeStepPhase::Queued
                && runtime.final_observations.is_empty()
                && matches!(
                    journal.status,
                    NativeJournalStatus::Executing
                        | NativeJournalStatus::Restoring
                        | NativeJournalStatus::RetiringRestore
                        | NativeJournalStatus::Restored
                ),
            "native state checkpoint has no action"
        ),
        Some(action) => {
            ensure!(
                action.location.root == intent.context.scope
                    && action.location.path == ".upskill-lock.json"
                    && action.before == intent.prior_state
                    && runtime.state_phase != NativeStepPhase::Queued
                    && journal
                        .checkpoints
                        .iter()
                        .all(|row| row.phase == NativeStepPhase::ObservedAfter),
                "native state action differs from completed operation"
            );
            let after = action
                .after
                .as_ref()
                .context("completed native state must remain present")?;
            let actual = store.state.parse_captured_state(&after.bytes)?;
            ensure!(
                actual == state::completed(store, intent, &runtime.final_observations)?,
                "native state action differs from confirmed observations"
            );
            if matches!(
                journal.status,
                NativeJournalStatus::Retiring | NativeJournalStatus::Complete
            ) {
                ensure!(
                    runtime.state_phase == NativeStepPhase::ObservedAfter,
                    "native retirement precedes observed state"
                );
            }
        }
    }
    Ok(())
}
