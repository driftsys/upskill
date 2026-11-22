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

/// `rel`, written with `/`, as a path built from its components, so that it
/// uses the native separator as the paths that the binary prints do.
fn native(rel: &str) -> std::path::PathBuf {
    rel.split('/').collect()
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
        stderr.contains(native(&format!("{output_dir}/my-skill")).to_str().unwrap()),
        "stderr does not name the source directory: {stderr}"
    );
    let output = fs::canonicalize(&proj)
        .unwrap()
        .join(native(&format!("{output_dir}/my-skill/SKILL.md")));
    assert!(
        stderr.contains(&format!("overlaps the output path {};", output.display())),
        "stderr does not name the output path: {stderr}"
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
    // `check_source_outside_outputs` checks the output paths of every
    // client, because an install can delete recorded files of a client that
    // it does not write. An `.agents/skills` source is refused even when
    // only Claude is selected.
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

/// Every file under `root`, as (path, content), sorted by path.
fn snapshot_files(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let content = fs::read(&path).unwrap();
                files.push((path, content));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn add_and_update_explain_a_source_rooted_in_a_client_directory() {
    // The first `add` writes `$HOME/.claude/skills/wanted`, which is inside
    // the registry root `$HOME/.claude`. Discovery then finds that output as a
    // second item `wanted`, so the second `add` and `update` are refused.
    let (tmp, home, registry) = stage_registry_in_global_claude_dir();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            registry.to_str().unwrap(),
            "wanted",
            "--claude",
            "--global",
        ])
        .assert()
        .success();
    let lock_path = home.join(".upskill-lock.json");
    let lock_before = fs::read_to_string(&lock_path).unwrap();
    let files_before = snapshot_files(&registry);
    let root = fs::canonicalize(&registry).unwrap();

    let runs: [&[&str]; 2] = [
        &[
            "add",
            registry.to_str().unwrap(),
            "wanted",
            "--claude",
            "--global",
        ],
        &["update", "--global"],
    ];
    for args in runs {
        let assert = common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(args)
            .assert()
            .failure();
        let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
        assert!(
            stderr.contains(&format!(
                "The source root {} contains the output path {}",
                root.display(),
                root.join("skills").join("wanted").display()
            )),
            "{args:?}: stderr does not name the source root: {stderr}"
        );
        assert!(
            stderr.contains(&format!("under the client directory {}", root.display())),
            "{args:?}: stderr does not name the client directory: {stderr}"
        );
        assert!(
            stderr.contains("A source must not be rooted in a client directory or contain one")
                && stderr.contains(
                    "Move the source items to a directory outside the client directories \
                     and add them from there"
                ),
            "{args:?}: stderr does not state the remedy: {stderr}"
        );
        assert_eq!(snapshot_files(&registry), files_before, "{args:?}");
        assert_eq!(
            fs::read_to_string(&lock_path).unwrap(),
            lock_before,
            "{args:?}"
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
/// assert that the overlap is refused with every file unchanged. When
/// `output` is given, also assert that the refusal names that output path,
/// relative to the canonical project directory.
fn assert_overlap_refused(files: &[(&str, &str)], args: &[&str], item: &str, output: Option<&str>) {
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
    if let Some(output) = output {
        let output = fs::canonicalize(&proj).unwrap().join(native(output));
        assert!(
            stderr.contains(&format!("overlaps the output path {};", output.display())),
            "stderr does not name the output path: {stderr}"
        );
    }
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
    assert_overlap_refused(
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
        Some(".claude/skills/bar"),
    );
}

#[test]
fn add_refuses_alias_whose_resource_dir_is_the_source() {
    assert_overlap_refused(
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
        Some(".claude/rules/bar"),
    );
}

#[test]
fn add_checks_an_aliased_item_only_under_its_alias() {
    // The item `foo` lives at `.claude/skills/foo`, the Claude output path of
    // the name `foo`. The alias `foo=bar` installs it at `.claude/skills/bar`,
    // which does not overlap the source item. This test covers the first
    // `add` only: the source then holds upskill's output, which the next
    // `add` refuses (see the unsupported source layout in commands.md).
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    let body = "---\nname: foo\ndescription: d\n---\nBody\n";
    write_files(&proj, &[(".claude/skills/foo/SKILL.md", body)]);

    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args([
            "add",
            "./.claude/skills",
            "--as",
            "foo=bar",
            "--claude",
            "--project",
        ])
        .assert()
        .success();
    assert!(proj.join(".claude/skills/bar/SKILL.md").is_file());
    assert_eq!(
        fs::read_to_string(proj.join(".claude/skills/foo/SKILL.md")).unwrap(),
        body
    );
}

/// Run `add` with `args` in a project whose `files` are staged first, and
/// assert that a cross-item overlap is refused with exit code 1: the source
/// directory of skill `inner` is inside the output path of skill `outer`.
/// Every file stays unchanged, `unwritten` is not created and no lockfile
/// is written.
fn assert_cross_item_overlap_refused(
    files: &[(&str, &str)],
    args: &[&str],
    inner: &str,
    outer: &str,
    unwritten: &str,
) {
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
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains(&format!("the source directory of skill '{inner}' ("))
            && stderr.contains(") is inside the output path")
            && stderr.contains(&format!("of skill '{outer}'; installing would delete")),
        "stderr does not explain the refusal: {stderr}"
    );
    for (rel, content) in files {
        assert_eq!(
            fs::read_to_string(proj.join(rel)).unwrap(),
            *content,
            "{rel} changed"
        );
    }
    assert!(!proj.join(unwritten).exists(), "{unwritten} written");
    assert!(!proj.join(".upskill-lock.json").exists());
}

#[test]
fn add_refuses_source_item_inside_the_output_path_of_another_item() {
    // `q` installs at `.claude/skills/q`, which holds the source of `y`.
    assert_cross_item_overlap_refused(
        &[
            (
                ".claude/skills/grp/q/SKILL.md",
                "---\nname: q\ndescription: d\n---\nBody\n",
            ),
            (
                ".claude/skills/q/y/SKILL.md",
                "---\nname: y\ndescription: d\n---\nBody\n",
            ),
        ],
        &["add", "./.claude/skills", "--claude", "--project"],
        "y",
        "q",
        ".claude/skills/q/SKILL.md",
    );
}

#[test]
fn add_refuses_source_item_inside_the_output_path_of_an_aliased_item() {
    // `p` installs as `bar` at `.claude/skills/bar`, which holds the source
    // of `y`.
    assert_cross_item_overlap_refused(
        &[
            (
                ".claude/skills/p/SKILL.md",
                "---\nname: p\ndescription: d\n---\nBody\n",
            ),
            (
                ".claude/skills/bar/y/SKILL.md",
                "---\nname: y\ndescription: d\n---\nBody\n",
            ),
        ],
        &[
            "add",
            "./.claude/skills",
            "--as",
            "p=bar",
            "--claude",
            "--project",
        ],
        "y",
        "bar",
        ".claude/skills/bar/SKILL.md",
    );
}

#[test]
fn add_refuses_overlap_with_a_source_item_that_is_not_installed() {
    // Only `q` is requested. The source of `y` is not installed, but `q`
    // would write into it, so the install is refused.
    assert_cross_item_overlap_refused(
        &[
            (
                ".claude/skills/grp/q/SKILL.md",
                "---\nname: q\ndescription: d\n---\nBody\n",
            ),
            (
                ".claude/skills/q/y/SKILL.md",
                "---\nname: y\ndescription: d\n---\nBody\n",
            ),
        ],
        &["add", "./.claude/skills", "q", "--claude", "--project"],
        "y",
        "q",
        ".claude/skills/q/SKILL.md",
    );
}

#[test]
fn add_ignores_the_output_paths_of_an_excluded_item() {
    // `q` is excluded, so nothing is written at `.claude/skills/q`, which
    // holds the source of `y`. `y` installs. Without names, `add` uses the
    // item closure, which already drops `q`; with the bundle name `pack`,
    // the bundle lists `q`, and only `--exclude` drops it.
    for names in [&[][..], &["pack"][..]] {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(proj.join(".git")).unwrap();
        write_files(
            &proj,
            &[
                (
                    ".claude/skills/grp/q/SKILL.md",
                    "---\nname: q\ndescription: d\n---\nBody\n",
                ),
                (
                    ".claude/skills/q/y/SKILL.md",
                    "---\nname: y\ndescription: d\n---\nBody\n",
                ),
                (
                    ".claude/skills/pack.bundle.yaml",
                    "schema: 1\nname: pack\ndescription: d\nitems:\n  skills: [q, y]\n",
                ),
            ],
        );

        common::upskill_cmd(&home)
            .current_dir(&proj)
            .arg("add")
            .arg("./.claude/skills")
            .args(names)
            .args(["--exclude", "q", "--claude", "--project"])
            .assert()
            .success();
        assert!(
            proj.join(".claude/skills/y/SKILL.md").is_file(),
            "{names:?}"
        );
        assert!(
            !proj.join(".claude/skills/q/SKILL.md").exists(),
            "{names:?}"
        );
    }
}

#[test]
fn add_refuses_overlap_with_an_excluded_source_item() {
    // `y` is excluded, but `q` would write into the source of `y`.
    assert_cross_item_overlap_refused(
        &[
            (
                ".claude/skills/grp/q/SKILL.md",
                "---\nname: q\ndescription: d\n---\nBody\n",
            ),
            (
                ".claude/skills/q/y/SKILL.md",
                "---\nname: y\ndescription: d\n---\nBody\n",
            ),
        ],
        &[
            "add",
            "./.claude/skills",
            "--exclude",
            "y",
            "--claude",
            "--project",
        ],
        "y",
        "q",
        ".claude/skills/q/SKILL.md",
    );
}

#[test]
fn add_does_not_check_an_excluded_item_against_its_own_output_paths() {
    // `foo` lives at `.claude/skills/foo`, its own Claude output path, and is
    // excluded, so nothing is written for it and it is not refused. Without
    // names, `add` uses the item closure; with the bundle name `pack`, `add`
    // uses the bundle closure, which lists `foo`.
    for names in [&[][..], &["pack"][..]] {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(proj.join(".git")).unwrap();
        let foo = "---\nname: foo\ndescription: d\n---\nBody\n";
        write_files(
            &proj,
            &[
                (".claude/skills/foo/SKILL.md", foo),
                (
                    ".claude/skills/grp/bar/SKILL.md",
                    "---\nname: bar\ndescription: d\n---\nBody\n",
                ),
                (
                    ".claude/skills/pack.bundle.yaml",
                    "schema: 1\nname: pack\ndescription: d\nitems:\n  skills: [foo, bar]\n",
                ),
            ],
        );

        common::upskill_cmd(&home)
            .current_dir(&proj)
            .arg("add")
            .arg("./.claude/skills")
            .args(names)
            .args(["--exclude", "foo", "--claude", "--project"])
            .assert()
            .success();
        assert!(
            proj.join(".claude/skills/bar/SKILL.md").is_file(),
            "{names:?}"
        );
        assert_eq!(
            fs::read_to_string(proj.join(".claude/skills/foo/SKILL.md")).unwrap(),
            foo,
            "{names:?}"
        );
        // The lockfile records no item `foo` (a bundle entry may list it).
        let lock: serde_json::Value =
            serde_json::from_slice(&fs::read(proj.join(".upskill-lock.json")).unwrap()).unwrap();
        assert!(
            lock["items"]
                .as_array()
                .unwrap()
                .iter()
                .all(|item| item["name"] != "foo"),
            "{names:?}: {lock}"
        );
    }
}

#[test]
fn add_refuses_a_source_item_that_contains_the_output_path_of_another_item() {
    // The source is the project. Its skill in the folder `.claude` (a skill
    // is named after its folder; not requested) contains `.claude/skills/q`,
    // the output path of `q`.
    let c = "---\nname: c\ndescription: d\n---\nBody\n";
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    write_files(
        &proj,
        &[
            (".claude/SKILL.md", c),
            (
                "src/q/SKILL.md",
                "---\nname: q\ndescription: d\n---\nBody\n",
            ),
        ],
    );

    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["add", "./", "q", "--claude", "--project"])
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("the source directory of skill '.claude' (")
            && stderr.contains(") contains the output path")
            && stderr.contains("of skill 'q'; installing would delete"),
        "stderr does not explain the refusal: {stderr}"
    );
    assert_eq!(
        fs::read_to_string(proj.join(".claude/SKILL.md")).unwrap(),
        c
    );
    assert!(!proj.join(".claude/skills/q").exists());
    assert!(!proj.join(".upskill-lock.json").exists());
}

