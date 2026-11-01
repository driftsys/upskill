//! Acceptance coverage for pure, owned native configuration edits.
use serde_json::json;
use upskill::files::plan::{FileLocation, FileValue, ObservedFile};
use upskill::native_config::{
    CapturedConfig, ConfigContribution, ConfigFormat, ConfigLayer, ConfigSelector,
    plan_config_patch,
};
use upskill::ownership::{InstallationRoot, OwnershipScope};
use upskill::target::TargetSurface;

fn capture(format: ConfigFormat, source: Option<&str>) -> CapturedConfig {
    let scope = InstallationRoot {
        scope: OwnershipScope::Project,
        path: "/workspace/project".into(),
    };
    let path = if format == ConfigFormat::Toml {
        ".codex/config.toml"
    } else {
        ".opencode/opencode.json"
    };
    CapturedConfig {
        surface: if format == ConfigFormat::Toml {
            TargetSurface::CodexCli
        } else {
            TargetSurface::OpenCodeCli
        },
        scope: scope.clone(),
        target: ConfigLayer {
            format,
            file: ObservedFile {
                location: FileLocation {
                    root: scope,
                    path: path.into(),
                },
                value: source.map(|s| FileValue {
                    bytes: s.as_bytes().to_vec(),
                    mode: Some(0o600),
                }),
            },
            tracked: false,
            ignored: true,
        },
        lower_layers: vec![],
        higher_layers: vec![],
        ownership: None,
    }
}
fn key(path: &[&str], desired: Option<serde_json::Value>) -> ConfigContribution {
    ConfigContribution {
        id: "example".into(),
        selector: ConfigSelector::Key(path.iter().map(|s| s.to_string()).collect()),
        expected: None,
        desired,
    }
}
fn apply_capture(config: &mut CapturedConfig, patch: upskill::native_config::ConfigPatch) {
    assert_eq!(patch.file.before, config.target.file.value);
    assert_eq!(patch.file.location, config.target.file.location);
    config.target.file.value = patch.file.after;
    config.ownership = patch.ownership_after;
}
fn expect_owned(config: &CapturedConfig, change: &mut ConfigContribution) {
    change.expected = Some(
        config
            .ownership
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .find(|e| e.id == change.id)
            .unwrap()
            .clone(),
    );
}
fn bytes(config: &CapturedConfig) -> &str {
    std::str::from_utf8(&config.target.file.value.as_ref().unwrap().bytes).unwrap()
}

#[test]
fn jsonc_add_update_remove_preserves_manual_document_exactly() {
    let original = "{\r\n  // manual café\r\n  \"manual\": [1, 2,],\r\n}\r\n";
    let mut config = capture(ConfigFormat::Jsonc, Some(original));
    let mut change = key(&["managed"], Some(json!("old")));
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    assert!(bytes(&config).contains("  // manual café\r\n  \"manual\": [1, 2,],\r\n"));
    expect_owned(&config, &mut change);
    let before = bytes(&config).to_owned();
    change.desired = Some(json!("new"));
    let patch = plan_config_patch(&config, &change).unwrap();
    assert_eq!(
        std::str::from_utf8(&patch.file.after.as_ref().unwrap().bytes).unwrap(),
        before.replace("\"old\"", "\"new\"")
    );
    apply_capture(&mut config, patch);
    expect_owned(&config, &mut change);
    change.desired = None;
    let patch = plan_config_patch(&config, &change).unwrap();
    assert_eq!(
        patch.file.after.as_ref().unwrap().bytes,
        original.as_bytes()
    );
    assert!(patch.ownership_after.is_none());
}

#[test]
fn toml_owned_value_updates_preserve_legal_unrelated_syntax() {
    let original =
        "# manual\nwhen = 1979-05-27T07:32:00Z\ntext = '''multi\nline'''\n[[manual]]\nx = 4\n";
    let mut config = capture(ConfigFormat::Toml, Some(original));
    let mut change = key(&["plugins", "sample", "enabled"], Some(json!(true)));
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    assert!(bytes(&config).contains(original));
    expect_owned(&config, &mut change);
    let before = bytes(&config).to_owned();
    change.desired = Some(json!(false));
    let patch = plan_config_patch(&config, &change).unwrap();
    assert_eq!(
        std::str::from_utf8(&patch.file.after.as_ref().unwrap().bytes).unwrap(),
        before.replace("true", "false")
    );
    apply_capture(&mut config, patch);
    expect_owned(&config, &mut change);
    change.desired = None;
    assert_eq!(
        plan_config_patch(&config, &change)
            .unwrap()
            .file
            .after
            .as_ref()
            .unwrap()
            .bytes,
        original.as_bytes()
    );
}

