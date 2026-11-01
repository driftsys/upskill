//! Native fixture with durable state for fresh-process coordinator tests.
#![allow(dead_code)]
use anyhow::Result;
use upskill::lifecycle::native_receipt::*;
use upskill::native_delivery::adapter::*;
use upskill::ownership::InstallationRoot;
pub struct Fake {
    pub selected: Option<NativeSkillSelection>,
    pub calls: usize,
    pub timeout: bool,
    pub reject_observation: bool,
    pub exposure: NativeExposureOrder,
    pub action: NativeActionKind,
    pub selector: Option<std::path::PathBuf>,
    pub file_selection: Option<NativeSkillSelection>,
    pub refuse: bool,
    pub inverse_timeout: bool,
    pub inverse_refuse: bool,
    pub profile_revision: u64,
    pub cache_valid: bool,
    pub invalidate_cache: bool,
    pub exposure_probe: Option<(std::path::PathBuf, String)>,
    pub error_after_apply: bool,
    pub unknown_after_apply: bool,
    pub fail_profile: bool,
    pub fail_capture: bool,
    pub backing: Option<std::path::PathBuf>,
}
impl Default for Fake {
    fn default() -> Self {
        Self {
            selected: None,
            calls: 0,
            timeout: false,
            reject_observation: false,
            exposure: NativeExposureOrder::BeforeNative,
            action: NativeActionKind::Command,
            selector: None,
            file_selection: None,
            refuse: false,
            inverse_timeout: false,
            inverse_refuse: false,
            profile_revision: 1,
            cache_valid: true,
            invalidate_cache: false,
            exposure_probe: None,
            error_after_apply: false,
            unknown_after_apply: false,
            fail_profile: false,
            fail_capture: false,
            backing: None,
        }
    }
}
impl Fake {
    fn native_selection(&self) -> Option<NativeSkillSelection> {
        if self.action == NativeActionKind::FileOnly {
            self.selector
                .as_ref()
                .filter(|path| path.exists())
                .and(self.file_selection.clone())
        } else if let Some(path) = &self.backing {
            std::fs::read(path)
                .ok()
                .map(|bytes| serde_json::from_slice(&bytes).unwrap())
                .unwrap_or(None)
        } else {
            self.selected.clone()
        }
    }
    fn persist(&self) -> Result<()> {
        if let Some(path) = &self.backing {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            for (path, bytes) in [
                (path.clone(), serde_json::to_vec(&self.selected)?),
                (
                    path.with_extension("calls"),
                    self.calls.to_string().into_bytes(),
                ),
            ] {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .mode(0o600)
                    .open(path)?;
                file.write_all(&bytes)?;
                file.sync_all()?;
            }
            std::fs::File::open(path.parent().unwrap())?.sync_all()?;
        }
        Ok(())
    }
}
impl NativeDeliveryAdapter for Fake {
    fn profile(
        &mut self,
        target: &SkillTarget,
        scope: &InstallationRoot,
    ) -> Result<NativeDeliveryProfile> {
        if self.fail_profile {
            anyhow::bail!("fixture unsupported native profile");
        }
        Ok(NativeDeliveryProfile {
            id: "fixture-command".into(),
            revision: self.profile_revision,
            surface: target.surface,
            scope: scope.clone(),
            client_version: "fixture-1".into(),
            platform: "fixture-os".into(),
            exposure: self.exposure,
            action: self.action,
        })
    }
    fn capture(
        &mut self,
        target: &SkillTarget,
        scope: &InstallationRoot,
    ) -> Result<NativeSkillObservation> {
        if self.fail_capture {
            anyhow::bail!("fixture native observer unavailable");
        }
        let selected = if self.reject_observation && self.calls > 0
            || self.selector.as_ref().is_some_and(|path| !path.exists())
        {
            None
        } else {
            self.native_selection()
        };
        Ok(NativeSkillObservation {
            key: target.clone(),
            scope: scope.clone(),
            client_version: "fixture-1".into(),
            platform: "fixture-os".into(),
            observation_id: format!("capture-{}", self.calls),
            captured_prestate: serde_json::to_vec(&selected)?,
            state: if self.unknown_after_apply && self.calls > 0 {
                NativeObservationState::Unknown
            } else {
                selected.map_or(
                    NativeObservationState::Absent,
                    NativeObservationState::Present,
                )
            },
            retention: if self.cache_valid {
                NativeResourceRetention::CachePreserved {
                    evidence_id: "fixture-cache".into(),
                }
            } else {
                NativeResourceRetention::Unknown
            },
        })
    }
    fn plan_inverse(&mut self, request: &NativeEffectRequest) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&request.change.before)?)
    }
    fn apply(&mut self, effect: &PlannedNativeEffect) -> Result<NativeBindingOutcome> {
        if let Some((path, selected)) = &self.exposure_probe {
            let exposed = std::fs::read(path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                .is_some_and(|value| {
                    value["plugins"]["fixture"] == selected.as_str() && value["enabled"] == true
                });
            anyhow::ensure!(
                exposed == (effect.request.profile.exposure == NativeExposureOrder::BeforeNative),
                "fake native command observed the wrong configuration exposure order"
            );
        }
        anyhow::ensure!(
            self.native_selection() == effect.request.change.before,
            "fake native forward precondition changed"
        );
        self.selected = self.native_selection();
        self.calls += 1;
        if self.refuse {
            self.persist()?;
            return Ok(NativeBindingOutcome::Rejected);
        }
        self.selected = effect.request.change.after.clone();
        if self.invalidate_cache {
            self.cache_valid = false;
        }
        self.persist()?;
        if self.error_after_apply {
            anyhow::bail!("fixture native command failed after effect");
        }
        Ok(if self.timeout {
            NativeBindingOutcome::Unknown
        } else {
            NativeBindingOutcome::Confirmed
        })
    }
    fn observe(&mut self, effect: &PlannedNativeEffect) -> Result<NativeEffectObservation> {
        let binding = self.capture(&effect.request.change.key, &effect.request.change.scope)?;
        let selected = self.native_selection();
        let relation = if selected == effect.request.change.after {
            NativeEffectRelation::After
        } else if selected == effect.request.change.before {
            NativeEffectRelation::Before
        } else {
            NativeEffectRelation::Diverged
        };
        Ok(NativeEffectObservation {
            operation_id: effect.request.operation_id.clone(),
            profile_id: effect.request.profile.id.clone(),
            profile_revision: effect.request.profile.revision,
            effect: relation,
            binding,
        })
    }
    fn inverse(&mut self, effect: &PlannedNativeEffect) -> Result<NativeBindingOutcome> {
        anyhow::ensure!(
            self.native_selection() == effect.request.change.after,
            "fake native inverse precondition changed"
        );
        self.selected = self.native_selection();
        self.calls += 1;
        if self.inverse_refuse {
            self.persist()?;
            return Ok(NativeBindingOutcome::Rejected);
        }
        self.selected = serde_json::from_slice(&effect.inverse_payload)?;
        self.persist()?;
        Ok(if self.inverse_timeout {
            NativeBindingOutcome::Unknown
        } else {
            NativeBindingOutcome::Confirmed
        })
    }
}
