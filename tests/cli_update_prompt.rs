//! ATDD tests for `upskill update` in a terminal: one block and at most one
//! question per source, the answers, and the closing summary.
//!
//! The tests give the command a pseudo-terminal on stdin with
//! `tests/fixtures/terminal_stdin.py`, which needs the Python `pty` module,
//! so the file is built on Unix only.
#![cfg(unix)]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const QUESTION: &str = "Apply? [y/N/q]";

/// Writes the skill `<root>/<dir>/<name>/SKILL.md` with `body`.
fn write_skill(root: &Path, dir: &str, name: &str, body: &str) {
    let path = root.join(dir).join(name);
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("SKILL.md"),
        format!("---\nschema: 1\nname: {name}\ndescription: Use for {name}\n---\n{body}\n"),
    )
    .unwrap();
}

/// A project with two local sources: `./a` with the skill `one` and `./b`
/// with the skill `two`, both installed with `extra` client flags.
fn two_sources(extra: &[&str]) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    write_skill(tmp.path(), "a", "one", "One.");
    write_skill(tmp.path(), "b", "two", "Two.");
    for dir in ["./a", "./b"] {
        common::upskill_cmd(&home)
            .current_dir(tmp.path())
            .args(["add", dir, "--project"])
            .args(extra)
            .assert()
            .success();
    }
    (tmp, home)
}

fn claude_skill(root: &Path, name: &str) -> PathBuf {
    root.join(".claude")
        .join("skills")
        .join(name)
        .join("SKILL.md")
}

fn copilot_skill(root: &Path, name: &str) -> PathBuf {
    root.join(".github")
        .join("skills")
        .join(name)
        .join("SKILL.md")
}

/// Changes the source of the skill `name` in `<root>/<dir>`.
fn change_source(root: &Path, dir: &str, name: &str) {
    write_skill(root, dir, name, "Changed.");
}

fn lockfile(root: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(root.join(".upskill-lock.json")).unwrap()).unwrap()
}

fn entry(root: &Path, name: &str) -> serde_json::Value {
    lockfile(root)["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == name)
        .unwrap()
        .clone()
}

/// The source label of the lockfile entry `name`.
fn label(root: &Path, name: &str) -> String {
    entry(root, name)["source"].as_str().unwrap().to_string()
}

/// Every file under `root`, with its bytes, except the lockfile, the
/// sources `a`, `b` and `r`, `.git` and `home`. Paths are relative to
/// `root`.
fn outputs(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for e in fs::read_dir(dir).unwrap() {
            let path = e.unwrap().path();
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            if dir == root
                && ["a", "b", "r", ".git", "home", ".upskill-lock.json"]
                    .iter()
                    .any(|skip| rel == Path::new(skip))
            {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn stdout(result: &serde_json::Value) -> &str {
    result["stdout"].as_str().unwrap()
}

fn stderr(result: &serde_json::Value) -> &str {
    result["stderr"].as_str().unwrap()
}

#[test]
fn yes_then_yes_applies_both() {
    let (tmp, home) = two_sources(&["--claude"]);
    change_source(tmp.path(), "a", "one");
    change_source(tmp.path(), "b", "two");

    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], "y\ny\n");
    assert_eq!(result["returncode"], 0, "{result}");
    assert_eq!(stderr(&result).matches(QUESTION).count(), 2, "{result}");
    for name in ["one", "two"] {
        let text = fs::read_to_string(claude_skill(tmp.path(), name)).unwrap();
        assert!(text.contains("Changed."), "{name}: {text}");
    }
    assert!(
        stdout(&result).contains("Updated 2 of 2 item(s) from 2 of 2 source(s)."),
        "{result}"
    );
}

#[test]
fn no_then_yes_leaves_the_first_source_untouched() {
    let (tmp, home) = two_sources(&["--claude"]);
    change_source(tmp.path(), "a", "one");
    change_source(tmp.path(), "b", "two");
    let one_before = fs::read(claude_skill(tmp.path(), "one")).unwrap();
    let entry_before = entry(tmp.path(), "one");

    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], "n\ny\n");
    assert_eq!(result["returncode"], 0, "{result}");
    assert_eq!(
        fs::read(claude_skill(tmp.path(), "one")).unwrap(),
        one_before
    );
    assert_eq!(entry(tmp.path(), "one"), entry_before);
    assert!(
        fs::read_to_string(claude_skill(tmp.path(), "two"))
            .unwrap()
            .contains("Changed.")
    );
    let summary = stdout(&result)
        .lines()
        .find(|line| line.starts_with("Updated "))
        .unwrap_or_else(|| panic!("no summary: {result}"));
    assert!(
        summary.ends_with("1 source(s) not applied."),
        "{summary}: {result}"
    );
}