#[test]
fn final_removal_restores_initial_absence_across_multiple_owners() {
    for format in [ConfigFormat::Jsonc, ConfigFormat::Toml] {
        let mut config = capture(format, None);
        let mut one = key(&["one"], Some(json!(1)));
        let mut two = key(&["two"], Some(json!(2)));
        two.id = "second".into();
        let patch = plan_config_patch(&config, &one).unwrap();
        apply_capture(&mut config, patch);
        let patch = plan_config_patch(&config, &two).unwrap();
        apply_capture(&mut config, patch);
        expect_owned(&config, &mut one);
        one.desired = None;
        let patch = plan_config_patch(&config, &one).unwrap();
        apply_capture(&mut config, patch);
        assert!(bytes(&config).contains("two"));
        expect_owned(&config, &mut two);
        two.desired = None;
        let patch = plan_config_patch(&config, &two).unwrap();
        assert_eq!(patch.file.after, None);
        assert_eq!(patch.ownership_after, None);
    }
}

#[test]
fn owned_array_element_does_not_adopt_or_rewrite_manual_entries() {
    let mut config = capture(
        ConfigFormat::Jsonc,
        Some("{\"skills\":{\"paths\":[\"/manual\" /* keep */,]}}"),
    );
    let mut change = ConfigContribution {
        id: "example".into(),
        selector: ConfigSelector::ArrayEntry {
            path: vec!["skills".into(), "paths".into()],
            value: json!("/old"),
        },
        expected: None,
        desired: Some(json!("/old")),
    };
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    assert!(bytes(&config).contains("\"/manual\" /* keep */"));
    let before = bytes(&config).to_owned();
    expect_owned(&config, &mut change);
    change.desired = Some(json!("/new"));
    let patch = plan_config_patch(&config, &change).unwrap();
    assert_eq!(
        std::str::from_utf8(&patch.file.after.as_ref().unwrap().bytes).unwrap(),
        before.replace("/old", "/new")
    );
    apply_capture(&mut config, patch);
    change.selector = ConfigSelector::ArrayEntry {
        path: vec!["skills".into(), "paths".into()],
        value: json!("/new"),
    };
    expect_owned(&config, &mut change);
    change.desired = None;
    let patch = plan_config_patch(&config, &change).unwrap();
    assert_eq!(
        patch.file.after.unwrap().bytes,
        b"{\"skills\":{\"paths\":[\"/manual\" /* keep */,]}}"
    );
}

#[test]
fn identical_manual_values_and_array_entries_are_not_adopted() {
    for (source, change) in [
        ("{\"managed\":true}", key(&["managed"], Some(json!(true)))),
        (
            "{\"paths\":[\"/manual\"]}",
            ConfigContribution {
                id: "example".into(),
                selector: ConfigSelector::ArrayEntry {
                    path: vec!["paths".into()],
                    value: json!("/manual"),
                },
                expected: None,
                desired: Some(json!("/manual")),
            },
        ),
    ] {
        assert!(plan_config_patch(&capture(ConfigFormat::Jsonc, Some(source)), &change).is_err());
    }
    assert!(
        plan_config_patch(
            &capture(ConfigFormat::Toml, Some("managed = true\n")),
            &key(&["managed"], Some(json!(true)))
        )
        .is_err()
    );
}

