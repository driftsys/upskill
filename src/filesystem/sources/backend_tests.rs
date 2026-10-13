use super::*;
use crate::ownership::{InstallationRoot, OwnershipScope};
use std::os::unix::fs::{PermissionsExt, symlink};

fn fixture() -> (tempfile::TempDir, QualifiedScope) {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join("nested")).unwrap();
    std::fs::write(tmp.path().join("nested/data"), b"original").unwrap();
    let scope = QualifiedScope::open(&InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().to_owned(),
    })
    .unwrap();
    (tmp, scope)
}
fn regular(evidence: &mut Evidence) -> &mut Regular {
    let Entry::Directory(nested) = &mut evidence.root.children[0] else {
        panic!("nested fixture")
    };
    let Entry::Regular(file) = &mut nested.children[0] else {
        panic!("regular fixture")
    };
    file
}
fn native_read(file: &File, bytes: &mut Vec<u8>) -> io::Result<usize> {
    (&*file).read_to_end(bytes)
}
#[test]
fn metadata_change_during_real_read_rejects_equal_bytes_and_count() {
    let (tmp, scope) = fixture();
    Evidence::capture(&scope).unwrap();
    let mut read = |file: &File, bytes: &mut Vec<u8>| {
        let count = native_read(file, bytes)?;
        assert_eq!(bytes, b"original");
        assert_eq!(count, 8);
        std::fs::set_permissions(
            tmp.path().join("nested/data"),
            std::fs::Permissions::from_mode(0o700),
        )?;
        Ok(count)
    };
    let result = Evidence::capture_with(&scope, &mut read, Mount::queries());
    assert!(
        result.is_err(),
        "acquisition ignored changed metadata with equal returned bytes"
    );
}
#[test]
fn short_read_rejects_unchanged_real_metadata() {
    let (_tmp, scope) = fixture();
    Evidence::capture(&scope).unwrap();
    let mut read = |file: &File, bytes: &mut Vec<u8>| {
        let before = Stamp::of(&fstat(file).unwrap());
        bytes.extend_from_slice(b"origina");
        assert_eq!(before, Stamp::of(&fstat(file).unwrap()));
        Ok(bytes.len())
    };
    assert!(
        Evidence::capture_with(&scope, &mut read, Mount::queries()).is_err(),
        "acquisition ignored short read with unchanged metadata"
    );
}
#[test]
fn wrong_reported_count_rejects_equal_bytes_and_metadata() {
    let (_tmp, scope) = fixture();
    let mut read =
        |file: &File, bytes: &mut Vec<u8>| native_read(file, bytes).map(|count| count - 1);
    assert!(
        Evidence::capture_with(&scope, &mut read, Mount::queries()).is_err(),
        "acquisition ignored inconsistent reported byte count"
    );
}
#[test]
fn selected_stamp_mismatch_rejects_before_read() {
    let (tmp, scope) = fixture();
    let file = File::open(tmp.path().join("nested/data")).unwrap();
    let stamp = Stamp::of(&fstat(&file).unwrap());
    capture_bytes(&file, &stamp, &mut native_read).unwrap();
    let mut wrong = stamp.clone();
    wrong.modified.1 ^= 1;
    use std::io::{Seek, SeekFrom};
    (&file).seek(SeekFrom::Start(0)).unwrap();
    let mut calls = 0;
    let mut read = |file: &File, bytes: &mut Vec<u8>| {
        calls += 1;
        native_read(file, bytes)
    };
    let result = capture_bytes(&file, &wrong, &mut read);
    assert!(
        result.is_err(),
        "acquisition ignored selected stamp before read"
    );
    assert_eq!(
        calls, 0,
        "acquisition read before validating immediate metadata"
    );
    drop(scope);
}
#[test]
fn read_error_retains_operation_path_and_original_cause() {
    let (_tmp, scope) = fixture();
    let mut read = |_file: &File, _bytes: &mut Vec<u8>| {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "source-read-sentinel",
        ))
    };
    let result = Evidence::capture_with(&scope, &mut read, Mount::queries());
    assert!(result.is_err(), "acquisition bypassed configured reader");
    let error = result.err().unwrap();
    let message = format!("{error:#}");
    assert!(message.contains("read source file bytes"), "{message}");
    assert!(message.contains("nested/data"), "{message}");
    assert!(message.contains("source-read-sentinel"), "{message}");
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::PermissionDenied
    );
}
#[test]
fn values_compare_independently_of_metadata() {
    let (_tmp, scope) = fixture();
    let (mut evidence, _) = Evidence::capture(&scope).unwrap();
    evidence.verify(&scope).unwrap();
    regular(&mut evidence).record.bytes = b"different".to_vec();
    assert!(
        evidence.verify(&scope).is_err(),
        "verification omitted exact source value comparison"
    );
}
#[test]
fn membership_compare_independently_of_directory_stamps() {
    let (tmp, scope) = fixture();
    let (mut evidence, _) = Evidence::capture(&scope).unwrap();
    std::fs::create_dir(tmp.path().join("new-empty")).unwrap();
    let (fresh, _) = Evidence::capture(&scope).unwrap();
    // Empty-directory insertion changes only root metadata and membership.
    // Normalize competing metadata; membership must reject the new empty child.
    evidence.root.stamp = fresh.root.stamp.clone();
    assert_eq!(evidence.root.names, ["nested"]);
    assert_eq!(fresh.root.names, ["nested", "new-empty"]);
    assert!(
        evidence.root.compare(&fresh.root, "").is_err(),
        "verification omitted complete membership comparison"
    );
}
#[test]
fn same_byte_inode_replacement_compares_identity_independently_of_other_stamps() {
    let (tmp, scope) = fixture();
    let (mut evidence, _) = Evidence::capture(&scope).unwrap();
    std::fs::write(tmp.path().join("replacement"), b"original").unwrap();
    std::fs::rename(
        tmp.path().join("replacement"),
        tmp.path().join("nested/data"),
    )
    .unwrap();
    let (mut fresh, _) = Evidence::capture(&scope).unwrap();
    evidence.root.stamp = fresh.root.stamp.clone();
    let Entry::Directory(old_dir) = &mut evidence.root.children[0] else {
        panic!("directory")
    };
    let Entry::Directory(new_dir) = &fresh.root.children[0] else {
        panic!("directory")
    };
    old_dir.stamp = new_dir.stamp.clone();
    let old = regular(&mut evidence);
    let new = regular(&mut fresh);
    let original_identity = old.stamp.identity;
    assert_ne!(original_identity, new.stamp.identity);
    assert_eq!(old.record, new.record);
    old.stamp = new.stamp.clone();
    old.stamp.identity = original_identity;
    assert!(
        evidence.root.compare(&fresh.root, "").is_err(),
        "verification ignored independent file identity"
    );
}
#[test]
fn same_second_fractional_mtime_compares_independently_of_ctime() {
    let (tmp, scope) = fixture();
    let (mut evidence, _) = Evidence::capture(&scope).unwrap();
    let regular = regular(&mut evidence);
    let before = Stamp::of(&fstat(&regular.file).unwrap());
    let nanos = if before.modified.1 < 900_000_000 {
        before.modified.1 + 1_000_000
    } else {
        before.modified.1 - 1_000_000
    };
    let modified =
        std::time::UNIX_EPOCH + std::time::Duration::new(before.modified.0 as u64, nanos as u32);
    File::options()
        .write(true)
        .open(tmp.path().join("nested/data"))
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let after = Stamp::of(&fstat(&regular.file).unwrap());
    assert_eq!(after.modified.0, before.modified.0);
    assert_ne!(after.modified.1, before.modified.1);
    assert_ne!(after.modified.1, 0);
    regular.stamp.changed = after.changed;
    let mut normalized = before;
    normalized.changed = after.changed;
    normalized.modified = after.modified;
    assert_eq!(
        normalized, after,
        "mtime must be the sole remaining metadata difference"
    );
    assert!(
        evidence.verify(&scope).is_err(),
        "verification ignored fractional mtime"
    );
}
fn cloexec(file: &File) {
    assert!(
        rustix::io::fcntl_getfd(file)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC),
        "source descriptor lacks CLOEXEC"
    );
}
#[test]
fn retained_descriptors_have_flags_and_opens_refuse_symlinks() {
    let (tmp, scope) = fixture();
    let (mut evidence, _) = Evidence::capture(&scope).unwrap();
    cloexec(&evidence.root.file);
    let Entry::Directory(directory) = &evidence.root.children[0] else {
        panic!("directory")
    };
    cloexec(&directory.file);
    let file = regular(&mut evidence);
    cloexec(&file.file);
    assert!(
        rustix::fs::fcntl_getfl(&file.file)
            .unwrap()
            .contains(OFlags::NONBLOCK),
        "source regular descriptor lacks NONBLOCK"
    );
    symlink("nested", tmp.path().join("directory-alias")).unwrap();
    symlink("nested/data", tmp.path().join("file-alias")).unwrap();
    assert!(
        open_entry(&evidence.root.file, "directory-alias", true).is_err(),
        "directory open followed symlink"
    );
    assert!(
        open_entry(&evidence.root.file, "file-alias", false).is_err(),
        "regular open followed symlink"
    );
}
#[test]
fn real_native_iterator_stat_and_open_errors_retain_context() {
    let (tmp, scope) = fixture();
    let file = File::open(tmp.path().join("nested/data")).unwrap();
    assert!(
        format!("{:#}", names(&file, "nested/data").unwrap_err())
            .contains("enumerate source directory")
    );
    let parent = scope.reopen_directory().unwrap();
    let stat_error = select_entry(&parent, "missing").unwrap_err();
    assert!(format!("{stat_error:#}").contains("inspect source component"));
    assert!(stat_error.downcast_ref::<rustix::io::Errno>().is_some());
    let open_error = open_entry(&parent, "missing", false).unwrap_err();
    assert!(format!("{open_error:#}").contains("open source component"));
    assert!(open_error.downcast_ref::<rustix::io::Errno>().is_some());
}
#[test]
fn failed_capture_releases_owned_descriptors() {
    use std::os::fd::AsRawFd;
    if bounded_child(
        "filesystem::sources::backend::tests::failed_capture_releases_owned_descriptors",
        "UPSKILL_SOURCE_RELEASE_CHILD",
    ) {
        return;
    }
    let (_tmp, scope) = fixture();
    fn descriptors() -> Vec<std::ffi::OsString> {
        let mut names = std::fs::read_dir("/dev/fd")
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        names.sort();
        names
    }
    let before = descriptors();
    let mut observed = Vec::new();
    let mut read = |file: &File, _bytes: &mut Vec<u8>| {
        observed.push(file.as_raw_fd());
        Err(io::Error::other("release sentinel"))
    };
    assert!(Evidence::capture_with(&scope, &mut read, Mount::queries()).is_err());
    // The isolated child runs one test, so no sibling test can reuse the number.
    assert_eq!(
        descriptors(),
        before,
        "failed capture retained directory or file descriptors"
    );
    for fd in observed {
        assert!(
            File::open(format!("/dev/fd/{fd}")).is_err(),
            "failed capture retained owned file handle"
        );
    }
}
#[test]
fn changed_mount_binding_rejects_source_revalidation() {
    let (_tmp, scope) = fixture();
    let (mut evidence, _) = Evidence::capture(&scope).unwrap();
    evidence.verify(&scope).unwrap();
    evidence.mount = crate::filesystem::qualification::tests::different_binding(&evidence.mount);
    assert!(
        evidence.verify(&scope).is_err(),
        "source verification ignored changed mount binding"
    );
}
fn unknown_profile(file: &File) -> rustix::io::Result<rustix::fs::StatFs> {
    let mut stat = rustix::fs::fstatfs(file)?;
    #[cfg(target_os = "macos")]
    {
        stat.f_fstypename.fill(0);
        stat.f_fstypename[0] = b'x' as std::ffi::c_char;
    }
    #[cfg(target_os = "linux")]
    {
        stat.f_type = 0x794c7630;
    }
    Ok(stat)
}
fn unknown_queries() -> DescriptorQueries {
    #[cfg(target_os = "macos")]
    {
        unknown_profile
    }
    #[cfg(target_os = "linux")]
    {
        (unknown_profile, Mount::queries().1)
    }
}
#[test]
fn source_acquisition_and_verify_reject_unknown_native_profile() {
    let (_tmp, scope) = fixture();
    let (evidence, _) = Evidence::capture(&scope).unwrap();
    assert!(
        Evidence::capture_with(&scope, &mut native_read, unknown_queries()).is_err(),
        "source acquisition bypassed native qualification"
    );
    assert!(
        evidence.verify_with(&scope, unknown_queries()).is_err(),
        "source verification bypassed native qualification"
    );
}
#[cfg(target_os = "linux")]
#[test]
fn source_acquisition_and_verify_reject_missing_native_mount_mask() {
    let (_tmp, scope) = fixture();
    let (evidence, _) = Evidence::capture(&scope).unwrap();
    let missing = |file: &File| {
        let mut stat = Mount::queries().1(file)?;
        stat.stx_mask &= !rustix::fs::StatxFlags::MNT_ID.bits();
        Ok(stat)
    };
    let queries = (Mount::queries().0, missing as fn(&File) -> _);
    assert!(
        Evidence::capture_with(&scope, &mut native_read, queries).is_err(),
        "source accepted missing native mount mask"
    );
    assert!(
        evidence.verify_with(&scope, queries).is_err(),
        "source verification accepted missing native mount mask"
    );
}
#[test]
fn directory_change_during_descendant_read_rejects() {
    let (tmp, scope) = fixture();
    let mut read = |file: &File, bytes: &mut Vec<u8>| {
        let count = native_read(file, bytes)?;
        std::fs::create_dir(tmp.path().join("nested/empty"))?;
        Ok(count)
    };
    assert!(
        Evidence::capture_with(&scope, &mut read, Mount::queries()).is_err(),
        "acquisition ignored directory change during descendant read"
    );
}
#[test]
fn repeated_directory_identity_rejects_before_descent() {
    let (_tmp, scope) = fixture();
    let file = scope.reopen_directory().unwrap();
    let identity = Identity::of(&fstat(&file).unwrap());
    let mount = Mount::of(&file, Mount::queries()).unwrap();
    let mut seen = HashSet::from([identity]);
    assert!(
        Directory::capture(
            file,
            "",
            &mount,
            &mut seen,
            &mut native_read,
            Mount::queries()
        )
        .is_err(),
        "acquisition accepted repeated directory identity"
    );
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
            assert!(status.success(), "owned source subprocess failed");
            return true;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("owned source subprocess timed out");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
#[test]
fn nonblocking_open_rejects_fifo_replacement_without_reading() {
    if bounded_child(
        "filesystem::sources::backend::tests::nonblocking_open_rejects_fifo_replacement_without_reading",
        "UPSKILL_SOURCE_FIFO_OPEN_CHILD",
    ) {
        return;
    }
    let (tmp, scope) = fixture();
    #[cfg(target_os = "linux")]
    rustix::fs::mkfifoat(rustix::fs::CWD, tmp.path().join("fifo"), Mode::RUSR).unwrap();
    #[cfg(target_os = "macos")]
    {
        let mut child = std::process::Command::new("/usr/bin/mkfifo")
            .arg(tmp.path().join("fifo"))
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
                panic!("mkfifo timed out");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    let parent = scope.reopen_directory().unwrap();
    let file = open_entry(&parent, "fifo", false).unwrap();
    assert!(
        require_kind(&fstat(&file).unwrap(), false).is_err(),
        "opened FIFO replacement accepted as regular"
    );
}

#[test]
fn opened_identity_and_stamp_match_inspected_entry_before_read() {
    let (tmp, scope) = fixture();
    let parent = File::open(tmp.path().join("nested")).unwrap();
    let mount = Mount::of(&parent, Mount::queries()).unwrap();
    let selected = select_entry(&parent, "data").unwrap();
    open_selected(&parent, "data", &selected, false, &mount, Mount::queries()).unwrap();
    let mut identity_only = selected;
    identity_only.st_ino ^= 1;
    assert!(
        open_selected(
            &parent,
            "data",
            &identity_only,
            false,
            &mount,
            Mount::queries()
        )
        .is_err(),
        "opened descriptor ignored selected identity"
    );
    let mut stamp_only = selected;
    stamp_only.st_mtime_nsec ^= 1;
    assert!(
        open_selected(
            &parent,
            "data",
            &stamp_only,
            false,
            &mount,
            Mount::queries()
        )
        .is_err(),
        "opened descriptor ignored selected metadata"
    );
    drop(scope);
}
#[test]
fn source_acquisition_rejects_changed_mount_binding() {
    let (_tmp, scope) = fixture();
    let file = scope.reopen_directory().unwrap();
    let mount = Mount::of(&file, Mount::queries()).unwrap();
    let different = crate::filesystem::qualification::tests::different_binding(&mount);
    assert!(
        Directory::capture(
            file,
            "",
            &different,
            &mut HashSet::new(),
            &mut native_read,
            Mount::queries()
        )
        .is_err(),
        "source acquisition ignored changed mount binding"
    );
}
#[test]
fn ancestor_rebind_during_capture_rejects_after_walk() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("outer/root")).unwrap();
    std::fs::write(tmp.path().join("outer/root/data"), b"original").unwrap();
    let scope = QualifiedScope::open(&InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().join("outer/root"),
    })
    .unwrap();
    let mut read = |file: &File, bytes: &mut Vec<u8>| {
        let count = native_read(file, bytes)?;
        std::fs::rename(tmp.path().join("outer"), tmp.path().join("moved"))?;
        std::os::unix::fs::symlink("moved", tmp.path().join("outer"))?;
        Ok(count)
    };
    assert!(
        Evidence::capture_with(&scope, &mut read, Mount::queries()).is_err(),
        "capture omitted final source root binding check"
    );
}

