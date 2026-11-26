//! Pins the behaviour of the `upskill-setup` session-start hook script.
//!
//! The script runs with `sh` directly. A fake `upskill` executable in
//! `<tmp>/bin` stands in for the real binary; a `PATH` without it stands in
//! for a machine where the CLI is not installed.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const LINE1: &str = "The upskill CLI is not installed. Install it with:";
const LINE2: &str =
    "curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh";
const COPILOT_REQUEST: &str = "Tell the user at the start of your first reply: ";

struct Env {
    tmp: tempfile::TempDir,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("bin")).unwrap();
        fs::create_dir(tmp.path().join("cache")).unwrap();
        Self { tmp }
    }

    fn root(&self) -> &Path {
        self.tmp.path()
    }

    fn cache(&self) -> PathBuf {
        self.root().join("cache")
    }

    fn marker(&self) -> PathBuf {
        self.cache().join("upskill/setup-offered")
    }

    fn install_fake(&self, script: &str) {
        let fake = self.root().join("bin/upskill");
        fs::write(
            &fake,
            format!(
                "#!/bin/sh\nprintf '%s ' \"$@\" > '{}'\n{}\n",
                self.root().join("args").display(),
                script
            ),
        )
        .unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn args(&self) -> String {
        fs::read_to_string(self.root().join("args")).unwrap_or_default()
    }

    fn run(&self, client: &str) -> Output {
        let script =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("skills/upskill-setup/upskill-setup.sh");
        let mut cmd = Command::new("sh");
        cmd.arg(script)
            .env_clear()
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.root().join("bin").display()),
            )
            .env("HOME", self.root().join("home"))
            .env("XDG_CACHE_HOME", self.cache())
            .current_dir(self.root());
        if !client.is_empty() {
            cmd.env("UPSKILL_CLIENT", client);
        }
        cmd.output().expect("sh runs")
    }
}

fn json(out: &Output) -> Value {
    assert!(out.status.success(), "exit status: {:?}", out.status);
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert!(stdout.ends_with('\n'));
    assert_eq!(stdout.trim_end_matches('\n').lines().count(), 1);
    serde_json::from_str(&stdout).expect("stdout is valid JSON")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn assert_silent(out: &Output) {
    assert!(out.status.success(), "exit status: {:?}", out.status);
    assert_eq!(stdout(out), "");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

fn message() -> String {
    format!("{LINE1}\n{LINE2}")
}

#[test]
fn setup_missing_cli_prints_install_message_for_claude() {
    let env = Env::new();
    let v = json(&env.run("claude"));
    assert_eq!(v["systemMessage"], message());
    assert_eq!(v["hookSpecificOutput"]["hookEventName"], "SessionStart");
    assert_eq!(v["hookSpecificOutput"]["additionalContext"], message());
}

#[test]
fn setup_missing_cli_prints_install_message_for_copilot() {
    let env = Env::new();
    let v = json(&env.run("copilot"));
    assert_eq!(v.as_object().unwrap().len(), 1);
    assert_eq!(
        v["additionalContext"],
        format!("{COPILOT_REQUEST}{}", message())
    );
}

#[test]
fn setup_missing_cli_prints_once_a_day() {
    let env = Env::new();
    json(&env.run("claude"));
    assert_silent(&env.run("claude"));
    fs::write(env.marker(), "2000-01-01\n").unwrap();
    json(&env.run("claude"));
}

#[test]
fn setup_unwritable_cache_exits_zero() {
    // The root user can write to a mode 0555 directory, so the case cannot be set up.
    let uid = std::process::Command::new("id").arg("-u").output().unwrap();
    if String::from_utf8_lossy(&uid.stdout).trim() == "0" {
        return;
    }
    let env = Env::new();
    fs::set_permissions(env.cache(), fs::Permissions::from_mode(0o555)).unwrap();
    let out = env.run("claude");
    fs::set_permissions(env.cache(), fs::Permissions::from_mode(0o755)).unwrap();
    json(&out);
}

#[test]
fn setup_passes_notice_through_for_claude() {
    let env = Env::new();
    let line = "upskill: 3 updates available (project). Run \"upskill update\".";
    env.install_fake(&format!("printf '%s\\n' '{line}'"));
    let v = json(&env.run("claude"));
    assert_eq!(v["systemMessage"], line);
    assert_eq!(v["hookSpecificOutput"]["additionalContext"], line);
    assert_eq!(env.args(), "update --notice ");
}

#[test]
fn setup_uses_repeat_for_copilot() {
    let env = Env::new();
    env.install_fake("printf '%s\\n' 'one update'");
    let v = json(&env.run("copilot"));
    assert_eq!(
        v["additionalContext"],
        format!("{COPILOT_REQUEST}one update")
    );
    assert_eq!(env.args(), "update --notice --repeat ");
}

#[test]
fn setup_silent_when_notice_empty_or_fails() {
    for script in ["exit 0", "printf 'x\\n'; exit 1"] {
        for client in ["claude", "copilot"] {
            let env = Env::new();
            env.install_fake(script);
            assert_silent(&env.run(client));
        }
    }
}

#[test]
fn setup_unknown_client_is_silent() {
    for client in ["", "other"] {
        let env = Env::new();
        assert_silent(&env.run(client));
        assert!(!env.marker().exists());
    }
}
