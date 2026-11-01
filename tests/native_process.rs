//! Bounded fake-native CLI execution under a qualified writer guard.
#![cfg(any(target_os = "linux", target_os = "macos"))]
use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use upskill::native_delivery::process::{
    NativeExecutable, NativeGroupState, NativeProcessError, NativeStopReason, run_native_process,
};

#[path = "common/native_process.rs"]
mod fixture;
use fixture::*;

#[test]
fn exact_arguments_environment_cwd_closed_stdin_and_nonzero_exit_are_preserved() {
    with_scope(|root, scope, guard| {
        let mut spec = script(
            root,
            r#"printf 'stdout-canary\n'
printf '%s\n' "$HOME" "$MARKER" "$PATH" "$@"
if read line; then printf 'unexpected-stdin'; else printf 'stdin-closed\n'; fi
pwd -P
printf 'stderr-canary' >&2
exit 7"#,
        );
        spec.environment
            .insert("MARKER".into(), "environment-canary".into());
        // sh can supply a default PATH; an inherited provider/user value must
        // still be replaced by this explicit caller value.
        spec.environment
            .insert("PATH".into(), "/fixture-only-path".into());
        spec.arguments = ["argument-canary with spaces", "", "; touch must-not-exist"]
            .map(OsString::from)
            .into();
        let output = run_native_process(scope, guard, &spec, &AtomicBool::new(false)).unwrap();
        assert_eq!(output.reason, NativeStopReason::Exited);
        assert_eq!(output.exit_code, Some(7));
        assert_eq!(output.stderr, b"stderr-canary");
        assert_eq!(
            String::from_utf8(output.stdout.clone()).unwrap(),
            format!(
                "stdout-canary\n{}\nenvironment-canary\n/fixture-only-path\nargument-canary with spaces\n\n; touch must-not-exist\nstdin-closed\n{}\n",
                root.display(),
                root.canonicalize().unwrap().display()
            )
        );
        assert!(!root.join("must-not-exist").exists());
        assert_eq!(output.group, NativeGroupState::Empty);
        assert!(output.output_complete && output.scope_unchanged && output.executable_unchanged);
        assert_eq!(
            format!("{output:?}"),
            format!(
                "NativeProcessOutput {{ reason: Exited, exit_code: Some(7), stdout_bytes: {}, stderr_bytes: 13, group: Empty, output_complete: true, scope_unchanged: true, executable_unchanged: true, .. }}",
                output.stdout.len()
            )
        );
        assert_eq!(
            format!("{spec:?}"),
            "NativeCommandSpec { argument_count: 3, environment_count: 4, timeout: 2s, output_limit: 4096, .. }"
        );
    });
}
#[test]
fn unavailable_changed_or_replaced_executables_cannot_run() {
    with_scope(|root, scope, guard| {
        assert!(matches!(
            NativeExecutable::capture(&root.join("missing")),
            Err(NativeProcessError::Unavailable)
        ));
        let spec = script(root, "printf ran > ran");
        std::fs::write(
            root.join("native-fixture"),
            b"#!/bin/sh\nprintf changed > ran\n",
        )
        .unwrap();
        assert!(matches!(
            run_native_process(scope, guard, &spec, &AtomicBool::new(false)),
            Err(NativeProcessError::ExecutableChanged)
        ));
        assert!(!root.join("ran").exists());
        let spec = script(root, "printf ran > ran");
        std::fs::rename(root.join("native-fixture"), root.join("old-fixture")).unwrap();
        std::fs::copy(root.join("old-fixture"), root.join("native-fixture")).unwrap();
        assert!(matches!(
            run_native_process(scope, guard, &spec, &AtomicBool::new(false)),
            Err(NativeProcessError::ExecutableChanged)
        ));
        assert!(!root.join("ran").exists());
    });
}
#[test]
fn invalid_limits_arguments_and_wrong_writer_guard_reject_without_effects() {
    with_scope(|root, scope, guard| {
        for case in 0..9 {
            let mut spec = script(root, "printf ran > ran");
            match case {
                0 => spec.timeout = Duration::ZERO,
                1 => spec.timeout = Duration::from_secs(301),
                2 => spec.output_limit = 0,
                3 => spec.output_limit = 1024 * 1024 + 1,
                4 => spec.arguments.push("bad\0argument".into()),
                5 => {
                    spec.environment.insert("".into(), "value".into());
                }
                6 => {
                    spec.environment.insert("bad=key".into(), "value".into());
                }
                7 => {
                    spec.environment.insert("bad\0key".into(), "value".into());
                }
                _ => {
                    spec.environment.insert("key".into(), "bad\0value".into());
                }
            }
            assert!(
                matches!(
                    run_native_process(scope, guard, &spec, &AtomicBool::new(false)),
                    Err(NativeProcessError::InvalidInvocation)
                ),
                "invalid case {case} was accepted"
            );
        }
        let spec = script(root, "printf ran > ran");
        with_scope(|_, _, other_guard| {
            assert!(matches!(
                run_native_process(scope, other_guard, &spec, &AtomicBool::new(false)),
                Err(NativeProcessError::UnsafeScope)
            ));
        });
        assert!(!root.join("ran").exists());
    });
}
#[test]
fn output_overflow_is_bounded_and_never_becomes_a_success_exit() {
    with_scope(|root, scope, guard| {
        let mut spec = script(
            root,
            "while :; do printf '0123456789'; printf 'abcdefghij' >&2; done",
        );
        spec.output_limit = 128;
        let output = run_native_process(scope, guard, &spec, &AtomicBool::new(false)).unwrap();
        assert_eq!(output.reason, NativeStopReason::OutputLimit);
        assert!(output.stdout.len() + output.stderr.len() <= 128);
        assert_eq!(output.group, NativeGroupState::Empty);
    });
}
#[test]
fn wall_clock_timeout_is_enforced_without_signalling_an_unrelated_group() {
    with_scope(|root, scope, guard| {
        struct ExtraChild(std::process::Child);
        impl Drop for ExtraChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut unrelated = ExtraChild(
            std::process::Command::new("/bin/sleep")
                .arg("20")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .process_group(0)
                .spawn()
                .unwrap(),
        );
        let mut spec = script(root, "while :; do :; done");
        spec.timeout = Duration::from_millis(100);
        let start = Instant::now();
        let output = run_native_process(scope, guard, &spec, &AtomicBool::new(false)).unwrap();
        assert_eq!(output.reason, NativeStopReason::TimedOut);
        assert!(unrelated.0.try_wait().unwrap().is_none());
        assert!(start.elapsed() < Duration::from_secs(4));
        assert!(output.direct_child_reaped);
    });
}

