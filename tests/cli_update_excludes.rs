//! Issue #570: `update` applies the `--exclude` names that the last `add` of a
//! source recorded in the lockfile. All commands use the project scope in a
//! tempdir with a fake `$HOME`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::assert::Assert;

struct Env {
    tmp: tempfile::TempDir,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("home")).unwrap();
        fs::create_dir_all(tmp.path().join("consumer")).unwrap();
        Env { tmp }
    }

    fn consumer(&self) -> PathBuf {
        self.tmp.path().join("consumer")
    }

    fn registry(&self, name: &str) -> PathBuf {
        let path = self.tmp.path().join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn run(&self, args: &[&str]) -> Assert {
        common::upskill_cmd(&self.tmp.path().join("home"))
            .current_dir(self.consumer())
            .args(args)
            .args(["--project", "--claude"])
            .assert()
            .success()
    }

    fn lock(&self) -> serde_json::Value {
        let text = fs::read_to_string(self.consumer().join(".upskill-lock.json")).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    fn installed(&self, name: &str) -> bool {
        self.consumer().join(".claude/skills").join(name).exists()
    }
}

fn write_skill(root: &Path, name: &str, requires: &str, text: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let body = format!(
        "---\nschema: 1\nname: {name}\ndescription: test skill {name}.\n{requires}---\n\n# {name}\n\n{text}\n"
    );
    fs::write(dir.join("SKILL.md"), body).unwrap();
}

fn write_bundle(root: &Path, file: &str, members: &[&str]) {
    let items: String = members.iter().map(|m| format!("    - {m}\n")).collect();
    let name = file.trim_end_matches(".bundle.yaml");
    fs::write(
        root.join(file),
        format!(
            "schema: 1\nname: {name}\ndescription: bundle {name} for issue 570.\nitems:\n  skills:\n{items}"
        ),
    )
    .unwrap();
}

fn has_lock_item(lock: &serde_json::Value, name: &str) -> bool {
    lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["name"] == name)
}

fn s(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn cross_requires(dep: &Path) -> String {
    format!("requires:\n  skills: [{{ name: d, source: {} }}]\n", s(dep))
}

/// Registry `reg` where skill `r` requires `d` from a second registry.
fn cross_source_setup(env: &Env) -> (PathBuf, PathBuf) {
    let reg = env.registry("reg");
    let dep = env.registry("dep");
    write_skill(&dep, "d", "", "dependency");
    write_skill(&reg, "r", &cross_requires(&dep), "first");
    (reg, dep)
}

#[test]
fn update_keeps_an_excluded_bundle_member_out_for_a_bundle_file_source() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "m", "", "first");
    write_skill(&reg, "n", "", "first");
    write_bundle(&reg, "b.bundle.yaml", &["m", "n"]);
    let bundle = reg.join("b.bundle.yaml");
    env.run(&["add", s(&bundle), "--exclude", "m"]);
    assert!(env.installed("n") && !env.installed("m"));

    write_skill(&reg, "n", "", "second");
    env.run(&["update", "--yes"]);

    assert!(!env.installed("m"));
    assert!(!has_lock_item(&env.lock(), "m"), "{}", env.lock());
    assert!(env.installed("n"));
}

#[test]
fn update_keeps_an_excluded_bundle_member_out_for_a_directory_source_with_a_recorded_bundle() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "m", "", "first");
    write_skill(&reg, "n", "", "first");
    write_bundle(&reg, "b.bundle.yaml", &["m", "n"]);
    env.run(&["add", s(&reg), "b", "--exclude", "m"]);
    assert!(env.installed("n") && !env.installed("m"));

    write_skill(&reg, "n", "", "second");
    env.run(&["update", "--yes"]);

    assert!(!env.installed("m"));
    assert!(!has_lock_item(&env.lock(), "m"), "{}", env.lock());
    assert!(env.installed("n"));
}

#[test]
fn update_keeps_an_excluded_dependency_out_for_a_plain_directory_source() {
    let env = Env::new();
    let (reg, dep) = cross_source_setup(&env);
    env.run(&["add", s(&reg), "r", "--exclude", "d"]);
    assert!(env.installed("r") && !env.installed("d"));

    write_skill(&reg, "r", &cross_requires(&dep), "second");
    env.run(&["update", "--yes"]);

    assert!(!env.installed("d"));
    assert!(!has_lock_item(&env.lock(), "d"), "{}", env.lock());
}

