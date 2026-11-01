//! Inactive native delivery infrastructure; no production profile admission.

use crate::lifecycle::skill_delivery::{SkillDeliveryInput, SkillDeliveryPlan};
use anyhow::Result;

/// Fixed private export policy. Captured source modes remain separate evidence.
pub const SKILL_FILE_POLICY: &str = "native-skill-files-v1";

/// Plan private immutable exports using real destination observations, without
/// applying files or calling a client. Owner execution requires known source
/// resource modes. Existing mode drift rejects even when selection is unchanged.
/// This is a planning prerequisite, not a durable coordinator or native admission.
pub fn plan_native_skill_files(input: &SkillDeliveryInput<'_>) -> Result<SkillDeliveryPlan> {
    crate::lifecycle::skill_delivery::plan_native_files(input)
}

pub mod state;
