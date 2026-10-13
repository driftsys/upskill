//! Complete descriptor-relative source acquisition and retained revalidation.
use crate::filesystem::QualifiedScope;
use crate::filesystem::qualification::{DescriptorQueries, Mount};
use crate::source_manifest::{CapturedSourceFile, SourceManifest, assemble_manifest, content_hash};
use anyhow::{Context, Result, bail};
use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags, Stat, fstat, openat, statat};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Read};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Identity {
    device: u64,
    inode: u64,
}
impl Identity {
    #[allow(clippy::unnecessary_cast)]
    fn of(stat: &Stat) -> Self {
        Self {
            device: stat.st_dev as u64,
            inode: stat.st_ino as u64,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    identity: Identity,
    mode: u32,
    links: u64,
    size: i64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl Stamp {
    #[allow(clippy::unnecessary_cast)]
    fn of(stat: &Stat) -> Self {
        Self {
            identity: Identity::of(stat),
            mode: stat.st_mode as u32,
            links: stat.st_nlink as u64,
            size: stat.st_size as i64,
            modified: (stat.st_mtime as i64, stat.st_mtime_nsec as i64),
            changed: (stat.st_ctime as i64, stat.st_ctime_nsec as i64),
        }
    }
}
fn directory_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}
fn regular_flags() -> OFlags {
    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK
}
fn open_entry(parent: &File, name: &str, directory: bool) -> Result<File> {
    let flags = if directory {
        directory_flags()
    } else {
        regular_flags()
    };
    Ok(File::from(
        openat(parent, name, flags, Mode::empty()).with_context(|| {
            format!("open source component {name:?} without following symlinks")
        })?,
    ))
}
fn select_entry(parent: &File, name: &str) -> Result<Stat> {
    statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)
        .with_context(|| format!("inspect source component {name:?} without following symlinks"))
}
fn open_selected(
    parent: &File,
    name: &str,
    selected: &Stat,
    directory: bool,
    mount: &Mount,
    queries: DescriptorQueries,
) -> Result<(File, Stat)> {
    let opened = open_entry(parent, name, directory)?;
    let stat = fstat(&opened).context("inspect opened source descriptor")?;
    require_kind(&stat, directory)?;
    require_identity(Identity::of(selected), Identity::of(&stat))?;
    require_stamp(&Stamp::of(selected), &Stamp::of(&stat))?;
    mount.require_same(&Mount::of(&opened, queries)?)?;
    Ok((opened, stat))
}
fn names(file: &File, path: &str) -> Result<Vec<String>> {
    let iterator =
        Dir::read_from(file).with_context(|| format!("enumerate source directory {path:?}"))?;
    collect_names(iterator, path)
}
fn collect_names(
    iterator: impl IntoIterator<Item = rustix::io::Result<rustix::fs::DirEntry>>,
    path: &str,
) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in iterator {
        let entry = entry.with_context(|| format!("read source directory entry in {path:?}"))?;
        let bytes = entry.file_name().to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        let name = std::str::from_utf8(bytes)
            .with_context(|| format!("non-UTF-8 source entry in {path:?}"))?;
        let relative = relative_path(path, name);
        crate::ownership::validate_relative_path(&relative)
            .with_context(|| format!("invalid source path {relative:?}"))?;
        names.push(name.to_owned());
    }
    names.sort();
    Ok(names)
}
fn relative_path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}
fn require_kind(stat: &Stat, directory: bool) -> Result<()> {
    let expected = if directory {
        FileType::Directory
    } else {
        FileType::RegularFile
    };
    if FileType::from_raw_mode(stat.st_mode) != expected {
        bail!("source entry is not an eligible directory or regular file");
    }
    Ok(())
}
fn require_stamp(expected: &Stamp, actual: &Stamp) -> Result<()> {
    if expected.mode != actual.mode
        || expected.links != actual.links
        || expected.size != actual.size
        || expected.modified != actual.modified
        || expected.changed != actual.changed
    {
        bail!("source metadata changed");
    }
    Ok(())
}
fn require_identity(expected: Identity, actual: Identity) -> Result<()> {
    if expected != actual {
        bail!("source identity changed");
    }
    Ok(())
}
fn capture_bytes(
    file: &File,
    stamp: &Stamp,
    read: &mut impl FnMut(&File, &mut Vec<u8>) -> io::Result<usize>,
) -> Result<Vec<u8>> {
    let before = Stamp::of(&fstat(file).context("inspect source file before read")?);
    require_identity(stamp.identity, before.identity)?;
    require_stamp(stamp, &before)?;
    let mut bytes = Vec::new();
    let count = read(file, &mut bytes).context("read source file bytes")?;
    let after = Stamp::of(&fstat(file).context("inspect source file after read")?);
    require_identity(before.identity, after.identity)?;
    require_stamp(&before, &after)?;
    if before.size < 0 || bytes.len() as u64 != before.size as u64 || count != bytes.len() {
        bail!("source byte count disagrees with observed size");
    }
    Ok(bytes)
}
struct Regular {
    file: File,
    stamp: Stamp,
    record: CapturedSourceFile,
}
struct Directory {
    file: File,
    stamp: Stamp,
    names: Vec<String>,
    children: Vec<Entry>,
}
enum Entry {
    Directory(Directory),
    Regular(Regular),
}
impl Directory {
    fn capture(
        file: File,
        path: &str,
        mount: &Mount,
        seen: &mut HashSet<Identity>,
        read: &mut impl FnMut(&File, &mut Vec<u8>) -> io::Result<usize>,
        queries: DescriptorQueries,
    ) -> Result<Self> {
        let stat = fstat(&file).context("inspect source directory descriptor")?;
        require_kind(&stat, true)?;
        let stamp = Stamp::of(&stat);
        mount.require_same(&Mount::of(&file, queries)?)?;
        if !seen.insert(stamp.identity) {
            bail!("repeated source directory identity");
        }
        let names = names(&file, path)?;
        let mut children = Vec::new();
        for name in &names {
            let relative = relative_path(path, name);
            let child = (|| -> Result<Entry> {
                let selected = select_entry(&file, name)?;
                let directory = match FileType::from_raw_mode(selected.st_mode) {
                    FileType::Directory => true,
                    FileType::RegularFile => false,
                    _ => bail!("source symlink or special entry is unsupported"),
                };
                let (opened, stat) =
                    open_selected(&file, name, &selected, directory, mount, queries)?;
                if directory {
                    Ok(Entry::Directory(Self::capture(
                        opened, &relative, mount, seen, read, queries,
                    )?))
                } else {
                    let stamp = Stamp::of(&stat);
                    let bytes = capture_bytes(&opened, &stamp, read)?;
                    let record = CapturedSourceFile {
                        path: relative.clone(),
                        content_sha256: content_hash(&bytes),
                        bytes,
                        mode: Some(stamp.mode & 0o7777),
                    };
                    Ok(Entry::Regular(Regular {
                        file: opened,
                        stamp,
                        record,
                    }))
                }
            })()
            .with_context(|| format!("capture source {relative:?}"))?;
            children.push(child);
        }
        require_stamp(
            &stamp,
            &Stamp::of(&fstat(&file).context("inspect acquired source directory")?),
        )
        .with_context(|| format!("source directory changed during capture {path:?}"))?;
        Ok(Self {
            file,
            stamp,
            names,
            children,
        })
    }
    fn records(&self, files: &mut Vec<CapturedSourceFile>) {
        for child in &self.children {
            match child {
                Entry::Directory(directory) => directory.records(files),
                Entry::Regular(regular) => files.push(regular.record.clone()),
            }
        }
    }
    fn verify_retained(&self, path: &str, mount: &Mount, queries: DescriptorQueries) -> Result<()> {
        self.verify_handle(&self.file, &self.stamp, mount, queries)
            .with_context(|| {
                format!(
                    "revalidate retained source {:?}",
                    if path.is_empty() { "." } else { path }
                )
            })?;
        for (name, child) in self.names.iter().zip(&self.children) {
            let relative = relative_path(path, name);
            match child {
                Entry::Directory(directory) => directory.verify_retained(&relative, mount, queries),
                Entry::Regular(regular) => {
                    self.verify_handle(&regular.file, &regular.stamp, mount, queries)
                }
            }
            .with_context(|| format!("revalidate retained source {relative:?}"))?;
        }
        Ok(())
    }
    fn verify_handle(
        &self,
        file: &File,
        stamp: &Stamp,
        mount: &Mount,
        queries: DescriptorQueries,
    ) -> Result<()> {
        let actual = Stamp::of(&fstat(file).context("inspect retained source descriptor")?);
        require_identity(stamp.identity, actual.identity)?;
        require_stamp(stamp, &actual)?;
        mount.require_same(&Mount::of(file, queries)?)
    }
    fn compare(&self, current: &Self, path: &str) -> Result<()> {
        (|| -> Result<()> {
            require_identity(self.stamp.identity, current.stamp.identity)?;
            require_stamp(&self.stamp, &current.stamp)?;
            if self.names != current.names {
                bail!("source directory membership changed");
            }
            for (name, (expected, actual)) in self
                .names
                .iter()
                .zip(self.children.iter().zip(&current.children))
            {
                let relative = relative_path(path, name);
                (|| -> Result<()> {
                    match (expected, actual) {
                        (Entry::Directory(expected), Entry::Directory(actual)) => {
                            expected.compare(actual, &relative)?
                        }
                        (Entry::Regular(expected), Entry::Regular(actual)) => {
                            require_identity(expected.stamp.identity, actual.stamp.identity)?;
                            require_stamp(&expected.stamp, &actual.stamp)?;
                            if expected.record != actual.record {
                                bail!("source exact values changed");
                            }
                        }
                        _ => bail!("source entry kind changed"),
                    }
                    Ok(())
                })()
                .with_context(|| format!("compare source {relative:?}"))?;
            }
            Ok(())
        })()
        .with_context(|| {
            format!(
                "compare source {:?}",
                if path.is_empty() { "." } else { path }
            )
        })
    }
}
pub(super) struct Evidence {
    root: Directory,
    mount: Mount,
}
impl Evidence {
    pub(super) fn capture(scope: &QualifiedScope) -> Result<(Self, SourceManifest)> {
        Self::capture_with(
            scope,
            &mut |file, bytes| (&*file).read_to_end(bytes),
            Mount::queries(),
        )
    }
    fn capture_with(
        scope: &QualifiedScope,
        read: &mut impl FnMut(&File, &mut Vec<u8>) -> io::Result<usize>,
        queries: DescriptorQueries,
    ) -> Result<(Self, SourceManifest)> {
        scope
            .verify_binding()
            .context("verify source root before capture")?;
        let file = scope
            .reopen_directory()
            .context("reopen source root for capture")?;
        let mount = Mount::of(&file, queries).context("qualify source root")?;
        let root = Directory::capture(file, "", &mount, &mut HashSet::new(), read, queries)
            .context("capture complete source tree")?;
        scope
            .verify_binding()
            .context("verify source root after capture")?;
        let mut files = Vec::new();
        root.records(&mut files);
        let manifest = assemble_manifest(files)?;
        Ok((Self { root, mount }, manifest))
    }
    pub(super) fn verify(&self, scope: &QualifiedScope) -> Result<()> {
        self.verify_with(scope, Mount::queries())
    }
    fn verify_with(&self, scope: &QualifiedScope, queries: DescriptorQueries) -> Result<()> {
        self.root.verify_retained("", &self.mount, queries)?;
        let (fresh, _) = Self::capture_with(
            scope,
            &mut |file, bytes| (&*file).read_to_end(bytes),
            queries,
        )?;
        self.mount.require_same(&fresh.mount)?;
        self.root.compare(&fresh.root, "")?;
        scope
            .verify_binding()
            .context("verify source root after revalidation")?;
        Ok(())
    }
}
#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;