#[test]
fn add_checks_an_item_that_its_audience_skips_as_installed() {
    // `q` lists only Copilot in `audience`, so `--claude` installs nothing
    // for it. The overlap check does not apply `audience`: the Claude output
    // path `.claude/skills/q` of `q` holds the source of `y`, and the install
    // is refused.
    assert_cross_item_overlap_refused(
        &[
            (
                ".claude/skills/grp/q/SKILL.md",
                "---\nname: q\ndescription: d\naudience:\n  - copilot\n---\nBody\n",
            ),
            (
                ".claude/skills/q/y/SKILL.md",
                "---\nname: y\ndescription: d\n---\nBody\n",
            ),
        ],
        &["add", "./.claude/skills", "--claude", "--project"],
        "y",
        "q",
        ".claude/skills/y/SKILL.md",
    );
}

#[test]
fn add_refuses_source_inside_copilot_skills_dir() {
    assert_source_overlap_refused(".github/skills", &["--copilot", "--project"]);
}

#[test]
fn add_refuses_source_inside_copilot_instructions_dir() {
    assert_overlap_refused(
        &[
            (
                ".github/instructions/my-rule/RULE.md",
                "---\nname: my-rule\ndescription: d\n---\nBody\n",
            ),
            (".github/instructions/my-rule/extra.md", "extra\n"),
        ],
        &["add", "./.github/instructions", "--copilot", "--project"],
        "my-rule",
        Some(".github/instructions/my-rule"),
    );
}

