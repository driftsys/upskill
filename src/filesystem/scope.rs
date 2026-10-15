use std::fmt;

use anyhow::Result;

use crate::ownership::InstallationRoot;

/// An existing directory pinned to its selected native filesystem identity.
///
/// Linux and macOS are supported. Other platforms return an explicit error.
/// Ancestor aliases are allowed at selection; this is not descendant containment
/// or protection against external namespace changes. No filesystem state is written.
///
/// Callers cannot construct a scope directly on any platform:
///
/// ```compile_fail
/// use upskill::filesystem::QualifiedScope;
/// let _scope = QualifiedScope {};
/// ```
pub struct QualifiedScope {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    root: backend::PinnedRoot,
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    _unsupported: (),
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(super) fn reopen_directory(&self) -> Result<std::fs::File> {
        self.root.reopen_directory()
    }

    /// Acquire a fresh nonblocking exclusive directory lock, or return an error.
    /// Contention returns a busy error; filesystem failures retain their cause.
    pub fn try_lock_writer(&self) -> Result<ScopeWriterGuard<'_>> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.try_lock_writer_with(backend::PinnedRoot::try_lock_writer)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        anyhow::bail!("qualified scope writer locking is unsupported on this operating system")
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn try_lock_writer_with(
        &self,
        acquire: impl FnOnce(&backend::PinnedRoot) -> Result<std::fs::File>,
    ) -> Result<ScopeWriterGuard<'_>> {
        self.verify_binding()?;
        let lock = acquire(&self.root)?;
        self.verify_binding()?;
        Ok(ScopeWriterGuard {
            scope: self,
            _lock: lock,
        })
    }
}

impl ScopeWriterGuard<'_> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(in crate::filesystem) fn require_scope(&self, scope: &QualifiedScope) -> Result<()> {
        anyhow::ensure!(
            std::ptr::eq(self.scope, scope),
            "writer guard belongs to a different qualified scope"
        );
        self.verify_binding()
    }

    /// Revalidate the scope binding while retaining writer exclusion.
    pub fn verify_binding(&self) -> Result<()> {
        self.scope.verify_binding()
    }
}

#[cfg(all(test, not(any(target_os = "linux", target_os = "macos"))))]
mod unsupported_tests {
    use super::QualifiedScope;
    #[test]
    fn execution_and_recovery_return_explicit_unsupported_errors() {
        use crate::files::plan::{FileLocation, PlannedFile};
        use crate::filesystem::execution::ExecutionRequest;
        use crate::ownership::{InstallationRoot, OwnershipScope};
        let scope = QualifiedScope { _unsupported: () };
        let guard = super::ScopeWriterGuard { scope: &scope };
        let request = ExecutionRequest {
            files: vec![],
            state: PlannedFile {
                location: FileLocation {
                    root: InstallationRoot {
                        scope: OwnershipScope::Project,
                        path: std::env::temp_dir(),
                    },
                    path: "state".into(),
                },
                before: None,
                after: None,
            },
        };
        assert!(
            scope
                .apply_operation(&guard, &[], request)
                .unwrap_err()
                .to_string()
                .contains("unsupported")
        );
        assert!(
            scope
                .recover_operation(&guard)
                .unwrap_err()
                .to_string()
                .contains("unsupported")
        );
    }

    #[test]
    fn source_observation_returns_explicit_unsupported_error() {
        let scope = QualifiedScope { _unsupported: () };
        let result = scope.observe_source_tree();
        assert!(result.is_err(), "unsupported host returned source evidence");
        let error = format!("{:#}", result.unwrap_err());
        assert!(
            error.contains("qualified source observation") && error.contains("unsupported"),
            "unsupported source observation lost operation/platform context: {error}"
        );
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
            self.reopen_directory().map(drop)
        }

        pub(super) fn reopen_directory(&self) -> Result<File> {
            let (resolved, identity, directory) =
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
            Ok(directory)
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

    #[cfg(test)]
    mod tests {
        use std::fs;
        use std::os::unix::fs::symlink;

        use super::open_directory;

        #[test]
        fn directory_open_refuses_final_symlink_independently_of_selection() {
            let tmp = tempfile::tempdir().unwrap();
            let directory = tmp.path().join("directory");
            let alias = tmp.path().join("alias");
            fs::create_dir(&directory).unwrap();
            fs::write(directory.join("data"), b"unchanged").unwrap();
            symlink("directory", &alias).unwrap();
            drop(open_directory(&directory).unwrap());
            assert!(
                open_directory(&alias).is_err(),
                "directory open followed final symlink"
            );
            assert_eq!(
                fs::read_link(&alias).unwrap(),
                std::path::Path::new("directory")
            );
            assert_eq!(fs::read(directory.join("data")).unwrap(), b"unchanged");
            assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 2);
        }
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use std::fs;

    use super::QualifiedScope;
    use crate::ownership::{InstallationRoot, OwnershipScope};

    #[test]
    fn namespace_replacement_during_acquisition_rejects_guard_and_releases_lock() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("scope");
        let moved = tmp.path().join("moved");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("data"), b"exact\0bytes\xff\n").unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: path.clone(),
        };
        let scope = QualifiedScope::open(&root).unwrap();
        let result = scope.try_lock_writer_with(|pinned| {
            // The initial check has completed. Replace the namespace before the
            // real lock completes on the original pinned directory.
            fs::rename(&path, &moved)?;
            fs::create_dir(&path)?;
            pinned.try_lock_writer()
        });
        assert!(
            result.is_err(),
            "replacement during acquisition returned guard"
        );
        let error = result.unwrap_err();
        assert!(error.to_string().contains("binding"), "{error:#}");
        assert_eq!(fs::read(moved.join("data")).unwrap(), b"exact\0bytes\xff\n");
        assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 2);
        let moved_scope = QualifiedScope::open(&InstallationRoot {
            path: moved,
            ..root
        })
        .unwrap();
        // Other library tests can fork concurrently, temporarily inheriting a
        // CLOEXEC description before exec. That transient lease is unsupported;
        // a persistent descriptor leak must still fail this bounded check.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match moved_scope.try_lock_writer() {
                Ok(guard) => {
                    drop(guard);
                    break;
                }
                Err(error) => {
                    assert!(
                        error.to_string().contains("busy") && std::time::Instant::now() < deadline,
                        "rejected acquisition must release its real lock: {error:#}"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
    }
}
