//! Explicit private paths only; no inherited credentials or client options.
use super::super::{CopilotEvidenceError as Error, CopilotProbeEvidence};
use crate::filesystem::native_effects;
use crate::ownership::{InstallationRoot, OwnershipScope};
use rustix::fs::{Mode, OFlags, mkdirat, openat};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::path::Path;

pub(super) fn values(
    evidence: &CopilotProbeEvidence<'_>,
) -> Result<BTreeMap<OsString, OsString>, Error> {
    let home = evidence.root.path.join("home");
    let node = evidence
        .preflight
        .assets
        .get(1)
        .ok_or(Error::Preconditions)?
        .selected_path();
    let node_dir = node.parent().ok_or(Error::Preconditions)?;
    let path = std::env::join_paths([
        node_dir,
        Path::new("/usr/bin"),
        Path::new("/bin"),
        Path::new("/usr/sbin"),
        Path::new("/sbin"),
    ])
    .map_err(|_| Error::Preconditions)?;
    let mut vars = BTreeMap::new();
    for (key, value) in [
        ("HOME", home.clone()),
        ("USERPROFILE", home.clone()),
        ("COPILOT_HOME", home.join(".copilot")),
        ("COPILOT_CACHE_HOME", home.join("cache/copilot")),
        ("XDG_CONFIG_HOME", home.join("config")),
        ("XDG_CACHE_HOME", home.join("cache")),
        ("XDG_DATA_HOME", home.join("data")),
        ("XDG_STATE_HOME", home.join("state")),
        ("TMPDIR", home.join("tmp")),
    ] {
        vars.insert(key.into(), value.into_os_string());
    }
    vars.insert("PATH".into(), path);
    for (key, value) in [
        ("LC_ALL", "C"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_SYSTEM", "/dev/null"),
        ("GIT_ATTR_NOSYSTEM", "1"),
    ] {
        vars.insert(key.into(), value.into());
    }
    Ok(vars)
}
pub(super) fn prepare(evidence: &CopilotProbeEvidence<'_>) -> Result<(), Error> {
    let home = InstallationRoot {
        scope: OwnershipScope::User,
        path: evidence.root.path.join("home"),
    };
    let (scope, guard) = evidence
        .locks
        .scope_and_guard(&home)
        .map_err(|_| Error::Changed)?;
    let base = native_effects::command_directory(scope, guard).map_err(|_| Error::Changed)?;
    fn new_dir(base: &File, name: &str) -> Result<File, Error> {
        mkdirat(base, name, Mode::RWXU).map_err(|_| Error::PublicationUncertain)?;
        let file = File::from(
            openat(
                base,
                name,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| Error::PublicationUncertain)?,
        );
        super::super::files::Directory::of(&file).map_err(|_| Error::PublicationUncertain)?;
        file.sync_all().map_err(|_| Error::PublicationUncertain)?;
        base.sync_all().map_err(|_| Error::PublicationUncertain)?;
        Ok(file)
    }
    for name in [".copilot", "config", "data", "state", "tmp"] {
        new_dir(&base, name)?;
    }
    let cache = new_dir(&base, "cache")?;
    new_dir(&cache, "copilot")?;
    Ok(())
}
