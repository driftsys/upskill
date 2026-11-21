//! opencode rules in the global scope (issue #581).
//!
//! opencode expands neither `**` nor a wildcard in a directory segment in
//! the `instructions` array of its global config. A rule installed with
//! `--global` is therefore listed there by the absolute path of its
//! generated file. The project scope keeps the single relative glob in
//! `<root>/opencode.json`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

struct Env {
    _tmp: tempfile::TempDir,
    home: PathBuf,
    cwd: PathBuf,
    source: PathBuf,
}

impl Env {
    /// A fake home, a directory that is not a project, and a source with
    /// the rules `rule-a` and `rule-b`.
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let home = root.join("home");
        let cwd = root.join("cwd");
        let source = root.join("source");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&cwd).unwrap();
        let env = Env {
            _tmp: tmp,
            home,
            cwd,
            source,
        };
        env.write_rule("rule-a");
        env.write_rule("rule-b");
        env
    }

    fn write_rule(&self, name: &str) {
        let dir = self.source.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("RULE.md"),
            format!(
                "---\nschema: 1\nname: {name}\ndescription: rule for the issue 581 tests.\n---\n\n## Body\n\nText.\n"
            ),
        )
        .unwrap();
    }

    fn run(&self, args: &[&str], xdg: Option<&Path>) -> assert_cmd::assert::Assert {
        let mut cmd = common::upskill_cmd(&self.home);
        cmd.current_dir(&self.cwd).args(args);
        if let Some(xdg) = xdg {
            cmd.env("XDG_CONFIG_HOME", xdg);
        }
        cmd.assert()
    }

    fn add_global(&self, xdg: Option<&Path>) {
        let source = self.source.to_str().unwrap();
        self.run(&["add", source, "--opencode", "--global"], xdg)
            .success();
    }

    fn config(&self) -> PathBuf {
        self.home.join(".config/opencode/opencode.json")
    }

    fn entry(&self, name: &str) -> String {
        self.home
            .join(format!(".agents/rules/{name}/RULE.md"))
            .to_str()
            .unwrap()
            .to_string()
    }
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

#[test]
fn global_add_writes_one_absolute_entry_per_rule_and_keeps_a_foreign_entry() {
    let env = Env::new();
    fs::create_dir_all(env.config().parent().unwrap()).unwrap();
    fs::write(
        env.config(),
        r#"{"theme": "dark", "instructions": ["/elsewhere/own.md"]}"#,
    )
    .unwrap();

    env.add_global(None);

    let config = read_json(&env.config());
    assert_eq!(
        config["instructions"],
        serde_json::json!([
            "/elsewhere/own.md",
            env.entry("rule-a"),
            env.entry("rule-b")
        ]),
        "{config}"
    );
    assert_eq!(config["theme"], "dark", "{config}");
    assert!(
        Path::new(&env.entry("rule-a")).is_file(),
        "the entry names the generated file"
    );
    assert!(!env.home.join("opencode.json").exists());
    assert!(
        !env.home.join(".config/opencode/AGENTS.md").exists(),
        "AGENTS.md of the user is not written"
    );
}

#[test]
fn global_add_twice_writes_each_entry_once() {
    let env = Env::new();
    env.add_global(None);
    env.add_global(None);

    let config = read_json(&env.config());
    assert_eq!(
        config["instructions"],
        serde_json::json!([env.entry("rule-a"), env.entry("rule-b")]),
        "{config}"
    );
}

#[test]
fn global_add_follows_an_absolute_xdg_config_home() {
    let env = Env::new();
    let xdg = env.home.parent().unwrap().join("xdg");
    env.add_global(Some(&xdg));

    let config = read_json(&xdg.join("opencode/opencode.json"));
    assert_eq!(
        config["instructions"],
        serde_json::json!([env.entry("rule-a"), env.entry("rule-b")]),
        "{config}"
    );
    assert!(!env.config().exists());
}

#[test]
fn global_add_for_another_client_writes_no_opencode_entry() {
    let env = Env::new();
    let source = env.source.to_str().unwrap();
    env.run(&["add", source, "--claude", "--global"], None)
        .success();

    assert!(!env.config().exists());
}

#[test]
fn global_add_without_a_rule_writes_no_instructions_entry() {
    let env = Env::new();
    fs::remove_dir_all(env.source.join("rule-a")).unwrap();
    fs::remove_dir_all(env.source.join("rule-b")).unwrap();
    let skill = env.source.join("skill-a/SKILL.md");
    fs::create_dir_all(skill.parent().unwrap()).unwrap();
    fs::write(
        &skill,
        "---\nschema: 1\nname: skill-a\ndescription: skill for the issue 581 tests.\n---\n\n## Body\n\nText.\n",
    )
    .unwrap();
    env.add_global(None);

    assert!(!env.config().exists());
}

