use super::NativeEvidenceStore;
use crate::files::permissions::{
    DestinationMode, PermissionSource, ResourceExecutionPolicy, select_file_mode,
};
use crate::files::plan::{FileLocation, FileValue, PlannedFile};
use crate::filesystem::DestinationSnapshot;
use crate::native_delivery::roots::NativeRootLocks;
use crate::native_delivery::state::{NativeDeliveryState, NativeStateContext};
use crate::ownership::InstallationRoot;
use anyhow::{Context, Result, ensure};

/// Integrated state capture and preparation under the complete root-lock set.
/// The authoritative graph remains at `.upskill-lock.json`. This API never
/// applies that file, migrates a prior schema or establishes native completion.
pub struct NativeStateStore<'locks, 'scope> {
    roots: &'locks NativeRootLocks<'scope>,
    evidence: NativeEvidenceStore<'locks>,
    context: NativeStateContext,
}
impl<'locks, 'scope: 'locks> NativeStateStore<'locks, 'scope> {
    /// Initialize only private evidence storage, after checking both held roots.
    pub fn initialize(
        roots: &'locks NativeRootLocks<'scope>,
        primary: &InstallationRoot,
        private: &InstallationRoot,
    ) -> Result<Self> {
        Self::open_inner(roots, primary, private, true)?.context("private evidence store missing")
    }

    /// Open existing private storage without creating it. `None` describes the
    /// store's absence, not the absence or validity of an installed graph.
    pub fn open(
        roots: &'locks NativeRootLocks<'scope>,
        primary: &InstallationRoot,
        private: &InstallationRoot,
    ) -> Result<Option<Self>> {
        Self::open_inner(roots, primary, private, false)
    }

    fn open_inner(
        roots: &'locks NativeRootLocks<'scope>,
        primary: &InstallationRoot,
        private: &InstallationRoot,
        create: bool,
    ) -> Result<Option<Self>> {
        roots.scope_and_guard(primary)?;
        let (scope, guard) = roots.scope_and_guard(private)?;
        let evidence = if create {
            Some(NativeEvidenceStore::initialize(private, scope, guard)?)
        } else {
            NativeEvidenceStore::open(private, scope, guard)?
        };
        let Some(evidence) = evidence else {
            return Ok(None);
        };
        let mut additional_roots: Vec<_> = roots
            .bindings()
            .iter()
            .filter(|binding| binding.root != *primary && binding.root != *private)
            .map(|binding| binding.root.clone())
            .collect();
        // State context uses stable logical ordering; physical lock order can
        // differ after a reboot and is not the persistent context order.
        additional_roots.sort_by(|a, b| (a.scope, &a.path).cmp(&(b.scope, &b.path)));
        let context = NativeStateContext {
            scope: primary.clone(),
            private_root: private.clone(),
            namespace: evidence.namespace().into(),
            additional_roots,
        };
        roots.verify_bindings()?;
        Ok(Some(Self {
            roots,
            evidence,
            context,
        }))
    }

    pub fn context(&self) -> &NativeStateContext {
        &self.context
    }

    /// Capture the sole authoritative lockfile and validate all referenced
    /// private evidence. Existing schemas are rejected without migration.
    pub fn load(&self) -> Result<Option<NativeDeliveryState>> {
        let snapshot = self.snapshot()?;
        let state = snapshot.observations()[0]
            .value
            .as_ref()
            .map(|value| {
                self.evidence
                    .load_state(std::str::from_utf8(&value.bytes)?, &self.context)
            })
            .transpose()?;
        snapshot.verify_unchanged()?;
        self.roots.verify_bindings()?;
        Ok(state)
    }

    /// Validate prior and supplied state, sync private evidence, then return an
    /// exact file precondition and reference-only JSON. The coordinator must
    /// establish observed native completion before applying this state action.
    pub fn prepare_state_file(&self, state: &NativeDeliveryState) -> Result<PlannedFile> {
        ensure!(
            state.context == self.context,
            "native state context differs from the held roots"
        );
        let snapshot = self.snapshot()?;
        let before = snapshot.observations()[0].value.clone();
        if let Some(value) = &before {
            self.evidence
                .load_state(std::str::from_utf8(&value.bytes)?, &self.context)?;
        }
        let mode = select_file_mode(
            before.as_ref().map_or(DestinationMode::Absent, |value| {
                DestinationMode::Present(value.mode)
            }),
            PermissionSource::Generated,
            ResourceExecutionPolicy::Disabled,
        )?;
        let json = self.evidence.store_state_evidence(state)?;
        snapshot.verify_unchanged()?;
        self.roots.verify_bindings()?;
        Ok(PlannedFile {
            location: self.location(),
            before,
            after: Some(FileValue {
                bytes: json.into_bytes(),
                mode: Some(mode),
            }),
        })
    }

    fn location(&self) -> FileLocation {
        FileLocation {
            root: self.context.scope.clone(),
            path: ".upskill-lock.json".into(),
        }
    }
    fn snapshot(&self) -> Result<DestinationSnapshot<'_>> {
        let (scope, _) = self.roots.scope_and_guard(&self.context.scope)?;
        scope.observe_destinations(&[self.location()])
    }
}
