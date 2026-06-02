//! Integration coverage for format-spec §2.4 supporting-resource copying
//! (#199): resources travel with each rendered entrypoint; flat-kind
//! bodies are link-rewritten; remove/update reconcile; `--as` is guarded.

mod common;

use assert_cmd::Command;
use common::upskill_cmd;
use std::fs;
use std::path::{Path, PathBuf};

/// Isolated test environment: a fake `$HOME`, an SSOT `src/` to author into,
/// and a `proj/` working dir marked with `.git` so `upskill` picks project
/// scope (cwd) instead of global ($HOME). See issue #193.
struct Env {
    _tmp: tempfile::TempDir,
    home: PathBuf,
    src: PathBuf,
    proj: PathBuf,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let src = tmp.path().join("src");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(proj.join(".git")).unwrap();
    Env {
        _tmp: tmp,
        home,
        src,
        proj,
    }
}

impl Env {
    /// `upskill` command with `$HOME` isolated and cwd set to the project.
    fn cmd(&self) -> Command {
        let mut c = upskill_cmd(&self.home);
        c.current_dir(&self.proj);
        c
    }
}

/// Write `SKILL.md`/`RULE.md`/`AGENT.md` + a script + a reference file into
/// `<root>/<name>/`. `entry` is the entrypoint filename.
fn write_item(root: &Path, name: &str, entry: &str, body: &str) {
    let dir = root.join(name);
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::create_dir_all(dir.join("references")).unwrap();
    let fm = format!("---\nschema: 1\nname: {name}\ndescription: demo {name}\n---\n\n{body}");
    fs::write(dir.join(entry), fm).unwrap();
    fs::write(dir.join("scripts/gate.sh"), "#!/bin/sh\necho gate\n").unwrap();
    fs::write(dir.join("references/notes.md"), "# Notes\n\nstuff\n").unwrap();
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap()
}

#[test]
fn skill_resources_copied_for_all_clients_no_rewrite() {
    let e = env();
    write_item(
        &e.src,
        "demo-skill",
        "SKILL.md",
        "## Demo\n\nRun [gate](./scripts/gate.sh). See [notes](./references/notes.md).\n",
    );

    e.cmd()
        .args(["add", e.src.to_str().unwrap()])
        .assert()
        .success();

    for base in [".claude/skills", ".github/skills", ".agents/skills"] {
        let dir = e.proj.join(base).join("demo-skill");
        assert!(
            dir.join("scripts/gate.sh").is_file(),
            "{base}: script copied"
        );
        assert!(
            dir.join("references/notes.md").is_file(),
            "{base}: ref copied"
        );
        // Directory-backed: body link is unchanged.
        assert!(
            read(&dir.join("SKILL.md")).contains("(./scripts/gate.sh)"),
            "{base}: skill body link must NOT be rewritten"
        );
    }
}

#[test]
fn rule_resources_namespaced_and_rewritten_per_client() {
    let e = env();
    write_item(
        &e.src,
        "demo-rule",
        "RULE.md",
        "## Gate\n\nRun [the gate](./scripts/gate.sh) in CI.\n",
    );

    e.cmd()
        .args(["add", e.src.to_str().unwrap()])
        .assert()
        .success();

    // Claude: flat entrypoint + sibling namespace dir + rewritten link.
    let claude_md = e.proj.join(".claude/rules/demo-rule.md");
    assert!(claude_md.is_file());
    assert!(
        e.proj
            .join(".claude/rules/demo-rule/scripts/gate.sh")
            .is_file(),
        "claude resource in sibling namespace dir"
    );
    assert!(
        read(&claude_md).contains("(./demo-rule/scripts/gate.sh)"),
        "claude rule link must be rewritten to the namespace dir"
    );

    // Copilot: same shape under .github/instructions/.
    assert!(
        e.proj
            .join(".github/instructions/demo-rule/scripts/gate.sh")
            .is_file()
    );
    assert!(
        read(
            &e.proj
                .join(".github/instructions/demo-rule.instructions.md")
        )
        .contains("(./demo-rule/scripts/gate.sh)")
    );

    // opencode: directory-backed — resources beside RULE.md, link unchanged.
    let oc = e.proj.join(".agents/rules/demo-rule");
    assert!(oc.join("scripts/gate.sh").is_file());
    assert!(
        read(&oc.join("RULE.md")).contains("(./scripts/gate.sh)"),
        "opencode rule link must NOT be rewritten"
    );
}

