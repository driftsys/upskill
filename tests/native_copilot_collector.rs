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
if [ "$n" -eq 0 ]; then
  if [ "$mode" = version ]; then printf 'unsupported'; else /bin/cat "$control/version"; fi
  exit 0
fi
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
  1|2|9|10) /bin/cat "$control/empty";;
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
        assert_eq!(result["qualified"], false);
    }
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
    let fixture = prepared("success");
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
    assert_no_secret(&fixture.root.join("evidence"));
}

#[test]
fn conflicting_or_outside_fixture_claims_stop_before_any_cleanup() {
    for (mode, calls) in [
        ("baseline", 2),
        ("version", 6),
        ("duplicate", 6),
        ("source", 7),
    ] {
        let fixture = prepared("success");
        let control = fixture.parent.join("control");
        if mode == "baseline" {
            fs::copy(control.join("plugins"), control.join("empty")).unwrap();
        } else {
            let path = control.join(if mode == "source" {
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
        assert!(!control.join(format!("call-{calls}")).exists());
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
    assert_no_secret(&fixture.root.join("evidence"));
}
