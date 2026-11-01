use super::{NativeOperationCapture, NativeOperationStore};
use crate::files::plan::{FileValue, ObservedFile};
use crate::filesystem::DestinationSnapshot;
use crate::lifecycle::skill_delivery::{
    SkillDeliveryInput, SkillDeliveryPlan, SkillDeliveryState, SkillIntentSelection,
};
use crate::lifecycle::state::CombinedState;
use crate::native_config::{ConfigPatch, plan_config_contributions};
use crate::native_delivery::plan_native_skill_files;
use crate::native_delivery::state::NativeDeliveryState;
use crate::ownership::{InstallationRoot, OwnershipScope};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

impl<'locks, 'scope: 'locks> NativeOperationStore<'locks, 'scope> {
    pub(super) fn prior(&self, value: &Option<FileValue>) -> Result<NativeDeliveryState> {
        if let Some(value) = value {
            return self.state.parse_captured_state(&value.bytes);
        }
        Ok(NativeDeliveryState {
            context: self.context().clone(),
            skills: SkillDeliveryState {
                scope: self.context().scope.clone(),
                graph: CombinedState {
                    schema: 2,
                    origin: None,
                    installations: vec![],
                    bundles: vec![],
                    references: vec![],
                    artifacts: vec![],
                    owner_consumers: vec![],
                    owner_contracts: vec![],
                    legacy: None,
                },
                revisions: vec![],
                staged: vec![],
                receipts: vec![],
            },
            intents: vec![],
            files: vec![],
            configurations: vec![],
        })
    }
    pub(super) fn plan(
        &self,
        prior: &NativeDeliveryState,
        capture: &NativeOperationCapture,
    ) -> Result<(SkillDeliveryPlan, Vec<ConfigPatch>)> {
        let plan = plan_native_skill_files(&SkillDeliveryInput {
            current: &prior.skills,
            scope: &self.context().scope,
            graph_changes: &capture.graph_changes,
            intents: &capture.intents,
            candidates: &capture.candidates,
            native_observations: &capture.native_observations,
            file_observations: &capture.file_observations,
        })?;
        let mut patches = Vec::new();
        for config in &capture.configurations {
            for layer in std::iter::once(&config.before.target)
                .chain(&config.before.lower_layers)
                .chain(&config.before.higher_layers)
            {
                self.roots.scope_and_guard(&layer.file.location.root)?;
            }
            let old = prior.configurations.iter().find(|old| {
                old.capture.target.file.location == config.before.target.file.location
                    && old.capture.surface == config.before.surface
            });
            ensure!(
                config.before.ownership.as_ref()
                    == old.and_then(|old| old.capture.ownership.as_ref()),
                "configuration ownership differs from prior state"
            );
            let changes = std::iter::once(config.contribution.clone())
                .chain(config.additional_contributions.iter().cloned())
                .collect::<Vec<_>>();
            let patch = plan_config_contributions(&config.before, &changes)?;
            let entries = patch
                .ownership_after
                .as_ref()
                .map(|ownership| ownership.entries.as_slice())
                .unwrap_or(&[]);
            ensure!(
                entries.len() == config.owners_after.len(),
                "configuration owner coverage differs"
            );
            let mut ids = std::collections::BTreeSet::new();
            for owner in &config.owners_after {
                ensure!(
                    ids.insert(&owner.entry_id)
                        && entries.iter().any(|entry| entry.id == owner.entry_id),
                    "configuration owner entry differs"
                );
                ensure!(
                    owner.target.surface == config.before.surface
                        && plan.intents.iter().any(|intent| intent.key == owner.target
                            && matches!(intent.selection, SkillIntentSelection::Selected { .. })),
                    "configuration owner has no selected target"
                );
            }
            patches.push(patch);
        }
        let files: Vec<_> = plan
            .stage_files
            .iter()
            .chain(&plan.selector_files)
            .chain(patches.iter().map(|patch| &patch.file))
            .collect();
        for (index, file) in files.iter().enumerate() {
            self.roots.scope_and_guard(&file.location.root)?;
            crate::ownership::validate_relative_path(&file.location.path)?;
            for reserved in [
                ".upskill-lock.json",
                ".upskill/native-evidence",
                ".upskill/native-pending",
                ".upskill/state",
            ] {
                ensure!(
                    !crate::filesystem::paths_may_overlap(&file.location.path, reserved),
                    "native operation target overlaps reserved storage"
                );
            }
            ensure!(
                files[..index]
                    .iter()
                    .all(|other| other.location.root != file.location.root
                        || !crate::filesystem::paths_may_overlap(
                            &other.location.path,
                            &file.location.path
                        )),
                "native operation targets overlap"
            );
        }
        Ok((plan, patches))
    }
    pub(super) fn observe_capture(
        &self,
        capture: &NativeOperationCapture,
    ) -> Result<Vec<DestinationSnapshot<'_>>> {
        let mut groups: BTreeMap<
            (OwnershipScope, std::path::PathBuf),
            BTreeMap<String, ObservedFile>,
        > = BTreeMap::new();
        for file in capture.file_observations.iter().chain(
            capture
                .configurations
                .iter()
                .flat_map(|config| {
                    std::iter::once(&config.before.target)
                        .chain(&config.before.lower_layers)
                        .chain(&config.before.higher_layers)
                })
                .map(|layer| &layer.file),
        ) {
            let files = groups
                .entry((file.location.root.scope, file.location.root.path.clone()))
                .or_default();
            if let Some(previous) = files.insert(file.location.path.clone(), file.clone()) {
                ensure!(previous == *file, "conflicting captured file observations");
            }
        }
        let mut snapshots = Vec::new();
        for ((scope, path), files) in groups {
            let root = InstallationRoot { scope, path };
            let (scope, _) = self.roots.scope_and_guard(&root)?;
            let locations: Vec<_> = files.values().map(|file| file.location.clone()).collect();
            let snapshot = scope.observe_destinations(&locations)?;
            for actual in snapshot.observations() {
                ensure!(
                    files.get(&actual.location.path) == Some(actual),
                    "native captured file changed before durable preparation"
                );
            }
            snapshots.push(snapshot);
        }
        Ok(snapshots)
    }
}
