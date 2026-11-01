//! Physical root identity and live multi-root exclusion for native coordination.
#![cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs;
use std::os::unix::fs::{MetadataExt, symlink};
use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};
use upskill::filesystem::QualifiedScope;
use upskill::native_delivery::roots::{NativeRoot, lock_native_roots};
use upskill::ownership::{InstallationRoot, OwnershipScope};

// Process creation can briefly inherit another thread's flock before exec.
// Serialize this harness, matching the existing scope-lock process tests.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn root(path: &Path, scope: OwnershipScope) -> InstallationRoot {
    InstallationRoot {
        scope,
        path: path.into(),
    }
}
fn entry<'a>(root: &InstallationRoot, scope: &'a QualifiedScope) -> NativeRoot<'a> {
    NativeRoot {
        root: root.clone(),
        scope,
    }
}

#[test]
fn root_binding_matches_actual_directory_and_cannot_retarget_after_replacement() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("root");
    fs::create_dir(&path).unwrap();
    let root = root(&path, OwnershipScope::Project);
    let scope = QualifiedScope::open(&root).unwrap();
    let binding = scope.binding().unwrap();
    let metadata = fs::metadata(&path).unwrap();
    assert_eq!(binding.selected, path);
    assert_eq!(binding.resolved, fs::canonicalize(&path).unwrap());
    assert_eq!(
        (binding.device, binding.inode),
        (metadata.dev(), metadata.ino())
    );
    fs::rename(&path, tmp.path().join("retained")).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(scope.binding().is_err());
}

#[test]
fn roots_lock_in_physical_order_hold_every_guard_and_release_without_files() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let ra = root(a.path(), OwnershipScope::Project);
    let rb = root(b.path(), OwnershipScope::User);
    let sa = QualifiedScope::open(&ra).unwrap();
    let sb = QualifiedScope::open(&rb).unwrap();
    let mut expected = vec![
        (
            fs::metadata(a.path()).unwrap().dev(),
            fs::metadata(a.path()).unwrap().ino(),
        ),
        (
            fs::metadata(b.path()).unwrap().dev(),
            fs::metadata(b.path()).unwrap().ino(),
        ),
    ];
    expected.sort();
    let input = if (
        fs::metadata(a.path()).unwrap().dev(),
        fs::metadata(a.path()).unwrap().ino(),
    ) < (
        fs::metadata(b.path()).unwrap().dev(),
        fs::metadata(b.path()).unwrap().ino(),
    ) {
        vec![entry(&rb, &sb), entry(&ra, &sa)]
    } else {
        vec![entry(&ra, &sa), entry(&rb, &sb)]
    };
    let locks = lock_native_roots(&input).unwrap();
    let bindings = locks.bindings();
    assert_eq!(
        bindings
            .iter()
            .map(|r| (r.binding.device, r.binding.inode))
            .collect::<Vec<_>>(),
        expected
    );
    assert!(sa.try_lock_writer().is_err());
    assert!(sb.try_lock_writer().is_err());
    locks.verify_bindings().unwrap();
    drop(locks);
    drop(sa.try_lock_writer().unwrap());
    drop(sb.try_lock_writer().unwrap());
    assert_eq!(fs::read_dir(a.path()).unwrap().count(), 0);
    assert_eq!(fs::read_dir(b.path()).unwrap().count(), 0);
}

#[test]
fn roots_reject_forged_bindings_duplicates_and_ancestor_aliases_before_locking() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let ra = root(a.path(), OwnershipScope::Project);
    let rb = root(b.path(), OwnershipScope::User);
    let sa = QualifiedScope::open(&ra).unwrap();
    let sb = QualifiedScope::open(&rb).unwrap();
    assert!(lock_native_roots(&[entry(&ra, &sb)]).is_err());
    assert!(
        lock_native_roots(&[entry(&ra, &sa), entry(&ra, &sa)])
            .unwrap_err()
            .to_string()
            .contains("duplicate or aliased")
    );
    let holder = tempfile::tempdir().unwrap();
    let child = a.path().join("child");
    fs::create_dir(&child).unwrap();
    symlink(a.path(), holder.path().join("alias")).unwrap();
    let direct = root(&child, OwnershipScope::Project);
    let alias = root(&holder.path().join("alias/child"), OwnershipScope::User);
    let sd = QualifiedScope::open(&direct).unwrap();
    let sl = QualifiedScope::open(&alias).unwrap();
    assert!(
        lock_native_roots(&[entry(&direct, &sd), entry(&alias, &sl)])
            .unwrap_err()
            .to_string()
            .contains("duplicate or aliased")
    );
    assert!(lock_native_roots(&[]).is_err());
    drop(sa.try_lock_writer().unwrap());
    drop(sb.try_lock_writer().unwrap());
    assert_eq!(fs::read_dir(b.path()).unwrap().count(), 0);
    let _ = rb;
}

