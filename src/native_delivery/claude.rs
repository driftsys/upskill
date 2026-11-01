//! Claude plugin-inventory decoding for the retained 2.1.308 wire shape.
//!
//! These are native metadata claims, not ownership, effective activation,
//! resource retention or profile qualification. The decoder performs no I/O;
//! the candidate reader invokes fixed metadata commands. Production admission
//! remains separate.
use serde::Deserialize;
use std::fmt;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ClaudeInventoryError {
    #[error(
        "Claude inventory version is unsupported; capture and review the exact native contract"
    )]
    UnsupportedVersion,
    #[error("Claude inventory exceeds the bounded capture limit")]
    TooLarge,
    #[error(
        "Claude inventory is malformed or incomplete; no absence or installation is established"
    )]
    Malformed,
    #[error("Claude inventory scope is unsupported or ambiguous; verify the qualified context")]
    AmbiguousScope,
    #[error(
        "Claude inventory contains multiple matching entries; do not adopt a first-found result"
    )]
    AmbiguousEntry,
    #[error("Claude marketplace source conflicts with the requested directory")]
    AmbiguousSource,
    #[error("Claude installed-plugin registry schema is unsupported")]
    UnsupportedRegistryVersion,
}

/// Only allowlisted metadata is retained. Unknown values and raw parse errors
/// are discarded. Paths are reported strings; decoding does not qualify them.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudePluginEntry {
    pub id: String,
    pub version: String,
    pub scope: String,
    pub enabled: bool,
    pub install_path: PathBuf,
    pub read_from_folder: Option<PathBuf>,
    pub folder_version: Option<String>,
    pub project_path: Option<PathBuf>,
    pub project_enabled: Option<bool>,
}
impl fmt::Debug for ClaudePluginEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudePluginEntry").finish_non_exhaustive()
    }
}
pub enum ClaudeLookupScope<'a> {
    User,
    Project(&'a Path),
}
/// Metadata projection only. It deliberately has no serde representation and
/// cannot be supplied as a native binding/effect/retention receipt.
///
/// ```compile_fail
/// use upskill::native_delivery::claude::ClaudePluginInventory;
/// let inventory = ClaudePluginInventory::parse("2.1.308", b"[]").unwrap();
/// let _ = serde_json::to_vec(&inventory);
/// ```
pub struct ClaudePluginInventory {
    entries: Vec<ClaudePluginEntry>,
}
impl fmt::Debug for ClaudePluginInventory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudePluginInventory")
            .field("entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}
fn invalid_text(value: &str) -> bool {
    value.is_empty() || value.contains('\0')
}
fn invalid_path(value: &Path) -> bool {
    value.to_str().is_none_or(invalid_text)
}
fn absolute_project(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|component| component == Component::ParentDir)
}
fn parse_capture(version: &str, bytes: &[u8]) -> Result<serde_json::Value, ClaudeInventoryError> {
    if version != "2.1.308" {
        return Err(ClaudeInventoryError::UnsupportedVersion);
    }
    if bytes.len() > 1024 * 1024 {
        return Err(ClaudeInventoryError::TooLarge);
    }
    let source = std::str::from_utf8(bytes).map_err(|_| ClaudeInventoryError::Malformed)?;
    crate::parse::json::parse_unique(source).map_err(|_| ClaudeInventoryError::Malformed)
}
impl ClaudePluginInventory {
    /// Decode the captured CLI list shape, rejecting decoded duplicate keys even
    /// in ignored fields. This pins syntax, not support for the executable pin.
    pub fn parse(version: &str, bytes: &[u8]) -> Result<Self, ClaudeInventoryError> {
        let value = parse_capture(version, bytes)?;
        let entries: Vec<ClaudePluginEntry> =
            serde_json::from_value(value).map_err(|_| ClaudeInventoryError::Malformed)?;
        for entry in &entries {
            if invalid_text(&entry.id)
                || invalid_text(&entry.version)
                || invalid_text(&entry.scope)
                || invalid_path(&entry.install_path)
                || entry.read_from_folder.as_deref().is_some_and(invalid_path)
                || entry.folder_version.as_deref().is_some_and(invalid_text)
                || entry.project_path.as_deref().is_some_and(invalid_path)
                || (entry.scope == "project"
                    && (entry.project_path.is_none() || entry.project_enabled.is_none()))
            {
                return Err(ClaudeInventoryError::Malformed);
            }
        }
        Ok(Self { entries })
    }
    pub fn entries(&self) -> &[ClaudePluginEntry] {
        &self.entries
    }

    /// Find exact metadata in one requested scope. Project paths compare as
    /// reported; filesystem alias resolution, observer freshness and competing
    /// readers remain obligations of the separately qualified adapter.
    /// None means no matching row in this supplied inventory, not native absence.
    pub fn find(
        &self,
        selector: &str,
        requested: ClaudeLookupScope<'_>,
    ) -> Result<Option<&ClaudePluginEntry>, ClaudeInventoryError> {
        if let ClaudeLookupScope::Project(path) = requested {
            if !absolute_project(path) {
                return Err(ClaudeInventoryError::AmbiguousScope);
            }
        }
        let mut found = None;
        for entry in self.entries.iter().filter(|entry| entry.id == selector) {
            let matches = match entry.scope.as_str() {
                "user" => matches!(requested, ClaudeLookupScope::User),
                "project" => {
                    let path = entry
                        .project_path
                        .as_deref()
                        .ok_or(ClaudeInventoryError::AmbiguousScope)?;
                    if !absolute_project(path) {
                        return Err(ClaudeInventoryError::AmbiguousScope);
                    }
                    matches!(requested, ClaudeLookupScope::Project(wanted) if path == wanted)
                }
                _ => return Err(ClaudeInventoryError::AmbiguousScope),
            };
            if matches {
                if found.is_some() {
                    return Err(ClaudeInventoryError::AmbiguousEntry);
                }
                found = Some(entry);
            }
        }
        Ok(found)
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod observe;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use observe::{
    ClaudeInventoryCapture, ClaudeInventoryReadError, ClaudeMarketplaceCapture,
    read_marketplace_inventory, read_plugin_inventory,
};

mod commands;
pub use commands::{
    ClaudeCommandError, ClaudePluginAction, ClaudePluginCommand, plan_plugin_command,
};

mod registry;
pub use registry::{
    ClaudeInstalledEntry, ClaudeInstalledRegistry, ClaudeKnownMarketplaces, ClaudeMarketplaceEntry,
    ClaudeMarketplaceInventory,
};