#[test]
fn update_dry_run_plans_no_excluded_item() {
    let env = Env::new();
    let (reg, _dep) = cross_source_setup(&env);
    env.run(&["add", s(&reg), "r", "--exclude", "d"]);

    let out = env.run(&["update", "--dry-run"]);
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        !stdout
            .lines()
            .any(|l| l.split_whitespace().any(|w| w == "d")),
        "{stdout}"
    );
}

#[test]
fn an_unrelated_add_from_the_same_source_keeps_the_exclude() {
    let env = Env::new();
    let (reg, _dep) = cross_source_setup(&env);
    write_skill(&reg, "other", "", "first");
    env.run(&["add", s(&reg), "r", "--exclude", "d"]);
    env.run(&["add", s(&reg), "other"]);

    let lock = env.lock();
    assert_eq!(lock["excludes"], serde_json::json!(["d"]), "{lock}");

    env.run(&["update", "--yes"]);
    assert!(!env.installed("d"));
}

#[test]
fn naming_an_excluded_item_undoes_the_exclude() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "d", "", "dependency");
    write_skill(&reg, "r", "requires:\n  skills: [d]\n", "first");
    env.run(&["add", s(&reg), "r", "--exclude", "d"]);
    assert!(!env.installed("d"));
    env.run(&["add", s(&reg), "d"]);

    let lock = env.lock();
    assert!(lock.get("excludes").is_none(), "{lock}");
    assert!(env.installed("d"));

    env.run(&["update", "--yes"]);
    assert!(env.installed("d"));
}

#[test]
fn update_does_not_touch_an_excluded_name_installed_from_another_source() {
    let env = Env::new();
    let x = env.registry("x");
    let reg = env.registry("reg");
    write_skill(&x, "d", "", "from x");
    write_skill(&reg, "r", &cross_requires(&x), "first");
    env.run(&["add", s(&x), "d"]);
    let file = env.consumer().join(".claude/skills/d/SKILL.md");
    let before = fs::read(&file).unwrap();
    env.run(&["add", s(&reg), "r", "--exclude", "d"]);
    assert_eq!(env.lock()["excludes"], serde_json::json!(["d"]));

    write_skill(&reg, "r", &cross_requires(&x), "second");
    env.run(&["update", "--yes"]);

    let lock = env.lock();
    let d = lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "d")
        .unwrap();
    assert_eq!(
        d["source"].as_str().unwrap(),
        format!("local:{}", x.display()),
        "{lock}"
    );
    assert_eq!(fs::read(&file).unwrap(), before);
}

#[test]
fn the_source_label_of_a_local_item_is_local_and_the_path() {
    let env = Env::new();
    let x = env.registry("x");
    write_skill(&x, "d", "", "from x");
    env.run(&["add", s(&x), "d"]);

    let lock = env.lock();
    let d = lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "d")
        .unwrap();
    let expected = format!("local:{}", x.display());
    assert_eq!(d["source"].as_str().unwrap(), expected, "{lock}");
}

fn excluded_names(lock: &serde_json::Value) -> Vec<String> {
    lock["excludes"]
        .as_array()
        .map(|a| a.iter().map(|n| n.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn the_lockfile_records_one_sorted_excludes_list() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "d", "", "dependency");
    write_skill(&reg, "e", "", "dependency");
    write_skill(&reg, "r", "requires:\n  skills: [d, e]\n", "first");
    env.run(&["add", s(&reg), "r", "--exclude", "e", "--exclude", "d"]);
    assert_eq!(env.lock()["excludes"], serde_json::json!(["d", "e"]));
}

#[test]
fn an_exclude_holds_across_a_symlinked_source_path() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "d", "", "dependency");
    write_skill(&reg, "r", "requires:\n  skills: [d]\n", "first");
    let link = env.tmp.path().join("link");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&reg, &link).unwrap();
    #[cfg(not(unix))]
    return;
    env.run(&["add", s(&link), "r", "--exclude", "d"]);
    assert!(!env.installed("d"));

    write_skill(&reg, "r", "requires:\n  skills: [d]\n", "second");
    env.run(&["update", "--yes"]);
    assert!(!env.installed("d"));
    assert_eq!(env.lock()["excludes"], serde_json::json!(["d"]));
}

#[test]
fn update_keeps_the_exclude_of_an_item_installed_before_the_exclude() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "a", "", "first");
    write_skill(&reg, "d", "", "first");
    env.run(&["add", s(&reg)]);
    assert!(env.installed("a") && env.installed("d"));
    env.run(&["add", s(&reg), "--exclude", "d"]);
    assert_eq!(excluded_names(&env.lock()), ["d"]);
    // The add removes the installed d, files and lock entry.
    assert!(!env.installed("d"));
    assert!(!has_lock_item(&env.lock(), "d"), "{}", env.lock());

    env.run(&["update", "--yes"]);
    let lock = env.lock();
    assert_eq!(excluded_names(&lock), ["d"], "{lock}");
    assert!(!env.installed("d"));
    assert!(env.installed("a"));
}

