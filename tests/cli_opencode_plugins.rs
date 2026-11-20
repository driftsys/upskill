//! opencode plugins (issue #229): the `doctor` check, the install command
//! form of opencode 2.x, the version check before any `opencode plugin`
//! command, and the install of a missing or skipped plugin by `update`.
//!
//! `update --yes` applies every source, and the plugin step of the apply
//! attempts every plugin of the recorded bundles. A terminal run does not
//! apply a source whose preview changes nothing; there the plugin repair
//! installs a missing or skipped plugin. The tests named `terminal_...`, and
//! those that call `terminal_update`, cover the repair.
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
    /// `opencode` that prints `opencode <version>` for `--version`.
    fn new(opencode_yaml: &str, version: &str) -> Self {
        Self::build(
            &format!("  superpowers:\n    opencode:\n      {opencode_yaml}\n"),
            "  skills:\n    - alpha\n",
            version,
        )
    }

    /// As [`Env::new`], with the `plugins:` map body and the `items:` map
    /// body of the bundle given as YAML. Also writes a fake `code` that logs
    /// to `code-log.txt` and lists the content of `code-list.txt`.
    fn build(plugins_yaml: &str, items_yaml: &str, version: &str) -> Self {
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
        let items = if items_yaml.is_empty() {
            "items: {}\n".to_string()
        } else {
            format!("items:\n{items_yaml}")
        };
        let bundle = registry.join("oc.bundle.yaml");
        fs::write(
            &bundle,
            format!(
                "schema: 1\nname: oc\ndescription: bundle with an opencode plugin.\n{items}plugins:\n{plugins_yaml}"
            ),
        )
        .unwrap();
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let d = bin.display();
        write_script(
            &bin.join("opencode"),
            &format!(
                "#!/bin/sh\necho \"$*\" >> '{d}/log.txt'\nif [ \"$1\" = \"--version\" ]; then cat '{d}/version.txt'; exit $(cat '{d}/version-exit.txt' 2>/dev/null || echo 0); fi\nif [ \"$1\" = \"plugin\" ] && [ \"$2\" = \"list\" ]; then cat '{d}/list.txt' 2>/dev/null; fi\nexit 0\n"
            ),
        );
        write_script(
            &bin.join("code"),
            &format!(
                "#!/bin/sh\necho \"$*\" >> '{d}/code-log.txt'\nif [ \"$1\" = \"--list-extensions\" ]; then cat '{d}/code-list.txt' 2>/dev/null; fi\nexit 0\n"
            ),
        );
        let env = Env {
            _tmp: tmp,
            home,
            project,
            bundle,
            bin,
        };
        env.set_version(&format!("opencode {version}\n"), 0);
        env
    }

    /// What the fake `opencode --version` prints, and its exit code.
    fn set_version(&self, reply: &str, exit: i32) {
        fs::write(self.bin.join("version.txt"), reply).unwrap();
        fs::write(self.bin.join("version-exit.txt"), exit.to_string()).unwrap();
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

    /// `add` with the bundle file as the source.
    fn add_bundle_file(&self) -> assert_cmd::assert::Assert {
        self.run(&["add", self.bundle.to_str().unwrap(), "--project"])
    }

    fn code_log(&self) -> String {
        fs::read_to_string(self.bin.join("code-log.txt")).unwrap_or_default()
    }

    fn lock(&self) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(self.project.join(".upskill-lock.json")).unwrap())
            .unwrap()
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

/// `update --project` in a terminal, with end of input as the answer. No
/// source has a change in these tests, so the run applies no source and only
/// the plugin repair can run a plugin command.
fn terminal_update(env: &Env) -> serde_json::Value {
    let path = format!("{}:/usr/bin:/bin", env.bin.display());
    let result = common::update_in_terminal_with_env(
        &env.project,
        &env.home,
        &["--project"],
        "\u{4}",
        &[("PATH", path)],
    );
    assert_eq!(result["returncode"], 0, "{result}");
    // The precondition: no source is applied.
    assert!(
        result["stdout"].as_str().unwrap().contains("nothing to do"),
        "{result}"
    );
    result
}

fn write_script(path: &std::path::Path, body: &str) {
    fs::write(path, body).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
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
    value["plugins"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v == URI))
}