#[test]
fn duplicate_keys_and_malformed_jsonc_are_rejected_before_editing() {
    for source in [
        "{\"x\":1,\"x\":2}",
        "{\"x\":{\"a\":1,\"\\u0061\":2}}",
        "{\"x\":[,]}",
        "{\"x\":[1,,]}",
        "{ /* unterminated",
        "{\"x\":1} garbage",
    ] {
        assert!(
            plan_config_patch(
                &capture(ConfigFormat::Jsonc, Some(source)),
                &key(&["managed"], Some(json!(true)))
            )
            .is_err(),
            "{source}"
        );
    }
    for source in ["x = 1\nx = 2\n", "[x]\na = 1\n[x]\nb = 2\n"] {
        assert!(
            plan_config_patch(
                &capture(ConfigFormat::Toml, Some(source)),
                &key(&["managed"], Some(json!(true)))
            )
            .is_err()
        );
    }
}

#[test]
fn unrelated_edits_survive_owned_removal_and_owned_edits_conflict() {
    for format in [ConfigFormat::Jsonc, ConfigFormat::Toml] {
        let original = if format == ConfigFormat::Jsonc {
            "{\"manual\":1}"
        } else {
            "manual = 1\n"
        };
        let mut config = capture(format, Some(original));
        let mut change = key(&["managed"], Some(json!(true)));
        let patch = plan_config_patch(&config, &change).unwrap();
        apply_capture(&mut config, patch);
        expect_owned(&config, &mut change);
        let updated = bytes(&config).replace('1', "2");
        config.target.file.value.as_mut().unwrap().bytes = updated.into_bytes();
        change.desired = None;
        let patch = plan_config_patch(&config, &change).unwrap();
        let after = String::from_utf8(patch.file.after.unwrap().bytes).unwrap();
        assert!(after.contains('2'));
        assert!(!after.contains("managed"));
        config.target.file.value.as_mut().unwrap().bytes =
            bytes(&config).replace("true", "false").into_bytes();
        assert!(plan_config_patch(&config, &change).is_err());
    }
}

#[test]
fn overlapping_ownership_and_stale_expected_receipts_are_rejected() {
    let mut config = capture(ConfigFormat::Jsonc, None);
    let mut change = key(&["managed"], Some(json!({"enabled":true})));
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    let mut child = key(&["managed", "other"], Some(json!(1)));
    child.id = "child".into();
    assert!(plan_config_patch(&config, &child).is_err());
    assert!(plan_config_patch(&config, &change).is_err());
    expect_owned(&config, &mut change);
    change.expected.as_mut().unwrap().value = json!({"enabled":false});
    assert!(plan_config_patch(&config, &change).is_err());
}

#[test]
fn higher_layer_masking_and_inherited_duplicate_sources_are_rejected() {
    let mut config = capture(ConfigFormat::Jsonc, Some("{}"));
    let mut layer = config.target.clone();
    layer.file.location.path = "opencode.json".into();
    layer.file.value = Some(FileValue {
        bytes: b"{\"managed\":false}".to_vec(),
        mode: None,
    });
    config.higher_layers.push(layer.clone());
    assert!(plan_config_patch(&config, &key(&["managed"], Some(json!(true)))).is_err());
    config.higher_layers.clear();
    layer.file.value.as_mut().unwrap().bytes = b"{\"paths\":[\"/source\"]}".to_vec();
    config.lower_layers.push(layer);
    let change = ConfigContribution {
        id: "example".into(),
        selector: ConfigSelector::ArrayEntry {
            path: vec!["paths".into()],
            value: json!("/source"),
        },
        expected: None,
        desired: Some(json!("/source")),
    };
    assert!(plan_config_patch(&config, &change).is_err());
}

#[test]
fn tracked_overlays_external_roots_and_copied_ownership_are_rejected() {
    let mut config = capture(ConfigFormat::Jsonc, None);
    let mut change = key(&["managed"], Some(json!(true)));
    config.target.tracked = true;
    assert!(plan_config_patch(&config, &change).is_err());
    config.target.tracked = false;
    config.target.ignored = false;
    assert!(plan_config_patch(&config, &change).is_err());
    config.target.ignored = true;
    config.target.file.location.root.path = "/external/xdg".into();
    assert!(plan_config_patch(&config, &change).is_err());
    config.target.file.location.root = config.scope.clone();
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    expect_owned(&config, &mut change);
    assert!(plan_config_patch(&config, &change).is_ok());
    config.scope.path = "/other/checkout".into();
    config.target.file.location.root = config.scope.clone();
    assert!(
        plan_config_patch(&config, &change)
            .unwrap_err()
            .to_string()
            .contains("ownership belongs to another document")
    );
}

