//! Golden-file tests for agent generation across all three clients.
//!
//! Input: `tests/fixtures/items/<name>/AGENT.md`
//! Expected: `tests/fixtures/expected/<client>/<name>.AGENT.md`

use std::fs;
use upskill::generate::{Client, render_agent};
use upskill::model::Agent;
use upskill::parse::frontmatter;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// Returns the parsed agent, its effective name (frontmatter `name`, else
/// the fixture folder), and the body.
fn load_agent(name: &str) -> (Agent, String, String) {
    let path = format!("{FIXTURES}/items/{name}/AGENT.md");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (agent, body) = frontmatter::parse::<Agent>(&raw).expect("parse fixture");
    let effective = agent.name.clone().unwrap_or_else(|| name.to_string());
    (agent, effective, body.to_string())
}

fn load_expected(client: &str, name: &str) -> String {
    let path = format!("{FIXTURES}/expected/{client}/{name}.AGENT.md");
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

// security-reviewer — mode/tools/preload-skills + opencode passthrough

#[test]
fn security_reviewer_claude() {
    let (agent, name, body) = load_agent("security-reviewer");
    let actual = render_agent(&agent, &name, &body, Client::Claude).expect("render");
    let expected = load_expected("claude", "security-reviewer");
    assert_byte_equal(&actual, &expected, "claude/security-reviewer");
}

#[test]
fn security_reviewer_copilot() {
    let (agent, name, body) = load_agent("security-reviewer");
    let actual = render_agent(&agent, &name, &body, Client::Copilot).expect("render");
    let expected = load_expected("copilot", "security-reviewer");
    assert_byte_equal(&actual, &expected, "copilot/security-reviewer");
}

#[test]
fn security_reviewer_opencode() {
    let (agent, name, body) = load_agent("security-reviewer");
    let actual = render_agent(&agent, &name, &body, Client::OpenCode).expect("render");
    let expected = load_expected("opencode", "security-reviewer");
    assert_byte_equal(&actual, &expected, "opencode/security-reviewer");
}
