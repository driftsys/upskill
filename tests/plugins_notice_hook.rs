//! Pins the behaviour of the session-start hook command in the two
//! session-notice plugins (Claude Code and Copilot CLI).
//!
//! The tests read the hooks file of each plugin, take the command string from
//! the documented position, and run it with `sh -c`. A fake `upskill`
//! executable on `PATH` stands in for the real binary.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

#[derive(Clone, Copy)]
enum Plugin {
    Claude,
    Copilot,
}

const PLUGINS: [Plugin; 2] = [Plugin::Claude, Plugin::Copilot];

/// The request that the Copilot plugin puts before the notice. Copilot CLI
/// gives the hook output to the model only, so the text asks the model to
/// pass the notice on to the user.
const COPILOT_REQUEST: &str = "Tell the user at the start of your first reply: ";

impl Plugin {
    fn dir(self) -> PathBuf {
        let name = match self {
            Plugin::Claude => "upskill-claude",
            Plugin::Copilot => "upskill-copilot",
        };
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins")
            .join(name)
    }

    fn hooks_path(self) -> PathBuf {
        match self {
            Plugin::Claude => self.dir().join("hooks/hooks.json"),
            Plugin::Copilot => self.dir().join("hooks.json"),
        }
    }

    fn manifest_path(self) -> PathBuf {
        match self {
            Plugin::Claude => self.dir().join(".claude-plugin/plugin.json"),
            Plugin::Copilot => self.dir().join("plugin.json"),
        }
    }

    fn hooks(self) -> Value {
        let text = fs::read_to_string(self.hooks_path()).expect("hooks file is readable");
        serde_json::from_str(&text).expect("hooks file is valid JSON")
    }

    /// The hook entry object at the documented position.
    fn entry(self) -> Value {
        let hooks = self.hooks();
        match self {
            Plugin::Claude => {
                let groups = &hooks["hooks"]["SessionStart"];
                assert_eq!(groups.as_array().unwrap().len(), 1);
                assert_eq!(groups[0]["matcher"], "startup");
                assert_eq!(groups[0]["hooks"].as_array().unwrap().len(), 1);
                groups[0]["hooks"][0].clone()
            }
            Plugin::Copilot => {
                let entries = &hooks["hooks"]["sessionStart"];
                assert_eq!(entries.as_array().unwrap().len(), 1);
                entries[0].clone()
            }
        }
    }

    fn command(self) -> String {
        let key = match self {
            Plugin::Claude => "command",
            Plugin::Copilot => "bash",
        };
        self.entry()[key]
            .as_str()
            .expect("command is a string")
            .to_string()
    }