#[test]
fn doctor_reads_opencode_json_for_a_plugin_uri_and_runs_no_plugin_command() {
    let env = Env::new(&format!("plugin_uri: \"{URI}\""), "v2.1.4");
    env.add().success();
    assert!(has_uri(&env.opencode_json()));
    env.clear_log();

    assert_eq!(env.lock()["plugins"][0]["config_write"], true);

    let out = env.run(&["doctor", "--project"]).code(0);
    let text = stdout(&out);
    assert!(!text.contains("missing"), "{text}");
    // No opencode command at all, not even `--version`.
    assert_eq!(env.log(), "");
}

#[test]
fn doctor_reports_a_removed_plugin_uri_and_update_yes_restores_it() {
    let env = Env::new(&format!("plugin_uri: \"{URI}\""), "v2.1.4");
    env.add().success();
    let mut json = env.opencode_json();
    json["plugins"] = serde_json::json!([]);
    env.write_opencode_json(&json);
    env.clear_log();

    let out = env.run(&["doctor", "--project"]).code(1);
    let text = stdout(&out);
    assert!(text.contains("missing"), "{text}");
    assert!(text.contains("superpowers"), "{text}");
    assert_eq!(env.log(), "");

    // `--dry-run` and `--check` write nothing.
    env.run(&["update", "--dry-run", "--project"]).success();
    env.run(&["update", "--check", "--project"]);
    assert!(!has_uri(&env.opencode_json()));

    env.run(&["update", "--yes", "--project"]).code(0);
    assert!(has_uri(&env.opencode_json()), "{}", env.opencode_json());

    env.clear_log();
    let out = env.run(&["doctor", "--project"]).code(0);
    assert!(!stdout(&out).contains("missing"), "{}", stdout(&out));
    assert_eq!(env.log(), "");
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

/// A source under a kind restriction runs no plugin step in `update`, so the
/// plugin repair does not run for it either.
#[test]
fn update_yes_does_not_repair_a_plugin_of_a_source_under_a_kind_restriction() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add().success();
    // The second `add` records the kind restriction on the bundle. The
    // plugin record of the first `add` stays in the lockfile.
    let registry = env.bundle.parent().unwrap().to_path_buf();
    env.run(&[
        "add",
        registry.to_str().unwrap(),
        "oc",
        "--project",
        "--kind",
        "skill",
    ])
    .success();
    let lock = env.lock();
    assert_eq!(lock["bundles"][0]["kinds"][0], "skill", "{lock}");
    assert_eq!(lock["plugins"][0]["status"], "installed", "{lock}");

    env.set_list("");
    env.clear_log();
    terminal_update(&env);
    env.run(&["update", "--yes", "--project"]).code(0);
    assert!(
        !plugin_lines(&env.log())
            .iter()
            .any(|l| l.starts_with("plugin add")),
        "{}",
        env.log()
    );
    // `update` does not install it, so `doctor` names the `add` command.
    let out = env.run(&["doctor", "--project"]).code(1);
    let text = stdout(&out);
    assert!(
        text.contains(&format!("upskill add {} oc --project", registry.display())),
        "{text}"
    );
}

/// With one unrestricted bundle of the source, no kind restriction applies,
/// and the repair runs.
#[test]
fn terminal_repair_runs_when_one_bundle_of_the_source_is_unrestricted() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    let registry = env.bundle.parent().unwrap().to_path_buf();
    // Two bundles of one source cannot hold the same item.
    fs::create_dir_all(registry.join("beta")).unwrap();
    fs::write(
        registry.join("beta").join("SKILL.md"),
        "---\nschema: 1\nname: beta\ndescription: second skill for the issue 229 tests.\n---\n\n## Body\n\nText.\n",
    )
    .unwrap();
    fs::write(
        registry.join("two.bundle.yaml"),
        "schema: 1\nname: two\ndescription: second bundle of the source.\nitems:\n  skills:\n    - beta\n",
    )
    .unwrap();
    let source = registry.to_str().unwrap();
    env.add().success();
    env.run(&["add", source, "two", "--project"]).success();
    env.run(&["add", source, "oc", "--project", "--kind", "skill"])
        .success();
    let lock = env.lock();
    let kinds: Vec<usize> = lock["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["kinds"].as_array().map_or(0, Vec::len))
        .collect();
    assert_eq!(kinds.iter().filter(|n| **n == 0).count(), 1, "{lock}");
    assert_eq!(kinds.len(), 2, "{lock}");

    env.set_list("");
    env.clear_log();
    terminal_update(&env);
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
    // The warning shows the version found.
    assert!(err.contains("opencode v1.15.13"), "{err}");
    let lock = fs::read_to_string(env.project.join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("\"skipped\""), "{lock}");
    assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());

    // `doctor` lists the skipped plugin; it checks no skipped plugin with a
    // command. `update` and `remove` are covered by
    // `update_yes_keeps_a_skipped_plugin_skipped_when_opencode_is_1x` and by
    // `remove_runs_no_opencode_command`.
    let out = env.run(&["doctor", "--project"]).code(0);
    assert!(stdout(&out).contains("never installed"), "{}", stdout(&out));
}

