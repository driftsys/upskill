//! Preflight uses the real offline compiler fixture and only fake native assets.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/copilot_probe.rs"]
mod fixture;
use fixture::{Fixture, digest, private};
use serde_json::json;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;

#[test]
fn preflight_accepts_real_generated_fixture_without_native_execution() {
    let fixture = Fixture::new();
    let preflight = fixture.capture().unwrap();
    assert_eq!(preflight.fixture_file_count(), 9);
    preflight.verify_unchanged().unwrap();
    fixture.no_calls();
    assert!(!fixture.root.join("home/.copilot").exists());
}

#[test]
fn preflight_rejects_rehashed_plan_drift_and_fixed_argv_changes() {
    for (field, value) in [
        ("scope", json!("project")),
        ("client_version", json!("1.0.4")),
        ("model_calls", json!(1)),
        ("maximum_authorized_spending", json!(1)),
        ("maximum_native_commands", json!(12)),
        ("timeout_seconds_per_command", json!(21)),
        ("output_limit_bytes", json!(1048577)),
        ("native_name", json!("upskill-truncated")),
        (
            "commands",
            json!([["--no-auto-update", "plugin", "update", "--all"]]),
        ),
        ("extra", json!("secret-canary")),
    ] {
        let mut fixture = Fixture::new();
        fixture.edit_plan(|plan| {
            plan[field] = value;
        });
        assert!(fixture.capture().is_err(), "accepted {field}");
        fixture.no_calls();
    }
}

#[test]
fn preflight_never_reads_paths_from_an_altered_manifest() {
    let mut fixture = Fixture::new();
    // A FIFO would hang an ordinary blocking read. It is an unknown path and
    // must be rejected from metadata before any attempt to open its contents.
    let trap = fixture.parent.join("secret-canary");
    assert_cmd::Command::new("/usr/bin/mkfifo")
        .arg(&trap)
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .success();
    fixture.edit_preparation(|receipt| {
        receipt["fixture_files"][0]["path"] = json!(trap);
    });
    let error = fixture.capture().unwrap_err();
    assert!(!format!("{error:?}: {error}").contains("secret-canary"));
    fixture.no_calls();
}

