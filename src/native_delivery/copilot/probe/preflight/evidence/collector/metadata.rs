//! Presence/type/mode samples at fixed paths; native file contents stay unread.
use super::super::CopilotProbeEvidence;
use crate::filesystem::native_effects;
use crate::ownership::{InstallationRoot, OwnershipScope};
use rustix::fs::{AtFlags, FileType, Mode, OFlags, openat, statat};
use serde_json::{Value, json};
use std::fs::File;

pub(super) fn sample(evidence: &CopilotProbeEvidence<'_>) -> Result<Value, &'static str> {
    fn at(base: &File, parts: &[&str]) -> Result<Value, &'static str> {
        let mut parent = base.try_clone().map_err(|_| "native_metadata_unknown")?;
        for (index, name) in parts.iter().enumerate() {
            let stat = match statat(&parent, *name, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(stat) => stat,
                Err(rustix::io::Errno::NOENT) => return Ok(json!({"type":"absent"})),
                Err(_) => return Err("native_metadata_unknown"),
            };
            let kind = FileType::from_raw_mode(stat.st_mode);
            let label = match kind {
                FileType::RegularFile => "regular",
                FileType::Directory => "directory",
                FileType::Symlink => "symlink",
                _ => "other",
            };
            if matches!(label, "symlink" | "other") {
                return Err("native_metadata_unsafe");
            }
            if index + 1 == parts.len() {
                return Ok(json!({"type":label,"mode":stat.st_mode & 0o7777}));
            }
            if kind != FileType::Directory {
                return Err("native_metadata_unsafe");
            }
            parent = File::from(
                openat(
                    &parent,
                    *name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|_| "native_metadata_unsafe")?,
            );
        }
        Err("native_metadata_unknown")
    }
    let mut records = Vec::new();
    for (scope_kind, subdir, path, parts) in [
        (
            OwnershipScope::User,
            "home",
            "home/.copilot",
            vec![".copilot"],
        ),
        (
            OwnershipScope::User,
            "home",
            "home/.copilot/config.json",
            vec![".copilot", "config.json"],
        ),
        (
            OwnershipScope::User,
            "home",
            "home/cache/copilot",
            vec!["cache", "copilot"],
        ),
        (
            OwnershipScope::Project,
            "project",
            "project/.git/config",
            vec![".git", "config"],
        ),
    ] {
        let root = InstallationRoot {
            scope: scope_kind,
            path: evidence.root.path.join(subdir),
        };
        let (scope, guard) = evidence
            .locks
            .scope_and_guard(&root)
            .map_err(|_| "native_metadata_unknown")?;
        let base = native_effects::command_directory(scope, guard)
            .map_err(|_| "native_metadata_unknown")?;
        records.push(json!({"path":path,"metadata":at(&base,&parts)?}));
    }
    Ok(json!(records))
}
