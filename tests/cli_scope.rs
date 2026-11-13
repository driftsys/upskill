//! ATDD tests for scope reporting, both-scope `list` and `doctor`, and the
//! global output paths (re-steer design, section 3).
mod common;

use std::fs;
use std::path::{Path, PathBuf};

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

/// `repo` holds `.git`, `source` holds the fixture items, `outside` is not
/// inside a git repository.
struct Env {
    #[allow(dead_code)] // Keeps the temporary directory alive.
    tmp: tempfile::TempDir,
    home: PathBuf,
    repo: PathBuf,
    outside: PathBuf,
    source: PathBuf,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    let outside = tmp.path().join("outside");
    let source = tmp.path().join("source");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    stage_source(&source);
    Env {
        tmp,
        home,
        repo,
        outside,
        source,
    }
}

fn run(env: &Env, cwd: &Path, args: &[&str]) -> std::process::Output {
    common::upskill_cmd(&env.home)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn add_outside_a_git_repository_without_a_flag_asks_for_one() {
    let e = env();
    let out = run(&e, &e.outside, &["add", e.source.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    let err = stderr(&out);
    assert!(err.contains("not inside a git repository"), "{err}");
    assert!(
        err.contains("--project") && err.contains("--global"),
        "{err}"
    );
    assert!(!e.home.join(".upskill-lock.json").exists());
    assert!(!e.outside.join(".upskill-lock.json").exists());
}

#[test]
fn add_with_an_invalid_source_outside_a_git_repository_reports_the_source() {
    let e = env();
    let out = run(&e, &e.outside, &["add", "not-a-valid-source"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(stderr(&out), "error: source must be in owner/repo format\n");
}

#[test]
fn add_update_and_remove_state_the_scope_and_its_root() {
    let e = env();
    let src = e.source.to_str().unwrap();
    let project = format!(
        "scope: project ({})",
        e.repo.canonicalize().unwrap().display()
    );
    for args in [
        vec!["add", src],
        vec!["update", "--yes"],
        vec!["remove", "api-conventions"],
    ] {
        let out = run(&e, &e.repo, &args);
        assert!(out.status.success(), "{args:?}: {}", stderr(&out));
        assert!(
            stderr(&out).contains(&project),
            "{args:?}: {}",
            stderr(&out)
        );
    }
    let global = format!("scope: global ({})", e.home.display());
    for args in [
        vec!["add", "--global", src],
        vec!["update", "--yes"],
        vec!["remove", "api-conventions"],
    ] {
        // Outside a git repository, `update` and `remove` use the global scope.
        let out = run(&e, &e.outside, &args);
        assert!(out.status.success(), "{args:?}: {}", stderr(&out));
        assert!(stderr(&out).contains(&global), "{args:?}: {}", stderr(&out));
    }
}

#[test]
fn quiet_hides_the_scope_line() {
    let e = env();
    let src = e.source.to_str().unwrap();
    // Each command with its expected exit code: `remove-mcp` fails because no
    // such MCP entry exists.
    for (args, code) in [
        (vec!["--quiet", "add", src], 0),
        (vec!["--quiet", "update", "--yes"], 0),
        (vec!["--quiet", "remove", "api-conventions"], 0),
        (vec!["--quiet", "remove-mcp", "x"], 1),
    ] {
        let out = run(&e, &e.repo, &args);
        assert_eq!(out.status.code(), Some(code), "{args:?}: {}", stderr(&out));
        assert!(
            !stderr(&out).contains("scope:"),
            "{args:?}: {}",
            stderr(&out)
        );
    }
}

#[test]
fn remove_mcp_states_the_scope_and_its_root() {
    let e = env();
    let out = run(&e, &e.repo, &["remove-mcp", "x"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let project = format!(
        "scope: project ({})",
        e.repo.canonicalize().unwrap().display()
    );
    assert!(stderr(&out).contains(&project), "{}", stderr(&out));
    // Outside a git repository, `remove-mcp` uses the global scope.
    let out = run(&e, &e.outside, &["remove-mcp", "x"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let global = format!("scope: global ({})", e.home.display());
    assert!(stderr(&out).contains(&global), "{}", stderr(&out));
}

const OLD: [&str; 3] = [
    ".github/skills/create-api-endpoint/SKILL.md",
    ".github/instructions/api-conventions.instructions.md",
    ".github/agents/security-reviewer.agent.md",
];
const NEW: [&str; 3] = [
    ".copilot/skills/create-api-endpoint/SKILL.md",
    ".copilot/instructions/api-conventions.instructions.md",
    ".copilot/agents/security-reviewer.agent.md",
];

#[test]
fn global_add_writes_the_personal_copilot_directories() {
    let e = env();
    let out = run(
        &e,
        &e.repo,
        &["add", "--global", e.source.to_str().unwrap()],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    for rel in NEW {
        assert!(e.home.join(rel).is_file(), "{rel}");
    }
    for rel in OLD {
        assert!(!e.home.join(rel).exists(), "{rel}");
    }
    // Claude Code paths are the same in both scopes.
    assert!(
        e.home
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .is_file()
    );
    assert!(e.home.join(".claude/rules/api-conventions.md").is_file());
    assert!(e.home.join(".claude/agents/security-reviewer.md").is_file());
    let lock = fs::read_to_string(e.home.join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("\".copilot/skills/create-api-endpoint/SKILL.md\""));
    assert!(!lock.contains(".github/"));
}

#[test]
fn project_add_keeps_the_github_paths() {
    let e = env();
    let out = run(&e, &e.repo, &["add", e.source.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    for rel in OLD {
        assert!(e.repo.join(rel).is_file(), "{rel}");
    }
    assert!(!e.repo.join(".copilot").exists());
}

/// A global install as a released version left it: files at the `.github`
/// paths, recorded in a `schema: 2` lockfile.
fn released_global_install(e: &Env) {
    let out = run(
        e,
        &e.outside,
        &["add", "--global", e.source.to_str().unwrap()],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let lock_path = e.home.join(".upskill-lock.json");
    let mut lock = fs::read_to_string(&lock_path).unwrap();
    // A released version also wrote the fourth fixture rule at the old path.
    let fourth = (
        ".github/instructions/license-awareness.instructions.md",
        ".copilot/instructions/license-awareness.instructions.md",
    );
    for (old, new) in OLD.iter().zip(NEW.iter()).chain([(&fourth.0, &fourth.1)]) {
        let from = e.home.join(new);
        let to = e.home.join(old);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::rename(&from, &to).unwrap();
        lock = lock.replace(new, old);
    }
    fs::write(&lock_path, lock).unwrap();
    fs::remove_dir_all(e.home.join(".copilot")).unwrap();
}

fn assert_moved(e: &Env) {
    for rel in NEW {
        assert!(e.home.join(rel).is_file(), "{rel}");
    }
    for rel in OLD {
        assert!(!e.home.join(rel).exists(), "{rel}");
    }
    // Client roots and kind directories are kept, so only the item
    // directory is asserted.
    assert!(
        !e.home.join(".github/skills/create-api-endpoint").exists(),
        "the old item directory is removed"
    );
}

#[test]
fn global_update_moves_a_released_install_without_force() {
    let e = env();
    released_global_install(&e);
    let out = run(&e, &e.outside, &["update", "--global", "--yes"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_moved(&e);
    // The items are reported as up to date because their source did not
    // change; the moved files are reported by this line.
    let text = stdout(&out);
    assert!(
        text.contains("Changed 8 file(s) written or deleted"),
        "{text}"
    );
}

#[test]
fn global_update_moves_a_schema_1_install() {
    let e = env();
    released_global_install(&e);
    // Rewrite the lockfile to `schema: 1`: no `schema` marker of 2, no `files`.
    let lock_path = e.home.join(".upskill-lock.json");
    let mut json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    json["schema"] = serde_json::json!(1);
    for item in json["items"].as_array_mut().unwrap() {
        item.as_object_mut().unwrap().remove("files");
    }
    fs::write(&lock_path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    let out = run(&e, &e.outside, &["update", "--global", "--yes"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_moved(&e);
    assert!(e.home.join(".upskill-lock.json.v1").exists());
}

#[test]
fn global_update_keeps_an_edited_file_at_the_old_path() {
    let e = env();
    released_global_install(&e);
    fs::write(e.home.join(OLD[1]), "edited by the user\n").unwrap();
    let out = run(&e, &e.outside, &["update", "--global", "--yes"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(e.home.join(NEW[1]).is_file());
    assert_eq!(
        fs::read_to_string(e.home.join(OLD[1])).unwrap(),
        "edited by the user\n"
    );
    assert!(stderr(&out).contains(OLD[1]) || stdout(&out).contains(OLD[1]));
}

#[test]
fn global_update_previews_the_move_in_a_dry_run() {
    let e = env();
    released_global_install(&e);
    let out = run(&e, &e.outside, &["update", "--global", "--dry-run"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    for name in [
        "create-api-endpoint",
        "api-conventions",
        "security-reviewer",
    ] {
        assert!(text.contains(name), "{text}");
    }
    assert!(!text.contains("up to date"), "{text}");
    // The skill writes one file at the new path and deletes one file at the
    // old path; the other clients are unchanged.
    assert!(
        text.contains("update   skill create-api-endpoint   2 files"),
        "{text}"
    );
    for rel in NEW {
        assert!(!e.home.join(rel).exists(), "{rel}");
    }
    for rel in OLD {
        assert!(e.home.join(rel).is_file(), "{rel}");
    }
}

fn install_project_rule_and_global_skill(e: &Env) {
    let src = e.source.to_str().unwrap();
    assert!(
        run(e, &e.repo, &["add", src, "api-conventions"])
            .status
            .success()
    );
    assert!(
        run(e, &e.repo, &["add", "--global", src, "create-api-endpoint"])
            .status
            .success()
    );
}

fn list_json(e: &Env, cwd: &Path, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["list", "--json"];
    all.extend_from_slice(args);
    let text = stdout(&run(e, cwd, &all));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("invalid JSON: {err}\n{text}"))
}

#[test]
fn list_shows_both_scopes_with_the_scope_of_each_item() {
    let e = env();
    install_project_rule_and_global_skill(&e);
    let text = stdout(&run(&e, &e.repo, &["list"]));
    let project = text.find("project (").expect(&text);
    let global = text
        .find(&format!("global ({})", e.home.display()))
        .expect(&text);
    assert!(project < global);
    let rule = text.find("api-conventions").unwrap();
    let skill = text.find("create-api-endpoint").unwrap();
    assert!(project < rule && rule < global && global < skill, "{text}");
}

#[test]
fn list_json_holds_one_entry_for_each_scope() {
    let e = env();
    install_project_rule_and_global_skill(&e);
    let v = list_json(&e, &e.repo, &[]);
    assert_eq!(v["scopes"][0]["scope"], "project", "{v}");
    assert_eq!(v["scopes"][0]["rules"][0]["name"], "api-conventions", "{v}");
    assert_eq!(v["scopes"][1]["scope"], "global", "{v}");
    assert_eq!(
        v["scopes"][1]["skills"][0]["name"], "create-api-endpoint",
        "{v}"
    );
    assert_eq!(
        v["scopes"][1]["root"].as_str().unwrap(),
        e.home.to_str().unwrap(),
        "{v}"
    );
}

#[test]
fn list_with_a_scope_flag_shows_that_scope_only() {
    let e = env();
    install_project_rule_and_global_skill(&e);
    let text = stdout(&run(&e, &e.repo, &["list", "--global"]));
    assert!(text.contains("global ("), "{text}");
    assert!(!text.contains("project ("), "{text}");
    let v = list_json(&e, &e.repo, &["--project"]);
    assert_eq!(v["scopes"].as_array().unwrap().len(), 1, "{v}");
    assert_eq!(v["scopes"][0]["scope"], "project", "{v}");
}

#[test]
fn list_names_a_scope_without_items() {
    let e = env();
    let src = e.source.to_str().unwrap();
    assert!(
        run(
            &e,
            &e.repo,
            &["add", "--global", src, "create-api-endpoint"]
        )
        .status
        .success()
    );
    let text = stdout(&run(&e, &e.repo, &["list"]));
    let header = text.find("project (").expect(&text);
    let empty = text.find("  No items installed.").expect(&text);
    let global = text.find("global (").expect(&text);
    assert!(header < empty && empty < global, "{text}");
}

#[test]
fn list_in_the_home_directory_shows_the_global_scope_once() {
    let e = env();
    let src = e.source.to_str().unwrap();
    assert!(
        run(
            &e,
            &e.home,
            &["add", "--global", src, "create-api-endpoint"]
        )
        .status
        .success()
    );
    let v = list_json(&e, &e.home, &[]);
    assert_eq!(v["scopes"].as_array().unwrap().len(), 1, "{v}");
    assert_eq!(v["scopes"][0]["scope"], "global", "{v}");
}

#[test]
fn list_without_a_flag_outside_a_git_repository_includes_the_project_scope() {
    let e = env();
    let src = e.source.to_str().unwrap();
    let out = run(
        &e,
        &e.outside,
        &["add", "--project", src, "api-conventions"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let out = run(
        &e,
        &e.outside,
        &["add", "--global", src, "create-api-endpoint"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let v = list_json(&e, &e.outside, &[]);
    assert_eq!(v["scopes"].as_array().unwrap().len(), 2, "{v}");
    assert_eq!(v["scopes"][0]["scope"], "project", "{v}");
    assert_eq!(
        v["scopes"][0]["root"].as_str().unwrap(),
        e.outside.canonicalize().unwrap().to_str().unwrap(),
        "{v}"
    );
    assert_eq!(v["scopes"][1]["scope"], "global", "{v}");
}

fn doctor_json(e: &Env, cwd: &Path, args: &[&str]) -> (Option<i32>, serde_json::Value) {
    let mut all = vec!["doctor", "--json"];
    all.extend_from_slice(args);
    let out = run(e, cwd, &all);
    let text = stdout(&out);
    let v = serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("invalid JSON: {err}\n{text}\n{}", stderr(&out)));
    (out.status.code(), v)
}

fn earlier_layout_files(v: &serde_json::Value, scope_index: usize) -> Vec<String> {
    v["scopes"][scope_index]["earlier_layout"]
        .as_array()
        .unwrap_or_else(|| panic!("no earlier_layout in {v}"))
        .iter()
        .flat_map(|item| item["files"].as_array().unwrap().clone())
        .map(|f| f.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn doctor_reports_each_scope_and_fails_when_one_is_not_clean() {
    let e = env();
    let src = e.source.to_str().unwrap();
    run(&e, &e.repo, &["add", src, "api-conventions"]);
    run(
        &e,
        &e.repo,
        &["add", "--global", src, "create-api-endpoint"],
    );
    fs::remove_file(e.home.join(".claude/skills/create-api-endpoint/SKILL.md")).unwrap();
    let out = run(&e, &e.repo, &["doctor", "--json"]);
    assert_eq!(out.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["scopes"][0]["scope"], "project");
    assert_eq!(
        v["scopes"][0]["missing_outputs"].as_array().unwrap().len(),
        0
    );
    assert_eq!(v["scopes"][1]["scope"], "global");
    assert_eq!(
        v["scopes"][1]["missing_outputs"][0]["name"],
        "create-api-endpoint"
    );
    // The project scope alone is clean.
    assert_eq!(
        run(&e, &e.repo, &["doctor", "--project"]).status.code(),
        Some(0)
    );
}

#[test]
fn doctor_text_names_each_scope() {
    let e = env();
    install_project_rule_and_global_skill(&e);
    let out = run(&e, &e.repo, &["doctor"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    let project = text.find("project (").expect(&text);
    let global = text
        .find(&format!("global ({})", e.home.display()))
        .expect(&text);
    assert!(project < global, "{text}");
    let first_clean = text.find("doctor: clean").expect(&text);
    let last_clean = text.rfind("doctor: clean").unwrap();
    assert!(
        project < first_clean && global < last_clean && first_clean < global,
        "{text}"
    );
}

#[test]
fn doctor_notes_an_item_installed_in_both_scopes() {
    let e = env();
    let src = e.source.to_str().unwrap();
    run(&e, &e.repo, &["add", src, "api-conventions"]);
    run(&e, &e.repo, &["add", "--global", src, "api-conventions"]);
    let (code, v) = doctor_json(&e, &e.repo, &[]);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(
        v["in_both_scopes"],
        serde_json::json!([{"kind": "rule", "name": "api-conventions"}]),
        "{v}"
    );
    let text = stdout(&run(&e, &e.repo, &["doctor"]));
    assert!(
        text.contains("note: rule api-conventions is installed in both scopes"),
        "{text}"
    );
    let (_, v) = doctor_json(&e, &e.repo, &["--global"]);
    assert!(v["in_both_scopes"].as_array().unwrap().is_empty(), "{v}");
}

#[test]
fn doctor_without_a_flag_outside_a_git_repository_includes_the_project_scope() {
    let e = env();
    let src = e.source.to_str().unwrap();
    let out = run(
        &e,
        &e.outside,
        &["add", "--project", src, "api-conventions"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let out = run(
        &e,
        &e.outside,
        &["add", "--global", src, "create-api-endpoint"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let (code, v) = doctor_json(&e, &e.outside, &[]);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["scopes"].as_array().unwrap().len(), 2, "{v}");
    assert_eq!(v["scopes"][0]["scope"], "project", "{v}");
    assert_eq!(
        v["scopes"][0]["root"].as_str().unwrap(),
        e.outside.canonicalize().unwrap().to_str().unwrap(),
        "{v}"
    );
    assert_eq!(v["scopes"][1]["scope"], "global", "{v}");
}

#[test]
fn doctor_lists_an_absent_recorded_file_at_an_earlier_path_as_earlier_layout() {
    let e = env();
    released_global_install(&e);
    fs::remove_file(e.home.join(OLD[0])).unwrap();
    let (code, v) = doctor_json(&e, &e.outside, &["--global"]);
    assert_eq!(code, Some(1), "{v}");
    assert!(
        earlier_layout_files(&v, 0).iter().any(|f| f == OLD[0]),
        "{v}"
    );
    let missing = v["scopes"][0]["missing_outputs"].as_array().unwrap();
    assert!(
        missing
            .iter()
            .all(|m| !m["missing_files"].to_string().contains(OLD[0])),
        "{v}"
    );
}

#[test]
fn doctor_reports_recorded_files_at_the_earlier_global_paths() {
    let e = env();
    released_global_install(&e);
    let (code, v) = doctor_json(&e, &e.outside, &["--global"]);
    assert_eq!(code, Some(1), "{v}");
    let files = earlier_layout_files(&v, 0);
    for rel in OLD {
        assert!(files.iter().any(|f| f == rel), "{rel}: {v}");
    }
    assert_eq!(
        v["scopes"][0]["missing_outputs"].as_array().unwrap().len(),
        0,
        "{v}"
    );
    let text = stdout(&run(&e, &e.outside, &["doctor", "--global"]));
    assert!(text.contains("upskill update --global"), "{text}");
    // After the update, the global scope is clean.
    run(&e, &e.outside, &["update", "--global", "--yes"]);
    assert_eq!(
        run(&e, &e.outside, &["doctor", "--global"]).status.code(),
        Some(0)
    );
}

#[test]
fn doctor_reports_files_at_the_earlier_global_paths_of_a_schema_1_install() {
    let e = env();
    released_global_install(&e);
    let lock_path = e.home.join(".upskill-lock.json");
    let mut json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    json["schema"] = serde_json::json!(1);
    for item in json["items"].as_array_mut().unwrap() {
        item.as_object_mut().unwrap().remove("files");
    }
    fs::write(&lock_path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    let fourth = ".github/instructions/license-awareness.instructions.md";
    let before = fs::read(&lock_path).unwrap();
    let (code, v) = doctor_json(&e, &e.outside, &["--global"]);
    assert_eq!(code, Some(1), "{v}");
    let files = earlier_layout_files(&v, 0);
    for rel in OLD.iter().chain(&[fourth]) {
        assert!(files.iter().any(|f| f == rel), "{rel}: {v}");
    }
    assert_eq!(files.len(), OLD.len() + 1, "{v}");
    assert_eq!(
        v["scopes"][0]["missing_outputs"].as_array().unwrap().len(),
        0,
        "{v}"
    );
    assert_eq!(fs::read(&lock_path).unwrap(), before);
}

#[test]
fn doctor_in_the_home_directory_reports_no_item_in_both_scopes() {
    let e = env();
    let src = e.source.to_str().unwrap();
    run(&e, &e.home, &["add", "--global", src, "api-conventions"]);
    let (code, v) = doctor_json(&e, &e.home, &[]);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["scopes"].as_array().unwrap().len(), 1, "{v}");
    assert!(v["in_both_scopes"].as_array().unwrap().is_empty(), "{v}");
}

const ITEMS: [&str; 3] = [
    "create-api-endpoint",
    "api-conventions",
    "security-reviewer",
];

#[test]
fn global_remove_deletes_the_personal_copilot_files() {
    let e = env();
    let out = run(
        &e,
        &e.outside,
        &["add", "--global", e.source.to_str().unwrap()],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let mut args = vec!["remove", "--global"];
    args.extend_from_slice(&ITEMS);
    let out = run(&e, &e.outside, &args);
    assert!(out.status.success(), "{}", stderr(&out));
    for rel in NEW {
        assert!(!e.home.join(rel).exists(), "{rel}");
    }
    assert!(!e.home.join(".copilot/skills/create-api-endpoint").exists());
    for rel in [
        ".claude/skills/create-api-endpoint/SKILL.md",
        ".claude/rules/api-conventions.md",
        ".claude/agents/security-reviewer.md",
    ] {
        assert!(!e.home.join(rel).exists(), "{rel}");
    }
    let lock = fs::read_to_string(e.home.join(".upskill-lock.json")).unwrap();
    for name in ITEMS {
        assert!(!lock.contains(&format!("\"{name}\"")), "{name}: {lock}");
    }
}

#[test]
fn global_update_deletes_a_copilot_file_that_the_source_no_longer_holds() {
    let e = env();
    let script = e.source.join("create-api-endpoint/scripts/run.sh");
    fs::create_dir_all(script.parent().unwrap()).unwrap();
    fs::write(&script, "#!/bin/sh\necho hi\n").unwrap();
    let out = run(
        &e,
        &e.outside,
        &["add", "--global", e.source.to_str().unwrap()],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let skill_dir = e.home.join(".copilot/skills/create-api-endpoint");
    assert!(skill_dir.join("scripts/run.sh").is_file());
    fs::remove_file(&script).unwrap();
    let out = run(&e, &e.outside, &["update", "--global", "--yes"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!skill_dir.join("scripts/run.sh").exists());
    assert!(skill_dir.join("SKILL.md").is_file());
}

#[test]
fn doctor_reports_a_missing_personal_copilot_file() {
    let e = env();
    run(
        &e,
        &e.outside,
        &["add", "--global", e.source.to_str().unwrap()],
    );
    fs::remove_file(e.home.join(NEW[0])).unwrap();
    let (code, v) = doctor_json(&e, &e.outside, &["--global"]);
    assert_eq!(code, Some(1), "{v}");
    let missing = &v["scopes"][0]["missing_outputs"];
    assert_eq!(missing[0]["name"], "create-api-endpoint", "{v}");
    assert_eq!(
        missing[0]["missing_files"],
        serde_json::json!([NEW[0]]),
        "{v}"
    );
}

#[test]
fn doctor_reports_a_modified_personal_copilot_file() {
    let e = env();
    run(
        &e,
        &e.outside,
        &["add", "--global", e.source.to_str().unwrap()],
    );
    fs::write(e.home.join(NEW[1]), "edited by the user\n").unwrap();
    let (code, v) = doctor_json(&e, &e.outside, &["--global"]);
    assert_eq!(code, Some(1), "{v}");
    let modified = &v["scopes"][0]["modified_files"];
    assert_eq!(modified[0]["name"], "api-conventions", "{v}");
    assert_eq!(modified[0]["files"], serde_json::json!([NEW[1]]), "{v}");
}

#[test]
fn update_and_remove_say_when_the_item_is_in_the_other_scope() {
    let e = env();
    run(
        &e,
        &e.repo,
        &[
            "add",
            "--global",
            e.source.to_str().unwrap(),
            "api-conventions",
        ],
    );
    for cmd in ["update", "remove"] {
        let out = run(&e, &e.repo, &[cmd, "api-conventions"]);
        assert_eq!(out.status.code(), Some(1), "{cmd}");
        let err = stderr(&out);
        assert!(err.contains("not in lockfile: api-conventions"), "{err}");
        assert!(
            err.contains(&format!(
                "'api-conventions' is installed in the global scope ({}); pass --global",
                e.home.display()
            )),
            "{err}"
        );
        assert!(!err.contains("upskill doctor"), "{err}");
    }
    assert!(e.home.join(".claude/rules/api-conventions.md").is_file());
}

#[test]
fn the_hint_names_the_project_scope_for_a_global_command() {
    let e = env();
    let add = run(
        &e,
        &e.repo,
        &["add", e.source.to_str().unwrap(), "api-conventions"],
    );
    assert!(add.status.success(), "{}", stderr(&add));
    for cmd in ["update", "remove"] {
        let out = run(&e, &e.repo, &[cmd, "--global", "api-conventions"]);
        assert_eq!(out.status.code(), Some(1), "{cmd}");
        let err = stderr(&out);
        assert!(err.contains("is installed in the project scope ("), "{err}");
        assert!(err.contains("pass --project"), "{err}");
    }
}

#[test]
fn no_hint_for_a_name_that_no_scope_holds() {
    let e = env();
    for cmd in ["update", "remove"] {
        let out = run(&e, &e.repo, &[cmd, "nothing-here"]);
        let err = stderr(&out);
        assert!(err.contains("not in lockfile"), "{err}");
        assert!(!err.contains("is installed in"), "{err}");
        assert!(err.contains("upskill doctor"), "{err}");
    }
}

/// Install `api-conventions` in the project scope (`e.repo`) and
/// `create-api-endpoint` in the global scope, or both in both scopes.
fn install_in_scopes(e: &Env, project: &[&str], global: &[&str]) {
    let src = e.source.to_str().unwrap();
    for name in project {
        let out = run(e, &e.repo, &["add", src, name]);
        assert!(out.status.success(), "{}", stderr(&out));
    }
    for name in global {
        let out = run(e, &e.repo, &["add", "--global", src, name]);
        assert!(out.status.success(), "{}", stderr(&out));
    }
}

#[test]
fn no_other_scope_hint_when_the_held_name_is_also_in_the_current_scope() {
    let e = env();
    install_in_scopes(&e, &["api-conventions"], &["api-conventions"]);
    for cmd in ["update", "remove"] {
        let out = run(&e, &e.repo, &[cmd, "api-conventions", "nothing-here"]);
        assert_eq!(out.status.code(), Some(1), "{cmd}");
        let err = stderr(&out);
        assert!(err.contains("not in lockfile: nothing-here"), "{err}");
        assert!(!err.contains("is installed in"), "{cmd}: {err}");
        assert!(err.contains("upskill doctor"), "{cmd}: {err}");
    }
}

#[test]
fn the_other_scope_hint_names_only_the_names_that_the_current_scope_lacks() {
    let e = env();
    install_in_scopes(&e, &["api-conventions"], &["create-api-endpoint"]);
    for cmd in ["update", "remove"] {
        let out = run(
            &e,
            &e.repo,
            &[
                cmd,
                "api-conventions",
                "create-api-endpoint",
                "nothing-here",
            ],
        );
        assert_eq!(out.status.code(), Some(1), "{cmd}");
        let err = stderr(&out);
        let hints: Vec<&str> = err
            .lines()
            .filter(|l| l.contains("is installed in"))
            .collect();
        assert_eq!(hints.len(), 1, "{cmd}: {err}");
        assert!(hints[0].contains("'create-api-endpoint'"), "{cmd}: {err}");
        assert!(
            !err.contains("'api-conventions' is installed"),
            "{cmd}: {err}"
        );
        assert!(!err.contains("'nothing-here' is installed"), "{cmd}: {err}");
    }
}

#[test]
fn no_other_scope_hint_when_the_command_fails_for_another_reason() {
    let e = env();
    install_in_scopes(&e, &[], &["api-conventions"]);
    fs::write(e.repo.join(".upskill-lock.json"), "not json").unwrap();
    for cmd in ["update", "remove"] {
        let out = run(&e, &e.repo, &[cmd, "api-conventions"]);
        assert_eq!(out.status.code(), Some(1), "{cmd}");
        let err = stderr(&out);
        assert!(!err.contains("not in lockfile"), "{cmd}: {err}");
        assert!(!err.contains("is installed in"), "{cmd}: {err}");
    }
}

const HOME_REFUSAL: &str = "the current directory is the home directory, which is the root of the global scope: pass --global";

#[test]
fn project_scope_is_refused_in_the_home_directory() {
    let e = env();
    let src = e.source.to_str().unwrap();
    for args in [
        vec!["add", "--project", src],
        vec!["update", "--project", "--yes"],
        vec!["remove", "--project", "api-conventions"],
        vec!["remove-mcp", "--project", "x"],
        vec!["list", "--project"],
        vec!["doctor", "--project"],
    ] {
        let out = run(&e, &e.home, &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stderr(&out));
        let err = stderr(&out);
        assert!(
            err.contains(&format!("error: {HOME_REFUSAL}")),
            "{args:?}: {err}"
        );
        assert!(!e.home.join(".upskill-lock.json").exists(), "{args:?}");
        assert!(!e.home.join(".github").exists(), "{args:?}");
    }
}

#[test]
fn no_flag_in_a_home_directory_that_is_not_a_git_repository_is_refused_for_add() {
    let e = env();
    let out = run(&e, &e.home, &["add", e.source.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(err.contains(&format!("error: {HOME_REFUSAL}")), "{err}");
    assert!(!err.contains("not inside a git repository"), "{err}");
    assert!(!e.home.join(".upskill-lock.json").exists());
}

#[test]
fn no_flag_in_a_home_directory_that_is_a_git_repository_is_refused_for_add() {
    let e = env();
    fs::create_dir_all(e.home.join(".git")).unwrap();
    let src = e.source.to_str().unwrap();

    let out = run(&e, &e.home, &["add", src]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stderr(&out).contains(HOME_REFUSAL), "{}", stderr(&out));
    assert!(!e.home.join(".upskill-lock.json").exists());

    let out = run(&e, &e.home, &["add", "--global", src]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        e.home
            .join(".copilot/skills/create-api-endpoint/SKILL.md")
            .exists()
    );

    for args in [vec!["update", "--yes"], vec!["remove", "api-conventions"]] {
        let out = run(&e, &e.home, &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stderr(&out));
        assert!(stderr(&out).contains(HOME_REFUSAL), "{args:?}");
    }

    let out = run(&e, &e.home, &["list", "--json"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let json: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let scopes = json["scopes"].as_array().unwrap();
    assert_eq!(scopes.len(), 1, "{json}");
    assert_eq!(scopes[0]["scope"], "global", "{json}");

    let out = run(&e, &e.home, &["doctor"]);
    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn global_flag_in_the_home_directory_still_works() {
    let e = env();
    let src = e.source.to_str().unwrap();
    for args in [
        vec!["add", "--global", src],
        vec!["update", "--global", "--yes"],
        vec!["list", "--global"],
        vec!["doctor", "--global"],
        vec!["remove", "--global", "api-conventions"],
    ] {
        let out = run(&e, &e.home, &args);
        assert!(out.status.success(), "{args:?}: {}", stderr(&out));
    }
}
