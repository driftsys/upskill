//! Golden-file tests for skill generation across all three clients.
//!
//! Input: `tests/fixtures/skills/<name>/SKILL.md`
//! Expected: `tests/fixtures/expected/<client>/<name>.SKILL.md`

use std::fs;
use upskill::generate::{Client, render_skill};
use upskill::model::Skill;
use upskill::parse::frontmatter;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn load_skill(name: &str) -> (Skill, String) {
    let path = format!("{FIXTURES}/skills/{name}/SKILL.md");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (skill, body) = frontmatter::parse::<Skill>(&raw).expect("parse fixture");
    (skill, body.to_string())
}

fn load_expected(client: &str, name: &str) -> String {
    let path = format!("{FIXTURES}/expected/{client}/{name}.SKILL.md");
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

#[test]
fn create_api_endpoint_claude() {
    let (skill, body) = load_skill("create-api-endpoint");
    let actual = render_skill(&skill, &body, Client::Claude).expect("render");
    let expected = load_expected("claude", "create-api-endpoint");
    assert_byte_equal(&actual, &expected, "claude/create-api-endpoint");
}

#[test]
fn create_api_endpoint_copilot() {
    let (skill, body) = load_skill("create-api-endpoint");
    let actual = render_skill(&skill, &body, Client::Copilot).expect("render");
    let expected = load_expected("copilot", "create-api-endpoint");
    assert_byte_equal(&actual, &expected, "copilot/create-api-endpoint");
}

#[test]
fn create_api_endpoint_opencode() {
    let (skill, body) = load_skill("create-api-endpoint");
    let actual = render_skill(&skill, &body, Client::OpenCode).expect("render");
    let expected = load_expected("opencode", "create-api-endpoint");
    assert_byte_equal(&actual, &expected, "opencode/create-api-endpoint");
}