    /// The JSON fields that must each hold the notice text.
    fn text_fields(self, out: &Value) -> Vec<String> {
        let get = |v: &Value| v.as_str().expect("text field is a string").to_string();
        match self {
            Plugin::Claude => {
                assert_eq!(out["hookSpecificOutput"]["hookEventName"], "SessionStart");
                assert_eq!(out.as_object().unwrap().len(), 2);
                let mut keys: Vec<&String> = out["hookSpecificOutput"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .collect();
                keys.sort();
                assert_eq!(keys, ["additionalContext", "hookEventName"]);
                vec![
                    get(&out["systemMessage"]),
                    get(&out["hookSpecificOutput"]["additionalContext"]),
                ]
            }
            Plugin::Copilot => {
                assert_eq!(out.as_object().unwrap().len(), 1);
                let text = get(&out["additionalContext"]);
                let notice = text
                    .strip_prefix(COPILOT_REQUEST)
                    .expect("the context text starts with the request to the model");
                vec![notice.to_string()]
            }
        }
    }
}

/// Runs the plugin command with a fake `upskill` whose body is `script`.
/// Returns the output and the recorded arguments of the fake, as text.
fn run_with_fake(plugin: Plugin, script: &str) -> (Output, String) {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let args_file = tmp.path().join("args");
    let fake = bin.join("upskill");
    fs::write(
        &fake,
        format!(
            "#!/bin/sh\nprintf '%s ' \"$@\" > '{}'\n{}\n",
            args_file.display(),
            script
        ),
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:/usr/bin:/bin", bin.display());
    let out = run(plugin, &path, tmp.path());
    let args = fs::read_to_string(&args_file).unwrap_or_default();
    (out, args)
}

fn run(plugin: Plugin, path: &str, cwd: &Path) -> Output {
    Command::new("sh")
        .arg("-c")
        .arg(plugin.command())
        .env_clear()
        .env("PATH", path)
        .current_dir(cwd)
        .output()
        .expect("sh runs")
}

fn assert_silent_success(out: &Output) {
    assert!(out.status.success(), "exit status: {:?}", out.status);
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

/// Checks one JSON line on stdout and returns the decoded text fields.
fn parse_notice(plugin: Plugin, out: &Output) -> Vec<String> {
    assert!(out.status.success(), "exit status: {:?}", out.status);
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert!(stdout.ends_with('\n'));
    assert_eq!(stdout.trim_end_matches('\n').lines().count(), 1);
    let value: Value = serde_json::from_str(&stdout).expect("stdout is valid JSON");
    plugin.text_fields(&value)
}

#[test]
fn one_notice_line_becomes_one_json_line() {
    for plugin in PLUGINS {
        let line = "upskill: 3 updates available (project). Run \"upskill update\".";
        let (out, _) = run_with_fake(plugin, &format!("printf '%s\\n' '{line}'"));
        let fields = parse_notice(plugin, &out);
        assert!(!fields.is_empty());
        for f in fields {
            assert_eq!(f, line);
        }
    }
}

#[test]
fn two_notice_lines_are_joined_by_one_newline() {
    for plugin in PLUGINS {
        let a = "upskill: 3 updates available (project). Run \"upskill update\".";
        let b = "upskill: 1 update available (global). Run \"upskill update --global\".";
        let script = format!("printf '%s\\n%s\\n' '{a}' '{b}'");
        let (out, _) = run_with_fake(plugin, &script);
        for f in parse_notice(plugin, &out) {
            assert_eq!(f, format!("{a}\n{b}"));
        }
    }
}

#[test]
fn backslash_and_quote_survive_encoding() {
    for plugin in PLUGINS {
        let line = r#"path C:\dir "quoted" \n end"#;
        let (out, _) = run_with_fake(plugin, &format!("printf '%s\\n' '{line}'"));
        for f in parse_notice(plugin, &out) {
            assert_eq!(f, line);
        }
    }
}

#[test]
fn empty_notice_prints_nothing() {
    for plugin in PLUGINS {
        let (out, _) = run_with_fake(plugin, "exit 0");
        assert_silent_success(&out);
    }
}

#[test]
fn failing_upskill_prints_nothing() {
    for plugin in PLUGINS {
        let (out, _) = run_with_fake(plugin, "echo 'upskill: notice'; echo oops >&2; exit 3");
        assert_silent_success(&out);
    }
}

#[test]
fn missing_upskill_prints_nothing() {
    for plugin in PLUGINS {
        let tmp = tempfile::tempdir().unwrap();
        let path = format!("{}:/usr/bin:/bin", tmp.path().display());
        let out = run(plugin, &path, tmp.path());
        assert_silent_success(&out);
    }
}

#[test]
fn stderr_of_upskill_is_not_forwarded() {
    for plugin in PLUGINS {
        let (out, _) = run_with_fake(plugin, "echo 'upskill: notice'; echo oops >&2");
        parse_notice(plugin, &out);
    }
}

#[test]
fn upskill_is_called_with_update_notice() {
    for plugin in PLUGINS {
        let (_, args) = run_with_fake(plugin, "echo 'upskill: notice'");
        assert_eq!(args.trim_end(), "update --notice");
    }
}

#[test]
fn static_shape_of_hooks_and_manifests() {
    let claude = Plugin::Claude.entry();
    assert_eq!(claude["type"], "command");
    assert_eq!(claude["timeout"], 30);

    let copilot_hooks = Plugin::Copilot.hooks();
    assert_eq!(copilot_hooks["version"], 1);
    let copilot = Plugin::Copilot.entry();
    assert_eq!(copilot["type"], "command");
    assert_eq!(copilot["timeoutSec"], 30);

    for plugin in PLUGINS {
        let text = fs::read_to_string(plugin.manifest_path()).unwrap();
        let manifest: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(manifest["name"], "upskill");
        assert!(manifest.get("version").is_none());
    }
}

#[test]
fn each_plugin_holds_exactly_two_files() {
    for plugin in PLUGINS {
        let mut files = Vec::new();
        collect_files(&plugin.dir(), &plugin.dir(), &mut files);
        files.sort();
        let mut expected = vec![
            plugin
                .hooks_path()
                .strip_prefix(plugin.dir())
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            plugin
                .manifest_path()
                .strip_prefix(plugin.dir())
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        ];
        expected.sort();
        assert_eq!(files, expected);
    }
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("plugin directory is readable") {
        let path = entry.unwrap().path();
        if path.file_name().is_some_and(|n| n == ".DS_Store") {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, out);
        } else {
            out.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
}

/// Runs the Copilot command with a fake `upskill` that records its working
/// directory. `run_dir` is the current directory of the command and `project`
/// is the value of `COPILOT_PROJECT_DIR`, if any. Returns the output and the
/// recorded directory, or `None` when the fake was not called.
fn run_copilot_in_dirs(run_dir: &Path, project: Option<&Path>) -> (Output, Option<PathBuf>) {
    run_copilot_with_env(run_dir, project, &[])
}

/// Like `run_copilot_in_dirs`, with extra environment variables.
fn run_copilot_with_env(
    run_dir: &Path,
    project: Option<&Path>,
    extra_env: &[(&str, &Path)],
) -> (Output, Option<PathBuf>) {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let cwd_file = tmp.path().join("cwd");
    let fake = bin.join("upskill");
    fs::write(
        &fake,
        format!(
            "#!/bin/sh\npwd -P > '{}'\necho 'upskill: notice'\n",
            cwd_file.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(Plugin::Copilot.command())
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .current_dir(run_dir);
    if let Some(p) = project {
        cmd.env("COPILOT_PROJECT_DIR", p);
    }
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let out = cmd.output().expect("sh runs");
    let recorded = fs::read_to_string(&cwd_file)
        .ok()
        .map(|s| PathBuf::from(s.trim_end()));
    (out, recorded)
}

#[test]
fn copilot_hook_changes_to_the_project_directory() {
    let run_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let (out, recorded) = run_copilot_in_dirs(run_dir.path(), Some(project.path()));
    assert!(out.status.success());
    assert_eq!(
        recorded.unwrap().canonicalize().unwrap(),
        project.path().canonicalize().unwrap()
    );
}

#[test]
fn copilot_hook_stays_in_place_without_project_directory() {
    let run_dir = tempfile::tempdir().unwrap();
    let (out, recorded) = run_copilot_in_dirs(run_dir.path(), None);
    assert!(out.status.success());
    assert_eq!(
        recorded.unwrap().canonicalize().unwrap(),
        run_dir.path().canonicalize().unwrap()
    );
}

#[test]
fn copilot_hook_stays_in_place_with_empty_project_directory() {
    let run_dir = tempfile::tempdir().unwrap();
    let (out, recorded) = run_copilot_in_dirs(run_dir.path(), Some(Path::new("")));
    assert!(out.status.success());
    assert_eq!(
        recorded.unwrap().canonicalize().unwrap(),
        run_dir.path().canonicalize().unwrap()
    );
}

#[test]
fn copilot_hook_prints_nothing_when_project_directory_is_missing() {
    let run_dir = tempfile::tempdir().unwrap();
    let missing = run_dir.path().join("does-not-exist");
    let (out, recorded) = run_copilot_in_dirs(run_dir.path(), Some(&missing));
    assert_silent_success(&out);
    assert!(recorded.is_none(), "upskill must not be called");
}

#[test]
fn copilot_hook_enters_a_relative_directory_that_starts_with_a_hyphen() {
    let run_dir = tempfile::tempdir().unwrap();
    let target = run_dir.path().join("-x");
    fs::create_dir(&target).unwrap();
    let (out, recorded) = run_copilot_in_dirs(run_dir.path(), Some(Path::new("-x")));
    assert!(out.status.success());
    assert_eq!(
        recorded
            .expect("upskill must be called")
            .canonicalize()
            .unwrap(),
        target.canonicalize().unwrap()
    );
}

#[test]
fn copilot_hook_enters_a_directory_whose_name_holds_a_space() {
    let run_dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let target = project.path().join("a b");
    fs::create_dir(&target).unwrap();
    let (out, recorded) = run_copilot_in_dirs(run_dir.path(), Some(&target));
    assert!(out.status.success());
    assert_eq!(
        recorded
            .expect("upskill must be called")
            .canonicalize()
            .unwrap(),
        target.canonicalize().unwrap()
    );
}

#[test]
fn copilot_hook_prints_only_json_when_cdpath_resolves_the_directory() {
    // With CDPATH set, `cd` prints the new directory on stdout. That text
    // must not reach the output of the hook.
    let base = tempfile::tempdir().unwrap();
    fs::create_dir(base.path().join("proj")).unwrap();
    let run_dir = tempfile::tempdir().unwrap();
    let (out, recorded) = run_copilot_with_env(
        run_dir.path(),
        Some(Path::new("proj")),
        &[("CDPATH", base.path())],
    );
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "{\"additionalContext\":\"Tell the user at the start of your first reply: upskill: notice\"}\n"
    );
    assert_eq!(
        recorded
            .expect("upskill must be called")
            .canonicalize()
            .unwrap(),
        base.path().join("proj").canonicalize().unwrap()
    );
}

#[test]
fn control_characters_in_the_notice_keep_the_output_valid_json() {
    // Every control character from \001 to \037 except newline (\012), one
    // per line, between two letters.
    let codes: Vec<u8> = (1..=0o37).filter(|c| *c != 0o12).collect();
    let script = codes
        .iter()
        .map(|c| format!("a\\{c:03o}b\\n"))
        .collect::<String>();
    let expected = vec!["a b"; codes.len()].join("\n");
    for plugin in PLUGINS {
        let (out, _) = run_with_fake(plugin, &format!("printf '{script}'"));
        for f in parse_notice(plugin, &out) {
            assert_eq!(f, expected);
        }
    }
}

#[test]
fn plugins_directory_holds_exactly_the_two_plugin_directories() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins");
    let mut names: Vec<String> = fs::read_dir(root)
        .expect("plugins directory is readable")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n != ".DS_Store")
        .collect();
    names.sort();
    assert_eq!(names, ["upskill-claude", "upskill-copilot"]);
}

#[test]
fn hook_runs_upskill_in_the_directory_it_was_started_in() {
    for plugin in PLUGINS {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("bin");
        let start = tmp.path().join("start");
        fs::create_dir(&bin).unwrap();
        fs::create_dir(&start).unwrap();
        let cwd_file = tmp.path().join("cwd");
        let fake = bin.join("upskill");
        fs::write(
            &fake,
            format!(
                "#!/bin/sh\npwd -P > '{}'\necho 'upskill: notice'\n",
                cwd_file.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
        let path = format!("{}:/usr/bin:/bin", bin.display());
        let out = run(plugin, &path, &start);
        assert!(out.status.success());
        let recorded = fs::read_to_string(&cwd_file).expect("upskill must be called");
        assert_eq!(
            PathBuf::from(recorded.trim_end()),
            start.canonicalize().unwrap()
        );
    }
}