#[test]
fn ancestor_rebind_after_fresh_comparison_rejects_final_revalidation() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("outer/root")).unwrap();
    std::fs::write(tmp.path().join("outer/root/data"), b"original").unwrap();
    let scope = QualifiedScope::open(&InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().join("outer/root"),
    })
    .unwrap();
    let (evidence, manifest) = Evidence::capture(&scope).unwrap();
    evidence.verify(&scope).unwrap();
    let result = evidence.verify_with_before_binding(&scope, Mount::queries(), || {
        std::fs::rename(tmp.path().join("outer"), tmp.path().join("moved"))?;
        symlink("moved", tmp.path().join("outer"))?;
        Ok(())
    });
    assert!(
        result.is_err(),
        "revalidation omitted final source root binding check after fresh comparison"
    );
    let error = format!("{:#}", result.unwrap_err());
    assert!(
        error.contains("verify source root after revalidation")
            && error.contains("scope root binding changed"),
        "final source binding failure lost operation/root context or original cause: {error}"
    );
    assert!(evidence.verify(&scope).is_err());
    let mut records = Vec::new();
    evidence.root.records(&mut records);
    assert_eq!(records, manifest.files);
    assert_eq!(
        std::fs::read(tmp.path().join("moved/root/data")).unwrap(),
        b"original"
    );
}

