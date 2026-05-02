//! ATDD test for `pipeline::install_from_local_path`.
//!
//! Given a local SSOT source directory laid out per format-spec §2.1
//! (`skills/<name>/SKILL.md`, `rules/<name>/RULE.md`, `agents/<name>/AGENT.md`),
//! `install_from_local_path(source, target)` writes per-client generated
//! output files under `target` at the paths defined in format-spec §7 and
//! ADR-0003.
//!
//! Content equality is verified against the existing golden fixtures in
//! `tests/fixtures/expected/` (the same fixtures used by `generate_*` tests).

use std::fs;
use std::path::Path;
use upskill::pipeline::{InstallReport, install_from_local_path};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// Stage the entire fixture corpus into a temp source directory.
fn stage_source(source: &Path) {
    for kind in ["skills", "rules", "agents"] {
        let from = format!("{FIXTURES}/{kind}");
        let to = source.join(kind);
        copy_dir_all(Path::new(&from), &to).unwrap();
    }
}

fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let to_path = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &to_path)?;
        } else {
            fs::copy(entry.path(), &to_path)?;
        }
    }
    Ok(())
}

fn read_expected(client: &str, fixture_name: &str) -> String {
    let path = format!("{FIXTURES}/expected/{client}/{fixture_name}");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn assert_file_eq(actual_path: &Path, expected: &str, label: &str) {
    let actual = fs::read_to_string(actual_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", actual_path.display()));
    if actual == expected {
        return;
    }
    eprintln!("=== {label}: actual ===\n{actual}");
    eprintln!("=== {label}: expected ===\n{expected}");
    panic!("{label} mismatch (see stderr above)");
}

#[test]
fn install_writes_per_client_output_for_all_kinds() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);

    let report: InstallReport = install_from_local_path(&source, &target).expect("install");

    // Skill — written to .claude/skills/<n>/SKILL.md, .github/skills/<n>/SKILL.md,
    // .agents/skills/<n>/SKILL.md (opencode canonical-store).
    assert_file_eq(
        &target.join(".claude/skills/create-api-endpoint/SKILL.md"),
        &read_expected("claude", "create-api-endpoint.SKILL.md"),
        "claude skill",
    );
    assert_file_eq(
        &target.join(".github/skills/create-api-endpoint/SKILL.md"),
        &read_expected("copilot", "create-api-endpoint.SKILL.md"),
        "copilot skill",
    );
    assert_file_eq(
        &target.join(".agents/skills/create-api-endpoint/SKILL.md"),
        &read_expected("opencode", "create-api-endpoint.SKILL.md"),
        "opencode skill",
    );

    // Rule (api-conventions, scoped) — Claude .claude/rules/<n>.md,
    // Copilot .github/instructions/<n>.instructions.md, opencode
    // .agents/rules/<n>/RULE.md.
    assert_file_eq(
        &target.join(".claude/rules/api-conventions.md"),
        &read_expected("claude", "api-conventions.RULE.md"),
        "claude rule (scoped)",
    );
    assert_file_eq(
        &target.join(".github/instructions/api-conventions.instructions.md"),
        &read_expected("copilot", "api-conventions.RULE.md"),
        "copilot rule (scoped)",
    );
    assert_file_eq(
        &target.join(".agents/rules/api-conventions/RULE.md"),
        &read_expected("opencode", "api-conventions.RULE.md"),
        "opencode rule (scoped)",
    );

    // Rule (license-awareness, unscoped).
    assert_file_eq(
        &target.join(".claude/rules/license-awareness.md"),
        &read_expected("claude", "license-awareness.RULE.md"),
        "claude rule (unscoped)",
    );
    assert_file_eq(
        &target.join(".github/instructions/license-awareness.instructions.md"),
        &read_expected("copilot", "license-awareness.RULE.md"),
        "copilot rule (unscoped)",
    );
    assert_file_eq(
        &target.join(".agents/rules/license-awareness/RULE.md"),
        &read_expected("opencode", "license-awareness.RULE.md"),
        "opencode rule (unscoped)",
    );

    // Agent — Claude .claude/agents/<n>.md, Copilot .github/agents/<n>.agent.md,
    // opencode .opencode/agents/<n>.md.
    assert_file_eq(
        &target.join(".claude/agents/security-reviewer.md"),
        &read_expected("claude", "security-reviewer.AGENT.md"),
        "claude agent",
    );
    assert_file_eq(
        &target.join(".github/agents/security-reviewer.agent.md"),
        &read_expected("copilot", "security-reviewer.AGENT.md"),
        "copilot agent",
    );
    assert_file_eq(
        &target.join(".opencode/agents/security-reviewer.md"),
        &read_expected("opencode", "security-reviewer.AGENT.md"),
        "opencode agent",
    );

    // Report enumerates everything written.
    // 1 skill * 3 clients + 2 rules * 3 clients + 1 agent * 3 clients = 12.
    assert_eq!(report.items.len(), 12, "report item count");
}

#[test]
fn install_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);

    install_from_local_path(&source, &target).expect("install 1");
    let snapshot = read_target_tree(&target);

    install_from_local_path(&source, &target).expect("install 2");
    let snapshot2 = read_target_tree(&target);

    assert_eq!(snapshot, snapshot2, "second install must be byte-identical");
}

fn read_target_tree(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let p = entry.path();
            if p.is_dir() {
                walk(&p, root, out);
            } else {
                let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                let content = fs::read_to_string(&p).unwrap();
                out.push((rel, content));
            }
        }
    }
    walk(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn install_respects_top_level_audience_filter() {
    // Audience as a top-level field per format-spec §3.1 — the canonical
    // shape after PR #76. Pipeline emits only for listed clients.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");

    let skill_dir = source.join("skills/claude-only-top");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
schema: 1
name: claude-only-top
description: Use only when working in Claude Code; this skill targets claude alone for testing top-level audience filtering.
audience:
  - claude
---

## Body

Hello.
"#,
    )
    .unwrap();

    upskill::pipeline::install_from_local_path(&source, &target).expect("install");

    assert!(
        target
            .join(".claude/skills/claude-only-top/SKILL.md")
            .exists()
    );
    assert!(
        !target
            .join(".github/skills/claude-only-top/SKILL.md")
            .exists()
    );
    assert!(
        !target
            .join(".agents/skills/claude-only-top/SKILL.md")
            .exists()
    );
}

#[test]
fn install_respects_audience_filter() {
    // Back-compat: audience under metadata still works (v0.1-draft shape).
    // Pipeline reads top-level first, falls back to metadata.audience.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");

    let skill_dir = source.join("skills/claude-only");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
schema: 1
name: claude-only
description: Use only when working in Claude Code; this skill targets claude alone for testing audience filtering.
metadata:
  audience:
    - claude
---

## Body

Hello.
"#,
    )
    .unwrap();

    install_from_local_path(&source, &target).expect("install");

    assert!(
        target.join(".claude/skills/claude-only/SKILL.md").exists(),
        "claude output present"
    );
    assert!(
        !target.join(".github/skills/claude-only/SKILL.md").exists(),
        "copilot output absent (filtered by audience)"
    );
    assert!(
        !target.join(".agents/skills/claude-only/SKILL.md").exists(),
        "opencode output absent (filtered by audience)"
    );
}
