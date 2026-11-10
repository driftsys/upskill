//! Preflight uses the real offline compiler fixture and only fake native assets.
#![allow(dead_code)]
#[path = "native_fixture_executable.rs"]
pub(super) mod fixture_executable;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use upskill::native_delivery::copilot::probe::{CopilotProbePreflight, CopilotProbeRequest};

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub fn private(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}
pub struct Fixture {
    _temp: tempfile::TempDir,
    pub root: PathBuf,
    pub parent: PathBuf,
    pub request: CopilotProbeRequest,
}
impl Fixture {
    pub fn new() -> Self {
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
            "wrapper_package_version":"1.0.107","required_cli_version":"1.0.107",
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
    pub fn capture(
        &self,
    ) -> Result<
        CopilotProbePreflight,
        upskill::native_delivery::copilot::probe::CopilotPreflightError,
    > {
        CopilotProbePreflight::capture(&self.request)
    }
    pub fn edit_preparation(&mut self, edit: impl FnOnce(&mut Value)) {
        let path = self.parent.join("preparation.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        edit(&mut value);
        private(&path, &serde_json::to_vec_pretty(&value).unwrap(), 0o600);
        self.request.preparation_sha256 = digest(&fs::read(path).unwrap());
    }
    pub fn edit_plan(&mut self, edit: impl FnOnce(&mut Value)) {
        let path = self.root.join("plan.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        edit(&mut value);
        self.save_plan(&serde_json::to_vec_pretty(&value).unwrap());
    }
    pub fn save_plan(&mut self, bytes: &[u8]) {
        let path = self.root.join("plan.json");
        private(&path, bytes, 0o600);
        self.request.plan_sha256 = digest(bytes);
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
    pub fn no_native_calls(&self) {
        assert!(!self.parent.join("native-invoked").exists());
        assert!(!self.root.join("home/native-invoked").exists());
    }
    pub fn no_calls(&self) {
        self.no_native_calls();
        assert!(!self.root.join("evidence").exists());
    }
}