#[test]
fn terminal_repair_runs_no_plugin_command_when_opencode_is_1x() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add().success();
    let lock = fs::read_to_string(env.project.join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("\"installed\""), "{lock}");
    // The plugin is missing, and the user goes back to opencode 1.x.
    env.set_list("");
    env.set_version("opencode v1.15.13\n", 0);
    env.clear_log();

    // A terminal run applies no source here, so only the repair could run.
    let result = terminal_update(&env);
    assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());
    // The doctor check cannot run under 1.x, so the plugin is not selected
    // for the repair and the user is told nothing about it.
    assert!(!result.to_string().contains("superpowers"), "{result}");
    let lock = fs::read_to_string(env.project.join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("\"installed\""), "{lock}");
}

#[test]
fn doctor_does_not_report_a_module_plugin_missing_when_opencode_is_1x() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add().success();
    // The user goes back to opencode 1.x after the install.
    env.set_version("opencode v1.15.13\n", 0);
    env.clear_log();

    let out = env.run(&["doctor", "--project"]).code(0);
    assert!(!stdout(&out).contains("missing"), "{}", stdout(&out));
    assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());
}

/// Rename the fake `opencode` away so that no `opencode` is on `PATH`, and
/// put it back with [`restore_opencode`].
fn hide_opencode(env: &Env) {
    fs::rename(env.bin.join("opencode"), env.bin.join("opencode.hidden")).unwrap();
}

fn restore_opencode(env: &Env, version: &str) {
    fs::rename(env.bin.join("opencode.hidden"), env.bin.join("opencode")).unwrap();
    env.set_version(&format!("opencode {version}\n"), 0);
}

#[test]
fn update_yes_installs_a_skipped_plugin_once_opencode_2x_is_on_path() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    hide_opencode(&env);
    env.add().success();
    let lock = fs::read_to_string(env.project.join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("\"skipped\""), "{lock}");

    restore_opencode(&env, "v2.1.4");
    env.set_list(&format!("{MODULE}\n"));
    env.run(&["update", "--yes", "--project"]).code(0);
    assert!(
        plugin_lines(&env.log()).contains(&format!("plugin add {MODULE}").as_str()),
        "{}",
        env.log()
    );
    let lock = fs::read_to_string(env.project.join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("\"installed\""), "{lock}");
    assert!(!lock.contains("\"skipped\""), "{lock}");

    let out = env.run(&["doctor", "--project"]).code(0);
    assert!(!stdout(&out).contains("skipped"), "{}", stdout(&out));
}

#[test]
fn update_yes_keeps_a_skipped_plugin_skipped_when_opencode_is_1x() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    hide_opencode(&env);
    env.add().success();

    restore_opencode(&env, "v1.15.13");
    env.clear_log();
    let out = env.run(&["update", "--yes", "--project"]).code(0);
    assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());
    // The warning of `add` is printed again.
    assert!(stderr(&out).contains("opencode 2"), "{}", stderr(&out));
    let lock = fs::read_to_string(env.project.join(".upskill-lock.json")).unwrap();
    assert!(lock.contains("\"skipped\""), "{lock}");
}

#[test]
fn terminal_repair_installs_a_skipped_plugin_and_records_it_installed() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    hide_opencode(&env);
    env.add().success();
    assert_eq!(env.lock()["plugins"][0]["status"], "skipped");

    restore_opencode(&env, "v2.1.4");
    env.clear_log();
    terminal_update(&env);
    assert!(
        plugin_lines(&env.log()).contains(&format!("plugin add {MODULE}").as_str()),
        "{}",
        env.log()
    );
    assert_eq!(env.lock()["plugins"][0]["status"], "installed");
}

