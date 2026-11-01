//! Production admission rejects unsupported profiles before adapter effects.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/native_adapter.rs"]
mod fake;
use upskill::lifecycle::native_receipt::{NativeResourceRetention, NativeSkillChange, SkillTarget};
use upskill::native_delivery::adapter::{
    NativeDeliveryAdapter, NativeEffectRequest, PlannedNativeEffect,
};
use upskill::native_delivery::admission::{NativeAdmissionError, QualifiedNativeAdapter};
use upskill::ownership::{InstallationRoot, OwnershipScope};
use upskill::target::TargetSurface;

#[test]
fn all_four_unqualified_routes_reject_before_capture() {
    let home = tempfile::tempdir().unwrap();
    for surface in [
        TargetSurface::ClaudeCli,
        TargetSurface::CopilotCli,
        TargetSurface::CodexCli,
        TargetSurface::OpenCodeCli,
    ] {
        for scope in [OwnershipScope::Project, OwnershipScope::User] {
            let root = InstallationRoot {
                scope,
                path: home.path().into(),
            };
            let target = SkillTarget {
                installation_id: "fixture".into(),
                surface,
            };
            // Every rejected operation may only request read-only profile metadata.
            let mut native = fake::Fake {
                fail_capture: true,
                backing: Some(home.path().join("binding.json")),
                ..Default::default()
            };
            let profile = native.profile(&target, &root).unwrap();
            let calls = native.method_calls.clone();
            calls.lock().unwrap().clear();
            let request = NativeEffectRequest {
                operation_id: "fixture-operation".into(),
                profile,
                change: NativeSkillChange {
                    key: target.clone(),
                    scope: root.clone(),
                    client_version: "fixture-1".into(),
                    platform: "fixture-os".into(),
                    observation_id: "fixture-observation".into(),
                    before: None,
                    after: None,
                    captured_prestate: b"null".to_vec(),
                    retention: NativeResourceRetention::Unknown,
                },
            };
            let effect = PlannedNativeEffect {
                request,
                inverse_payload: b"null".to_vec(),
            };
            let mut adapter = QualifiedNativeAdapter::production(native);
            let check = |error: anyhow::Error| {
                let error = error.downcast_ref::<NativeAdmissionError>().unwrap();
                assert_eq!(error.surface, surface.name());
                assert_eq!(error.version, "fixture-1");
                assert_eq!(error.platform, "fixture-os");
                assert_eq!(
                    error.reason,
                    "no admitted exact profile; retain the existing delivery route"
                );
                assert_eq!(std::mem::take(&mut *calls.lock().unwrap()), vec!["profile"]);
            };
            check(adapter.profile(&target, &root).unwrap_err());
            check(adapter.capture(&target, &root).unwrap_err());
            check(adapter.plan_inverse(&effect.request).unwrap_err());
            check(adapter.apply(&effect).unwrap_err());
            check(adapter.observe(&effect).unwrap_err());
            check(adapter.inverse(&effect).unwrap_err());
            assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
        }
    }
}
