//! Exact destination observations borrowing one qualified physical scope.
use super::QualifiedScope;
use crate::files::plan::{FileLocation, ObservedFile};
use anyhow::Result;
use std::fmt;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod backend;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod candidates;

/// Opaque, non-Clone evidence for exact values beneath a borrowed scope.
///
/// Observations alone convey no execution or permission authority. Capture and
/// verification perform no application writes and do not acquire a writer lock.
/// Checks sample the namespace; they do not exclude external writers.
///
/// External callers cannot construct evidence:
/// ```compile_fail
/// use upskill::filesystem::DestinationSnapshot;
/// let _snapshot = DestinationSnapshot {};
/// ```
/// Evidence cannot be cloned independently of its handles:
/// ```compile_fail
/// use upskill::filesystem::DestinationSnapshot;
/// fn clone_evidence(snapshot: DestinationSnapshot<'_>) { let _ = snapshot.clone(); }
/// ```
/// Captured observations remain private and cannot be cleared by callers:
/// ```compile_fail,E0616
/// use upskill::filesystem::DestinationSnapshot;
/// fn clear_observations(mut snapshot: DestinationSnapshot<'_>) {
///     snapshot.observations.clear();
/// }
/// ```
pub struct DestinationSnapshot<'scope> {
    scope: &'scope QualifiedScope,
    observations: Vec<ObservedFile>,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    evidence: backend::Evidence,
}
impl fmt::Debug for DestinationSnapshot<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DestinationSnapshot")
            .finish_non_exhaustive()
    }
}
impl QualifiedScope {
    /// Capture exact file bytes/modes or first-missing-component evidence.
    /// Every root must remain bound to this physical scope. Unsafe entries,
    /// potentially overlapping paths and unqualified mounts are errors.
    pub fn observe_destinations<'scope>(
        &'scope self,
        locations: &[FileLocation],
    ) -> Result<DestinationSnapshot<'scope>> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.observe_destinations_with(locations, || Ok(()))
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = locations;
            anyhow::bail!(
                "qualified destination observation is unsupported on this operating system"
            )
        }
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn observe_destinations_with<'scope>(
        &'scope self,
        locations: &[FileLocation],
        before_verify: impl FnOnce() -> Result<()>,
    ) -> Result<DestinationSnapshot<'scope>> {
        let locations = candidates::prepare(locations)?;
        let (evidence, observations) = backend::Evidence::capture(self, &locations)?;
        let snapshot = DestinationSnapshot {
            scope: self,
            observations,
            evidence,
        };
        before_verify()?;
        snapshot.verify_unchanged()?;
        Ok(snapshot)
    }
}
impl DestinationSnapshot<'_> {
    /// Borrow captured plain values, without exposing private filesystem evidence.
    pub fn observations(&self) -> &[ObservedFile] {
        &self.observations
    }
    /// Revalidate all retained bindings and exact values without refreshing them.
    pub fn verify_unchanged(&self) -> Result<()> {
        self.scope.verify_binding()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.evidence.verify(self.scope, &self.observations)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        anyhow::bail!("qualified destination observation is unsupported on this operating system")
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use super::*;
    use crate::ownership::{InstallationRoot, OwnershipScope};

    #[test]
    fn namespace_replacement_before_return_rejects_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("parent")).unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().to_owned(),
        };
        let scope = QualifiedScope::open(&root).unwrap();
        let locations = [FileLocation {
            root,
            path: "parent/missing".into(),
        }];
        let result = scope.observe_destinations_with(&locations, || {
            // Acquisition finished. Change only a directory identity, while the
            // exact missing suffix stays absent, before the production postcheck.
            std::fs::rename(tmp.path().join("parent"), tmp.path().join("moved"))?;
            std::fs::create_dir(tmp.path().join("parent"))?;
            Ok(())
        });
        assert!(
            result.is_err(),
            "capture returned stale evidence before any later verification"
        );
        assert!(tmp.path().join("moved").is_dir());
        assert_eq!(
            std::fs::read_dir(tmp.path().join("parent"))
                .unwrap()
                .count(),
            0
        );
        drop(scope.try_lock_writer().unwrap());
    }
}
