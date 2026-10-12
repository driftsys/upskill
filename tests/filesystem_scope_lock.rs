//! Real-filesystem acceptance examples for the inactive scope-lock API.

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod qualified {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::os::unix::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, ExitStatus};
    use std::thread;
    use std::time::{Duration, Instant};

    use upskill::filesystem::QualifiedScope;
    use upskill::ownership::{InstallationRoot, OwnershipScope};

    // Process creation must not overlap another fixture's immediate release check:
    // fork-safe leases are outside this API contract.
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    const DEADLINE: Duration = Duration::from_secs(10);

    fn root(path: &Path) -> InstallationRoot {
        InstallationRoot {
            scope: OwnershipScope::Project,
            path: path.to_owned(),
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Contents {
        Directory,
        File(Vec<u8>),
        Link(PathBuf),
        Special,
    }

    fn snapshot(path: &Path) -> BTreeMap<PathBuf, (u32, Contents)> {
        fn visit(base: &Path, path: &Path, result: &mut BTreeMap<PathBuf, (u32, Contents)>) {
            let metadata = fs::symlink_metadata(path).unwrap();
            let contents = if metadata.is_symlink() {
                Contents::Link(fs::read_link(path).unwrap())
            } else if metadata.is_dir() {
                Contents::Directory
            } else if metadata.is_file() {
                Contents::File(fs::read(path).unwrap())
            } else {
                Contents::Special
            };
            result.insert(
                path.strip_prefix(base).unwrap().to_owned(),
                (metadata.permissions().mode(), contents),
            );
            if metadata.is_dir() {
                for entry in fs::read_dir(path).unwrap() {
                    visit(base, &entry.unwrap().path(), result);
                }
            }
        }
        let mut result = BTreeMap::new();
        visit(path, path, &mut result);
        result
    }

    fn fixture(path: &Path) {
        fs::create_dir(path.join("nested")).unwrap();
        fs::write(path.join("nested/data"), b"exact\0bytes\xff\n").unwrap();
        fs::set_permissions(path.join("nested/data"), fs::Permissions::from_mode(0o640)).unwrap();
        fs::set_permissions(path.join("nested"), fs::Permissions::from_mode(0o750)).unwrap();
        symlink("nested/data", path.join("link")).unwrap();
    }

    fn busy(scope: &QualifiedScope) {
        let started = Instant::now();
        let error = scope
            .try_lock_writer()
            .expect_err("competing writer must be busy");
        assert!(error.to_string().contains("busy"), "{error:#}");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "lock must be nonblocking"
        );
    }

    #[test]
    fn q01_pin_lock_release_preserves_bytes_entries_and_modes() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        fixture(tmp.path());
        let before = snapshot(tmp.path());
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        scope.verify_binding().unwrap();
        let guard = scope.try_lock_writer().unwrap();
        guard.verify_binding().unwrap();
        assert_eq!(snapshot(tmp.path()), before);
        drop(guard);
        drop(scope.try_lock_writer().unwrap());
        drop(scope);
        assert_eq!(snapshot(tmp.path()), before);
    }

    #[test]
    fn q02_reject_invalid_roots_without_effects() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("directory")).unwrap();
        fs::write(tmp.path().join("file"), b"unchanged").unwrap();
        symlink("directory", tmp.path().join("alias")).unwrap();
        let _socket = std::os::unix::net::UnixListener::bind(tmp.path().join("socket")).unwrap();
        let before = snapshot(tmp.path());
        assert!(
            Path::new(".").is_dir(),
            "relative directory fixture must exist"
        );
        let invalid = [
            PathBuf::new(),
            PathBuf::from("."),
            PathBuf::from("relative"),
            tmp.path().join("directory/../directory"),
            tmp.path().join("missing"),
            tmp.path().join("socket"),
            tmp.path().join("file"),
            tmp.path().join("alias"),
            tmp.path().join("alias/"),
            tmp.path().join("alias/."),
        ];
        for path in invalid {
            assert!(
                QualifiedScope::open(&root(&path)).is_err(),
                "accepted {path:?}"
            );
            assert_eq!(snapshot(tmp.path()), before);
        }
    }

    #[test]
    fn q03_aliases_labels_contend_and_distinct_roots_do_not() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let directory = tmp.path().join("directory");
        let other = tmp.path().join("other");
        fs::create_dir(&directory).unwrap();
        fs::create_dir(&other).unwrap();
        symlink(&directory, tmp.path().join("ancestor")).unwrap();
        fs::create_dir(directory.join("child")).unwrap();
        let before = snapshot(tmp.path());
        let scope = QualifiedScope::open(&root(&directory.join("child"))).unwrap();
        let alias = QualifiedScope::open(&InstallationRoot {
            scope: OwnershipScope::User,
            path: tmp.path().join("ancestor/child/."),
        })
        .unwrap();
        let normalized = QualifiedScope::open(&root(&directory.join("child/"))).unwrap();
        let independent = QualifiedScope::open(&root(&other)).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        busy(&alias);
        busy(&normalized);
        drop(independent.try_lock_writer().unwrap());
        drop(guard);
        drop(alias.try_lock_writer().unwrap());
        assert_eq!(snapshot(tmp.path()), before);
    }

    #[test]
    fn q04_same_scope_uses_independent_open_descriptions() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        busy(&scope);
        drop(guard);
        let next = scope.try_lock_writer().unwrap();
        busy(&scope);
        drop(next);
        drop(scope.try_lock_writer().unwrap());
    }

    struct Process(Child);
    impl Process {
        fn wait(&mut self) -> ExitStatus {
            let deadline = Instant::now() + DEADLINE;
            loop {
                if let Some(status) = self.0.try_wait().unwrap() {
                    return status;
                }
                assert!(Instant::now() < deadline, "child exit deadline exceeded");
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    impl Drop for Process {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn helper_command(mode: &str, scope: &Path, control: &Path) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "qualified::child_helper", "--nocapture"])
            .env("UPSKILL_SCOPE_TEST_MODE", mode)
            .env("UPSKILL_SCOPE_TEST_ROOT", scope)
            .env("UPSKILL_SCOPE_TEST_CONTROL", control)
            .env("HOME", control.join("home"))
            .env("USERPROFILE", control.join("home"));
        command
    }
    fn ready(process: &mut Process, control: &Path) {
        let deadline = Instant::now() + DEADLINE;
        while !control.join("ready").exists() {
            assert!(
                process.0.try_wait().unwrap().is_none(),
                "helper exited before readiness"
            );
            assert!(
                Instant::now() < deadline,
                "helper readiness deadline exceeded"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn wait_release(control: &Path) {
        fs::write(control.join("ready"), b"ready").unwrap();
        let deadline = Instant::now() + DEADLINE;
        while !control.join("release").exists() {
            assert!(
                Instant::now() < deadline,
                "helper release deadline exceeded"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn child_helper() {
        let Some(mode) = std::env::var_os("UPSKILL_SCOPE_TEST_MODE") else {
            return;
        };
        let path = PathBuf::from(std::env::var_os("UPSKILL_SCOPE_TEST_ROOT").unwrap());
        let control = PathBuf::from(std::env::var_os("UPSKILL_SCOPE_TEST_CONTROL").unwrap());
        assert_eq!(std::env::var_os("HOME").unwrap(), control.join("home"));
        assert_eq!(
            std::env::var_os("USERPROFILE").unwrap(),
            control.join("home")
        );
        if mode == "sleeper" {
            wait_release(&control);
            return;
        }
        assert!(mode == "holder" || mode == "exec", "unknown helper mode");
        let scope = QualifiedScope::open(&root(&path)).unwrap();
        let _guard = scope.try_lock_writer().unwrap();
        if mode == "exec" {
            let error = helper_command("sleeper", &path, &control).exec();
            panic!("exec failed: {error}");
        }
        wait_release(&control);
    }

    #[test]
    fn q05_child_contention_normal_and_forced_release() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        for forced in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            let control = tempfile::tempdir().unwrap();
            fs::create_dir(control.path().join("home")).unwrap();
            fixture(tmp.path());
            let before = snapshot(tmp.path());
            let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
            let mut process = Process(
                helper_command("holder", tmp.path(), control.path())
                    .spawn()
                    .unwrap(),
            );
            ready(&mut process, control.path());
            busy(&scope);
            if forced {
                process.0.kill().unwrap();
            } else {
                fs::write(control.path().join("release"), b"release").unwrap();
            }
            assert_eq!(process.wait().success(), !forced);
            drop(scope.try_lock_writer().unwrap());
            assert_eq!(snapshot(tmp.path()), before);
        }
    }

    #[test]
    fn q06_replacement_and_symlink_substitution_invalidate_binding() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        for replacement in ["missing", "directory", "symlink"] {
            let tmp = tempfile::tempdir().unwrap();
            let path = tmp.path().join("scope");
            fs::create_dir(&path).unwrap();
            fixture(&path);
            let scope = QualifiedScope::open(&root(&path)).unwrap();
            let guard = scope.try_lock_writer().unwrap();
            fs::rename(&path, tmp.path().join("moved")).unwrap();
            match replacement {
                "directory" => fs::create_dir(&path).unwrap(),
                "symlink" => symlink("moved", &path).unwrap(),
                _ => {}
            }
            let before = snapshot(tmp.path());
            assert!(scope.verify_binding().is_err());
            assert!(guard.verify_binding().is_err());
            assert!(
                scope.try_lock_writer().is_err(),
                "stale scope returned a guard"
            );
            drop(guard);
            assert!(scope.try_lock_writer().is_err());
            assert_eq!(snapshot(tmp.path()), before);
        }
    }

    #[test]
    fn q06_ancestor_alias_retargeting_invalidates_binding() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        for name in ["first", "second"] {
            fs::create_dir_all(tmp.path().join(name).join("child")).unwrap();
        }
        let alias = tmp.path().join("alias");
        symlink("first", &alias).unwrap();
        let scope = QualifiedScope::open(&root(&alias.join("child"))).unwrap();
        fs::remove_file(&alias).unwrap();
        symlink("second", &alias).unwrap();
        let before = snapshot(tmp.path());
        assert!(scope.verify_binding().is_err());
        assert!(scope.try_lock_writer().is_err());
        assert_eq!(snapshot(tmp.path()), before);
    }

    #[test]
    fn q06_stale_binding_is_rejected_before_attempting_a_contended_lock() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("scope");
        fs::create_dir(&path).unwrap();
        let scope = QualifiedScope::open(&root(&path)).unwrap();
        let _guard = scope.try_lock_writer().unwrap();
        fs::rename(&path, tmp.path().join("moved")).unwrap();
        fs::create_dir(&path).unwrap();
        let before = snapshot(tmp.path());
        let error = scope.try_lock_writer().unwrap_err();
        assert!(
            error.to_string().contains("binding"),
            "lock attempted before stale binding rejection: {error:#}"
        );
        assert_eq!(snapshot(tmp.path()), before);
    }

    #[test]
    fn q06_changed_canonical_path_with_same_inode_invalidates_binding() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let original = tmp.path().join("original");
        let renamed = tmp.path().join("renamed");
        fs::create_dir_all(original.join("child")).unwrap();
        fixture(&original.join("child"));
        let alias = tmp.path().join("alias");
        symlink("original", &alias).unwrap();
        let path = alias.join("child");
        let scope = QualifiedScope::open(&root(&path)).unwrap();
        let selected = fs::metadata(&path).unwrap();
        let canonical = fs::canonicalize(&path).unwrap();
        fs::rename(&original, &renamed).unwrap();
        fs::remove_file(&alias).unwrap();
        symlink("renamed", &alias).unwrap();
        let retargeted = fs::metadata(&path).unwrap();
        assert_eq!(
            (selected.dev(), selected.ino()),
            (retargeted.dev(), retargeted.ino())
        );
        assert_ne!(fs::canonicalize(&path).unwrap(), canonical);
        let before = snapshot(tmp.path());
        assert!(
            scope.verify_binding().is_err(),
            "same-inode canonical relocation accepted"
        );
        assert!(
            scope.try_lock_writer().is_err(),
            "same-inode canonical relocation returned guard"
        );
        assert_eq!(snapshot(tmp.path()), before);
    }

    #[test]
    fn q07_native_non_utf8_root_identity() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(OsString::from_vec(b"root-\xff".to_vec()));
        let created = fs::create_dir(&path);
        #[cfg(target_os = "macos")]
        if let Err(error) = &created {
            // APFS rejects invalid UTF-8 before the API can select a root.
            assert_eq!(error.raw_os_error(), Some(92), "unexpected fixture refusal");
            let before = snapshot(tmp.path());
            let error = QualifiedScope::open(&root(&path)).unwrap_err();
            assert!(error.to_string().contains("inspect selected scope root"));
            assert_eq!(snapshot(tmp.path()), before);
            return;
        }
        created.unwrap();
        fixture(&path);
        let before = snapshot(tmp.path());
        let scope = QualifiedScope::open(&root(&path)).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        guard.verify_binding().unwrap();
        busy(&scope);
        drop(guard);
        assert_eq!(snapshot(tmp.path()), before);
    }

    #[test]
    fn q08_exec_does_not_inherit_the_writer_lock() {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let control = tempfile::tempdir().unwrap();
        fs::create_dir(control.path().join("home")).unwrap();
        let before = snapshot(tmp.path());
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        let mut process = Process(
            helper_command("exec", tmp.path(), control.path())
                .spawn()
                .unwrap(),
        );
        ready(&mut process, control.path());
        assert!(
            process.0.try_wait().unwrap().is_none(),
            "exec successor must remain alive"
        );
        let result = scope.try_lock_writer();
        assert!(result.is_ok(), "exec successor retained lock: {result:?}");
        drop(result);
        fs::write(control.path().join("release"), b"release").unwrap();
        assert!(process.wait().success());
        assert_eq!(snapshot(tmp.path()), before);
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[test]
fn q07_unsupported_platform_is_explicit_and_preserves_root() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("existing");
    std::fs::write(&path, b"unchanged").unwrap();
    let root = upskill::ownership::InstallationRoot {
        scope: upskill::ownership::OwnershipScope::Project,
        path: tmp.path().to_owned(),
    };
    let error = upskill::filesystem::QualifiedScope::open(&root).unwrap_err();
    assert!(error.to_string().contains("unsupported"));
    assert_eq!(std::fs::read(path).unwrap(), b"unchanged");
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 1);
}
