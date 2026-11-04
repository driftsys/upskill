//! Bounded collector acceptance uses fake Copilot assets and an isolated Git repo.
#![cfg(any(target_os = "linux", target_os = "macos"))]
use fixture::fixture_executable as executable;
#[path = "common/copilot_probe.rs"]
mod fixture;
use fixture::{Fixture, digest, private};
use serde_json::{Value, json};
use std::fs;
use std::sync::atomic::AtomicBool;

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
fn prepared(mode: &str) -> Fixture {
    let mut fixture = Fixture::new();
    let control = fixture.parent.join("control");
    fs::create_dir(&control).unwrap();
    private(&control.join("mode"), mode.as_bytes(), 0o600);
    private(
        &control.join("version"),
        include_bytes!("fixtures/native-adapters/2026-11-01-help/copilot-version.stdout"),
        0o600,
    );
    let plan: Value =
        serde_json::from_slice(&fs::read(fixture.root.join("plan.json")).unwrap()).unwrap();
    let name = plan["native_name"].as_str().unwrap();
    let version = plan["version"].as_str().unwrap();
    let catalog = plan["catalog"].as_str().unwrap();
    for (file, value) in [
        ("empty", json!([])),
        (
            "plugins",
            json!([{"name":name,"marketplace":name,"source":catalog,"version":version,"enabled":true}]),
        ),
        (
            "markets",
            json!([{"name":name,"source":catalog,"isDefault":false}]),
        ),
    ] {
        private(
            &control.join(file),
            &serde_json::to_vec(&value).unwrap(),
            0o600,
        );
    }
    let script = format!(
        r#"#!/bin/sh
control={}
n=0
if [ -f "$control/count" ]; then n=$(/bin/cat "$control/count"); fi
seq=$((n+1))
step=$(printf '%02d' "$seq")
previous=$(printf '%02d' "$n")
area="$HOME/../evidence"
if [ ! -f "$area/step-$step.pending.json" ] || [ ! -f "$area/step-$previous.result.json" ]; then
  printf 'missing durable predecessor' >&2; exit 93
fi
printf '%s\n' "$#" "$@" > "$control/call-$n"
printf '%s' "$((n+1))" > "$control/count"
/bin/pwd -P > "$control/cwd-$n"
printf '%s\n' "$HOME" "$COPILOT_HOME" "$COPILOT_CACHE_HOME" "$GIT_CONFIG_GLOBAL" "$GIT_CONFIG_SYSTEM" > "$control/env-$n"
if [ -n "$GITHUB_TOKEN$GH_TOKEN$NODE_OPTIONS$COPILOT_MODEL$GIT_CONFIG_PARAMETERS" ]; then printf 'inherited environment' >&2; exit 94; fi
mode=$(/bin/cat "$control/mode")
if [ "$mode" = closed-env ]; then /usr/bin/env > "$control/all-env-$n"; fi
if [ "$n" -eq 0 ]; then
  if [ "$mode" = version ]; then printf 'unsupported'; else /bin/cat "$control/version"; fi
  exit 0
fi
if [ "$n" -eq 1 ] && [ "$mode" = marketplace-only ]; then /bin/cat "$control/baseline-markets"; exit 0; fi
if [ "$n" -eq 1 ]; then
  case "$mode" in
    unknown) printf '{{}}'; exit 0;;
    duplicate) printf '[{{"name":"a","name":"b"}}]'; exit 0;;
    overflow) while :; do printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'; done;;
    timeout|cancel) printf ready > "$control/ready"; while :; do /bin/sleep 0.01; done;;
  esac
fi
if [ "$n" -eq 3 ]; then
  case "$mode" in
    stderr) printf 'secret-canary' >&2; exit 0;;
    diagnostic) printf 'secret-canary'; exit 0;;
    refuse) exit 17;;
    input-drift) printf changed > "$HOME/manual-preservation.txt";;
    code-drift) printf '#!/bin/sh\nexit 0\n' > "$0";;
    native-config) printf 'secret-canary' > "$HOME/.copilot/config.json";;
    metadata-symlink) /bin/ln -s "$control/missing-secret" "$HOME/.copilot/config.json";;
    evidence-drift) printf changed > "$area/header.json";;
    result-collision) printf 'manual-sentinel' > "$area/step-04.result.json";;
    root-drift) /bin/mv "$HOME/../project" "$control/moved-project"; /bin/mkdir "$HOME/../project";;
  esac