fn assert_stops_at_the_first_source(input: &str) {
    let (tmp, home) = two_sources(&["--claude"]);
    change_source(tmp.path(), "a", "one");
    change_source(tmp.path(), "b", "two");
    let label_b = label(tmp.path(), "two");
    let before = outputs(tmp.path());
    let lock_before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();

    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], input);
    assert_eq!(result["returncode"], 0, "{result}");
    assert_eq!(stderr(&result).matches(QUESTION).count(), 1, "{result}");
    assert!(!stdout(&result).contains(&label_b), "{result}");
    assert!(
        stdout(&result).contains(
            "Updated 0 of 1 item(s) from 0 of 1 source(s). 1 source(s) not applied. \
             Stopped; 1 source(s) not examined.\n"
        ),
        "{result}"
    );
    assert_eq!(outputs(tmp.path()), before);
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}

#[test]
fn quit_stops_before_the_second_source() {
    assert_stops_at_the_first_source("q\n");
}

#[test]
fn end_of_input_stops_like_quit() {
    // Ctrl-D on an empty line of a terminal is end of input.
    assert_stops_at_the_first_source("\u{4}");
}

#[test]
fn a_source_without_a_change_asks_nothing() {
    let (tmp, home) = two_sources(&["--claude"]);
    change_source(tmp.path(), "b", "two");

    let label_a = label(tmp.path(), "one");

    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], "y\n");
    assert_eq!(result["returncode"], 0, "{result}");
    assert_eq!(stderr(&result).matches("Apply?").count(), 1, "{result}");
    let mut lines = stdout(&result).lines().skip_while(|line| *line != label_a);
    assert_eq!(lines.next(), Some(label_a.as_str()), "{result}");
    assert_eq!(lines.next(), Some("  up to date"), "{result}");
    assert!(
        fs::read_to_string(claude_skill(tmp.path(), "two"))
            .unwrap()
            .contains("Changed.")
    );
}

#[test]
fn an_edited_file_without_a_source_change_is_a_note() {
    let (tmp, home) = two_sources(&["--claude"]);
    let dest = claude_skill(tmp.path(), "one");
    fs::write(&dest, "my edit\n").unwrap();
    let lock_before = fs::read(tmp.path().join(".upskill-lock.json")).unwrap();

    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], "");
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        stdout(&result).contains("kept modified file: .claude/skills/one/SKILL.md"),
        "{result}"
    );
    assert!(!stderr(&result).contains("Apply?"), "{result}");
    assert!(!stdout(&result).contains("nothing to do"), "{result}");
    assert_eq!(fs::read(&dest).unwrap(), b"my edit\n");
    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}

#[test]
fn a_skip_only_source_asks_nothing_and_exits_1() {
    let (tmp, home) = two_sources(&["--claude"]);
    let dest = claude_skill(tmp.path(), "one");
    fs::write(&dest, "my edit\n").unwrap();
    change_source(tmp.path(), "a", "one");

    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], "");
    assert_eq!(result["returncode"], 1, "{result}");
    assert!(!stderr(&result).contains("Apply?"), "{result}");
    let skip = stdout(&result)
        .lines()
        .find(|line| line.trim_start().starts_with("skip "))
        .unwrap_or_else(|| panic!("no skip line: {result}"));
    assert!(skip.contains("one"), "{skip}");
    assert!(skip.contains(".claude/skills/one/SKILL.md"), "{skip}");
    assert_eq!(fs::read(&dest).unwrap(), b"my edit\n");
}

#[test]
fn a_narrowed_selection_lists_the_deletion_and_asks() {
    let (tmp, home) = two_sources(&["--claude", "--copilot"]);
    assert!(copilot_skill(tmp.path(), "one").is_file());

    let result =
        common::update_in_terminal(tmp.path(), &home, &["--project", "--claude"], "y\ny\n");
    assert_eq!(result["returncode"], 0, "{result}");
    assert_eq!(stderr(&result).matches(QUESTION).count(), 2, "{result}");
    let line = stdout(&result)
        .lines()
        .find(|line| line.trim_start().starts_with("update ") && line.contains("one"))
        .unwrap_or_else(|| panic!("no update line: {result}"));
    assert!(line.contains("1 file"), "{line}");
    assert!(!copilot_skill(tmp.path(), "one").exists());
    assert!(claude_skill(tmp.path(), "one").is_file());
    // The source hashes did not change: no item is counted as updated, and
    // the result lines and the summary report the deleted files.
    let out = stdout(&result);
    assert_eq!(
        out.matches("\n  1 file(s) changed\n").count(),
        2,
        "{result}"
    );
    assert!(
        out.contains(
            "\nUpdated 0 of 2 item(s) from 2 of 2 source(s).\n\
             Changed 2 file(s) written or deleted\n"
        ),
        "{result}"
    );
}

