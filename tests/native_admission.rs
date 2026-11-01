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
            // If admission calls capture first, this returns a different error.
            let mut native = fake::Fake {
                fail_capture: true,
                backing: Some(home.path().join("binding.json")),
                ..Default::default()
            };
            let profile = native.profile(&target, &root).unwrap();
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
            let errors = [
                adapter.profile(&target, &root).unwrap_err(),
                adapter.capture(&target, &root).unwrap_err(),
                adapter.plan_inverse(&effect.request).unwrap_err(),
                adapter.apply(&effect).unwrap_err(),
                adapter.observe(&effect).unwrap_err(),
                adapter.inverse(&effect).unwrap_err(),
            ];
            for error in errors {
                assert!(
                    error.downcast_ref::<NativeAdmissionError>().is_some(),
                    "{error:#}"
                );
                assert!(error.to_string().contains(surface.name()));
                assert!(error.to_string().contains("qualification"));
            }
            assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
        }
    }
}
