use super::NativeEvidenceStore;
use crate::files::permissions::{
    DestinationMode, PermissionSource, ResourceExecutionPolicy, select_file_mode,
};
use crate::files::plan::{FileLocation, FileValue, PlannedFile};
use crate::filesystem::DestinationSnapshot;
use crate::native_delivery::roots::NativeRootLocks;
use crate::native_delivery::state::{NativeDeliveryState, NativeStateContext, declared_context};
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

    /// Capture the sole authoritative lockfile and validate referenced evidence.
    /// Declared additional roots must be held; extra held roots permit context
    /// transitions. Prior schemas reject without migration.
    pub fn load(&self) -> Result<Option<NativeDeliveryState>> {
        let snapshot = self.snapshot()?;
        let state = snapshot.observations()[0]
            .value
            .as_ref()
            .map(|value| self.parse_captured_state(&value.bytes))
            .transpose()?;
        snapshot.verify_unchanged()?;
        self.roots.verify_bindings()?;
        Ok(state)
    }

    /// Validate prior and supplied state, sync private evidence, then return an
    /// exact file precondition and reference-only JSON. The coordinator must
    /// establish observed native completion before applying this state action.
    pub fn prepare_state_file(&self, state: &NativeDeliveryState) -> Result<PlannedFile> {
        self.require_context(&state.context)?;
        let snapshot = self.snapshot()?;
        let before = snapshot.observations()[0].value.clone();
        if let Some(value) = &before {
            self.parse_captured_state(&value.bytes)?;
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

    pub(crate) fn parse_captured_state(&self, bytes: &[u8]) -> Result<NativeDeliveryState> {
        let json = std::str::from_utf8(bytes)?;
        let context = declared_context(json)?;
        self.require_context(&context)?;
        self.evidence.load_state(json, &context)
    }
    fn require_context(&self, context: &NativeStateContext) -> Result<()> {
        ensure!(
            context.scope == self.context.scope
                && context.private_root == self.context.private_root
                && context.namespace == self.context.namespace,
            "native state principal/private context differs from held roots"
        );
        for root in &context.additional_roots {
            self.roots.scope_and_guard(root)?;
        }
        self.roots.verify_bindings()
    }

    pub(crate) fn evidence(&self) -> &NativeEvidenceStore<'locks> {
        &self.evidence
    }

    fn location(&self) -> FileLocation {
        FileLocation {
            root: self.context.scope.clone(),
            path: ".upskill-lock.json".into(),
        }
    }
    pub(crate) fn snapshot(&self) -> Result<DestinationSnapshot<'_>> {
        let (scope, _) = self.roots.scope_and_guard(&self.context.scope)?;
        scope.observe_destinations(&[self.location()])
    }
}