#[test]
fn preflight_rejects_file_and_directory_drift_without_following_links() {
    for change in [
        "bytes",
        "mode",
        "symlink",
        "parent-link",
        "hardlink",
        "extra",
        "home-link",
    ] {
        let fixture = Fixture::new();
        let path = fixture.root.join("source/resources/probe.txt");
        match change {
            "bytes" => private(&path, b"secret-canary", 0o600),
            "mode" => fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap(),
            "symlink" => {
                let target = fixture.parent.join("outside-target");
                fs::copy(&path, &target).unwrap();
                fs::remove_file(&path).unwrap();
                symlink(&target, &path).unwrap();
            }
            "parent-link" => {
                fs::rename(
                    fixture.root.join("source/resources"),
                    fixture.parent.join("moved"),
                )
                .unwrap();
                symlink(
                    fixture.parent.join("moved"),
                    fixture.root.join("source/resources"),
                )
                .unwrap();
            }
            "hardlink" => fs::hard_link(&path, fixture.parent.join("alias")).unwrap(),
            "extra" => private(
                &fixture.root.join("unexpected-secret-name"),
                b"secret-canary",
                0o600,
            ),
            "home-link" => {
                fs::rename(fixture.root.join("home"), fixture.parent.join("moved")).unwrap();
                symlink(fixture.parent.join("moved"), fixture.root.join("home")).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(fixture.capture().is_err(), "accepted {change}");
        fixture.no_calls();
    }
}

#[test]
fn preflight_requires_reviewed_digests_and_unused_private_roots() {
    for change in [
        "plan-hash",
        "preparation-hash",
        "root-mode",
        "parent-mode",
        "evidence",
        "native-home",
    ] {
        let mut fixture = Fixture::new();
        match change {
            "plan-hash" => fixture.request.plan_sha256 = "0".repeat(64),
            "preparation-hash" => fixture.request.preparation_sha256 = "0".repeat(64),
            "root-mode" => {
                fs::set_permissions(&fixture.root, fs::Permissions::from_mode(0o755)).unwrap()
            }
            "parent-mode" => {
                fs::set_permissions(&fixture.parent, fs::Permissions::from_mode(0o755)).unwrap()
            }
            "evidence" => {
                fs::create_dir(fixture.root.join("evidence")).unwrap();
            }
            "native-home" => {
                fs::create_dir(fixture.root.join("home/.copilot")).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(fixture.capture().is_err(), "accepted {change}");
        assert!(!fixture.parent.join("native-invoked").exists());
        assert!(!fixture.root.join("home/native-invoked").exists());
    }
}

#[test]
fn preflight_revalidates_retained_files_roots_and_native_asset_identities() {
    for change in [
        "bytes",
        "project",
        "home",
        "outside",
        "nested",
        "code",
        "launcher-target",
        "preparation",
    ] {
        let fixture = Fixture::new();
        let preflight = fixture.capture().unwrap();
        match change {
            "bytes" => private(&fixture.root.join("source/SKILL.md"), b"changed", 0o600),
            "project" | "home" | "outside" => {
                let path = fixture.root.join(change);
                fs::rename(&path, fixture.parent.join("moved")).unwrap();
                fs::create_dir(&path).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            }
            "nested" => {
                let path = fixture.root.join("source/resources");
                fs::rename(&path, fixture.parent.join("moved")).unwrap();
                fs::create_dir(&path).unwrap();
                fs::rename(
                    fixture.parent.join("moved/probe.txt"),
                    path.join("probe.txt"),
                )
                .unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            }
            "code" => private(&fixture.parent.join("assets/node"), b"changed code", 0o700),
            "launcher-target" => {
                fs::remove_file(fixture.parent.join("assets/copilot")).unwrap();
                symlink("node", fixture.parent.join("assets/copilot")).unwrap();
            }
            "preparation" => private(&fixture.parent.join("preparation.json"), b"{}", 0o600),
            _ => unreachable!(),
        }
        assert!(preflight.verify_unchanged().is_err(), "accepted {change}");
        fixture.no_calls();
    }
}

#[test]
fn preflight_rejects_duplicate_json_and_inconsistent_preparation_claims() {
    for change in [
        "duplicate-json",
        "duplicate-file",
        "missing-file",
        "code-hash",
        "code-size",
        "code-size-big",
        "code-mode",
        "os",
        "version",
    ] {
        let mut fixture = Fixture::new();
        if change == "duplicate-json" {
            let path = fixture.parent.join("preparation.json");
            let text = fs::read_to_string(&path).unwrap();
            let changed = format!("{{\"st\\u0061tus\":\"secret-canary\",{}", &text[1..]);
            private(&path, changed.as_bytes(), 0o600);
            fixture.request.preparation_sha256 = digest(changed.as_bytes());
        } else {
            fixture.edit_preparation(|receipt| match change {
                "duplicate-file" => {
                    receipt["fixture_files"][1] = receipt["fixture_files"][0].clone()
                }
                "missing-file" => {
                    receipt["fixture_files"].as_array_mut().unwrap().pop();
                }
                "code-hash" => receipt["node"]["sha256"] = json!("0".repeat(64)),
                "code-size" => receipt["node"]["bytes"] = json!(0),
                "code-size-big" => receipt["node"]["bytes"] = json!(1073741825u64),
                "code-mode" => receipt["node"]["mode"] = json!("0o7777"),
                "os" => receipt["os"] = json!("unsupported"),
                "version" => receipt["required_cli_version"] = json!("1.0.4"),
                _ => unreachable!(),
            });
        }
        assert!(fixture.capture().is_err(), "accepted {change}");
        fixture.no_calls();
    }
}

#[test]
fn preservation_recheck_ignores_new_native_configuration_content() {
    let fixture = Fixture::new();
    let preflight = fixture.capture().unwrap();
    let native = fixture.root.join("home/.copilot");
    fs::create_dir(&native).unwrap();
    fs::set_permissions(&native, fs::Permissions::from_mode(0o700)).unwrap();
    assert_cmd::Command::new("/usr/bin/mkfifo")
        .arg(native.join("config.json"))
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .success();
    private(&native.join("unknown-secret-name"), b"secret-value", 0o600);
    preflight.verify_unchanged().unwrap();
    fixture.no_calls();
    assert_eq!(
        fs::read(native.join("unknown-secret-name")).unwrap(),
        b"secret-value"
    );
}

#[test]
fn preflight_binds_each_command_in_the_full_sequence() {
    for (index, argument, replacement) in [
        (0, 0, "--allow-all"),
        (3, 4, "/secret-canary"),
        (4, 3, "wrong@selector"),
        (7, 3, "wrong@selector"),
        (8, 4, "wrong-marketplace"),
        (1, 2, "update"),
        (2, 3, "add"),
        (5, 2, "update"),
        (6, 3, "add"),
        (9, 2, "update"),
        (10, 3, "add"),
    ] {
        let mut fixture = Fixture::new();
        fixture.edit_plan(|plan| {
            plan["commands"][index][argument] = json!(replacement);
        });
        assert!(fixture.capture().is_err(), "accepted command {index}");
        fixture.no_calls();
    }
}

fn asset_paths(fixture: &Fixture) -> [PathBuf; 3] {
    [
        fixture.parent.join("assets/npm-loader.js"),
        fixture.parent.join("assets/node"),
        fixture.parent.join("assets/platform/copilot"),
    ]
}

#[test]
fn preflight_binds_actual_modes_and_content_of_every_asset() {
    for asset in 0..3 {
        for mode in [0o600, 0o755] {
            let fixture = Fixture::new();
            fs::set_permissions(
                &asset_paths(&fixture)[asset],
                fs::Permissions::from_mode(mode),
            )
            .unwrap();
            assert!(
                fixture.capture().is_err(),
                "accepted asset {asset} mode {mode:o}"
            );
            fixture.no_calls();
        }
        let fixture = Fixture::new();
        private(&asset_paths(&fixture)[asset], b"changed code", 0o700);
        assert!(fixture.capture().is_err(), "accepted changed asset {asset}");
        fixture.no_calls();
    }
}

#[test]
fn preservation_recheck_rejects_identical_replacements_of_every_asset_and_fixture() {
    for asset in 0..3 {
        for replace in [false, true] {
            let fixture = Fixture::new();
            let preflight = fixture.capture().unwrap();
            let path = asset_paths(&fixture)[asset].clone();
            if replace {
                let retained = fixture.parent.join("retained-code");
                fs::rename(&path, &retained).unwrap();
                fs::copy(&retained, &path).unwrap();
                assert_eq!(fs::read(&path).unwrap(), fs::read(&retained).unwrap());
            } else {
                private(&path, b"changed code", 0o700);
            }
            assert!(
                preflight.verify_unchanged().is_err(),
                "accepted asset {asset} replace {replace}"
            );
            fixture.no_calls();
        }
    }
    let fixture = Fixture::new();
    let preflight = fixture.capture().unwrap();
    let path = fixture.root.join("source/resources/probe.txt");
    let retained = fixture.parent.join("retained-fixture");
    fs::rename(&path, &retained).unwrap();
    fs::copy(&retained, &path).unwrap();
    assert_eq!(fs::read(&path).unwrap(), fs::read(&retained).unwrap());
    assert!(preflight.verify_unchanged().is_err());
    fixture.no_calls();
}

#[test]
fn preflight_enforces_plan_and_preparation_read_boundaries() {
    for (kind, limit) in [("plan", 16384), ("preparation", 65536)] {
        for extra in [0, 1] {
            let mut fixture = Fixture::new();
            let path = if kind == "plan" {
                fixture.root.join("plan.json")
            } else {
                fixture.parent.join("preparation.json")
            };
            let mut bytes = fs::read(&path).unwrap();
            assert!(bytes.len() < limit);
            bytes.resize(limit + extra, b' ');
            if kind == "plan" {
                fixture.save_plan(&bytes);
            } else {
                private(&path, &bytes, 0o600);
                fixture.request.preparation_sha256 = digest(&bytes);
            }
            let result = fixture.capture();
            assert_eq!(
                result.is_ok(),
                extra == 0,
                "{kind} boundary extra {extra}: {result:?}"
            );
            fixture.no_calls();
        }
    }
}

#[test]
fn preflight_rejects_each_preparation_policy_mismatch_and_inventory_reordering() {
    for field in [
        "status",
        "machine",
        "wrapper_package_version",
        "plan_sha256",
        "version_mismatch_policy",
    ] {
        let mut fixture = Fixture::new();
        fixture.edit_preparation(|receipt| {
            receipt[field] = json!("different-reviewed-value");
        });
        assert!(fixture.capture().is_err(), "accepted {field}");
        fixture.no_calls();
    }
    for (first, second) in [(1, 2), (5, 6), (9, 10)] {
        let mut fixture = Fixture::new();
        fixture.edit_plan(|plan| {
            plan["commands"].as_array_mut().unwrap().swap(first, second);
        });
        assert!(
            fixture.capture().is_err(),
            "accepted swapped inventories {first}/{second}"
        );
        fixture.no_calls();
    }
}
