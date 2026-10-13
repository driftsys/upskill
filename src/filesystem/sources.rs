//! Immutable source evidence borrowing one qualified scope.
use super::QualifiedScope;
use crate::source_manifest::SourceManifest;
use anyhow::Result;
use std::fmt;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod backend;

/// Opaque, non-Clone complete source tree evidence.
///
/// Captures every regular-file path and retains private directory membership,
/// native identities and metadata. Verification samples the complete tree and
/// never refreshes captured values. It provides no atomic snapshot, authenticity,
/// permission authority or exclusion against external writers. Capture and
/// verification perform no application writes and acquire no writer lock.
///
/// External construction is unavailable:
/// ```compile_fail
/// use upskill::filesystem::SourceSnapshot;
/// let _ = SourceSnapshot {};
/// ```
/// Evidence cannot be cloned:
/// ```compile_fail
/// use upskill::filesystem::SourceSnapshot;
/// fn copy(snapshot: SourceSnapshot<'_>) { let _ = snapshot.clone(); }
/// ```
/// Captured values cannot be mutated through the snapshot:
/// ```compile_fail
/// use upskill::filesystem::SourceSnapshot;
/// fn clear(snapshot: SourceSnapshot<'_>) { snapshot.manifest().files.clear(); }
/// ```
pub struct SourceSnapshot<'scope> {
    scope: &'scope QualifiedScope,
    manifest: SourceManifest,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    evidence: backend::Evidence,
}
impl fmt::Debug for SourceSnapshot<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceSnapshot").finish_non_exhaustive()
    }
}
impl QualifiedScope {
    /// Observe every source descendant without writes or writer locks.
    pub fn observe_source_tree(&self) -> Result<SourceSnapshot<'_>> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.observe_source_tree_with(|| Ok(()))
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        anyhow::bail!("qualified source observation is unsupported on this operating system")
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn observe_source_tree_with(
        &self,
        before_verify: impl FnOnce() -> Result<()>,
    ) -> Result<SourceSnapshot<'_>> {
        let (evidence, manifest) = backend::Evidence::capture(self)?;
        let snapshot = SourceSnapshot {
            scope: self,
            evidence,
            manifest,
        };
        before_verify()?;
        snapshot.verify_unchanged()?;
        Ok(snapshot)
    }
}
impl SourceSnapshot<'_> {
    /// Borrow captured values without exposing retained evidence.
    pub fn manifest(&self) -> &SourceManifest {
        &self.manifest
    }
    /// Verify without refreshing original evidence.
    pub fn verify_unchanged(&self) -> Result<()> {
        self.scope.verify_binding()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.evidence.verify(self.scope)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        anyhow::bail!("qualified source observation is unsupported on this operating system")
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use super::*;
    use crate::ownership::{InstallationRoot, OwnershipScope};
    #[test]
    fn mutation_after_capture_before_public_return_rejects() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("data"), b"original").unwrap();
        let scope = QualifiedScope::open(&InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().to_owned(),
        })
        .unwrap();
        let result = scope.observe_source_tree_with(|| {
            std::fs::create_dir(tmp.path().join("empty"))?;
            Ok(())
        });
        assert!(
            result.is_err(),
            "public capture omitted pre-return verification"
        );
        assert_eq!(std::fs::read(tmp.path().join("data")).unwrap(), b"original");
        drop(scope.try_lock_writer().unwrap());
    }
}