#[test]
fn add_refuses_agent_source_inside_claude_agents_dir() {
    assert_overlap_refused(
        &[
            (
                ".claude/agents/my-agent/AGENT.md",
                "---\nname: my-agent\ndescription: d\n---\nBody\n",
            ),
            (".claude/agents/my-agent/extra.md", "extra\n"),
        ],
        &["add", "./.claude/agents", "--claude", "--project"],
        "my-agent",
        Some(".claude/agents/my-agent"),
    );
}

#[test]
fn add_refuses_overlap_on_the_name_resolution_path() {
    // `pack my-skill` mixes a bundle name and an item name, so `add` uses the
    // name-resolution path; the requested item `my-skill` overlaps its own
    // global Claude output path.
    let (tmp, home, registry) = stage_registry_in_global_claude_dir();
    let other = registry.join("skills/my-skill");
    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            registry.to_str().unwrap(),
            "pack",
            "my-skill",
            "--claude",
            "--global",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("'my-skill'") && stderr.contains("overlaps the output path"),
        "stderr does not explain the refusal: {stderr}"
    );
    assert_eq!(
        fs::read_to_string(other.join("SKILL.md")).unwrap(),
        "---\nname: my-skill\ndescription: d\n---\nBody\n"
    );
    assert_eq!(
        fs::read_to_string(other.join("references/notes.md")).unwrap(),
        "user notes\n"
    );
    assert!(!home.join(".claude/skills/wanted").exists());
    assert!(!home.join(".upskill-lock.json").exists());
}

