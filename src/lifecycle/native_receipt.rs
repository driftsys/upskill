//! Captured native claims, independent of codecs and runtime qualification.

use crate::compile::skill_package::{SkillPackage, SkillPackageBinding};
use crate::ownership::InstallationRoot;
use crate::target::TargetSurface;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillTarget {
    pub installation_id: String,
    pub surface: TargetSurface,
}

impl SkillTarget {
    pub(crate) fn index_key(&self) -> (String, &'static str) {
        (self.installation_id.clone(), self.surface.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSkillSelection {
    pub native_name: String,
    pub payload_sha256: String,
    pub package_sha256: String,
    pub revision_root: String,
    pub binding: SkillPackageBinding,
}

impl NativeSkillSelection {
    pub fn from_package(package: &SkillPackage) -> Self {
        Self {
            native_name: package.native_name.clone(),
            payload_sha256: package.payload_sha256.clone(),
            package_sha256: package.package_sha256.clone(),
            revision_root: package.revision_root.clone(),
            binding: package.binding.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBindingOutcome {
    Confirmed,
    Rejected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBindingReceipt {
    pub key: SkillTarget,
    pub scope: InstallationRoot,
    pub client_version: String,
    pub platform: String,
    pub operation_id: String,
    pub before: Option<NativeSkillSelection>,
    pub after: Option<NativeSkillSelection>,
    pub captured_prestate: Vec<u8>,
    pub outcome: NativeBindingOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeObservationState {
    Absent,
    Present(NativeSkillSelection),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeResourceRetention {
    Unknown,
    SessionsCleared { evidence_id: String },
    CachePreserved { evidence_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSkillObservation {
    pub key: SkillTarget,
    pub scope: InstallationRoot,
    pub client_version: String,
    pub platform: String,
    pub observation_id: String,
    pub captured_prestate: Vec<u8>,
    pub state: NativeObservationState,
    pub retention: NativeResourceRetention,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSkillChange {
    pub key: SkillTarget,
    pub scope: InstallationRoot,
    pub client_version: String,
    pub platform: String,
    pub observation_id: String,
    pub before: Option<NativeSkillSelection>,
    pub after: Option<NativeSkillSelection>,
    pub captured_prestate: Vec<u8>,
    pub retention: NativeResourceRetention,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NativeReceiptError {
    #[error("native recovery required for {installation_id} ({surface})", surface = .surface.name())]
    RecoveryRequired {
        installation_id: String,
        surface: TargetSurface,
    },
}
