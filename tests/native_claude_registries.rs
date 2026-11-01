//! Captured native metadata shapes do not establish native ownership or activation.
#![cfg(any(target_os = "linux", target_os = "macos"))]
use serde_json::{Value, json};
use std::path::Path;
use upskill::native_delivery::claude::{
    ClaudeInstalledRegistry, ClaudeInventoryError, ClaudeKnownMarketplaces, ClaudeLookupScope,
    ClaudeMarketplaceInventory,
};
const PIN: &str = "2.1.308";
fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn receipts() -> Value {
    serde_json::from_slice(include_bytes!(
        "../docs/technotes/evidence/2026-11-01-claude-local-catalog-contract/summary.json"
    ))
    .unwrap()
}
fn marketplace() -> Value {
    json!({"name":"owned", "source":"directory", "path":"/fixture/catalog", "installLocation":"/fixture/catalog"})
}
fn known() -> Value {
    json!({"owned":{"source":{"source":"directory","path":"/fixture/catalog"},"installLocation":"/fixture/catalog"}})
}
fn installed() -> Value {
    json!({"version":2,"plugins":{"owned@owned":[{"scope":"project","installPath":"/fixture/cache","version":"0.0.0-r123","projectPath":"/fixture/project"}]}})
}

// Catches confusing the three captured wire shapes, discarding metadata or
// treating the installed version as a live folder version.
#[test]
fn retained_local_catalog_shapes_decode_as_distinct_metadata_views() {
    let packet = receipts();
    let command = packet["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["index"] == 8)
        .unwrap();
    let listed =
        ClaudeMarketplaceInventory::parse(PIN, &bytes(&command["native_json_normalized"])).unwrap();
    let name = "upskill-abf246364bf8157a4733c703b56a9abf148788fb747d0f5a92bccb22b571c4b0";
    let row = &listed.entries()[0];
    assert_eq!(row.name, name);
    assert_eq!(row.source, "directory");
    assert_eq!(
        row.path.as_deref(),
        Some(Path::new(
            "$PACKET/project/.upskill/exports/claude/native-contract-probe"
        ))
    );
    assert_eq!(
        row.install_location,
        Path::new("$PACKET/project/.upskill/exports/claude/native-contract-probe")
    );
    let sample = &packet["registry_samples_normalized"]["6"];
    let registered = ClaudeKnownMarketplaces::parse(
        PIN,
        &bytes(&sample["home/.claude/plugins/known_marketplaces.json"]),
    )
    .unwrap();
    assert_eq!(registered.entries().len(), 1);
    assert_eq!(registered.entries()[0].name, name);
    assert_eq!(registered.entries()[0].path, row.path);
    let registry = ClaudeInstalledRegistry::parse(
        PIN,
        &bytes(&sample["home/.claude/plugins/installed_plugins.json"]),
    )
    .unwrap();
    let selector = format!("{name}@{name}");
    let entry = &registry.entries()[&selector][0];
    assert_eq!(entry.scope, "project");
    assert_eq!(
        entry.version,
        "0.0.0-r95bde28c505242a7af12e31433e17c076c021c26effdc600e609d91ca36a0664"
    );
    assert_eq!(
        entry.project_path.as_deref(),
        Some(Path::new("$PACKET/project"))
    );
    assert!(
        entry
            .install_path
            .to_str()
            .unwrap()
            .contains("/plugins/cache/")
    );
    assert_eq!(
        registry
            .find(
                &selector,
                ClaudeLookupScope::Project(Path::new("/fixture/project"))
            )
            .unwrap_err(),
        ClaudeInventoryError::AmbiguousScope
    );
}

// Catches prefix selection, first-match adoption and treating a conflicting
// same-name source as missing. Both independent directory claims must match.
#[test]
fn directory_lookup_requires_exact_unique_name_source_and_both_paths() {
    let list = ClaudeMarketplaceInventory::parse(PIN, &bytes(&json!([marketplace()]))).unwrap();
    let map = ClaudeKnownMarketplaces::parse(PIN, &bytes(&known())).unwrap();
    assert!(
        list.find_directory("owned", Path::new("/fixture/catalog"))
            .unwrap()
            .is_some()
    );
    assert!(
        map.find_directory("owned", Path::new("/fixture/catalog"))
            .unwrap()
            .is_some()
    );
    assert!(
        list.find_directory("ow", Path::new("/fixture/catalog"))
            .unwrap()
            .is_none()
    );
    assert!(
        map.find_directory("absent", Path::new("/fixture/catalog"))
            .unwrap()
            .is_none()
    );
    for bad in ["relative", "/fixture/../catalog", ""] {
        assert_eq!(
            list.find_directory("owned", Path::new(bad)).unwrap_err(),
            ClaudeInventoryError::AmbiguousSource
        );
        assert_eq!(
            map.find_directory("owned", Path::new(bad)).unwrap_err(),
            ClaudeInventoryError::AmbiguousSource
        );
    }
    let duplicated =
        ClaudeMarketplaceInventory::parse(PIN, &bytes(&json!([marketplace(), marketplace()])))
            .unwrap();
    assert_eq!(
        duplicated
            .find_directory("owned", Path::new("/fixture/catalog"))
            .unwrap_err(),
        ClaudeInventoryError::AmbiguousEntry
    );
    for field in ["path", "installLocation", "source"] {
        let mut row = marketplace();
        row[field] = json!(if field == "source" {
            "github"
        } else {
            "/other"
        });
        let list = ClaudeMarketplaceInventory::parse(PIN, &bytes(&json!([row]))).unwrap();
        assert_eq!(
            list.find_directory("owned", Path::new("/fixture/catalog"))
                .unwrap_err(),
            ClaudeInventoryError::AmbiguousSource
        );
        let mut map = known();
        if field == "installLocation" {
            map["owned"][field] = json!("/other");
        } else {
            map["owned"]["source"][if field == "source" { "source" } else { "path" }] =
                json!(if field == "source" {
                    "github"
                } else {
                    "/other"
                });
        }
        let map = ClaudeKnownMarketplaces::parse(PIN, &bytes(&map)).unwrap();
        assert_eq!(
            map.find_directory("owned", Path::new("/fixture/catalog"))
                .unwrap_err(),
            ClaudeInventoryError::AmbiguousSource
        );
    }
    let unrelated = json!({"other":{"source":{"source":"github","repo":"secret-canary"},"installLocation":"/elsewhere"}});
    let map = ClaudeKnownMarketplaces::parse(PIN, &bytes(&unrelated)).unwrap();
    assert!(
        map.find_directory("owned", Path::new("/fixture/catalog"))
            .unwrap()
            .is_none()
    );
    assert!(!format!("{map:?}").contains("secret-canary"));
}

// Catches schema coercion, loss of empty selector keys, project/user conflation,
// and first-match selection in a registry that can hold several scope rows.
#[test]
fn installed_registry_preserves_empty_keys_and_scoped_conflicts() {
    let mut value = installed();
    value["plugins"]["empty@empty"] = json!([]);
    value["plugins"]["owned@owned"]
        .as_array_mut()
        .unwrap()
        .push(json!({"scope":"user","installPath":"/user/cache","version":"user-version"}));
    let registry = ClaudeInstalledRegistry::parse(PIN, &bytes(&value)).unwrap();
    assert_eq!(registry.entries().len(), 2);
    assert!(registry.entries()["empty@empty"].is_empty());
    assert_eq!(
        registry
            .find("owned@owned", ClaudeLookupScope::User)
            .unwrap()
            .unwrap()
            .version,
        "user-version"
    );
    assert_eq!(
        registry
            .find(
                "owned@owned",
                ClaudeLookupScope::Project(Path::new("/fixture/project"))
            )
            .unwrap()
            .unwrap()
            .install_path,
        Path::new("/fixture/cache")
    );
    assert!(
        registry
            .find(
                "owned@owned",
                ClaudeLookupScope::Project(Path::new("/different"))
            )
            .unwrap()
            .is_none()
    );
    assert!(
        registry
            .find("owned", ClaudeLookupScope::User)
            .unwrap()
            .is_none()
    );
    assert!(
        registry
            .find("empty@empty", ClaudeLookupScope::User)
            .unwrap()
            .is_none()
    );
    for bad in ["relative", "/fixture/../project"] {
        assert_eq!(
            registry
                .find("owned@owned", ClaudeLookupScope::Project(Path::new(bad)))
                .unwrap_err(),
            ClaudeInventoryError::AmbiguousScope
        );
    }
    let row = value["plugins"]["owned@owned"][0].clone();
    value["plugins"]["owned@owned"]
        .as_array_mut()
        .unwrap()
        .push(row);
    let registry = ClaudeInstalledRegistry::parse(PIN, &bytes(&value)).unwrap();
    assert_eq!(
        registry
            .find(
                "owned@owned",
                ClaudeLookupScope::Project(Path::new("/fixture/project"))
            )
            .unwrap_err(),
        ClaudeInventoryError::AmbiguousEntry
    );
    value["plugins"]["owned@owned"][0]["scope"] = json!("synced");
    let registry = ClaudeInstalledRegistry::parse(PIN, &bytes(&value)).unwrap();
    assert_eq!(
        registry
            .find("owned@owned", ClaudeLookupScope::User)
            .unwrap_err(),
        ClaudeInventoryError::AmbiguousScope
    );
    assert!(
        registry
            .find("unrelated", ClaudeLookupScope::User)
            .unwrap()
            .is_none()
    );
    for version in [json!(1), json!(3), json!("2"), json!(null), json!(2.0)] {
        let mut value = installed();
        value["version"] = version;
        assert_eq!(
            ClaudeInstalledRegistry::parse(PIN, &bytes(&value)).unwrap_err(),
            ClaudeInventoryError::UnsupportedRegistryVersion
        );
    }
}

// Catches bypasses of version/size/UTF-8/recursive duplicate-key validation and
// malformed empty/missing fields being mistaken for an empty registry.
#[test]
fn malformed_capture_is_rejected_without_raw_input_in_errors_or_debug() {
    type Decoder = fn(&str, &[u8]) -> Result<(), ClaudeInventoryError>;
    let decoders: [Decoder; 3] = [
        |pin, b| ClaudeMarketplaceInventory::parse(pin, b).map(|_| ()),
        |pin, b| ClaudeKnownMarketplaces::parse(pin, b).map(|_| ()),
        |pin, b| ClaudeInstalledRegistry::parse(pin, b).map(|_| ()),
    ];
    for decode in decoders {
        assert_eq!(
            decode("2.1.309", b"[]").unwrap_err(),
            ClaudeInventoryError::UnsupportedVersion
        );
        assert_eq!(
            decode(PIN, &vec![b' '; 1024 * 1024 + 1]).unwrap_err(),
            ClaudeInventoryError::TooLarge
        );
        for raw in [
            b"\xff".as_slice(),
            br#"{"secret-canary": "x""#,
            br#"{"ignored":{"x":1,"\u0078":2}}"#,
        ] {
            let err = decode(PIN, raw).unwrap_err();
            assert_eq!(err, ClaudeInventoryError::Malformed);
            assert!(!format!("{err:?} {err}").contains("secret-canary"));
        }
    }
    for field in ["name", "source", "path", "installLocation"] {
        for bad in [Value::Null, json!(""), json!("bad\u{0}path"), json!(42)] {
            let mut row = marketplace();
            row[field] = bad;
            assert!(
                ClaudeMarketplaceInventory::parse(PIN, &bytes(&json!([row]))).is_err(),
                "{field}"
            );
        }
    }
    for field in ["scope", "installPath", "version", "projectPath"] {
        for bad in [Value::Null, json!(""), json!("bad\u{0}path"), json!(42)] {
            let mut value = installed();
            value["plugins"]["owned@owned"][0][field] = bad;
            assert!(
                ClaudeInstalledRegistry::parse(PIN, &bytes(&value)).is_err(),
                "{field}"
            );
        }
    }
    for raw in [
        br#"{"owned":{"source":{"source":"directory"},"installLocation":"/a"}}"#.as_slice(),
        br#"{"":{"source":{"source":"github"},"installLocation":"/a"}}"#,
    ] {
        assert!(ClaudeKnownMarketplaces::parse(PIN, raw).is_err());
    }
    let value = json!({"version":2,"plugins":{"":[]}});
    assert!(ClaudeInstalledRegistry::parse(PIN, &bytes(&value)).is_err());
    let mut value = installed();
    value["plugins"]["owned@owned"][0]["unknown"] = json!("secret-canary");
    let registry = ClaudeInstalledRegistry::parse(PIN, &bytes(&value)).unwrap();
    assert!(
        !format!("{registry:?} {:?}", registry.entries()["owned@owned"][0])
            .contains("secret-canary")
    );
    let empty = ClaudeInstalledRegistry::parse(PIN, br#"{"version":2,"plugins":{}}"#).unwrap();
    assert!(empty.entries().is_empty()); // Parsed emptiness does not encode file absence.
    assert!(
        ClaudeMarketplaceInventory::parse(PIN, b"[]")
            .unwrap()
            .entries()
            .is_empty()
    );
    assert!(
        ClaudeKnownMarketplaces::parse(PIN, b"{}")
            .unwrap()
            .entries()
            .is_empty()
    );
}

// Each input is otherwise valid for its decoder. This catches replacing the
// recursive unique-key parser with serde_json's last-key-wins decoder.
#[test]
fn decoded_duplicate_keys_in_ignored_fields_are_not_discarded_silently() {
    let list = br#"[{"name":"owned","source":"directory","path":"/a","installLocation":"/a","ignored":{"x":1,"\u0078":2}}]"#;
    let map = br#"{"owned":{"source":{"source":"directory","path":"/a","ignored":{"x":1,"\u0078":2}},"installLocation":"/a"}}"#;
    let installed = br#"{"version":2,"plugins":{"owned@owned":[{"scope":"user","version":"v1","installPath":"/cache","ignored":{"x":1,"\u0078":2}}]}}"#;
    assert_eq!(
        ClaudeMarketplaceInventory::parse(PIN, list).unwrap_err(),
        ClaudeInventoryError::Malformed
    );
    assert_eq!(
        ClaudeKnownMarketplaces::parse(PIN, map).unwrap_err(),
        ClaudeInventoryError::Malformed
    );
    assert_eq!(
        ClaudeInstalledRegistry::parse(PIN, installed).unwrap_err(),
        ClaudeInventoryError::Malformed
    );
    let duplicate_name = br#"{"owned":{"source":{"source":"github"},"installLocation":"/a"},"owned":{"source":{"source":"directory","path":"/a"},"installLocation":"/a"}}"#;
    assert_eq!(
        ClaudeKnownMarketplaces::parse(PIN, duplicate_name).unwrap_err(),
        ClaudeInventoryError::Malformed
    );
    let mut at_limit = b"[]".to_vec();
    at_limit.resize(1024 * 1024, b' ');
    assert!(
        ClaudeMarketplaceInventory::parse(PIN, &at_limit)
            .unwrap()
            .entries()
            .is_empty()
    );
}

// Catches Debug accidentally deriving or printing retained selector/path fields.
#[test]
fn debug_redacts_allowlisted_fields_as_well_as_ignored_fields() {
    let row = json!({"name":"private-name-canary","source":"directory","path":"/private-path-canary","installLocation":"/private-path-canary"});
    let list = ClaudeMarketplaceInventory::parse(PIN, &bytes(&json!([row]))).unwrap();
    let map = ClaudeKnownMarketplaces::parse(PIN,&bytes(&json!({"private-name-canary":{"source":{"source":"directory","path":"/private-path-canary"},"installLocation":"/private-path-canary"}}))).unwrap();
    let registry = ClaudeInstalledRegistry::parse(PIN,&bytes(&json!({"version":2,"plugins":{"private-name-canary":[{"scope":"project","projectPath":"/private-project-canary","version":"private-version-canary","installPath":"/private-path-canary"}]}}))).unwrap();
    let rendered = format!(
        "{list:?} {:?} {map:?} {:?} {registry:?} {:?}",
        list.entries()[0],
        map.entries()[0],
        registry.entries()["private-name-canary"][0]
    );
    for canary in [
        "private-name-canary",
        "private-version-canary",
        "private-path-canary",
        "private-project-canary",
    ] {
        assert!(!rendered.contains(canary));
    }
}

// Catches reconstructing a selector with the marketplace in the plugin role,
// or treating either side of plugin@marketplace as sufficient for a match.
#[test]
fn installed_lookup_keeps_plugin_and_marketplace_roles_distinct() {
    let value = json!({"version":2,"plugins":{
        "alpha@catalog":[{"scope":"user","version":"alpha-main","installPath":"/cache/alpha-main"}],
        "beta@catalog":[{"scope":"user","version":"beta-main","installPath":"/cache/beta-main"}],
        "alpha@other":[{"scope":"user","version":"alpha-other","installPath":"/cache/alpha-other"}],
        "catalog@catalog":[{"scope":"user","version":"catalog-trap","installPath":"/cache/catalog-trap"}]
    }});
    let registry = ClaudeInstalledRegistry::parse(PIN, &bytes(&value)).unwrap();
    for (selector, version, path) in [
        ("alpha@catalog", "alpha-main", "/cache/alpha-main"),
        ("beta@catalog", "beta-main", "/cache/beta-main"),
        ("alpha@other", "alpha-other", "/cache/alpha-other"),
        ("catalog@catalog", "catalog-trap", "/cache/catalog-trap"),
    ] {
        let row = registry
            .find(selector, ClaudeLookupScope::User)
            .unwrap()
            .unwrap();
        assert_eq!(row.version, version);
        assert_eq!(row.install_path, Path::new(path));
    }
    for absent in ["beta@other", "catalog@alpha", "alpha", "alpha@cat"] {
        assert!(
            registry
                .find(absent, ClaudeLookupScope::User)
                .unwrap()
                .is_none(),
            "{absent}"
        );
    }
}

// Catches ancestor matching in place of exact project-path equality.
#[test]
fn installed_lookup_distinguishes_parent_nested_and_unregistered_projects() {
    let value = json!({"version":2,"plugins":{"alpha@catalog":[
        {"scope":"project","projectPath":"/fixture/project","version":"parent-version","installPath":"/cache/parent"},
        {"scope":"project","projectPath":"/fixture/project/nested","version":"nested-version","installPath":"/cache/nested"}
    ]}});
    let registry = ClaudeInstalledRegistry::parse(PIN, &bytes(&value)).unwrap();
    for (project, version, path) in [
        ("/fixture/project", "parent-version", "/cache/parent"),
        ("/fixture/project/nested", "nested-version", "/cache/nested"),
    ] {
        let row = registry
            .find(
                "alpha@catalog",
                ClaudeLookupScope::Project(Path::new(project)),
            )
            .unwrap()
            .unwrap();
        assert_eq!(row.version, version);
        assert_eq!(row.install_path, Path::new(path));
    }
    for absent in [
        "/fixture/project/nested/deeper",
        "/fixture",
        "/fixture/project-sibling",
    ] {
        assert!(
            registry
                .find(
                    "alpha@catalog",
                    ClaudeLookupScope::Project(Path::new(absent))
                )
                .unwrap()
                .is_none(),
            "{absent}"
        );
    }
}

// Catches a duplicate guard that rejects only project rows or adopts the last
// matching user row. Other scopes under that selector remain independent.
#[test]
fn installed_lookup_rejects_duplicate_user_rows_without_affecting_project_lookup() {
    let value = json!({"version":2,"plugins":{"alpha@catalog":[
        {"scope":"user","version":"first-user","installPath":"/cache/first-user"},
        {"scope":"project","projectPath":"/fixture/project","version":"project-version","installPath":"/cache/project"},
        {"scope":"user","version":"second-user","installPath":"/cache/second-user"}
    ]}});
    let registry = ClaudeInstalledRegistry::parse(PIN, &bytes(&value)).unwrap();
    assert_eq!(
        registry
            .find("alpha@catalog", ClaudeLookupScope::User)
            .unwrap_err(),
        ClaudeInventoryError::AmbiguousEntry
    );
    let project = registry
        .find(
            "alpha@catalog",
            ClaudeLookupScope::Project(Path::new("/fixture/project")),
        )
        .unwrap()
        .unwrap();
    assert_eq!(project.version, "project-version");
    assert_eq!(project.install_path, Path::new("/cache/project"));
}
