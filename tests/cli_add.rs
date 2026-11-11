//! ATDD tests for `upskill add <source>`.
//!
//! Drives the v0.2 install pipeline end-to-end via the CLI: parses an
//! `InstallSource`, runs `pipeline::install_with_lockfile`, and writes
//! per-client output for rules / skills / agents (format-spec §7).
//!
//! Local-path source only — no network. Git-source coverage is
//! in `tests/pipeline_source.rs` at the library level.

mod common;

use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn stage_source(source: &Path) {
    let from = format!("{FIXTURES}/items");
    copy_dir_all(Path::new(&from), source).unwrap();
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

#[test]
fn add_installs_local_ssot_to_cwd() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    // Mark `target` as a project directory so auto-fallback picks
    // project scope (cwd) instead of global ($HOME).
    fs::create_dir_all(target.join(".git")).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    // Report header includes the item count (12 outputs from
    // 1 skill + 2 rules + 1 agent × 3 clients).
    assert!(out.contains("Installed 12"), "stdout missing report: {out}");

    // Spot-check one output per client at the expected per-client paths.
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
    assert!(
        target
            .join(".github/instructions/api-conventions.instructions.md")
            .exists()
    );
    assert!(
        target
            .join(".opencode/agents/security-reviewer.md")
            .exists()
    );
}

#[test]
fn add_invalid_source_returns_usage_error() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "not a valid source"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn add_with_items_subset_installs_only_named_items() {
    // Spec §2.1: `upskill add <source> [items...]` filters the install
    // to a subset. Pick the skill (`create-api-endpoint`) and one rule
    // (`license-awareness`); leave the agent and the second rule out.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "create-api-endpoint",
            "license-awareness",
        ])
        .assert()
        .success();

    // Selected items are present.
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
    assert!(target.join(".claude/rules/license-awareness.md").exists());
    // Non-selected items are absent (skipped during the filtered walk).
    assert!(
        !target.join(".claude/agents/security-reviewer.md").exists(),
        "agent should not have been installed when not in the subset"
    );
    assert!(
        !target.join(".claude/rules/api-conventions.md").exists(),
        "second rule should not have been installed when not in the subset"
    );
}

#[test]
fn add_with_items_no_match_is_general_error() {
    // No name in the list matches anything in the source — the install
    // should refuse rather than silently emitting nothing.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "nope-not-real",
            "also-fictional",
        ])
        .assert()
        .failure()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("nope-not-real") || stderr.contains("no matching"),
        "stderr should name the missing items or say no match: {stderr}"
    );
}

/// Stage `source/<name>/SKILL.md` with `content` and a project target.
/// Returns `(tmp, home, source, target)`.
fn stage_single_skill(
    name: &str,
    content: &str,
) -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    stage_single_item(name, "SKILL.md", content)
}

/// Stage `source/<name>/<entrypoint>` with `content` and a project target.
/// Returns `(tmp, home, source, target)`.
fn stage_single_item(
    name: &str,
    entrypoint: &str,
    content: &str,
) -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    fs::create_dir_all(source.join(name)).unwrap();
    fs::write(source.join(name).join(entrypoint), content).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(target.join(".git")).unwrap();
    (tmp, home, source, target)
}

/// Run `upskill add <source> <name> --claude --project` from `target`.
fn add_one(home: &Path, source: &Path, target: &Path, name: &str) -> assert_cmd::assert::Assert {
    common::upskill_cmd(home)
        .current_dir(target)
        .args([
            "add",
            source.to_str().unwrap(),
            name,
            "--claude",
            "--project",
        ])
        .assert()
}

#[test]
fn add_accepts_skill_without_schema_field() {
    let (_tmp, home, source, target) = stage_single_skill(
        "plain",
        "---\nname: plain\ndescription: A standard Agent Skills file.\n---\nBody\n",
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "plain",
            "--claude",
            "--project",
        ])
        .assert()
        .success();

    assert!(target.join(".claude/skills/plain/SKILL.md").exists());
}