#[test]
fn update_keeps_the_exclude_of_a_dependency_installed_before_the_exclude() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "d", "", "dependency");
    write_skill(&reg, "r", "requires:\n  skills: [d]\n", "first");
    write_skill(&reg, "x", "", "other");
    env.run(&["add", s(&reg), "r"]);
    assert!(env.installed("d"));
    env.run(&["add", s(&reg), "x", "--exclude", "d"]);
    assert_eq!(excluded_names(&env.lock()), ["d"]);

    env.run(&["update", "--yes"]);
    let lock = env.lock();
    assert_eq!(excluded_names(&lock), ["d"], "{lock}");
    assert!(!env.installed("d"));
    assert!(!has_lock_item(&lock, "d"), "{lock}");
    env.run(&["update", "--yes"]);
    assert_eq!(excluded_names(&env.lock()), ["d"], "{}", env.lock());
}

fn stdout_of(out: &Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn add_of_a_requiring_item_keeps_an_excluded_dependency_out() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "d", "", "dependency");
    write_skill(&reg, "r", "requires:\n  skills: [d]\n", "first");
    write_skill(&reg, "x", "", "other");
    env.run(&["add", s(&reg), "x", "--exclude", "d"]);
    env.run(&["add", s(&reg), "r"]);

    assert!(env.installed("r"));
    assert!(!env.installed("d"));
    let lock = env.lock();
    assert!(!has_lock_item(&lock, "d"), "{lock}");
    assert_eq!(excluded_names(&lock), ["d"], "{lock}");

    let stdout = stdout_of(&env.run(&["update", "--yes"]));
    assert!(!stdout.contains("removed"), "{stdout}");
}

#[test]
fn a_whole_source_add_keeps_excluded_items_out() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "a", "", "first");
    write_skill(&reg, "d", "", "first");
    env.run(&["add", s(&reg), "a", "--exclude", "d"]);
    env.run(&["add", s(&reg)]);

    assert!(env.installed("a"));
    assert!(!env.installed("d"));
    assert_eq!(excluded_names(&env.lock()), ["d"], "{}", env.lock());
}

#[test]
fn naming_and_excluding_the_same_item_keeps_it_excluded() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "a", "", "first");
    write_skill(&reg, "d", "", "first");
    env.run(&["add", s(&reg), "a", "--exclude", "d"]);
    env.run(&["add", s(&reg), "d", "--exclude", "d"]);

    assert!(!env.installed("d"));
    assert_eq!(excluded_names(&env.lock()), ["d"], "{}", env.lock());
}

#[test]
fn add_removes_the_only_installed_item_that_it_excludes() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "b", "", "first");
    env.run(&["add", s(&reg), "b"]);
    assert!(env.installed("b"));
    env.run(&["add", s(&reg), "--exclude", "b"]);
    assert!(!env.installed("b"));
    assert!(!has_lock_item(&env.lock(), "b"), "{}", env.lock());
    assert_eq!(excluded_names(&env.lock()), ["b"]);

    env.run(&["update", "--yes"]);
    assert!(!env.installed("b"));
}

#[test]
fn add_removes_only_the_excluded_member_of_a_group() {
    let env = Env::new();
    let reg = env.registry("reg");
    let dir = reg.join("g");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nschema: 1\nname: g\ndescription: group skill.\n---\n\n# g\n",
    )
    .unwrap();
    fs::write(
        dir.join("RULE.md"),
        "---\nschema: 1\nname: g-rule\ndescription: group rule.\n---\n\n# g\n",
    )
    .unwrap();
    env.run(&["add", s(&reg)]);
    assert!(env.installed("g"));
    let rule = env.consumer().join(".claude/rules/g-rule.md");
    assert!(rule.exists());

    env.run(&["add", s(&reg), "--exclude", "g"]);

    assert!(!env.installed("g"));
    assert!(rule.exists(), "the member that is not excluded stays");
    assert!(has_lock_item(&env.lock(), "g-rule"), "{}", env.lock());
    assert!(!has_lock_item(&env.lock(), "g"), "{}", env.lock());
}

