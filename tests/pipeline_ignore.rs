//! Integration coverage for the `ignore` field (format-spec §2.4):
//! supporting resources matching an item's `ignore` globs are dropped
//! before being copied into per-client output.

mod common;

use assert_cmd::Command;
use common::upskill_cmd;
use std::fs;
use std::path::PathBuf;

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
    fn cmd(&self) -> Command {
        let mut c = upskill_cmd(&self.home);
        c.current_dir(&self.proj);
        c
    }
}

#[test]
fn ignored_resource_subtree_is_not_copied() {
    let e = env();
    let dir = e.src.join("demo");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::create_dir_all(dir.join("refs")).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nschema: 1\nname: demo\ndescription: demo skill\nignore:\n  - scripts/**\n---\n\n## Demo\n\nSee [the reference](./refs/p.md).\n",
    )
    .unwrap();
    fs::write(dir.join("scripts/gate.sh"), "#!/bin/sh\necho gate\n").unwrap();
    fs::write(dir.join("refs/p.md"), "# Reference\n\nstuff\n").unwrap();

    e.cmd()
        .args(["add", e.src.to_str().unwrap()])
        .assert()
        .success();

    let claude = e.proj.join(".claude/skills/demo");
    assert!(
        claude.join("refs/p.md").is_file(),
        "non-ignored resource must be copied"
    );
    assert!(
        !claude.join("scripts/gate.sh").exists(),
        "resource under an ignored subtree must NOT be copied"
    );
    assert!(
        !claude.join("scripts").exists(),
        "ignored subtree directory must not be materialised"
    );
}