#[test]
fn add_applies_an_alias_only_to_the_item_it_names() {
    // The rule in folder `bar` is named `baz`; its directory is the resource
    // base of a rule named `bar`. The alias `foo=bar` renames only the skill
    // `foo`, so the rule `baz` is not checked under the name `bar`.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    // Both items live under the source `.claude/rules`.
    write_files(
        &proj,
        &[
            (
                ".claude/rules/foo/SKILL.md",
                "---\nname: foo\ndescription: d\n---\nBody\n",
            ),
            (
                ".claude/rules/bar/RULE.md",
                "---\nname: baz\ndescription: d\n---\nBody\n",
            ),
        ],
    );

    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args([
            "add",
            "./.claude/rules",
            "--as",
            "foo=bar",
            "--claude",
            "--project",
        ])
        .assert()
        .success();
    assert!(proj.join(".claude/skills/bar/SKILL.md").exists());
    assert!(proj.join(".claude/rules/baz.md").exists());
    assert!(proj.join(".claude/rules/bar/RULE.md").exists());
}

#[test]
fn add_force_prints_the_replaced_file() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let skill = tmp.path().join("source").join("res");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nschema: 1\nname: res\ndescription: Use for res\n---\nBody.\n",
    )
    .unwrap();
    let dest = tmp
        .path()
        .join(".claude")
        .join("skills")
        .join("res")
        .join("SKILL.md");
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    fs::write(&dest, "not managed\n").unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "res", "--claude", "--project", "--force"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(
        stdout.contains("Replaced or deleted 1 modified or unmanaged file(s) (--force)"),
        "{stdout}"
    );
    let root = tmp.path().display().to_string();
    let listed = stdout.lines().any(|line| {
        line.starts_with("  ") && line.trim_end().ends_with("SKILL.md") && !line.contains(&root)
    });
    assert!(listed, "{stdout}");
    assert_ne!(fs::read(&dest).unwrap(), b"not managed\n");
}

