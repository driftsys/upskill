//! Guard test: enforce the fake-`$HOME` test harness.
//!
//! Every integration test that launches the `upskill` binary MUST go through
//! [`common::upskill_cmd`], which points `HOME`/`USERPROFILE` at a tempdir so a
//! stray global-scope write can never land in the developer's real `$HOME`.
//! `add` defaults to global scope outside a git repo, so a raw
//! `Command::cargo_bin("upskill")` that forgets a `.git` marker silently
//! pollutes the real home. This has bitten twice (issues #193 and #199).
//!
//! This test scans every top-level `tests/*.rs` source file and fails if it
//! constructs the binary directly instead of through the harness. A file that
//! legitimately needs raw construction — e.g. one that tests global scope by
//! setting `HOME`/`USERPROFILE` itself, or by deliberately leaving them unset
//! — opts out with a comment containing the marker
//! `upskill-allow-raw-cargo-bin: <reason>`.
//!
//! See <https://github.com/driftsys/upskill/issues/202>.

use std::fs;
use std::path::Path;

/// Marker a test file places in a comment to opt out of the harness check,
/// e.g. `// upskill-allow-raw-cargo-bin: tests global scope, sets HOME itself`.
const OPT_OUT_MARKER: &str = "upskill-allow-raw-cargo-bin";

/// Substrings that indicate the binary is being constructed directly rather
/// than through `common::upskill_cmd`.
const RAW_PATTERNS: [&str; 2] = ["cargo_bin(\"upskill\")", "cargo_bin!(\"upskill\")"];

#[test]
fn integration_tests_use_fake_home_harness() {
    let tests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut violations: Vec<String> = Vec::new();

    for entry in fs::read_dir(&tests_dir).expect("tests/ directory is readable") {
        let path = entry.expect("readable dir entry").path();
        // Only top-level `*.rs` files. The harness itself lives in
        // `tests/common/` (a subdirectory, skipped by the extension filter).
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        // This guard names the raw patterns in string literals; skip itself.
        if file_name == "cli_test_harness_guard.rs" {
            continue;
        }

        let src = fs::read_to_string(&path).expect("test source is readable");
        if src.contains(OPT_OUT_MARKER) {
            continue;
        }

        let hits: Vec<usize> = src
            .lines()
            .enumerate()
            .filter(|(_, line)| RAW_PATTERNS.iter().any(|p| line.contains(p)))
            .map(|(i, _)| i + 1)
            .collect();

        if !hits.is_empty() {
            violations.push(format!(
                "  {file_name}: line(s) {hits:?} construct the binary directly"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "integration tests must launch `upskill` via `common::upskill_cmd` \
         (fake $HOME), not a raw `Command::cargo_bin(\"upskill\")`:\n{}\n\n\
         Fix: route the command through `common::upskill_cmd(&fake_home)`. If a \
         file legitimately needs raw construction (e.g. it tests global scope \
         and controls HOME/USERPROFILE itself), add a comment containing \
         `{OPT_OUT_MARKER}: <reason>`.",
        violations.join("\n"),
    );
}
