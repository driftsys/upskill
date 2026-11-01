//! Native inventory metadata is not effective activation or qualification proof.
#![cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::Path;
use upskill::native_delivery::claude::{
    ClaudeInventoryError, ClaudeLookupScope, ClaudePluginInventory,
};
const USER: &[u8] =
    include_bytes!("../docs/technotes/evidence/2026-10-24-claude-managed-root/user-list.stdout");
const PROJECT: &[u8] = include_bytes!(
    "../docs/technotes/evidence/2026-10-24-claude-managed-root/retry-project-list.stdout"
);

#[test]
fn retained_native_user_project_and_synced_metadata_are_preserved_as_claims() {
    let inventory = ClaudePluginInventory::parse("2.1.308", USER).unwrap();
    let entry = &inventory.entries()[0];
    assert_eq!(
        entry.id,
        "upskill-q6-66d31ae858b12d98@upskill-q6-market-66d31ae858b12d98"
    );
    assert_eq!(entry.version, "0.0.0");
    assert_eq!(entry.scope, "user");
    assert!(entry.enabled);
    assert_eq!(
        entry.install_path,
        Path::new(
            "$PROFILE/plugins/cache/upskill-q6-market-66d31ae858b12d98/upskill-q6-66d31ae858b12d98/0.0.0"
        )
    );
    assert_eq!(
        entry.read_from_folder.as_deref(),
        Some(Path::new(
            "$FIXTURE/delivery/packages/upskill-q6-66d31ae858b12d98"
        ))
    );
    assert_eq!(entry.folder_version.as_deref(), Some("0.0.0"));
    assert_eq!(entry.project_enabled, Some(false));
    assert!(entry.project_path.is_none());
    assert!(
        inventory
            .find(&entry.id, ClaudeLookupScope::User)
            .unwrap()
            .is_some()
    );
    let inventory = ClaudePluginInventory::parse("2.1.308", PROJECT).unwrap();
    assert_eq!(inventory.entries().len(), 6);
    assert_eq!(inventory.entries()[0].scope, "project");
    assert_eq!(
        inventory.entries()[0].project_path.as_deref(),
        Some(Path::new("$FIXTURE/project"))
    );
    assert_eq!(inventory.entries()[0].project_enabled, Some(true));
    assert_eq!(inventory.entries()[1].scope, "synced");
    // Sanitized receipt placeholders cannot identify a real absolute project.
    assert_eq!(
        inventory
            .find(
                &inventory.entries()[0].id,
                ClaudeLookupScope::Project(Path::new("/project"))
            )
            .unwrap_err(),
        ClaudeInventoryError::AmbiguousScope
    );
}
fn row(scope: &str) -> serde_json::Value {
    let mut row = serde_json::json!({"id":"owned@catalog","version":"0.0.0-rabc","scope":scope,"enabled":true,"installPath":"/cache/version","readFromFolder":"/exports/revision","folderVersion":"0.0.0-rabc"});
    if scope == "project" {
        row["projectPath"] = "/project".into();
        row["projectEnabled"] = false.into();
    }
    row
}
#[test]
fn lookup_uses_exact_identity_scope_and_project_without_first_found_adoption() {
    let user = row("user");
    let project = row("project");
    let mut outside = project.clone();
    outside["projectPath"] = "/outside".into();
    let bytes = serde_json::to_vec(&[user.clone(), project.clone(), outside]).unwrap();
    let inventory = ClaudePluginInventory::parse("2.1.308", &bytes).unwrap();
    assert_eq!(
        inventory
            .find("owned@catalog", ClaudeLookupScope::User)
            .unwrap()
            .unwrap()
            .scope,
        "user"
    );
    let entry = inventory
        .find(
            "owned@catalog",
            ClaudeLookupScope::Project(Path::new("/project")),
        )
        .unwrap()
        .unwrap();
    assert_eq!(entry.project_enabled, Some(false));
    assert!(
        inventory
            .find(
                "owned@catalog",
                ClaudeLookupScope::Project(Path::new("/absent"))
            )
            .unwrap()
            .is_none()
    );
    assert!(
        inventory
            .find("owned@other", ClaudeLookupScope::User)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        inventory
            .find(
                "owned@catalog",
                ClaudeLookupScope::Project(Path::new("relative"))
            )
            .unwrap_err(),
        ClaudeInventoryError::AmbiguousScope
    );
    for (rows, scope, expected) in [
        (
            vec![user.clone(), user],
            ClaudeLookupScope::User,
            ClaudeInventoryError::AmbiguousEntry,
        ),
        (
            vec![project.clone(), project],
            ClaudeLookupScope::Project(Path::new("/project")),
            ClaudeInventoryError::AmbiguousEntry,
        ),
        (
            vec![row("user"), row("managed")],
            ClaudeLookupScope::User,
            ClaudeInventoryError::AmbiguousScope,
        ),
    ] {
        let inventory =
            ClaudePluginInventory::parse("2.1.308", &serde_json::to_vec(&rows).unwrap()).unwrap();
        assert_eq!(
            inventory.find("owned@catalog", scope).unwrap_err(),
            expected
        );
    }
}
#[test]
fn malformed_incomplete_duplicate_or_wrong_version_input_never_becomes_empty_inventory() {
    for bytes in [
        b"".as_slice(),
        b"{}",
        b"[null]",
        b"not-json",
        b"[] trailing",
        b"[{}]",
        &[0xff],
    ] {
        assert_eq!(
            ClaudePluginInventory::parse("2.1.308", bytes).unwrap_err(),
            ClaudeInventoryError::Malformed
        );
    }
    let valid = serde_json::to_string(&row("user")).unwrap();
    for duplicate in [
        r#""id":"first","#,
        r#""\u0069d":"first","#,
        r#""ignored":{"key":1,"key":2},"#,
    ] {
        let bytes = format!("[{{{duplicate}{}]", &valid[1..]);
        assert_eq!(
            ClaudePluginInventory::parse("2.1.308", bytes.as_bytes()).unwrap_err(),
            ClaudeInventoryError::Malformed
        );
    }
    for field in [
        "id",
        "version",
        "scope",
        "enabled",
        "installPath",
        "projectPath",
        "projectEnabled",
    ] {
        let mut malformed = row("project");
        malformed.as_object_mut().unwrap().remove(field);
        assert_eq!(
            ClaudePluginInventory::parse("2.1.308", &serde_json::to_vec(&[malformed]).unwrap())
                .unwrap_err(),
            ClaudeInventoryError::Malformed,
            "missing {field}"
        );
    }
    assert_eq!(
        ClaudePluginInventory::parse("2.1.309", b"[]").unwrap_err(),
        ClaudeInventoryError::UnsupportedVersion
    );
    assert_eq!(
        ClaudePluginInventory::parse("2.1.308", &vec![b' '; 1_048_577]).unwrap_err(),
        ClaudeInventoryError::TooLarge
    );
    assert!(
        ClaudePluginInventory::parse("2.1.308", b"[]")
            .unwrap()
            .entries()
            .is_empty()
    );
}
#[test]
fn unrecognized_fields_and_parse_errors_do_not_export_native_secrets() {
    let mut value = row("user");
    value["unknown"] = serde_json::json!({"credential":"private-native-canary"});
    let inventory =
        ClaudePluginInventory::parse("2.1.308", &serde_json::to_vec(&[value]).unwrap()).unwrap();
    assert_eq!(
        format!("{inventory:?}"),
        "ClaudePluginInventory { entries: 1, .. }"
    );
    assert_eq!(
        format!("{:?}", inventory.entries()[0]),
        "ClaudePluginEntry { .. }"
    );
    let error =
        ClaudePluginInventory::parse("2.1.308", br#"[{"id":{"secret":"private-native-canary"}}]"#)
            .unwrap_err();
    assert_eq!(error, ClaudeInventoryError::Malformed);
    assert_eq!(format!("{error:?}"), "Malformed");
    assert!(!error.to_string().contains("private-native-canary"));
}

#[test]
fn wrong_scalar_types_empty_values_and_nul_values_do_not_become_usable_metadata() {
    for field in [
        "id",
        "version",
        "scope",
        "installPath",
        "readFromFolder",
        "folderVersion",
        "projectPath",
    ] {
        for bad in [serde_json::json!(""), serde_json::json!("bad\0value")] {
            let mut malformed = row("project");
            malformed[field] = bad;
            assert_eq!(
                ClaudePluginInventory::parse("2.1.308", &serde_json::to_vec(&[malformed]).unwrap())
                    .unwrap_err(),
                ClaudeInventoryError::Malformed,
                "field {field}"
            );
        }
    }
    for field in ["enabled", "projectEnabled"] {
        for bad in [serde_json::json!("true"), serde_json::Value::Null] {
            let mut malformed = row("project");
            malformed[field] = bad;
            assert_eq!(
                ClaudePluginInventory::parse("2.1.308", &serde_json::to_vec(&[malformed]).unwrap())
                    .unwrap_err(),
                ClaudeInventoryError::Malformed
            );
        }
    }
}