#[test]
fn terminal_repair_keeps_a_skipped_plugin_skipped_when_opencode_is_1x() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    hide_opencode(&env);
    env.add().success();

    restore_opencode(&env, "v1.15.13");
    env.clear_log();
    let result = terminal_update(&env);
    assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());
    // The warning of `add` is printed again.
    assert!(result.to_string().contains("opencode 2"), "{result}");
    assert_eq!(env.lock()["plugins"][0]["status"], "skipped");
}

#[test]
fn terminal_repair_sets_config_write_on_a_plugin_uri_record_without_it() {
    let env = Env::new(&format!("plugin_uri: \"{URI}\""), "v2.1.4");
    env.add().success();
    // A record written before the field existed.
    let mut lock = env.lock();
    lock["plugins"][0]
        .as_object_mut()
        .unwrap()
        .remove("config_write");
    fs::write(
        env.project.join(".upskill-lock.json"),
        serde_json::to_string_pretty(&lock).unwrap(),
    )
    .unwrap();
    env.write_opencode_json(&serde_json::json!({ "plugins": [] }));

    terminal_update(&env);
    assert_eq!(env.lock()["plugins"][0]["config_write"], true);
    assert!(has_uri(&env.opencode_json()));
}

/// The plugin step of an applied source attempts every plugin of the
/// recorded bundles: every client, and a plugin that needs a manual step.
#[test]
fn update_yes_attempts_every_client_and_prints_instructions_again() {
    let env = Env::build(
        &format!(
            "  superpowers:\n    opencode:\n      module: {MODULE}\n    vscode:\n      extension: acme.sp\n"
        ),
        "  skills:\n    - alpha\n",
        "v2.1.4",
    );
    env.add().success();
    fs::write(env.bin.join("code-list.txt"), "acme.sp\n").unwrap();
    env.set_list(&format!("{MODULE}\n"));
    env.clear_log();
    let _ = fs::remove_file(env.bin.join("code-log.txt"));
    let out = env.run(&["update", "--yes", "--project"]).code(0);
    assert!(
        plugin_lines(&env.log()).contains(&format!("plugin add {MODULE}").as_str()),
        "{}",
        env.log()
    );
    assert!(
        env.code_log().contains("--install-extension acme.sp"),
        "{}",
        env.code_log()
    );
    assert!(
        stdout(&out).contains("Plugins: 2 plugin(s) installed"),
        "{}",
        stdout(&out)
    );

    let env = Env::new("instructions_url: https://example.com/sp", "v2.1.4");
    env.add().success();
    let out = env.run(&["update", "--yes", "--project"]).code(0);
    assert!(stderr(&out).contains("manual step"), "{}", stderr(&out));
}

#[test]
fn removed_plugin_uri_is_missing_and_restored_with_opencode_1x_or_absent() {
    for absent in [false, true] {
        let env = Env::new(&format!("plugin_uri: \"{URI}\""), "v2.1.4");
        env.add().success();
        if absent {
            hide_opencode(&env);
        } else {
            env.set_version("opencode v1.15.13\n", 0);
        }
        env.write_opencode_json(&serde_json::json!({ "plugins": [] }));
        env.clear_log();

        let out = env.run(&["doctor", "--project"]).code(1);
        assert!(stdout(&out).contains("missing"), "{}", stdout(&out));
        env.run(&["update", "--yes", "--project"]).code(0);
        assert!(has_uri(&env.opencode_json()), "{}", env.opencode_json());
        env.run(&["doctor", "--project"]).code(0);
        assert_eq!(env.log(), "", "absent: {absent}");
    }
}

#[test]
fn update_sets_config_write_on_a_plugin_uri_record_without_it() {
    // opencode 2 with an empty `plugin list`: a record without the field
    // falls back to the CLI check, which reports the plugin missing.
    let env = Env::new(&format!("plugin_uri: \"{URI}\""), "v2.1.4");
    env.add().success();
    // A record written before the field existed.
    let mut lock = env.lock();
    lock["plugins"][0]
        .as_object_mut()
        .unwrap()
        .remove("config_write");
    fs::write(
        env.project.join(".upskill-lock.json"),
        serde_json::to_string_pretty(&lock).unwrap(),
    )
    .unwrap();
    env.write_opencode_json(&serde_json::json!({ "plugins": [] }));

    env.run(&["update", "--yes", "--project"]).code(0);
    assert_eq!(env.lock()["plugins"][0]["config_write"], true);
    assert!(has_uri(&env.opencode_json()));
}

