//! Explicit owned inverse, retaining every staged immutable resource revision.
use super::*;
use crate::files::plan::ObservedFile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRestoreRuntime {
    pub checkpoints: Vec<NativeStepCheckpoint>,
    pub final_observations: Vec<NativeSkillObservation>,
    pub retained_files: Vec<ObservedFile>,
    pub state_action: Option<PlannedFile>,
    pub state_phase: NativeStepPhase,
}
pub(super) fn recover(
    store: &NativeOperationStore<'_, '_>,
    adapter: &mut dyn NativeDeliveryAdapter,
    id: &str,
    allow_inverse: bool,
) -> Result<NativeDeliveryOutcome> {
    let mut inspection = store.inspect(id)?;
    ensure!(
        inspection.journal.status != NativeJournalStatus::Complete,
        "completed operation requires a new lifecycle operation"
    );
    if matches!(
        inspection.journal.status,
        NativeJournalStatus::RetiringRestore | NativeJournalStatus::Restored
    ) {
        return retire_restore(store, adapter, id);
    }
    if inspection.journal.status != NativeJournalStatus::Restoring {
        ensure!(allow_inverse, "restore requires an explicit disposition");
        repair_markers(store, &inspection)?;
        if inspection.journal.execution.is_none() {
            store.prepare_execution(id, adapter)?;
        }
        initialize_runtime(store, id)?;
        inspection = store.inspect(id)?;
        let mut journal = inspection.journal.clone();
        journal.status = NativeJournalStatus::Restoring;
        journal.runtime.as_mut().unwrap().restore = Some(NativeRestoreRuntime {
            checkpoints: journal
                .checkpoints
                .iter()
                .map(|_| NativeStepCheckpoint {
                    phase: NativeStepPhase::Queued,
                    attempts: vec![],
                    observations: vec![],
                })
                .collect(),
            final_observations: vec![],
            retained_files: vec![],
            state_action: None,
            state_phase: NativeStepPhase::Queued,
        });
        persist(store, &inspection.journal, journal)?;
        checkpoint("after_restore_pending", 0, id)?;
    }
    drive_restore(store, adapter, id, allow_inverse)
}
pub(super) fn repair_markers(
    store: &NativeOperationStore<'_, '_>,
    inspection: &NativeOperationInspection,
) -> Result<()> {
    for marker in &inspection.intent.marker_namespaces {
        let storage = store
            .marker_store(&marker.root.root, false)?
            .context("native marker directory was lost")?;
        ensure!(
            storage.namespace() == marker.namespace,
            "native marker namespace changed"
        );
        if storage.read_record("pending.json")?.is_none() {
            storage.write_record(
                "pending.json",
                None,
                Some(&serde_json::to_vec(&pending(
                    &inspection.intent,
                    &inspection.journal,
                    marker,
                ))?),
            )?;
        }
    }
    ensure!(
        store
            .inspect(&inspection.intent.operation_id)?
            .markers_complete,
        "native marker repair is incomplete"
    );
    Ok(())
}
fn drive_restore(
    store: &NativeOperationStore<'_, '_>,
    adapter: &mut dyn NativeDeliveryAdapter,
    id: &str,
    allow_inverse: bool,
) -> Result<NativeDeliveryOutcome> {
    let inspection = store.inspect(id)?;
    let execution = inspection.journal.execution.as_ref().unwrap();
    validate_profiles(adapter, execution)?;
    preflight_restore(store, &inspection)?;
    // Refuse all inverse effects if any native target is unknown/diverged.
    for (index, step) in execution.steps.iter().enumerate() {
        if let NativeExecutionStep::Native(effect) = step {
            let observation = adapter.observe(effect)?;
            if effect.request.profile.action == NativeActionKind::Command
                && inspection.journal.checkpoints[index].phase == NativeStepPhase::Queued
            {
                ensure!(
                    observation.effect == NativeEffectRelation::Before,
                    "native restore cannot adopt an unstarted native command"
                );
            }
            super::super::checkpoints::validate_native(effect, &observation)?;
            ensure!(
                super::super::checkpoints::known_binding(effect, &observation.binding),
                "native inverse effective binding is unknown or foreign"
            );
            ensure!(
                !matches!(
                    observation.effect,
                    NativeEffectRelation::Unknown | NativeEffectRelation::Diverged
                ),
                "native inverse prestate is unknown or diverged"
            );
        }
    }
    for index in (0..execution.steps.len()).rev() {
        let inspection = store.inspect(id)?;
        preflight_restore(store, &inspection)?;
        let step = &inspection.journal.execution.as_ref().unwrap().steps[index];
        let phase = restoration(&inspection.journal)?.checkpoints[index].phase;
        if phase == NativeStepPhase::ObservedAfter {
            continue;
        }
        match step {
            NativeExecutionStep::Stage(file) => {
                let (scope, _) = store.roots.scope_and_guard(&file.location.root)?;
                let value = io::observe(scope, file)?;
                acknowledge(
                    store,
                    id,
                    index,
                    NativeStepObservation::File(ObservedFile {
                        location: file.location.clone(),
                        value,
                    }),
                )?;
            }
            NativeExecutionStep::Exposure(file) => {
                let (scope, guard) = store.roots.scope_and_guard(&file.location.root)?;
                let value = io::observe(scope, file)?;
                if value != file.before {
                    ensure!(
                        value == file.after,
                        "native exposure inverse would overwrite manual changes"
                    );
                    validate_profiles(adapter, inspection.journal.execution.as_ref().unwrap())?;
                    for change in &inspection.intent.plan.native_changes {
                        let actual = adapter.capture(&change.key, &change.scope)?;
                        let expected = inspection
                            .intent
                            .capture
                            .native_observations
                            .iter()
                            .find(|row| row.key == change.key)
                            .context("inverse change observation missing")?;
                        validate_binding_attribution(&actual, expected)?;
                        require_inverse_retention(&inspection, &actual)?;
                    }
                    mark_pending(store, id, index)?;
                    let inverse = PlannedFile {
                        location: file.location.clone(),
                        before: file.after.clone(),
                        after: file.before.clone(),
                    };
                    io::apply(scope, guard, &inverse, false, index)?;
                    checkpoint("after_inverse_file", index, id)?;
                }
                let value = io::observe(scope, file)?;
                acknowledge(
                    store,
                    id,
                    index,
                    NativeStepObservation::File(ObservedFile {
                        location: file.location.clone(),
                        value,
                    }),
                )?;
            }
            NativeExecutionStep::Native(effect) => {
                let observation = adapter.observe(effect)?;
                super::super::checkpoints::validate_native(effect, &observation)?;
                ensure!(
                    super::super::checkpoints::known_binding(effect, &observation.binding),
                    "native inverse effective binding is unknown or foreign"
                );
                match observation.effect {
                    NativeEffectRelation::Before => acknowledge(
                        store,
                        id,
                        index,
                        NativeStepObservation::Native(Box::new(observation)),
                    )?,
                    NativeEffectRelation::After => {
                        mark_pending(store, id, index)?;
                        if effect.request.profile.action == NativeActionKind::FileOnly {
                            continue;
                        }
                        ensure!(
                            allow_inverse,
                            "native inverse requires explicit Restore after observed forward effect"
                        );
                        let capture = adapter
                            .capture(&effect.request.change.key, &effect.request.change.scope)?;
                        let expected = inspection
                            .intent
                            .capture
                            .native_observations
                            .iter()
                            .find(|row| row.key == capture.key)
                            .context("inverse capture target differs")?;
                        validate_binding_attribution(&capture, expected)?;
                        require_inverse_retention(&inspection, &capture)?;
                        begin_call(
                            store,
                            id,
                            index,
                            NativeCallDirection::Restore,
                            observation.clone(),
                            capture,
                        )?;
                        let result = adapter.inverse(effect);
                        checkpoint("after_inverse_native", index, id)?;
                        let (outcome, detail) = match result {
                            Ok(value) => (value, None),
                            Err(error) => {
                                (NativeBindingOutcome::Unknown, Some(format!("{error:#}")))
                            }
                        };
                        finish_call(store, id, index, outcome, detail.clone())?;
                        record_attempt(store, id, index, outcome, detail)?;
                        ensure!(
                            outcome == NativeBindingOutcome::Confirmed,
                            "native inverse outcome requires observation-first recovery"
                        );
                        let observation = adapter.observe(effect)?;
                        super::super::checkpoints::validate_native(effect, &observation)?;
                        ensure!(
                            super::super::checkpoints::known_binding(effect, &observation.binding),
                            "native inverse effective binding is unknown or foreign"
                        );
                        ensure!(
                            observation.effect == NativeEffectRelation::Before,
                            "native inverse has not been independently observed"
                        );
                        acknowledge(
                            store,
                            id,
                            index,
                            NativeStepObservation::Native(Box::new(observation)),
                        )?;
                    }
                    NativeEffectRelation::Unknown | NativeEffectRelation::Diverged => {
                        bail!("native inverse is unknown or manually diverged")
                    }
                }
            }
        }
    }
    // File-only binding changes can become absent only after an earlier exposure
    // has been restored. Never invent an inverse command or an early observation.
    for (index, step) in execution.steps.iter().enumerate() {
        if let NativeExecutionStep::Native(effect) = step {
            if restoration(&store.inspect(id)?.journal)?.checkpoints[index].phase
                != NativeStepPhase::ObservedAfter
            {
                let observation = adapter.observe(effect)?;
                super::super::checkpoints::validate_native(effect, &observation)?;
                ensure!(
                    super::super::checkpoints::known_binding(effect, &observation.binding),
                    "native inverse effective binding is unknown or foreign"
                );
                ensure!(
                    observation.effect == NativeEffectRelation::Before,
                    "file-only inverse is not observed after exposure restoration"
                );
                acknowledge(
                    store,
                    id,
                    index,
                    NativeStepObservation::Native(Box::new(observation)),
                )?;
            }
        }
    }
    commit_restore(store, adapter, id)?;
    retire_restore(store, adapter, id)
}
fn restoration(journal: &NativeOperationJournal) -> Result<&NativeRestoreRuntime> {
    journal
        .runtime
        .as_ref()
        .and_then(|runtime| runtime.restore.as_ref())
        .context("native restore runtime missing")
}
fn mark_pending(store: &NativeOperationStore<'_, '_>, id: &str, index: usize) -> Result<()> {
    let inspection = store.inspect(id)?;
    let mut journal = inspection.journal.clone();
    let row = &mut journal
        .runtime
        .as_mut()
        .unwrap()
        .restore
        .as_mut()
        .unwrap()
        .checkpoints[index];
    if row.phase == NativeStepPhase::Queued {
        row.phase = NativeStepPhase::Pending;
        persist(store, &inspection.journal, journal)?;
        checkpoint("after_inverse_pending", index, id)?;
    }
    Ok(())
}
fn record_attempt(
    store: &NativeOperationStore<'_, '_>,
    id: &str,
    index: usize,
    outcome: NativeBindingOutcome,
    detail: Option<String>,
) -> Result<()> {
    let inspection = store.inspect(id)?;
    let mut journal = inspection.journal.clone();
    journal
        .runtime
        .as_mut()
        .unwrap()
        .restore
        .as_mut()
        .unwrap()
        .checkpoints[index]
        .attempts
        .push(NativeAttempt { outcome, detail });
    persist(store, &inspection.journal, journal)
}
fn acknowledge(
    store: &NativeOperationStore<'_, '_>,
    id: &str,
    index: usize,
    observation: NativeStepObservation,
) -> Result<()> {
    let inspection = store.inspect(id)?;
    let mut journal = inspection.journal.clone();
    let row = &mut journal
        .runtime
        .as_mut()
        .unwrap()
        .restore
        .as_mut()
        .unwrap()
        .checkpoints[index];
    row.observations.push(observation);
    row.phase = NativeStepPhase::ObservedAfter;
    persist(store, &inspection.journal, journal)?;
    checkpoint("after_inverse_observation", index, id)
}
fn preflight_restore(
    store: &NativeOperationStore<'_, '_>,
    inspection: &NativeOperationInspection,
) -> Result<()> {
    ensure!(
        inspection.markers_complete,
        "native restore requires every root marker"
    );
    for root in store.roots.bindings() {
        let (scope, _) = store.roots.scope_and_guard(&root.root)?;
        ensure!(
            !scope.has_pending_operation()?,
            "recover existing file operation before native restore"
        );
    }
    for (index, step) in inspection
        .journal
        .execution
        .as_ref()
        .unwrap()
        .steps
        .iter()
        .enumerate()
    {
        if let NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file) = step {
            let (scope, guard) = store.roots.scope_and_guard(&file.location.root)?;
            io::preflight(scope, guard, file)?;
            let actual = io::observe(scope, file)?;
            ensure!(
                actual == file.before || actual == file.after,
                "native restore would overwrite manual file divergence"
            );
            if inspection.journal.checkpoints[index].phase == NativeStepPhase::Queued {
                ensure!(
                    actual == file.before,
                    "native restore cannot adopt an unstarted output"
                );
            }
            if restoration(&inspection.journal)?.checkpoints[index].phase
                == NativeStepPhase::ObservedAfter
                && matches!(step, NativeExecutionStep::Exposure(_))
            {
                ensure!(
                    actual == file.before,
                    "restored exposure changed outside operation"
                );
            }
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
                "native configuration layer changed during restore"
            );
            snapshot.verify_unchanged()?;
        }
    }
    let snapshot = store.state.snapshot()?;
    let value = &snapshot.observations()[0].value;
    let runtime = inspection.journal.runtime.as_ref().unwrap();
    ensure!(
        value == &inspection.intent.prior_state
            || runtime
                .state_action
                .as_ref()
                .is_some_and(|action| value == &action.after)
            || runtime
                .restore
                .as_ref()
                .and_then(|restore| restore.state_action.as_ref())
                .is_some_and(|action| value == &action.after),
        "native state diverged during restore"
    );
    snapshot.verify_unchanged()?;
    store.roots.verify_bindings()
}
fn require_inverse_retention(
    inspection: &NativeOperationInspection,
    observation: &NativeSkillObservation,
) -> Result<()> {
    validate_capture_state(&inspection.intent, observation)?;
    if inspection.intent.plan.native_changes.iter().any(|change| change.key == observation.key && (change.before.is_some() || change.after.is_some() && inspection.journal.execution.as_ref().unwrap().steps.iter().zip(&inspection.journal.checkpoints).any(|(step, row)| matches!(step, NativeExecutionStep::Exposure(_)) && row.phase != NativeStepPhase::Queued))) || inspection.journal.runtime.as_ref().unwrap().calls.iter().any(|call| matches!(inspection.journal.execution.as_ref().unwrap().steps.get(call.index), Some(NativeExecutionStep::Native(effect)) if effect.request.change.key == observation.key)) {
        ensure!(matches!(&observation.retention, NativeResourceRetention::CachePreserved{evidence_id} | NativeResourceRetention::SessionsCleared{evidence_id} if !evidence_id.trim().is_empty()), "native inverse cache retention or session clearance is not verified");
    }
    Ok(())
}
fn final_before(
    store: &NativeOperationStore<'_, '_>,
    adapter: &mut dyn NativeDeliveryAdapter,
    inspection: &NativeOperationInspection,
) -> Result<Vec<NativeSkillObservation>> {
    validate_profiles(adapter, inspection.journal.execution.as_ref().unwrap())?;
    let mut observations = Vec::new();
    for expected in &inspection.intent.capture.native_observations {
        let actual = adapter.capture(&expected.key, &expected.scope)?;
        validate_binding_attribution(&actual, expected)?;
        ensure!(
            selection(&actual)? == selection(expected)?,
            "native restore has not established prior effective binding"
        );
        require_retention(store, &inspection.intent, &actual)?;
        require_inverse_retention(inspection, &actual)?;
        observations.push(actual);
    }
    Ok(observations)
}
fn captured_files(
    store: &NativeOperationStore<'_, '_>,
    inspection: &NativeOperationInspection,
) -> Result<Vec<ObservedFile>> {
    let mut files = Vec::new();
    for step in &inspection.journal.execution.as_ref().unwrap().steps {
        if let NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file) = step {
            let (scope, _) = store.roots.scope_and_guard(&file.location.root)?;
            files.push(ObservedFile {
                location: file.location.clone(),
                value: io::observe(scope, file)?,
            });
        }
    }
    Ok(files)
}
fn commit_restore(
    store: &NativeOperationStore<'_, '_>,
    adapter: &mut dyn NativeDeliveryAdapter,
    id: &str,
) -> Result<()> {
    let mut inspection = store.inspect(id)?;
    preflight_restore(store, &inspection)?;
    ensure!(
        restoration(&inspection.journal)?
            .checkpoints
            .iter()
            .all(|row| row.phase == NativeStepPhase::ObservedAfter),
        "native inverses are not observed after"
    );
    let observations = final_before(store, adapter, &inspection)?;
    if restoration(&inspection.journal)?.state_action.is_none() {
        let files = captured_files(store, &inspection)?;
        let candidate = state::restored(store, &inspection.intent, &observations, &files)?;
        let prior = store.prior(&inspection.intent.prior_state)?;
        let action = if candidate == prior {
            PlannedFile {
                location: FileLocation {
                    root: inspection.intent.context.scope.clone(),
                    path: ".upskill-lock.json".into(),
                },
                before: store.state.snapshot()?.observations()[0].value.clone(),
                after: inspection.intent.prior_state.clone(),
            }
        } else {
            store.state.prepare_state_file(&candidate)?
        };
        let mut journal = inspection.journal.clone();
        let restore = journal.runtime.as_mut().unwrap().restore.as_mut().unwrap();
        restore.state_action = Some(action);
        restore.final_observations = observations;
        restore.retained_files = files;
        restore.state_phase = NativeStepPhase::Pending;
        persist(store, &inspection.journal, journal)?;
        checkpoint("after_restore_state_pending", 0, id)?;
        inspection = store.inspect(id)?;
    }
    let action = restoration(&inspection.journal)?
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
            "restore state precondition changed"
        );
        io::apply(scope, guard, action, false, 0)?;
        checkpoint("after_restore_state_effect", 0, id)?;
    }
    final_before(store, adapter, &inspection)?;
    preflight_restore(store, &store.inspect(id)?)?;
    let current = store.inspect(id)?;
    let mut journal = current.journal.clone();
    journal
        .runtime
        .as_mut()
        .unwrap()
        .restore
        .as_mut()
        .unwrap()
        .state_phase = NativeStepPhase::ObservedAfter;
    journal.status = NativeJournalStatus::RetiringRestore;
    persist(store, &current.journal, journal)?;
    checkpoint("after_restore_terminal_pending", 0, id)
}
fn retire_restore(
    store: &NativeOperationStore<'_, '_>,
    adapter: &mut dyn NativeDeliveryAdapter,
    id: &str,
) -> Result<NativeDeliveryOutcome> {
    let inspection = store.inspect(id)?;
    final_before(store, adapter, &inspection)?;
    verify_layers(store, &inspection.intent)?;
    let restore = restoration(&inspection.journal)?;
    let action = restore
        .state_action
        .as_ref()
        .context("native restore state missing")?;
    let (scope, guard) = store.roots.scope_and_guard(&action.location.root)?;
    io::sync(scope, guard, action)?;
    for file in &restore.retained_files {
        let (scope, _) = store.roots.scope_and_guard(&file.location.root)?;
        ensure!(
            io::observe(
                scope,
                &PlannedFile {
                    location: file.location.clone(),
                    before: file.value.clone(),
                    after: file.value.clone()
                }
            )? == file.value,
            "retained native files changed during retirement"
        );
    }
    for (index, marker) in inspection.intent.marker_namespaces.iter().enumerate() {
        let storage = store
            .marker_store(&marker.root.root, false)?
            .context("native marker storage missing")?;
        if let Some(bytes) = storage.read_record("pending.json")? {
            ensure!(
                json::<PendingMarker>(&bytes)?
                    == pending(&inspection.intent, &inspection.journal, marker),
                "native marker changed during restore retirement"
            );
            storage.write_record("pending.json", Some(&bytes), None)?;
            checkpoint("after_restore_retired_marker", index, id)?;
        }
    }
    let current = store.inspect(id)?;
    if current.journal.status != NativeJournalStatus::Restored {
        let mut journal = current.journal.clone();
        journal.status = NativeJournalStatus::Restored;
        persist(store, &current.journal, journal)?;
    }
    for marker in &inspection.intent.marker_namespaces {
        ensure!(
            store
                .marker_store(&marker.root.root, false)?
                .context("native marker storage missing")?
                .read_record("pending.json")?
                .is_none(),
            "native restore markers were not retired"
        );
    }
    Ok(NativeDeliveryOutcome::Restored {
        operation_id: id.into(),
    })
}
pub(super) fn validate(
    store: &NativeOperationStore<'_, '_>,
    intent: &NativeOperationIntent,
    journal: &NativeOperationJournal,
) -> Result<()> {
    let restore = restoration(journal)?;
    let execution = journal.execution.as_ref().unwrap();
    ensure!(
        restore.checkpoints.len() == execution.steps.len(),
        "native inverse checkpoint coverage differs"
    );
    for (step, row) in execution.steps.iter().zip(&restore.checkpoints) {
        if row.phase == NativeStepPhase::Queued {
            ensure!(
                row.attempts.is_empty() && row.observations.is_empty(),
                "queued inverse has effect evidence"
            );
        }
        for observation in &row.observations {
            match (step, observation) {
                (
                    NativeExecutionStep::Stage(file) | NativeExecutionStep::Exposure(file),
                    NativeStepObservation::File(actual),
                ) => ensure!(
                    file.location == actual.location,
                    "inverse file observation belongs to another step"
                ),
                (NativeExecutionStep::Native(effect), NativeStepObservation::Native(actual)) => {
                    super::super::checkpoints::validate_native(effect, actual)?
                }
                _ => bail!("inverse observation kind differs"),
            }
        }
        if row.phase == NativeStepPhase::ObservedAfter {
            ensure!(
                row.observations
                    .last()
                    .is_some_and(|observation| match (step, observation) {
                        (NativeExecutionStep::Stage(file), NativeStepObservation::File(actual)) =>
                            actual.value == file.before || actual.value == file.after,
                        (
                            NativeExecutionStep::Exposure(file),
                            NativeStepObservation::File(actual),
                        ) => actual.value == file.before,
                        (NativeExecutionStep::Native(_), NativeStepObservation::Native(actual)) =>
                            actual.effect == NativeEffectRelation::Before,
                        _ => false,
                    }),
                "native inverse acknowledged without observed before-state"
            );
        }
    }
    if let Some(action) = &restore.state_action {
        ensure!(
            action.location.root == intent.context.scope
                && action.location.path == ".upskill-lock.json"
                && restore.state_phase != NativeStepPhase::Queued
                && restore
                    .checkpoints
                    .iter()
                    .all(|row| row.phase == NativeStepPhase::ObservedAfter),
            "restore state action differs from operation"
        );
        let candidate = state::restored(
            store,
            intent,
            &restore.final_observations,
            &restore.retained_files,
        )?;
        match &action.after {
            Some(value) => ensure!(
                store.state.parse_captured_state(&value.bytes)? == candidate,
                "restore state differs from confirmed observations"
            ),
            None => ensure!(
                intent.prior_state.is_none() && candidate == store.prior(&None)?,
                "restore discarded retained staging ownership"
            ),
        }
    } else {
        ensure!(
            restore.state_phase == NativeStepPhase::Queued
                && restore.final_observations.is_empty()
                && restore.retained_files.is_empty()
                && journal.status == NativeJournalStatus::Restoring,
            "restore state checkpoint has no action"
        );
    }
    if matches!(
        journal.status,
        NativeJournalStatus::RetiringRestore | NativeJournalStatus::Restored
    ) {
        ensure!(
            restore.state_phase == NativeStepPhase::ObservedAfter && restore.state_action.is_some(),
            "restore retirement precedes observed state"
        );
    }
    Ok(())
}