fi
case "$n" in
  1|9) if [ -f "$control/baseline-plugins" ]; then /bin/cat "$control/baseline-plugins"; else /bin/cat "$control/empty"; fi;;
  2|10) if [ -f "$control/baseline-markets" ]; then /bin/cat "$control/baseline-markets"; else /bin/cat "$control/empty"; fi;;
  5) /bin/cat "$control/plugins";;
  6) /bin/cat "$control/markets";;
esac
"#,
        shell_quote(control.to_str().unwrap())
    );
    let path = fixture.parent.join("assets/npm-loader.js");
    private(&path, script.as_bytes(), 0o700);
    fixture.edit_preparation(|receipt| {
        receipt["launcher"]["sha256"] = json!(digest(script.as_bytes()));
        receipt["launcher"]["bytes"] = json!(script.len());
    });
    fixture
}
fn json_file(path: impl AsRef<std::path::Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn assert_no_secret(area: &std::path::Path) {
    for entry in fs::read_dir(area).unwrap() {
        let bytes = fs::read(entry.unwrap().path()).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("secret-canary"));
    }
}
#[test]
fn collector_orders_all_commands_after_durable_pending_and_preserves_exact_argv() {
    let fixture = prepared("success");
    let preflight = fixture.capture().unwrap();
    let mut evidence = preflight.begin_evidence().unwrap();
    let summary = evidence.collect(&AtomicBool::new(false)).unwrap();
    let summary = serde_json::to_value(summary).unwrap();
    assert_eq!(summary["complete"], true);
    assert_eq!(summary["client_commands"], 11);
    assert_eq!(summary["setup_commands"], 1);
    assert_eq!(summary["model_requests"], 0);
    assert_eq!(summary["qualified"], false);
    assert!(fixture.root.join("project/.git").is_dir());
    let plan = json_file(fixture.root.join("plan.json"));
    let area = fixture.root.join("evidence");
    let header = json_file(area.join("header.json"));
    assert_eq!(header["maximum_native_commands"], 12);
    assert_eq!(header["maximum_client_commands"], 11);
    assert_eq!(header["maximum_setup_commands"], 1);
    assert_eq!(
        header["inventory_source_policy"],
        json!({"filesystem_paths":"packet_only","unrelated_builtin_label":"count_only","owned_builtin_label":"reject"})
    );
    let mut previous = digest(&fs::read(area.join("header.json")).unwrap());
    for sequence in 0..12 {
        let pending_path = area.join(format!("step-{sequence:02}.pending.json"));
        let result_path = area.join(format!("step-{sequence:02}.result.json"));
        let pending = json_file(&pending_path);
        let result = json_file(&result_path);
        assert_eq!(pending["previous_sha256"], previous);
        assert_eq!(pending["timeout_seconds"], 20);
        assert_eq!(pending["output_limit_bytes"], 1048576);
        assert_eq!(
            result["pending_sha256"],
            digest(&fs::read(pending_path).unwrap())
        );
        previous = digest(&fs::read(result_path).unwrap());
    }
    for index in 0..11 {
        let mut expected = vec![
            plan["commands"][index]
                .as_array()
                .unwrap()
                .len()
                .to_string(),
        ];
        expected.extend(
            plan["commands"][index]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned()),
        );
        assert_eq!(
            fs::read_to_string(fixture.parent.join(format!("control/call-{index}"))).unwrap(),
            expected.join("\n") + "\n"
        );
        assert_eq!(
            fs::read_to_string(fixture.parent.join(format!("control/cwd-{index}"))).unwrap(),
            format!("{}\n", fixture.root.join("project").display())
        );
        let seq = index + 1;
        let pending = json_file(area.join(format!("step-{seq:02}.pending.json")));
        let result = json_file(area.join(format!("step-{seq:02}.result.json")));
        assert_eq!(pending["arguments"], plan["commands"][index]);
        assert_eq!(pending["sequence"], seq);
        assert_eq!(result["sequence"], seq);
        assert_eq!(result["accepted"], true);
        assert_eq!(result.get("source_rejection_category"), Some(&Value::Null));
        assert_eq!(result["qualified"], false);
    }
    let setup = json_file(area.join("step-00.pending.json"));
    assert_eq!(
        setup["arguments"],
        json!([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "init.templateDir=",
            "init",
            "--quiet",
            "--template=",
            "."
        ])
    );
    for (sequence, path) in [
        (0, std::path::PathBuf::from("/usr/bin/git")),
        (1, fixture.parent.join("assets/copilot")),
    ] {
        use std::os::unix::fs::PermissionsExt;
        let pending = json_file(area.join(format!("step-{sequence:02}.pending.json")));
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            pending["executable"],
            json!({"selected":path,"resolved":path.canonicalize().unwrap(),
            "sha256":digest(&bytes),"bytes":bytes.len(),"mode":fs::metadata(&path).unwrap().permissions().mode() & 0o7777})
        );
    }
    assert!(!fixture.root.join("project/.git/hooks").exists());
    assert!(!fixture.root.join("project/.git/info/exclude").exists());
    assert_eq!(fs::read_dir(&area).unwrap().count(), 34);
    evidence.verify_unchanged().unwrap();
    assert!(evidence.collect(&AtomicBool::new(false)).is_err());
    assert_no_secret(&area);
}
#[test]
fn collector_failure_retains_observations_without_any_later_command_or_cleanup() {
    for (mode, calls) in [
        ("version", 1),
        ("unknown", 2),
        ("duplicate", 2),
        ("overflow", 2),
        ("stderr", 4),
        ("diagnostic", 4),
        ("refuse", 4),
        ("input-drift", 4),
        ("code-drift", 4),
        ("metadata-symlink", 4),
    ] {
        let fixture = prepared(mode);
        let preflight = fixture.capture().unwrap();
        let mut evidence = preflight.begin_evidence().unwrap();
        let result =
            serde_json::to_value(evidence.collect(&AtomicBool::new(false)).unwrap()).unwrap();
        assert_eq!(result["complete"], false, "{mode}");
        assert_eq!(result["client_commands"], calls, "{mode}");
        assert_eq!(
            fs::read_to_string(fixture.parent.join("control/count")).unwrap(),
            calls.to_string()
        );
        assert!(
            !fixture
                .parent
                .join(format!("control/call-{calls}"))
                .exists()
        );
        let terminal = json_file(
            fixture
                .root
                .join(format!("evidence/step-{calls:02}.result.json")),
        );
        assert_eq!(
            terminal.get("source_rejection_category"),
            Some(&Value::Null),
            "{mode}"
        );
        assert!(evidence.collect(&AtomicBool::new(false)).is_err());
        assert_no_secret(&fixture.root.join("evidence"));
    }
}

