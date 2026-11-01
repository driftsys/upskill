//! Fixed native metadata commands, exercised only with owned fake executables.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/native_process.rs"]
mod fixture;
use fixture::*;
use std::sync::atomic::AtomicBool;
use upskill::native_delivery::claude::{
    ClaudeInventoryError, ClaudeInventoryReadError, read_plugin_inventory,
};
fn fake(root: &std::path::Path) -> upskill::native_delivery::process::NativeCommandSpec {
    let mut spec = script(
        root,
        r#"printf '%s\n' "$*" >> "$CALLS"
case "$*" in
  --version) printf '%s\n' "$VERSION"; exit 0;;
  'plugin list --json')
    case "$MODE" in
      refuse) printf 'private-native-stderr' >&2; exit 17;;
      timeout) while :; do :; done;;
      overflow) while :; do printf 'unbounded-native-output'; done;;
      change-executable) printf '#!/bin/sh\nexit 0\n' > "$0"; printf '[]'; exit 0;;
    esac
    /bin/cat "$INVENTORY";;
  *) printf 'unexpected arguments' >&2; exit 3;;
esac"#,
    );
    std::fs::write(root.join("inventory.json"), b"[]").unwrap();
    for (name, file) in [("CALLS", "calls"), ("INVENTORY", "inventory.json")] {
        spec.environment
            .insert(name.into(), root.join(file).into_os_string());
    }
    spec.environment
        .insert("VERSION".into(), "2.1.308 (Claude Code)".into());
    spec
}
#[test]
fn reader_binds_fixed_version_then_inventory_to_the_same_qualified_cwd() {
    with_scope(|root, scope, guard| {
        let spec = fake(root);
        std::fs::write(root.join("inventory.json"), br#"[{"id":"owned@catalog","version":"installed-old","scope":"user","enabled":false,"installPath":"/cache/old","folderVersion":"folder-new"}]"#).unwrap();
        let capture = read_plugin_inventory(scope, guard, &spec, &AtomicBool::new(false)).unwrap();
        assert_eq!(capture.inventory.entries().len(), 1);
        let entry = &capture.inventory.entries()[0];
        assert_eq!(entry.id, "owned@catalog");
        assert_eq!(entry.version, "installed-old");
        assert_eq!(entry.folder_version.as_deref(), Some("folder-new"));
        assert!(!entry.enabled);
        assert_eq!(capture.cwd, scope.binding().unwrap());
        assert_eq!(
            std::fs::read(root.join("calls")).unwrap(),
            b"--version\nplugin list --json\n"
        );
        assert_eq!(
            format!("{capture:?}"),
            "ClaudeInventoryCapture { entries: 1, .. }"
        );
        assert!(spec.arguments.is_empty());
    });
}
#[test]
fn unsupported_version_or_extra_arguments_stop_before_inventory() {
    with_scope(|root, scope, guard| {
        let mut spec = fake(root);
        spec.arguments = vec!["--print".into(), "must-not-run".into()];
        assert!(matches!(
            read_plugin_inventory(scope, guard, &spec, &AtomicBool::new(false)),
            Err(ClaudeInventoryReadError::UnexpectedArguments)
        ));
        assert!(!root.join("calls").exists());
        spec.arguments.clear();
        spec.environment.insert(
            "VERSION".into(),
            "unsupported-private-native-version".into(),
        );
        let error =
            read_plugin_inventory(scope, guard, &spec, &AtomicBool::new(false)).unwrap_err();
        assert!(matches!(error, ClaudeInventoryReadError::VersionMismatch));
        assert!(!format!("{error:?}").contains("private-native"));
        assert_eq!(std::fs::read(root.join("calls")).unwrap(), b"--version\n");
    });
}
#[test]
fn refusal_timeout_overflow_and_malformed_data_never_produce_empty_inventory() {
    for mode in [
        "refuse",
        "timeout",
        "overflow",
        "malformed",
        "change-executable",
    ] {
        with_scope(|root, scope, guard| {
            let mut spec = fake(root);
            spec.environment.insert("MODE".into(), mode.into());
            if mode == "timeout" {
                spec.timeout = std::time::Duration::from_secs(2);
            }
            if mode == "overflow" {
                spec.output_limit = 64;
            }
            if mode == "malformed" {
                std::fs::write(
                    root.join("inventory.json"),
                    b"private-native-malformed-json",
                )
                .unwrap();
            }
            let error =
                read_plugin_inventory(scope, guard, &spec, &AtomicBool::new(false)).unwrap_err();
            if mode == "malformed" {
                assert!(matches!(
                    error,
                    ClaudeInventoryReadError::Inventory(ClaudeInventoryError::Malformed)
                ));
            } else {
                assert!(matches!(
                    error,
                    ClaudeInventoryReadError::UnresolvedTransport
                ));
            }
            assert!(!format!("{error:?}").contains("private-native"));
            assert_eq!(
                std::fs::read(root.join("calls")).unwrap(),
                b"--version\nplugin list --json\n"
            );
        });
    }
}
#[test]
fn caller_cancellation_and_wrong_guard_cannot_invoke_the_client() {
    with_scope(|root, scope, guard| {
        let spec = fake(root);
        assert!(matches!(
            read_plugin_inventory(scope, guard, &spec, &AtomicBool::new(true)),
            Err(ClaudeInventoryReadError::Invocation(
                upskill::native_delivery::process::NativeProcessError::CancelledBeforeLaunch
            ))
        ));
        assert!(!root.join("calls").exists());
        with_scope(|_, _, other_guard| {
            assert!(matches!(
                read_plugin_inventory(scope, other_guard, &spec, &AtomicBool::new(false)),
                Err(ClaudeInventoryReadError::Invocation(
                    upskill::native_delivery::process::NativeProcessError::UnsafeScope
                ))
            ));
        });
        assert!(!root.join("calls").exists());
    });
}
