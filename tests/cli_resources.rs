//! Integration coverage for format-spec §2.4 supporting-resource copying
//! (#199): resources travel with each rendered entrypoint; flat-kind
//! bodies are link-rewritten; remove/update reconcile; `--as` relocates the
//! resource directory and re-prefixes namespaced links (#200).

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
fn alias_relocates_dir_backed_skill_resources() {
    // Dir-backed kinds (all skills, opencode rules): the resource directory
    // IS the entrypoint's own `<name>/` dir, so `--as` must move the whole
    // directory to `<alias>/`. Links are not namespaced, so they are intact.
    let e = env();
    write_item(
        &e.src,
        "demo-skill",
        "SKILL.md",
        "## X\n\nRun [gate](./scripts/gate.sh). See [notes](./references/notes.md).\n",
    );

    e.cmd()
        .args(["add", e.src.to_str().unwrap(), "--as", "renamed"])
        .assert()
        .success();

    for base in [".claude/skills", ".github/skills", ".agents/skills"] {
        let aliased = e.proj.join(base).join("renamed");
        assert!(
            aliased.join("SKILL.md").is_file(),
            "{base}: entrypoint moved"
        );
        assert!(
            aliased.join("scripts/gate.sh").is_file(),
            "{base}: script relocated"
        );
        assert!(
            aliased.join("references/notes.md").is_file(),
            "{base}: ref relocated"
        );
        assert!(
            read(&aliased.join("SKILL.md")).contains("(./scripts/gate.sh)"),
            "{base}: dir-backed link must NOT be rewritten"
        );
        assert!(
            !e.proj.join(base).join("demo-skill").exists(),
            "{base}: original directory must be gone"
        );
    }
}

#[test]
fn alias_relocates_and_reprefixes_flat_rule_resources() {
    // Flat kinds (Claude/Copilot rules) keep resources in a sibling
    // `<name>/` namespace dir and namespace their links into it. `--as` must
    // move the entrypoint file, move the namespace dir, and re-prefix the
    // links from `<orig>/` to `<alias>/`. opencode rules are dir-backed.
    let e = env();
    write_item(
        &e.src,
        "demo-rule",
        "RULE.md",
        "## Gate\n\nRun [the gate](./scripts/gate.sh) in CI.\n",
    );

    e.cmd()
        .args(["add", e.src.to_str().unwrap(), "--as", "renamed"])
        .assert()
        .success();

    // Claude: flat entrypoint moved, namespace dir moved, link re-prefixed.
    let claude_md = e.proj.join(".claude/rules/renamed.md");
    assert!(claude_md.is_file(), "claude entrypoint renamed");
    assert!(
        e.proj
            .join(".claude/rules/renamed/scripts/gate.sh")
            .is_file(),
        "claude resource relocated to aliased namespace dir"
    );
    assert!(
        read(&claude_md).contains("(./renamed/scripts/gate.sh)"),
        "claude rule link must be re-prefixed to the aliased namespace dir"
    );
    assert!(!e.proj.join(".claude/rules/demo-rule.md").exists());
    assert!(!e.proj.join(".claude/rules/demo-rule").exists());

    // Copilot: same flat shape under .github/instructions/.
    let copilot_md = e.proj.join(".github/instructions/renamed.instructions.md");
    assert!(copilot_md.is_file());
    assert!(
        e.proj
            .join(".github/instructions/renamed/scripts/gate.sh")
            .is_file()
    );
    assert!(read(&copilot_md).contains("(./renamed/scripts/gate.sh)"));
    assert!(
        !e.proj
            .join(".github/instructions/demo-rule.instructions.md")
            .exists()
    );

    // opencode: directory-backed — whole dir moved, link unchanged.
    let oc = e.proj.join(".agents/rules/renamed");
    assert!(oc.join("scripts/gate.sh").is_file());
    assert!(read(&oc.join("RULE.md")).contains("(./scripts/gate.sh)"));
    assert!(!e.proj.join(".agents/rules/demo-rule").exists());
}

#[test]
fn alias_relocates_and_reprefixes_flat_agent_resources() {
    // Agents are flat on every client.
    let e = env();
    write_item(
        &e.src,
        "demo-agent",
        "AGENT.md",
        "## Agent\n\nUses [gate](./scripts/gate.sh).\n",
    );

    e.cmd()
        .args(["add", e.src.to_str().unwrap(), "--as", "renamed"])
        .assert()
        .success();

    for (entry, dir, orig_entry) in [
        (
            ".claude/agents/renamed.md",
            ".claude/agents/renamed",
            ".claude/agents/demo-agent.md",
        ),
        (
            ".github/agents/renamed.agent.md",
            ".github/agents/renamed",
            ".github/agents/demo-agent.agent.md",
        ),
        (
            ".opencode/agents/renamed.md",
            ".opencode/agents/renamed",
            ".opencode/agents/demo-agent.md",
        ),
    ] {
        assert!(
            e.proj.join(dir).join("scripts/gate.sh").is_file(),
            "{dir}: resource relocated"
        );
        assert!(
            read(&e.proj.join(entry)).contains("(./renamed/scripts/gate.sh)"),
            "{entry}: agent link re-prefixed"
        );
        assert!(!e.proj.join(orig_entry).exists(), "{orig_entry}: gone");
    }
}