/// A project with the skill `res` (with `scripts/a.sh`) installed for
/// Claude from `./source`.
fn res_with_script_project() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let skill = tmp.path().join("source").join("res");
    fs::create_dir_all(skill.join("scripts")).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nschema: 1\nname: res\ndescription: Use for res\n---\nBody.\n",
    )
    .unwrap();
    fs::write(skill.join("scripts").join("a.sh"), "echo a\n").unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "res", "--claude", "--project"])
        .assert()
        .success();
    (tmp, home)
}

fn installed_script(root: &Path) -> std::path::PathBuf {
    root.join(".claude")
        .join("skills")
        .join("res")
        .join("scripts")
        .join("a.sh")
}

#[test]
fn add_force_deletes_and_lists_a_modified_file_the_source_no_longer_contains() {
    let (tmp, home) = res_with_script_project();
    let script = installed_script(tmp.path());
    fs::write(&script, "my edit\n").unwrap();
    fs::remove_dir_all(tmp.path().join("source").join("res").join("scripts")).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./source", "res", "--claude", "--project", "--force"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(
        stdout.contains("Replaced or deleted 1 modified or unmanaged file(s) (--force)"),
        "{stdout}"
    );
    assert!(
        stdout
            .lines()
            .any(|line| line.starts_with("  ") && line.trim_end().ends_with("a.sh")),
        "{stdout}"
    );
    assert!(!script.exists());
}

#[test]
fn add_force_with_quiet_prints_no_list() {
    let (tmp, home) = res_with_script_project();
    fs::write(installed_script(tmp.path()), "my edit\n").unwrap();
    fs::remove_dir_all(tmp.path().join("source").join("res").join("scripts")).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args([
            "add",
            "./source",
            "res",
            "--claude",
            "--project",
            "--force",
            "--quiet",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(!stdout.contains("Replaced or deleted"), "{stdout}");
    assert!(!installed_script(tmp.path()).exists());
}

/// Names of the direct entries of `dir`, sorted.
fn entry_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn add_of_a_missing_local_path_fails_and_writes_nothing_in_the_project_scope() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    let before = entry_names(&proj);
    let missing = proj.join("missing");

    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["add", missing.to_str().unwrap(), "--claude"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();

    assert!(
        stderr.contains(&format!("source path {} does not exist", missing.display())),
        "{stderr}"
    );
    assert!(!proj.join(".upskill-lock.json").exists());
    assert!(!proj.join("CLAUDE.md").exists());
    assert_eq!(entry_names(&proj), before);
}

#[test]
fn add_of_a_missing_local_path_fails_and_writes_nothing_in_the_global_scope() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let before = entry_names(&home);
    let missing = tmp.path().join("missing");

    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", missing.to_str().unwrap(), "--claude", "--global"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();

    assert!(
        stderr.contains(&format!("source path {} does not exist", missing.display())),
        "{stderr}"
    );
    assert!(!home.join(".upskill-lock.json").exists());
    assert!(!home.join("CLAUDE.md").exists());
    assert_eq!(entry_names(&home), before);
}

#[test]
fn add_of_an_existing_local_path_without_an_item_installs_nothing_and_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    let empty = tmp.path().join("empty-source");
    fs::create_dir_all(&empty).unwrap();

    common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["add", empty.to_str().unwrap(), "--claude"])
        .assert()
        .code(0)
        .stdout(predicates::str::contains("Installed 0 files"));
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(proj.join(".upskill-lock.json")).unwrap())
            .unwrap();
    assert_eq!(lock["items"].as_array().unwrap().len(), 0, "{lock}");
    assert!(proj.join("CLAUDE.md").exists());
}

