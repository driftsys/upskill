//! Pure combined-state records. Validation establishes no filesystem ownership.
use crate::lockfile::Lockfile;
use crate::ownership::{ArtifactOwner, InstallationRoot, OwnedArtifact};
use crate::target::TargetSurface;
use serde::{Deserialize, Serialize};
mod validate;
pub use validate::validate_state;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombinedState {
    pub schema: u32,
    pub installations: Vec<Installation>,
    pub bundles: Vec<BundleNode>,
    pub references: Vec<Reference>,
    pub artifacts: Vec<OwnedArtifact>,
    pub owner_consumers: Vec<OwnerConsumers>,
    pub legacy: Option<Lockfile>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Installation {
    pub owner: ArtifactOwner,
    pub root: InstallationRoot,
    pub requested: Vec<TargetSurface>,
    pub source_evidence: SourceEvidence,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SourceEvidence {
    Git {
        resolved_revision: String,
        manifest_sha256: String,
    },
    Local {
        manifest_sha256: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleNode {
    pub id: String,
    pub source: String,
    pub name: String,
    pub source_revision: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Reference {
    DirectInstallation {
        installation_id: String,
    },
    DirectBundle {
        bundle_id: String,
    },
    BundleInstallation {
        bundle_id: String,
        installation_id: String,
    },
    BundleDependency {
        bundle_id: String,
        dependency_id: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerConsumers {
    pub root: InstallationRoot,
    pub path: String,
    pub contribution_id: Option<String>,
    pub installation_id: String,
    pub consumers: Vec<TargetSurface>,
}
