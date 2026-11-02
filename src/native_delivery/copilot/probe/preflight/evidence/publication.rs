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

trait PublicationIo {
    fn write(&mut self, file: &mut File, bytes: &[u8]) -> std::io::Result<()>;
    fn sync(&mut self, file: &File) -> std::io::Result<()>;
}
struct StdIo;
impl PublicationIo for StdIo {
    fn write(&mut self, file: &mut File, bytes: &[u8]) -> std::io::Result<()> {
        file.write_all(bytes)
    }
    fn sync(&mut self, file: &File) -> std::io::Result<()> {
        file.sync_all()
    }
}

/// Checkpoint zero follows directory sync; later checkpoints precede each file.
/// A failed checkpoint follows the same retain-without-replay path as I/O errors.
pub(super) fn publish(
    root: &File,
    payload: &[(String, Vec<u8>)],
    checkpoint: impl FnMut(usize) -> Result<(), Error>,
) -> Result<(File, BTreeMap<String, files::Capture>), Error> {
    publish_with_io(root, payload, checkpoint, &mut StdIo)
}
fn publish_with_io(
    root: &File,
    payload: &[(String, Vec<u8>)],
    mut checkpoint: impl FnMut(usize) -> Result<(), Error>,
    io: &mut impl PublicationIo,
) -> Result<(File, BTreeMap<String, files::Capture>), Error> {
    mkdirat(root, "evidence", Mode::RWXU).map_err(|error| {
        if error == rustix::io::Errno::EXIST {
            Error::Unavailable
        } else {
            Error::PublicationUncertain
        }
    })?;
    io.sync(root).map_err(|_| Error::PublicationUncertain)?;
    let directory = open_area(root).map_err(|_| Error::PublicationUncertain)?;
    files::Directory::of(&directory).map_err(|_| Error::PublicationUncertain)?;
    let meta = directory
        .metadata()
        .map_err(|_| Error::PublicationUncertain)?;
    let parent = root.metadata().map_err(|_| Error::PublicationUncertain)?;
    if meta.dev() != parent.dev() || meta.uid() != parent.uid() {
        return Err(Error::PublicationUncertain);
    }
    io.sync(&directory)
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
        io.write(&mut file, bytes)
            .map_err(|_| Error::PublicationUncertain)?;
        io.sync(&file).map_err(|_| Error::PublicationUncertain)?;
        io.sync(&directory)
            .map_err(|_| Error::PublicationUncertain)?;
        let capture =
            files::Capture::read(&directory, name, &files::digest(bytes), bytes.len() as u64)
                .map_err(|_| Error::PublicationUncertain)?;
        captures.insert(name.clone(), capture);
    }
    io.sync(&directory)
        .map_err(|_| Error::PublicationUncertain)?;
    io.sync(root).map_err(|_| Error::PublicationUncertain)?;
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
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Request {
        Write((u64, u64), usize),
        Sync((u64, u64)),
    }
    #[derive(Default)]
    struct ObservedIo {
        requests: Vec<Request>,
        fail_at: Option<usize>,
    }
    fn identity(file: &File) -> (u64, u64) {
        let meta = file.metadata().unwrap();
        (meta.dev(), meta.ino())
    }
    impl PublicationIo for ObservedIo {
        fn write(&mut self, file: &mut File, bytes: &[u8]) -> std::io::Result<()> {
            let index = self.requests.len();
            self.requests
                .push(Request::Write(identity(file), bytes.len()));
            if self.fail_at == Some(index) {
                file.write_all(&bytes[..5])?;
                return Err(std::io::Error::other("injected partial write"));
            }
            file.write_all(bytes)
        }
        fn sync(&mut self, file: &File) -> std::io::Result<()> {
            let index = self.requests.len();
            self.requests.push(Request::Sync(identity(file)));
            file.sync_all()?;
            if self.fail_at == Some(index) {
                return Err(std::io::Error::other("injected sync error"));
            }
            Ok(())
        }
    }
    fn prepared_root() -> (tempfile::TempDir, File) {
        let temp = tempfile::tempdir().unwrap();
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let root = files::directory(temp.path()).unwrap();
        (temp, root)
    }
    fn payload() -> Vec<(String, Vec<u8>)> {
        vec![
            ("header.json".into(), b"known initial evidence".to_vec()),
            ("fixture-00.bin".into(), b"known fixture".to_vec()),
        ]
    }
    #[test]
    fn publication_orders_real_file_and_directory_sync_requests() {
        let (temp, root) = prepared_root();
        let mut io = ObservedIo::default();
        let (area, _) = publish_with_io(&root, &payload(), |_| Ok(()), &mut io).unwrap();
        let header = File::open(temp.path().join("evidence/header.json")).unwrap();
        let fixture = File::open(temp.path().join("evidence/fixture-00.bin")).unwrap();
        let parent_id = identity(&root);
        let area_id = identity(&area);
        let header_id = identity(&header);
        let fixture_id = identity(&fixture);
        assert_eq!(
            io.requests,
            vec![
                Request::Sync(parent_id),
                Request::Sync(area_id),
                Request::Write(header_id, 22),
                Request::Sync(header_id),
                Request::Sync(area_id),
                Request::Write(fixture_id, 13),
                Request::Sync(fixture_id),
                Request::Sync(area_id),
                Request::Sync(area_id),
                Request::Sync(parent_id),
            ]
        );
        assert_eq!(
            std::fs::read(temp.path().join("evidence/header.json")).unwrap(),
            b"known initial evidence"
        );
        assert_eq!(
            std::fs::read(temp.path().join("evidence/fixture-00.bin")).unwrap(),
            b"known fixture"
        );
    }
    #[test]
    fn publication_retains_partial_files_and_stops_after_every_io_failure() {
        for fail_at in 0..10 {
            let (temp, root) = prepared_root();
            let mut io = ObservedIo {
                requests: vec![],
                fail_at: Some(fail_at),
            };
            assert!(matches!(
                publish_with_io(&root, &payload(), |_| Ok(()), &mut io),
                Err(Error::PublicationUncertain)
            ));
            assert_eq!(
                io.requests.len(),
                fail_at + 1,
                "continued after I/O failure {fail_at}"
            );
            let area = temp.path().join("evidence");
            assert!(area.is_dir());
            for (name, first_write, full) in [
                ("header.json", 2, b"known initial evidence".as_slice()),
                ("fixture-00.bin", 5, b"known fixture".as_slice()),
            ] {
                let path = area.join(name);
                if fail_at < first_write {
                    assert!(!path.exists());
                } else {
                    assert_eq!(
                        std::fs::read(&path).unwrap(),
                        if fail_at == first_write {
                            &full[..5]
                        } else {
                            full
                        }
                    );
                    assert_eq!(
                        std::fs::metadata(path).unwrap().permissions().mode() & 0o7777,
                        0o600
                    );
                }
            }
            let before = std::fs::read_dir(&area).unwrap().count();
            assert!(matches!(
                publish(&root, &payload(), |_| Ok(())),
                Err(Error::Unavailable)
            ));
            assert_eq!(std::fs::read_dir(&area).unwrap().count(), before);
        }
    }
}
