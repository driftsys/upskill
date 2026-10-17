use crate::files::plan::{FileLocation, ObservedFile, PlannedFile};
use crate::filesystem::{QualifiedScope, qualification::Mount};
use anyhow::{Context, Result, ensure};
use rustix::fs::{
    AtFlags, Dir, FileType, Mode, OFlags, RenameFlags, fstat, mkdirat, openat, renameat,
    renameat_with, statat, unlinkat,
};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

type ExtendedMetadata = Vec<(Vec<u8>, Vec<u8>)>;
#[cfg(test)]
type SyncHook = Box<dyn FnMut(&File, &'static str) -> Result<()>>;
#[cfg(test)]
thread_local! {
    pub(super) static TEST_SYNC: std::cell::RefCell<Option<SyncHook>> = std::cell::RefCell::new(None);
}
fn sync_recovery(file: &File, kind: &'static str) -> Result<()> {
    #[cfg(test)]
    if let Some(result) =
        TEST_SYNC.with(|hook| hook.borrow_mut().as_mut().map(|sync| sync(file, kind)))
    {
        return result;
    }
    let _ = kind;
    file.sync_all()?;
    Ok(())
}
pub(super) fn require_plain_metadata(file: &File) -> Result<ExtendedMetadata> {
    let metadata = file.metadata()?;
    ensure!(
        metadata.mode() & 0o7000 == 0,
        "special permission bits are not qualified for execution"
    );
    let mut buffer = vec![0u8; 65536];
    let length = rustix::fs::flistxattr(file, &mut buffer[..])
        .context("extended metadata cannot be qualified")?;
    #[allow(unused_mut)]
    let mut attributes = Vec::new();
    #[cfg(target_os = "linux")]
    ensure!(
        buffer[..length]
            .split(|b| *b == 0)
            .all(|name| name.is_empty()),
        "extended attributes are unsupported for execution"
    );
    #[cfg(target_os = "macos")]
    for name in buffer[..length]
        .split(|b| *b == 0)
        .filter(|v| !v.is_empty())
    {
        #[cfg(target_os = "macos")]
        {
            ensure!(
                name == b"com.apple.provenance",
                "extended attributes are unsupported for execution"
            );
            let mut value = vec![0u8; 65536];
            let length = rustix::fs::fgetxattr(file, "com.apple.provenance", &mut value[..])?;
            value.truncate(length);
            attributes.push((name.to_vec(), value));
        }
    }
    #[cfg(target_os = "macos")]
    {
        super::macos_acl::require_empty(file)?;
        ensure!(
            std::os::macos::fs::MetadataExt::st_flags(&metadata) == 0,
            "native file flags are unsupported for execution"
        );
    }
    Ok(attributes)
}
pub(super) fn qualify_metadata(scope: &QualifiedScope, location: &FileLocation) -> Result<()> {
    let snapshot = scope.observe_destinations(std::slice::from_ref(location))?;
    let mut dir = scope.reopen_directory()?;
    let mount = Mount::of(&dir, Mount::queries())?;
    require_plain_metadata(&dir)?;
    let parts: Vec<_> = location.path.split('/').collect();
    for part in &parts[..parts.len() - 1] {
        let next = match directory(&dir, part) {
            Ok(next) => next,
            Err(error)
                if error.downcast_ref::<rustix::io::Errno>() == Some(&rustix::io::Errno::NOENT) =>
            {
                break;
            }
            Err(error) => return Err(error),
        };
        spelling(&dir, part)?;
        mount.require_same(&Mount::of(&next, Mount::queries())?)?;
        require_plain_metadata(&next)?;
        dir = next;
    }
    if snapshot.observations()[0].value.is_some() {
        let (dir, name) = parent(scope, location, false)?;
        let file = File::from(openat(
            &dir,
            name.as_str(),
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )?);
        require_plain_metadata(&file)?;
        let root = scope.reopen_directory()?.metadata()?;
        ensure!(
            file.metadata()?.uid() == root.uid() && file.metadata()?.gid() == root.gid(),
            "destination ownership differs from the qualified scope"
        );
    }
    snapshot.verify_unchanged()
}
pub(super) fn nonce() -> Result<String> {
    let mut bytes = [0u8; 16];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub(super) fn directory(parent: &File, name: &str) -> Result<File> {
    Ok(File::from(openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?))
}
pub(super) fn spelling(parent: &File, name: &str) -> Result<()> {
    for entry in Dir::read_from(parent)? {
        if entry?.file_name().to_bytes() == name.as_bytes() {
            return Ok(());
        }
    }
    anyhow::bail!("directory component spelling changed: {name}")
}
pub(super) fn identity(file: &File) -> Result<(u64, u64)> {
    let m = file.metadata()?;
    Ok((m.dev(), m.ino()))
}
fn parent(scope: &QualifiedScope, location: &FileLocation, create: bool) -> Result<(File, String)> {
    crate::ownership::validate_relative_path(&location.path)?;
    let mut dir = scope.reopen_directory()?;
    let mount = Mount::of(&dir, Mount::queries())?;
    let parts: Vec<_> = location.path.split('/').collect();
    for part in &parts[..parts.len() - 1] {
        if create {
            match mkdirat(&dir, *part, Mode::from_raw_mode(0o700)) {
                Ok(()) => dir.sync_all()?,
                Err(rustix::io::Errno::EXIST) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let next =
            directory(&dir, part).context("open execution parent without following links")?;
        spelling(&dir, part)?;
        mount.require_same(&Mount::of(&next, Mount::queries())?)?;
        require_plain_metadata(&next)?;
        dir = next;
    }
    scope.verify_binding()?;
    Ok((dir, parts.last().unwrap().to_string()))
}
pub(super) fn observe(
    scope: &QualifiedScope,
    locations: &[FileLocation],
) -> Result<Vec<ObservedFile>> {
    let snapshot = scope.observe_destinations(locations)?;
    locations
        .iter()
        .map(|location| {
            snapshot
                .observations()
                .iter()
                .find(|v| v.location == *location)
                .cloned()
                .context("qualified observation missing requested location")
        })
        .collect()
}
pub(super) fn sync_post(
    scope: &QualifiedScope,
    location: &FileLocation,
    expected: &Option<crate::files::plan::FileValue>,
) -> Result<()> {
    let snapshot = scope.observe_destinations(std::slice::from_ref(location))?;
    ensure!(
        snapshot.observations()[0].value == *expected,
        "post-state changed before durable acknowledgement"
    );
    let (dir, name) = parent(scope, location, false)?;
    if expected.is_some() {
        let file = File::from(openat(
            &dir,
            name.as_str(),
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )?);
        require_plain_metadata(&file)?;
        sync_recovery(&file, "file")?;
    }
    sync_recovery(&dir, "directory")?;
    snapshot.verify_unchanged()
}
pub(super) fn apply(scope: &QualifiedScope, action: &PlannedFile, index: usize) -> Result<()> {
    apply_named(scope, action, &nonce()?, index)
}
pub(super) fn apply_create_only(
    scope: &QualifiedScope,
    action: &PlannedFile,
    index: usize,
) -> Result<()> {
    ensure!(
        action.after.is_some() && (action.before.is_none() || action.before == action.after),
        "create-only action cannot replace or remove content"
    );
    apply_named_with_policy(scope, action, &nonce()?, index, true)
}
fn apply_named(
    scope: &QualifiedScope,
    action: &PlannedFile,
    token: &str,
    index: usize,
) -> Result<()> {
    apply_named_with_policy(scope, action, token, index, false)
}
fn apply_named_with_policy(
    scope: &QualifiedScope,
    action: &PlannedFile,
    token: &str,
    index: usize,
    create_only: bool,
) -> Result<()> {
    qualify_metadata(scope, &action.location)?;
    let locations = [action.location.clone()];
    let before = scope.observe_destinations(&locations)?;
    ensure!(
        before.observations()[0].value == action.before,
        "destination changed before execution action"
    );
    if action.before == action.after {
        before.verify_unchanged()?;
        return Ok(());
    }
    let (dir, name) = parent(scope, &action.location, action.after.is_some())?;
    let snapshot = scope.observe_destinations(&locations)?;
    ensure!(
        snapshot.observations()[0].value == action.before,
        "destination changed while preparing execution parent"
    );
    let temp = format!(".upskill-stage-{token}");
    let mut owned_temp = None;
    let result = (|| -> Result<()> {
        if let Some(after) = &action.after {
            let mut file = File::from(openat(
                &dir,
                temp.as_str(),
                OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            )?);
            owned_temp = Some(identity(&file)?);
            require_plain_metadata(&file)?;
            if action.before.is_some() {
                let prior = File::from(openat(
                    &dir,
                    name.as_str(),
                    OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )?);
                ensure!(
                    prior.metadata()?.uid() == file.metadata()?.uid()
                        && prior.metadata()?.gid() == file.metadata()?.gid(),
                    "replacement would change file ownership"
                );
                ensure!(
                    require_plain_metadata(&prior)? == require_plain_metadata(&file)?,
                    "replacement would alter supported extended metadata"
                );
            }
            file.write_all(&after.bytes)?;
            let mode = after.mode.context("execution mode must be known")?;
            ensure!(mode <= 0o777, "unsupported execution permission bits");
            file.set_permissions(std::fs::Permissions::from_mode(mode))?;
            file.sync_all()?;
            file.rewind()?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            let stat = statat(&dir, temp.as_str(), AtFlags::SYMLINK_NOFOLLOW)?;
            let held = fstat(&file)?;
            ensure!(
                stat.st_dev == held.st_dev
                    && stat.st_ino == held.st_ino
                    && stat.st_nlink == 1
                    && FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile
                    && bytes == after.bytes
                    && file.metadata()?.mode() & 0o7777 == mode,
                "execution staging evidence changed"
            );
            snapshot.verify_unchanged()?;
            let (current, _) = parent(scope, &action.location, false)?;
            ensure!(
                identity(&current)? == identity(&dir)?,
                "execution parent binding changed"
            );
            super::checkpoint("before_effect", index)?;
            if create_only {
                renameat_with(
                    &dir,
                    temp.as_str(),
                    &dir,
                    name.as_str(),
                    RenameFlags::NOREPLACE,
                )?;
            } else {
                renameat(&dir, temp.as_str(), &dir, name.as_str())?;
            }
        } else {
            snapshot.verify_unchanged()?;
            let (current, _) = parent(scope, &action.location, false)?;
            ensure!(
                identity(&current)? == identity(&dir)?,
                "execution parent binding changed"
            );
            super::checkpoint("before_effect", index)?;
            unlinkat(&dir, name.as_str(), AtFlags::empty())?;
        }
        super::checkpoint("after_effect", index)?;
        dir.sync_all()?;
        super::checkpoint("after_effect_sync", index)?;
        ensure!(
            observe(scope, &locations)?[0].value == action.after,
            "execution post-state differs from desired output"
        );
        Ok(())
    })();
    if result.is_err() {
        // This exclusive staging name belongs to this invocation only.
        if let Some(expected) = owned_temp {
            if statat(&dir, temp.as_str(), AtFlags::SYMLINK_NOFOLLOW).is_ok_and(|v| {
                #[allow(clippy::unnecessary_cast)]
                let id = (v.st_dev as u64, v.st_ino as u64);
                id == expected
                    && v.st_nlink == 1
                    && FileType::from_raw_mode(v.st_mode) == FileType::RegularFile
            }) {
                let _ = unlinkat(&dir, temp.as_str(), AtFlags::empty());
            }
        }
    }
    result
}
#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;