#[test]
fn global_remove_deletes_only_the_entry_of_the_removed_rule() {
    let env = Env::new();
    env.add_global(None);
    let mut config = read_json(&env.config());
    config["instructions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!("/elsewhere/own.md"));
    fs::write(env.config(), config.to_string()).unwrap();

    env.run(&["remove", "rule-a", "--global"], None).success();

    let config = read_json(&env.config());
    assert_eq!(
        config["instructions"],
        serde_json::json!([env.entry("rule-b"), "/elsewhere/own.md"]),
        "{config}"
    );
}

#[test]
fn global_update_removes_the_entry_of_a_rule_that_the_source_no_longer_has() {
    let env = Env::new();
    env.add_global(None);
    fs::remove_dir_all(env.source.join("rule-b")).unwrap();

    env.run(&["update", "--yes", "--global"], None).code(0);

    let config = read_json(&env.config());
    assert_eq!(
        config["instructions"],
        serde_json::json!([env.entry("rule-a")]),
        "{config}"
    );
}

#[test]
fn global_doctor_reports_a_removed_entry_and_update_restores_it() {
    let env = Env::new();
    env.add_global(None);
    env.run(&["doctor", "--global"], None).code(0);

    let mut config = read_json(&env.config());
    config["instructions"] = serde_json::json!([env.entry("rule-b")]);
    fs::write(env.config(), config.to_string()).unwrap();

    let out = env.run(&["doctor", "--global"], None).code(1);
    let text = stdout(&out);
    assert!(
        text.contains(
            "doctor: 1 rule(s) installed for opencode are missing from the global opencode config"
        ),
        "{text}"
    );
    assert!(
        text.contains(&format!("rule rule-a ({})", env.config().display())),
        "{text}"
    );
    assert!(!text.contains("rule rule-b"), "{text}");

    env.run(&["update", "--yes", "--global"], None).code(0);

    let config = read_json(&env.config());
    assert_eq!(
        config["instructions"],
        serde_json::json!([env.entry("rule-b"), env.entry("rule-a")]),
        "{config}"
    );
    env.run(&["doctor", "--global"], None).code(0);
}

#[test]
fn global_doctor_json_lists_the_missing_entry() {
    let env = Env::new();
    env.add_global(None);
    fs::remove_file(env.config()).unwrap();

    let out = env.run(&["doctor", "--global", "--json"], None).code(1);
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let missing = &value["scopes"][0]["missing_rule_entries"];
    assert_eq!(missing.as_array().unwrap().len(), 2, "{value}");
    assert_eq!(missing[0]["name"], "rule-a", "{value}");
    assert_eq!(missing[0]["client"], "opencode", "{value}");
}

#[cfg(unix)]
#[test]
fn global_update_preview_lists_the_config_when_an_entry_would_be_added() {
    let env = Env::new();
    env.add_global(None);
    fs::remove_file(env.config()).unwrap();

    let result =
        common::update_in_terminal(env.home.parent().unwrap(), &env.home, &["--global"], "n\n");
    let out = result["stdout"].as_str().unwrap();
    assert!(
        out.lines()
            .any(|line| line.trim_start().starts_with("write") && line.contains("opencode.json")),
        "{result}"
    );
    assert!(!env.config().exists(), "a declined update writes nothing");
}

#[test]
fn global_add_errors_when_instructions_is_not_an_array() {
    let env = Env::new();
    fs::create_dir_all(env.config().parent().unwrap()).unwrap();
    fs::write(env.config(), r#"{"instructions": "x"}"#).unwrap();
    let source = env.source.to_str().unwrap();

    let out = env
        .run(&["add", source, "--opencode", "--global"], None)
        .failure();

    let err = String::from_utf8_lossy(&out.get_output().stderr).into_owned();
    assert!(err.contains("`instructions` must be an array"), "{err}");
}

#[test]
fn project_add_still_writes_the_single_relative_glob() {
    let env = Env::new();
    fs::create_dir_all(env.cwd.join(".git")).unwrap();
    let source = env.source.to_str().unwrap();
    env.run(&["add", source, "--opencode", "--project"], None)
        .success();

    let config = read_json(&env.cwd.join("opencode.json"));
    assert_eq!(
        config["instructions"],
        serde_json::json!([".agents/rules/**/RULE.md"]),
        "{config}"
    );
    assert!(!env.config().exists());
}

const JSONC: &str = "{ // a comment\n  \"theme\": \"x\"\n}\n";

fn write_jsonc_config(env: &Env) {
    fs::create_dir_all(env.config().parent().unwrap()).unwrap();
    fs::write(env.config(), JSONC).unwrap();
}

#[test]
fn global_add_of_a_skill_does_not_read_the_opencode_config() {
    let env = Env::new();
    fs::remove_dir_all(env.source.join("rule-a")).unwrap();
    fs::remove_dir_all(env.source.join("rule-b")).unwrap();
    let skill = env.source.join("skill-a/SKILL.md");
    fs::create_dir_all(skill.parent().unwrap()).unwrap();
    fs::write(
        &skill,
        "---\nschema: 1\nname: skill-a\ndescription: skill for the issue 581 tests.\n---\n\n## Body\n\nText.\n",
    )
    .unwrap();
    write_jsonc_config(&env);

    let source = env.source.to_str().unwrap();
    env.run(&["add", source, "--claude", "--global"], None)
        .success();

    assert_eq!(fs::read_to_string(env.config()).unwrap(), JSONC);
}

#[test]
fn global_add_of_a_rule_for_another_client_does_not_read_the_opencode_config() {
    let env = Env::new();
    write_jsonc_config(&env);

    let source = env.source.to_str().unwrap();
    env.run(&["add", source, "--claude", "--global"], None)
        .success();
    env.run(&["update", "--yes", "--global"], None).code(0);
    env.run(&["remove", "rule-a", "--global"], None).success();

    assert_eq!(fs::read_to_string(env.config()).unwrap(), JSONC);
}

#[test]
fn a_hand_written_entry_of_the_same_form_survives_add_update_and_remove() {
    let env = Env::new();
    let mine = env.entry("mine");
    fs::create_dir_all(env.config().parent().unwrap()).unwrap();
    fs::write(
        env.config(),
        serde_json::json!({ "instructions": [mine] }).to_string(),
    )
    .unwrap();

    env.add_global(None);
    let has_mine = |env: &Env| {
        read_json(&env.config())["instructions"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(env.entry("mine")))
    };
    assert!(has_mine(&env));

    fs::remove_dir_all(env.source.join("rule-b")).unwrap();
    env.run(&["update", "--yes", "--global"], None).code(0);
    assert!(has_mine(&env));

    env.run(&["remove", "rule-a", "--global"], None).success();
    assert!(has_mine(&env));
    let config = read_json(&env.config());
    assert_eq!(
        config["instructions"],
        serde_json::json!([env.entry("mine")]),
        "{config}"
    );
}

#[test]
fn reinstalling_a_rule_for_fewer_clients_deletes_its_opencode_entry() {
    let env = Env::new();
    env.add_global(None);
    let source = env.source.to_str().unwrap();
    env.run(
        &["add", source, "rule-a", "--claude", "--global", "--force"],
        None,
    )
    .success();

    let config = read_json(&env.config());
    assert_eq!(
        config["instructions"],
        serde_json::json!([env.entry("rule-b")]),
        "{config}"
    );
}

#[test]
fn update_with_a_narrower_client_selection_deletes_the_opencode_entries() {
    let env = Env::new();
    env.add_global(None);
    env.run(&["update", "--yes", "--claude", "--global"], None)
        .code(0);

    let config = read_json(&env.config());
    assert_eq!(config["instructions"], serde_json::json!([]), "{config}");
}

#[test]
fn update_restores_the_entry_of_a_rule_whose_generated_file_was_modified() {
    let env = Env::new();
    env.add_global(None);
    let file = env.home.join(".agents/rules/rule-a/RULE.md");
    let text = fs::read_to_string(&file).unwrap();
    fs::write(&file, format!("{text}my edit\n")).unwrap();
    let mut config = read_json(&env.config());
    config["instructions"] = serde_json::json!([env.entry("rule-b")]);
    fs::write(env.config(), config.to_string()).unwrap();

    env.run(&["update", "--yes", "--global"], None).code(0);

    let config = read_json(&env.config());
    assert_eq!(
        config["instructions"],
        serde_json::json!([env.entry("rule-b"), env.entry("rule-a")]),
        "{config}"
    );
    assert!(fs::read_to_string(&file).unwrap().contains("my edit"));
}

fn set_instructions(env: &Env, value: serde_json::Value) {
    fs::create_dir_all(env.config().parent().unwrap()).unwrap();
    fs::write(
        env.config(),
        serde_json::json!({ "instructions": value }).to_string(),
    )
    .unwrap();
}

fn instructions_of(env: &Env) -> serde_json::Value {
    read_json(&env.config())["instructions"].clone()
}

#[test]
fn global_add_of_a_rule_skipped_for_an_unowned_file_writes_no_entry() {
    let env = Env::new();
    let file = env.home.join(".agents/rules/rule-a/RULE.md");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "mine\n").unwrap();

    let source = env.source.to_str().unwrap();
    // A skipped item makes `add` exit with 1.
    env.run(&["add", source, "--opencode", "--global"], None)
        .code(1);

    assert_eq!(fs::read_to_string(&file).unwrap(), "mine\n");
    let config = read_json(&env.config());
    assert_eq!(
        config["instructions"],
        serde_json::json!([env.entry("rule-b")]),
        "{config}"
    );
}

#[test]
fn remove_keeps_a_hand_written_entry_for_a_rule_that_never_targeted_opencode() {
    let env = Env::new();
    let source = env.source.to_str().unwrap();
    env.run(&["add", source, "--claude", "--global"], None)
        .success();
    set_instructions(&env, serde_json::json!([env.entry("rule-a")]));

    env.run(&["remove", "rule-a", "--global"], None).success();

    assert_eq!(
        instructions_of(&env),
        serde_json::json!([env.entry("rule-a")])
    );
}

#[test]
fn update_orphan_removal_keeps_a_hand_written_entry_for_a_rule_that_never_targeted_opencode() {
    let env = Env::new();
    let source = env.source.to_str().unwrap();
    env.run(&["add", source, "--claude", "--global"], None)
        .success();
    set_instructions(&env, serde_json::json!([env.entry("rule-b")]));
    fs::remove_dir_all(env.source.join("rule-b")).unwrap();

    env.run(&["update", "--yes", "--global"], None).code(0);

    assert_eq!(
        instructions_of(&env),
        serde_json::json!([env.entry("rule-b")])
    );
}

#[test]
fn remove_succeeds_when_the_global_config_cannot_be_edited() {
    let env = Env::new();
    env.add_global(None);
    fs::write(env.config(), r#"{"instructions": "x"}"#).unwrap();

    env.run(&["remove", "rule-a", "--global"], None).success();

    let lock = read_json(&env.home.join(".upskill-lock.json"));
    let names: Vec<_> = lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["rule-b"], "{lock}");
    assert_eq!(
        fs::read_to_string(env.config()).unwrap(),
        r#"{"instructions": "x"}"#
    );
}

#[cfg(unix)]
#[test]
fn update_preview_lists_the_config_for_a_deletion_only_change() {
    let env = Env::new();
    env.add_global(None);
    // `--claude` narrows both rules to claude, so the preview lists the config
    // for the deletion of both entries. Declining writes nothing, so both stay.
    let result = common::update_in_terminal(
        env.home.parent().unwrap(),
        &env.home,
        &["--claude", "--global"],
        "n\n",
    );
    let out = result["stdout"].as_str().unwrap();
    assert!(
        out.lines()
            .any(|line| line.trim_start().starts_with("write") && line.contains("opencode.json")),
        "{result}"
    );
    assert_eq!(instructions_of(&env).as_array().unwrap().len(), 2);
}

#[test]
fn update_validates_the_global_config_before_it_writes_any_file() {
    let env = Env::new();
    env.add_global(None);
    fs::write(env.config(), r#"{"instructions": "x"}"#).unwrap();
    let lock_before = fs::read_to_string(env.home.join(".upskill-lock.json")).unwrap();

    env.run(&["update", "--yes", "--claude", "--global"], None)
        .failure();

    assert!(
        env.home.join(".agents/rules/rule-a/RULE.md").exists(),
        "no output was deleted"
    );
    assert_eq!(
        fs::read_to_string(env.home.join(".upskill-lock.json")).unwrap(),
        lock_before
    );
}

fn write_skill(env: &Env) {
    let skill = env.source.join("skill-a/SKILL.md");
    fs::create_dir_all(skill.parent().unwrap()).unwrap();
    fs::write(
        &skill,
        "---\nschema: 1\nname: skill-a\ndescription: skill for the issue 581 tests.\n---\n\n## Body\n\nText.\n",
    )
    .unwrap();
}

/// A source with `rule-a` and a skill, so that the source stays non-empty.
fn env_with_one_rule_and_a_skill() -> Env {
    let env = Env::new();
    fs::remove_dir_all(env.source.join("rule-b")).unwrap();
    write_skill(&env);
    env
}

#[cfg(unix)]
fn terminal_update(env: &Env, args: &[&str]) -> String {
    let result = common::update_in_terminal(env.home.parent().unwrap(), &env.home, args, "n\n");
    format!(
        "{}{}",
        result["stdout"].as_str().unwrap(),
        result["stderr"].as_str().unwrap()
    )
}

#[cfg(unix)]
fn lists_the_config(text: &str) -> bool {
    text.lines()
        .any(|line| line.trim_start().starts_with("write") && line.contains("opencode.json"))
}

#[cfg(unix)]
#[test]
fn update_preview_lists_the_config_when_only_an_orphaned_rule_entry_is_deleted() {
    let env = Env::new();
    fs::remove_dir_all(env.source.join("rule-a")).unwrap();
    write_skill(&env);
    env.add_global(None);
    fs::remove_dir_all(env.source.join("rule-b")).unwrap();

    let text = terminal_update(&env, &["--global"]);

    assert!(lists_the_config(&text), "{text}");
}

#[test]
fn update_validates_the_config_before_it_deletes_an_orphaned_rule() {
    let env = Env::new();
    fs::remove_dir_all(env.source.join("rule-a")).unwrap();
    write_skill(&env);
    env.add_global(None);
    fs::remove_dir_all(env.source.join("rule-b")).unwrap();
    fs::write(env.config(), r#"{"instructions": "x"}"#).unwrap();

    let out = env.run(&["update", "--yes", "--global"], None).failure();

    let err = String::from_utf8_lossy(&out.get_output().stderr).into_owned();
    assert!(err.contains("`instructions` must be an array"), "{err}");
    assert!(env.home.join(".agents/rules/rule-b/RULE.md").exists());
}

#[cfg(unix)]
#[test]
fn update_preview_of_an_unchanged_install_does_not_list_the_config() {
    let env = Env::new();
    let second = env.home.parent().unwrap().join("second");
    fs::create_dir_all(second.join("rule-c")).unwrap();
    fs::write(
        second.join("rule-c/RULE.md"),
        "---\nschema: 1\nname: rule-c\ndescription: rule for the issue 581 tests.\n---\n\n## Body\n\nText.\n",
    )
    .unwrap();
    env.add_global(None);
    env.run(
        &["add", second.to_str().unwrap(), "--opencode", "--global"],
        None,
    )
    .success();

    let text = terminal_update(&env, &["--global"]);

    assert!(!lists_the_config(&text), "{text}");
}

#[test]
fn update_preview_ignores_a_skipped_rule_for_the_clients_it_would_drop() {
    let env = env_with_one_rule_and_a_skill();
    let source = env.source.to_str().unwrap();
    env.run(&["add", source, "--opencode", "--global"], None)
        .success();
    // An unowned file at the Claude path makes the rule a skipped item.
    let claude = env.home.join(".claude/rules/rule-a.md");
    fs::create_dir_all(claude.parent().unwrap()).unwrap();
    fs::write(&claude, "mine\n").unwrap();
    fs::write(env.config(), r#"{"instructions": "x"}"#).unwrap();

    let out = env.run(&["update", "--dry-run", "--claude", "--global"], None);

    let err = String::from_utf8_lossy(&out.get_output().stderr).into_owned();
    assert!(!err.contains("must be an array"), "{err}");
}

#[test]
fn update_preview_does_not_add_the_entry_of_a_skipped_rule() {
    let env = env_with_one_rule_and_a_skill();
    let source = env.source.to_str().unwrap();
    env.run(&["add", source, "--claude", "--global"], None)
        .success();
    // An unowned file at the opencode path makes the rule a skipped item.
    let file = env.home.join(".agents/rules/rule-a/RULE.md");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "mine\n").unwrap();
    fs::create_dir_all(env.config().parent().unwrap()).unwrap();
    fs::write(env.config(), r#"{"instructions": "x"}"#).unwrap();

    let out = env.run(&["update", "--dry-run", "--opencode", "--global"], None);

    let err = String::from_utf8_lossy(&out.get_output().stderr).into_owned();
    assert!(!err.contains("must be an array"), "{err}");
}

#[test]
fn a_config_that_is_not_strict_json_fails_add_and_update_with_the_parse_error() {
    let env = Env::new();
    write_jsonc_config(&env);
    let source = env.source.to_str().unwrap();
    let out = env
        .run(&["add", source, "--opencode", "--global"], None)
        .failure();
    let err = String::from_utf8_lossy(&out.get_output().stderr).into_owned();
    assert!(err.contains("parse"), "{err}");

    let env = Env::new();
    env.add_global(None);
    write_jsonc_config(&env);
    let out = env.run(&["update", "--yes", "--global"], None).failure();
    let err = String::from_utf8_lossy(&out.get_output().stderr).into_owned();
    assert!(err.contains("parse"), "{err}");
}