#[test]
fn force_names_the_file_that_is_lost() {
    let (tmp, home) = two_sources(&["--claude"]);
    let dest = claude_skill(tmp.path(), "one");
    fs::write(&dest, "my edit\n").unwrap();

    let result =
        common::update_in_terminal_merged(tmp.path(), &home, &["--project", "--force"], "n\n");
    assert_eq!(result["returncode"], 0, "{result}");
    let out = stdout(&result);
    let named = out
        .find(".claude/skills/one/SKILL.md")
        .unwrap_or_else(|| panic!("the edited path is not named: {result}"));
    let asked = out.find(QUESTION).unwrap();
    assert!(named < asked, "{result}");
    assert_eq!(fs::read(&dest).unwrap(), b"my edit\n");
}

/// Rewrite the lockfile as schema 1 without `files` records.
fn downgrade_to_schema_1(root: &Path) {
    let path = root.join(".upskill-lock.json");
    let mut lock = lockfile(root);
    lock["schema"] = 1.into();
    for entry in lock["items"].as_array_mut().unwrap() {
        let entry = entry.as_object_mut().unwrap();
        entry.remove("files");
        entry.remove("pending");
    }
    fs::write(&path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();
}

#[test]
fn adoption_notice_comes_before_the_first_block_and_stays_after_quit() {
    let (tmp, home) = two_sources(&["--claude"]);
    let label_a = label(tmp.path(), "one");
    downgrade_to_schema_1(tmp.path());
    change_source(tmp.path(), "a", "one");

    let result = common::update_in_terminal_merged(tmp.path(), &home, &["--project"], "q\n");
    assert_eq!(result["returncode"], 0, "{result}");
    let out = stdout(&result);
    assert_eq!(out.matches("note: adopted").count(), 1, "{result}");
    let notice = out.find("note: adopted").unwrap();
    let block = out
        .lines()
        .position(|line| line == label_a)
        .unwrap_or_else(|| panic!("no label line: {result}"));
    let block_offset: usize = out.lines().take(block).map(|l| l.len() + 1).sum();
    assert!(notice < block_offset, "{result}");
    assert!(tmp.path().join(".upskill-lock.json.v1").is_file());
    assert_eq!(lockfile(tmp.path())["schema"], 2);
}

/// The paths named in the item and hand-shake lines of `block`, and the
/// number of files that the item lines count without naming them.
fn named_and_counted(block: &[&str]) -> (Vec<PathBuf>, usize) {
    let mut named = Vec::new();
    let mut counted = 0;
    for line in block {
        let line = line.trim_start();
        if let Some(path) = line.strip_prefix("write ") {
            named.push(PathBuf::from(path.trim()));
            continue;
        }
        for part in line.split(", ") {
            let words: Vec<&str> = part.split_whitespace().collect();
            if let Some(n) = words
                .windows(2)
                .find(|w| w[1] == "file" || w[1] == "files")
                .and_then(|w| w[0].parse::<usize>().ok())
            {
                counted += n;
            }
            // `keeps modified <path>`, `replaces <path>`, `deletes <path>`.
            if let Some(i) = words
                .iter()
                .position(|w| ["modified", "replaces", "deletes"].contains(w))
            {
                if let Some(path) = words.get(i + 1) {
                    named.push(PathBuf::from(path));
                }
            }
        }
    }
    (named, counted)
}

#[test]
fn the_block_names_exactly_what_the_apply_changes() {
    // The update of `one` replaces an edited Claude file (named) and
    // deletes the Copilot file that the selection no longer targets
    // (counted).
    let (tmp, home) = two_sources(&["--claude", "--copilot"]);
    change_source(tmp.path(), "a", "one");
    fs::write(claude_skill(tmp.path(), "one"), "my edit\n").unwrap();
    let before = outputs(tmp.path());

    let result = common::update_in_terminal(
        tmp.path(),
        &home,
        &["--project", "--claude", "--force"],
        "y\nn\n",
    );
    assert_eq!(result["returncode"], 0, "{result}");
    let label_a = label(tmp.path(), "one");
    let block: Vec<&str> = stdout(&result)
        .lines()
        .skip_while(|line| *line != label_a)
        .skip(1)
        .take_while(|line| line.starts_with("  ") && !line.contains("updated"))
        .collect();
    let (named, counted) = named_and_counted(&block);
    assert!(!named.is_empty(), "{result}");

    let after = outputs(tmp.path());
    let changed: Vec<PathBuf> = before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    for path in &named {
        assert!(changed.contains(path), "{path:?} not changed: {result}");
    }
    assert_eq!(
        changed.len(),
        named.len() + counted,
        "{changed:?}: {result}"
    );
}

#[test]
fn quiet_prints_only_the_question() {
    let (tmp, home) = two_sources(&["--claude"]);
    change_source(tmp.path(), "a", "one");
    change_source(tmp.path(), "b", "two");
    let label_a = label(tmp.path(), "one");

    let label_b = label(tmp.path(), "two");

    let result = common::update_in_terminal(tmp.path(), &home, &["--project", "--quiet"], "y\ny\n");
    assert_eq!(result["returncode"], 0, "{result}");
    assert_eq!(stdout(&result), "", "{result}");
    assert_eq!(
        stderr(&result),
        format!(
            "terminal stdin verified\nApply changes from {label_a}? [y/N/q] \
             Apply changes from {label_b}? [y/N/q] "
        ),
        "{result}"
    );
    assert!(
        fs::read_to_string(claude_skill(tmp.path(), "two"))
            .unwrap()
            .contains("Changed.")
    );
}

#[test]
fn quiet_names_a_skipped_file_on_stderr_and_exits_1() {
    let (tmp, home) = two_sources(&["--claude"]);
    fs::write(claude_skill(tmp.path(), "one"), "my edit\n").unwrap();
    change_source(tmp.path(), "a", "one");

    let result = common::update_in_terminal(tmp.path(), &home, &["--project", "--quiet"], "");
    assert_eq!(result["returncode"], 1, "{result}");
    assert!(
        stderr(&result).contains(
            "skipped skill one: .claude/skills/one/SKILL.md was modified or is not managed by upskill"
        ),
        "{result}"
    );
    assert!(!stderr(&result).contains("Apply"), "{result}");
}

#[test]
fn force_says_deletes_for_an_edited_file_of_a_deselected_client() {
    let (tmp, home) = two_sources(&["--claude", "--copilot"]);
    let copilot = copilot_skill(tmp.path(), "one");
    fs::write(&copilot, "my edit\n").unwrap();

    let result = common::update_in_terminal_merged(
        tmp.path(),
        &home,
        &["--project", "--claude", "--force"],
        "n\nn\n",
    );
    assert_eq!(result["returncode"], 0, "{result}");
    let out = stdout(&result);
    let named = out
        .find("deletes .github/skills/one/SKILL.md (--force)")
        .unwrap_or_else(|| panic!("no deletes part: {result}"));
    assert!(named < out.find(QUESTION).unwrap(), "{result}");
    assert_eq!(fs::read(&copilot).unwrap(), b"my edit\n");
}

#[test]
fn every_source_up_to_date_prints_nothing_to_do() {
    let (tmp, home) = two_sources(&["--claude"]);
    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], "");
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        stdout(&result).ends_with("\neverything is up to date — nothing to do\n"),
        "{result}"
    );
    assert!(!stderr(&result).contains("Apply"), "{result}");
}

