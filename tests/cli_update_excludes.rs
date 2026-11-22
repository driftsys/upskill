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

    fn remove(&self, name: &str) {
        common::upskill_cmd(&self.tmp.path().join("home"))
            .current_dir(self.consumer())
            .args(["remove", name, "--project"])
            .assert()
            .success();
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
    assert_eq!(
        lock["excludes"][0]["names"],
        serde_json::json!(["d"]),
        "{lock}"
    );

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

    write_skill(&reg, "r", &cross_requires(&x), "second");
    env.run(&["update", "--yes"]);

    let lock = env.lock();
    let d = lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "d")
        .unwrap();
    assert!(d["source"].as_str().unwrap().contains("/x"), "{lock}");
    assert_eq!(fs::read(&file).unwrap(), before);
}

fn excluded_names(lock: &serde_json::Value) -> Vec<String> {
    lock["excludes"][0]["names"]
        .as_array()
        .map(|a| a.iter().map(|n| n.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn removing_the_last_item_of_a_source_deletes_its_excludes_record() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "d", "", "dependency");
    write_skill(&reg, "r", "requires:\n  skills: [d]\n", "first");
    env.run(&["add", s(&reg), "r", "--exclude", "d"]);
    assert_eq!(excluded_names(&env.lock()), ["d"]);

    env.remove("r");

    let lock = env.lock();
    assert!(lock.get("excludes").is_none(), "{lock}");
}

#[test]
fn removing_one_of_two_items_keeps_the_excludes_record() {
    let env = Env::new();
    let reg = env.registry("reg");
    write_skill(&reg, "d", "", "dependency");
    write_skill(&reg, "r", "requires:\n  skills: [d]\n", "first");
    write_skill(&reg, "q", "", "other");
    env.run(&["add", s(&reg), "r", "q", "--exclude", "d"]);
    assert_eq!(excluded_names(&env.lock()), ["d"]);

    env.remove("r");

    let lock = env.lock();
    assert!(has_lock_item(&lock, "q"), "{lock}");
    assert_eq!(excluded_names(&lock), ["d"], "{lock}");
}