#[test]
fn native_iterator_failure_rejects_without_partial_names() {
    let (_tmp, scope) = fixture();
    let file = scope.reopen_directory().unwrap();
    let real = Dir::read_from(&file).unwrap();
    let iterator = real.map(|entry| {
        let entry = entry?;
        if entry.file_name().to_bytes() == b"nested" {
            Err(rustix::io::Errno::IO)
        } else {
            Ok(entry)
        }
    });
    let result = collect_names(iterator, "source-root");
    assert!(
        result.is_err(),
        "source iterator error was silently ignored"
    );
    let error = result.unwrap_err();
    let message = format!("{error:#}");
    assert!(
        message.contains("read source directory entry in"),
        "{message}"
    );
    assert!(message.contains("source-root"), "{message}");
    assert_eq!(
        error.downcast_ref::<rustix::io::Errno>(),
        Some(&rustix::io::Errno::IO)
    );
}

#[test]
fn selection_keeps_symlink_metadata_without_following() {
    let (tmp, scope) = fixture();
    symlink("nested/data", tmp.path().join("alias")).unwrap();
    let parent = scope.reopen_directory().unwrap();
    let stat = select_entry(&parent, "alias").unwrap();
    assert_eq!(
        FileType::from_raw_mode(stat.st_mode),
        FileType::Symlink,
        "source selection followed a symlink"
    );
    assert_eq!(
        std::fs::read(tmp.path().join("nested/data")).unwrap(),
        b"original"
    );
}
#[test]
fn selected_kind_must_match_opened_kind() {
    let (tmp, scope) = fixture();
    let parent = scope.reopen_directory().unwrap();
    let mount = Mount::of(&parent, Mount::queries()).unwrap();
    let stat = select_entry(&parent, "nested").unwrap();
    assert!(
        open_selected(&parent, "nested", &stat, false, &mount, Mount::queries()).is_err(),
        "opened source kind check was omitted"
    );
    assert!(tmp.path().join("nested/data").is_file());
}