#[test]
fn a_run_that_only_adopts_does_not_print_nothing_to_do() {
    let (tmp, home) = two_sources(&["--claude"]);
    downgrade_to_schema_1(tmp.path());
    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], "");
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(stderr(&result).contains("note: adopted"), "{result}");
    assert!(!stdout(&result).contains("nothing to do"), "{result}");
    assert!(!stderr(&result).contains("Apply"), "{result}");
    assert_eq!(lockfile(tmp.path())["schema"], 2);
}

#[test]
fn an_error_after_a_printed_adoption_notice_prints_the_notice_once() {
    let (tmp, home) = two_sources(&["--claude"]);
    downgrade_to_schema_1(tmp.path());
    // The source `b` cannot be fetched: the run fails after `a` adopted.
    fs::remove_dir_all(tmp.path().join("b")).unwrap();
    let result = common::update_in_terminal_merged(tmp.path(), &home, &["--project"], "");
    assert_eq!(result["returncode"], 1, "{result}");
    assert_eq!(
        stdout(&result).matches("note: adopted").count(),
        1,
        "{result}"
    );
}

#[test]
fn a_missing_handshake_file_is_listed_and_asks() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let rule = tmp.path().join("r").join("style");
    fs::create_dir_all(&rule).unwrap();
    fs::write(
        rule.join("RULE.md"),
        "---\nschema: 1\nname: style\ndescription: Use for style\n---\nBody.\n",
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./r", "--claude", "--project"])
        .assert()
        .success();
    let bridge = tmp.path().join("CLAUDE.md");
    assert!(bridge.is_file());
    fs::remove_file(&bridge).unwrap();

    let result = common::update_in_terminal(tmp.path(), &home, &["--project"], "y\n");
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        stdout(&result)
            .lines()
            .any(|line| line.trim_start().starts_with("write") && line.contains("CLAUDE.md")),
        "{result}"
    );
    assert_eq!(stderr(&result).matches(QUESTION).count(), 1, "{result}");
    assert!(bridge.is_file());
}