#[test]
fn add_of_a_relative_missing_local_path_prints_the_absolute_path() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    let before = entry_names(&proj);
    // The spelling of the working directory in the message differs between
    // platforms (a symlinked temp dir, a Windows short name), so the test
    // checks the end of the path and that the typed form is not printed.
    let expected = Path::new("proj").join("missing");

    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["add", "./missing", "--claude"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();

    assert!(stderr.contains("source path "), "{stderr}");
    assert!(
        stderr.contains(&format!("{} does not exist", expected.display())),
        "{stderr}"
    );
    assert!(!stderr.contains("./missing"), "{stderr}");
    assert_eq!(entry_names(&proj), before);
}

#[test]
fn add_of_an_item_that_requires_a_missing_local_source_fails_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join(".git")).unwrap();
    let gone = tmp.path().join("reg-gone");
    let reg = tmp.path().join("reg-a");
    let dir = reg.join("code-review");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("AGENT.md"),
        format!(
            "---\nschema: 1\nname: code-review\ndescription: test\nrequires:\n  skills: [{{ name: sarif, source: {} }}]\n---\n# code-review\n",
            gone.display()
        ),
    )
    .unwrap();
    let before = entry_names(&proj);

    let assert = common::upskill_cmd(&home)
        .current_dir(&proj)
        .args(["add", reg.to_str().unwrap(), "code-review", "--claude"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();

    assert!(
        stderr.contains(&format!(
            "fetch requires source `local:{}`: source path {} does not exist",
            gone.display(),
            gone.display()
        )),
        "{stderr}"
    );
    assert!(!proj.join(".upskill-lock.json").exists());
    assert!(!proj.join("CLAUDE.md").exists());
    assert_eq!(entry_names(&proj), before);
}

fn stage_kind_source() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let (tmp, home, source, target) = stage_single_item(
        "r",
        "RULE.md",
        "---\nname: r\ndescription: A rule.\n---\nRule body.\n",
    );
    write_files(
        &source,
        &[
            (
                "s/SKILL.md",
                "---\nname: s\ndescription: A skill.\n---\nSkill body.\n",
            ),
            (
                "g/AGENT.md",
                "---\nname: g\ndescription: An agent.\n---\nAgent body.\n",
            ),
            (
                "bundles/bun.bundle.yaml",
                "schema: 1\nname: bun\ndescription: A mixed bundle.\nitems:\n  rules: [r]\n  skills: [s]\n  agents: [g]\nplugins:\n  kind-plugin:\n    claude:\n      source: example/plugins\n      marketplace: kind-marketplace\n      plugin: kind-plugin\nmcps:\n  kind-server:\n    local:\n      command: kind-server-command\n",
            ),
        ],
    );
    (tmp, home, source, target)
}

fn read_kind_lock(target: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(target.join(".upskill-lock.json")).unwrap()).unwrap()
}

