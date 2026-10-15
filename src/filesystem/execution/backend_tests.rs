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
