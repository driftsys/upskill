//! Marketplace metadata queries exercise owned fake executables only.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/native_process.rs"]
mod fixture;
use fixture::*;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use upskill::native_delivery::claude::{
    ClaudeInventoryError, ClaudeInventoryReadError, read_marketplace_inventory,
};
use upskill::native_delivery::process::NativeCommandSpec;

fn fake(root: &Path) -> NativeCommandSpec {
    let mut spec = script(
        root,
        r#"if [ "$#" -eq 1 ] && [ "$1" = --version ]; then
  query=version
elif [ "$#" -eq 4 ] && [ "$1" = plugin ] && [ "$2" = marketplace ] && [ "$3" = list ] && [ "$4" = --json ]; then
  query=marketplaces
else
  printf 'unexpected argument boundaries' >&2; exit 3
fi
printf '%s\n' "$#" "$@" >> "$ARGUMENTS"
printf '%s\n' "$*" >> "$CALLS"
printf '%s\n' "$HOME" >> "$HOMES"
/bin/pwd -P >> "$CWDS"
case "$query" in
  version)
    printf '%s\n' "$VERSION"
    case "$VERSION_MODE" in
      refuse) printf 'private-version-stderr' >&2; exit 17;;
      oversize) n=0; while [ "$n" -lt 300 ]; do printf ' '; n=$((n+1)); done;;
    esac
    exit 0;;
  marketplaces)
    case "$MODE" in
      refuse) printf 'private-marketplace-stderr' >&2; exit 17;;
      timeout) while :; do :; done;;
      cancel) printf ready > "$READY"; while :; do /bin/sleep 0.01; done;;
      overflow) while :; do printf 'private-marketplace-overflow'; done;;
      change-executable) printf '#!/bin/sh\nexit 0\n' > "$0"; printf '[]'; exit 0;;
    esac
    /bin/cat "$INVENTORY";;
  *) printf 'unexpected arguments' >&2; exit 3;;
esac"#,
    );
    std::fs::write(root.join("marketplaces.json"), b"[]").unwrap();
    for (name, file) in [
        ("CALLS", "calls"),
        ("ARGUMENTS", "arguments"),
        ("HOMES", "homes"),
        ("CWDS", "cwds"),
        ("INVENTORY", "marketplaces.json"),
    ] {
        spec.environment
            .insert(name.into(), root.join(file).into_os_string());
    }
    spec.environment
        .insert("VERSION".into(), "2.1.308 (Claude Code)".into());
    spec
}