#[test]
fn remove_runs_no_opencode_command() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add().success();
    env.clear_log();
    env.run(&["remove", "alpha", "--project"]).success();
    assert_eq!(env.log(), "");
}

#[test]
fn version_that_fails_or_cannot_be_read_is_not_supported() {
    for (reply, exit) in [("opencode v2.1.4\n", 1), ("opencode dev build\n", 0)] {
        let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
        env.set_version(reply, exit);
        env.add().success();
        assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());
        assert_eq!(env.lock()["plugins"][0]["status"], "skipped");
    }
}

#[test]
fn terminal_update_repairs_a_missing_plugin_of_a_source_without_a_change() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add().success();
    env.set_list("");
    env.clear_log();

    let path = format!("{}:/usr/bin:/bin", env.bin.display());
    let result = common::update_in_terminal_with_env(
        &env.project,
        &env.home,
        &["--project"],
        "\u{4}",
        &[("PATH", path)],
    );
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        plugin_lines(&env.log()).contains(&format!("plugin add {MODULE}").as_str()),
        "{}",
        env.log()
    );
}

#[test]
fn repair_installs_only_the_missing_client_of_a_plugin() {
    let env = Env::build(
        &format!(
            "  superpowers:\n    opencode:\n      module: {MODULE}\n    vscode:\n      extension: acme.sp\n"
        ),
        "  skills:\n    - alpha\n",
        "v2.1.4",
    );
    env.add().success();
    fs::write(env.bin.join("code-list.txt"), "acme.sp\n").unwrap();
    env.set_list("");
    env.clear_log();
    let _ = fs::remove_file(env.bin.join("code-log.txt"));

    // A terminal run applies no source here: the repair alone installs.
    let result = terminal_update(&env);
    assert!(
        plugin_lines(&env.log()).contains(&format!("plugin add {MODULE}").as_str()),
        "{}",
        env.log()
    );
    assert!(
        !env.code_log().contains("--install-extension"),
        "{}",
        env.code_log()
    );
    // The output of a run whose only change is the repair.
    let text = result["stdout"].as_str().unwrap().to_string();
    assert!(text.contains("Plugins: 1 plugin(s) installed"), "{text}");
    assert!(text.contains("nothing to do"), "{text}");
}

#[test]
fn bundle_file_source_installs_a_plugin_once_per_update() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add_bundle_file().success();
    env.set_list("");
    env.clear_log();

    // The apply of the source installs the plugin; the repair does not
    // install it a second time.
    env.run(&["update", "--yes", "--project"]).code(0);
    let adds = plugin_lines(&env.log())
        .into_iter()
        .filter(|l| l.starts_with("plugin add"))
        .count();
    assert_eq!(adds, 1, "{}", env.log());
}

#[test]
fn terminal_update_repairs_a_plugin_of_a_bundle_file_source() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add_bundle_file().success();
    env.set_list("");
    env.clear_log();

    // A terminal run does not apply a source without a change, so only the
    // repair, through the bundle that the source resolved, installs it.
    let path = format!("{}:/usr/bin:/bin", env.bin.display());
    let result = common::update_in_terminal_with_env(
        &env.project,
        &env.home,
        &["--project"],
        "\u{4}",
        &[("PATH", path)],
    );
    assert_eq!(result["returncode"], 0, "{result}");
    assert!(
        plugin_lines(&env.log()).contains(&format!("plugin add {MODULE}").as_str()),
        "{}",
        env.log()
    );
}

#[test]
fn instructions_record_is_not_tried_again() {
    let env = Env::new("instructions_url: https://example.com/sp", "v2.1.4");
    env.add().success();
    assert_eq!(env.lock()["plugins"][0]["status"], "instructions");
    // A terminal run applies no source here: the repair does not try it.
    let result = terminal_update(&env);
    assert!(!result.to_string().contains("manual step"), "{result}");
}

