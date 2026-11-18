//! opencode plugins (issue #229): the `doctor` check, the install command
//! form of opencode 2.x, the version check before any `opencode plugin`
//! command, and the repair of a missing plugin by `update --yes`.
//!
//! A fake `opencode` script, first on `PATH`, records every argument list in
//! a log file, answers `--version` with a fixed version, and answers
//! `plugin list` with the content of a list file. Unix only: the fake is a
//! shell script.
#![cfg(unix)]

mod common;

use std::fs;
use std::path::PathBuf;

const URI: &str = "superpowers@git+https://example.com/superpowers.git";
const MODULE: &str = "opencode-sp-module";

struct Env {
    _tmp: tempfile::TempDir,
    home: PathBuf,
    project: PathBuf,
    bundle: PathBuf,
    bin: PathBuf,
}

impl Env {
    /// A project, a registry with one skill and a bundle that declares one
    /// opencode plugin (`opencode_yaml` is the descriptor body), and a fake
    /// `opencode` that prints `version` for `--version`.
    fn new(opencode_yaml: &str, version: &str) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        // The canonical path: the recorded local source label must name the
        // same path that `doctor` resolves later (`/var` is a link on macOS).
        let root = tmp.path().canonicalize().unwrap();
        let home = root.join("home");
        fs::create_dir_all(&home).unwrap();
        let project = root.join("project");
        fs::create_dir_all(project.join(".git")).unwrap();
        let registry = root.join("registry");
        let skill = registry.join("alpha").join("SKILL.md");
        fs::create_dir_all(skill.parent().unwrap()).unwrap();
        fs::write(
            &skill,
            "---\nschema: 1\nname: alpha\ndescription: skill for the issue 229 tests.\n---\n\n## Body\n\nText.\n",
        )
        .unwrap();
        let bundle = registry.join("oc.bundle.yaml");
        fs::write(
            &bundle,
            format!(
                "schema: 1\nname: oc\ndescription: bundle with an opencode plugin.\nitems:\n  skills:\n    - alpha\nplugins:\n  superpowers:\n    opencode:\n      {opencode_yaml}\n"
            ),
        )
        .unwrap();
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let script = bin.join("opencode");
        let log = bin.join("log.txt");
        let list = bin.join("list.txt");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\necho \"$*\" >> '{log}'\nif [ \"$1\" = \"--version\" ]; then echo 'opencode {version}'; exit 0; fi\nif [ \"$1\" = \"plugin\" ] && [ \"$2\" = \"list\" ]; then cat '{list}' 2>/dev/null; fi\nexit 0\n",
                log = log.display(),
                list = list.display(),
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        Env {
            _tmp: tmp,
            home,
            project,
            bundle,
            bin,
        }
    }

    fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        let path = format!("{}:/usr/bin:/bin", self.bin.display());
        common::upskill_cmd(&self.home)
            .current_dir(&self.project)
            .env("PATH", path)
            .args(args)
            .assert()
    }

    fn add(&self) -> assert_cmd::assert::Assert {
        // The registry directory is the source, and the bundle is named: a
        // bundle file as the source is reported by `doctor` as a local path
        // that is gone, because `doctor` expects a directory.
        let registry = self.bundle.parent().unwrap();
        self.run(&["add", registry.to_str().unwrap(), "oc", "--project"])
    }

    fn log(&self) -> String {
        fs::read_to_string(self.bin.join("log.txt")).unwrap_or_default()
    }

    fn clear_log(&self) {
        let _ = fs::remove_file(self.bin.join("log.txt"));
    }

    fn set_list(&self, body: &str) {
        fs::write(self.bin.join("list.txt"), body).unwrap();
    }

    fn opencode_json(&self) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(self.project.join("opencode.json")).unwrap())
            .unwrap()
    }

    fn write_opencode_json(&self, value: &serde_json::Value) {
        fs::write(
            self.project.join("opencode.json"),
            serde_json::to_string_pretty(value).unwrap(),
        )
        .unwrap();
    }
}

fn plugin_lines(log: &str) -> Vec<&str> {
    log.lines().filter(|l| l.starts_with("plugin")).collect()
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).to_string()
}

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).to_string()
}

