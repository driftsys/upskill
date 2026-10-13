use super::*;
use crate::ownership::{InstallationRoot, OwnershipScope};
use std::os::unix::fs::symlink;
fn fixture() -> (tempfile::TempDir, QualifiedScope, Vec<FileLocation>) {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join("a")).unwrap();
    std::fs::write(tmp.path().join("a/data"), b"unchanged").unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().to_owned(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let locations = vec![FileLocation {
        root,
        path: "a/data".into(),
    }];
    (tmp, scope, locations)
}
#[test]
fn direct_directory_open_refuses_real_symlink() {
    let (tmp, scope, _) = fixture();
    symlink("a", tmp.path().join("alias")).unwrap();
    let root = scope.reopen_directory().unwrap();
    assert!(
        open_directory(&root, "alias").is_err(),
        "directory opener followed symlink"
    );
    assert!(open_directory(&root, "a").is_ok());
}
#[test]
fn exact_bytes_comparison_independent_of_unchanged_stamp() {
    let (_tmp, scope, locations) = fixture();
    let (mut evidence, observations) = Evidence::capture(&scope, &locations).unwrap();
    let Final::Present { value, .. } = &mut evidence.paths[0].last else {
        panic!("fixture is present")
    };
    let different = FileValue {
        bytes: b"different".to_vec(),
        mode: value.mode,
    };
    assert_eq!(different.bytes.len(), value.bytes.len());
    assert!(
        verify_value(&different, value).is_err(),
        "exact-byte comparison ignored expected bytes"
    );
    verify_value(value, value).unwrap();
    // Change only private expected bytes. The actual file and both stamps stay
    // unchanged, so production verification must reach its exact-byte comparison.
    *value = different;
    assert!(
        evidence.verify(&scope, &observations).is_err(),
        "production verification omitted exact-byte comparison"
    );
}

#[test]
fn missing_first_component_insertion_is_rejected_independently() {
    let (tmp, scope, mut locations) = fixture();
    locations[0].path = "absent/parent/data".into();
    let (evidence, _) = Evidence::capture(&scope, &locations).unwrap();
    let Final::Missing { suffix } = &evidence.paths[0].last else {
        panic!("fixture must be absent")
    };
    let parent = &evidence.bindings[0].directory.file;
    verify_absence(parent, suffix).unwrap();
    std::fs::create_dir(tmp.path().join("absent")).unwrap();
    assert!(
        verify_absence(parent, suffix).is_err(),
        "inserted first missing component accepted"
    );
}

#[test]
fn non_noent_errors_never_become_absence() {
    for error in [
        Errno::ACCESS,
        Errno::NOTDIR,
        Errno::IO,
        Errno::NOSYS,
        Errno::MFILE,
        Errno::LOOP,
    ] {
        assert!(classify_absence(error).is_err(), "{error:?} became absence");
    }
    assert!(classify_absence(Errno::NOENT).unwrap().is_none());
}
#[test]
fn direct_file_open_refuses_real_symlink() {
    let (tmp, scope, _) = fixture();
    symlink("a/data", tmp.path().join("alias")).unwrap();
    let parent = scope.reopen_directory().unwrap();
    assert!(
        open_file(&parent, "alias").is_err(),
        "file opener followed symlink"
    );
    let file = open_file(&parent, "a/data").unwrap();
    assert!(
        rustix::fs::fcntl_getfl(&file)
            .unwrap()
            .contains(OFlags::NONBLOCK)
    );
}
#[test]
fn static_nonregular_classification() {
    let (tmp, scope, _) = fixture();
    std::fs::create_dir(tmp.path().join("directory")).unwrap();
    symlink("a/data", tmp.path().join("link")).unwrap();
    let parent = scope.reopen_directory().unwrap();
    for name in ["directory", "link"] {
        assert!(require_regular(&entry(&parent, name).unwrap().unwrap()).is_err());
    }
    // Pure metadata variation pins types that cannot be created without privileges.
    let mut stat = entry(&parent, "a").unwrap().unwrap();
    stat.st_nlink = 1;
    for kind in [
        FileType::Fifo,
        FileType::Socket,
        FileType::CharacterDevice,
        FileType::BlockDevice,
    ] {
        stat.st_mode = kind.as_raw_mode();
        assert!(require_regular(&stat).is_err(), "{kind:?}");
    }
}
fn cloexec(file: &File) {
    assert!(
        rustix::io::fcntl_getfd(file)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC),
        "retained descriptor lacks CLOEXEC"
    );
}
#[test]
fn every_retained_descriptor_class_is_cloexec() {
    let (_tmp, scope, locations) = fixture();
    cloexec(&scope.reopen_directory().unwrap());
    let (evidence, _) = Evidence::capture(&scope, &locations).unwrap();
    cloexec(&evidence.root.file);
    for binding in &evidence.bindings {
        cloexec(&binding.directory.file);
        cloexec(&binding.binding.reopen_directory().unwrap());
    }
    for path in &evidence.paths {
        for directory in &path.directories {
            cloexec(&directory.file);
        }
        if let Final::Present { file, .. } = &path.last {
            cloexec(file);
        }
    }
}

fn descriptor_count() -> usize {
    #[cfg(target_os = "linux")]
    let directory = "/proc/self/fd";
    #[cfg(target_os = "macos")]
    let directory = "/dev/fd";
    std::fs::read_dir(directory).unwrap().count()
}
#[test]
fn failed_capture_and_snapshot_drop_close_owned_descriptors() {
    const CHILD: &str = "UPSKILL_DESTINATION_DESCRIPTOR_CHILD";
    if std::env::var_os(CHILD).is_none() {
        // Isolate descriptor counts from unrelated parallel library tests.
        let home = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact","filesystem::destinations::backend::tests::failed_capture_and_snapshot_drop_close_owned_descriptors","--nocapture"])
                .env(CHILD,"1").env("HOME",home.path()).env("USERPROFILE",home.path())
                .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                let output = child.wait_with_output().unwrap();
                assert!(
                    status.success(),
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                return;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("descriptor fixture timed out");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    let (_tmp, scope, locations) = fixture();
    let before = descriptor_count();
    for _ in 0..8 {
        let (evidence, _) = Evidence::capture(&scope, &locations).unwrap();
        assert!(
            descriptor_count() >= before + 5,
            "positive control must count retained descriptors"
        );
        drop(evidence);
        assert_eq!(
            descriptor_count(),
            before,
            "snapshot leaked retained descriptions"
        );
        let mut invalid = locations.clone();
        invalid.push(FileLocation {
            root: locations[0].root.clone(),
            path: "a".into(),
        });
        assert!(Evidence::capture(&scope, &invalid).is_err());
        assert_eq!(
            descriptor_count(),
            before,
            "failed capture leaked descriptions"
        );
    }
}