#[test]
fn doctor_names_the_add_command_for_a_bundle_without_items() {
    let env = Env::build(
        &format!("  superpowers:\n    opencode:\n      module: {MODULE}\n"),
        "",
        "v2.1.4",
    );
    env.add().success();
    env.set_list("");

    let out = env.run(&["doctor", "--project"]).code(1);
    let text = stdout(&out);
    let registry = env.bundle.parent().unwrap().display().to_string();
    let command = format!("upskill add {registry} oc --project");
    assert!(text.contains(&command), "{text}");

    // The named command installs the plugin again.
    env.clear_log();
    env.add().success();
    assert!(
        plugin_lines(&env.log()).contains(&format!("plugin add {MODULE}").as_str()),
        "{}",
        env.log()
    );
}

#[test]
fn update_reports_a_still_skipped_plugin_of_a_bundle_file_source_once() {
    let env = Env::new(&format!("module: {MODULE}"), "v1.15.13");
    env.add_bundle_file().success();
    assert_eq!(env.lock()["plugins"][0]["status"], "skipped");
    env.clear_log();

    // The apply of the source attempts the plugin again; the repair does not.
    let out = env.run(&["update", "--yes", "--project"]).code(0);
    let err = stderr(&out);
    assert_eq!(
        err.matches("opencode 2 or later is needed").count(),
        1,
        "{err}"
    );
    assert!(plugin_lines(&env.log()).is_empty(), "{}", env.log());
}

#[test]
fn update_reports_a_plugin_installed_by_the_apply_once() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add_bundle_file().success();
    let out = env.run(&["update", "--yes", "--project"]).code(0);
    let text = stdout(&out);
    assert!(text.contains("Plugins: 1 plugin(s) installed"), "{text}");
    assert_eq!(text.matches("superpowers").count(), 1, "{text}");
}

#[test]
fn doctor_gives_no_add_command_when_update_reaches_the_plugin() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add().success();
    env.set_list("");
    let out = env.run(&["doctor", "--project"]).code(1);
    assert!(stdout(&out).contains("missing"), "{}", stdout(&out));
    assert!(!stdout(&out).contains("does not reach"), "{}", stdout(&out));
    let out = env.run(&["doctor", "--json", "--project"]).code(1);
    assert!(!stdout(&out).contains("repair_command"), "{}", stdout(&out));
}

#[test]
fn doctor_header_and_footer_of_plugins_name_the_condition() {
    let env = Env::build(
        &format!(
            "  superpowers:\n    opencode:\n      module: {MODULE}\n  other:\n    vscode:\n      extension: acme.other\n"
        ),
        "  skills:\n    - alpha\n",
        "v1.15.13",
    );
    env.add().success();
    // `code` lists nothing: the vscode plugin is missing; opencode is skipped.
    let out = env.run(&["doctor", "--project"]).code(1);
    let text = stdout(&out);
    assert!(
        text.contains(
            "`upskill update --yes` installs them, unless a line below names another command"
        ),
        "{text}"
    );
    assert!(
        text.contains("to install the plugins skipped for a missing CLI or an opencode version below 2, unless a line above names another command; a plugin that needs a manual step keeps its instructions"),
        "{text}"
    );
    // The skipped plugin is reached by `update`: no `repair_command` key.
    let out = env.run(&["doctor", "--json", "--project"]).code(1);
    assert!(!stdout(&out).contains("repair_command"), "{}", stdout(&out));
}

#[test]
fn doctor_gives_the_add_command_when_only_another_source_has_items() {
    let env = Env::build(
        &format!("  superpowers:\n    opencode:\n      module: {MODULE}\n"),
        "",
        "v2.1.4",
    );
    env.add().success();
    // Another skill source installed in the project.
    let other = env.project.parent().unwrap().join("other");
    let skill = other.join("beta").join("SKILL.md");
    fs::create_dir_all(skill.parent().unwrap()).unwrap();
    fs::write(
        &skill,
        "---\nschema: 1\nname: beta\ndescription: another skill.\n---\n\nText.\n",
    )
    .unwrap();
    env.run(&["add", other.to_str().unwrap(), "--project"])
        .success();
    env.set_list("");

    let out = env.run(&["doctor", "--project"]).code(1);
    let registry = env.bundle.parent().unwrap().display().to_string();
    let command = format!("upskill add {registry} oc --project");
    assert!(stdout(&out).contains(&command), "{}", stdout(&out));
}