#[test]
fn same_descendant_directory_replacement_compares_independent_identity() {
    let (tmp, scope) = fixture();
    let (mut evidence, _) = Evidence::capture(&scope).unwrap();
    std::fs::rename(tmp.path().join("nested"), tmp.path().join("moved")).unwrap();
    std::fs::create_dir(tmp.path().join("nested")).unwrap();
    std::fs::rename(
        tmp.path().join("moved/data"),
        tmp.path().join("nested/data"),
    )
    .unwrap();
    std::fs::remove_dir(tmp.path().join("moved")).unwrap();
    let (mut fresh, _) = Evidence::capture(&scope).unwrap();
    evidence.root.stamp = fresh.root.stamp.clone();
    let Entry::Directory(old) = &mut evidence.root.children[0] else {
        panic!("directory")
    };
    let Entry::Directory(new) = &fresh.root.children[0] else {
        panic!("directory")
    };
    let old_identity = old.stamp.identity;
    assert_ne!(old_identity, new.stamp.identity);
    old.stamp = new.stamp.clone();
    old.stamp.identity = old_identity;
    let old_file = regular(&mut evidence);
    let new_file = regular(&mut fresh);
    assert_eq!(old_file.stamp.identity, new_file.stamp.identity);
    assert_eq!(old_file.record, new_file.record);
    old_file.stamp = new_file.stamp.clone();
    assert!(
        evidence.root.compare(&fresh.root, "").is_err(),
        "verification ignored independent directory identity"
    );
}

