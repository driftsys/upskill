//! File-only candidate planning from non-secret captured fixtures; no native I/O.
#![cfg(unix)]
use serde_json::json;
use upskill::compile::skill_package::SkillPackageBinding;
use upskill::files::plan::{FileLocation, FileValue, ObservedFile};
use upskill::lifecycle::native_receipt::{
    NativeResourceRetention, NativeSkillChange, NativeSkillSelection, SkillTarget,
};
use upskill::native_config::{
    CapturedConfig, ConfigFormat, ConfigLayer, ConfigPatch, ConfigSelector,
};
use upskill::native_delivery::adapter::{
    NativeActionKind, NativeDeliveryProfile, NativeExposureOrder,
};
use upskill::native_delivery::opencode::{OpenCodeSourceError, plan_skill_sources};
use upskill::ownership::{InstallationRoot, OwnershipScope};
use upskill::target::TargetSurface;
const OLD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const NEW: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
fn selection(id: &str, payload: &str) -> NativeSkillSelection {
    let revision = format!(".upskill/exports/opencode/{id}/revisions/{payload}");
    NativeSkillSelection {
        native_name: format!("upskill-{}", "c".repeat(64)),
        payload_sha256: payload.into(),
        package_sha256: "d".repeat(64),
        binding: SkillPackageBinding::ExplicitSkillSource {
            directory: format!("/fixture/project/{revision}/skills").into(),
        },
        revision_root: revision,
    }
}
fn input(source: Option<&str>) -> (NativeDeliveryProfile, CapturedConfig, NativeSkillChange) {
    let scope = InstallationRoot {
        scope: OwnershipScope::Project,
        path: "/fixture/project".into(),
    };
    let profile = NativeDeliveryProfile {
        id: "candidate-only".into(),
        revision: 1,
        surface: TargetSurface::OpenCodeCli,
        scope: scope.clone(),
        client_version: "2.1.4".into(),
        platform: "unqualified-fixture".into(),
        exposure: NativeExposureOrder::BeforeNative,
        action: NativeActionKind::FileOnly,
    };
    let config = CapturedConfig {
        surface: TargetSurface::OpenCodeCli,
        scope: scope.clone(),
        target: ConfigLayer {
            format: ConfigFormat::Jsonc,
            file: ObservedFile {
                location: FileLocation {
                    root: scope.clone(),
                    path: ".opencode/opencode.json".into(),
                },
                value: source.map(|source| FileValue {
                    bytes: source.as_bytes().to_vec(),
                    mode: Some(0o640),
                }),
            },
            tracked: false,
            ignored: true,
        },
        lower_layers: vec![],
        higher_layers: vec![],
        ownership: None,
    };
    let change = NativeSkillChange {
        key: SkillTarget {
            installation_id: "owned-1".into(),
            surface: TargetSurface::OpenCodeCli,
        },
        scope,
        client_version: "2.1.4".into(),
        platform: "unqualified-fixture".into(),
        observation_id: "offline-capture".into(),
        before: None,
        after: Some(selection("owned-1", OLD)),
        captured_prestate: vec![],
        retention: NativeResourceRetention::Unknown,
    };
    (profile, config, change)
}
fn apply(config: &mut CapturedConfig, patch: ConfigPatch) {
    assert_eq!(patch.file.location, config.target.file.location);
    assert_eq!(patch.file.before, config.target.file.value);
    config.target.file.value = patch.file.after;
    config.ownership = patch.ownership_after;
}
fn text(config: &CapturedConfig) -> &str {
    std::str::from_utf8(&config.target.file.value.as_ref().unwrap().bytes).unwrap()
}
#[test]
fn array_attach_refresh_noop_and_detach_preserve_manual_bytes_and_modes() {
    let original =
        "{\r\n // manual\r\n \"skills\": [\"/manual\" /* keep */,],\r\n \"other\": true,\r\n}\r\n";
    let (profile, mut config, mut change) = input(Some(original));
    let old = format!("/fixture/project/.upskill/exports/opencode/owned-1/revisions/{OLD}/skills");
    let new = format!("/fixture/project/.upskill/exports/opencode/owned-1/revisions/{NEW}/skills");
    let patch = plan_skill_sources(&profile, &config, &[change.clone()]).unwrap();
    assert_eq!(
        patch.ownership_after.as_ref().unwrap().entries[0].selector,
        ConfigSelector::ArrayEntry {
            path: vec!["skills".into()],
            value: json!(old)
        }
    );
    apply(&mut config, patch);
    assert!(text(&config).contains("\"/manual\" /* keep */"));
    assert_eq!(text(&config).matches(&old).count(), 1);
    assert_eq!(config.target.file.value.as_ref().unwrap().mode, Some(0o640));
    let before = text(&config).to_owned();
    change.before = change.after.clone();
    change.after = Some(selection("owned-1", NEW));
    let patch = plan_skill_sources(&profile, &config, &[change.clone()]).unwrap();
    apply(&mut config, patch);
    assert_eq!(text(&config), before.replace(&old, &new));
    change.before = change.after.clone();
    let patch = plan_skill_sources(&profile, &config, &[change.clone()]).unwrap();
    assert_eq!(patch.file.before, patch.file.after);
    change.after = None;
    let patch = plan_skill_sources(&profile, &config, &[change]).unwrap();
    assert_eq!(
        patch.file.after,
        Some(FileValue {
            bytes: original.as_bytes().to_vec(),
            mode: Some(0o640)
        })
    );
    assert!(patch.ownership_after.is_none());
}
#[test]
fn legacy_object_form_preserves_paths_urls_and_exact_baseline() {
    let original = "{\"skills\":{\"paths\":[\"/manual\" /* keep */,],\"urls\":[\"https://example.invalid/skills/\"]}}";
    let (profile, mut config, mut change) = input(Some(original));
    let patch = plan_skill_sources(&profile, &config, &[change.clone()]).unwrap();
    assert_eq!(
        patch.ownership_after.as_ref().unwrap().entries[0].selector,
        ConfigSelector::ArrayEntry {
            path: vec!["skills".into(), "paths".into()],
            value: json!(format!(
                "/fixture/project/.upskill/exports/opencode/owned-1/revisions/{OLD}/skills"
            ))
        }
    );
    apply(&mut config, patch);
    assert!(text(&config).contains("\"urls\":[\"https://example.invalid/skills/\"]"));
    change.before = change.after.take();
    let patch = plan_skill_sources(&profile, &config, &[change]).unwrap();
    assert_eq!(patch.file.after.unwrap().bytes, original.as_bytes());
}
#[test]
fn multiple_items_share_one_patch_and_removing_one_retains_the_other() {
    let (profile, mut config, one) = input(None);
    let mut two = one.clone();
    two.key.installation_id = "owned-2".into();
    two.after = Some(selection("owned-2", NEW));
    let patch = plan_skill_sources(&profile, &config, &[one.clone(), two.clone()]).unwrap();
    assert_eq!(patch.file.after.as_ref().unwrap().mode, Some(0o600));
    assert_eq!(patch.ownership_after.as_ref().unwrap().entries.len(), 2);
    assert!(
        serde_json::from_slice::<serde_json::Value>(&patch.file.after.as_ref().unwrap().bytes)
            .unwrap()["skills"]
            .is_array()
    );
    apply(&mut config, patch);
    let mut remove_one = one;
    remove_one.before = remove_one.after.take();
    let patch = plan_skill_sources(&profile, &config, &[remove_one]).unwrap();
    apply(&mut config, patch);
    assert!(!text(&config).contains("owned-1/"));
    assert!(text(&config).contains("owned-2/"));
    assert_eq!(config.ownership.as_ref().unwrap().entries.len(), 1);
    two.before = two.after.take();
    let patch = plan_skill_sources(&profile, &config, &[two]).unwrap();
    assert_eq!(patch.file.after, None);
    assert_eq!(patch.ownership_after, None);
}
#[test]
fn unsafe_overlay_layers_manual_duplicates_and_malformed_shapes_cannot_be_planned() {
    for index in 0..10 {
        let (profile, mut config, change) = input(Some("{}"));
        match index {
   0=>config.target.tracked=true,1=>config.target.ignored=false,2=>config.target.file.location.path="opencode.json".into(),3=>config.target.file.location.root.path="/external-xdg".into(),
   4=>config.target.file.value.as_mut().unwrap().bytes=b"{\"skills\":1}".to_vec(),
   5=>config.target.file.value.as_mut().unwrap().bytes=b"{\"skills\":[1]}".to_vec(),
   6=>config.target.file.value.as_mut().unwrap().bytes=b"{\"skills\":[],\"skills\":[]}".to_vec(),
   7=>config.target.file.value.as_mut().unwrap().bytes=serde_json::to_vec(&json!({"skills":[format!("/fixture/project/.upskill/exports/opencode/owned-1/revisions/{OLD}/skills")]})).unwrap(),
   8|9=>{let layer=ConfigLayer{file:ObservedFile{location:FileLocation{root:config.scope.clone(),path:"manual.json".into()},value:Some(FileValue{bytes:serde_json::to_vec(&json!({"skills":[format!("/fixture/project/.upskill/exports/opencode/owned-1/revisions/{OLD}/skills")]})).unwrap(),mode:Some(0o644)})},format:ConfigFormat::Jsonc,tracked:true,ignored:false};if index==8 {config.lower_layers.push(layer)}else{config.higher_layers.push(layer)}},
   _=>unreachable!()
  }
        assert!(
            plan_skill_sources(&profile, &config, &[change]).is_err(),
            "case {index}"
        );
    }
}
#[test]
fn profile_change_and_owned_selector_mismatches_never_redirect_a_contribution() {
    for index in 0..12 {
        let (mut profile, mut config, mut change) = input(Some("{}"));
        match index {
            0 => {
                profile.client_version = "private-canary-pin".into();
                change.client_version = profile.client_version.clone();
            }
            1 => profile.surface = TargetSurface::ClaudeCli,
            2 => profile.action = NativeActionKind::Command,
            3 => change.key.surface = TargetSurface::CopilotCli,
            4 => change.scope.path = "/other".into(),
            5 => change.client_version = "2.1.3".into(),
            6 => change.platform = "other".into(),
            7 => config.surface = TargetSurface::ClaudeCli,
            8 => {
                config.scope.path = "/other".into();
                config.target.file.location.root = config.scope.clone();
            }
            9 => change.before = change.after.take(),
            10 => {
                change.after.as_mut().unwrap().binding = SkillPackageBinding::ExplicitSkillSource {
                    directory: "/foreign/skills".into(),
                }
            }
            11 => {
                change.after.as_mut().unwrap().binding = SkillPackageBinding::LocalMarketplace {
                    directory: format!(
                        "/fixture/project/.upskill/exports/opencode/owned-1/revisions/{OLD}/skills"
                    )
                    .into(),
                }
            }
            _ => unreachable!(),
        }
        let error = plan_skill_sources(&profile, &config, &[change]).unwrap_err();
        assert!(!format!("{error:?}: {error}").contains("private-canary"));
    }
    let (profile, config, change) = input(None);
    assert_eq!(
        plan_skill_sources(&profile, &config, &[]).unwrap_err(),
        OpenCodeSourceError::InvalidChange
    );
    assert!(plan_skill_sources(&profile, &config, &[change.clone(), change]).is_err());
}

