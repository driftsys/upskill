//! Real-filesystem acceptance examples for inactive destination observation.
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod qualified {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use upskill::files::plan::FileLocation;
    use upskill::filesystem::QualifiedScope;
    use upskill::ownership::{InstallationRoot, OwnershipScope};
    fn root(path: &Path) -> InstallationRoot {
        InstallationRoot {
            scope: OwnershipScope::Project,
            path: path.to_owned(),
        }
    }
    fn location(path: &Path, relative: &str) -> FileLocation {
        FileLocation {
            root: root(path),
            path: relative.to_owned(),
        }
    }
    #[test]
    fn d01_capture_exact_binary_bytes_and_mode() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("data"), b"exact\0\xff\n").unwrap();
        fs::set_permissions(tmp.path().join("data"), fs::Permissions::from_mode(0o640)).unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        let snapshot = scope
            .observe_destinations(&[location(tmp.path(), "data")])
            .unwrap();
        let value = snapshot.observations()[0].value.as_ref().unwrap();
        assert_eq!(value.bytes, b"exact\0\xff\n");
        assert_eq!(value.mode, Some(0o640));
        assert_eq!(format!("{snapshot:?}"), "DestinationSnapshot { .. }");
        snapshot.verify_unchanged().unwrap();
    }
    #[test]
    fn d01_heterogeneous_batch_keeps_exact_values_associated_with_locations() {
        use upskill::files::plan::{FileValue, ObservedFile};

        let tmp = tempfile::tempdir().unwrap();
        let first_bytes = b"first\0\xff\n";
        let second_bytes = b"\x80second\0\x01";
        fs::write(tmp.path().join("a-first.bin"), first_bytes).unwrap();
        fs::write(tmp.path().join("b-second.bin"), second_bytes).unwrap();
        fs::set_permissions(
            tmp.path().join("a-first.bin"),
            fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        fs::set_permissions(
            tmp.path().join("b-second.bin"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let first = location(tmp.path(), "a-first.bin");
        let second = location(tmp.path(), "b-second.bin");
        let absent = location(tmp.path(), "z-absent.bin");
        let expected = vec![
            ObservedFile {
                location: first.clone(),
                value: Some(FileValue {
                    bytes: first_bytes.to_vec(),
                    mode: Some(0o640),
                }),
            },
            ObservedFile {
                location: second.clone(),
                value: Some(FileValue {
                    bytes: second_bytes.to_vec(),
                    mode: Some(0o600),
                }),
            },
            ObservedFile {
                location: absent.clone(),
                value: None,
            },
        ];
        let input = vec![absent, first, second];
        let reversed: Vec<_> = input.iter().cloned().rev().collect();
        let before = unchanged_tree(tmp.path());
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        for locations in [&input, &reversed] {
            let snapshot = scope.observe_destinations(locations).unwrap();
            assert_eq!(snapshot.observations(), expected.as_slice());
            snapshot.verify_unchanged().unwrap();
        }
        assert_eq!(unchanged_tree(tmp.path()), before);
    }
    #[test]
    fn d02_absent_leaf_and_parents_create_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        let snapshot = scope
            .observe_destinations(&[
                location(tmp.path(), "leaf"),
                location(tmp.path(), "missing/parent/file"),
            ])
            .unwrap();
        assert!(
            snapshot
                .observations()
                .iter()
                .all(|file| file.value.is_none())
        );
        snapshot.verify_unchanged().unwrap();
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
        assert!(
            scope
                .observe_destinations(&[])
                .unwrap()
                .observations()
                .is_empty()
        );
    }

    fn unchanged_tree(
        path: &Path,
    ) -> std::collections::BTreeMap<std::path::PathBuf, (u32, Vec<u8>)> {
        fn visit(
            base: &Path,
            path: &Path,
            out: &mut std::collections::BTreeMap<std::path::PathBuf, (u32, Vec<u8>)>,
        ) {
            let metadata = fs::symlink_metadata(path).unwrap();
            let bytes = if metadata.is_file() {
                fs::read(path).unwrap()
            } else if metadata.is_symlink() {
                use std::os::unix::ffi::OsStrExt;
                fs::read_link(path).unwrap().as_os_str().as_bytes().to_vec()
            } else {
                Vec::new()
            };
            out.insert(
                path.strip_prefix(base).unwrap().to_owned(),
                (metadata.permissions().mode(), bytes),
            );
            if metadata.is_dir() {
                for entry in fs::read_dir(path).unwrap() {
                    visit(base, &entry.unwrap().path(), out);
                }
            }
        }
        let mut out = std::collections::BTreeMap::new();
        visit(path, path, &mut out);
        out
    }
    #[test]
    fn d01_capture_and_verify_preserve_recursive_entries_bytes_modes() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("a")).unwrap();
        fs::write(tmp.path().join("a/data"), b"\0\xff\n").unwrap();
        fs::set_permissions(tmp.path().join("a"), fs::Permissions::from_mode(0o750)).unwrap();
        let before = unchanged_tree(tmp.path());
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        let snapshot = scope
            .observe_destinations(&[location(tmp.path(), "a/data")])
            .unwrap();
        snapshot.verify_unchanged().unwrap();
        assert_eq!(unchanged_tree(tmp.path()), before);
    }
    #[test]
    fn d03_invalid_paths_and_different_root() {
        let tmp = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        for path in [
            "", ".", "../x", "a/../x", "/x", "a//x", "a\\x", "a:x", "a/\u{1}",
        ] {
            assert!(
                scope
                    .observe_destinations(&[location(tmp.path(), path)])
                    .is_err(),
                "{path:?}"
            );
        }
        assert!(
            scope
                .observe_destinations(&[location(other.path(), "data")])
                .is_err()
        );
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    }
    #[test]
    fn d03_equivalent_roots_labels_preserve_exact_locations_and_order() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("real")).unwrap();
        symlink("real", tmp.path().join("alias")).unwrap();
        fs::create_dir(tmp.path().join("real/nested")).unwrap();
        let direct = tmp.path().join("real/nested");
        let alias = tmp.path().join("alias/nested");
        let scope = QualifiedScope::open(&root(&direct)).unwrap();
        let one = location(&direct, "a/x");
        let mut two = location(&alias, "b/y");
        two.root.scope = OwnershipScope::User;
        let snapshot = scope
            .observe_destinations(&[one.clone(), two.clone()])
            .unwrap();
        let reverse = scope
            .observe_destinations(&[two.clone(), one.clone()])
            .unwrap();
        assert_eq!(snapshot.observations(), reverse.observations());
        assert!(snapshot.observations().iter().any(|o| o.location == one));
        assert!(snapshot.observations().iter().any(|o| o.location == two));
        snapshot.verify_unchanged().unwrap();
    }
    fn fifo(path: &Path) {
        #[cfg(target_os = "linux")]
        rustix::fs::mkfifoat(
            rustix::fs::CWD,
            path,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .unwrap();
        #[cfg(target_os = "macos")]
        {
            let home = tempfile::tempdir().unwrap();
            let mut child = std::process::Command::new("/usr/bin/mkfifo")
                .arg(path)
                .env("HOME", home.path())
                .env("USERPROFILE", home.path())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    assert!(status.success());
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!("mkfifo hung");
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
    #[test]
    fn d04_reject_symlinks_file_ancestors_directory_hardlink_socket_and_fifo() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("data"), b"exact").unwrap();
        fs::write(outside.path().join("outside"), b"outside").unwrap();
        fs::create_dir(tmp.path().join("directory")).unwrap();
        symlink("data", tmp.path().join("inside")).unwrap();
        symlink(outside.path().join("outside"), tmp.path().join("outside")).unwrap();
        symlink("directory", tmp.path().join("dirlink")).unwrap();
        symlink(outside.path(), tmp.path().join("outdir")).unwrap();
        fs::hard_link(outside.path().join("outside"), tmp.path().join("hardlink")).unwrap();
        let _socket = std::os::unix::net::UnixListener::bind(tmp.path().join("socket")).unwrap();
        fifo(&tmp.path().join("fifo"));
        let before = unchanged_tree(tmp.path());
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        // The bounded subprocess below independently guards FIFO hangs.
        for path in [
            "inside",
            "outside",
            "dirlink/file",
            "outdir/file",
            "data/child",
            "directory",
            "hardlink",
            "socket",
        ] {
            assert!(
                scope
                    .observe_destinations(&[location(tmp.path(), path)])
                    .is_err(),
                "{path}"
            );
        }
        assert_eq!(unchanged_tree(tmp.path()), before);
    }
    #[test]
    fn d04_fifo_bounded_child() {
        const CHILD: &str = "UPSKILL_DESTINATION_FIFO_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let tmp = tempfile::tempdir().unwrap();
            fifo(&tmp.path().join("fifo"));
            let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
            assert!(
                scope
                    .observe_destinations(&[location(tmp.path(), "fifo")])
                    .is_err()
            );
            return;
        }
        let home = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .env("HOME", home.path())
            .env("USERPROFILE", home.path())
            .args([
                "--exact",
                "qualified::d04_fifo_bounded_child",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("FIFO observation hung");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    #[test]
    fn d05_conservative_overlap_and_permutation() {
        let tmp = tempfile::tempdir().unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        for (a, b) in [
            ("a/x", "a/x"),
            ("a/X", "a/x"),
            ("a/é", "a/e\u{301}"),
            ("a/é", "a/e"),
            ("a/x", "a/x/y"),
            ("a", "a/x"),
            ("absent/X", "absent/x/y"),
        ] {
            let mut two = location(tmp.path(), b);
            two.root.scope = OwnershipScope::User;
            let one = location(tmp.path(), a);
            assert!(
                scope
                    .observe_destinations(&[one.clone(), two.clone()])
                    .is_err(),
                "{a} {b}"
            );
            assert!(scope.observe_destinations(&[two, one]).is_err(), "{b} {a}");
        }
        for (a, b) in [("a/x", "b/y"), ("a/é", "b/e"), ("a/b", "ab/c")] {
            let one = location(tmp.path(), a);
            let two = location(tmp.path(), b);
            assert_eq!(
                scope
                    .observe_destinations(&[one.clone(), two.clone()])
                    .unwrap()
                    .observations(),
                scope
                    .observe_destinations(&[two, one])
                    .unwrap()
                    .observations()
            );
        }
    }
    #[test]
    fn d05_absent_prefixes_across_ancestor_alias_roots() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("real")).unwrap();
        fs::create_dir(tmp.path().join("real/nested")).unwrap();
        symlink("real", tmp.path().join("alias")).unwrap();
        let direct = tmp.path().join("real/nested");
        let alias = tmp.path().join("alias/nested");
        let scope = QualifiedScope::open(&root(&direct)).unwrap();
        assert!(
            scope
                .observe_destinations(&[
                    location(&direct, "missing/a"),
                    location(&alias, "missing/A/child")
                ])
                .is_err()
        );
    }
    #[test]
    fn d06_exact_unicode_and_lookup_aliases() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Exact"), b"case").unwrap();
        fs::write(tmp.path().join("é"), b"unicode").unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        scope
            .observe_destinations(&[location(tmp.path(), "é")])
            .unwrap()
            .verify_unchanged()
            .unwrap();
        for alias in ["exact", "e\u{301}"] {
            if fs::symlink_metadata(tmp.path().join(alias)).is_ok() {
                assert!(
                    scope
                        .observe_destinations(&[location(tmp.path(), alias)])
                        .is_err(),
                    "resolving alias {alias}"
                );
            } else {
                assert!(
                    scope
                        .observe_destinations(&[location(tmp.path(), alias)])
                        .unwrap()
                        .observations()[0]
                        .value
                        .is_none(),
                    "case-sensitive absence"
                );
            }
        }
        fs::create_dir(tmp.path().join("Parent")).unwrap();
        if fs::metadata(tmp.path().join("parent")).is_ok() {
            assert!(
                scope
                    .observe_destinations(&[location(tmp.path(), "parent/missing")])
                    .is_err()
            );
        }
    }
    fn changed(mutate: impl FnOnce(&Path)) {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("parent")).unwrap();
        fs::write(tmp.path().join("parent/data"), b"same-size-before").unwrap();
        fs::set_permissions(
            tmp.path().join("parent/data"),
            fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        let snapshot = scope
            .observe_destinations(&[location(tmp.path(), "parent/data")])
            .unwrap();
        mutate(tmp.path());
        assert!(snapshot.verify_unchanged().is_err());
    }
    #[test]
    fn d07_same_length_bytes_restored_mtime() {
        changed(|path| {
            let path = path.join("parent/data");
            let before = fs::metadata(&path).unwrap();
            assert_eq!(b"same-size-before".len(), b"same-size-change".len());
            fs::write(&path, b"same-size-change").unwrap();
            fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_times(fs::FileTimes::new().set_modified(before.modified().unwrap()))
                .unwrap();
        });
    }
    #[test]
    fn d07_mode_change() {
        changed(|path| {
            let path = path.join("parent/data");
            let before = fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
            assert_eq!(before, 0o640);
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            let after = fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
            assert_eq!(after, 0o600);
            assert_ne!(before, after);
        });
    }
    #[test]
    fn d07_identical_bytes_atomic_file_replacement() {
        changed(|path| {
            fs::write(path.join("replacement"), b"same-size-before").unwrap();
            fs::rename(path.join("replacement"), path.join("parent/data")).unwrap();
        });
    }
    #[test]
    fn d07_directory_identity_independent_of_final_file_identity() {
        changed(|path| {
            fs::rename(path.join("parent"), path.join("moved")).unwrap();
            fs::create_dir(path.join("parent")).unwrap();
            // Preserve the exact final file inode: only directory evidence distinguishes this chain.
            fs::rename(path.join("moved/data"), path.join("parent/data")).unwrap();
        });
    }
    #[test]
    fn d07_directory_identity_with_unchanged_absence() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("parent")).unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        let snapshot = scope
            .observe_destinations(&[location(tmp.path(), "parent/missing")])
            .unwrap();
        fs::rename(tmp.path().join("parent"), tmp.path().join("moved")).unwrap();
        fs::create_dir(tmp.path().join("parent")).unwrap();
        assert!(
            snapshot.verify_unchanged().is_err(),
            "replaced directory accepted with unchanged missing suffix"
        );
    }
    #[test]
    fn d07_absence_transitions() {
        use std::os::unix::fs::symlink;
        for kind in 0..3 {
            let tmp = tempfile::tempdir().unwrap();
            let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
            let path = if kind == 2 {
                "missing"
            } else {
                "missing/parent/file"
            };
            let snapshot = scope
                .observe_destinations(&[location(tmp.path(), path)])
                .unwrap();
            match kind {
                0 => fs::create_dir(tmp.path().join("missing")).unwrap(),
                1 => symlink("elsewhere", tmp.path().join("missing")).unwrap(),
                _ => fs::write(tmp.path().join("missing"), b"present").unwrap(),
            }
            assert!(snapshot.verify_unchanged().is_err(), "transition {kind}");
        }
    }
    #[test]
    fn d07_root_replacement_empty_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("root");
        fs::create_dir(&path).unwrap();
        let scope = QualifiedScope::open(&root(&path)).unwrap();
        let snapshot = scope.observe_destinations(&[]).unwrap();
        fs::rename(&path, tmp.path().join("moved")).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(snapshot.verify_unchanged().is_err());
    }
    #[test]
    fn d07_additional_root_alias_retargeting() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        for name in ["real", "other"] {
            fs::create_dir(tmp.path().join(name)).unwrap();
            fs::create_dir(tmp.path().join(name).join("nested")).unwrap();
        }
        symlink("real", tmp.path().join("alias")).unwrap();
        let direct = tmp.path().join("real/nested");
        let alias = tmp.path().join("alias/nested");
        let scope = QualifiedScope::open(&root(&direct)).unwrap();
        let snapshot = scope
            .observe_destinations(&[location(&alias, "missing")])
            .unwrap();
        fs::remove_file(tmp.path().join("alias")).unwrap();
        symlink("other", tmp.path().join("alias")).unwrap();
        assert!(snapshot.verify_unchanged().is_err());
    }
    #[test]
    fn d07_sibling_edits_do_not_invalidate_directory_timestamps() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("a")).unwrap();
        fs::write(tmp.path().join("a/data"), b"same").unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        let snapshot = scope
            .observe_destinations(&[location(tmp.path(), "a/data")])
            .unwrap();
        fs::write(tmp.path().join("a/sibling"), b"unrelated").unwrap();
        snapshot.verify_unchanged().unwrap();
    }
    #[test]
    fn d08_failure_cleanup_and_d09_no_implicit_lock() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("data"), b"same").unwrap();
        fs::create_dir(tmp.path().join("directory")).unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let snapshot = scope
            .observe_destinations(&[location(tmp.path(), "data")])
            .unwrap();
        snapshot.verify_unchanged().unwrap();
        assert!(
            scope
                .try_lock_writer()
                .unwrap_err()
                .to_string()
                .contains("busy")
        );
        assert!(
            scope
                .observe_destinations(&[
                    location(tmp.path(), "data"),
                    location(tmp.path(), "directory")
                ])
                .is_err()
        );
        drop(guard);
        drop(scope.try_lock_writer().unwrap());
        // Holding a snapshot alone does not reserve the writer lock.
        drop(scope.try_lock_writer().unwrap());
        snapshot.verify_unchanged().unwrap();
    }
    #[test]
    fn d08_permission_failure_is_not_absence() {
        let tmp = tempfile::tempdir().unwrap();
        let blocked = tmp.path().join("blocked");
        fs::create_dir(&blocked).unwrap();
        let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();
        let denied = fs::read_dir(&blocked).is_err();
        let result = scope.observe_destinations(&[location(tmp.path(), "blocked/missing")]);
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o700)).unwrap();
        if denied {
            assert!(
                result.is_err(),
                "permission-denied component became absence"
            );
        } else {
            assert!(
                result.is_ok(),
                "elevated execution cannot establish the permission-denied case"
            );
        }
    }
    #[test]
    fn d06_native_root_when_supported() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(OsString::from_vec(vec![b'r', 0xff]));
        match fs::create_dir(&path) {
            Ok(()) => {
                let scope = QualifiedScope::open(&root(&path)).unwrap();
                let loc = location(&path, "missing");
                let snapshot = scope
                    .observe_destinations(std::slice::from_ref(&loc))
                    .unwrap();
                assert_eq!(snapshot.observations()[0].location, loc);
                snapshot.verify_unchanged().unwrap();
            }
            Err(error) => {
                #[cfg(target_os = "macos")]
                assert_eq!(
                    error.raw_os_error(),
                    Some(92),
                    "unexpected native-root fixture refusal"
                );
                #[cfg(target_os = "linux")]
                panic!("unexpected native-root fixture refusal: {error}");
            }
        }
    }
}
