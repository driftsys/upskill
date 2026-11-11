//! Collector projection must reject wire drift without retaining native values.
use upskill::native_delivery::copilot::probe::{
    CopilotInventoryKind, CopilotProbeError, summarize_inventory,
};

const PLUGIN: &[u8] = br#"[{"name":"secret-name","marketplace":"secret-market","source":"/secret/path","version":"secret-version","enabled":true}]"#;
const MARKETPLACE: &[u8] =
    br#"[{"name":"secret-market","source":"/secret/path","isDefault":false}]"#;

#[test]
fn collector_retains_counts_without_native_scalar_values() {
    for (kind, bytes, expected) in [
        (
            CopilotInventoryKind::Plugins,
            PLUGIN,
            r#"{"kind":"plugins","entries":1}"#,
        ),
        (
            CopilotInventoryKind::Marketplaces,
            MARKETPLACE,
            r#"{"kind":"marketplaces","entries":1}"#,
        ),
    ] {
        let summary = summarize_inventory("1.0.107", kind, bytes).unwrap();
        assert_eq!(serde_json::to_string(&summary).unwrap(), expected);
        assert!(!format!("{summary:?}").contains("secret"));
        assert_eq!(summary.entries(), 1);
        let empty = summarize_inventory("1.0.107", kind, b"[]").unwrap();
        assert_eq!(empty.entries(), 0);
    }
}

#[test]
fn collector_rejects_schema_changes_in_every_row() {
    for bytes in [
        br#"{}"#.as_slice(), b"null", b"[null]", b"[{}]",
        br#"[{"name":"a","marketplace":"b","source":"c","version":"d","enabled":"true"}]"#,
        br#"[{"name":"a","marketplace":"b","source":"c","version":"d","enabled":true,"secret-extra":"secret-value"}]"#,
        br#"[{"name":"a","marketplace":"b","source":"c","version":"d","enabled":true},{}]"#,
        br#"[{"name":"a","marketplace":"b","source":"live","version":"d","enabled":true,"installedFrom":4}]"#,
    ] {
        assert_eq!(summarize_inventory("1.0.107", CopilotInventoryKind::Plugins, bytes), Err(CopilotProbeError::UnsupportedShape));
    }
    for bytes in [
        PLUGIN,
        br#"[{"name":"a","source":4,"isDefault":true}]"#,
        br#"[{"name":"a","source":"b"}]"#,
        br#"[{"name":"a","source":"b","isDefault":true,"extra":null}]"#,
    ] {
        assert_eq!(
            summarize_inventory("1.0.107", CopilotInventoryKind::Marketplaces, bytes),
            Err(CopilotProbeError::UnsupportedShape)
        );
    }
}

#[test]
fn collector_rejects_duplicate_decoded_keys_and_trailing_input() {
    for bytes in [
        br#"[{"name":"a","\u006eame":"b","marketplace":"c","source":"d","version":"e","enabled":true}]"#.as_slice(),
        br#"[{"ignored":{"x":1,"\u0078":2}}]"#,
        b"[] []", b"[", b"\xff",
    ] {
        assert_eq!(summarize_inventory("1.0.107", CopilotInventoryKind::Plugins, bytes), Err(CopilotProbeError::Malformed));
    }
}

#[test]
fn collector_enforces_version_and_byte_limits_before_parsing() {
    for version in ["1.0.4", "1.0.105", "1.0.106", "1.0.107\n", ""] {
        assert_eq!(
            summarize_inventory(version, CopilotInventoryKind::Plugins, PLUGIN),
            Err(CopilotProbeError::UnsupportedVersion)
        );
    }
    let mut exact = b"[]".to_vec();
    exact.resize(1024 * 1024, b' ');
    assert_eq!(
        summarize_inventory("1.0.107", CopilotInventoryKind::Plugins, &exact)
            .unwrap()
            .entries(),
        0
    );
    exact.push(b' ');
    assert_eq!(
        summarize_inventory("1.0.107", CopilotInventoryKind::Plugins, &exact),
        Err(CopilotProbeError::TooLarge)
    );
}

#[test]
fn collector_errors_do_not_echo_input() {
    let error = summarize_inventory(
        "1.0.107",
        CopilotInventoryKind::Plugins,
        br#"secret-authentication-diagnostic"#,
    )
    .unwrap_err();
    assert!(!format!("{error:?}: {error}").contains("secret"));
}

#[test]
fn collector_requires_each_field_type_and_does_not_sample_rows() {
    for (kind, bytes, keys) in [
        (
            CopilotInventoryKind::Plugins,
            PLUGIN,
            &["name", "marketplace", "source", "version", "enabled"][..],
        ),
        (
            CopilotInventoryKind::Marketplaces,
            MARKETPLACE,
            &["name", "source", "isDefault"][..],
        ),
    ] {
        let original: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        for key in keys {
            for replacement in [
                Some(serde_json::Value::Null),
                Some(serde_json::json!(42)),
                Some(serde_json::json!([])),
                Some(serde_json::json!({})),
                Some(serde_json::json!("wrong-field-type")),
                Some(serde_json::json!(true)),
                Some(serde_json::json!(false)),
                None,
            ] {
                if replacement.as_ref().is_some_and(|value| {
                    (original[0][*key].is_string() && value.is_string())
                        || (original[0][*key].is_boolean() && value.is_boolean())
                }) {
                    continue;
                }
                let mut changed = original.clone();
                let fields = changed[0].as_object_mut().unwrap();
                if let Some(value) = replacement {
                    fields.insert((*key).into(), value);
                } else {
                    fields.remove(*key);
                }
                let bytes = serde_json::to_vec(&changed).unwrap();
                assert_eq!(
                    summarize_inventory("1.0.107", kind, &bytes),
                    Err(CopilotProbeError::UnsupportedShape),
                    "{kind:?} {key}"
                );
            }
        }
        let mut rows = vec![original[0].clone(); 100];
        rows.push(serde_json::json!({}));
        assert_eq!(
            summarize_inventory("1.0.107", kind, &serde_json::to_vec(&rows).unwrap()),
            Err(CopilotProbeError::UnsupportedShape)
        );
    }
}

#[test]
fn collector_counts_both_boolean_values_and_all_entries() {
    for (kind, bytes, expected) in [
        (CopilotInventoryKind::Plugins,
         br#"[{"name":"a","marketplace":"b","source":"c","version":"d","enabled":true},{"name":"e","marketplace":"f","source":"g","version":"h","enabled":false}]"#.as_slice(),
         r#"{"kind":"plugins","entries":2}"#),
        (CopilotInventoryKind::Marketplaces,
         br#"[{"name":"a","source":"b","isDefault":true},{"name":"c","source":"d","isDefault":false}]"#.as_slice(),
         r#"{"kind":"marketplaces","entries":2}"#),
    ] {
        let summary = summarize_inventory("1.0.107", kind, bytes).unwrap();
        assert_eq!(summary.entries(), 2);
        assert_eq!(serde_json::to_string(&summary).unwrap(), expected);
    }
}
