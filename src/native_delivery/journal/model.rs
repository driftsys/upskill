use crate::files::plan::{FileValue, ObservedFile};
use crate::lifecycle::native_receipt::NativeSkillObservation;
use crate::lifecycle::reconcile::GraphChanges;
use crate::lifecycle::skill_delivery::{PackageRevision, SkillDeliveryIntent, SkillDeliveryPlan};
use crate::native_config::{CapturedConfig, ConfigContribution, ConfigPatch};
use crate::native_delivery::roots::NativeRootBinding;
use crate::native_delivery::state::{ConfigOwner, NativeStateContext};
use crate::ownership::InstallationRoot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeConfigurationCapture {
    pub before: CapturedConfig,
    pub contribution: ConfigContribution,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_contributions: Vec<ConfigContribution>,
    pub owners_after: Vec<ConfigOwner>,
}

/// Owned non-secret captured inputs, not filesystem capabilities or qualification.
/// Callers must establish an approved capture boundary before constructing these
/// inputs. Private storage does not authorize credential-bearing snapshots.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOperationCapture {
    pub graph_changes: GraphChanges,
    pub intents: Vec<SkillDeliveryIntent>,
    pub candidates: Vec<PackageRevision>,
    pub native_observations: Vec<NativeSkillObservation>,
    pub file_observations: Vec<ObservedFile>,
    pub configurations: Vec<NativeConfigurationCapture>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootMarkerNamespace {
    pub root: NativeRootBinding,
    pub namespace: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOperationIntent {
    pub schema: u32,
    pub operation_id: String,
    pub context: NativeStateContext,
    pub roots: Vec<NativeRootBinding>,
    pub marker_namespaces: Vec<RootMarkerNamespace>,
    pub prior_state: Option<FileValue>,
    pub capture: NativeOperationCapture,
    pub plan: SkillDeliveryPlan,
    pub configuration_patches: Vec<ConfigPatch>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum NativeJournalStatus {
    Prepared,
    Executing,
    Retiring,
    Complete,
    Restoring,
    RetiringRestore,
    Restored,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOperationJournal {
    pub schema: u32,
    pub operation_id: String,
    pub intent_sha256: String,
    pub generation: u64,
    pub status: NativeJournalStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<crate::native_delivery::adapter::NativeExecutionPlan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checkpoints: Vec<crate::native_delivery::adapter::NativeStepCheckpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<super::coordinator::NativeRuntime>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PendingMarker {
    pub(super) schema: u32,
    pub(super) operation_id: String,
    pub(super) root: NativeRootBinding,
    pub(super) marker_namespace: String,
    pub(super) private_root: InstallationRoot,
    pub(super) namespace: String,
    pub(super) journal_name: String,
    pub(super) intent_sha256: String,
}
#[derive(Debug, thiserror::Error)]
#[error("native operation {operation_id} requires recovery")]
pub struct NativeRecoveryRequired {
    pub operation_id: String,
    pub private_root: InstallationRoot,
    pub journal_name: String,
}
/// Metadata receipt only. It grants no authority for a future effect.
#[derive(Debug)]
pub struct PreparedNativeOperation {
    pub(super) id: String,
}
impl PreparedNativeOperation {
    pub fn id(&self) -> &str {
        &self.id
    }
}
#[derive(Debug)]
pub struct NativeOperationInspection {
    pub intent: NativeOperationIntent,
    pub journal: NativeOperationJournal,
    pub markers_complete: bool,
}