#[test]
fn add_kind_rule_installs_only_the_rules_of_a_bundle() {
    let (_tmp, home, source, target) = stage_kind_source();
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "bun",
            "--kind",
            "rule",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert!(target.join(".claude/rules/r.md").exists());
    assert!(!target.join(".claude/skills").exists());
    assert!(!target.join(".claude/agents").exists());
    let lock = read_kind_lock(&target);
    let items = lock["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["kind"], "rule");
    assert_eq!(items[0]["name"], "r");
}

#[test]
fn add_kind_is_repeatable() {
    let (_tmp, home, source, target) = stage_kind_source();
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "bun",
            "--kind",
            "rule",
            "--kind",
            "agent",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert!(target.join(".claude/rules/r.md").exists());
    assert!(target.join(".claude/agents/g.md").exists());
    assert!(!target.join(".claude/skills").exists());
    assert_eq!(
        read_kind_lock(&target)["items"].as_array().unwrap().len(),
        2
    );
}

#[cfg(unix)]
#[test]
fn add_kind_skips_the_plugins_and_mcp_servers_of_a_bundle() {
    use std::os::unix::fs::PermissionsExt;
    let (tmp, home, source, target) = stage_kind_source();
    let bin = tmp.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let script = bin.join("claude");
    let marker = tmp.path().join("claude-called");
    fs::write(&script, "#!/bin/sh\n: > \"$KIND_MARKER\"\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let path = std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    )))
    .unwrap();
    let result = common::upskill_cmd(&home)
        .current_dir(&target)
        .env("PATH", path)
        .env("KIND_MARKER", &marker)
        .args([
            "add",
            source.to_str().unwrap(),
            "bun",
            "--kind",
            "rule",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert!(!marker.exists());
    assert!(!target.join(".mcp.json").exists());
    let lock = read_kind_lock(&target);
    assert!(lock.get("plugins").is_none());
    assert!(lock.get("mcps").is_none());
    for bytes in [&result.get_output().stdout, &result.get_output().stderr] {
        let text = String::from_utf8_lossy(bytes);
        assert!(!text.contains("kind-plugin"), "{text}");
        assert!(!text.contains("kind-server"), "{text}");
    }
}

#[test]
fn add_kind_applies_to_a_source_without_item_names() {
    let (_tmp, home, source, target) = stage_kind_source();
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "--kind",
            "skill",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert!(target.join(".claude/skills/s/SKILL.md").exists());
    assert!(!target.join(".claude/rules").exists());
    assert!(!target.join(".claude/agents").exists());
    let lock = read_kind_lock(&target);
    assert_eq!(lock["items"].as_array().unwrap().len(), 1);
    assert_eq!(lock["items"][0]["kind"], "skill");
}

#[test]
fn add_kind_does_not_install_a_required_item_of_another_kind() {
    let (_tmp, home, source, target) = stage_kind_source();
    write_files(
        &source,
        &[(
            "r2/RULE.md",
            "---\nname: r2\ndescription: A dependent rule.\nrequires:\n  skills: [s]\n---\nRule body.\n",
        )],
    );
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "r2",
            "--kind",
            "rule",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert!(target.join(".claude/rules/r2.md").exists());
    assert!(!target.join(".claude/skills").exists());
    assert_eq!(
        read_kind_lock(&target)["items"].as_array().unwrap().len(),
        1
    );
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor", "--project"])
        .assert()
        .success();
}

#[test]
fn add_with_an_unknown_kind_is_a_usage_error() {
    let (_tmp, home, source, target) = stage_kind_source();
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "bun",
            "--kind",
            "hook",
            "--project",
            "--claude",
        ])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("unknown item kind: hook"));
    assert!(!target.join(".claude").exists());
    assert!(!target.join(".upskill-lock.json").exists());
    assert!(!target.join(".mcp.json").exists());
}

#[test]
fn add_kind_that_removes_every_named_item_is_an_error() {
    let (_tmp, home, source, target) = stage_kind_source();
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "s",
            "--kind",
            "rule",
            "--project",
            "--claude",
        ])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("no item of kind rule matches: s"));
    assert!(!target.join(".claude").exists());
    assert!(!target.join(".upskill-lock.json").exists());
}

#[test]
fn add_kind_records_sorted_unique_kinds_independent_of_flag_order() {
    let (_tmp, home, source, target) = stage_kind_source();
    let mut previous = None;
    for flags in [
        vec!["rule", "rule"],
        vec!["rule", "agent", "rule"],
        vec!["agent", "rule"],
    ] {
        let mut cmd = common::upskill_cmd(&home);
        cmd.current_dir(&target).args([
            "add",
            source.to_str().unwrap(),
            "bun",
            "--project",
            "--claude",
        ]);
        for kind in &flags {
            cmd.args(["--kind", kind]);
        }
        cmd.assert().success();
        let expected = if flags.contains(&"agent") {
            serde_json::json!(["rule", "agent"])
        } else {
            serde_json::json!(["rule"])
        };
        assert_eq!(read_kind_lock(&target)["bundles"][0]["kinds"], expected);
        if flags.contains(&"agent") {
            let bytes = fs::read(target.join(".upskill-lock.json")).unwrap();
            if let Some(previous) = &previous {
                assert_eq!(&bytes, previous);
            }
            previous = Some(bytes);
        }
    }
}

