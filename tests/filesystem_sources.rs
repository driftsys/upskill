use std::fs;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::Path;
use upskill::filesystem::QualifiedScope;
use upskill::ownership::{InstallationRoot, OwnershipScope};

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn source_scope(path: &Path) -> QualifiedScope {
    QualifiedScope::open(&InstallationRoot {
        scope: OwnershipScope::Project,
        path: path.to_owned(),
    })
    .unwrap()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod native {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use upskill::source_manifest::{capture_source, validate_manifest};

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
    fn bounded_child(test: &str, key: &str) -> bool {
        if std::env::var_os(key).is_some() {
            return false;
        }
        let home = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env(key, "1")
            .env("HOME", home.path())
            .env("USERPROFILE", home.path())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                return true;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source FIFO test timed out");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    #[test]
    fn complete_stable_tree_matches_legacy_records_and_compiles_captured_values() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("assets/empty")).unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        fs::write(
            tmp.path().join("SKILL.md"),
            b"---\nschema: 1\ndescription: demo\nignore: [ignored]\n---\n\n# Demo\n",
        )
        .unwrap();
        fs::write(tmp.path().join("assets/binary"), [0, 255, 128]).unwrap();
        fs::write(tmp.path().join(".hidden"), b"hidden").unwrap();
        fs::write(tmp.path().join(".git/config"), b"git").unwrap();
        fs::write(tmp.path().join("ignored"), b"ignored").unwrap();
        fs::write(tmp.path().join("zero"), b"").unwrap();
        fs::hard_link(tmp.path().join("assets/binary"), tmp.path().join("linked")).unwrap();
        let legacy = capture_source(tmp.path()).unwrap();
        let scope = source_scope(tmp.path());
        let result = scope.observe_source_tree();
        assert!(
            result.is_ok(),
            "stable complete source capture rejected: {result:?}"
        );
        let snapshot = result.unwrap();
        assert_eq!(snapshot.manifest(), &legacy);
        assert_eq!(
            snapshot
                .manifest()
                .files
                .iter()
                .map(|f| f.path.as_str())
                .collect::<Vec<_>>(),
            [
                ".git/config",
                ".hidden",
                "SKILL.md",
                "assets/binary",
                "ignored",
                "linked",
                "zero"
            ]
        );
        assert_eq!(snapshot.manifest().files[3].bytes, [0, 255, 128]);
        assert_eq!(snapshot.manifest().files[5].bytes, [0, 255, 128]);
        validate_manifest(snapshot.manifest()).unwrap();
        snapshot.verify_unchanged().unwrap();
        snapshot.verify_unchanged().unwrap();
        let before = snapshot.manifest().clone();
        fs::remove_file(tmp.path().join("SKILL.md")).unwrap();
        let compiled = upskill::compile::item::compile_item(
            snapshot.manifest(),
            &upskill::compile::item::ItemCompileRequest {
                kind: upskill::pipeline::ItemKind::Skill,
                folder_name: "demo".into(),
                alias: None,
                clients: vec![upskill::generate::Client::Claude],
            },
        )
        .unwrap();
        assert_eq!(compiled.source_manifest_sha256, before.content_sha256);
        assert!(
            compiled.outputs[0]
                .files
                .iter()
                .any(|f| f.bytes == [0, 255, 128])
        );
        assert_eq!(snapshot.manifest(), &before);
        assert_eq!(fs::read(tmp.path().join(".hidden")).unwrap(), b"hidden");
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 6);
    }

    fn stale(label: &str, mutate: impl FnOnce(&Path)) {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("parent")).unwrap();
        fs::create_dir(tmp.path().join("empty")).unwrap();
        fs::write(tmp.path().join("parent/data"), b"original").unwrap();
        let scope = source_scope(tmp.path());
        let snapshot = scope.observe_source_tree().unwrap();
        let original = snapshot.manifest().clone();
        snapshot.verify_unchanged().unwrap();
        mutate(tmp.path());
        for call in 1..=2 {
            assert!(
                snapshot.verify_unchanged().is_err(),
                "{label} accepted on verification {call}"
            );
            assert_eq!(
                snapshot.manifest(),
                &original,
                "{label} refreshed captured manifest"
            );
        }
    }
    #[test]
    fn empty_directory_insertion_stays_stale() {
        stale("empty directory insertion", |p| {
            fs::create_dir(p.join("new-empty")).unwrap()
        });
    }
    #[test]
    fn file_addition_removal_and_rename_stay_stale() {
        stale("file insertion", |p| {
            fs::write(p.join("new"), b"new").unwrap()
        });
        stale("file removal", |p| {
            fs::remove_file(p.join("parent/data")).unwrap()
        });
        stale("file rename", |p| {
            fs::rename(p.join("parent/data"), p.join("parent/renamed")).unwrap()
        });
    }
    #[test]
    fn inode_directory_and_mode_replacements_stay_stale() {
        stale("same-byte inode replacement", |p| {
            fs::write(p.join("replacement"), b"original").unwrap();
            fs::rename(p.join("replacement"), p.join("parent/data")).unwrap();
        });
        stale("empty directory removal", |p| {
            fs::remove_dir(p.join("empty")).unwrap()
        });
        stale("same-descendant directory replacement", |p| {
            fs::rename(p.join("parent"), p.join("moved")).unwrap();
            fs::create_dir(p.join("parent")).unwrap();
            fs::write(p.join("parent/data"), b"original").unwrap();
            fs::remove_dir_all(p.join("moved")).unwrap();
        });
        stale("mode change", |p| {
            fs::set_permissions(p.join("parent/data"), fs::Permissions::from_mode(0o700)).unwrap()
        });
    }
    #[test]
    fn selected_root_and_ancestor_alias_rebinding_stay_stale() {
        for ancestor in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            fs::create_dir_all(tmp.path().join("outer/root")).unwrap();
            fs::write(tmp.path().join("outer/root/data"), b"original").unwrap();
            let scope = source_scope(&tmp.path().join("outer/root"));
            let snapshot = scope.observe_source_tree().unwrap();
            let original = snapshot.manifest().clone();
            let selected = if ancestor { "outer" } else { "outer/root" };
            fs::rename(tmp.path().join(selected), tmp.path().join("moved")).unwrap();
            if ancestor {
                symlink("moved", tmp.path().join("outer")).unwrap();
            } else {
                fs::create_dir(tmp.path().join(selected)).unwrap();
            }
            for call in 1..=2 {
                assert!(
                    snapshot.verify_unchanged().is_err(),
                    "root/ancestor rebind accepted on verification {call}"
                );
                assert_eq!(snapshot.manifest(), &original);
            }
        }
    }
    #[test]
    fn unsafe_entries_reject_without_following_or_writes() {
        if bounded_child(
            "native::unsafe_entries_reject_without_following_or_writes",
            "UPSKILL_SOURCE_UNSAFE_CHILD",
        ) {
            return;
        }
        for kind in ["symlink", "socket", "fifo", "invalid"] {
            let tmp = tempfile::tempdir().unwrap();
            fs::write(tmp.path().join("data"), b"original").unwrap();
            let entry = tmp.path().join(if kind == "invalid" {
                "bad:name"
            } else {
                "unsafe"
            });
            let _socket = match kind {
                "symlink" => {
                    symlink("data", &entry).unwrap();
                    None
                }
                "socket" => Some(std::os::unix::net::UnixListener::bind(&entry).unwrap()),
                "fifo" => {
                    fifo(&entry);
                    None
                }
                _ => {
                    fs::write(&entry, b"invalid").unwrap();
                    None
                }
            };
            let scope = source_scope(tmp.path());
            assert!(
                scope.observe_source_tree().is_err(),
                "{kind} source accepted"
            );
            assert_eq!(fs::read(tmp.path().join("data")).unwrap(), b"original");
            assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 2);
        }
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("real")).unwrap();
        symlink("real", tmp.path().join("alias")).unwrap();
        assert!(
            QualifiedScope::open(&InstallationRoot {
                scope: OwnershipScope::Project,
                path: tmp.path().join("alias")
            })
            .is_err()
        );
    }
    #[test]
    fn symlink_and_fifo_substitutions_stay_stale() {
        if bounded_child(
            "native::symlink_and_fifo_substitutions_stay_stale",
            "UPSKILL_SOURCE_SUBSTITUTION_CHILD",
        ) {
            return;
        }
        stale("symlink substitution", |p| {
            fs::remove_file(p.join("parent/data")).unwrap();
            symlink("../empty", p.join("parent/data")).unwrap();
        });
        stale("FIFO substitution", |p| {
            fs::remove_file(p.join("parent/data")).unwrap();
            fifo(&p.join("parent/data"));
        });
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn non_utf8_entry_rejects() {
        use std::os::unix::ffi::OsStringExt;
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join(std::ffi::OsString::from_vec(vec![255])),
            b"bytes",
        )
        .unwrap();
        assert!(source_scope(tmp.path()).observe_source_tree().is_err());
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[test]
fn unsupported_host_returns_error_without_artifacts() {
    let tmp = tempfile::tempdir().unwrap();
    let result = QualifiedScope::open(&InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().to_owned(),
    });
    assert!(result.is_err());
    assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    fn opaque_api(scope: &QualifiedScope) {
        assert!(scope.observe_source_tree().is_err());
    }
    let _ = opaque_api;
}
