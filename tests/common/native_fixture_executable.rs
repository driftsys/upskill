//! Resolve the normal offline fixture generator artifact once per test process.
#![allow(dead_code)]
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

pub fn example() -> &'static Path {
    static EXECUTABLE: OnceLock<PathBuf> = OnceLock::new();
    EXECUTABLE.get_or_init(|| build("native_adapter_fixture"))
}
pub fn collector_example() -> &'static Path {
    static EXECUTABLE: OnceLock<PathBuf> = OnceLock::new();
    EXECUTABLE.get_or_init(|| build("copilot_contract_probe"))
}
fn build(name: &str) -> PathBuf {
    let mut build = Command::new(env!("CARGO"));
    build
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "build",
            "--locked",
            "--offline",
            "--example",
            name,
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
            (row["reason"] == "compiler-artifact" && row["target"]["name"] == name)
                .then(|| row["executable"].as_str().map(PathBuf::from))
                .flatten()
        })
        .expect("normal example executable missing from Cargo artifact output")
}
