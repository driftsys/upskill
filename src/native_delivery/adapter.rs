//! Native adapter contracts. A profile records attribution, not qualification.
use crate::files::plan::PlannedFile;
use crate::lifecycle::native_receipt::{
    NativeBindingOutcome, NativeSkillChange, NativeSkillObservation, SkillTarget,
};
use crate::ownership::InstallationRoot;
use crate::target::TargetSurface;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeExposureOrder {
    BeforeNative,
    AfterNative,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeActionKind {
    Command,
    FileOnly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDeliveryProfile {
    pub id: String,
    pub revision: u64,
    pub surface: TargetSurface,
    pub scope: InstallationRoot,
    pub client_version: String,
    pub platform: String,
    pub exposure: NativeExposureOrder,
    pub action: NativeActionKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeTargetProfile {
    pub key: SkillTarget,
    pub profile: NativeDeliveryProfile,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEffectRequest {
    pub operation_id: String,
    pub profile: NativeDeliveryProfile,
    pub change: NativeSkillChange,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedNativeEffect {
    pub request: NativeEffectRequest,
    /// Opaque private data describing an owned inverse and its preconditions.
    pub inverse_payload: Vec<u8>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeEffectRelation {
    Before,
    After,
    Diverged,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEffectObservation {
    pub operation_id: String,
    pub profile_id: String,
    pub profile_revision: u64,
    pub effect: NativeEffectRelation,
    pub binding: NativeSkillObservation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum NativeExecutionStep {
    Stage(PlannedFile),
    Exposure(PlannedFile),
    Native(Box<PlannedNativeEffect>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeExecutionPlan {
    pub profiles: Vec<NativeTargetProfile>,
    pub steps: Vec<NativeExecutionStep>,
}

/// Planning methods must be read-only. Mutation methods must check the recorded
/// preconditions and touch only the owned delta. Command outcomes never replace
/// independent effect and effective-binding observations. Real implementations
/// require separate qualification before production admission. Capture/prestate
/// and inverse payloads must contain only approved non-secret facts. A shared
/// credential-bearing document requires a separate approved capture/storage
/// contract before any producer passes it to durable delivery storage.
/// A Before/After relation must describe stable, attributable owned state. If
/// an earlier native command may still be running, observation is Unknown until
/// the qualified profile establishes quiescence. Errors must also remain
/// non-secret; raw authentication output is not journal evidence.
pub trait NativeDeliveryAdapter {
    fn profile(
        &mut self,
        target: &SkillTarget,
        scope: &InstallationRoot,
    ) -> Result<NativeDeliveryProfile>;
    fn capture(
        &mut self,
        target: &SkillTarget,
        scope: &InstallationRoot,
    ) -> Result<NativeSkillObservation>;
    fn plan_inverse(&mut self, request: &NativeEffectRequest) -> Result<Vec<u8>>;
    fn apply(&mut self, effect: &PlannedNativeEffect) -> Result<NativeBindingOutcome>;
    fn observe(&mut self, effect: &PlannedNativeEffect) -> Result<NativeEffectObservation>;
    fn inverse(&mut self, effect: &PlannedNativeEffect) -> Result<NativeBindingOutcome>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeStepPhase {
    Queued,
    Pending,
    ObservedAfter,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum NativeStepObservation {
    File(crate::files::plan::ObservedFile),
    Native(Box<NativeEffectObservation>),
    Failed(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAttempt {
    pub outcome: NativeBindingOutcome,
    pub detail: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeStepCheckpoint {
    pub phase: NativeStepPhase,
    pub attempts: Vec<NativeAttempt>,
    pub observations: Vec<NativeStepObservation>,
}