#[test]
fn nested_toml_removal_preserves_new_manual_comments_and_assignments() {
    let mut config = capture(ConfigFormat::Toml, Some("# baseline\n"));
    let mut change = key(&["plugins", "sample", "enabled"], Some(json!(true)));
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    expect_owned(&config, &mut change);
    config
        .target
        .file
        .value
        .as_mut()
        .unwrap()
        .bytes
        .extend_from_slice(b"# added manually\nmanual = 9\n");
    change.desired = None;
    let patch = plan_config_patch(&config, &change).unwrap();
    let text = String::from_utf8(patch.file.after.unwrap().bytes).unwrap();
    assert!(text.contains("# baseline\n# added manually\nmanual = 9\n"));
    assert!(!text.contains("enabled"));
}

#[test]
fn inserting_into_existing_toml_sections_and_arrays_preserves_adjacent_bytes() {
    let original = "[plugins.sample] # manual section\nother = 5 # keep\n[next]\nx = 7\n";
    let mut config = capture(ConfigFormat::Toml, Some(original));
    let mut change = key(&["plugins", "sample", "enabled"], Some(json!(true)));
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    assert!(bytes(&config).contains("other = 5 # keep\n[next]\nx = 7\n"));
    expect_owned(&config, &mut change);
    change.desired = None;
    assert_eq!(
        plan_config_patch(&config, &change)
            .unwrap()
            .file
            .after
            .unwrap()
            .bytes,
        original.as_bytes()
    );
    let mut config = capture(ConfigFormat::Toml, Some("paths = [\"manual\", # keep\n]\n"));
    let mut change = ConfigContribution {
        id: "example".into(),
        selector: ConfigSelector::ArrayEntry {
            path: vec!["paths".into()],
            value: json!("owned"),
        },
        expected: None,
        desired: Some(json!("owned")),
    };
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    assert!(bytes(&config).contains("\"manual\", # keep\n"));
    expect_owned(&config, &mut change);
    change.desired = Some(json!("updated"));
    assert!(plan_config_patch(&config, &change).is_ok());
}

#[test]
fn json_strings_private_rawvalue_keys_and_comment_tokens_keep_their_meaning() {
    let original =
        r#"{"$serde_json::private::RawValue":"0","url":"https://host/*a*/,//b","unicode":"é"}"#;
    let config = capture(ConfigFormat::Jsonc, Some(original));
    let patch = plan_config_patch(&config, &key(&["managed"], Some(json!(true)))).unwrap();
    let after = String::from_utf8(patch.file.after.unwrap().bytes).unwrap();
    assert!(after.contains(&original[..original.len() - 1]));
    let parsed: serde_json::Value = serde_json::from_str(&after).unwrap();
    assert_eq!(parsed["$serde_json::private::RawValue"], json!("0"));
    assert_eq!(parsed["url"], json!("https://host/*a*/,//b"));
}

#[test]
fn legal_unrelated_inline_dotted_toml_does_not_require_selected_value_support() {
    let original = "inline = { first.second = 1 }\n";
    let config = capture(ConfigFormat::Toml, Some(original));
    let patch = plan_config_patch(&config, &key(&["managed"], Some(json!(true)))).unwrap();
    assert!(
        String::from_utf8(patch.file.after.unwrap().bytes)
            .unwrap()
            .contains(original)
    );
}

#[test]
fn before_precondition_and_raw_owned_hash_detect_concurrent_changes() {
    let mut config = capture(ConfigFormat::Jsonc, Some("{\"manual\":1}"));
    let mut change = key(&["owned"], Some(json!("abc")));
    let initial = config.target.file.value.clone();
    let patch = plan_config_patch(&config, &change).unwrap();
    assert_eq!(patch.file.before, initial);
    apply_capture(&mut config, patch);
    expect_owned(&config, &mut change);
    let observed = bytes(&config).replace("\"abc\"", "\"a\\u0062c\"");
    config.target.file.value.as_mut().unwrap().bytes = observed.into_bytes();
    change.desired = Some(json!("new"));
    assert!(plan_config_patch(&config, &change).is_err());
}

