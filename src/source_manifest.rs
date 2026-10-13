//! Strict captured source bytes and pure validation for future lifecycle planning.
//!
//! Observation uses ordinary synchronous filesystem APIs. It is not an atomic
//! snapshot or a containment guarantee, and does not establish source authenticity.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

/// Exact bytes and available permission evidence captured at a relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedSourceFile {
    pub path: String,
    pub bytes: Vec<u8>,
    pub content_sha256: String,
    pub mode: Option<u32>,
}

/// Sorted captured files and their versioned framed digest.
///
/// Public values may be modified or supplied independently; validate them before
/// consumption. A valid digest does not prove the origin of these bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceManifest {
    pub files: Vec<CapturedSourceFile>,
    pub content_sha256: String,
}

/// Capture every observed regular file, failing on errors, links or special entries.
///
/// The root's trailing separators and dot suffixes are removed lexically before
/// inspecting it. Ancestor symlinks and concurrent replacements are not contained.
pub fn capture_source(root: &Path) -> Result<SourceManifest> {
    capture_with(root, &StdIo)
}

/// Check paths, strict order, permission bits and all digests without I/O or mutation.
pub fn validate_manifest(manifest: &SourceManifest) -> Result<()> {
    let mut previous: Option<&str> = None;
    for file in &manifest.files {
        crate::ownership::validate_relative_path(&file.path)
            .with_context(|| format!("invalid source path {:?}", file.path))?;
        if previous.is_some_and(|path| path >= file.path.as_str()) {
            bail!(
                "source paths must be unique and in strict ascending order: {:?}",
                file.path
            );
        }
        previous = Some(&file.path);
        if file.mode.is_some_and(|mode| mode > 0o7777) {
            bail!("source permission bits must be 0..=0o7777: {:?}", file.path);
        }
        if file.content_sha256 != content_hash(&file.bytes) {
            bail!("source file digest mismatch: {:?}", file.path);
        }
    }
    if manifest.content_sha256 != manifest_hash(&manifest.files)? {
        bail!("source manifest digest mismatch");
    }
    Ok(())
}

trait ObservationIo {
    fn entries(&self, path: &Path) -> std::io::Result<Vec<std::io::Result<PathBuf>>>;
    fn metadata(&self, path: &Path) -> std::io::Result<fs::Metadata>;
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>>;
}

struct StdIo;

impl ObservationIo for StdIo {
    fn entries(&self, path: &Path) -> std::io::Result<Vec<std::io::Result<PathBuf>>> {
        Ok(fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect())
    }

    fn metadata(&self, path: &Path) -> std::io::Result<fs::Metadata> {
        fs::symlink_metadata(path)
    }

    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        fs::read(path)
    }
}

fn capture_with(root: &Path, io: &impl ObservationIo) -> Result<SourceManifest> {
    let root: PathBuf = root.components().collect();
    let metadata = io
        .metadata(&root)
        .with_context(|| format!("inspect source root metadata {}", root.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("source root symlink is unsupported: {}", root.display());
    }
    if !metadata.is_dir() {
        bail!("source root must be a directory: {}", root.display());
    }
    let mut files = Vec::new();
    capture_directory(&root, Path::new(""), io, &mut files)?;
    assemble_manifest(files)
}

pub(crate) fn assemble_manifest(mut files: Vec<CapturedSourceFile>) -> Result<SourceManifest> {
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let manifest = SourceManifest {
        content_sha256: manifest_hash(&files)?,
        files,
    };
    validate_manifest(&manifest).context("validate captured source manifest")?;
    Ok(manifest)
}

fn capture_directory(
    directory: &Path,
    relative: &Path,
    io: &impl ObservationIo,
    files: &mut Vec<CapturedSourceFile>,
) -> Result<()> {
    let entries = io
        .entries(directory)
        .with_context(|| format!("enumerate source directory {}", directory.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| {
            format!(
                "enumerate source directory entry in {}",
                directory.display()
            )
        })?;
        let metadata = io
            .metadata(&entry)
            .with_context(|| format!("inspect source entry metadata {}", entry.display()))?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            bail!("source entry symlink is unsupported: {}", entry.display());
        }
        let name = entry
            .file_name()
            .with_context(|| format!("source entry has no filename: {}", entry.display()))?;
        let relative = relative.join(name);
        let path = source_path(&relative)?;
        if file_type.is_dir() {
            capture_directory(&entry, &relative, io, files)?;
        } else if file_type.is_file() {
            let bytes = io
                .read(&entry)
                .with_context(|| format!("read source file {}", entry.display()))?;
            #[cfg(unix)]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                Some(metadata.permissions().mode() & 0o7777)
            };
            #[cfg(not(unix))]
            let mode = None;
            files.push(CapturedSourceFile {
                path,
                content_sha256: content_hash(&bytes),
                bytes,
                mode,
            });
        } else {
            bail!("source special entry is unsupported: {}", entry.display());
        }
    }
    Ok(())
}

fn source_path(path: &Path) -> Result<String> {
    let components = path
        .iter()
        .map(|part| {
            part.to_str()
                .with_context(|| format!("non-UTF-8 source path: {}", path.display()))
        })
        .collect::<Result<Vec<_>>>()?;
    let path = components.join("/");
    crate::ownership::validate_relative_path(&path)
        .with_context(|| format!("invalid source path {path:?}"))?;
    Ok(path)
}

pub(crate) fn content_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn put_length(hasher: &mut Sha256, length: usize) -> Result<()> {
    let encoded = u64::try_from(length).context("source manifest length exceeds u64")?;
    hasher.update(encoded.to_be_bytes());
    Ok(())
}

