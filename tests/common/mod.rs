//! Shared test harness utilities.
//!
//! Every integration test that invokes `upskill` MUST use [`upskill_cmd`] (or
//! manually set `HOME`) to prevent writes to the developer's real home
//! directory. See <https://github.com/driftsys/upskill/issues/193>.

use assert_cmd::Command;
use std::path::Path;

/// Returns a `Command` for the `upskill` binary with `HOME` pointed at
/// `fake_home` (which the caller should create inside a `tempfile::TempDir`).
///
/// This is the **belt-and-suspenders** defense: even if a test forgets to
/// create a `.git/` marker in its working directory, the worst case is a
/// write into the tempdir's fake home — never the developer's real `$HOME`.
pub fn upskill_cmd(fake_home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("upskill").unwrap();
    cmd.env("HOME", fake_home);
    // Also override USERPROFILE for Windows-compat code paths.
    cmd.env("USERPROFILE", fake_home);
    cmd
}

/// Run `upskill update <args>` with a terminal on stdin and write `input`
/// to it (`"\u{4}"` is end of input). Returns the JSON result of
/// `tests/fixtures/terminal_stdin.py`, with the keys `returncode`,
/// `stdout`, `stderr` and `timed_out`.
#[cfg(unix)]
#[allow(dead_code)] // Not every test binary that includes this module uses it.
pub fn update_in_terminal(
    root: &Path,
    home: &Path,
    args: &[&str],
    input: &str,
) -> serde_json::Value {
    run_update_in_terminal(root, home, args, input, false, None)
}

/// As [`update_in_terminal`], with stderr written to the same pipe as
/// stdout: `stdout` of the result holds both streams in the order the
/// user sees them, and `stderr` is empty.
#[cfg(unix)]
#[allow(dead_code)] // Not every test binary that includes this module uses it.
pub fn update_in_terminal_merged(
    root: &Path,
    home: &Path,
    args: &[&str],
    input: &str,
) -> serde_json::Value {
    run_update_in_terminal(root, home, args, input, true, None)
}

/// As [`update_in_terminal_merged`] without input: when the output contains
/// `at`, the fixture sends SIGINT to the process group of the command.
#[cfg(unix)]
#[allow(dead_code)] // Not every test binary that includes this module uses it.
pub fn update_in_terminal_interrupted(
    root: &Path,
    home: &Path,
    args: &[&str],
    at: &str,
) -> serde_json::Value {
    run_update_in_terminal(root, home, args, "", true, Some(at))
}

#[cfg(unix)]
#[allow(dead_code)] // Not every test binary that includes this module uses it.
fn run_update_in_terminal(
    root: &Path,
    home: &Path,
    args: &[&str],
    input: &str,
    merge: bool,
    interrupt_at: Option<&str>,
) -> serde_json::Value {
    let mut update = upskill_cmd(home);
    update
        .current_dir(root)
        .arg("update")
        .args(args)
        .env("NO_COLOR", "1");
    let spec = serde_json::json!({
        "program": update.get_program().to_str().unwrap(),
        "args": update.get_args().map(|arg| arg.to_str().unwrap()).collect::<Vec<_>>(),
        "cwd": update.get_current_dir().unwrap(),
        "home": home,
        "env": update.get_envs().map(|(key, value)| {
            (key.to_str().unwrap(), value.map(|value| value.to_str().unwrap()))
        }).collect::<Vec<_>>(),
        "input": input,
        "merge": merge,
        "interrupt_at": interrupt_at,
    });
    let output = std::process::Command::new("python3")
        .env("HOME", home)
        .env("USERPROFILE", home)
        .args([
            "-c",
            include_str!("../fixtures/terminal_stdin.py"),
            &spec.to_string(),
        ])
        .output()
        .expect("Python 3 with stdlib pty is required for the Unix terminal test");
    assert!(
        output.status.success(),
        "terminal fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let verified = if merge { "stdout" } else { "stderr" };
    assert!(
        result[verified]
            .as_str()
            .unwrap()
            .contains("terminal stdin verified\n"),
        "child did not verify terminal stdin: {result}"
    );
    assert_eq!(result["timed_out"], false, "{result}");
    result
}
