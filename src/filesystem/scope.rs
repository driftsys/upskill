use std::fmt;

use anyhow::Result;

use crate::ownership::InstallationRoot;

/// An existing directory pinned to its selected native filesystem identity.
///
/// Linux and macOS are supported. Other platforms return an explicit error.
/// Ancestor aliases are allowed at selection; this is not descendant containment
/// or protection against external namespace changes. No filesystem state is written.
pub struct QualifiedScope {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    root: backend::PinnedRoot,
}

/// A nonblocking advisory exclusive writer lock borrowing its qualified scope.
///
/// Drop closes the independent lock description. Ordinary exec does not inherit
/// it. This is not a fork-safe lease or exclusion against non-cooperating editors.
pub struct ScopeWriterGuard<'scope> {
    scope: &'scope QualifiedScope,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    _lock: std::fs::File,
}

impl fmt::Debug for QualifiedScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QualifiedScope").finish_non_exhaustive()
    }
}

impl fmt::Debug for ScopeWriterGuard<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScopeWriterGuard").finish_non_exhaustive()
    }
}

impl QualifiedScope {
    /// Select and pin an absolute existing directory without a final symlink.
    /// Parent components are rejected before any filesystem access.
    pub fn open(root: &InstallationRoot) -> Result<Self> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            Ok(Self {
                root: backend::PinnedRoot::open(&root.path)?,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = root;
            anyhow::bail!("qualified scope roots are unsupported on this operating system")
        }
    }

    /// Revalidate the selected path, canonical path and pinned identity.
    /// A successful check does not freeze the namespace after it returns.
    pub fn verify_binding(&self) -> Result<()> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.root.verify_binding()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        anyhow::bail!("qualified scope roots are unsupported on this operating system")
    }

    /// Acquire a fresh nonblocking exclusive directory lock, or return an error.
    /// Contention returns a busy error; filesystem failures retain their cause.
    pub fn try_lock_writer(&self) -> Result<ScopeWriterGuard<'_>> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.verify_binding()?;
            let lock = self.root.try_lock_writer()?;
            self.verify_binding()?;
            Ok(ScopeWriterGuard {
                scope: self,
                _lock: lock,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        anyhow::bail!("qualified scope writer locking is unsupported on this operating system")
    }
}

impl ScopeWriterGuard<'_> {
    /// Revalidate the scope binding while retaining writer exclusion.
    pub fn verify_binding(&self) -> Result<()> {
        self.scope.verify_binding()
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod backend {
    use std::fs::{self, File, Metadata};
    use std::os::unix::fs::MetadataExt;
    use std::path::{Component, Path, PathBuf};

    use anyhow::{Context, Result, bail};
    use rustix::fs::{FlockOperation, Mode, OFlags, flock, open, openat};
    use rustix::io::Errno;

    #[derive(PartialEq, Eq)]
    struct Identity {
        device: u64,
        inode: u64,
    }

    impl Identity {
        fn of(metadata: &Metadata) -> Self {
            Self {
                device: metadata.dev(),
                inode: metadata.ino(),
            }
        }
    }

    pub(super) struct PinnedRoot {
        selected: PathBuf,
        resolved: PathBuf,
        identity: Identity,
        directory: File,
    }

    fn flags() -> OFlags {
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
    }

    fn open_directory(path: &Path) -> Result<File> {
        let descriptor = open(path, flags(), Mode::empty())
            .context("open qualified scope directory without following its final component")?;
        Ok(File::from(descriptor))
    }

    fn select(path: &Path) -> Result<(PathBuf, Identity, File)> {
        let metadata = fs::symlink_metadata(path).context("inspect selected scope root")?;
        if !metadata.is_dir() || metadata.is_symlink() {
            bail!("selected scope root must be a directory without a final symlink");
        }
        let identity = Identity::of(&metadata);
        let selected_file = open_directory(path)?;
        let resolved = fs::canonicalize(path).context("resolve selected scope root")?;
        let directory = open_directory(&resolved)?;
        for file in [&selected_file, &directory] {
            let opened = file.metadata().context("inspect opened scope root")?;
            if !opened.is_dir() || Identity::of(&opened) != identity {
                bail!("selected scope root changed during qualification");
            }
        }
        Ok((resolved, identity, directory))
    }

    impl PinnedRoot {
        pub(super) fn open(path: &Path) -> Result<Self> {
            if path.as_os_str().is_empty() || !path.is_absolute() {
                bail!("scope root must be a nonempty absolute path");
            }
            if path
                .components()
                .any(|component| component == Component::ParentDir)
            {
                bail!("scope root must not contain parent components");
            }
            // Components remove dot and trailing separators before final-link checks.
            let selected: PathBuf = path.components().collect();
            let (resolved, identity, directory) = select(&selected)?;
            Ok(Self {
                selected,
                resolved,
                identity,
                directory,
            })
        }

        pub(super) fn verify_binding(&self) -> Result<()> {
            let (resolved, identity, _) =
                select(&self.selected).context("revalidate qualified scope root binding")?;
            let held = self
                .directory
                .metadata()
                .context("inspect pinned scope directory")?;
            if resolved != self.resolved
                || identity != self.identity
                || Identity::of(&held) != self.identity
            {
                bail!("qualified scope root binding changed");
            }
            Ok(())
        }

        pub(super) fn try_lock_writer(&self) -> Result<File> {
            // A cloned descriptor shares flock state. Reopen relative to the pinned
            // directory to give every guard an independent open description.
            let descriptor = openat(&self.directory, ".", flags(), Mode::empty())
                .context("open independent scope writer lock description")?;
            let lock = File::from(descriptor);
            let metadata = lock
                .metadata()
                .context("inspect scope writer lock directory")?;
            if !metadata.is_dir() || Identity::of(&metadata) != self.identity {
                bail!("scope writer lock directory identity changed");
            }
            flock(&lock, FlockOperation::NonBlockingLockExclusive).map_err(|error| {
                if error == Errno::WOULDBLOCK {
                    anyhow::Error::new(error).context("scope writer is busy")
                } else {
                    anyhow::Error::new(error).context("acquire scope writer lock")
                }
            })?;
            Ok(lock)
        }
    }
}
