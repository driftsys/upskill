use super::*;
use crate::files::plan::FileValue;
use crate::ownership::{InstallationRoot, OwnershipScope};
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
fn fixture() -> (tempfile::TempDir, QualifiedScope, PlannedFile) {
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().into(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let action = PlannedFile {
        location: FileLocation {
            root,
            path: "nested/a".into(),
        },
        before: None,
        after: Some(FileValue {
            bytes: b"new".to_vec(),
            mode: Some(0o600),
        }),
    };
    (tmp, scope, action)
}
#[test]
fn replacement_preserves_selected_mode() {
    let (tmp, scope, mut action) = fixture();
    apply(&scope, &action, 0).unwrap();
    let path = tmp.path().join("nested/a");
    assert_eq!(std::fs::read(&path).unwrap(), b"new");
    assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o7777, 0o600);
    action.before = action.after.clone();
    action.after.as_mut().unwrap().bytes = b"changed".to_vec();
    apply(&scope, &action, 0).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"changed");
}
#[test]
fn removal_retains_extra_files_and_directory() {
    let (tmp, scope, mut action) = fixture();
    apply(&scope, &action, 0).unwrap();
    std::fs::write(tmp.path().join("nested/extra"), b"manual").unwrap();
    action.before = action.after.take();
    apply(&scope, &action, 0).unwrap();
    assert!(!tmp.path().join("nested/a").exists());
    assert_eq!(
        std::fs::read(tmp.path().join("nested/extra")).unwrap(),
        b"manual"
    );
}
#[test]
fn noop_preserves_identity() {
    let (tmp, scope, mut action) = fixture();
    apply(&scope, &action, 0).unwrap();
    let ino = std::fs::metadata(tmp.path().join("nested/a"))
        .unwrap()
        .ino();
    action.before = action.after.clone();
    apply(&scope, &action, 0).unwrap();
    assert_eq!(
        std::fs::metadata(tmp.path().join("nested/a"))
            .unwrap()
            .ino(),
        ino
    );
}
#[test]
fn parent_symlink_or_changed_destination_rejects() {
    let (tmp, scope, action) = fixture();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), tmp.path().join("nested")).unwrap();
    assert!(apply(&scope, &action, 0).is_err());
    assert!(!outside.path().join("a").exists());
    std::fs::remove_file(tmp.path().join("nested")).unwrap();
    std::fs::create_dir(tmp.path().join("nested")).unwrap();
    std::fs::write(tmp.path().join("nested/a"), b"manual").unwrap();
    std::fs::set_permissions(
        tmp.path().join("nested/a"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(apply(&scope, &action, 0).is_err());
    assert_eq!(
        std::fs::read(tmp.path().join("nested/a")).unwrap(),
        b"manual"
    );
}
#[test]
fn existing_staging_collision_is_preserved() {
    let (tmp, scope, action) = fixture();
    std::fs::create_dir(tmp.path().join("nested")).unwrap();
    let path = tmp.path().join("nested/.upskill-stage-known");
    std::fs::write(&path, b"manual").unwrap();
    assert!(apply_named(&scope, &action, "known", 0).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"manual");
}
#[test]
fn extended_metadata_is_rejected_before_replacement() {
    let (tmp, scope, mut action) = fixture();
    apply(&scope, &action, 0).unwrap();
    let path = tmp.path().join("nested/a");
    let file = std::fs::File::open(&path).unwrap();
    #[cfg(target_os = "linux")]
    let name = "user.upskill-test";
    #[cfg(target_os = "macos")]
    let name = "upskill-test";
    rustix::fs::fsetxattr(&file, name, b"manual", rustix::fs::XattrFlags::empty()).unwrap();
    action.before = action.after.clone();
    action.after.as_mut().unwrap().bytes = b"changed".to_vec();
    assert!(apply(&scope, &action, 0).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"new");
}
#[cfg(target_os = "macos")]
#[test]
fn acl_metadata_is_rejected_before_replacement() {
    let (tmp, scope, mut action) = fixture();
    apply(&scope, &action, 0).unwrap();
    let path = tmp.path().join("nested/a");
    assert!(
        std::process::Command::new("/bin/chmod")
            .args(["+a", "everyone allow read"])
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    action.before = action.after.clone();
    action.before.as_mut().unwrap().mode = Some(std::fs::metadata(&path).unwrap().mode() & 0o7777);
    action.after = action.before.clone();
    action.after.as_mut().unwrap().bytes = b"changed".to_vec();
    assert!(apply(&scope, &action, 0).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"new");
}

#[test]
fn new_executable_and_existing_ordinary_modes_are_exact() {
    for mode in [0o700, 0o644, 0o755] {
        let (tmp, scope, mut action) = fixture();
        if mode == 0o700 {
            action.after.as_mut().unwrap().mode = Some(mode);
        } else {
            apply(&scope, &action, 0).unwrap();
            std::fs::set_permissions(
                tmp.path().join("nested/a"),
                std::fs::Permissions::from_mode(mode),
            )
            .unwrap();
            action.before = action.after.clone();
            action.before.as_mut().unwrap().mode = Some(mode);
            action.after.as_mut().unwrap().mode = Some(mode);
            action.after.as_mut().unwrap().bytes = b"changed".to_vec();
        }
        apply(&scope, &action, 0).unwrap();
        assert_eq!(
            std::fs::metadata(tmp.path().join("nested/a"))
                .unwrap()
                .mode()
                & 0o7777,
            mode
        );
    }
}

#[test]
fn bound_parent_removed_after_validation_is_not_recreated() {
    let (tmp, scope, action) = fixture();
    std::fs::create_dir(tmp.path().join("nested")).unwrap();
    let metadata = std::fs::metadata(tmp.path().join("nested")).unwrap();
    let expected = (metadata.dev(), metadata.ino());
    let old = tmp.path().join("nested");
    let retained = tmp.path().join("retained");
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            crate::filesystem::execution::TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
        }
    }
    let _reset = Reset;
    crate::filesystem::execution::TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |label, _| {
            if label == "after_bound_parent_validation" {
                std::fs::rename(&old, &retained)?;
            }
            Ok(())
        }));
    });
    assert!(apply_create_only_bound(&scope, &action, 0, expected).is_err());
    assert!(!tmp.path().join("nested").exists());
    assert_eq!(
        std::fs::read_dir(tmp.path().join("retained"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn repeat_private_evidence_syncs_exact_file_and_parent_and_propagates_failures() {
    use crate::native_delivery::state::{EvidenceBlob, EvidenceKind, EvidenceReference};
    use crate::native_delivery::store::NativeEvidenceStore;
    use std::cell::RefCell;
    use std::rc::Rc;
    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::User,
        path: tmp.path().into(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let store = NativeEvidenceStore::initialize(&root, &scope, &guard).unwrap();
    let blob = EvidenceBlob {
        reference: EvidenceReference {
            kind: EvidenceKind::NativePrestate,
            digest: "5eadfd350ace3da982d31480c7aa2f9f5d3b9fda1838b35196e9068c83f969bc".into(),
            length: 3,
        },
        bytes: vec![0, 255, 1],
    };
    store.put(&blob).unwrap();
    let directory = tmp.path().join(".upskill/native-evidence");
    let file = directory.join(format!("native-prestate-{}", blob.reference.digest));
    let f = std::fs::metadata(&file).unwrap();
    let d = std::fs::metadata(&directory).unwrap();
    let expected = [
        ("file", (f.dev(), f.ino())),
        ("directory", (d.dev(), d.ino())),
    ];
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_SYNC.with(|hook| *hook.borrow_mut() = None);
        }
    }
    let _reset = Reset;
    for fail in [None, Some("file"), Some("directory")] {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let captured = seen.clone();
        TEST_SYNC.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |file, kind| {
                let metadata = file.metadata()?;
                captured
                    .borrow_mut()
                    .push((kind, (metadata.dev(), metadata.ino())));
                anyhow::ensure!(Some(kind) != fail, "injected private evidence sync failure");
                file.sync_all()?;
                Ok(())
            }));
        });
        let result = store.put(&blob);
        assert_eq!(result.is_err(), fail.is_some());
        let count = if fail == Some("file") { 1 } else { 2 };
        assert_eq!(&*seen.borrow(), &expected[..count]);
        assert_eq!(store.read(&blob.reference).unwrap(), Some(blob.clone()));
    }
}