#[test]
fn add_accepts_rule_and_agent_without_schema_field() {
    for (entrypoint, output) in [
        ("RULE.md", ".claude/rules/plain.md"),
        ("AGENT.md", ".claude/agents/plain.md"),
    ] {
        let (_tmp, home, source, target) = stage_single_item(
            "plain",
            entrypoint,
            "---\nname: plain\ndescription: An item without schema.\n---\nBody\n",
        );
        add_one(&home, &source, &target, "plain").success();
        assert!(target.join(output).exists(), "{entrypoint}: {output}");
    }
}

#[test]
fn missing_required_field_error_names_the_file() {
    // `description` is still a required field; the error must name the file
    // for every item kind.
    for entrypoint in ["SKILL.md", "RULE.md", "AGENT.md"] {
        let (_tmp, home, source, target) =
            stage_single_item("broken", entrypoint, "---\nname: broken\n---\nBody\n");
        let assert = add_one(&home, &source, &target, "broken").failure();
        let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
        let expected = Path::new("broken").join(entrypoint);
        assert!(
            stderr.contains(expected.to_str().unwrap()),
            "{entrypoint}: stderr does not name the file: {stderr}"
        );
        assert!(
            stderr.contains("missing field `description`"),
            "{entrypoint}: stderr does not name the missing field: {stderr}"
        );
    }
}

#[test]
fn add_rejects_explicit_unsupported_schema() {
    for entrypoint in ["SKILL.md", "RULE.md", "AGENT.md"] {
        for (version, message) in [
            ("0", "schema version must be >= 1, got 0"),
            ("2", "schema version 2 is not supported"),
        ] {
            let (_tmp, home, source, target) = stage_single_item(
                "versioned",
                entrypoint,
                &format!("---\nschema: {version}\nname: versioned\ndescription: d\n---\nBody\n"),
            );
            let assert = add_one(&home, &source, &target, "versioned").failure();
            let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
            assert!(
                stderr.contains(message),
                "{entrypoint} schema {version}: {stderr}"
            );
            let expected = Path::new("versioned").join(entrypoint);
            assert!(
                stderr.contains(expected.to_str().unwrap()),
                "{entrypoint} schema {version}: stderr does not name the file: {stderr}"
            );
        }
    }
}

/// Stage a project at `<tmp>/proj` whose `<output_dir>/my-skill/` holds a
/// schema-less `SKILL.md` and one resource file. Returns `(tmp, home, proj)`.
fn stage_skill_in_client_output_dir(
    output_dir: &str,
) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    let item = proj.join(output_dir).join("my-skill");
    fs::create_dir_all(item.join("references")).unwrap();
    fs::write(
        item.join("SKILL.md"),
        "---\nname: my-skill\ndescription: d\n---\nBody\n",
    )
    .unwrap();
    fs::write(item.join("references/notes.md"), "user notes\n").unwrap();
    (tmp, home, proj)
}

fn assert_source_overlap_refused(output_dir: &str, extra_args: &[&str]) {
    let (_tmp, home, proj) = stage_skill_in_client_output_dir(output_dir);
    let item = proj.join(output_dir).join("my-skill");
    let skill_before = fs::read_to_string(item.join("SKILL.md")).unwrap();

    let source = format!("./{output_dir}");
    let mut args = vec!["add", source.as_str()];
    args.extend_from_slice(extra_args);
    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(&args)
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("my-skill") && stderr.contains("overlaps the output path"),
        "stderr does not explain the refusal: {stderr}"
    );
    assert!(
        stderr.contains(Path::new(output_dir).join("my-skill").to_str().unwrap()),
        "stderr does not name the source directory: {stderr}"
    );

    assert_eq!(
        fs::read_to_string(item.join("SKILL.md")).unwrap(),
        skill_before
    );
    assert_eq!(
        fs::read_to_string(item.join("references/notes.md")).unwrap(),
        "user notes\n"
    );
    assert!(!proj.join(".upskill-lock.json").exists());
}

#[test]
fn add_refuses_source_inside_claude_output_dir() {
    assert_source_overlap_refused(".claude/skills", &["--claude", "--project"]);
}

#[test]
fn add_refuses_source_inside_unselected_client_output_dir() {
    // `remove_item_outputs` cleans every client, so an `.agents/skills`
    // source is deleted even when only Claude is selected.
    assert_source_overlap_refused(".agents/skills", &["--claude", "--project"]);
}