#[test]
fn invalid_package_records_cannot_choose_a_foreign_or_unframed_source() {
    for index in 0..7 {
        let (profile, config, mut change) = input(Some("{}"));
        match index {
            0 => {
                change.key.installation_id = "bad.name".into();
                change.after = Some(selection("bad.name", OLD));
            }
            1 => change.after = Some(selection("owned-1", &"a".repeat(63))),
            2 => change.after.as_mut().unwrap().package_sha256 = "D".repeat(64),
            3 => change.after.as_mut().unwrap().native_name = "c".repeat(64),
            4 => change.after.as_mut().unwrap().native_name = format!("foreign-{}", "c".repeat(64)),
            5 => change.after.as_mut().unwrap().revision_root = "foreign-revision".into(),
            6 => {
                change.after.as_mut().unwrap().binding = SkillPackageBinding::ExplicitSkillSource {
                    directory: "/foreign/skills".into(),
                }
            }
            _ => unreachable!(),
        }
        assert_eq!(
            plan_skill_sources(&profile, &config, &[change]).unwrap_err(),
            OpenCodeSourceError::InvalidSelection,
            "case {index}"
        );
    }
    let (profile, mut config, mut change) = input(Some("{}"));
    let patch = plan_skill_sources(&profile, &config, &[change.clone()]).unwrap();
    apply(&mut config, patch);
    change.before = change.after.clone();
    change.after = Some(selection("owned-1", NEW));
    change.after.as_mut().unwrap().native_name = format!("upskill-{}", "e".repeat(64));
    assert_eq!(
        plan_skill_sources(&profile, &config, &[change]).unwrap_err(),
        OpenCodeSourceError::InvalidSelection
    );
}
#[test]
fn user_configuration_stays_inside_the_selected_root_and_errors_discard_raw_parse_details() {
    let (mut profile, mut config, mut change) = input(Some("{}"));
    profile.scope.scope = OwnershipScope::User;
    config.scope.scope = OwnershipScope::User;
    config.target.file.location.root.scope = OwnershipScope::User;
    change.scope.scope = OwnershipScope::User;
    config.target.file.location.path = ".config/opencode/opencode.json".into();
    let patch = plan_skill_sources(&profile, &config, &[change.clone()]).unwrap();
    assert_eq!(patch.file.location, config.target.file.location);
    config.target.file.location.root.path = "/external-xdg".into();
    assert!(plan_skill_sources(&profile, &config, &[change.clone()]).is_err());
    config.target.file.location.root = profile.scope.clone();
    config.target.file.value.as_mut().unwrap().bytes =
        b"{\"skills\":private-config-canary}".to_vec();
    let error = plan_skill_sources(&profile, &config, &[change]).unwrap_err();
    assert_eq!(error, OpenCodeSourceError::ConfigurationConflict);
    assert!(!format!("{error:?}: {error}").contains("private-config-canary"));
}