#[test]
fn update_relocates_aliased_resource_item() {
    // Regression for the `update` deadlock noted on #200: `update` rebuilds
    // the alias from the lockfile and re-runs the install. A resource-bearing
    // aliased item must update cleanly, staying at its alias.
    let e = env();
    write_item(
        &e.src,
        "demo-rule",
        "RULE.md",
        "## Gate\n\nRun [the gate](./scripts/gate.sh) in CI.\n",
    );

    e.cmd()
        .args(["add", e.src.to_str().unwrap(), "--as", "renamed"])
        .assert()
        .success();

    e.cmd().args(["update"]).assert().success();

    let claude_md = e.proj.join(".claude/rules/renamed.md");
    assert!(
        claude_md.is_file(),
        "still installed at the alias after update"
    );
    assert!(
        e.proj
            .join(".claude/rules/renamed/scripts/gate.sh")
            .is_file(),
        "resource still relocated after update"
    );
    assert!(
        read(&claude_md).contains("(./renamed/scripts/gate.sh)"),
        "link still re-prefixed after update"
    );
    assert!(!e.proj.join(".claude/rules/demo-rule.md").exists());
}

#[test]
fn colocated_kinds_share_resources() {
    // One directory holding SKILL.md + AGENT.md (same name), sharing
    // references/. Each emitted entrypoint must get the resource.
    let e = env();
    let dir = e.src.join("paired");
    fs::create_dir_all(dir.join("references")).unwrap();
    for entry in ["SKILL.md", "AGENT.md"] {
        fs::write(
            dir.join(entry),
            "---\nschema: 1\nname: paired\ndescription: d\n---\n\n## P\n\n[n](./references/notes.md)\n",
        )
        .unwrap();
    }
    fs::write(dir.join("references/notes.md"), "# n\n").unwrap();

    e.cmd()
        .args(["add", e.src.to_str().unwrap()])
        .assert()
        .success();

    // Skill (dir-backed) gets it beside SKILL.md.
    assert!(
        e.proj
            .join(".claude/skills/paired/references/notes.md")
            .is_file()
    );
    // Agent (flat) gets it in the namespace dir.
    assert!(
        e.proj
            .join(".claude/agents/paired/references/notes.md")
            .is_file()
    );
}

#[test]
fn alias_on_bundle_source_does_not_crash_guard() {
    // The --as resource guard must skip bundle-file sources (local_source is
    // a file, not a directory) instead of erroring on iter_item_dirs. Bundle
    // item has no resources, so aliasing it succeeds.
    let e = env();
    let dir = e.src.join("demo-skill");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nschema: 1\nname: demo-skill\ndescription: d\n---\n\n## X\n\nbody\n",
    )
    .unwrap();
    fs::write(
        e.src.join("demo.bundle.yaml"),
        "schema: 1\nname: demo\ndescription: d\nitems:\n  skills:\n    - demo-skill\n",
    )
    .unwrap();

    e.cmd()
        .args([
            "add",
            e.src.join("demo.bundle.yaml").to_str().unwrap(),
            "--as",
            "demo-skill=renamed",
        ])
        .assert()
        .success();

    assert!(e.proj.join(".claude/skills/renamed/SKILL.md").is_file());
}

#[test]
fn bundle_install_copies_resources() {
    // Registry with one resource-bearing skill and a bundle that names it.
    let e = env();
    write_item(
        &e.src,
        "demo-skill",
        "SKILL.md",
        "## X\n\n[g](./scripts/gate.sh)\n",
    );
    fs::write(
        e.src.join("demo.bundle.yaml"),
        "schema: 1\nname: demo\ndescription: d\nitems:\n  skills:\n    - demo-skill\n",
    )
    .unwrap();

    e.cmd()
        .args(["add", e.src.join("demo.bundle.yaml").to_str().unwrap()])
        .assert()
        .success();

    assert!(
        e.proj
            .join(".claude/skills/demo-skill/scripts/gate.sh")
            .is_file(),
        "bundle-installed item must carry its resources"
    );
}
