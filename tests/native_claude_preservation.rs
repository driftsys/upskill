//! Offline preservation controls derived from the approved Claude probe.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/native_adapter.rs"]
mod fake;
#[path = "common/native_delivery.rs"]
mod fixture;

use anyhow::Result;
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use upskill::files::plan::{FileLocation, FileValue, ObservedFile};
use upskill::filesystem::QualifiedScope;
use upskill::lifecycle::native_receipt::{
    NativeBindingOutcome, NativeSkillObservation, SkillTarget,
};
use upskill::native_config::{
    CapturedConfig, ConfigContribution, ConfigFormat, ConfigLayer, ConfigSelector,
    plan_config_contributions,
};
use upskill::native_delivery::adapter::*;
use upskill::native_delivery::coordinator::{
    NativeDeliveryCoordinator, NativeDeliveryOutcome, RecoveryDisposition,
};
use upskill::native_delivery::journal::{NativeConfigurationCapture, NativeOperationStore};
use upskill::native_delivery::roots::{NativeRoot, lock_native_roots};
use upskill::native_delivery::state::ConfigOwner;
use upskill::ownership::{InstallationRoot, OwnershipScope};
use upskill::target::TargetSurface;

fn receipts() -> Value {
    serde_json::from_slice(include_bytes!(
        "../docs/technotes/evidence/2026-11-01-claude-local-catalog-contract/summary.json"
    ))
    .unwrap()
}

fn captured(root: &InstallationRoot, bytes: Vec<u8>) -> CapturedConfig {
    CapturedConfig {
        surface: TargetSurface::ClaudeCli,
        scope: root.clone(),
        target: ConfigLayer {
            format: ConfigFormat::Jsonc,
            file: ObservedFile {
                location: FileLocation {
                    root: root.clone(),
                    path: ".claude/settings.json".into(),
                },
                value: Some(FileValue {
                    bytes,
                    mode: Some(0o600),
                }),
            },
            tracked: true,
            ignored: false,
        },
        lower_layers: vec![],
        higher_layers: vec![],
        ownership: None,
    }
}

fn contribution(name: &str, value: Value) -> ConfigContribution {
    ConfigContribution {
        id: name.into(),
        selector: ConfigSelector::Key(vec![name.into(), "owned@catalog".into()]),
        expected: None,
        desired: Some(value),
    }
}

#[test]
fn receipt_rewrite_is_semantically_equal_but_not_exact_preservation() {
    use sha2::{Digest, Sha256};
    let packet = receipts();
    let before = packet["settings_before"].as_str().unwrap();
    let after = packet["settings_final_sample"].as_str().unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(before).unwrap(),
        serde_json::from_str::<Value>(after).unwrap()
    );
    assert_ne!(before.as_bytes(), after.as_bytes());
    for (text, field) in [(before, "baseline"), (after, "final_sample")] {
        let sample = &packet["protected_files"][0][field];
        assert_eq!(sample["mode"], "0o600");
        assert_eq!(
            Sha256::digest(text.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            sample["sha256"].as_str().unwrap()
        );
    }
    assert_eq!(packet["protected_files"][0]["exact_match"], false);
}

#[test]
fn span_owned_contributions_restore_the_receipt_baseline_without_native_reserialization() {
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: "/fixture/project".into(),
    };
    let baseline = receipts()["settings_before"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let mut config = captured(&root, baseline.clone());
    let changes = [
        contribution("enabledPlugins", json!(true)),
        contribution(
            "extraKnownMarketplaces",
            json!({"source":{"source":"directory","path":"/fixture/catalog"}}),
        ),
    ];
    let patch = plan_config_contributions(&config, &changes).unwrap();
    assert_eq!(patch.file.before, config.target.file.value);
    let expected = String::from_utf8(baseline.clone())
        .unwrap()
        .replace(
            r#""enabledPlugins": {}"#,
            r#""enabledPlugins": {"owned@catalog":true}"#,
        )
        .replace(
            r#""extraKnownMarketplaces": {}"#,
            r#""extraKnownMarketplaces": {"owned@catalog":{"source":{"path":"/fixture/catalog","source":"directory"}}}"#,
        );
    assert_eq!(
        patch.file.after,
        Some(FileValue {
            bytes: expected.into_bytes(),
            mode: Some(0o600)
        })
    );
    config.target.file.value = patch.file.after;
    config.ownership = patch.ownership_after;
    let remove = config
        .ownership
        .as_ref()
        .unwrap()
        .entries
        .iter()
        .map(|entry| ConfigContribution {
            id: entry.id.clone(),
            selector: entry.selector.clone(),
            expected: Some(entry.clone()),
            desired: None,
        })
        .collect::<Vec<_>>();
    let inverse = plan_config_contributions(&config, &remove).unwrap();
    assert_eq!(
        inverse.file.after,
        Some(FileValue {
            bytes: baseline,
            mode: Some(0o600)
        })
    );
    assert!(inverse.ownership_after.is_none());
}

