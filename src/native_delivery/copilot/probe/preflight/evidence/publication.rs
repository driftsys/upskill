//! Exclusive descriptor-relative publication, with no deletion or adoption path.
use super::{CopilotEvidenceError as Error, files};
use rustix::fs::{Mode, OFlags, mkdirat, openat};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::MetadataExt;

pub(super) fn open_area(root: &File) -> Result<File, Error> {
    Ok(File::from(
        openat(
            root,
            "evidence",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| Error::Changed)?,
    ))
}

/// Checkpoint zero follows directory sync; later checkpoints precede each file.
/// A failed checkpoint follows the same retain-without-replay path as I/O errors.
pub(super) fn publish(
    root: &File,
    payload: &[(String, Vec<u8>)],
    mut checkpoint: impl FnMut(usize) -> Result<(), Error>,
) -> Result<(File, BTreeMap<String, files::Capture>), Error> {
    mkdirat(root, "evidence", Mode::RWXU).map_err(|error| {
        if error == rustix::io::Errno::EXIST {
            Error::Unavailable
        } else {
            Error::PublicationUncertain
        }
    })?;
    root.sync_all().map_err(|_| Error::PublicationUncertain)?;
    let directory = open_area(root).map_err(|_| Error::PublicationUncertain)?;
    files::Directory::of(&directory).map_err(|_| Error::PublicationUncertain)?;
    let meta = directory
        .metadata()
        .map_err(|_| Error::PublicationUncertain)?;
    let parent = root.metadata().map_err(|_| Error::PublicationUncertain)?;
    if meta.dev() != parent.dev() || meta.uid() != parent.uid() {
        return Err(Error::PublicationUncertain);
    }
    directory
        .sync_all()
        .map_err(|_| Error::PublicationUncertain)?;
    checkpoint(0)?;
    let mut captures = BTreeMap::new();
    for (index, (name, bytes)) in payload.iter().enumerate() {
        checkpoint(index + 1)?;
        let mut file = File::from(
            openat(
                &directory,
                name.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(|_| Error::PublicationUncertain)?,
        );
        file.write_all(bytes)
            .map_err(|_| Error::PublicationUncertain)?;
        file.sync_all().map_err(|_| Error::PublicationUncertain)?;
        directory
            .sync_all()
            .map_err(|_| Error::PublicationUncertain)?;
        let capture =
            files::Capture::read(&directory, name, &files::digest(bytes), bytes.len() as u64)
                .map_err(|_| Error::PublicationUncertain)?;
        captures.insert(name.clone(), capture);
    }
    directory
        .sync_all()
        .map_err(|_| Error::PublicationUncertain)?;
    root.sync_all().map_err(|_| Error::PublicationUncertain)?;
    Ok((directory, captures))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn partial_publication_is_retained_and_never_reopened_after_failure() {
        for fail_at in [0, 2] {
            let temp = tempfile::tempdir().unwrap();
            std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            let root = files::directory(temp.path()).unwrap();
            let payload = vec![
                ("header.json".into(), b"known initial evidence".to_vec()),
                ("fixture-00.bin".into(), b"known fixture".to_vec()),
            ];
            let result = publish(&root, &payload, |index| {
                if index == fail_at {
                    Err(Error::PublicationUncertain)
                } else {
                    Ok(())
                }
            });
            assert!(matches!(result, Err(Error::PublicationUncertain)));
            let area = temp.path().join("evidence");
            assert!(area.is_dir());
            assert_eq!(
                std::fs::metadata(&area).unwrap().permissions().mode() & 0o7777,
                0o700
            );
            assert!(!area.join("fixture-00.bin").exists());
            if fail_at == 2 {
                assert_eq!(
                    std::fs::read(area.join("header.json")).unwrap(),
                    b"known initial evidence"
                );
            } else {
                assert!(!area.join("header.json").exists());
            }
            assert!(matches!(
                publish(&root, &payload, |_| Ok(())),
                Err(Error::Unavailable)
            ));
            if fail_at == 2 {
                assert_eq!(
                    std::fs::read(area.join("header.json")).unwrap(),
                    b"known initial evidence"
                );
            }
        }
    }
    #[test]
    fn file_collision_preserves_existing_content_and_retains_the_namespace() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let root = files::directory(temp.path()).unwrap();
        let payload = vec![("header.json".into(), b"known initial evidence".to_vec())];
        let path = temp.path().join("evidence/header.json");
        let result = publish(&root, &payload, |index| {
            if index == 1 {
                std::fs::write(&path, b"manual-sentinel").unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
            }
            Ok(())
        });
        assert!(matches!(result, Err(Error::PublicationUncertain)));
        assert_eq!(std::fs::read(&path).unwrap(), b"manual-sentinel");
        assert!(matches!(
            publish(&root, &payload, |_| Ok(())),
            Err(Error::Unavailable)
        ));
        assert_eq!(std::fs::read(&path).unwrap(), b"manual-sentinel");
    }
}