#[test]
fn agent_resources_namespaced_and_rewritten_all_clients() {
    let e = env();
    write_item(
        &e.src,
        "demo-agent",
        "AGENT.md",
        "## Agent\n\nUses [gate](./scripts/gate.sh).\n",
    );

    e.cmd()
        .args(["add", e.src.to_str().unwrap()])
        .assert()
        .success();

    for (entry, dir) in [
        (".claude/agents/demo-agent.md", ".claude/agents/demo-agent"),
        (
            ".github/agents/demo-agent.agent.md",
            ".github/agents/demo-agent",
        ),
        (
            ".opencode/agents/demo-agent.md",
            ".opencode/agents/demo-agent",
        ),
    ] {
        assert!(
            e.proj.join(dir).join("scripts/gate.sh").is_file(),
            "{dir}: resource copied"
        );
        assert!(
            read(&e.proj.join(entry)).contains("(./demo-agent/scripts/gate.sh)"),
            "{entry}: agent link rewritten"
        );
    }
}

#[test]
fn audience_scopes_resource_copy() {
    let e = env();
    let dir = e.src.join("only-claude");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nschema: 1\nname: only-claude\ndescription: d\naudience:\n  - claude\n---\n\n## X\n\n[g](./scripts/gate.sh)\n",
    )
    .unwrap();
    fs::write(dir.join("scripts/gate.sh"), "x").unwrap();

    e.cmd()
        .args(["add", e.src.to_str().unwrap()])
        .assert()
        .success();

    assert!(
        e.proj
            .join(".claude/skills/only-claude/scripts/gate.sh")
            .is_file()
    );
    assert!(!e.proj.join(".github/skills/only-claude").exists());
    assert!(!e.proj.join(".agents/skills/only-claude").exists());
}

#[test]
fn remove_deletes_resource_tree() {
    let e = env();
    write_item(
        &e.src,
        "demo-rule",
        "RULE.md",
        "## G\n\n[g](./scripts/gate.sh)\n",
    );

    e.cmd()
        .args(["add", e.src.to_str().unwrap()])
        .assert()
        .success();
    assert!(
        e.proj
            .join(".claude/rules/demo-rule/scripts/gate.sh")
            .is_file()
    );

    e.cmd().args(["remove", "demo-rule"]).assert().success();

    assert!(!e.proj.join(".claude/rules/demo-rule.md").exists());
    assert!(
        !e.proj.join(".claude/rules/demo-rule").exists(),
        "resource namespace dir removed"
    );
}

#[test]
fn readd_is_idempotent() {
    let e = env();
    write_item(
        &e.src,
        "demo-skill",
        "SKILL.md",
        "## X\n\n[g](./scripts/gate.sh)\n",
    );

    let run = || {
        e.cmd()
            .args(["add", e.src.to_str().unwrap(), "--force"])
            .assert()
            .success();
    };
    run();
    let first = read(&e.proj.join(".claude/skills/demo-skill/SKILL.md"));
    run();
    let second = read(&e.proj.join(".claude/skills/demo-skill/SKILL.md"));
    assert_eq!(first, second, "re-add must be byte-identical");
}

#[test]
fn update_removes_stale_resource_after_source_deletes_it() {
    let e = env();
    write_item(
        &e.src,
        "demo-skill",
        "SKILL.md",
        "## X\n\n[g](./scripts/gate.sh)\n",
    );

    e.cmd()
        .args(["add", e.src.to_str().unwrap()])
        .assert()
        .success();
    let copied = e.proj.join(".claude/skills/demo-skill/references/notes.md");
    assert!(copied.is_file());

    // Delete a resource from the SSOT source, then update.
    fs::remove_file(e.src.join("demo-skill/references/notes.md")).unwrap();
    e.cmd().args(["update"]).assert().success();

    assert!(
        !copied.exists(),
        "stale resource must be cleaned (remove_item_outputs runs before re-copy)"
    );
}

#[test]
fn alias_on_resource_item_is_rejected() {
    let e = env();
    write_item(
        &e.src,
        "demo-skill",
        "SKILL.md",
        "## X\n\n[g](./scripts/gate.sh)\n",
    );

    e.cmd()
        .args(["add", e.src.to_str().unwrap(), "--as", "renamed"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "aliasing items with supporting resources is not yet supported",
        ));

    // Nothing written.
    assert!(!e.proj.join(".claude/skills/renamed").exists());
    assert!(!e.proj.join(".claude/skills/demo-skill").exists());
}