#[test]
fn distinct_manual_lower_array_entries_can_coexist_but_higher_arrays_reject() {
    let mut config = capture(ConfigFormat::Jsonc, Some("{}"));
    let mut layer = config.target.clone();
    layer.file.location.path = "opencode.json".into();
    layer.file.value = Some(FileValue {
        bytes: b"{\"paths\":[\"/manual\"]}".to_vec(),
        mode: None,
    });
    config.lower_layers.push(layer.clone());
    let change = ConfigContribution {
        id: "example".into(),
        selector: ConfigSelector::ArrayEntry {
            path: vec!["paths".into()],
            value: json!("/owned"),
        },
        expected: None,
        desired: Some(json!("/owned")),
    };
    assert!(plan_config_patch(&config, &change).is_ok());
    config.lower_layers.clear();
    config.higher_layers.push(layer);
    assert!(plan_config_patch(&config, &change).is_err());
}

#[test]
fn removal_keeps_new_comments_between_owned_json_key_and_value() {
    let mut config = capture(ConfigFormat::Jsonc, Some("{}"));
    let mut change = key(&["managed"], Some(json!(true)));
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    expect_owned(&config, &mut change);
    config.target.file.value.as_mut().unwrap().bytes =
        b"{\"managed\" /* keep key note */ : /* keep value note */ true,\"manual\":2}".to_vec();
    change.desired = None;
    let patch = plan_config_patch(&config, &change).unwrap();
    assert_eq!(
        patch.file.after.unwrap().bytes,
        b"{ /* keep key note */  /* keep value note */ \"manual\":2}"
    );
}

#[test]
fn corrupted_rollback_baselines_do_not_bypass_output_validation() {
    let mut config = capture(ConfigFormat::Jsonc, None);
    let mut change = key(&["managed"], Some(json!(true)));
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    expect_owned(&config, &mut change);
    config
        .ownership
        .as_mut()
        .unwrap()
        .rollback
        .as_mut()
        .unwrap()
        .baseline = Some(FileValue {
        bytes: b"{broken".to_vec(),
        mode: None,
    });
    change.desired = None;
    assert!(plan_config_patch(&config, &change).is_err());
}

#[test]
fn no_op_revalidates_owned_values_and_preserves_exact_file_precondition() {
    let mut config = capture(ConfigFormat::Jsonc, None);
    let mut change = key(&["managed"], Some(json!(true)));
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    expect_owned(&config, &mut change);
    let patch = plan_config_patch(&config, &change).unwrap();
    assert_eq!(patch.file.before, patch.file.after);
    assert_eq!(patch.ownership_after, config.ownership);
    config.target.file.value.as_mut().unwrap().bytes = b"{\"managed\":false}".to_vec();
    assert!(plan_config_patch(&config, &change).is_err());
    config.target.file.value = None;
    assert!(plan_config_patch(&config, &change).is_err());
}

#[test]
fn unsupported_toml_selected_shapes_return_errors_without_changing_capture() {
    let config = capture(ConfigFormat::Toml, Some("inline = { first.second = 1 }\n"));
    let original = config.clone();
    assert!(
        plan_config_patch(
            &config,
            &key(&["inline", "first", "third"], Some(json!(true)))
        )
        .is_err()
    );
    assert!(plan_config_patch(&config, &key(&["managed"], Some(serde_json::Value::Null))).is_err());
    assert_eq!(config, original);
}

#[test]
fn toml_byte_order_mark_stays_first_during_add_and_manual_edit_removal() {
    let original = "\u{feff}# manual\nmanual = 1\n";
    let mut config = capture(ConfigFormat::Toml, Some(original));
    let mut change = key(&["managed"], Some(json!(true)));
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    expect_owned(&config, &mut change);
    assert!(bytes(&config).starts_with('\u{feff}'));
    assert!(bytes(&config).contains("# manual\nmanual = 1\n"));
    config
        .target
        .file
        .value
        .as_mut()
        .unwrap()
        .bytes
        .extend_from_slice(b"# new manual note\n");
    change.desired = None;
    let patch = plan_config_patch(&config, &change).unwrap();
    let after = String::from_utf8(patch.file.after.unwrap().bytes).unwrap();
    assert!(after.starts_with('\u{feff}'));
    assert!(after.contains("# manual\nmanual = 1\n# new manual note\n"));
    assert!(!after.contains("managed"));
}