/// Stage a global-scope registry at `$HOME/.claude`. Its item
/// `skills/my-skill` is the global Claude output path of `my-skill`; its item
/// `wanted` overlaps no output path. A bundle `pack` lists `wanted`.
/// Returns `(tmp, home, registry)`.
fn stage_registry_in_global_claude_dir()
-> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let registry = home.join(".claude");
    let other = registry.join("skills/my-skill");
    fs::create_dir_all(other.join("references")).unwrap();
    fs::write(
        other.join("SKILL.md"),
        "---\nname: my-skill\ndescription: d\n---\nBody\n",
    )
    .unwrap();
    fs::write(other.join("references/notes.md"), "user notes\n").unwrap();
    fs::create_dir_all(registry.join("wanted")).unwrap();
    fs::write(
        registry.join("wanted/SKILL.md"),
        "---\nname: wanted\ndescription: d\n---\nBody\n",
    )
    .unwrap();
    fs::write(
        registry.join("pack.bundle.yaml"),
        "schema: 1\nname: pack\ndescription: d\nitems:\n  skills: [wanted]\n",
    )
    .unwrap();
    (tmp, home, registry)
}

#[test]
fn add_ignores_overlap_of_an_item_that_is_not_requested() {
    // `wanted` alone uses the dependency closure; `pack wanted` mixes a bundle
    // name and an item name and uses the name-resolution path.
    for names in [&["wanted"][..], &["pack", "wanted"][..]] {
        let (tmp, home, registry) = stage_registry_in_global_claude_dir();
        let other = registry.join("skills/my-skill");
        let mut args = vec!["add", registry.to_str().unwrap()];
        args.extend_from_slice(names);
        args.extend_from_slice(&["--claude", "--global"]);

        common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(&args)
            .assert()
            .success();

        assert!(
            home.join(".claude/skills/wanted/SKILL.md").exists(),
            "{names:?}"
        );
        assert_eq!(
            fs::read_to_string(other.join("SKILL.md")).unwrap(),
            "---\nname: my-skill\ndescription: d\n---\nBody\n",
            "{names:?}"
        );
        assert_eq!(
            fs::read_to_string(other.join("references/notes.md")).unwrap(),
            "user notes\n",
            "{names:?}"
        );
    }
}

/// Write `files` (relative path, content) under `root`.
fn write_files(root: &Path, files: &[(&str, &str)]) {
    for (rel, content) in files {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
}

/// Run `add` with `args` in a project whose `files` are staged first, and
/// assert that the alias overlap is refused with every file unchanged.
fn assert_alias_overlap_refused(files: &[(&str, &str)], args: &[&str], item: &str) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    write_files(&proj, files);

    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(args)
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains(&format!("'{item}'")) && stderr.contains("overlaps the output path"),
        "stderr does not explain the refusal: {stderr}"
    );
    for (rel, content) in files {
        assert_eq!(
            fs::read_to_string(proj.join(rel)).unwrap(),
            *content,
            "{rel} changed"
        );
    }
    assert!(!proj.join(".upskill-lock.json").exists());
}

#[test]
fn add_refuses_alias_whose_output_contains_the_source() {
    assert_alias_overlap_refused(
        &[
            (
                ".claude/skills/bar/foo/SKILL.md",
                "---\nname: foo\ndescription: d\n---\nBody\n",
            ),
            (".claude/skills/bar/foo/scripts/run.sh", "echo run\n"),
            (".claude/skills/bar/notes.txt", "user notes\n"),
        ],
        &[
            "add",
            "./.claude/skills/bar",
            "--as",
            "bar",
            "--claude",
            "--project",
        ],
        "bar",
    );
}

#[test]
fn add_refuses_alias_whose_resource_dir_is_the_source() {
    assert_alias_overlap_refused(
        &[
            (
                ".claude/rules/bar/RULE.md",
                "---\nname: foo\ndescription: d\nignore: [draft.md]\n---\nBody\n",
            ),
            (".claude/rules/bar/extra.md", "extra\n"),
            (".claude/rules/bar/draft.md", "draft\n"),
        ],
        &[
            "add",
            "./.claude/rules",
            "--as",
            "foo=bar",
            "--claude",
            "--project",
        ],
        "bar",
    );
}
