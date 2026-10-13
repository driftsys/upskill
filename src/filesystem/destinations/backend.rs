//! Descriptor-relative traversal, retained identities and exact revalidation.
use super::qualification::Mount;
use crate::files::plan::{FileLocation, FileValue, ObservedFile};
use crate::filesystem::QualifiedScope;
use anyhow::{Context, Result, bail};
use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags, Stat, fstat, openat, statat};
use rustix::io::Errno;
use std::fs::File;
use std::io::Read;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identity {
    device: u64,
    inode: u64,
}
impl Identity {
    #[allow(clippy::unnecessary_cast)] // Native stat widths differ across the qualified targets.
    fn of(stat: &Stat) -> Self {
        Self {
            device: stat.st_dev as u64,
            inode: stat.st_ino as u64,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
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
fn open_directory(parent: &File, component: &str) -> Result<File> {
    Ok(File::from(
        openat(parent, component, directory_flags(), Mode::empty())
            .context("open destination directory without following symlinks")?,
    ))
}
fn open_file(parent: &File, component: &str) -> Result<File> {
    Ok(File::from(
        openat(
            parent,
            component,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .context("open regular destination without following symlinks")?,
    ))
}
fn entry(parent: &File, component: &str) -> Result<Option<Stat>> {
    match statat(parent, component, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => Ok(Some(stat)),
        Err(error) => classify_absence(error),
    }
}
fn classify_absence(error: Errno) -> Result<Option<Stat>> {
    if error == Errno::NOENT {
        Ok(None)
    } else {
        Err(anyhow::Error::new(error)
            .context("inspect destination component without following symlinks"))
    }
}
fn exact_spelling(parent: &File, component: &str) -> Result<()> {
    let entries = Dir::read_from(parent).context("enumerate destination parent spellings")?;
    for item in entries {
        if item
            .context("read destination parent spelling")?
            .file_name()
            .to_bytes()
            == component.as_bytes()
        {
            return Ok(());
        }
    }
    bail!("destination component does not have its exact stored spelling: {component}")
}
fn require_regular(stat: &Stat) -> Result<()> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile || stat.st_nlink != 1 {
        bail!("destination must be a regular file with exactly one link");
    }
    Ok(())
}
fn verify_absence(parent: &File, suffix: &[String]) -> Result<()> {
    if entry(parent, &suffix[0])?.is_some() {
        bail!("destination first-missing component became present");
    }
    Ok(())
}

fn verify_value(expected: &FileValue, actual: &FileValue) -> Result<()> {
    if expected != actual {
        bail!("destination exact bytes or permission mode changed");
    }
    Ok(())
}
struct Directory {
    file: File,
    identity: Identity,
}
impl Directory {
    fn new(file: File, mount: &Mount) -> Result<Self> {
        let stat = fstat(&file).context("inspect destination directory descriptor")?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
            bail!("destination ancestor is not a directory");
        }
        mount.require_same(&Mount::of(&file)?)?;
        Ok(Self {
            file,
            identity: Identity::of(&stat),
        })
    }
    fn verify(&self, mount: &Mount) -> Result<()> {
        let stat = fstat(&self.file).context("revalidate retained directory")?;
        if Identity::of(&stat) != self.identity
            || FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        {
            bail!("retained directory identity changed");
        }
        mount.require_same(&Mount::of(&self.file)?)
    }
}
struct BoundRoot {
    binding: QualifiedScope,
    directory: Directory,
}
enum Final {
    Present {
        file: File,
        stamp: Stamp,
        value: FileValue,
    },
    Missing {
        suffix: Vec<String>,
    },
}
struct PathEvidence {
    root: usize,
    directories: Vec<Directory>,
    last: Final,
}
impl PathEvidence {
    fn value(&self) -> Option<FileValue> {
        match &self.last {
            Final::Present { value, .. } => Some(value.clone()),
            Final::Missing { .. } => None,
        }
    }
    fn capture(root: usize, directory: &File, mount: &Mount, path: &str) -> Result<Self> {
        let components: Vec<_> = path.split('/').collect();
        let mut directories = Vec::<Directory>::new();
        for (index, component) in components.iter().enumerate() {
            let parent = directories
                .last()
                .map_or(directory, |directory| &directory.file);
            let Some(selected) = entry(parent, component)? else {
                return Ok(Self {
                    root,
                    directories,
                    last: Final::Missing {
                        suffix: components[index..]
                            .iter()
                            .map(|c| (*c).to_owned())
                            .collect(),
                    },
                });
            };
            exact_spelling(parent, component)?;
            if index + 1 == components.len() {
                require_regular(&selected)?;
                let mut file = open_file(parent, component)?;
                let opened = fstat(&file).context("inspect opened destination file")?;
                require_regular(&opened)?;
                if Stamp::of(&selected) != Stamp::of(&opened) {
                    bail!("destination file changed during open");
                }
                mount.require_same(&Mount::of(&file)?)?;
                let stamp = Stamp::of(&opened);
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)
                    .context("capture destination bytes")?;
                let after = fstat(&file).context("inspect captured destination file")?;
                if stamp != Stamp::of(&after)
                    || stamp.size < 0
                    || bytes.len() as u64 != stamp.size as u64
                {
                    bail!("destination file changed during capture");
                }
                let value = FileValue {
                    bytes,
                    mode: Some(stamp.mode & 0o7777),
                };
                return Ok(Self {
                    root,
                    directories,
                    last: Final::Present { file, stamp, value },
                });
            }
            if FileType::from_raw_mode(selected.st_mode) != FileType::Directory {
                bail!("destination ancestor is not a directory");
            }
            let opened = Directory::new(open_directory(parent, component)?, mount)?;
            if opened.identity != Identity::of(&selected) {
                bail!("destination directory changed during open");
            }
            directories.push(opened);
        }
        unreachable!("portable validation rejects empty destination paths")
    }
    fn verify_against(&self, current: &Self, mount: &Mount) -> Result<()> {
        if self.directories.len() != current.directories.len() {
            bail!("destination directory chain changed");
        }
        for (expected, actual) in self.directories.iter().zip(&current.directories) {
            expected.verify(mount)?;
            if expected.identity != actual.identity {
                bail!("destination directory identity changed");
            }
        }
        match (&self.last, &current.last) {
            (Final::Missing { suffix: expected }, Final::Missing { suffix: actual })
                if expected == actual =>
            {
                Ok(())
            }
            (
                Final::Present { file, stamp, value },
                Final::Present {
                    stamp: actual,
                    value: current,
                    ..
                },
            ) => {
                mount.require_same(&Mount::of(file)?)?;
                if stamp != actual
                    || *stamp
                        != Stamp::of(&fstat(file).context("revalidate retained destination file")?)
                {
                    bail!("destination file stamp changed");
                }
                verify_value(value, current)
            }
            _ => bail!("destination first-missing component or presence changed"),
        }
    }
}
pub(super) struct Evidence {
    root: Directory,
    mount: Mount,
    bindings: Vec<BoundRoot>,
    paths: Vec<PathEvidence>,
}
impl Evidence {
    pub(super) fn capture(
        scope: &QualifiedScope,
        locations: &[FileLocation],
    ) -> Result<(Self, Vec<ObservedFile>)> {
        let file = scope.reopen_directory()?;
        let mount = Mount::of(&file)?;
        let root = Directory::new(file, &mount)?;
        let mut bindings = Vec::<BoundRoot>::new();
        let mut indices = Vec::new();
        // Prove every supplied root before accessing any destination descendant.
        for location in locations {
            let binding = QualifiedScope::open(&location.root)?;
            let directory = Directory::new(binding.reopen_directory()?, &mount)?;
            if directory.identity != root.identity {
                bail!("destination root is a different physical scope");
            }
            indices.push(bindings.len());
            bindings.push(BoundRoot { binding, directory });
        }
        let mut paths = Vec::new();
        let mut observations = Vec::new();
        for (location, index) in locations.iter().zip(indices) {
            let path = PathEvidence::capture(
                index,
                &bindings[index].directory.file,
                &mount,
                &location.path,
            )
            .with_context(|| format!("capture destination {}", location.path))?;
            if let Final::Present { stamp, .. } = &path.last {
                for previous in &paths {
                    if let PathEvidence {
                        last: Final::Present { stamp: prior, .. },
                        ..
                    } = previous
                    {
                        if stamp.identity == prior.identity {
                            bail!("duplicate physical destination file identity");
                        }
                    }
                }
            }
            observations.push(ObservedFile {
                location: location.clone(),
                value: path.value(),
            });
            paths.push(path);
        }
        Ok((
            Self {
                root,
                mount,
                bindings,
                paths,
            },
            observations,
        ))
    }
    pub(super) fn verify(
        &self,
        scope: &QualifiedScope,
        observations: &[ObservedFile],
    ) -> Result<()> {
        self.root.verify(&self.mount)?;
        let root = Directory::new(scope.reopen_directory()?, &self.mount)?;
        if self.root.identity != root.identity {
            bail!("qualified physical scope changed");
        }
        let mut fresh_roots = Vec::new();
        for binding in &self.bindings {
            binding.directory.verify(&self.mount)?;
            let fresh = Directory::new(binding.binding.reopen_directory()?, &self.mount)?;
            if fresh.identity != self.root.identity || fresh.identity != binding.directory.identity
            {
                bail!("destination root binding changed");
            }
            fresh_roots.push(fresh);
        }
        for (path, observation) in self.paths.iter().zip(observations) {
            if let Final::Missing { suffix } = &path.last {
                let parent = path
                    .directories
                    .last()
                    .map_or(&self.bindings[path.root].directory.file, |directory| {
                        &directory.file
                    });
                verify_absence(parent, suffix)?;
            }
            let current = PathEvidence::capture(
                path.root,
                &fresh_roots[path.root].file,
                &self.mount,
                &observation.location.path,
            )
            .with_context(|| format!("revalidate destination {}", observation.location.path))?;
            path.verify_against(&current, &self.mount)?;
        }
        scope.verify_binding()?;
        for binding in &self.bindings {
            binding.binding.verify_binding()?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;