#[test]
fn doctor_gives_the_add_command_for_a_skipped_plugin_update_does_not_reach() {
    let env = Env::build(
        &format!("  superpowers:\n    opencode:\n      module: {MODULE}\n"),
        "",
        "v2.1.4",
    );
    hide_opencode(&env);
    env.add().success();
    let registry = env.bundle.parent().unwrap().display().to_string();
    let command = format!("upskill add {registry} oc --project");

    let out = env.run(&["doctor", "--project"]).code(0);
    let text = stdout(&out);
    assert!(text.contains("never installed"), "{text}");
    assert!(text.contains(&command), "{text}");
    let out = env.run(&["doctor", "--json", "--project"]).code(0);
    let json: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let text = json.to_string();
    assert!(
        text.contains(&format!("\"repair_command\":\"{command}\"")),
        "{text}"
    );
}

#[test]
fn module_plugin_record_has_no_config_write_key() {
    let env = Env::new(&format!("module: {MODULE}"), "v2.1.4");
    env.add().success();
    let plugin = &env.lock()["plugins"][0];
    assert!(plugin.get("config_write").is_none(), "{plugin}");
}

// ---------------------------------------------------------------------------
// opencode config file per scope (issues #565 and #573)
// ---------------------------------------------------------------------------

const MCP_NAME: &str = "srv";

impl Env {
    /// A bundle with the plugin URI, one skill, one rule and one local MCP
    /// server, so that one `add` exercises every writer of `opencode.json`.
    fn with_rule_and_mcp() -> Self {
        let env = Env::build(
            &format!("  superpowers:\n    opencode:\n      plugin_uri: \"{URI}\"\n"),
            "  skills:\n    - alpha\n  rules:\n    - rule-a\n",
            "v2.1.4",
        );
        let registry = env.bundle.parent().unwrap();
        let rule = registry.join("rule-a").join("RULE.md");
        fs::create_dir_all(rule.parent().unwrap()).unwrap();
        fs::write(
            &rule,
            "---\nschema: 1\nname: rule-a\ndescription: rule for the opencode config tests.\n---\n\n## Body\n\nText.\n",
        )
        .unwrap();
        let mut bundle = fs::read_to_string(&env.bundle).unwrap();
        bundle.push_str(&format!(
            "mcps:\n  {MCP_NAME}:\n    local:\n      command: npx\n      args:\n        - srv-mcp\n"
        ));
        fs::write(&env.bundle, bundle).unwrap();
        env
    }

    fn global_config(&self) -> PathBuf {
        self.home.join(".config/opencode/opencode.json")
    }

    /// `args` with `--global`, run in a directory that is not a project.
    fn run_global(
        &self,
        args: &[&str],
        xdg: Option<&std::path::Path>,
    ) -> assert_cmd::assert::Assert {
        let path = format!("{}:/usr/bin:/bin", self.bin.display());
        let mut cmd = common::upskill_cmd(&self.home);
        cmd.current_dir(self.home.parent().unwrap())
            .env("PATH", path)
            .args(args)
            .arg("--global");
        if let Some(xdg) = xdg {
            cmd.env("XDG_CONFIG_HOME", xdg);
        }
        cmd.assert()
    }

    fn add_global(&self, xdg: Option<&std::path::Path>) -> assert_cmd::assert::Assert {
        let registry = self.bundle.parent().unwrap().to_str().unwrap().to_string();
        self.run_global(&["add", &registry, "oc"], xdg)
    }
}

