//! Persist the complete recipe before any staged or exposed file/native effect.
use super::{
    NativeJournalStatus, NativeOperationIntent, NativeOperationStore, checkpoint, codec::*,
};
use crate::files::plan::PlannedFile;
use crate::native_delivery::adapter::*;
use crate::native_delivery::state::NativeDeliveryState;
use crate::target::TargetSurface;
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

impl<'locks, 'scope: 'locks> NativeOperationStore<'locks, 'scope> {
    /// Plan and sync one immutable effect recipe. Only adapter profile/inverse
    /// planning is called; this method neither mutates native state nor stages
    /// files. A complete marker set is required and remains pending afterwards.
    /// This is not filesystem preflight, effect authority or a completion receipt.
    pub fn prepare_execution(
        &self,
        id: &str,
        adapter: &mut dyn NativeDeliveryAdapter,
    ) -> Result<()> {
        let inspection = self.inspect(id)?;
        ensure!(
            inspection.markers_complete,
            "native execution requires every root marker"
        );
        ensure!(
            inspection.journal.status == NativeJournalStatus::Prepared
                && inspection.journal.execution.is_none(),
            "native execution recipe already exists"
        );
        let intent = &inspection.intent;
        let snapshot = self.state.snapshot()?;
        ensure!(
            snapshot.observations()[0].value == intent.prior_state,
            "native state changed before execution planning"
        );
        let observations = self.observe_capture(&intent.capture)?;
        let mut profiles = Vec::new();
        for observation in &intent.capture.native_observations {
            profiles.push(NativeTargetProfile {
                key: observation.key.clone(),
                profile: adapter.profile(&observation.key, &observation.scope)?,
            });
        }
        validate_profiles(intent, &profiles)?;
        let mut effects = Vec::new();
        for change in &intent.plan.native_changes {
            let profile = profiles
                .iter()
                .find(|profile| profile.key == change.key)
                .context("native change has no profile")?
                .profile
                .clone();
            let request = NativeEffectRequest {
                operation_id: id.into(),
                profile,
                change: change.clone(),
            };
            let inverse_payload = adapter.plan_inverse(&request)?;
            effects.push(PlannedNativeEffect {
                request,
                inverse_payload,
            });
        }
        let prior = self.prior(&intent.prior_state)?;
        let execution = recipe(intent, profiles, effects, &prior)?;
        validate(intent, &execution, &prior)?;
        for observation in observations {
            observation.verify_unchanged()?;
        }
        snapshot.verify_unchanged()?;
        // Re-read complete metadata after caller-supplied read-only planning.
        let current = self.inspect(id)?;
        ensure!(
            current.markers_complete && current.journal == inspection.journal,
            "native operation changed during execution planning"
        );
        let before = self
            .state
            .evidence()
            .read_record(&journal_name(id))?
            .context("native journal missing")?;
        let captured: super::NativeOperationJournal = json(&before)?;
        ensure!(
            captured == inspection.journal,
            "native journal changed before recipe publication"
        );
        let mut journal = inspection.journal;
        journal.generation = 1;
        journal.status = NativeJournalStatus::Executing;
        journal.checkpoints = execution
            .steps
            .iter()
            .map(|_| NativeStepCheckpoint {
                phase: NativeStepPhase::Queued,
                attempts: vec![],
                observations: vec![],
            })
            .collect();
        journal.execution = Some(execution);
        self.state.evidence().write_record(
            &journal_name(id),
            Some(&before),
            Some(&serde_json::to_vec(&journal)?),
        )?;
        checkpoint("after_execution", 0, id)?;
        self.inspect(id)?;
        Ok(())
    }
}