#[test]
fn a_successful_leader_cannot_leave_a_live_background_writer() {
    with_scope(|root, scope, guard| {
        let spec = descendants(root, true);
        let output = run_native_process(scope, guard, &spec, &AtomicBool::new(false)).unwrap();
        assert_eq!(output.reason, NativeStopReason::Exited);
        assert_eq!(output.exit_code, Some(0));
        stopped_heartbeat(root);
        assert!(output.direct_child_reaped);
    });
}
#[test]
fn cancellation_before_spawn_is_typed_and_after_spawn_stops_descendants() {
    with_scope(|root, scope, guard| {
        let spec = descendants(root, false);
        assert!(matches!(
            run_native_process(scope, guard, &spec, &AtomicBool::new(true)),
            Err(NativeProcessError::CancelledBeforeLaunch)
        ));
        assert!(!root.join("child-pid").exists());
        let cancellation = AtomicBool::new(false);
        std::thread::scope(|threads| {
            threads.spawn(|| {
                let deadline = Instant::now() + Duration::from_secs(4);
                while !(root.join("parent-ready").exists()
                    && std::fs::metadata(root.join("heartbeat")).is_ok_and(|m| m.len() > 0))
                    && Instant::now() < deadline
                {
                    std::thread::sleep(Duration::from_millis(5));
                }
                cancellation.store(true, Ordering::SeqCst);
            });
            let output = run_native_process(scope, guard, &spec, &cancellation).unwrap();
            assert_eq!(output.reason, NativeStopReason::Cancelled);
            stopped_heartbeat(root);
        });
    });
}

