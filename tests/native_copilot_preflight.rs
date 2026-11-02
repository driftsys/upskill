//! Preflight uses the real offline compiler fixture and only fake native assets.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/native_fixture_executable.rs"]
mod fixture_executable;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use upskill::native_delivery::copilot::probe::{CopilotProbePreflight, CopilotProbeRequest};

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn private(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    parent: PathBuf,
    request: CopilotProbeRequest,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().canonicalize().unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        let root = parent.join("packet");
        let home = parent.join("generator-home");
        fs::create_dir(&home).unwrap();
        let mut command = Command::new(fixture_executable::example());
        command
            .args(["--copilot"])
            .arg(&root)
            .env_clear()
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("PATH", "/usr/bin:/bin")
            .current_dir(&parent)
            .stdin(Stdio::null());
        assert_cmd::Command::from_std(command)
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success();
        let assets = parent.join("assets");
        fs::create_dir(&assets).unwrap();
        let launcher = assets.join("npm-loader.js");
        // Only a test spy: native execution would leave an independent receipt.
        private(
            &launcher,
            b"#!/bin/sh\nprintf invoked > \"$HOME/native-invoked\"\n",
            0o700,
        );
        let selected = assets.join("copilot");
        symlink("npm-loader.js", &selected).unwrap();
        let node = assets.join("node");
        fs::copy(&launcher, &node).unwrap();
        let platform_dir = assets.join("platform");
        fs::create_dir(&platform_dir).unwrap();
        let platform = platform_dir.join("copilot");
        fs::copy(&launcher, &platform).unwrap();
        // Positive control establishes that the receipt detects this exact spy.
        assert!(
            Command::new(&selected)
                .env_clear()
                .env("HOME", &parent)
                .status()
                .unwrap()
                .success()
        );
        assert_eq!(fs::read(parent.join("native-invoked")).unwrap(), b"invoked");
        fs::remove_file(parent.join("native-invoked")).unwrap();
        fn pin(path: &Path) -> Value {
            let resolved = path.canonicalize().unwrap();
            let bytes = fs::read(&resolved).unwrap();
            json!({"selected":path,"resolved":resolved,"sha256":digest(&bytes),"bytes":bytes.len(),"mode":"0o700"})
        }
        fn files(base: &Path, dir: &Path, rows: &mut Vec<Value>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    files(base, &path, rows);
                } else {
                    let bytes = fs::read(&path).unwrap();
                    rows.push(json!({"path":path.strip_prefix(base).unwrap().to_str().unwrap(),"bytes":bytes.len(),"sha256":digest(&bytes),"mode":"0o600"}));
                }
            }
        }
        let plan_sha256 = digest(&fs::read(root.join("plan.json")).unwrap());
        let mut rows = vec![];
        files(&root, &root, &mut rows);
        rows.sort_by_key(|row| row["path"].as_str().unwrap().to_owned());
        let preparation = json!({"status":"prepared only; no native command executed",
            "os": if cfg!(target_os="macos") {"Darwin"} else {"Linux"},
            "machine": if cfg!(all(target_os="macos",target_arch="aarch64")) {"arm64"} else {std::env::consts::ARCH},
            "launcher":pin(&selected),"node":pin(&node),"platform_entrypoint":pin(&platform),
            "wrapper_package_version":"1.0.4","required_cli_version":"1.0.105",
            "version_mismatch_policy":"stop before inventories or mutations; no update or fallback",
            "plan_sha256":plan_sha256,"fixture_files":rows});
        private(
            &parent.join("preparation.json"),
            &serde_json::to_vec_pretty(&preparation).unwrap(),
            0o600,
        );
        let preparation_sha256 = digest(&fs::read(parent.join("preparation.json")).unwrap());
        let request = CopilotProbeRequest {
            packet_root: root.clone(),
            plan_sha256,
            preparation_sha256,
        };
        Self {
            _temp: temp,
            root,
            parent,
            request,
        }
    }
    fn capture(
        &self,
    ) -> Result<
        CopilotProbePreflight,
        upskill::native_delivery::copilot::probe::CopilotPreflightError,
    > {
        CopilotProbePreflight::capture(&self.request)
    }
    fn edit_preparation(&mut self, edit: impl FnOnce(&mut Value)) {
        let path = self.parent.join("preparation.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        edit(&mut value);
        private(&path, &serde_json::to_vec_pretty(&value).unwrap(), 0o600);
        self.request.preparation_sha256 = digest(&fs::read(path).unwrap());
    }
    fn edit_plan(&mut self, edit: impl FnOnce(&mut Value)) {
        let path = self.root.join("plan.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        edit(&mut value);
        private(&path, &serde_json::to_vec_pretty(&value).unwrap(), 0o600);
        self.request.plan_sha256 = digest(&fs::read(path).unwrap());
        let new_hash = self.request.plan_sha256.clone();
        let new_bytes = fs::metadata(self.root.join("plan.json")).unwrap().len();
        self.edit_preparation(|receipt| {
            receipt["plan_sha256"] = json!(new_hash);
            for file in receipt["fixture_files"].as_array_mut().unwrap() {
                if file["path"] == "plan.json" {
                    file["sha256"] = json!(new_hash);
                    file["bytes"] = json!(new_bytes);
                }
            }
        });
    }
    fn no_calls(&self) {
        assert!(!self.parent.join("native-invoked").exists());
        assert!(!self.root.join("home/native-invoked").exists());
        assert!(!self.root.join("evidence").exists());
    }
}

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
    ] {
        let mut fixture = Fixture::new();
        fixture.edit_plan(|plan| {
            plan["commands"][index][argument] = json!(replacement);
        });
        assert!(fixture.capture().is_err(), "accepted command {index}");
        fixture.no_calls();
    }
}