fn has_uri(value: &serde_json::Value) -> bool {
    value["plugin"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v == URI))
}

#[test]
fn doctor_reads_opencode_json_for_a_plugin_uri_and_runs_no_plugin_command() {
    let env = Env::new(&format!("plugin_uri: \"{URI}\""), "v2.1.4");
    env.add().success();
    assert!(has_uri(&env.opencode_json()));
    env.clear_log();

    let out = env.run(&["doctor", "--project"]).code(0);
    let text = stdout(&out);
    assert!(!text.contains("missing"), "{text}");
    assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());
}

#[test]
fn doctor_reports_a_removed_plugin_uri_and_update_yes_restores_it() {
    let env = Env::new(&format!("plugin_uri: \"{URI}\""), "v2.1.4");
    env.add().success();
    let mut json = env.opencode_json();
    json["plugin"] = serde_json::json!([]);
    env.write_opencode_json(&json);

    let out = env.run(&["doctor", "--project"]).code(1);
    let text = stdout(&out);
    assert!(text.contains("missing"), "{text}");
    assert!(text.contains("superpowers"), "{text}");

    // `--dry-run` and `--check` write nothing.
    env.run(&["update", "--dry-run", "--project"]).success();
    env.run(&["update", "--check", "--project"]);
    assert!(!has_uri(&env.opencode_json()));

    env.run(&["update", "--yes", "--project"]).code(0);
    assert!(has_uri(&env.opencode_json()), "{}", env.opencode_json());

    env.clear_log();
    let out = env.run(&["doctor", "--project"]).code(0);
    assert!(!stdout(&out).contains("missing"), "{}", stdout(&out));
    assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());
}

#[test]
fn module_plugin_installs_with_plugin_add_and_doctor_uses_plugin_list() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add().success();
    let log = env.log();
    assert!(
        plugin_lines(&log).contains(&format!("plugin add {MODULE}").as_str()),
        "{log}"
    );
    assert!(log.lines().any(|l| l == "--version"), "{log}");

    env.clear_log();
    env.set_list(&format!("{MODULE}\n"));
    env.run(&["doctor", "--project"]).code(0);
    assert_eq!(
        plugin_lines(&env.log()),
        vec!["plugin list"],
        "{}",
        env.log()
    );
}

#[test]
fn update_yes_installs_a_missing_module_plugin_again_and_dry_run_does_not() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add().success();
    env.set_list("");
    let out = env.run(&["doctor", "--project"]).code(1);
    assert!(stdout(&out).contains("missing"), "{}", stdout(&out));

    env.clear_log();
    env.run(&["update", "--dry-run", "--project"]).success();
    env.run(&["update", "--check", "--project"]);
    assert!(
        !plugin_lines(&env.log())
            .iter()
            .any(|l| l.starts_with("plugin add")),
        "{}",
        env.log()
    );

    env.run(&["update", "--yes", "--project"]).code(0);
    assert!(
        plugin_lines(&env.log()).contains(&format!("plugin add {MODULE}").as_str()),
        "{}",
        env.log()
    );
}

#[test]
fn opencode_1x_runs_no_plugin_command_and_warns() {
    let env = Env::new(&format!("module: {MODULE}"), "v1.15.13");
    let out = env.add().success();
    let err = stderr(&out);
    assert!(err.contains("opencode 2"), "{err}");
    let lock = fs::read_to_string(env.project.join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("\"skipped\""), "{lock}");

    env.run(&["doctor", "--project"]).code(0);
    // The plugin is recorded as skipped, so `update` does not try it again.
    env.run(&["update", "--yes", "--project"]).code(0);
    env.run(&["remove", "alpha", "--project"]).success();

    assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());
}

#[test]
fn doctor_does_not_report_a_module_plugin_missing_when_opencode_is_1x() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add().success();
    // The user goes back to opencode 1.x after the install.
    let script = env.bin.join("opencode");
    let body = fs::read_to_string(&script)
        .unwrap()
        .replace("v2.1.4", "v1.15.13");
    fs::write(&script, body).unwrap();
    env.clear_log();

    let out = env.run(&["doctor", "--project"]).code(0);
    assert!(!stdout(&out).contains("missing"), "{}", stdout(&out));
    assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());
}
