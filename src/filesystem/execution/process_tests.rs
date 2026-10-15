use super::*;
use crate::files::plan::{FileLocation, FileValue, PlannedFile};
use crate::filesystem::QualifiedScope;
use crate::ownership::{InstallationRoot, OwnershipScope};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::{
    fs::{MetadataExt, PermissionsExt},
    net::{UnixListener, UnixStream},
};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn root(path: &std::path::Path) -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: path.into(),
    }
}
fn request(path: &std::path::Path) -> ExecutionRequest {
    let action = |name: &str, before: &[u8], after: Option<&[u8]>| PlannedFile {
        location: FileLocation {
            root: root(path),
            path: name.into(),
        },
        before: Some(FileValue {
            bytes: before.into(),
            mode: Some(0o600),
        }),
        after: after.map(|v| FileValue {
            bytes: v.into(),
            mode: Some(0o600),
        }),
    };
    ExecutionRequest {
        files: vec![
            action("z", b"old-z", Some(b"new-z")),
            action("b", b"old-b", Some(b"new-b")),
            action("removed", b"remove", None),
            action("noop", b"same", Some(b"same")),
        ],
        state: action(".upskill-lock.json", b"old-state", Some(b"new-state")),
    }
}
fn setup() -> tempfile::TempDir {
    let tmp = tempfile::Builder::new()
        .prefix("upskill-exec-")
        .tempdir_in("/tmp")
        .unwrap();
    for action in request(tmp.path())
        .files
        .iter()
        .chain(std::iter::once(&request(tmp.path()).state))
    {
        let path = tmp.path().join(&action.location.path);
        std::fs::write(&path, &action.before.as_ref().unwrap().bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    tmp
}
struct Paused {
    child: Child,
    _stream: UnixStream,
}
fn stop_at(tmp: &tempfile::TempDir, point: &str, index: usize) -> Paused {
    let socket = tmp.path().join("probe.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "filesystem::execution::process_tests::probe",
            "--ignored",
            "--nocapture",
        ])
        .env("UPSKILL_EXECUTION_PROBE_ROOT", tmp.path())
        .env("UPSKILL_EXECUTION_PROBE_POINT", point)
        .env("UPSKILL_EXECUTION_PROBE_INDEX", index.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "probe exited before connecting"
                );
                if start.elapsed() > Duration::from_secs(15) {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!("probe connection timeout");
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(e) => panic!("probe connection failed: {e}"),
        }
    };
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    let mut receipt = String::new();
    let mut reader = BufReader::new(stream);
    let result = reader.read_line(&mut receipt);
    if result.is_err() || receipt != format!("{point}:{index}\n") {
        let _ = child.kill();
        let _ = child.wait();
        panic!("probe did not stop at {point}:{index}: {receipt:?}, {result:?}");
    }
    Paused {
        child,
        _stream: reader.into_inner(),
    }
}
fn kill(paused: &mut Paused) {
    paused.child.kill().unwrap();
    paused.child.wait().unwrap();
}
fn finish(tmp: &tempfile::TempDir) -> ExecutionReport {
    let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let report = match scope.recover_operation(&guard).unwrap() {
        Some(report) => report,
        None => scope
            .apply_operation(&guard, &[], request(tmp.path()))
            .unwrap(),
    };
    assert_eq!(report.outcome, ExecutionOutcome::Complete);
    for action in request(tmp.path())
        .files
        .iter()
        .chain(std::iter::once(&request(tmp.path()).state))
    {
        let path = tmp.path().join(&action.location.path);
        if let Some(value) = &action.after {
            assert_eq!(std::fs::read(&path).unwrap(), value.bytes);
            assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o7777, 0o600);
        } else {
            assert!(!path.exists());
        }
    }
    report
}
#[test]
#[ignore = "subprocess entrypoint; invoked only by process acceptance tests"]
fn probe() {
    let path = std::path::PathBuf::from(
        std::env::var_os("UPSKILL_EXECUTION_PROBE_ROOT").expect("probe needs isolated root"),
    );
    let target = std::env::var("UPSKILL_EXECUTION_PROBE_POINT").unwrap();
    let target_index: usize = std::env::var("UPSKILL_EXECUTION_PROBE_INDEX")
        .unwrap()
        .parse()
        .unwrap();
    let mut stream = UnixStream::connect(path.join("probe.sock")).unwrap();
    let mut ready = stream.try_clone().unwrap();
    TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |point, index| {
            if point == target && index == target_index {
                writeln!(ready, "{point}:{index}")?;
                let mut byte = [0];
                ready.read_exact(&mut byte)?;
            }
            Ok(())
        }))
    });
    let scope = QualifiedScope::open(&root(&path)).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    scope.apply_operation(&guard, &[], request(&path)).unwrap();
    writeln!(stream, "complete").unwrap();
}
#[test]
fn recovers_after_every_effect_and_receipt_boundary() {
    for index in 0..5 {
        for point in [
            "before_pending",
            "after_pending",
            "before_effect",
            "after_effect",
            "after_effect_sync",
            "before_done",
            "after_done",
        ] {
            if index == 3 && !["before_done", "after_done"].contains(&point) {
                continue;
            }
            let tmp = setup();
            let mut child = stop_at(&tmp, point, index);
            kill(&mut child);
            let path = tmp.path().join(if index == 4 {
                ".upskill-lock.json"
            } else {
                ["z", "b", "removed", "noop"][index]
            });
            let inode = std::fs::metadata(&path).ok().map(|v| v.ino());
            finish(&tmp);
            if [
                "after_effect",
                "after_effect_sync",
                "before_done",
                "after_done",
            ]
            .contains(&point)
            {
                assert_eq!(
                    std::fs::metadata(&path).ok().map(|v| v.ino()),
                    inode,
                    "replayed observed effect at {point}:{index}"
                );
            }
        }
    }
}
#[test]
fn concurrent_writer_is_rejected_and_kill_releases_lock() {
    let tmp = setup();
    let mut child = stop_at(&tmp, "after_pending", 0);
    let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
    assert!(scope.try_lock_writer().is_err());
    kill(&mut child);
    finish(&tmp);
}
#[test]
fn unexpected_edit_after_crash_survives() {
    let tmp = setup();
    let mut child = stop_at(&tmp, "after_pending", 0);
    kill(&mut child);
    std::fs::write(tmp.path().join("z"), b"manual").unwrap();
    let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let report = scope.recover_operation(&guard).unwrap().unwrap();
    assert_eq!(report.outcome, ExecutionOutcome::Reconcile { index: 0 });
    assert_eq!(std::fs::read(tmp.path().join("z")).unwrap(), b"manual");
    assert_eq!(
        std::fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        b"old-state"
    );
}
#[test]
fn initial_publication_and_cleanup_interruptions_are_safe() {
    for (point, index) in [
        ("after_setup", 0),
        ("after_payload", 0),
        ("before_initial_journal", 0),
        ("after_initial_journal", 0),
        ("after_cleanup_marker", 0),
        ("after_cleanup_payload", 0),
        ("before_cleanup_receipt_removal", 0),
    ] {
        let tmp = setup();
        let mut child = stop_at(&tmp, point, index);
        kill(&mut child);
        std::fs::write(tmp.path().join(".upskill/state/user-extra"), b"manual").unwrap();
        finish(&tmp);
        assert_eq!(
            std::fs::read(tmp.path().join(".upskill/state/user-extra")).unwrap(),
            b"manual"
        );
    }
}
#[test]
fn journal_generation_crashes_retain_published_intent() {
    for generation in 0..=10 {
        for point in [
            "before_journal_rename",
            "after_journal_rename",
            "after_journal_sync",
        ] {
            let tmp = setup();
            let mut child = stop_at(&tmp, point, generation);
            kill(&mut child);
            finish(&tmp);
        }
    }
}
