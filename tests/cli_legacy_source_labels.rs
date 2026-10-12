//! Persisted GitLab labels update through real git fetches without network access.
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use predicates::prelude::*;
use serde_json::{Value, json};

struct Fixture {
    _tmp: tempfile::TempDir,
    home: PathBuf,
    repo: PathBuf,
    target: PathBuf,
    remote: String,
}

impl Fixture {
    fn new(remote: &str) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let repo = tmp.path().join("repo");
        let target = tmp.path().join("target");
        for dir in [&home, &repo, &target, &target.join(".git")] {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(home.join("empty-gitconfig"), "").unwrap();
        let fixture = Self {
            _tmp: tmp,
            home,
            repo,
            target,
            remote: remote.into(),
        };
        fixture.git(&["init"]);
        fixture.write_skill("content/skills/original/SKILL.md", "Initial content.");
        fixture.write_skill("skills/original/SKILL.md", "Initial content.");
        fs::write(
            fixture.repo.join("skills/team.bundle.yaml"),
            "schema: 1\nname: team\ndescription: Team skills\nitems:\n  skills:\n    - original\n",
        )
        .unwrap();
        fixture.commit();
        fixture
    }

    fn write_skill(&self, path: &str, body: &str) {
        let path = self.repo.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("---\nschema: 1\nname: original\ndescription: Use for testing source updates\n---\n\n{body}\n")).unwrap();
    }

    fn git(&self, args: &[&str]) {
        let mut cmd = Command::new("git");
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_") {
                cmd.env_remove(key);
            }
        }
        let output = cmd
            .current_dir(&self.repo)
            .args(args)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.home.join("empty-gitconfig"))
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn commit(&self) {
        self.git(&["add", "."]);
        self.git(&["commit", "-m", "test content"]);
    }

    fn cmd(&self) -> assert_cmd::Command {
        let mut cmd = common::upskill_cmd(&self.home);
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_") {
                cmd.env_remove(key);
            }
        }
        cmd.current_dir(&self.target)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.home.join("empty-gitconfig"))
            .env("GIT_CONFIG_COUNT", "2")
            .env(
                "GIT_CONFIG_KEY_0",
                format!("url.file://{}.insteadOf", self.repo.display()),
            )
            .env("GIT_CONFIG_VALUE_0", format!("{}.git", self.remote))
            .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
            .env("GIT_CONFIG_VALUE_1", "always")
            // If a regression bypasses the rewrite, fail locally before DNS/network.
            .env("GIT_ALLOW_PROTOCOL", "file")
            .env("GIT_TERMINAL_PROMPT", "0");
        cmd
    }

    fn lock(&self) -> Value {
        serde_json::from_slice(&fs::read(self.target.join(".upskill-lock.json")).unwrap()).unwrap()
    }

    fn save_lock(&self, lock: &Value) {
        fs::write(
            self.target.join(".upskill-lock.json"),
            serde_json::to_vec_pretty(lock).unwrap(),
        )
        .unwrap();
    }

    fn historicize(&self, modern: &str, legacy: &str) {
        let mut lock = self.lock();
        for bucket in ["items", "bundles"] {
            for entry in lock[bucket].as_array_mut().unwrap() {
                if entry["source"] == modern {
                    entry["source"] = legacy.into();
                }
            }
        }
        self.save_lock(&lock);
    }

    fn output(&self, name: &str) -> Vec<u8> {
        fs::read(self.target.join(format!(".claude/skills/{name}/SKILL.md"))).unwrap()
    }
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_name() == ".git" {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                walk(root, &entry.path(), files);
            } else {
                files.insert(
                    entry.path().strip_prefix(root).unwrap().into(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

#[test]
fn legacy_item_update_preserves_alias_subfolder_and_preview_bytes() {
    // Removing compatibility parsing, dropping aliases, or losing the HTTPS port
    // must break this real clone/update example.
    for (remote, legacy) in [
        (
            "https://gitlab.com/org/team/repo",
            "gitlab:org/team/repo:content",
        ),
        (
            "https://gitlab.example.com:8443/org/team/repo",
            "gitlab+gitlab.example.com:8443:org/team/repo:content",
        ),
    ] {
        let f = Fixture::new(remote);
        let modern = format!("{remote}:content");
        f.cmd()
            .args(["add", &modern, "original", "--as", "installed", "--claude"])
            .assert()
            .success();
        f.historicize(&modern, legacy);
        let old_lock = f.lock();
        let before = snapshot(&f.target);
        f.write_skill(
            "content/skills/original/SKILL.md",
            "Updated upstream content.",
        );
        f.commit();
        f.cmd().args(["list"]).assert().success();
        f.cmd().args(["doctor"]).assert().success();
        assert_eq!(
            snapshot(&f.target),
            before,
            "inspection must preserve bytes"
        );
        f.cmd()
            .args(["update", "installed", "--dry-run"])
            .assert()
            .success()
            .stdout(predicate::str::contains("would change"));
        assert_eq!(
            snapshot(&f.target),
            before,
            "preview must preserve all bytes"
        );
        f.cmd().args(["update", "installed"]).assert().success();
        assert!(
            String::from_utf8(f.output("installed"))
                .unwrap()
                .contains("Updated upstream content.")
        );
        assert!(!f.target.join(".claude/skills/original").exists());
        let lock = f.lock();
        let mut expected = old_lock;
        expected["items"][0]["source"] = modern.into();
        expected["items"][0]["hash"] = lock["items"][0]["hash"].clone();
        assert_eq!(
            lock, expected,
            "only source spelling and changed hash may change"
        );
    }
}

#[test]
fn reported_legacy_bundle_updates_and_preserves_unrelated_metadata() {
    let f = Fixture::new("https://gitlab.example.com/org/team/repo");
    let modern = "https://gitlab.example.com/org/team/repo:skills/team.bundle.yaml";
    let legacy = "gitlab+gitlab.example.com:org/team/repo:skills/team.bundle.yaml";
    f.cmd().args(["add", modern, "--claude"]).assert().success();
    let unrelated = f._tmp.path().join("unrelated");
    fs::create_dir_all(unrelated.join("separate")).unwrap();
    fs::write(unrelated.join("separate/SKILL.md"), "---\nschema: 1\nname: separate\ndescription: Use for unrelated work\n---\n\nUnrelated content.\n").unwrap();
    f.cmd()
        .args(["add", unrelated.to_str().unwrap(), "--claude"])
        .assert()
        .success();
    f.historicize(modern, legacy);
    let mut old_lock = f.lock();
    old_lock["plugins"] = json!([{"name":"retained-plugin","client":"opencode","identifier":"retained-module","bundle":"unrelated-bundle","status":"instructions"}]);
    old_lock["mcps"] = json!([{"name":"retained-mcp","client":"opencode","bundle":"unrelated-bundle","status":"skipped"}]);
    f.save_lock(&old_lock);
    let before = snapshot(&f.target);
    f.write_skill("skills/original/SKILL.md", "Changed bundle content.");
    f.commit();
    f.cmd().args(["list"]).assert().success();
    f.cmd().args(["doctor"]).assert().success();
    assert_eq!(snapshot(&f.target), before);
    f.cmd()
        .args(["update", "original", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("would change"));
    assert_eq!(snapshot(&f.target), before);
    f.cmd().args(["update", "original"]).assert().success();
    assert!(
        String::from_utf8(f.output("original"))
            .unwrap()
            .contains("Changed bundle content.")
    );
    assert_eq!(
        f.output("separate"),
        before[Path::new(".claude/skills/separate/SKILL.md")]
    );
    let lock = f.lock();
    let mut expected = old_lock;
    for entry in expected["items"].as_array_mut().unwrap() {
        if entry["name"] == "original" {
            entry["source"] = modern.into();
            entry["hash"] = lock["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["name"] == "original")
                .unwrap()["hash"]
                .clone();
        }
    }
    expected["bundles"][0]["source"] = modern.into();
    assert_eq!(
        lock, expected,
        "preserve schema, bundle membership and unrelated records"
    );
}

#[test]
fn legacy_pinned_ref_stays_selected_when_default_branch_advances() {
    let f = Fixture::new("https://gitlab.com/org/team/repo");
    f.git(&["tag", "v1"]);
    let modern = "https://gitlab.com/org/team/repo@v1:content";
    f.cmd()
        .args(["add", modern, "original", "--as", "pinned", "--claude"])
        .assert()
        .success();
    f.historicize(modern, "gitlab:org/team/repo@v1:content");
    let before = snapshot(&f.target);
    let old_lock = f.lock();
    f.write_skill(
        "content/skills/original/SKILL.md",
        "Must not select default branch.",
    );
    f.commit();
    f.cmd()
        .args(["update", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("would update 0 of 1"));
    assert_eq!(snapshot(&f.target), before);
    f.cmd().args(["update"]).assert().success();
    assert_eq!(
        f.output("pinned"),
        before[Path::new(".claude/skills/pinned/SKILL.md")]
    );
    let mut expected = old_lock;
    expected["items"][0]["source"] = modern.into();
    assert_eq!(
        f.lock(),
        expected,
        "pin, alias and hash must survive unchanged-content update"
    );
}

#[test]
fn invalid_legacy_label_fails_before_fetch_with_reconciliation_context() {
    let f = Fixture::new("https://gitlab.com/org/team/repo");
    let modern = "https://gitlab.com/org/team/repo:content";
    f.cmd().args(["add", modern, "--claude"]).assert().success();
    let invalid = "gitlab+user@gitlab.example.com:org/team/repo";
    f.historicize(modern, invalid);
    let before = snapshot(&f.target);
    for args in [vec!["update"], vec!["update", "--dry-run"]] {
        f.cmd()
            .args(args)
            .assert()
            .code(1)
            .stderr(predicate::str::contains(invalid))
            .stderr(predicate::str::contains("reconcile"))
            .stderr(predicate::str::contains("https://"))
            .stderr(predicate::str::contains("clone").not());
        assert_eq!(snapshot(&f.target), before);
    }
    f.cmd().args(["list"]).assert().success();
    assert_eq!(
        snapshot(&f.target),
        before,
        "invalid legacy state remains inspectable"
    );
}