#[test]
fn partial_lock_failure_releases_earlier_roots_and_later_replacement_is_detected() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let ra = root(a.path(), OwnershipScope::Project);
    let rb = root(b.path(), OwnershipScope::User);
    let sa = QualifiedScope::open(&ra).unwrap();
    let sb = QualifiedScope::open(&rb).unwrap();
    let pair = if (
        fs::metadata(a.path()).unwrap().dev(),
        fs::metadata(a.path()).unwrap().ino(),
    ) < (
        fs::metadata(b.path()).unwrap().dev(),
        fs::metadata(b.path()).unwrap().ino(),
    ) {
        [entry(&ra, &sa), entry(&rb, &sb)]
    } else {
        [entry(&rb, &sb), entry(&ra, &sa)]
    };
    let held = pair[1].scope.try_lock_writer().unwrap();
    assert!(lock_native_roots(&pair).is_err());
    drop(pair[0].scope.try_lock_writer().unwrap());
    drop(held);
    let locks = lock_native_roots(&pair).unwrap();
    locks.verify_bindings().unwrap();
    let retained = tempfile::tempdir().unwrap();
    let old = retained.path().join("retained-root");
    fs::rename(a.path(), &old).unwrap();
    fs::create_dir(a.path()).unwrap();
    assert!(locks.verify_bindings().is_err());
    drop(locks);
    fs::rename(&old, a.path().join("old-root")).unwrap();
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn wait_file(child: &mut Process, path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(child.0.try_wait().unwrap().is_none(), "helper exited");
        assert!(Instant::now() < deadline, "helper readiness timeout");
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn root_lock_child() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let Some(first) = std::env::var_os("UPSKILL_NATIVE_ROOT_FIRST") else {
        return;
    };
    let second = std::env::var_os("UPSKILL_NATIVE_ROOT_SECOND").unwrap();
    let control =
        std::path::PathBuf::from(std::env::var_os("UPSKILL_NATIVE_ROOT_CONTROL").unwrap());
    let ra = root(Path::new(&first), OwnershipScope::Project);
    let rb = root(Path::new(&second), OwnershipScope::User);
    let sa = QualifiedScope::open(&ra).unwrap();
    let sb = QualifiedScope::open(&rb).unwrap();
    let _locks = lock_native_roots(&[entry(&ra, &sa), entry(&rb, &sb)]).unwrap();
    fs::write(control.join("ready"), b"ready").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !control.join("release").exists() {
        assert!(Instant::now() < deadline, "helper release timeout");
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn fresh_process_shared_secondary_root_blocks_then_releases_on_exit_or_crash() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    for crash in [false, true] {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let control = tempfile::tempdir().unwrap();
        fs::create_dir(control.path().join("home")).unwrap();
        let mut child = Process(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "root_lock_child", "--nocapture"])
                .env("UPSKILL_NATIVE_ROOT_FIRST", a.path())
                .env("UPSKILL_NATIVE_ROOT_SECOND", b.path())
                .env("UPSKILL_NATIVE_ROOT_CONTROL", control.path())
                .env("HOME", control.path().join("home"))
                .env("USERPROFILE", control.path().join("home"))
                .spawn()
                .unwrap(),
        );
        wait_file(&mut child, &control.path().join("ready"));
        let ra = root(other.path(), OwnershipScope::Project);
        let rb = root(b.path(), OwnershipScope::User);
        let sa = QualifiedScope::open(&ra).unwrap();
        let sb = QualifiedScope::open(&rb).unwrap();
        assert!(lock_native_roots(&[entry(&ra, &sa), entry(&rb, &sb)]).is_err());
        drop(sa.try_lock_writer().unwrap());
        if crash {
            child.0.kill().unwrap();
        } else {
            fs::write(control.path().join("release"), b"release").unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                if !crash {
                    assert!(status.success());
                }
                break;
            }
            assert!(Instant::now() < deadline, "helper exit timeout");
            thread::sleep(Duration::from_millis(10));
        }
        // Advisory locks alone cannot exclude a pending operation after a crash.
        // Durable root markers are the later store/coordinator prerequisite.
        let locks = lock_native_roots(&[entry(&ra, &sa), entry(&rb, &sb)]).unwrap();
        drop(locks);
        assert_eq!(fs::read_dir(a.path()).unwrap().count(), 0);
        assert_eq!(fs::read_dir(b.path()).unwrap().count(), 0);
    }
}
