//! ATDD for optional + relaxed item naming (layout-independent identity).
//!
//! - Skills: effective name is always the folder (Agent Skills standard);
//!   an absent `name:` falls back to the folder.
//! - Rules/agents: effective name is the frontmatter `name:` when present,
//!   else the folder — it may diverge from the folder.
//! - Lint: a skill whose `name:` diverges from its folder is an error; a
//!   rule whose `name:` diverges is silent.

mod common;

use std::fs;
use std::path::Path;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn skill_without_name_falls_back_to_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // Skill folder `demo/` with NO `name:` in its SKILL.md.
    write(
        &source.join("demo/SKILL.md"),
        "---\nschema: 1\ndescription: a demo skill with no name field.\n---\n\n## Body\n\nText.\n",
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    let out = target.join(".claude/skills/demo/SKILL.md");
    assert!(out.exists(), "expected {} to exist", out.display());
    let content = fs::read_to_string(&out).unwrap();
    assert!(
        content.contains("name: demo"),
        "skill frontmatter should carry the folder-derived name; got:\n{content}"
    );
}

#[test]
fn rule_name_diverges_from_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // Rule folder `stuff/` whose RULE.md declares a divergent name.
    write(
        &source.join("stuff/RULE.md"),
        "---\nschema: 1\nname: security-baseline\ndescription: a rule whose name diverges from its folder.\n---\n\n## Body\n\nText.\n",
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    // Output path + frontmatter follow the effective (frontmatter) name.
    let out = target.join(".claude/rules/security-baseline.md");
    assert!(out.exists(), "expected {} to exist", out.display());
    let content = fs::read_to_string(&out).unwrap();
    assert!(
        content.contains("name: security-baseline"),
        "rule frontmatter should carry the effective name; got:\n{content}"
    );
    // The folder name must NOT be used for the output path.
    assert!(
        !target.join(".claude/rules/stuff.md").exists(),
        "rule output must not use the folder name"
    );
}

#[test]
fn lint_errors_on_skill_name_folder_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");

    write(
        &registry.join("demo/SKILL.md"),
        "---\nschema: 1\nname: not-demo\ndescription: a skill whose name diverges from its folder.\n---\n\n## Body\n\nText.\n",
    );

    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", registry.to_str().unwrap()])
        .assert()
        .failure();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap()
        + &String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        out.contains("Agent Skills standard") || out.contains("directory"),
        "lint should mention the standard/directory; got:\n{out}"
    );
}

#[test]
fn lint_allows_rule_name_folder_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = tmp.path().join("registry");

    write(
        &registry.join("stuff/RULE.md"),
        "---\nschema: 1\nname: security-baseline\ndescription: a rule whose name diverges from its folder.\n---\n\n## Body\n\nText.\n",
    );

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", registry.to_str().unwrap()])
        .assert()
        .success();
}
