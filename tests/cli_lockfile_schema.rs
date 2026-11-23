//! The lockfile is written as `schema: 3`. A `schema: 2` lockfile is read and
//! written back as `schema: 3` with its content kept, so an upskill version
//! that predates the `excludes` list refuses the file instead of dropping the
//! list. A `schema: 4` lockfile is refused. All commands use the project scope
//! in a tempdir with a fake `$HOME`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

struct Env {
    tmp: tempfile::TempDir,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("home")).unwrap();
        fs::create_dir_all(tmp.path().join("consumer")).unwrap();
        let reg = tmp.path().join("reg");
        for name in ["a", "b", "c"] {
            let dir = reg.join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("SKILL.md"),
                format!(
                    "---\nschema: 1\nname: {name}\ndescription: skill {name}.\n---\n\n# {name}\n"
                ),
            )
            .unwrap();
        }
        Env { tmp }
    }

    fn consumer(&self) -> PathBuf {
        self.tmp.path().join("consumer")
    }

    fn reg(&self) -> String {
        self.tmp.path().join("reg").to_str().unwrap().to_string()
    }

    fn cmd(&self, args: &[&str]) -> assert_cmd::Command {
        let mut cmd = common::upskill_cmd(&self.tmp.path().join("home"));
        cmd.current_dir(self.consumer()).args(args).arg("--project");
        if args[0] != "remove" {
            cmd.arg("--claude");
        }
        cmd
    }

    fn lock_path(&self) -> PathBuf {
        self.consumer().join(".upskill-lock.json")
    }

    fn lock(&self) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(self.lock_path()).unwrap()).unwrap()
    }

    /// Rewrites the lockfile with the given `schema` number and keeps all
    /// other content.
    fn set_schema(&self, schema: u32) {
        let mut lock = self.lock();
        lock["schema"] = schema.into();
        fs::write(self.lock_path(), serde_json::to_vec_pretty(&lock).unwrap()).unwrap();
    }

    /// Installs `a` and `b` and records `c` as excluded, then rewrites the
    /// lockfile as `schema: 2`.
    fn schema_2_lockfile(&self) {
        let reg = self.reg();
        self.cmd(&["add", &reg, "a", "b"]).assert().success();
        let mut lock = self.lock();
        lock["excludes"] = serde_json::json!(["c"]);
        fs::write(self.lock_path(), serde_json::to_vec_pretty(&lock).unwrap()).unwrap();
        self.set_schema(2);
    }
}

fn names(lock: &serde_json::Value) -> Vec<String> {
    lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap().to_string())
        .collect()
}

fn installed(root: &Path, name: &str) -> bool {
    root.join(".claude/skills").join(name).exists()
}

#[test]
fn add_writes_schema_3() {
    let env = Env::new();
    env.cmd(&["add", &env.reg(), "a"]).assert().success();
    assert_eq!(env.lock()["schema"], 3);
}

#[test]
fn add_rewrites_a_schema_2_lockfile_as_schema_3_and_keeps_its_content() {
    let env = Env::new();
    env.schema_2_lockfile();
    let before = env.lock();

    env.cmd(&["add", &env.reg(), "a"]).assert().success();

    let after = env.lock();
    assert_eq!(after["schema"], 3);
    assert_eq!(after["excludes"], serde_json::json!(["c"]));
    assert_eq!(after["items"], before["items"]);
}

#[test]
fn update_rewrites_a_schema_2_lockfile_as_schema_3_and_keeps_its_content() {
    let env = Env::new();
    env.schema_2_lockfile();
    let before = env.lock();

    env.cmd(&["update", "--yes"]).assert().success();

    let after = env.lock();
    assert_eq!(after["schema"], 3, "{after}");
    assert_eq!(after["excludes"], serde_json::json!(["c"]));
    assert_eq!(after["items"], before["items"]);
    assert!(!installed(&env.consumer(), "c"));
}

#[test]
fn remove_rewrites_a_schema_2_lockfile_as_schema_3_and_keeps_its_content() {
    let env = Env::new();
    env.schema_2_lockfile();

    env.cmd(&["remove", "b"]).assert().success();

    let after = env.lock();
    assert_eq!(after["schema"], 3, "{after}");
    assert_eq!(after["excludes"], serde_json::json!(["c"]));
    assert_eq!(names(&after), vec!["a".to_string()]);
}

#[test]
fn a_schema_4_lockfile_is_refused_with_its_version() {
    let env = Env::new();
    env.cmd(&["add", &env.reg(), "a"]).assert().success();
    env.set_schema(4);
    let before = fs::read(env.lock_path()).unwrap();

    let out = env.cmd(&["add", &env.reg(), "b"]).assert().failure();

    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("schema 4"), "stderr: {stderr}");
    assert_eq!(fs::read(env.lock_path()).unwrap(), before);
}

#[test]
fn a_schema_1_lockfile_is_adopted_as_schema_3() {
    let env = Env::new();
    env.cmd(&["add", &env.reg(), "a"]).assert().success();
    let mut lock = env.lock();
    for item in lock["items"].as_array_mut().unwrap() {
        item.as_object_mut().unwrap().remove("files");
    }
    lock["schema"] = 1.into();
    fs::write(env.lock_path(), serde_json::to_vec_pretty(&lock).unwrap()).unwrap();

    env.cmd(&["add", &env.reg(), "b"]).assert().success();

    let after = env.lock();
    assert_eq!(after["schema"], 3, "{after}");
    assert!(env.consumer().join(".upskill-lock.json.v1").exists());
    assert_eq!(names(&after), vec!["a".to_string(), "b".to_string()]);
}