#[test]
fn new_configuration_files_select_private_mode_and_existing_modes_survive() {
    for format in [ConfigFormat::Jsonc, ConfigFormat::Toml] {
        let config = capture(format, None);
        let patch = plan_config_patch(&config, &key(&["managed"], Some(json!(true)))).unwrap();
        assert_eq!(patch.file.after.as_ref().unwrap().mode, Some(0o600));
        assert_eq!(
            patch
                .ownership_after
                .unwrap()
                .rollback
                .unwrap()
                .expected
                .mode,
            Some(0o600)
        );
        let mut config = capture(
            format,
            Some(if format == ConfigFormat::Jsonc {
                "{}"
            } else {
                ""
            }),
        );
        config.target.file.value.as_mut().unwrap().mode = Some(0o640);
        let patch = plan_config_patch(&config, &key(&["managed"], Some(json!(true)))).unwrap();
        assert_eq!(patch.file.after.unwrap().mode, Some(0o640));
    }
}

fn array_entry(id: &str, value: &str) -> ConfigContribution {
    ConfigContribution {
        id: id.into(),
        selector: ConfigSelector::ArrayEntry {
            path: vec!["paths".into()],
            value: json!(value),
        },
        expected: None,
        desired: Some(json!(value)),
    }
}

#[test]
fn array_removal_after_manual_edits_preserves_only_manual_entries_and_comments() {
    for (format, original, edited) in [
        (
            ConfigFormat::Jsonc,
            "{\"paths\":[\"/manual\" /* keep */,]}",
            "{\"paths\":[\"/manual\" /* keep extra */,]}",
        ),
        (
            ConfigFormat::Toml,
            "paths = [\"/manual\", # keep\n]\n",
            "paths = [\"/manual\", # keep extra\n]\n",
        ),
    ] {
        let mut config = capture(format, Some(original));
        let mut change = array_entry("example", "/owned");
        let patch = plan_config_patch(&config, &change).unwrap();
        apply_capture(&mut config, patch);
        expect_owned(&config, &mut change);
        config.target.file.value.as_mut().unwrap().bytes =
            bytes(&config).replace("keep", "keep extra").into_bytes();
        change.desired = None;
        let patch = plan_config_patch(&config, &change).unwrap();
        assert_eq!(patch.file.before, config.target.file.value);
        assert_eq!(patch.file.after.unwrap().bytes, edited.as_bytes());
        assert!(patch.ownership_after.is_none());
    }
}

#[test]
fn updating_an_owned_array_entry_rejects_only_the_new_inherited_identity() {
    for (format, original, inherited) in [
        (
            ConfigFormat::Jsonc,
            "{\"paths\":[]}",
            "{\"paths\":[\"/new\"]}",
        ),
        (ConfigFormat::Toml, "paths = []\n", "paths = [\"/new\"]\n"),
    ] {
        let mut config = capture(format, Some(original));
        let mut change = array_entry("example", "/old");
        let patch = plan_config_patch(&config, &change).unwrap();
        apply_capture(&mut config, patch);
        expect_owned(&config, &mut change);
        change.desired = Some(json!("/new"));
        assert!(plan_config_patch(&config, &change).is_ok());
        let mut lower = config.target.clone();
        lower.file.location.path = "inherited.config".into();
        lower.file.value.as_mut().unwrap().bytes = inherited.as_bytes().to_vec();
        config.lower_layers.push(lower);
        assert!(
            plan_config_patch(&config, &change)
                .unwrap_err()
                .to_string()
                .contains("desired source already occurs in another configuration layer")
        );
    }
}