// Catches the wrong query family, wrong decoder, changed invocation context,
// or discarding directory claims. Native metadata is not an effect receipt.
#[test]
fn marketplace_reader_uses_fixed_version_and_list_in_one_captured_context() {
    with_scope(|root, scope, guard| {
        let spec = fake(root);
        std::fs::write(root.join("marketplaces.json"),br#"[{"name":"private-catalog","source":"directory","path":"/fixture/catalog","installLocation":"/fixture/catalog"}]"#).unwrap();
        let capture =
            read_marketplace_inventory(scope, guard, &spec, &AtomicBool::new(false)).unwrap();
        let row = capture
            .inventory
            .find_directory("private-catalog", Path::new("/fixture/catalog"))
            .unwrap()
            .unwrap();
        assert_eq!(row.name, "private-catalog");
        assert_eq!(row.path.as_deref(), Some(Path::new("/fixture/catalog")));
        assert_eq!(row.install_location, Path::new("/fixture/catalog"));
        assert_eq!(capture.cwd, scope.binding().unwrap());
        assert_eq!(
            std::fs::read(root.join("calls")).unwrap(),
            b"--version\nplugin marketplace list --json\n"
        );
        assert_eq!(
            std::fs::read(root.join("arguments")).unwrap(),
            b"1\n--version\n4\nplugin\nmarketplace\nlist\n--json\n"
        );
        let canonical = std::fs::canonicalize(root).unwrap();
        let twice = format!("{0}\n{0}\n", canonical.display());
        assert_eq!(std::fs::read_to_string(root.join("cwds")).unwrap(), twice);
        assert_eq!(
            std::fs::read_to_string(root.join("homes")).unwrap(),
            format!("{0}\n{0}\n", root.display())
        );
        assert!(!format!("{capture:?}").contains("private-catalog"));
    });
}

// Catches skipping the version gate or accepting partial/nonzero version output.
#[test]
fn marketplace_reader_refuses_unpinned_incomplete_versions_and_extra_arguments() {
    for mode in ["wrong", "suffix", "refuse", "oversize", "arguments"] {
        with_scope(|root, scope, guard| {
            let mut spec = fake(root);
            let expected_calls = if mode == "arguments" {
                spec.arguments.push("--print".into());
                None
            } else {
                if matches!(mode, "wrong" | "suffix") {
                    spec.environment.insert(
                        "VERSION".into(),
                        if mode == "wrong" {
                            "2.1.309 (Claude Code)"
                        } else {
                            "2.1.308 (Claude Code)-modified"
                        }
                        .into(),
                    );
                } else {
                    spec.environment.insert("VERSION_MODE".into(), mode.into());
                }
                Some(b"--version\n".as_slice())
            };
            let error = read_marketplace_inventory(scope, guard, &spec, &AtomicBool::new(false))
                .unwrap_err();
            match mode {
                "arguments" => assert!(matches!(
                    error,
                    ClaudeInventoryReadError::UnexpectedArguments
                )),
                "wrong" | "suffix" => {
                    assert!(matches!(error, ClaudeInventoryReadError::VersionMismatch))
                }
                _ => assert!(matches!(
                    error,
                    ClaudeInventoryReadError::UnresolvedTransport
                )),
            }
            if let Some(expected) = expected_calls {
                assert_eq!(std::fs::read(root.join("calls")).unwrap(), expected);
            } else {
                assert!(!root.join("calls").exists());
            }
            assert!(!format!("{error:?}").contains("private-version"));
        });
    }
}

// Catches treating query failure, truncation or ambiguous JSON as empty metadata.
#[test]
fn incomplete_or_malformed_marketplace_capture_cannot_establish_empty_inventory() {
    for mode in [
        "refuse",
        "timeout",
        "overflow",
        "malformed",
        "duplicate",
        "change-executable",
    ] {
        with_scope(|root, scope, guard| {
            let mut spec = fake(root);
            spec.environment.insert("MODE".into(), mode.into());
            if mode == "overflow" {
                spec.output_limit = 128;
            }
            if mode == "malformed" {
                std::fs::write(
                    root.join("marketplaces.json"),
                    b"private-marketplace-malformed",
                )
                .unwrap();
            }
            if mode == "duplicate" {
                std::fs::write(root.join("marketplaces.json"),br#"[{"name":"owned","source":"directory","path":"/a","installLocation":"/a","ignored":{"x":1,"\u0078":2}}]"#).unwrap();
            }
            let error = read_marketplace_inventory(scope, guard, &spec, &AtomicBool::new(false))
                .unwrap_err();
            if matches!(mode, "malformed" | "duplicate") {
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
            assert_eq!(
                std::fs::read(root.join("calls")).unwrap(),
                b"--version\nplugin marketplace list --json\n"
            );
            assert!(!format!("{error:?}").contains("private-marketplace"));
        });
    }
}

// Catches the new entry point bypassing launch cancellation or root locking.
#[test]
fn marketplace_reader_cancellation_and_wrong_guard_do_not_launch() {
    with_scope(|root, scope, guard| {
        let spec = fake(root);
        assert!(matches!(
            read_marketplace_inventory(scope, guard, &spec, &AtomicBool::new(true)),
            Err(ClaudeInventoryReadError::Invocation(
                upskill::native_delivery::process::NativeProcessError::CancelledBeforeLaunch
            ))
        ));
        with_scope(|_, _, other_guard| {
            assert!(matches!(
                read_marketplace_inventory(scope, other_guard, &spec, &AtomicBool::new(false)),
                Err(ClaudeInventoryReadError::Invocation(
                    upskill::native_delivery::process::NativeProcessError::UnsafeScope
                ))
            ));
        });
        assert!(!root.join("calls").exists());
    });
}

// Catches dropping the reader's 1 MiB ceiling when a caller supplies a larger
// template limit. At the boundary, complete empty metadata still decodes.
#[test]
fn marketplace_reader_caps_transport_before_decoding_large_caller_templates() {
    with_scope(|root, scope, guard| {
        let mut spec = fake(root);
        spec.output_limit = 2 * 1024 * 1024;
        let mut bytes = b"[]".to_vec();
        bytes.resize(1024 * 1024, b' ');
        std::fs::write(root.join("marketplaces.json"), &bytes).unwrap();
        assert!(
            read_marketplace_inventory(scope, guard, &spec, &AtomicBool::new(false))
                .unwrap()
                .inventory
                .entries()
                .is_empty()
        );
        bytes.push(b' ');
        std::fs::write(root.join("marketplaces.json"), bytes).unwrap();
        assert!(matches!(
            read_marketplace_inventory(scope, guard, &spec, &AtomicBool::new(false)),
            Err(ClaudeInventoryReadError::UnresolvedTransport)
        ));
        assert_eq!(std::fs::read(root.join("calls")).unwrap(),b"--version\nplugin marketplace list --json\n--version\nplugin marketplace list --json\n");
    });
}

// Catches discarding a smaller caller cap in either version or listing capture.
#[test]
fn finite_output_obeys_caller_limits_at_both_query_boundaries() {
    for (limit, length, success, calls) in [
        (21, 2, false, "--version\n"),
        (22, 2, true, "--version\nplugin marketplace list --json\n"),
        (32, 32, true, "--version\nplugin marketplace list --json\n"),
        (32, 33, false, "--version\nplugin marketplace list --json\n"),
    ] {
        with_scope(|root, scope, guard| {
            let mut spec = fake(root);
            spec.output_limit = limit;
            let mut input = b"[]".to_vec();
            input.resize(length, b' ');
            std::fs::write(root.join("marketplaces.json"), input).unwrap();
            let result = read_marketplace_inventory(scope, guard, &spec, &AtomicBool::new(false));
            if success {
                assert!(result.unwrap().inventory.entries().is_empty());
            } else {
                assert!(matches!(
                    result,
                    Err(ClaudeInventoryReadError::UnresolvedTransport)
                ));
            }
            assert_eq!(std::fs::read_to_string(root.join("calls")).unwrap(), calls);
        });
    }
}

// Catches replacing the listing call's shared cancellation with a false flag.
// A ready marker proves that the version has completed and listing has started.
#[test]
fn cancellation_during_listing_returns_before_the_command_deadline() {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    with_scope(|root, scope, guard| {
        let mut spec = fake(root);
        spec.timeout = Duration::from_secs(20);
        spec.environment.insert("MODE".into(), "cancel".into());
        let ready = root.join("listing-ready");
        spec.environment
            .insert("READY".into(), ready.clone().into_os_string());
        let cancellation = AtomicBool::new(false);
        let started = Instant::now();
        std::thread::scope(|threads| {
            let watcher = threads.spawn(|| {
                let deadline = Instant::now() + Duration::from_secs(5);
                while !ready.exists() && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(5));
                }
                let observed = ready.exists();
                cancellation.store(true, Ordering::SeqCst);
                observed
            });
            let result = read_marketplace_inventory(scope, guard, &spec, &cancellation);
            assert!(
                watcher.join().unwrap(),
                "listing never reached its positive control"
            );
            assert!(matches!(
                result,
                Err(ClaudeInventoryReadError::UnresolvedTransport)
            ));
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "cancellation waited for the longer command deadline"
            );
        });
        assert_eq!(
            std::fs::read(root.join("calls")).unwrap(),
            b"--version\nplugin marketplace list --json\n"
        );
    });
}

