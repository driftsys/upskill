//! Pure integrated native state codec. No file effects or production migration.
use crate::lifecycle::native_receipt::SkillTarget;
use crate::lifecycle::skill_delivery::{SkillDeliveryIntent, SkillDeliveryState};
use crate::native_config::CapturedConfig;
use crate::ownership::{InstallationRoot, OwnedArtifact};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeStateContext {
    pub scope: InstallationRoot,
    pub private_root: InstallationRoot,
    pub namespace: String,
    pub additional_roots: Vec<InstallationRoot>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigOwner {
    pub entry_id: String,
    pub target: SkillTarget,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeConfigState {
    pub capture: CapturedConfig,
    pub owners: Vec<ConfigOwner>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NativeDeliveryState {
    pub context: NativeStateContext,
    pub skills: SkillDeliveryState,
    pub intents: Vec<SkillDeliveryIntent>,
    pub files: Vec<OwnedArtifact>,
    pub configurations: Vec<NativeConfigState>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKind {
    NativePrestate,
    ConfigurationSnapshot,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceReference {
    pub kind: EvidenceKind,
    pub digest: String,
    pub length: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBlob {
    pub reference: EvidenceReference,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedNativeState {
    pub json: String,
    pub private_evidence: Vec<EvidenceBlob>,
}
mod codec;
mod evidence;
mod validate;
pub use codec::{parse_native_state, serialize_native_state};

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) use evidence::{validate_blob, validate_reference};

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) use codec::evidence_references;