fn manifest_hash(files: &[CapturedSourceFile]) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"upskill:source-manifest:v1\0");
    put_length(&mut hasher, files.len())?;
    for file in files {
        put_length(&mut hasher, file.path.len())?;
        hasher.update(file.path.as_bytes());
        put_length(&mut hasher, file.bytes.len())?;
        hasher.update(&file.bytes);
        match file.mode {
            None => hasher.update([0]),
            Some(mode) => {
                hasher.update([1]);
                hasher.update(mode.to_be_bytes());
            }
        }
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::fs;
    use std::io;
    use std::path::PathBuf;

    struct TestIo {
        failure: Option<(&'static str, PathBuf)>,
        reads: RefCell<BTreeMap<PathBuf, usize>>,
    }

    impl TestIo {
        fn new(failure: Option<(&'static str, PathBuf)>) -> Self {
            Self {
                failure,
                reads: RefCell::new(BTreeMap::new()),
            }
        }

        fn check(&self, operation: &str, path: &Path) -> io::Result<()> {
            if self
                .failure
                .as_ref()
                .is_some_and(|(op, selected)| *op == operation && selected == path)
            {
                return Err(io::Error::other("injected observation failure"));
            }
            Ok(())
        }
    }

    impl ObservationIo for TestIo {
        fn entries(&self, path: &Path) -> io::Result<Vec<io::Result<PathBuf>>> {
            self.check("enumerate", path)?;
            let mut entries = fs::read_dir(path)?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<io::Result<Vec<_>>>()?;
            entries.sort();
            Ok(entries
                .into_iter()
                .map(|entry| {
                    if self.failure.as_ref().is_some_and(|(operation, selected)| {
                        *operation == "iterator" && selected == &entry
                    }) {
                        Err(io::Error::other("injected observation failure"))
                    } else {
                        Ok(entry)
                    }
                })
                .collect())
        }

        fn metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
            self.check("metadata", path)?;
            fs::symlink_metadata(path)
        }

        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            *self
                .reads
                .borrow_mut()
                .entry(path.to_path_buf())
                .or_default() += 1;
            self.check("read", path)?;
            fs::read(path)
        }
    }

    fn assert_fault(operation: &'static str, directory: bool) {
        let root = tempfile::tempdir().unwrap();
        let earlier = root.path().join("a-readable");
        fs::write(&earlier, b"earlier").unwrap();
        let failing = root.path().join("z-failing");
        if directory {
            fs::create_dir(&failing).unwrap();
        } else {
            fs::write(&failing, b"later").unwrap();
        }
        let io = TestIo::new(Some((operation, failing.clone())));
        let result = capture_with(root.path(), &io);
        assert!(
            result.is_err(),
            "expected {operation} failure, got {result:?}"
        );
        let message = format!("{:#}", result.unwrap_err());
        if operation == "iterator" {
            assert!(message.contains("directory entry"), "{message}");
            assert!(message.contains(root.path().to_str().unwrap()), "{message}");
        } else {
            assert!(message.contains(operation), "{message}");
            assert!(message.contains(failing.to_str().unwrap()), "{message}");
        }
        assert!(
            message.contains("injected observation failure"),
            "{message}"
        );
        assert_eq!(io.reads.borrow().get(&earlier), Some(&1));
    }

    #[test]
    fn injected_enumeration_failure_returns_no_partial_manifest() {
        assert_fault("enumerate", true);
    }

    #[test]
    fn injected_metadata_failure_returns_no_partial_manifest() {
        assert_fault("metadata", false);
    }

    #[test]
    fn injected_read_failure_returns_no_partial_manifest() {
        assert_fault("read", false);
    }

    #[test]
    fn injected_iterator_failure_returns_no_partial_manifest() {
        assert_fault("iterator", false);
    }

    #[test]
    fn each_regular_file_is_read_once() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("nested")).unwrap();
        let first = root.path().join("a");
        let second = root.path().join("nested/b");
        fs::write(&first, b"one").unwrap();
        fs::write(&second, b"two").unwrap();
        let io = TestIo::new(None);
        let captured = capture_with(root.path(), &io).unwrap();
        assert_eq!(captured.files.len(), 2);
        assert_eq!(
            *io.reads.borrow(),
            BTreeMap::from([(first, 1), (second, 1)])
        );
    }

    #[cfg(unix)]
    #[test]
    fn observed_links_and_special_entries_are_never_read() {
        use std::os::unix::fs::symlink;
        use std::os::unix::net::UnixListener;
        for link in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("entry");
            let _listener = if link {
                symlink("missing", &path).unwrap();
                None
            } else {
                Some(UnixListener::bind(&path).unwrap())
            };
            let io = TestIo::new(None);
            let result = capture_with(root.path(), &io);
            assert!(
                result.is_err(),
                "expected unsupported entry rejection, got {result:?}"
            );
            let message = format!("{:#}", result.unwrap_err());
            assert!(
                message.contains(if link {
                    "entry symlink"
                } else {
                    "special entry"
                }),
                "{message}"
            );
            assert!(io.reads.borrow().is_empty());
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_relative_path_is_rejected_without_filesystem_fixture() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(OsString::from_vec(vec![0xff]));
        let result = source_path(&path);
        assert!(
            result.is_err(),
            "expected non-UTF-8 rejection, got {result:?}"
        );
        assert!(format!("{:#}", result.unwrap_err()).contains("non-UTF-8"));
    }
}