fn unknown_regular_profile(file: &File) -> rustix::io::Result<rustix::fs::StatFs> {
    if FileType::from_raw_mode(fstat(file)?.st_mode) == FileType::RegularFile {
        unknown_profile(file)
    } else {
        rustix::fs::fstatfs(file)
    }
}
fn unknown_regular_queries() -> DescriptorQueries {
    #[cfg(target_os = "macos")]
    {
        unknown_regular_profile
    }
    #[cfg(target_os = "linux")]
    {
        (unknown_regular_profile, Mount::queries().1)
    }
}
fn changed_regular_binding_queries() -> DescriptorQueries {
    #[cfg(target_os = "macos")]
    {
        |file| {
            let mut stat = rustix::fs::fstatfs(file)?;
            if FileType::from_raw_mode(fstat(file)?.st_mode) == FileType::RegularFile {
                stat.f_mntonname[0] ^= 1;
            }
            Ok(stat)
        }
    }
    #[cfg(target_os = "linux")]
    {
        (Mount::queries().0, |file| {
            let mut stat = Mount::queries().1(file)?;
            if FileType::from_raw_mode(fstat(file)?.st_mode) == FileType::RegularFile {
                stat.stx_mnt_id ^= 1;
            }
            Ok(stat)
        })
    }
}
fn reject_regular_before_read(queries: DescriptorQueries, assertion: &str) {
    let (tmp, scope) = fixture();
    Evidence::capture(&scope).unwrap();
    let root = scope.reopen_directory().unwrap();
    let mount = Mount::of(&root, queries).unwrap();
    let nested = open_entry(&root, "nested", true).unwrap();
    mount
        .require_same(&Mount::of(&nested, queries).unwrap())
        .unwrap();
    let regular = open_entry(&nested, "data", false).unwrap();
    assert!(
        Mount::of(&regular, queries)
            .and_then(|actual| mount.require_same(&actual))
            .is_err(),
        "fixture changed no regular-file qualification evidence"
    );
    let mut reads = 0;
    let mut read = |file: &File, bytes: &mut Vec<u8>| {
        reads += 1;
        native_read(file, bytes)
    };
    let result = Evidence::capture_with(&scope, &mut read, queries);
    assert_eq!(reads, 0, "{assertion}");
    assert!(
        result.is_err(),
        "unqualified regular descendant capture succeeded"
    );
    assert_eq!(
        std::fs::read(tmp.path().join("nested/data")).unwrap(),
        b"original"
    );
}
#[test]
fn descendant_profile_rejects_before_regular_reader_runs() {
    reject_regular_before_read(
        unknown_regular_queries(),
        "descendant profile qualification was bypassed before reading",
    );
}
#[test]
fn descendant_mount_binding_rejects_before_regular_reader_runs() {
    reject_regular_before_read(
        changed_regular_binding_queries(),
        "descendant mount comparison was bypassed before reading",
    );
}