#[test]
fn a_claimed_refresh_requires_an_owned_before_source() {
    let (profile, config, mut change) = input(Some("{}"));
    change.before = change.after.clone();
    change.after = Some(selection("owned-1", NEW));
    assert_eq!(
        plan_skill_sources(&profile, &config, &[change]).unwrap_err(),
        OpenCodeSourceError::OwnershipConflict
    );
}

#[test]
fn legacy_urls_only_object_gains_paths_without_changing_its_existing_source() {
    let original = "{\"skills\":{\"urls\":[\"https://example.invalid/catalog/\"]}}";
    let (profile, mut config, mut change) = input(Some(original));
    let patch = plan_skill_sources(&profile, &config, &[change.clone()]).unwrap();
    apply(&mut config, patch);
    assert!(text(&config).contains("\"urls\":[\"https://example.invalid/catalog/\"]"));
    change.before = change.after.take();
    let patch = plan_skill_sources(&profile, &config, &[change]).unwrap();
    assert_eq!(patch.file.after.unwrap().bytes, original.as_bytes());
}

#[test]
fn legacy_object_fields_and_each_array_type_are_checked_independently() {
    for skills in [
        json!({"unknown": []}),
        json!({"paths": "not-an-array"}),
        json!({"urls": "not-an-array"}),
        json!({"paths": ["/manual", 1]}),
        json!({"urls": ["https://example.invalid/catalog/", false]}),
    ] {
        let source = serde_json::to_string(&json!({"skills":skills})).unwrap();
        let (profile, config, change) = input(Some(&source));
        assert_eq!(
            plan_skill_sources(&profile, &config, &[change]).unwrap_err(),
            OpenCodeSourceError::UnsupportedConfiguration
        );
    }
}
#[test]
fn a_managed_prefix_requires_a_complete_lowercase_hexadecimal_suffix() {
    for suffix in ["a".repeat(63), "A".repeat(64), "z".repeat(64)] {
        let (profile, config, mut change) = input(Some("{}"));
        change.after.as_mut().unwrap().native_name = format!("upskill-{suffix}");
        assert_eq!(
            plan_skill_sources(&profile, &config, &[change]).unwrap_err(),
            OpenCodeSourceError::InvalidSelection
        );
    }
}
#[test]
fn a_source_change_requires_a_nonempty_observation_id() {
    let (profile, config, mut change) = input(Some("{}"));
    change.observation_id.clear();
    assert_eq!(
        plan_skill_sources(&profile, &config, &[change]).unwrap_err(),
        OpenCodeSourceError::InvalidChange
    );
}
