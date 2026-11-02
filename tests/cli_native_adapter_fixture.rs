//! Exercise the normal fixture executable with isolated homes and client spies.
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

fn example() -> &'static Path {
    static EXECUTABLE: OnceLock<PathBuf> = OnceLock::new();
    EXECUTABLE.get_or_init(|| {
        // Cargo releases its build lock before executing integration tests.
        // Resolve the normal example artifact, rather than a unit-test harness.
        let mut build = Command::new(env!("CARGO"));
        build
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args([
                "build",
                "--locked",
                "--offline",
                "--example",
                "native_adapter_fixture",
                "--message-format=json",
            ])
            .env_remove("CARGO_MAKEFLAGS")
            .env_remove("MAKEFLAGS")
            .stdin(Stdio::null());
        let output = assert_cmd::Command::from_std(build)
            .timeout(Duration::from_secs(180))
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        output
            .split(|byte| *byte == b'\n')
            .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
            .find_map(|row| {
                (row["reason"] == "compiler-artifact"
                    && row["target"]["name"] == "native_adapter_fixture")
                    .then(|| row["executable"].as_str().map(PathBuf::from))
                    .flatten()
            })
            .expect("normal example executable missing from Cargo artifact output")
    })
}
fn spies(root: &Path) -> PathBuf {
    let bin = root.join("spies");
    std::fs::create_dir(&bin).unwrap();
    let source = root.join("native_spy.rs");
    std::fs::write(
        &source,
        r#"use std::io::Write;
fn main() {
    let receipt = std::env::var_os("UPSKILL_NATIVE_SPY_RECEIPT").unwrap();
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(receipt).unwrap();
    let program = std::env::args_os().next().unwrap();
    let name = std::path::Path::new(&program).file_stem().unwrap().to_string_lossy();
    writeln!(file, "{name}").unwrap();
    println!("[]");
}
"#,
    )
    .unwrap();
    let compiled = bin.join(format!("native_spy{}", std::env::consts::EXE_SUFFIX));
    let mut compiler = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
    compiler
        .args(["--edition=2024", "--crate-name", "native_spy"])
        .arg(&source)
        .arg("-o")
        .arg(&compiled)
        .stdin(Stdio::null());
    assert_cmd::Command::from_std(compiler)
        .timeout(Duration::from_secs(120))
        .assert()
        .success();
    for name in ["claude", "copilot", "codex", "opencode"] {
        std::fs::copy(
            &compiled,
            bin.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
        )
        .unwrap();
    }
    bin
}
fn isolated(program: &Path, root: &Path, bin: &Path) -> Command {
    let home = root.join("process-home");
    std::fs::create_dir_all(&home).unwrap();
    let mut command = Command::new(program);
    command
        .env_clear()
        .current_dir(root)
        .stdin(Stdio::null())
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("PATH", bin)
        .env("TMPDIR", root)
        .env("TMP", root)
        .env("TEMP", root)
        .env("COPILOT_HOME", home.join(".copilot"))
        .env("COPILOT_CACHE_HOME", home.join("copilot-cache"))
        .env("CLAUDE_CONFIG_DIR", home.join(".claude"))
        .env("CODEX_HOME", home.join(".codex"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_STATE_HOME", home.join("state"))
        .env("UPSKILL_NATIVE_SPY_RECEIPT", root.join("native-calls"));
    for name in ["SYSTEMROOT", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
}

// Catches dropping the real argv's client flag, changing default routing, or
// invoking a native client as an ignored preparation side effect.
#[test]
fn executable_routes_both_forms_without_invoking_native_clients() {
    let executable = example();
    let temp = tempfile::tempdir().unwrap();
    let bin = spies(temp.path());
    let receipt = temp.path().join("native-calls");
    for client in ["claude", "copilot", "codex", "opencode"] {
        let mut control = isolated(Path::new(client), temp.path(), &bin);
        control.arg("--version");
        assert_cmd::Command::from_std(control)
            .timeout(Duration::from_secs(10))
            .assert()
            .success();
    }
    assert_eq!(
        std::fs::read(&receipt).unwrap(),
        b"claude\ncopilot\ncodex\nopencode\n"
    );
    std::fs::remove_file(&receipt).unwrap();
    for (client, scope, subdir, manifest, version, count) in [
        (
            "claude",
            "project",
            "project",
            ".claude-plugin/plugin.json",
            "2.1.308",
            12,
        ),
        (
            "copilot",
            "user",
            "home",
            ".plugin/plugin.json",
            "1.0.105",
            11,
        ),
    ] {
        let requested = temp.path().join(format!("{client}-packet"));
        let mut command = isolated(executable, temp.path(), &bin);
        if client == "copilot" {
            command.arg("--copilot");
        }
        command.arg(&requested);
        let output = assert_cmd::Command::from_std(command)
            .timeout(Duration::from_secs(30))
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let packet = requested.canonicalize().unwrap();
        assert_eq!(
            Path::new(std::str::from_utf8(&output).unwrap().trim()),
            packet.join("plan.json")
        );
        let plan: Value =
            serde_json::from_slice(&std::fs::read(packet.join("plan.json")).unwrap()).unwrap();
        assert_eq!(plan["client_version"], version);
        assert_eq!(plan["scope"], scope);
        assert_eq!(plan["scope_root"], json!(packet.join(subdir)));
        assert_eq!(plan["maximum_native_commands"], count);
        assert_eq!(plan["model_calls"], 0);
        let payload = plan["payload_sha256"].as_str().unwrap();
        assert_eq!(payload.len(), 64);
        assert!(
            payload
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        );
        let revision = packet.join(subdir).join(format!(
            ".upskill/exports/{client}/native-contract-probe/revisions/{payload}"
        ));
        let plugin: Value =
            serde_json::from_slice(&std::fs::read(revision.join(manifest)).unwrap()).unwrap();
        assert_eq!(plugin["name"], plan["native_name"]);
        assert_eq!(plugin["version"], plan["version"]);
        assert_eq!(
            std::fs::read(revision.join("skills/contract-probe/resources/probe.txt")).unwrap(),
            b"Native contract resource. No model request is part of this fixture.\n"
        );
        assert!(!receipt.exists(), "preparation invoked a native-client spy");
    }
}