fn read_json(path: &std::path::Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn global_add_writes_plugin_and_mcp_to_the_xdg_default_file_only() {
    let env = Env::with_rule_and_mcp();
    env.add_global(None).success();

    let config = read_json(&env.global_config());
    assert_eq!(config["plugins"], serde_json::json!([URI]), "{config}");
    assert!(config["mcp"][MCP_NAME].is_object(), "{config}");
    assert!(
        !env.home.join("opencode.json").exists(),
        "nothing is written to $HOME/opencode.json"
    );
}

#[test]
fn global_add_writes_no_instructions_entry_for_a_rule() {
    let env = Env::with_rule_and_mcp();
    env.add_global(None).success();

    let config = read_json(&env.global_config());
    assert!(config.get("instructions").is_none(), "{config}");
    assert!(!env.home.join("opencode.json").exists());
}

#[test]
fn project_add_writes_the_plugins_key_and_the_rules_entry_at_the_project_root() {
    let env = Env::with_rule_and_mcp();
    env.add().success();

    let config = env.opencode_json();
    assert_eq!(config["plugins"], serde_json::json!([URI]), "{config}");
    assert!(config.get("plugin").is_none(), "{config}");
    assert!(config["mcp"][MCP_NAME].is_object(), "{config}");
    assert_eq!(
        config["instructions"],
        serde_json::json!([".agents/rules/**/RULE.md"]),
        "{config}"
    );
    assert!(!env.global_config().exists());
}

#[test]
fn global_add_follows_an_absolute_xdg_config_home() {
    let env = Env::with_rule_and_mcp();
    let xdg = env.home.parent().unwrap().join("xdg");
    env.add_global(Some(&xdg)).success();

    let config = read_json(&xdg.join("opencode/opencode.json"));
    assert_eq!(config["plugins"], serde_json::json!([URI]), "{config}");
    assert!(config["mcp"][MCP_NAME].is_object(), "{config}");
    assert!(!env.global_config().exists());
    assert!(!env.home.join("opencode.json").exists());
}

#[test]
fn global_add_ignores_a_relative_xdg_config_home() {
    let env = Env::with_rule_and_mcp();
    env.add_global(Some(std::path::Path::new("relative-xdg")))
        .success();

    assert!(env.global_config().exists());
}

#[test]
fn global_doctor_reads_the_new_file_and_update_restores_the_plugin_there() {
    let env = Env::with_rule_and_mcp();
    env.add_global(None).success();
    env.run_global(&["doctor"], None).code(0);

    // Drop the plugin and the MCP entry from the file that opencode reads.
    let mut config = read_json(&env.global_config());
    config["plugins"] = serde_json::json!([]);
    config["mcp"] = serde_json::json!({});
    fs::write(env.global_config(), config.to_string()).unwrap();

    let out = env.run_global(&["doctor"], None).code(1);
    let text = stdout(&out);
    assert!(text.contains("superpowers"), "{text}");
    assert!(text.contains(MCP_NAME), "{text}");

    env.run_global(&["update", "--yes"], None).code(0);
    let config = read_json(&env.global_config());
    assert_eq!(config["plugins"], serde_json::json!([URI]), "{config}");
    assert!(!env.home.join("opencode.json").exists());
    env.run_global(&["doctor"], None).code(0);
}

#[test]
fn global_remove_mcp_edits_the_new_file() {
    let env = Env::with_rule_and_mcp();
    env.add_global(None).success();
    env.run_global(&["remove-mcp", MCP_NAME], None).success();

    let config = read_json(&env.global_config());
    assert!(config["mcp"].get(MCP_NAME).is_none(), "{config}");
    assert!(!env.home.join("opencode.json").exists());
}

#[test]
fn global_remove_deletes_the_plugin_from_the_new_file() {
    let env = Env::with_rule_and_mcp();
    env.add_global(None).success();
    env.run_global(&["remove", "alpha"], None).success();

    let config = read_json(&env.global_config());
    assert_eq!(config["plugins"], serde_json::json!([]), "{config}");
}

#[test]
fn doctor_accepts_a_legacy_plugin_entry() {
    let env = Env::new(&format!("plugin_uri: \"{URI}\""), "v2.1.4");
    env.add().success();
    env.write_opencode_json(&serde_json::json!({ "plugin": [URI] }));

    let out = env.run(&["doctor", "--project"]).code(0);
    assert!(!stdout(&out).contains("missing"), "{}", stdout(&out));
}

#[test]
fn remove_deletes_the_plugin_from_both_keys() {
    let env = Env::new(&format!("plugin_uri: \"{URI}\""), "v2.1.4");
    env.add().success();
    env.write_opencode_json(&serde_json::json!({ "plugin": [URI, "x"], "plugins": [URI] }));

    env.run(&["remove", "alpha", "--project"]).success();

    let config = env.opencode_json();
    assert_eq!(config["plugin"], serde_json::json!(["x"]), "{config}");
    assert_eq!(config["plugins"], serde_json::json!([]), "{config}");
}
