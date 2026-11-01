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
    pub(super) fn variants(approved: &NativeDeliveryProfile) -> Vec<NativeDeliveryProfile> {
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
        variants
    }
    #[test]
    fn exact_profile_matching_has_no_version_scope_or_surface_fallback() {
        let approved = profile();
        let registry = ProfileRegistry {
            entries: vec![approved.clone()],
        };
        registry.require(&approved).unwrap();
        for variant in variants(&approved) {
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

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod wrapper_tests {
    use super::*;
    use crate::lifecycle::native_receipt::{NativeResourceRetention, NativeSkillChange};
    use crate::native_delivery::test_support::fake::Fake;
    use crate::ownership::OwnershipScope;
    use crate::target::TargetSurface;

    struct Fixture {
        gated: QualifiedNativeAdapter<Fake>,
        key: SkillTarget,
        root: InstallationRoot,
        effect: PlannedNativeEffect,
    }
    fn fixture() -> Fixture {
        let key = SkillTarget {
            installation_id: "test-installation".into(),
            surface: TargetSurface::ClaudeCli,
        };
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: "/fixture/project".into(),
        };
        let mut adapter = Fake::default();
        let profile = adapter.profile(&key, &root).unwrap();
        adapter.method_calls.lock().unwrap().clear();
        let effect = PlannedNativeEffect {
            request: NativeEffectRequest {
                operation_id: "test-operation".into(),
                profile: profile.clone(),
                change: NativeSkillChange {
                    key: key.clone(),
                    scope: root.clone(),
                    client_version: profile.client_version.clone(),
                    platform: profile.platform.clone(),
                    observation_id: "test-prestate".into(),
                    before: None,
                    after: None,
                    captured_prestate: b"null".to_vec(),
                    retention: NativeResourceRetention::Unknown,
                },
            },
            inverse_payload: b"null".to_vec(),
        };
        Fixture {
            gated: QualifiedNativeAdapter {
                adapter,
                registry: ProfileRegistry {
                    entries: vec![profile],
                },
            },
            key,
            root,
            effect,
        }
    }
    fn calls(fixture: &Fixture, expected: &[&str]) {
        assert_eq!(
            std::mem::take(&mut *fixture.gated.adapter.method_calls.lock().unwrap()),
            expected
        );
    }
    fn reason(error: anyhow::Error, expected: &str) {
        assert_eq!(
            error.downcast_ref::<NativeAdmissionError>().unwrap().reason,
            expected
        );
    }
    #[test]
    fn mismatched_returned_surface_scope_kind_and_root_reject_before_capture() {
        for field in 0..3 {
            let mut f = fixture();
            let mut returned = f.effect.request.profile.clone();
            match field {
                0 => returned.surface = TargetSurface::CopilotCli,
                1 => returned.scope.scope = OwnershipScope::User,
                _ => returned.scope.path = "/fixture/other".into(),
            }
            // Both profiles are in the private fixture registry, so admission
            // alone cannot hide a missing request-attribution guard.
            f.gated.registry.entries.push(returned.clone());
            f.gated.adapter.returned_profile = Some(returned);
            reason(
                f.gated.capture(&f.key, &f.root).unwrap_err(),
                "the adapter returned a different target or scope",
            );
            calls(&f, &["profile"]);
        }
    }
    #[test]
    fn recorded_effect_profile_must_match_the_current_admitted_profile() {
        let original = fixture().effect.request.profile;
        let variants = super::tests::variants(&original);
        for changed in variants {
            let mut f = fixture();
            f.effect.request.profile = changed;
            reason(
                f.gated.plan_inverse(&f.effect.request).unwrap_err(),
                "the recorded effect profile has changed",
            );
            calls(&f, &["profile"]);
            reason(
                f.gated.apply(&f.effect).unwrap_err(),
                "the recorded effect profile has changed",
            );
            calls(&f, &["profile"]);
            reason(
                f.gated.observe(&f.effect).unwrap_err(),
                "the recorded effect profile has changed",
            );
            calls(&f, &["profile"]);
            reason(
                f.gated.inverse(&f.effect).unwrap_err(),
                "the recorded effect profile has changed",
            );
            calls(&f, &["profile"]);
        }
    }
    #[test]
    fn admitted_fixture_forwards_exact_arguments_results_and_errors() {
        let mut f = fixture();
        let expected = f.gated.adapter.capture(&f.key, &f.root).unwrap();
        calls(&f, &["capture"]);
        assert_eq!(f.gated.capture(&f.key, &f.root).unwrap(), expected);
        calls(&f, &["profile", "capture"]);
        assert_eq!(f.gated.plan_inverse(&f.effect.request).unwrap(), b"null");
        calls(&f, &["profile", "plan_inverse"]);
        f.effect.request.change.after =
            Some(crate::lifecycle::native_receipt::NativeSkillSelection {
                native_name: "fixture-package".into(),
                payload_sha256: "a".repeat(64),
                package_sha256: "b".repeat(64),
                revision_root: "exports/revision".into(),
                binding: crate::compile::skill_package::SkillPackageBinding::LocalMarketplace {
                    directory: "/fixture/catalog".into(),
                },
            });
        f.gated.adapter.timeout = true;
        assert_eq!(
            f.gated.apply(&f.effect).unwrap(),
            NativeBindingOutcome::Unknown
        );
        assert_eq!(f.gated.adapter.calls, 1);
        assert_eq!(f.gated.adapter.selected, f.effect.request.change.after);
        calls(&f, &["profile", "apply"]);
        let expected = f.gated.adapter.observe(&f.effect).unwrap();
        calls(&f, &["observe", "capture"]);
        assert_eq!(f.gated.observe(&f.effect).unwrap(), expected);
        calls(&f, &["profile", "observe", "capture"]);
        f.gated.adapter.inverse_refuse = true;
        assert_eq!(
            f.gated.inverse(&f.effect).unwrap(),
            NativeBindingOutcome::Rejected
        );
        assert_eq!(f.gated.adapter.calls, 2);
        calls(&f, &["profile", "inverse"]);
        f.gated.adapter.fail_capture = true;
        assert_eq!(
            f.gated.capture(&f.key, &f.root).unwrap_err().to_string(),
            "fixture native observer unavailable"
        );
        calls(&f, &["profile", "capture"]);
    }
}