#[test]
fn retained_root_revalidation_reports_operation_and_original_cause() {
    let (tmp, scope) = fixture();
    let (evidence, manifest) = Evidence::capture(&scope).unwrap();
    std::fs::create_dir(tmp.path().join("new-empty")).unwrap();
    for _ in 0..2 {
        let error = evidence.verify(&scope).unwrap_err();
        assert!(
            format!("{error:#}").contains("revalidate retained source \".\""),
            "retained root diagnostic lost operation and source-root context: {error:#}"
        );
        assert_eq!(error.root_cause().to_string(), "source metadata changed");
        let mut files = Vec::new();
        evidence.root.records(&mut files);
        assert_eq!(assemble_manifest(files).unwrap(), manifest);
    }
}
#[test]
fn complete_root_comparison_reports_operation_and_original_cause() {
    let (tmp, scope) = fixture();
    let (mut evidence, _) = Evidence::capture(&scope).unwrap();
    std::fs::create_dir(tmp.path().join("new-empty")).unwrap();
    let (fresh, _) = Evidence::capture(&scope).unwrap();
    evidence.root.stamp = fresh.root.stamp.clone();
    let error = evidence.root.compare(&fresh.root, "").unwrap_err();
    assert!(
        format!("{error:#}").contains("compare source \".\""),
        "complete root comparison lost operation and source-root context: {error:#}"
    );
    assert_eq!(
        error.root_cause().to_string(),
        "source directory membership changed"
    );
}
#[test]
fn complete_nested_comparison_reports_relative_path_and_original_cause() {
    let (_tmp, scope) = fixture();
    let (evidence, _) = Evidence::capture(&scope).unwrap();
    let (mut fresh, _) = Evidence::capture(&scope).unwrap();
    regular(&mut fresh).record.bytes[0] ^= 1;
    let error = evidence.root.compare(&fresh.root, "").unwrap_err();
    assert!(
        format!("{error:#}").contains("compare source \"nested/data\""),
        "complete nested comparison lost operation and relative path: {error:#}"
    );
    assert_eq!(
        error.root_cause().to_string(),
        "source exact values changed"
    );
}

