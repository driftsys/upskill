//! Read-only capture of the concrete disposable Copilot contract packet.
use crate::filesystem::QualifiedScope;
use crate::ownership::{InstallationRoot, OwnershipScope};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

mod contract;
mod files;

/// Digests must come from separately reviewed preparation, not discovery.
#[derive(Clone)]
pub struct CopilotProbeRequest {
    pub packet_root: PathBuf,
    pub plan_sha256: String,
    pub preparation_sha256: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CopilotPreflightError {
    #[error("Copilot probe input does not match the reviewed contract")]
    InvalidInput,
    #[error("Copilot probe filesystem preconditions are unsafe or unavailable")]
    UnsafeFilesystem,
    #[error("Copilot probe captured inputs or bindings changed")]
    Changed,
}
/// Private known non-secret fixture capture; deliberately not serializable.
/// Capture and revalidation perform no commands or writes. The explicit
/// `begin_evidence` method publishes initial diagnostic files separately;
/// native-generated configuration is never snapshotted.
/// Mode/binding checks do not qualify ACLs, cache roots or native isolation.
pub struct CopilotProbePreflight {
    request: CopilotProbeRequest,
    scopes: Vec<QualifiedScope>,
    directories: BTreeMap<String, files::Directory>,
    fixtures: BTreeMap<String, files::Capture>,
    preparation: files::Capture,
    assets: Vec<files::Asset>,
}
impl fmt::Debug for CopilotProbePreflight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CopilotProbePreflight")
            .field("fixture_files", &self.fixtures.len())
            .finish_non_exhaustive()
    }
}
fn scope(path: &Path, kind: OwnershipScope) -> Result<QualifiedScope, CopilotPreflightError> {
    QualifiedScope::open(&InstallationRoot {
        scope: kind,
        path: path.into(),
    })
    .map_err(|_| CopilotPreflightError::UnsafeFilesystem)
}
fn allowed<'a>(paths: impl Iterator<Item = &'a str>) -> BTreeMap<String, bool> {
    let mut allowed = BTreeMap::from([("outside".into(), true)]);
    for name in paths {
        allowed.insert(name.to_owned(), false);
        let mut prefix = name;
        while let Some((parent, _)) = prefix.rsplit_once('/') {
            allowed.insert(parent.into(), true);
            prefix = parent;
        }
    }
    allowed
}
impl CopilotProbePreflight {
    /// Validate the fixed fixture and retain only digest-matched input.
    /// Unknown manifest paths are rejected before opening those paths. Only
    /// the nine approved files and preparation receipt are captured in memory;
    /// native code assets are streamed for identity checks without snapshots.
    pub fn capture(request: &CopilotProbeRequest) -> Result<Self, CopilotPreflightError> {
        use CopilotPreflightError as Error;
        let root = &request.packet_root;
        if !contract::absolute(root)
            || !contract::hash(&request.plan_sha256)
            || !contract::hash(&request.preparation_sha256)
            || root.canonicalize().ok().as_ref() != Some(root)
        {
            return Err(Error::InvalidInput);
        }
        let parent = root.parent().ok_or(Error::InvalidInput)?;
        let scopes = vec![
            scope(root, OwnershipScope::Project)?,
            scope(parent, OwnershipScope::Project)?,
            scope(&root.join("home"), OwnershipScope::User)?,
            scope(&root.join("project"), OwnershipScope::Project)?,
        ];
        let root_file = files::directory(root)?;
        let parent_file = files::directory(parent)?;
        files::Directory::of(&parent_file)?;
        let preparation = files::Capture::read(
            &parent_file,
            "preparation.json",
            &request.preparation_sha256,
            65536,
        )?;
        let plan = files::Capture::read(&root_file, "plan.json", &request.plan_sha256, 16384)?;
        let contract = contract::Contract::parse(request, &plan.bytes, &preparation.bytes)?;
        let allowed = allowed(contract.files.keys().map(String::as_str));
        let directories = files::tree(&root_file, &allowed)?;
        let mut fixtures = BTreeMap::new();
        for (path, pin) in &contract.files {
            let capture = files::Capture::read(&root_file, path, &pin.sha256, pin.bytes)?;
            if capture.bytes.len() as u64 != pin.bytes {
                return Err(Error::InvalidInput);
            }
            fixtures.insert(path.clone(), capture);
        }
        let assets = contract
            .code
            .into_iter()
            .map(files::Asset::capture)
            .collect::<Result<Vec<_>, _>>()?;
        let result = Self {
            request: request.clone(),
            scopes,
            directories,
            fixtures,
            preparation,
            assets,
        };
        result.verify_unchanged()?;
        if files::tree(&root_file, &allowed)? != result.directories {
            return Err(Error::Changed);
        }
        Ok(result)
    }
    pub fn fixture_file_count(&self) -> usize {
        self.fixtures.len()
    }
    /// Revalidate captured inputs without refreshing or adopting them. New
    /// native bookkeeping is not read or hashed. This samples known bindings;
    /// it does not freeze the filesystem or qualify post-crash quiescence.
    pub fn verify_unchanged(&self) -> Result<(), CopilotPreflightError> {
        use CopilotPreflightError as Error;
        for scope in &self.scopes {
            scope.verify_binding().map_err(|_| Error::Changed)?;
        }
        let root = files::directory(&self.request.packet_root)?;
        files::verify_directories(&root, &self.directories)?;
        let parent_path = self
            .request
            .packet_root
            .parent()
            .ok_or(Error::InvalidInput)?;
        let parent = files::directory(parent_path)?;
        let preparation = files::Capture::read(
            &parent,
            "preparation.json",
            &self.request.preparation_sha256,
            65536,
        )?;
        if preparation != self.preparation {
            return Err(Error::Changed);
        }
        for (path, capture) in &self.fixtures {
            let current = files::Capture::read(
                &root,
                path,
                &files::digest(&capture.bytes),
                capture.bytes.len() as u64,
            )?;
            if &current != capture {
                return Err(Error::Changed);
            }
        }
        for asset in &self.assets {
            asset.verify()?;
        }
        Ok(())
    }
}

mod evidence;
pub use evidence::{CopilotCollectionSummary, CopilotEvidenceError, CopilotProbeEvidence};