#[test]
fn manual_mode_changes_disable_baseline_restoration_without_deleting_the_file() {
    for (format, original, expected_bytes) in [
        (ConfigFormat::Jsonc, None, "{}"),
        (
            ConfigFormat::Jsonc,
            Some("{\"manual\":1}"),
            "{\"manual\":1}",
        ),
        (ConfigFormat::Toml, None, "\n"),
        (ConfigFormat::Toml, Some("# manual\n"), "\n# manual\n"),
    ] {
        let mut config = capture(format, original);
        let baseline = config.target.file.value.clone();
        let mut change = key(&["managed"], Some(json!(true)));
        let patch = plan_config_patch(&config, &change).unwrap();
        apply_capture(&mut config, patch);
        expect_owned(&config, &mut change);
        change.desired = None;
        assert_eq!(
            plan_config_patch(&config, &change).unwrap().file.after,
            baseline
        );
        config.target.file.value.as_mut().unwrap().mode = Some(0o640);
        let patch = plan_config_patch(&config, &change).unwrap();
        assert_eq!(patch.file.before, config.target.file.value);
        let after = patch.file.after.unwrap();
        assert_eq!(after.mode, Some(0o640));
        assert_eq!(after.bytes, expected_bytes.as_bytes());
        assert!(patch.ownership_after.is_none());
    }
}

#[test]
fn independent_owners_share_one_array_and_retire_without_removing_each_other() {
    for (format, original) in [
        (ConfigFormat::Jsonc, "{\"paths\":[\"/manual\",]}"),
        (ConfigFormat::Toml, "paths = [\"/manual\", # keep\n]\n"),
    ] {
        let mut config = capture(format, Some(original));
        let mut one = array_entry("one", "/one");
        let mut two = array_entry("two", "/two");
        let patch = plan_config_patch(&config, &one).unwrap();
        apply_capture(&mut config, patch);
        let patch = plan_config_patch(&config, &two).unwrap();
        apply_capture(&mut config, patch);
        assert_eq!(config.ownership.as_ref().unwrap().entries.len(), 2);
        assert!(bytes(&config).contains("/one"));
        assert!(bytes(&config).contains("/two"));
        expect_owned(&config, &mut one);
        one.desired = None;
        let patch = plan_config_patch(&config, &one).unwrap();
        apply_capture(&mut config, patch);
        assert!(!bytes(&config).contains("/one"));
        assert!(bytes(&config).contains("/two"));
        assert!(bytes(&config).contains("/manual"));
        assert_eq!(config.ownership.as_ref().unwrap().entries.len(), 1);
        assert_eq!(config.ownership.as_ref().unwrap().entries[0].id, "two");
        expect_owned(&config, &mut two);
        two.desired = None;
        let patch = plan_config_patch(&config, &two).unwrap();
        assert_eq!(patch.file.after.unwrap().bytes, original.as_bytes());
        assert!(patch.ownership_after.is_none());
    }
}

#[test]
fn lower_layer_scalar_conflicts_reject_without_a_higher_layer_mask() {
    for (format, empty, inherited) in [
        (ConfigFormat::Jsonc, "{}", "{\"managed\":false}"),
        (ConfigFormat::Toml, "", "managed = false\n"),
    ] {
        let mut config = capture(format, Some(empty));
        let change = key(&["managed"], Some(json!(true)));
        assert!(plan_config_patch(&config, &change).is_ok());
        let mut lower = config.target.clone();
        lower.file.location.path = "inherited.config".into();
        lower.file.value.as_mut().unwrap().bytes = inherited.as_bytes().to_vec();
        config.lower_layers.push(lower);
        assert!(
            plan_config_patch(&config, &change)
                .unwrap_err()
                .to_string()
                .contains("selector conflicts with an inherited layer")
        );
    }
}

#[test]
fn removing_a_nested_jsonc_entry_retains_empty_parents_after_manual_edits() {
    let mut config = capture(ConfigFormat::Jsonc, Some("{}"));
    let mut change = key(&["managed", "child"], Some(json!(true)));
    let patch = plan_config_patch(&config, &change).unwrap();
    apply_capture(&mut config, patch);
    expect_owned(&config, &mut change);
    config.target.file.value.as_mut().unwrap().bytes =
        b"{\"managed\":{\"child\":true},/* added */\"manual\":2}".to_vec();
    change.desired = None;
    let patch = plan_config_patch(&config, &change).unwrap();
    assert_eq!(
        patch.file.after.unwrap().bytes,
        b"{\"managed\":{},/* added */\"manual\":2}"
    );
    assert!(patch.ownership_after.is_none());
}