#[test]
fn capture_rejects_special_files_without_blocking_on_open() {
    with_scope(|root, _, _| {
        let fifo = root.join("fifo-launcher");
        assert!(
            std::process::Command::new("/usr/bin/mkfifo")
                .args(["-m", "700"])
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        assert!(matches!(
            NativeExecutable::capture(&fifo),
            Err(NativeProcessError::Unavailable)
        ));
    });
}

#[test]
fn namespace_and_launcher_drift_are_reported_without_losing_the_attempt() {
    with_scope(|root, scope, guard| {
        let moved = root.with_file_name("moved-scope");
        let mut spec = script(
            root,
            r#"/bin/mv "$SELECTED" "$MOVED"
printf finished"#,
        );
        spec.environment
            .insert("SELECTED".into(), root.as_os_str().into());
        spec.environment
            .insert("MOVED".into(), moved.as_os_str().into());
        let attempt = run_native_process(scope, guard, &spec, &AtomicBool::new(false));
        if moved.exists() {
            std::fs::rename(&moved, root).unwrap();
        }
        let output = attempt.unwrap();
        assert_eq!(output.reason, NativeStopReason::Exited);
        assert_eq!(output.stdout, b"finished");
        assert!(!output.scope_unchanged);
        assert!(!output.executable_unchanged);
    });
    with_scope(|root, scope, guard| {
        let spec = script(
            root,
            r#"printf '#!/bin/sh\nexit 0\n' > "$0"
printf modified"#,
        );
        let output = run_native_process(scope, guard, &spec, &AtomicBool::new(false)).unwrap();
        assert_eq!(output.reason, NativeStopReason::Exited);
        assert_eq!(output.stdout, b"modified");
        assert!(output.scope_unchanged);
        assert!(!output.executable_unchanged);
    });
}
#[test]
fn controller_environment_cannot_leak_into_the_native_child() {
    let home = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "isolated_environment_probe"])
        .current_dir(home.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("UPSKILL_NATIVE_ENV_CANARY", "must-not-inherit")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
#[ignore = "fresh controller process entrypoint, invoked by its parent acceptance test"]
fn isolated_environment_probe() {
    assert_eq!(
        std::env::var("UPSKILL_NATIVE_ENV_CANARY").unwrap(),
        "must-not-inherit"
    );
    with_scope(|root, scope, guard| {
        let spec = script(root, r#"printf '%s' "$UPSKILL_NATIVE_ENV_CANARY""#);
        let output = run_native_process(scope, guard, &spec, &AtomicBool::new(false)).unwrap();
        assert_eq!(output.reason, NativeStopReason::Exited);
        assert!(output.stdout.is_empty());
    });
}

#[test]
fn exact_output_boundary_is_complete_and_one_extra_byte_is_truncated() {
    with_scope(|root, scope, guard| {
        let mut spec = script(root, "printf '12345678'; printf 'abcdefgh' >&2");
        spec.output_limit = 16;
        let output = run_native_process(scope, guard, &spec, &AtomicBool::new(false)).unwrap();
        assert_eq!(output.reason, NativeStopReason::Exited);
        assert_eq!(output.stdout, b"12345678");
        assert_eq!(output.stderr, b"abcdefgh");
        assert!(output.output_complete);
        let mut spec = script(root, "printf '12345678'; printf 'abcdefghi' >&2");
        spec.output_limit = 16;
        let output = run_native_process(scope, guard, &spec, &AtomicBool::new(false)).unwrap();
        assert_eq!(output.reason, NativeStopReason::OutputLimit);
        assert_eq!(output.stdout.len() + output.stderr.len(), 16);
        assert!(!output.output_complete);
    });
}

#[test]
fn launcher_contract_rejects_paths_permissions_and_oversize_files() {
    use std::os::unix::fs::PermissionsExt;
    with_scope(|root, _, _| {
        let _ = script(root, "exit 0");
        assert!(matches!(
            NativeExecutable::capture(std::path::Path::new("./native-fixture")),
            Err(NativeProcessError::InvalidInvocation)
        ));
        let parent_path = root.join("..").join("scope").join("native-fixture");
        assert!(matches!(
            NativeExecutable::capture(&parent_path),
            Err(NativeProcessError::InvalidInvocation)
        ));
        let launcher = root.join("native-fixture");
        for mode in [0o600, 0o4700, 0o2700] {
            std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(
                std::fs::metadata(&launcher).unwrap().permissions().mode() & 0o7777,
                mode
            );
            assert!(
                matches!(
                    NativeExecutable::capture(&launcher),
                    Err(NativeProcessError::Unavailable)
                ),
                "accepted mode {mode:o}"
            );
        }
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o700)).unwrap();
        // A sparse file pins the public 1 GiB admission limit without writing
        // a gigabyte or depending on the implementation's private constant.
        std::fs::OpenOptions::new()
            .write(true)
            .open(&launcher)
            .unwrap()
            .set_len(1_073_741_825)
            .unwrap();
        assert!(matches!(
            NativeExecutable::capture(&launcher),
            Err(NativeProcessError::Unavailable)
        ));
    });
}