#[test]
fn collector_timeout_and_cancellation_stop_without_later_calls() {
    for mode in ["timeout", "cancel"] {
        let fixture = prepared(mode);
        let preflight = fixture.capture().unwrap();
        let mut evidence = preflight.begin_evidence().unwrap();
        let cancellation = AtomicBool::new(false);
        let done = AtomicBool::new(false);
        std::thread::scope(|threads| {
            let watcher = threads.spawn(|| {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
                while !fixture.parent.join("control/ready").exists()
                    && !done.load(std::sync::atomic::Ordering::SeqCst)
                    && std::time::Instant::now() < deadline
                {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                let ready = fixture.parent.join("control/ready").exists();
                let observed = std::time::Instant::now();
                if mode == "cancel" {
                    cancellation.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                (ready, observed)
            });
            let result = serde_json::to_value(evidence.collect(&cancellation).unwrap()).unwrap();
            done.store(true, std::sync::atomic::Ordering::SeqCst);
            let (ready, observed) = watcher.join().unwrap();
            assert!(ready, "client never reached the waiting control");
            let elapsed = observed.elapsed();
            if mode == "cancel" {
                assert!(
                    elapsed < std::time::Duration::from_secs(10),
                    "cancellation exceeded its latency bound: {elapsed:?}"
                );
            } else {
                assert!(
                    elapsed < std::time::Duration::from_secs(30),
                    "timeout exceeded its latency bound: {elapsed:?}"
                );
                assert!(
                    elapsed > std::time::Duration::from_secs(15),
                    "timeout fired before the command deadline: {elapsed:?}"
                );
            }
            assert_eq!(result["complete"], false);
            assert_eq!(result["client_commands"], 2);
            let terminal = json_file(fixture.root.join("evidence/step-02.result.json"));
            assert_eq!(
                terminal["transport"]["reason"],
                if mode == "cancel" {
                    "cancelled"
                } else {
                    "timed_out"
                }
            );
        });
        assert!(!fixture.parent.join("control/call-2").exists());
        assert_no_secret(&fixture.root.join("evidence"));
    }
}
#[test]
fn cancelled_collection_never_invokes_setup_or_client() {
    let fixture = prepared("success");
    let preflight = fixture.capture().unwrap();
    let mut evidence = preflight.begin_evidence().unwrap();
    let summary = serde_json::to_value(evidence.collect(&AtomicBool::new(true)).unwrap()).unwrap();
    assert_eq!(summary["client_commands"], 0);
    assert_eq!(summary["setup_commands"], 0);
    assert!(!fixture.root.join("project/.git").exists());
    assert!(!fixture.parent.join("control/count").exists());
}

#[test]
fn maintainer_example_check_is_read_only_and_execution_uses_closed_environment() {
    use std::process::{Command, Stdio};
    let program = executable::collector_example();
    let fixture = prepared("closed-env");
    let command = |mode: Option<&str>| {
        let mut command = Command::new(program);
        command
            .env_clear()
            .env("HOME", &fixture.parent)
            .env("USERPROFILE", &fixture.parent)
            .env("PATH", "/usr/bin:/bin")
            .env("GITHUB_TOKEN", "secret-canary")
            .env("GH_TOKEN", "secret-canary")
            .env("NODE_OPTIONS", "secret-canary")
            .env("COPILOT_MODEL", "secret-canary")
            .env("GIT_CONFIG_PARAMETERS", "secret-canary")
            .stdin(Stdio::null());
        if let Some(mode) = mode {
            command.arg(mode);
        }
        command
            .arg(&fixture.root)
            .arg(&fixture.request.plan_sha256)
            .arg(&fixture.request.preparation_sha256);
        command
    };
    for mode in [None, Some("--check")] {
        let output = assert_cmd::Command::from_std(command(mode))
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        assert_eq!(
            serde_json::from_slice::<Value>(&output).unwrap(),
            json!({"fixture_files":9,"executed":false,"qualified":false})
        );
        assert!(!fixture.root.join("evidence").exists());
        assert!(!fixture.root.join("project/.git").exists());
        assert!(!fixture.parent.join("control/count").exists());
    }
    let output = assert_cmd::Command::from_std(command(Some("--execute")))
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        serde_json::from_slice::<Value>(&output).unwrap()["complete"],
        true
    );
    for index in 0..11 {
        let environment =
            fs::read_to_string(fixture.parent.join(format!("control/env-{index}"))).unwrap();
        assert_eq!(
            environment,
            format!(
                "{}\n{}\n{}\n/dev/null\n/dev/null\n",
                fixture.root.join("home").display(),
                fixture.root.join("home/.copilot").display(),
                fixture.root.join("home/cache/copilot").display()
            )
        );
    }
    use std::collections::BTreeMap;
    let home = fixture.root.join("home");
    let mut expected = BTreeMap::new();
    for (key, suffix) in [
        ("HOME", ""),
        ("USERPROFILE", ""),
        ("COPILOT_HOME", ".copilot"),
        ("COPILOT_CACHE_HOME", "cache/copilot"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_STATE_HOME", "state"),
        ("TMPDIR", "tmp"),
    ] {
        expected.insert(
            key.to_owned(),
            (if suffix.is_empty() {
                home.clone()
            } else {
                home.join(suffix)
            })
            .display()
            .to_string(),
        );
    }
    for (key, value) in [
        ("LC_ALL", "C"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_SYSTEM", "/dev/null"),
        ("GIT_ATTR_NOSYSTEM", "1"),
    ] {
        expected.insert(key.to_owned(), value.to_owned());
    }
    expected.insert(
        "PATH".into(),
        format!(
            "{}:/usr/bin:/bin:/usr/sbin:/sbin",
            fixture.parent.join("assets").display()
        ),
    );
    for index in 0..11 {
        // Only this example test dumps the full environment: its parent has
        // already been cleared and contains deliberately supplied test values.
        let text =
            fs::read_to_string(fixture.parent.join(format!("control/all-env-{index}"))).unwrap();
        let mut actual = BTreeMap::new();
        for line in text.lines() {
            let (key, value) = line.split_once('=').unwrap();
            assert!(actual.insert(key.to_owned(), value.to_owned()).is_none());
        }
        assert_eq!(
            actual.remove("PWD"),
            Some(fixture.root.join("project").display().to_string())
        );
        // /bin/sh can add these variables after exec; they are not runner input.
        if let Some(value) = actual.remove("SHLVL") {
            assert_eq!(value, "1");
        }
        if let Some(value) = actual.remove("_") {
            assert_eq!(value, "/usr/bin/env");
        }
        assert_eq!(actual, expected, "command {index}");
    }
    assert_no_secret(&fixture.root.join("evidence"));
}

#[test]
fn conflicting_or_outside_fixture_claims_stop_before_any_cleanup() {
    for (mode, calls, reason) in [
        ("baseline", 2, "fixture_conflict"),
        ("version", 6, "fixture_conflict"),
        ("duplicate", 6, "fixture_conflict"),
        ("source", 7, "outside_packet_source"),
        ("marketplace-selector", 7, "fixture_conflict"),
        ("inside-source", 6, "fixture_conflict"),
        ("enabled", 6, "fixture_conflict"),
        ("marketplace", 6, "fixture_conflict"),
        ("name", 6, "fixture_conflict"),
    ] {
        let fixture = prepared("success");
        let control = fixture.parent.join("control");
        if mode == "baseline" {
            fs::copy(control.join("plugins"), control.join("empty")).unwrap();
        } else {
            let path = control.join(if matches!(mode, "source" | "marketplace-selector") {
                "markets"
            } else {
                "plugins"
            });
            let mut value = json_file(&path);
            match mode {
                "version" => value[0]["version"] = json!("wrong-version"),
                "duplicate" => {
                    let row = value[0].clone();
                    value.as_array_mut().unwrap().push(row);
                }
                "source" => value[0]["source"] = json!("/outside-packet"),
                "marketplace-selector" => {
                    let name = value[0]["name"].as_str().unwrap();
                    value[0]["name"] = json!(format!("{name}@{name}"));
                }
                "inside-source" => {
                    value[0]["source"] = json!(fixture.root.join("unapproved-source"))
                }
                "enabled" => value[0]["enabled"] = json!(false),
                "marketplace" => value[0]["marketplace"] = json!("wrong-marketplace"),
                "name" => value[0]["name"] = json!("wrong-plugin"),
                _ => unreachable!(),
            }
            private(&path, &serde_json::to_vec(&value).unwrap(), 0o600);
        }
        let preflight = fixture.capture().unwrap();
        let mut evidence = preflight.begin_evidence().unwrap();
        let summary =
            serde_json::to_value(evidence.collect(&AtomicBool::new(false)).unwrap()).unwrap();
        assert_eq!(summary["complete"], false, "{mode}");
        assert_eq!(summary["client_commands"], calls, "{mode}");
        assert_eq!(summary["stop"], reason, "{mode}");
        assert_eq!(
            json_file(
                fixture
                    .root
                    .join(format!("evidence/step-{calls:02}.result.json"))
            )["reason"],
            reason,
            "{mode}"
        );
        assert!(!control.join(format!("call-{calls}")).exists());
    }
}

#[test]
fn unrelated_sources_with_ambiguous_or_escaping_paths_stop_at_baseline() {
    for (file, marketplace, calls) in [
        ("baseline-plugins", false, 2),
        ("baseline-markets", true, 3),
    ] {
        for mode in [
            "relative",
            "parent",
            "absolute-parent",
            "prefix",
            "remote",
            "builtin-relative",
            "builtin-uri",
            "builtin-case",
            "builtin-hyphen",
        ] {
            let fixture = prepared("success");
            let source = match mode {
                "relative" => "unknown-origin".to_owned(),
                "builtin-relative" => "./builtin".to_owned(),
                "builtin-uri" => "builtin://secret-canary".to_owned(),
                "builtin-case" => "BUILTIN".to_owned(),
                "builtin-hyphen" => "built-in".to_owned(),
                "parent" => "../../outside".to_owned(),
                "absolute-parent" => format!("{}/../outside", fixture.root.display()),
                "prefix" => format!("{}-outside/source", fixture.root.display()),
                "remote" => "https://example.invalid/secret-canary".to_owned(),
                _ => unreachable!(),
            };
            let row = if marketplace {
                json!([{"name":"unrelated","source":source,"isDefault":true}])
            } else {
                json!([{"name":"unrelated","marketplace":"unrelated","version":"0","enabled":true,"source":source}])
            };
            private(
                &fixture.parent.join("control").join(file),
                &serde_json::to_vec(&row).unwrap(),
                0o600,
            );
            let preflight = fixture.capture().unwrap();
            let mut evidence = preflight.begin_evidence().unwrap();
            let summary =
                serde_json::to_value(evidence.collect(&AtomicBool::new(false)).unwrap()).unwrap();
            assert_eq!(summary["stop"], "outside_packet_source", "{mode}");
            assert_eq!(summary["client_commands"], calls, "{mode}");
            assert!(
                !fixture
                    .parent
                    .join(format!("control/call-{calls}"))
                    .exists()
            );
            assert_no_secret(&fixture.root.join("evidence"));
        }
    }
}

#[test]
fn terminal_record_or_root_drift_leaves_pending_and_never_replays() {
    for mode in ["evidence-drift", "result-collision", "root-drift"] {
        let fixture = prepared(mode);
        let preflight = fixture.capture().unwrap();
        let mut evidence = preflight.begin_evidence().unwrap();
        assert!(evidence.collect(&AtomicBool::new(false)).is_err(), "{mode}");
        assert_eq!(
            fs::read_to_string(fixture.parent.join("control/count")).unwrap(),
            "4"
        );
        assert!(!fixture.parent.join("control/call-4").exists());
        assert!(fixture.root.join("evidence/step-04.pending.json").exists());
        if mode == "result-collision" {
            assert_eq!(
                fs::read(fixture.root.join("evidence/step-04.result.json")).unwrap(),
                b"manual-sentinel"
            );
        }
        assert!(evidence.collect(&AtomicBool::new(false)).is_err());
    }
}
#[test]
fn native_configuration_content_never_enters_evidence() {
    let fixture = prepared("native-config");
    let preflight = fixture.capture().unwrap();
    let mut evidence = preflight.begin_evidence().unwrap();
    let summary = serde_json::to_value(evidence.collect(&AtomicBool::new(false)).unwrap()).unwrap();
    assert_eq!(summary["complete"], true);
    let terminal = json_file(fixture.root.join("evidence/step-04.result.json"));
    let row = terminal["native_files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == "home/.copilot/config.json")
        .unwrap();
    assert_eq!(row["metadata"]["type"], "regular");
    assert_eq!(row["metadata"].as_object().unwrap().len(), 2);
    let rows = terminal["native_files"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    for path in [
        "home/.copilot",
        "home/.copilot/config.json",
        "home/cache/copilot",
        "project/.git/config",
    ] {
        use std::os::unix::fs::PermissionsExt;
        let matching = rows
            .iter()
            .filter(|row| row["path"] == path)
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 1, "{path}");
        let meta = fs::symlink_metadata(fixture.root.join(path)).unwrap();
        assert_eq!(
            matching[0]["metadata"],
            json!({"type":if meta.is_dir() {"directory"}else{"regular"},"mode":meta.permissions().mode() & 0o7777})
        );
    }

    assert_no_secret(&fixture.root.join("evidence"));
}

#[test]
fn unrelated_builtin_labels_are_counted_without_relaxing_owned_source_checks() {
    let fixture = prepared("success");
    let control = fixture.parent.join("control");
    let plugin = json!({"name":"private-native-name","marketplace":"private-native-marketplace","source":"builtin","version":"secret-canary","enabled":true});
    let market = json!({"name":"private-native-marketplace","source":"builtin","isDefault":true});
    for (file, row, installed, builtin_count) in [
        ("baseline-plugins", plugin.clone(), false, 2),
        ("baseline-markets", market.clone(), false, 0),
        ("plugins", plugin, true, 2),
        ("markets", market, true, 3),
    ] {
        let mut rows = if installed {
            json_file(control.join(file)).as_array().unwrap().clone()
        } else {
            Vec::new()
        };
        let mut filesystem_row = row.clone();
        filesystem_row["name"] = json!("private-filesystem-name");
        filesystem_row["source"] = json!(fixture.root.join("source"));
        rows.insert(0, filesystem_row);
        for index in 0..builtin_count {
            let mut builtin_row = row.clone();
            builtin_row["name"] = json!(format!("private-native-name-{index}"));
            rows.insert(0, builtin_row);
        }
        private(
            &control.join(file),
            &serde_json::to_vec(&rows).unwrap(),
            0o600,
        );
    }
    let preflight = fixture.capture().unwrap();
    let mut evidence = preflight.begin_evidence().unwrap();
    let summary = serde_json::to_value(evidence.collect(&AtomicBool::new(false)).unwrap()).unwrap();
    assert_eq!(summary["complete"], true);
    assert_eq!(summary["client_commands"], 11);
    for sequence in [2, 3, 6, 7, 10, 11] {
        let result = json_file(
            fixture
                .root
                .join(format!("evidence/step-{sequence:02}.result.json")),
        );
        let expected_count = match sequence {
            2 | 6 | 10 => 2,
            7 => 3,
            _ => 0,
        };
        assert_eq!(
            result["observation"]["unrelated_builtin_label_entries"],
            expected_count
        );
        assert_eq!(
            result["observation"]["fixture_named_entries"],
            usize::from(matches!(sequence, 6 | 7))
        );
        assert_eq!(result["qualified"], false);
    }
    assert_no_secret(&fixture.root.join("evidence"));
    for entry in fs::read_dir(fixture.root.join("evidence")).unwrap() {
        let bytes = fs::read(entry.unwrap().path()).unwrap();
        for secret in [
            "private-native-name",
            "private-native-marketplace",
            "private-filesystem-name",
        ] {
            assert!(!String::from_utf8_lossy(&bytes).contains(secret));
        }
    }
}

#[test]
fn builtin_label_cannot_hide_any_owned_namespace_reference() {
    for reference in [
        "plugin-name",
        "plugin-selector",
        "plugin-marketplace",
        "marketplace-name",
        "marketplace-selector",
    ] {
        let fixture = prepared("success");
        let plan = json_file(fixture.root.join("plan.json"));
        let name = plan["native_name"].as_str().unwrap();
        let (file, row, calls) = if matches!(reference, "marketplace-name" | "marketplace-selector")
        {
            let row_name = if reference == "marketplace-selector" {
                format!("{name}@{name}")
            } else {
                name.to_owned()
            };
            (
                "baseline-markets",
                json!({"name":row_name,"source":"builtin","isDefault":true}),
                3,
            )
        } else {
            let row_name = match reference {
                "plugin-name" => name.to_owned(),
                "plugin-selector" => format!("{name}@{name}"),
                _ => "unrelated".to_owned(),
            };
            let marketplace = if reference == "plugin-marketplace" {
                name
            } else {
                "unrelated"
            };
            (
                "baseline-plugins",
                json!({"name":row_name,"marketplace":marketplace,"source":"builtin","version":plan["version"],"enabled":true}),
                2,
            )
        };
        private(
            &fixture.parent.join("control").join(file),
            &serde_json::to_vec(&json!([row])).unwrap(),
            0o600,
        );
        let preflight = fixture.capture().unwrap();
        let mut evidence = preflight.begin_evidence().unwrap();
        let summary =
            serde_json::to_value(evidence.collect(&AtomicBool::new(false)).unwrap()).unwrap();
        assert_eq!(summary["stop"], "outside_packet_source", "{reference}");
        assert_eq!(summary["client_commands"], calls, "{reference}");
        assert!(
            !fixture
                .parent
                .join(format!("control/call-{calls}"))
                .exists()
        );
    }
}

#[test]
fn rejected_marketplace_sources_retain_only_a_fixed_category() {
    for (source, category) in [
        ("GitHub: secret-canary/private", "display_github"),
        ("Local: /secret-canary", "display_local"),
        ("URL: https://secret-canary.invalid", "display_url"),
        ("https://secret-canary.invalid", "uri"),
        ("../secret-canary", "parent_component"),
        ("unknown-secret-canary", "relative"),
        ("/secret-canary", "outside_absolute"),
    ] {
        let fixture = prepared("success");
        private(
            &fixture.parent.join("control/baseline-markets"),
            &serde_json::to_vec(&json!([{
                "name":"secret-canary", "source":source, "isDefault":true
            }]))
            .unwrap(),
            0o600,
        );
        let preflight = fixture.capture().unwrap();
        let mut evidence = preflight.begin_evidence().unwrap();
        let summary =
            serde_json::to_value(evidence.collect(&AtomicBool::new(false)).unwrap()).unwrap();
        assert_eq!(summary["stop"], "outside_packet_source");
        assert_eq!(summary["client_commands"], 3);
        let terminal = json_file(fixture.root.join("evidence/step-03.result.json"));
        assert_eq!(terminal["accepted"], false);
        assert_eq!(terminal["observation"], Value::Null);
        assert_eq!(terminal["source_rejection_category"], category);
        assert!(!fixture.parent.join("control/call-3").exists());
        assert_no_secret(&fixture.root.join("evidence"));
    }
}

#[test]
fn marketplace_only_diagnostic_never_invokes_registration_or_plugin_inventory() {
    for reject in [false, true] {
        let fixture = prepared("marketplace-only");
        let control = fixture.parent.join("control");
        let rows = if reject {
            json!([{"name":"secret-canary", "source":"GitHub: secret-canary", "isDefault":true}])
        } else {
            json!([])
        };
        private(
            &control.join("baseline-markets"),
            &serde_json::to_vec(&rows).unwrap(),
            0o600,
        );
        let preflight = fixture.capture().unwrap();
        let mut evidence = preflight.begin_evidence().unwrap();
        let result = serde_json::to_value(
            evidence
                .collect_marketplace_sources(&AtomicBool::new(false))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(result["client_commands"], 2);
        assert_eq!(result["setup_commands"], 1);
        assert_eq!(result["complete"], !reject);
        assert!(!control.join("call-2").exists());
        let plan = json_file(fixture.root.join("plan.json"));
        assert_eq!(
            json_file(fixture.root.join("evidence/step-01.pending.json"))["arguments"],
            plan["commands"][0]
        );
        assert_eq!(
            json_file(fixture.root.join("evidence/step-02.pending.json"))["arguments"],
            plan["commands"][2]
        );
        let terminal = json_file(fixture.root.join("evidence/step-02.result.json"));
        assert_eq!(
            terminal["source_rejection_category"],
            if reject {
                json!("display_github")
            } else {
                Value::Null
            }
        );
        assert!(evidence.collect(&AtomicBool::new(false)).is_err());
        assert_no_secret(&fixture.root.join("evidence"));
    }
}

#[test]
fn marketplace_check_example_selects_only_the_read_only_sequence() {
    let fixture = prepared("marketplace-only");
    let control = fixture.parent.join("control");
    private(&control.join("baseline-markets"), b"[]", 0o600);
    let output = std::process::Command::new(executable::collector_example())
        .env_clear()
        .env("HOME", &fixture.parent)
        .env("PATH", "/usr/bin:/bin")
        .args(["--execute-marketplace-check"])
        .arg(&fixture.request.packet_root)
        .arg(&fixture.request.plan_sha256)
        .arg(&fixture.request.preparation_sha256)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["complete"], true);
    assert_eq!(summary["client_commands"], 2);
    assert!(!control.join("call-2").exists());
    let pending = json_file(fixture.root.join("evidence/step-02.pending.json"));
    assert_eq!(
        pending["arguments"],
        json!([
            "--no-auto-update",
            "plugin",
            "marketplace",
            "list",
            "--json"
        ])
    );
    assert_eq!(pending["collection"], "marketplace_sources");
}