#[test]
fn add_that_removes_an_excluded_bundle_member_keeps_the_bundle_record() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "a", "", "first");
    write_skill(&reg, "d", "", "first");
    write_bundle(&reg, "b.bundle.yaml", &["a", "d"]);
    let bundle = reg.join("b.bundle.yaml");
    env.run(&["add", s(&bundle)]);
    assert!(env.installed("a") && env.installed("d"));

    env.run(&["add", s(&bundle), "--exclude", "d"]);
    let lock = env.lock();
    assert!(env.installed("a") && !env.installed("d"));
    assert_eq!(lock["bundles"].as_array().unwrap().len(), 1, "{lock}");
    assert!(!has_lock_item(&lock, "d"), "{lock}");

    write_skill(&reg, "a", "", "second");
    env.run(&["update", "--yes"]);
    let lock = env.lock();
    assert!(env.installed("a") && !env.installed("d"));
    assert_eq!(lock["bundles"].as_array().unwrap().len(), 1, "{lock}");
}

#[test]
fn add_reports_each_installed_item_that_it_removes_as_excluded() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "a", "", "first");
    write_skill(&reg, "d", "", "first");
    env.run(&["add", s(&reg)]);

    let stdout = stdout_of(&env.run(&["add", s(&reg), "--exclude", "d"]));
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains("removed — excluded"))
        .collect();
    assert_eq!(lines, ["removed — excluded skill d"], "{stdout}");
}

#[test]
fn an_add_that_installs_nothing_still_records_the_exclude() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "b", "", "first");
    env.run(&["add", s(&reg), "--exclude", "b"]);

    assert!(!env.installed("b"));
    assert_eq!(env.lock()["excludes"], serde_json::json!(["b"]));
    common::upskill_cmd(&env.tmp.path().join("home"))
        .current_dir(env.consumer())
        .args(["doctor", "--project"])
        .assert()
        .success();
}

fn lock_item<'a>(lock: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == name)
        .unwrap()
}

fn required_by_is_empty(item: &serde_json::Value) -> bool {
    item.get("required_by")
        .and_then(|r| r.as_array())
        .is_none_or(|a| a.is_empty())
}

#[test]
fn an_excluded_dependency_gets_no_required_by_link_on_a_direct_install() {
    let env = Env::new();
    let (reg, dep) = cross_source_setup(&env);
    env.run(&["add", s(&dep), "d"]);
    env.run(&["add", s(&reg), "r", "--exclude", "d"]);

    let lock = env.lock();
    let d = lock_item(&lock, "d");
    assert_eq!(
        d["source"].as_str().unwrap(),
        format!("local:{}", dep.display()),
        "{lock}"
    );
    assert!(required_by_is_empty(d), "{lock}");

    env.run(&["update", "--yes"]);
    let lock = env.lock();
    assert!(env.installed("d"));
    assert!(required_by_is_empty(lock_item(&lock, "d")), "{lock}");
}

#[test]
fn an_excluded_dependency_required_by_another_source_stays_out() {
    let env = Env::new();
    let (reg, dep) = cross_source_setup(&env);
    env.run(&["add", s(&reg), "r", "--exclude", "d"]);

    write_skill(&reg, "r", &cross_requires(&dep), "second");
    env.run(&["update", "--yes"]);
    assert!(!env.installed("d"));
    assert!(!has_lock_item(&env.lock(), "d"), "{}", env.lock());

    let stdout = stdout_of(&env.run(&["update", "--yes"]));
    assert!(!env.installed("d"));
    assert!(!stdout.contains("removed"), "{stdout}");
}

#[test]
fn an_excluded_cross_source_transitive_dependency_stays_out() {
    let env = Env::new();
    let reg = env.registry("reg");
    let dep = env.registry("dep");
    write_skill(&dep, "d", "", "dependency");
    write_skill(&dep, "m", "requires:\n  skills: [d]\n", "middle");
    write_skill(
        &reg,
        "r",
        &format!(
            "requires:\n  skills: [{{ name: m, source: {} }}]\n",
            s(&dep)
        ),
        "first",
    );
    env.run(&["add", s(&reg), "r", "--exclude", "d"]);

    assert!(env.installed("r") && env.installed("m"));
    assert!(!env.installed("d"));
    assert!(!has_lock_item(&env.lock(), "d"), "{}", env.lock());
}