/// Replays a native serializer side effect while the fake reports a successful
/// binding. The real client is never launched, and no fixture receipt is upgraded.
struct RewritingNative {
    inner: fake::Fake,
    path: PathBuf,
    reorder: bool,
    change_mode: bool,
    rewritten: Vec<u8>,
}
impl NativeDeliveryAdapter for RewritingNative {
    fn profile(
        &mut self,
        key: &SkillTarget,
        scope: &InstallationRoot,
    ) -> Result<NativeDeliveryProfile> {
        self.inner.profile(key, scope)
    }
    fn capture(
        &mut self,
        key: &SkillTarget,
        scope: &InstallationRoot,
    ) -> Result<NativeSkillObservation> {
        self.inner.capture(key, scope)
    }
    fn plan_inverse(&mut self, request: &NativeEffectRequest) -> Result<Vec<u8>> {
        self.inner.plan_inverse(request)
    }
    fn apply(&mut self, effect: &PlannedNativeEffect) -> Result<NativeBindingOutcome> {
        let result = self.inner.apply(effect)?;
        let before = std::fs::read(&self.path)?;
        self.rewritten = before.clone();
        if self.reorder {
            let value: Value = serde_json::from_slice(&before)?;
            self.rewritten = serde_json::to_vec_pretty(&value)?;
            self.rewritten.push(b'\n');
            assert_ne!(before, self.rewritten);
            assert_eq!(serde_json::from_slice::<Value>(&self.rewritten)?, value);
        }
        std::fs::write(&self.path, &self.rewritten)?;
        if self.change_mode {
            std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o644))?;
        }
        Ok(result)
    }
    fn observe(&mut self, effect: &PlannedNativeEffect) -> Result<NativeEffectObservation> {
        self.inner.observe(effect)
    }
    fn inverse(&mut self, effect: &PlannedNativeEffect) -> Result<NativeBindingOutcome> {
        self.inner.inverse(effect)
    }
}

#[test]
fn native_settings_rewrite_cannot_commit_or_restore_over_the_diverged_file() {
    for (reorder, change_mode) in [(true, false), (false, true)] {
        let project = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let primary = InstallationRoot {
            scope: OwnershipScope::Project,
            path: project.path().into(),
        };
        let private = InstallationRoot {
            scope: OwnershipScope::User,
            path: home.path().into(),
        };
        let project_scope = QualifiedScope::open(&primary).unwrap();
        let home_scope = QualifiedScope::open(&private).unwrap();
        let locks = lock_native_roots(&[
            NativeRoot {
                root: primary.clone(),
                scope: &project_scope,
            },
            NativeRoot {
                root: private.clone(),
                scope: &home_scope,
            },
        ])
        .unwrap();
        let store = NativeOperationStore::initialize(&locks, &primary, &private).unwrap();
        let path = primary.path.join(".claude/settings.json");
        std::fs::create_dir(path.parent().unwrap()).unwrap();
        let baseline = receipts()["settings_before"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec();
        std::fs::write(&path, &baseline).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut capture = fixture::capture(store.context());
        let key = capture.intents[0].key.clone();
        capture.configurations.push(NativeConfigurationCapture {
            before: captured(&primary, baseline),
            contribution: contribution("enabledPlugins", json!(true)),
            additional_contributions: vec![],
            owners_after: vec![ConfigOwner {
                entry_id: "enabledPlugins".into(),
                target: key,
            }],
        });
        let mut adapter = RewritingNative {
            inner: fake::Fake::default(),
            path: path.clone(),
            reorder,
            change_mode,
            rewritten: vec![],
        };
        let outcome = NativeDeliveryCoordinator::apply(&store, &mut adapter, capture).unwrap();
        let NativeDeliveryOutcome::RecoveryRequired {
            operation_id: Some(id),
            ..
        } = outcome
        else {
            panic!("native rewrite was accepted: {outcome:?}");
        };
        assert_eq!(adapter.inner.calls, 1);
        assert!(!primary.path.join(".upskill-lock.json").exists());
        let expected_mode = if change_mode { 0o644 } else { 0o600 };
        for disposition in [
            RecoveryDisposition::ObserveOnly,
            RecoveryDisposition::Forward,
            RecoveryDisposition::Restore,
        ] {
            let result =
                NativeDeliveryCoordinator::recover(&store, &mut adapter, &id, disposition).unwrap();
            assert!(
                matches!(result, NativeDeliveryOutcome::RecoveryRequired { .. }),
                "{disposition:?} accepted native rewrite: {result:?}"
            );
            assert_eq!(
                adapter.inner.calls, 1,
                "recovery invoked another native effect"
            );
            assert_eq!(std::fs::read(&path).unwrap(), adapter.rewritten);
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                expected_mode
            );
            assert!(!primary.path.join(".upskill-lock.json").exists());
            for root in [&primary, &private] {
                assert!(
                    root.path
                        .join(".upskill/native-pending/pending.json")
                        .exists()
                );
            }
            assert!(store.inspect(&id).unwrap().markers_complete);
        }
        assert!(
            !adapter
                .inner
                .method_calls
                .lock()
                .unwrap()
                .contains(&"inverse")
        );
    }
}
