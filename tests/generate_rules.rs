//! Golden-file tests for rule generation across all three clients.
//!
//! Input: `tests/fixtures/items/<name>/RULE.md`
//! Expected: `tests/fixtures/expected/<client>/<name>.RULE.md`

use std::fs;
use upskill::generate::{Client, render_rule};
use upskill::model::Rule;
use upskill::parse::frontmatter;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn load_rule(name: &str) -> (Rule, String) {
    let path = format!("{FIXTURES}/items/{name}/RULE.md");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (rule, body) = frontmatter::parse::<Rule>(&raw).expect("parse fixture");
    (rule, body.to_string())
}

fn load_expected(client: &str, name: &str) -> String {
    let path = format!("{FIXTURES}/expected/{client}/{name}.RULE.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn assert_byte_equal(actual: &str, expected: &str, label: &str) {
    if actual == expected {
        return;
    }
    eprintln!("=== {label}: actual ===\n{actual}");
    eprintln!("=== {label}: expected ===\n{expected}");
    panic!("{label} mismatch (see stderr above)");
}

// license-awareness — always-on, no scope, no client passthroughs

#[test]
fn license_awareness_claude() {
    let (rule, body) = load_rule("license-awareness");
    let actual = render_rule(&rule, &body, Client::Claude).expect("render");
    let expected = load_expected("claude", "license-awareness");
    assert_byte_equal(&actual, &expected, "claude/license-awareness");
}

#[test]
fn license_awareness_copilot() {
    let (rule, body) = load_rule("license-awareness");
    let actual = render_rule(&rule, &body, Client::Copilot).expect("render");
    let expected = load_expected("copilot", "license-awareness");
    assert_byte_equal(&actual, &expected, "copilot/license-awareness");
}

#[test]
fn license_awareness_opencode() {
    let (rule, body) = load_rule("license-awareness");
    let actual = render_rule(&rule, &body, Client::OpenCode).expect("render");
    let expected = load_expected("opencode", "license-awareness");
    assert_byte_equal(&actual, &expected, "opencode/license-awareness");
}

// api-conventions — path-scoped, has copilot.* passthrough

#[test]
fn api_conventions_claude() {
    let (rule, body) = load_rule("api-conventions");
    let actual = render_rule(&rule, &body, Client::Claude).expect("render");
    let expected = load_expected("claude", "api-conventions");
    assert_byte_equal(&actual, &expected, "claude/api-conventions");
}

#[test]
fn api_conventions_copilot() {
    let (rule, body) = load_rule("api-conventions");
    let actual = render_rule(&rule, &body, Client::Copilot).expect("render");
    let expected = load_expected("copilot", "api-conventions");
    assert_byte_equal(&actual, &expected, "copilot/api-conventions");
}

#[test]
fn api_conventions_opencode() {
    let (rule, body) = load_rule("api-conventions");
    let actual = render_rule(&rule, &body, Client::OpenCode).expect("render");
    let expected = load_expected("opencode", "api-conventions");
    assert_byte_equal(&actual, &expected, "opencode/api-conventions");
}