#[test]
fn retained_nested_file_reports_full_path_without_parent_changes() {
    let (tmp, scope) = fixture();
    std::fs::set_permissions(
        tmp.path().join("nested/data"),
        std::fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    let (evidence, manifest) = Evidence::capture(&scope).unwrap();
    let Entry::Directory(nested) = &evidence.root.children[0] else {
        panic!("nested directory")
    };
    std::fs::set_permissions(
        tmp.path().join("nested/data"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    assert_eq!(
        Stamp::of(&fstat(&evidence.root.file).unwrap()),
        evidence.root.stamp
    );
    assert_eq!(Stamp::of(&fstat(&nested.file).unwrap()), nested.stamp);
    assert_eq!(names(&evidence.root.file, "").unwrap(), evidence.root.names);
    assert_eq!(names(&nested.file, "nested").unwrap(), nested.names);
    for _ in 0..2 {
        let error = evidence.verify(&scope).unwrap_err();
        assert!(
            format!("{error:#}").contains("revalidate retained source \"nested/data\""),
            "retained descendant revalidation lost complete relative path: {error:#}"
        );
        assert_eq!(error.root_cause().to_string(), "source metadata changed");
        let mut files = Vec::new();
        evidence.root.records(&mut files);
        assert_eq!(assemble_manifest(files).unwrap(), manifest);
    }
}