#[test]
fn add_kind_has_no_false_conflict_for_a_bundle_or_plain_item_request() {
    for request in ["bun", "r"] {
        let (tmp, home, source, target) = stage_kind_source();
        if request == "r" {
            write_files(
                &source,
                &[(
                    "r/RULE.md",
                    "---\nname: r\ndescription: A dependent rule.\nrequires:\n  skills: [s]\n---\nRule body.\n",
                )],
            );
        }
        let other = tmp.path().join("other");
        write_files(
            &other,
            &[(
                "s/SKILL.md",
                "---\nname: s\ndescription: Another source skill.\n---\nOriginal skill.\n",
            )],
        );
        common::upskill_cmd(&home)
            .current_dir(&target)
            .args(["add", other.to_str().unwrap(), "s", "--project", "--claude"])
            .assert()
            .success();
        let old_skill = fs::read(target.join(".claude/skills/s/SKILL.md")).unwrap();
        common::upskill_cmd(&home)
            .current_dir(&target)
            .args([
                "add",
                source.to_str().unwrap(),
                request,
                "--kind",
                "rule",
                "--project",
                "--claude",
            ])
            .assert()
            .success();
        assert_eq!(
            fs::read(target.join(".claude/skills/s/SKILL.md")).unwrap(),
            old_skill
        );
        assert!(target.join(".claude/rules/r.md").is_file());
    }
}

#[test]
fn add_kind_records_kinds_on_a_mixed_bundle_and_item_request() {
    let (_tmp, home, source, target) = stage_kind_source();
    write_files(
        &source,
        &[(
            "x/RULE.md",
            "---\nname: x\ndescription: Another rule.\n---\nRule body.\n",
        )],
    );
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "bun",
            "x",
            "--kind",
            "rule",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert_eq!(
        read_kind_lock(&target)["bundles"][0]["kinds"],
        serde_json::json!(["rule"])
    );
    assert!(target.join(".claude/rules/x.md").is_file());
}

#[test]
fn add_kind_short_flag_installs_only_rules() {
    let (_tmp, home, source, target) = stage_kind_source();
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            source.to_str().unwrap(),
            "bun",
            "-k",
            "rule",
            "--project",
            "--claude",
        ])
        .assert()
        .success();
    assert!(target.join(".claude/rules/r.md").is_file());
    assert!(!target.join(".claude/skills").exists());
    assert_eq!(
        read_kind_lock(&target)["bundles"][0]["kinds"],
        serde_json::json!(["rule"])
    );
}

/// Issue #607: `add` from a source whose kind directory cannot be read fails
/// with an error that names the directory, instead of installing nothing.
#[cfg(unix)]
#[test]
fn add_fails_when_a_kind_directory_of_the_source_is_unreadable() {
    use std::os::unix::fs::PermissionsExt;

    /// Restores the mode of the directory on drop, also when the test panics.
    struct Restore<'a>(&'a Path);
    impl Drop for Restore<'_> {
        fn drop(&mut self) {
            let _ = fs::set_permissions(self.0, fs::Permissions::from_mode(0o755));
        }
    }

    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("reg");
    let item = source.join("skills/one");
    fs::create_dir_all(&item).unwrap();
    fs::write(
        item.join("SKILL.md"),
        "---\nname: one\ndescription: Skill one.\n---\n\nBody.\n",
    )
    .unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(target.join(".git")).unwrap();

    let kind_dir = source.join("skills");
    fs::set_permissions(&kind_dir, fs::Permissions::from_mode(0o000)).unwrap();
    let _restore = Restore(&kind_dir);
    if fs::read_dir(&kind_dir).is_ok() {
        // Running as root: the permission has no effect.
        return;
    }
    let out = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains(&kind_dir.display().to_string()), "{stderr}");
}
