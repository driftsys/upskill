//! Exact-profile admission for native delivery. The production registry is empty.
//!
//! This gate adds no discovery or qualification authority. A future registry
//! entry requires reviewed native evidence and the complete invocation contract;
//! matching this metadata alone will not qualify an executable or environment.
use super::adapter::*;
use crate::lifecycle::native_receipt::{NativeBindingOutcome, NativeSkillObservation, SkillTarget};
use crate::ownership::InstallationRoot;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("native qualification is required for {surface} version {version} on {platform}: {reason}")]
pub struct NativeAdmissionError {
    pub surface: &'static str,
    pub version: String,
    pub platform: String,
    pub reason: &'static str,
}

/// Private entries prevent callers from treating supplied profile claims as
/// qualified evidence. No production entry has been admitted.
#[derive(Default)]
struct ProfileRegistry {
    entries: Vec<NativeDeliveryProfile>,
}
impl ProfileRegistry {
    fn require(&self, profile: &NativeDeliveryProfile) -> Result<()> {
        if self.entries.iter().any(|entry| entry == profile) {
            return Ok(());
        }
        Err(rejection(
            profile,
            "no admitted exact profile; retain the existing delivery route",
        )
        .into())
    }
}
fn rejection(profile: &NativeDeliveryProfile, reason: &'static str) -> NativeAdmissionError {
    NativeAdmissionError {
        surface: profile.surface.name(),
        version: profile.client_version.clone(),
        platform: profile.platform.clone(),
        reason,
    }
}

/// Apply the closed production admission registry to an adapter. The wrapped
/// adapter's profile method must be read-only, as required by the adapter trait.
/// This does not expose a production CLI route or migrate legacy state.
pub struct QualifiedNativeAdapter<A> {
    adapter: A,
    registry: ProfileRegistry,
}
impl<A: NativeDeliveryAdapter> QualifiedNativeAdapter<A> {
    pub fn production(adapter: A) -> Self {
        Self {
            adapter,
            registry: ProfileRegistry::default(),
        }
    }
    fn require_effect(&mut self, effect: &NativeEffectRequest) -> Result<()> {
        let current = self.profile(&effect.change.key, &effect.change.scope)?;
        if current != effect.profile {
            return Err(rejection(&current, "the recorded effect profile has changed").into());
        }
        Ok(())
    }
}
impl<A: NativeDeliveryAdapter> NativeDeliveryAdapter for QualifiedNativeAdapter<A> {
    fn profile(
        &mut self,
        target: &SkillTarget,
        scope: &InstallationRoot,
    ) -> Result<NativeDeliveryProfile> {
        let profile = self.adapter.profile(target, scope)?;
        if profile.surface != target.surface || &profile.scope != scope {
            return Err(
                rejection(&profile, "the adapter returned a different target or scope").into(),
            );
        }
        self.registry.require(&profile)?;
        Ok(profile)
    }
    fn capture(
        &mut self,
        target: &SkillTarget,
        scope: &InstallationRoot,
    ) -> Result<NativeSkillObservation> {
        self.profile(target, scope)?;
        self.adapter.capture(target, scope)
    }
    fn plan_inverse(&mut self, request: &NativeEffectRequest) -> Result<Vec<u8>> {
        self.require_effect(request)?;
        self.adapter.plan_inverse(request)
    }
    fn apply(&mut self, effect: &PlannedNativeEffect) -> Result<NativeBindingOutcome> {
        self.require_effect(&effect.request)?;
        self.adapter.apply(effect)
    }
    fn observe(&mut self, effect: &PlannedNativeEffect) -> Result<NativeEffectObservation> {
        self.require_effect(&effect.request)?;
        self.adapter.observe(effect)
    }
    fn inverse(&mut self, effect: &PlannedNativeEffect) -> Result<NativeBindingOutcome> {
        self.require_effect(&effect.request)?;
        self.adapter.inverse(effect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ownership::OwnershipScope;
    use crate::target::TargetSurface;

    fn profile() -> NativeDeliveryProfile {
        NativeDeliveryProfile {
            id: "fixture".into(),
            revision: 1,
            surface: TargetSurface::ClaudeCli,
            scope: InstallationRoot {
                scope: OwnershipScope::Project,
                path: "/fixture/project".into(),
            },
            client_version: "fixture-1".into(),
            platform: "fixture-os".into(),
            exposure: NativeExposureOrder::BeforeNative,
            action: NativeActionKind::Command,
        }
    }
    #[test]
    fn exact_profile_matching_has_no_version_scope_or_surface_fallback() {
        let approved = profile();
        let registry = ProfileRegistry {
            entries: vec![approved.clone()],
        };
        registry.require(&approved).unwrap();
        let mut variants = vec![];
        let mut changed = approved.clone();
        changed.id.push_str("-other");
        variants.push(changed);
        let mut changed = approved.clone();
        changed.revision += 1;
        variants.push(changed);
        let mut changed = approved.clone();
        changed.surface = TargetSurface::CodexCli;
        variants.push(changed);
        let mut changed = approved.clone();
        changed.scope.scope = OwnershipScope::User;
        variants.push(changed);
        let mut changed = approved.clone();
        changed.scope.path = "/fixture/other".into();
        variants.push(changed);
        let mut changed = approved.clone();
        changed.client_version.push_str("-new");
        variants.push(changed);
        let mut changed = approved.clone();
        changed.platform.push_str("-new");
        variants.push(changed);
        let mut changed = approved.clone();
        changed.exposure = NativeExposureOrder::AfterNative;
        variants.push(changed);
        let mut changed = approved.clone();
        changed.action = NativeActionKind::FileOnly;
        variants.push(changed);
        for variant in variants {
            assert!(
                registry
                    .require(&variant)
                    .unwrap_err()
                    .downcast_ref::<NativeAdmissionError>()
                    .is_some()
            );
        }
    }
}
