//! Fixed-path, non-following, bounded reads. No native configuration discovery.
use super::CopilotPreflightError as Error;
use super::contract::CodePin;
use rustix::fs::{Dir, Mode, OFlags, open, openat};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{File, Metadata};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    mode: u32,
    uid: u32,
    links: u64,
    bytes: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl Stamp {
    fn of(meta: &Metadata) -> Self {
        Self {
            dev: meta.dev(),
            ino: meta.ino(),
            mode: meta.mode(),
            uid: meta.uid(),
            links: meta.nlink(),
            bytes: meta.len(),
            modified: (meta.mtime(), meta.mtime_nsec()),
            changed: (meta.ctime(), meta.ctime_nsec()),
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Directory {
    dev: u64,
    ino: u64,
    owner: u32,
}
impl Directory {
    pub fn of(file: &File) -> Result<Self, Error> {
        let meta = file.metadata().map_err(|_| Error::UnsafeFilesystem)?;
        if !meta.is_dir() || meta.mode() & 0o7777 != 0o700 {
            return Err(Error::UnsafeFilesystem);
        }
        Ok(Self {
            dev: meta.dev(),
            ino: meta.ino(),
            owner: meta.uid(),
        })
    }
}
pub(super) fn directory(path: &Path) -> Result<File, Error> {
    Ok(File::from(
        open(path, dir_flags(), Mode::empty()).map_err(|_| Error::UnsafeFilesystem)?,
    ))
}
fn dir_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}
fn child(parent: &File, name: &str, is_dir: bool) -> Result<File, Error> {
    let flags = if is_dir {
        dir_flags()
    } else {
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC
    };
    Ok(File::from(
        openat(parent, name, flags, Mode::empty()).map_err(|_| Error::UnsafeFilesystem)?,
    ))
}
fn parent(root: &File, path: &str) -> Result<(File, String), Error> {
    let mut parts = path.split('/').collect::<Vec<_>>();
    if parts
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return Err(Error::InvalidInput);
    }
    let name = parts.pop().ok_or(Error::InvalidInput)?.to_owned();
    let expected = Directory::of(root)?;
    let mut opened = root.try_clone().map_err(|_| Error::UnsafeFilesystem)?;
    for part in parts {
        opened = child(&opened, part, true)?;
        let selected = Directory::of(&opened)?;
        if selected.dev != expected.dev || selected.owner != expected.owner {
            return Err(Error::UnsafeFilesystem);
        }
    }
    Ok((opened, name))
}
pub(super) fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
#[derive(PartialEq, Eq)]
pub(super) struct Capture {
    pub bytes: Vec<u8>,
    stamp: Stamp,
}
impl Capture {
    pub fn read(root: &File, path: &str, hash: &str, limit: u64) -> Result<Self, Error> {
        let (parent, name) = parent(root, path)?;
        let mut file = child(&parent, &name, false)?;
        let meta = file.metadata().map_err(|_| Error::UnsafeFilesystem)?;
        if !meta.is_file()
            || meta.mode() & 0o7777 != 0o600
            || meta.nlink() != 1
            || meta.len() > limit
            || meta.uid() != Directory::of(root)?.owner
            || meta.dev() != Directory::of(root)?.dev
        {
            return Err(Error::UnsafeFilesystem);
        }
        let stamp = Stamp::of(&meta);
        let mut bytes = Vec::new();
        (&mut file)
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::UnsafeFilesystem)?;
        if bytes.len() as u64 != meta.len()
            || digest(&bytes) != hash
            || Stamp::of(&file.metadata().map_err(|_| Error::UnsafeFilesystem)?) != stamp
        {
            return Err(Error::Changed);
        }
        Ok(Self { bytes, stamp })
    }
}
/// Enumerate only names; refuse an unapproved entry before opening its contents.
pub(super) fn tree(
    root: &File,
    allowed: &BTreeMap<String, bool>,
) -> Result<BTreeMap<String, Directory>, Error> {
    fn visit(
        file: &File,
        prefix: &str,
        allowed: &BTreeMap<String, bool>,
        seen: &mut BTreeMap<String, bool>,
        dirs: &mut BTreeMap<String, Directory>,
        expected: &Directory,
    ) -> Result<(), Error> {
        let selected = Directory::of(file)?;
        if selected.dev != expected.dev || selected.owner != expected.owner {
            return Err(Error::UnsafeFilesystem);
        }
        dirs.insert(prefix.into(), selected);
        let entries = Dir::read_from(file).map_err(|_| Error::UnsafeFilesystem)?;
        for entry in entries {
            let entry = entry.map_err(|_| Error::UnsafeFilesystem)?;
            let name = entry.file_name().to_bytes();
            if name == b"." || name == b".." {
                continue;
            }
            let name = std::str::from_utf8(name).map_err(|_| Error::UnsafeFilesystem)?;
            let path = if prefix.is_empty() {
                name.into()
            } else {
                format!("{prefix}/{name}")
            };
            let is_dir = *allowed.get(&path).ok_or(Error::UnsafeFilesystem)?;
            if seen.insert(path.clone(), is_dir).is_some() {
                return Err(Error::UnsafeFilesystem);
            }
            if is_dir {
                visit(
                    &child(file, name, true)?,
                    &path,
                    allowed,
                    seen,
                    dirs,
                    expected,
                )?;
            }
        }
        Ok(())
    }
    let mut seen = BTreeMap::new();
    let mut dirs = BTreeMap::new();
    visit(
        root,
        "",
        allowed,
        &mut seen,
        &mut dirs,
        &Directory::of(root)?,
    )?;
    if &seen != allowed {
        return Err(Error::UnsafeFilesystem);
    }
    Ok(dirs)
}
pub(super) fn verify_directories(
    root: &File,
    expected: &BTreeMap<String, Directory>,
) -> Result<(), Error> {
    for (path, identity) in expected {
        let file = if path.is_empty() {
            root.try_clone().map_err(|_| Error::UnsafeFilesystem)?
        } else {
            let (parent, name) = parent(root, path)?;
            child(&parent, &name, true)?
        };
        if &Directory::of(&file)? != identity {
            return Err(Error::Changed);
        }
    }
    Ok(())
}
pub(super) struct Asset {
    pin: CodePin,
    stamp: Stamp,
}
impl Asset {
    pub fn capture(pin: CodePin) -> Result<Self, Error> {
        let stamp = Self::read(&pin)?;
        Ok(Self { pin, stamp })
    }
    fn read(pin: &CodePin) -> Result<Stamp, Error> {
        if pin.selected.canonicalize().ok().as_ref() != Some(&pin.resolved) {
            return Err(Error::Changed);
        }
        let mut file = File::from(
            open(
                &pin.resolved,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| Error::UnsafeFilesystem)?,
        );
        let meta = file.metadata().map_err(|_| Error::UnsafeFilesystem)?;
        let mode = pin
            .mode
            .strip_prefix("0o")
            .and_then(|value| u32::from_str_radix(value, 8).ok())
            .ok_or(Error::InvalidInput)?;
        if !meta.is_file()
            || mode > 0o777
            || mode & 0o111 == 0
            || meta.mode() & 0o7777 != mode
            || meta.len() != pin.bytes
        {
            return Err(Error::UnsafeFilesystem);
        }
        let stamp = Stamp::of(&meta);
        let mut hash = Sha256::new();
        let mut total = 0u64;
        let mut buffer = [0u8; 32768];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|_| Error::UnsafeFilesystem)?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > pin.bytes {
                return Err(Error::Changed);
            }
            hash.update(&buffer[..count]);
        }
        let observed: String = hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if total != pin.bytes
            || observed != pin.sha256
            || Stamp::of(&file.metadata().map_err(|_| Error::UnsafeFilesystem)?) != stamp
            || pin.selected.canonicalize().ok().as_ref() != Some(&pin.resolved)
            || Stamp::of(&std::fs::metadata(&pin.resolved).map_err(|_| Error::UnsafeFilesystem)?)
                != stamp
        {
            return Err(Error::Changed);
        }
        Ok(stamp)
    }
    pub fn verify(&self) -> Result<(), Error> {
        if Self::read(&self.pin)? != self.stamp {
            return Err(Error::Changed);
        }
        Ok(())
    }
}