#[test]
fn an_excluded_member_of_a_required_bundle_stays_out() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "a", "", "first");
    write_skill(&reg, "d", "", "first");
    write_skill(&reg, "e", "", "first");
    write_bundle(&reg, "inner.bundle.yaml", &["d", "e"]);
    write_bundle(&reg, "outer.bundle.yaml", &["a"]);
    let outer = reg.join("outer.bundle.yaml");
    let text = fs::read_to_string(&outer).unwrap();
    fs::write(&outer, format!("{text}requires:\n  - name: inner\n")).unwrap();

    env.run(&["add", s(&outer), "--exclude", "d"]);
    assert!(env.installed("a") && env.installed("e"));
    assert!(!env.installed("d"));
    assert!(!has_lock_item(&env.lock(), "d"), "{}", env.lock());
}

#[test]
fn add_that_removes_an_excluded_item_leaves_no_link_to_it() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "e", "", "dependency");
    write_skill(&reg, "d", "requires:\n  skills: [e]\n", "first");
    write_skill(&reg, "x", "", "other");
    env.run(&["add", s(&reg), "d"]);
    assert_eq!(
        lock_item(&env.lock(), "e")["required_by"],
        serde_json::json!(["skill:d"])
    );

    env.run(&["add", s(&reg), "x", "--exclude", "d"]);

    let lock = env.lock();
    assert!(!has_lock_item(&lock, "d"), "{lock}");
    assert!(
        lock["items"].as_array().unwrap().iter().all(|i| {
            i.get("required_by")
                .and_then(|r| r.as_array())
                .is_none_or(|a| a.iter().all(|r| r != "skill:d"))
        }),
        "{lock}"
    );
}

fn stderr_of(out: &Assert) -> String {
    String::from_utf8(out.get_output().stderr.clone()).unwrap()
}

#[test]
fn an_exclude_that_matches_nothing_is_refused() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "a", "", "first");
    env.run(&["add", s(&reg), "a"]);
    let lock_path = env.consumer().join(".upskill-lock.json");
    let before = fs::read(&lock_path).unwrap();

    let out = common::upskill_cmd(&env.tmp.path().join("home"))
        .current_dir(env.consumer())
        .args(["add", s(&reg), "--exclude", "nothing-by-this-name"])
        .args(["--project", "--claude"])
        .assert()
        .code(2);

    let stderr = stderr_of(&out);
    assert!(stderr.contains("typo"), "{stderr}");
    assert!(stderr.contains("nothing-by-this-name"), "{stderr}");
    assert_eq!(fs::read(&lock_path).unwrap(), before);
    assert!(env.installed("a"));
}

#[test]
fn add_prints_each_excluded_item() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "d", "", "dependency");
    write_skill(&reg, "r", "requires:\n  skills: [d]\n", "first");
    write_skill(&reg, "r2", "requires:\n  skills: [d]\n", "second");
    write_skill(&reg, "w", "", "whole source skip");
    let out = env.run(&["add", s(&reg), "--exclude", "d", "--exclude", "w"]);

    let stderr = stderr_of(&out);
    assert_eq!(stderr.matches("excluded skill d").count(), 1, "{stderr}");
    assert!(stderr.contains("excluded skill w"), "{stderr}");
    assert!(!env.installed("d"));
}

#[test]
fn an_item_removed_by_this_run_is_not_also_listed_as_excluded() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "b", "", "first");
    write_skill(&reg, "c", "", "second");
    env.run(&["add", s(&reg), "b"]);
    let out = env.run(&["add", s(&reg), "--exclude", "b"]);

    let stderr = stderr_of(&out);
    let stdout = stdout_of(&out);
    assert!(stdout.contains("removed"), "{stdout}");
    assert!(!stderr.contains("excluded skill b"), "{stderr}");
}

#[test]
fn quiet_hides_the_excluded_lines() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "a", "", "first");
    write_skill(&reg, "d", "", "first");
    let out = env.run(&["add", s(&reg), "--exclude", "d", "--quiet"]);

    let stderr = stderr_of(&out);
    assert!(!stderr.contains("excluded"), "{stderr}");
}

#[test]
fn update_prints_the_items_that_an_exclude_keeps_out() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "d", "", "dependency");
    write_skill(&reg, "r", "requires:\n  skills: [d]\n", "first");
    write_skill(&reg, "x", "", "other");
    env.run(&["add", s(&reg), "r", "x", "--exclude", "d"]);

    let stderr = stderr_of(&env.run(&["update", "--yes"]));
    assert_eq!(stderr.matches("excluded skill d").count(), 1, "{stderr}");
    let stderr = stderr_of(&env.run(&["update", "--dry-run"]));
    assert!(stderr.contains("excluded skill d"), "{stderr}");
    let stderr = stderr_of(&env.run(&["update", "--yes", "--quiet"]));
    assert!(!stderr.contains("excluded"), "{stderr}");
}
