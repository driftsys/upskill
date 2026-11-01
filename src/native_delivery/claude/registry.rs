//! Independent metadata projections of CLI marketplace output and registry files.
//! No view encodes file presence, exact bytes/modes, ownership or native effects.
use super::{ClaudeInventoryError, ClaudeLookupScope};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeMarketplaceEntry {
    pub name: String,
    pub source: String,
    pub path: Option<PathBuf>,
    pub install_location: PathBuf,
}
impl fmt::Debug for ClaudeMarketplaceEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudeMarketplaceEntry")
            .finish_non_exhaustive()
    }
}
/// Metadata projection only; original file bytes, modes and presence must be
/// captured separately under an approved non-secret evidence contract.
///
/// ```compile_fail
/// use upskill::native_delivery::claude::ClaudeMarketplaceInventory;
/// let view = ClaudeMarketplaceInventory::parse("2.1.308", br#"[]"#).unwrap();
/// let _ = serde_json::to_vec(&view);
/// ```
pub struct ClaudeMarketplaceInventory {
    entries: Vec<ClaudeMarketplaceEntry>,
}
/// Metadata projection only; original file bytes, modes and presence must be
/// captured separately under an approved non-secret evidence contract.
///
/// ```compile_fail
/// use upskill::native_delivery::claude::ClaudeKnownMarketplaces;
/// let view = ClaudeKnownMarketplaces::parse("2.1.308", br#"{}"#).unwrap();
/// let _ = serde_json::to_vec(&view);
/// ```
pub struct ClaudeKnownMarketplaces {
    entries: Vec<ClaudeMarketplaceEntry>,
}
impl fmt::Debug for ClaudeMarketplaceInventory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudeMarketplaceInventory")
            .field("entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}
impl fmt::Debug for ClaudeKnownMarketplaces {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudeKnownMarketplaces")
            .field("entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}
impl ClaudeMarketplaceInventory {
    /// Decode marketplace CLI list output. No registry-file facts are inferred.
    pub fn parse(version: &str, bytes: &[u8]) -> Result<Self, ClaudeInventoryError> {
        let entries: Vec<ClaudeMarketplaceEntry> =
            serde_json::from_value(super::parse_capture(version, bytes)?)
                .map_err(|_| ClaudeInventoryError::Malformed)?;
        validate_marketplaces(&entries)?;
        Ok(Self { entries })
    }
    pub fn entries(&self) -> &[ClaudeMarketplaceEntry] {
        &self.entries
    }
    /// None means no exact name in this metadata, not native absence.
    pub fn find_directory(
        &self,
        name: &str,
        directory: &Path,
    ) -> Result<Option<&ClaudeMarketplaceEntry>, ClaudeInventoryError> {
        find_directory(&self.entries, name, directory)
    }
}
impl ClaudeKnownMarketplaces {
    /// Decode known_marketplaces.json; timestamps and unapproved fields are discarded.
    pub fn parse(version: &str, bytes: &[u8]) -> Result<Self, ClaudeInventoryError> {
        #[derive(Deserialize)]
        struct Source {
            source: String,
            path: Option<PathBuf>,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Registered {
            source: Source,
            install_location: PathBuf,
        }
        let records: BTreeMap<String, Registered> =
            serde_json::from_value(super::parse_capture(version, bytes)?)
                .map_err(|_| ClaudeInventoryError::Malformed)?;
        let entries = records
            .into_iter()
            .map(|(name, record)| ClaudeMarketplaceEntry {
                name,
                source: record.source.source,
                path: record.source.path,
                install_location: record.install_location,
            })
            .collect::<Vec<_>>();
        validate_marketplaces(&entries)?;
        Ok(Self { entries })
    }
    pub fn entries(&self) -> &[ClaudeMarketplaceEntry] {
        &self.entries
    }
    /// None means no exact name in this metadata, not native absence.
    pub fn find_directory(
        &self,
        name: &str,
        directory: &Path,
    ) -> Result<Option<&ClaudeMarketplaceEntry>, ClaudeInventoryError> {
        find_directory(&self.entries, name, directory)
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeInstalledEntry {
    pub scope: String,
    pub version: String,
    pub install_path: PathBuf,
    pub project_path: Option<PathBuf>,
}
impl fmt::Debug for ClaudeInstalledEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudeInstalledEntry")
            .finish_non_exhaustive()
    }
}
/// Metadata projection only; original file bytes, modes and presence must be
/// captured separately under an approved non-secret evidence contract.
///
/// ```compile_fail
/// use upskill::native_delivery::claude::ClaudeInstalledRegistry;
/// let view = ClaudeInstalledRegistry::parse("2.1.308", br#"{"version":2,"plugins":{}}"#).unwrap();
/// let _ = serde_json::to_vec(&view);
/// ```
pub struct ClaudeInstalledRegistry {
    entries: BTreeMap<String, Vec<ClaudeInstalledEntry>>,
}
impl fmt::Debug for ClaudeInstalledRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudeInstalledRegistry")
            .field("selectors", &self.entries.len())
            .finish_non_exhaustive()
    }
}
impl ClaudeInstalledRegistry {
    /// Decode installed_plugins.json schema 2. Empty selector arrays are retained.
    /// File presence, original bytes/mode and native folder version are separate facts.
    pub fn parse(version: &str, bytes: &[u8]) -> Result<Self, ClaudeInventoryError> {
        let value = super::parse_capture(version, bytes)?;
        if value.get("version").and_then(serde_json::Value::as_u64) != Some(2) {
            return Err(ClaudeInventoryError::UnsupportedRegistryVersion);
        }
        #[derive(Deserialize)]
        struct Installed {
            plugins: BTreeMap<String, Vec<ClaudeInstalledEntry>>,
        }
        let records: Installed =
            serde_json::from_value(value).map_err(|_| ClaudeInventoryError::Malformed)?;
        for (selector, entries) in &records.plugins {
            if super::invalid_text(selector) {
                return Err(ClaudeInventoryError::Malformed);
            }
            for entry in entries {
                if super::invalid_text(&entry.scope)
                    || super::invalid_text(&entry.version)
                    || super::invalid_path(&entry.install_path)
                    || entry
                        .project_path
                        .as_deref()
                        .is_some_and(super::invalid_path)
                    || (entry.scope == "project" && entry.project_path.is_none())
                {
                    return Err(ClaudeInventoryError::Malformed);
                }
            }
        }
        Ok(Self {
            entries: records.plugins,
        })
    }
    pub fn entries(&self) -> &BTreeMap<String, Vec<ClaudeInstalledEntry>> {
        &self.entries
    }
    /// Exact selector and scope lookup; paths remain unqualified native claims.
    /// None does not establish native absence or permit a mutation.
    pub fn find(
        &self,
        selector: &str,
        requested: ClaudeLookupScope<'_>,
    ) -> Result<Option<&ClaudeInstalledEntry>, ClaudeInventoryError> {
        if let ClaudeLookupScope::Project(path) = requested {
            if !super::absolute_project(path) {
                return Err(ClaudeInventoryError::AmbiguousScope);
            }
        }
        let mut found = None;
        for entry in self.entries.get(selector).into_iter().flatten() {
            let matches = match entry.scope.as_str() {
                "user" => matches!(requested, ClaudeLookupScope::User),
                "project" => {
                    let path = entry
                        .project_path
                        .as_deref()
                        .ok_or(ClaudeInventoryError::AmbiguousScope)?;
                    if !super::absolute_project(path) {
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

fn validate_marketplaces(entries: &[ClaudeMarketplaceEntry]) -> Result<(), ClaudeInventoryError> {
    for entry in entries {
        if super::invalid_text(&entry.name)
            || super::invalid_text(&entry.source)
            || super::invalid_path(&entry.install_location)
            || entry.path.as_deref().is_some_and(super::invalid_path)
            || (entry.source == "directory" && entry.path.is_none())
        {
            return Err(ClaudeInventoryError::Malformed);
        }
    }
    Ok(())
}
fn find_directory<'a>(
    entries: &'a [ClaudeMarketplaceEntry],
    name: &str,
    directory: &Path,
) -> Result<Option<&'a ClaudeMarketplaceEntry>, ClaudeInventoryError> {
    if super::invalid_text(name)
        || super::invalid_path(directory)
        || !super::absolute_project(directory)
    {
        return Err(ClaudeInventoryError::AmbiguousSource);
    }
    let mut rows = entries.iter().filter(|entry| entry.name == name);
    let Some(entry) = rows.next() else {
        return Ok(None);
    };
    if rows.next().is_some() {
        return Err(ClaudeInventoryError::AmbiguousEntry);
    }
    if entry.source != "directory"
        || entry.path.as_deref() != Some(directory)
        || entry.install_location != directory
    {
        return Err(ClaudeInventoryError::AmbiguousSource);
    }
    Ok(Some(entry))
}