// Catches persisting captured metadata or diagnostics beside the query. This
// covers the owned fixture root, not arbitrary native side effects elsewhere.
#[test]
fn completed_reader_preserves_fixture_files_and_creates_only_fake_receipts() {
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    fn snapshot(root: &Path) -> BTreeMap<PathBuf, (u32, Option<Vec<u8>>)> {
        fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, (u32, Option<Vec<u8>>)>) {
            let metadata = std::fs::symlink_metadata(path).unwrap();
            assert!(
                !metadata.file_type().is_symlink(),
                "unexpected fixture symlink"
            );
            let contents = metadata.is_file().then(|| std::fs::read(path).unwrap());
            out.insert(
                path.strip_prefix(root).unwrap().to_path_buf(),
                (metadata.permissions().mode(), contents),
            );
            if metadata.is_dir() {
                for entry in std::fs::read_dir(path).unwrap() {
                    walk(root, &entry.unwrap().path(), out);
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }
    for mode in ["success", "refuse"] {
        with_scope(|root, scope, guard| {
            let mut spec = fake(root);
            spec.environment.insert("MODE".into(), mode.into());
            let before = snapshot(root);
            let result = read_marketplace_inventory(scope, guard, &spec, &AtomicBool::new(false));
            assert_eq!(result.is_ok(), mode == "success");
            let after = snapshot(root);
            for (path, contents) in &before {
                assert_eq!(
                    after.get(path),
                    Some(contents),
                    "changed pre-existing fixture {path:?}"
                );
            }
            for path in after.keys().filter(|path| !before.contains_key(*path)) {
                assert!(
                    ["leader-pid", "calls", "arguments", "homes", "cwds"]
                        .iter()
                        .any(|allowed| path == Path::new(allowed)),
                    "unexpected reader artifact {path:?}"
                );
            }
        });
    }
}