fn validate_profiles(
    intent: &NativeOperationIntent,
    profiles: &[NativeTargetProfile],
) -> Result<()> {
    ensure!(
        profiles.len() == intent.capture.native_observations.len(),
        "native profile coverage differs"
    );
    let mut orders = BTreeMap::new();
    for (binding, observation) in profiles.iter().zip(&intent.capture.native_observations) {
        let profile = &binding.profile;
        ensure!(
            binding.key == observation.key
                && profile.surface == observation.key.surface
                && profile.scope == observation.scope
                && profile.client_version == observation.client_version
                && profile.platform == observation.platform
                && !profile.id.trim().is_empty()
                && profile.revision > 0
                && !profile.client_version.trim().is_empty()
                && !profile.platform.trim().is_empty(),
            "native profile differs from captured attribution"
        );
        if let Some(order) = orders.insert(profile.surface.name(), profile.exposure) {
            ensure!(
                order == profile.exposure,
                "native exposure order differs within a surface"
            );
        }
    }
    Ok(())
}

pub(super) fn validate(
    intent: &NativeOperationIntent,
    execution: &NativeExecutionPlan,
    prior: &NativeDeliveryState,
) -> Result<()> {
    validate_profiles(intent, &execution.profiles)?;
    let effects = execution
        .steps
        .iter()
        .filter_map(|step| match step {
            NativeExecutionStep::Native(effect) => Some(effect.as_ref().clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    ensure!(
        effects.len() == intent.plan.native_changes.len(),
        "native effect coverage differs"
    );
    for (effect, change) in effects.iter().zip(&intent.plan.native_changes) {
        let profile = &execution
            .profiles
            .iter()
            .find(|binding| binding.key == change.key)
            .context("native change has no profile")?
            .profile;
        ensure!(
            effect.request.operation_id == intent.operation_id
                && effect.request.change == *change
                && effect.request.profile == *profile,
            "native effect differs from immutable intent/profile"
        );
    }
    ensure!(
        *execution == recipe(intent, execution.profiles.clone(), effects, prior)?,
        "native execution recipe differs from intent"
    );
    Ok(())
}

fn recipe(
    intent: &NativeOperationIntent,
    profiles: Vec<NativeTargetProfile>,
    effects: Vec<PlannedNativeEffect>,
    prior: &NativeDeliveryState,
) -> Result<NativeExecutionPlan> {
    let mut before = Vec::new();
    let mut after = Vec::new();
    let order = |surfaces: &[TargetSurface]| -> Result<NativeExposureOrder> {
        let mut orders = surfaces.iter().map(|surface| {
            profiles
                .iter()
                .find(|binding| binding.profile.surface == *surface)
                .map(|binding| binding.profile.exposure)
                .context("exposure has no native profile")
        });
        let first = orders.next().context("exposure has no client surface")??;
        for candidate in orders {
            ensure!(
                candidate? == first,
                "shared exposure has ambiguous native order"
            );
        }
        Ok(first)
    };
    let mut add = |file: &PlannedFile, surfaces: &[TargetSurface]| -> Result<()> {
        let step = NativeExecutionStep::Exposure(file.clone());
        match order(surfaces)? {
            NativeExposureOrder::BeforeNative => before.push(step),
            NativeExposureOrder::AfterNative => after.push(step),
        }
        Ok(())
    };
    for file in &intent.plan.selector_files {
        let artifacts: Vec<_> = intent
            .plan
            .file_ownership
            .iter()
            .chain(&prior.files)
            .filter(|artifact| {
                artifact.root == file.location.root && artifact.path == file.location.path
            })
            .collect();
        let mut surfaces = artifacts
            .iter()
            .flat_map(|artifact| artifact.consumers.iter().copied())
            .collect::<Vec<_>>();
        surfaces.sort_by_key(|surface| surface.name());
        surfaces.dedup();
        add(file, &surfaces)?;
    }
    for (config, patch) in intent
        .capture
        .configurations
        .iter()
        .zip(&intent.configuration_patches)
    {
        add(&patch.file, &[config.before.surface])?;
    }
    let mut steps = intent
        .plan
        .stage_files
        .iter()
        .cloned()
        .map(NativeExecutionStep::Stage)
        .collect::<Vec<_>>();
    steps.extend(before);
    steps.extend(
        effects
            .into_iter()
            .map(|effect| NativeExecutionStep::Native(Box::new(effect))),
    );
    steps.extend(after);
    Ok(NativeExecutionPlan { profiles, steps })
}
