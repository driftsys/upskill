//! Durable preparation only. No application/native effects or completed state.
use super::roots::NativeRootLocks;
use super::state::NativeStateContext;
use super::store::{NativeEvidenceStore, NativeStateStore};
use crate::ownership::InstallationRoot;
use anyhow::{Context, Result, ensure};
mod codec;
mod model;
mod planning;
use codec::*;
use model::PendingMarker;
pub use model::*;

/// One private intent/journal and pending markers in every held root. Preparation
/// never stages exports, changes configuration or applies native/state effects.
pub struct NativeOperationStore<'locks, 'scope> {
    roots: &'locks NativeRootLocks<'scope>,
    state: NativeStateStore<'locks, 'scope>,
}
impl<'locks, 'scope: 'locks> NativeOperationStore<'locks, 'scope> {
    pub fn initialize(
        roots: &'locks NativeRootLocks<'scope>,
        primary: &InstallationRoot,
        private: &InstallationRoot,
    ) -> Result<Self> {
        Ok(Self {
            roots,
            state: NativeStateStore::initialize(roots, primary, private)?,
        })
    }
    pub fn context(&self) -> &NativeStateContext {
        self.state.context()
    }

    pub fn prepare(&self, capture: &NativeOperationCapture) -> Result<PreparedNativeOperation> {
        // Scan the entire set before creating marker storage in any new root.
        // A secondary marker blocks even when its private journal is not held.
        for binding in self.roots.bindings() {
            if let Some(store) = self.marker_store(&binding.root, false)? {
                if let Some(bytes) = store.read_record("pending.json")? {
                    let marker: PendingMarker = json(&bytes)?;
                    validate_marker(&marker, binding, store.namespace())?;
                    return Err(NativeRecoveryRequired {
                        operation_id: marker.operation_id,
                        private_root: marker.private_root,
                        journal_name: marker.journal_name,
                    }
                    .into());
                }
            }
        }
        let snapshot = self.state.snapshot()?;
        let prior_state = snapshot.observations()[0].value.clone();
        let prior = self.prior(&prior_state)?;
        let (plan, configuration_patches) = self.plan(&prior, capture)?;
        let mut marker_namespaces = Vec::new();
        for binding in self.roots.bindings() {
            let store = self
                .marker_store(&binding.root, true)?
                .context("missing initialized marker storage")?;
            marker_namespaces.push(RootMarkerNamespace {
                root: binding.clone(),
                namespace: store.namespace().into(),
            });
        }
        let observations = self.observe_capture(capture)?;
        let id = self.state.evidence().new_operation_id()?;
        let intent = NativeOperationIntent {
            schema: 1,
            operation_id: id.clone(),
            context: self.context().clone(),
            roots: self.roots.bindings().to_vec(),
            marker_namespaces,
            prior_state,
            capture: capture.clone(),
            plan,
            configuration_patches,
        };
        let bytes = serde_yaml_ng::to_string(&intent)?.into_bytes();
        let decoded: NativeOperationIntent = yaml(&bytes)?;
        ensure!(
            decoded == intent,
            "native intent cannot round-trip losslessly"
        );
        let journal = NativeOperationJournal {
            schema: 1,
            operation_id: id.clone(),
            intent_sha256: intent_digest(&bytes),
            generation: 0,
            status: NativeJournalStatus::Prepared,
        };
        self.state
            .evidence()
            .write_record(&intent_name(&id), None, Some(&bytes))?;
        self.state.evidence().write_record(
            &journal_name(&id),
            None,
            Some(&serde_json::to_vec(&journal)?),
        )?;
        checkpoint("after_intent", 0, &id)?;
        for (index, marker) in intent.marker_namespaces.iter().enumerate() {
            let store = self
                .marker_store(&marker.root.root, false)?
                .context("marker storage disappeared")?;
            ensure!(
                store.namespace() == marker.namespace,
                "marker storage binding changed"
            );
            let pending = pending(&intent, &journal, marker);
            store.write_record("pending.json", None, Some(&serde_json::to_vec(&pending)?))?;
            checkpoint("after_marker", index, &id)?;
        }
        for observation in observations {
            observation.verify_unchanged()?;
        }
        snapshot.verify_unchanged()?;
        self.roots.verify_bindings()?;
        let inspection = self.inspect(&id)?;
        ensure!(
            inspection.markers_complete,
            "native operation preparation requires every root marker"
        );
        Ok(PreparedNativeOperation { id })
    }

    /// Inspect immutable intent and initial journal under exactly the recorded
    /// roots. Missing markers report incomplete preparation. No retry or cleanup
    /// follows, and this does not observe application/native completion.
    pub fn inspect(&self, id: &str) -> Result<NativeOperationInspection> {
        validate_id(id)?;
        self.roots.verify_bindings()?;
        let journal: NativeOperationJournal = json(
            &self
                .state
                .evidence()
                .read_record(&journal_name(id))?
                .context("native journal missing")?,
        )?;
        ensure!(
            journal.schema == 1
                && journal.operation_id == id
                && journal.generation == 0
                && journal.status == NativeJournalStatus::Prepared,
            "unsupported native journal"
        );
        let bytes = self
            .state
            .evidence()
            .read_record(&intent_name(id))?
            .context("native intent missing")?;
        ensure!(
            intent_digest(&bytes) == journal.intent_sha256,
            "native intent digest changed"
        );
        let intent: NativeOperationIntent = yaml(&bytes)?;
        ensure!(
            intent.schema == 1
                && intent.operation_id == id
                && intent.context == *self.context()
                && intent.roots == self.roots.bindings(),
            "native intent root/context binding differs"
        );
        ensure!(
            intent.marker_namespaces.len() == intent.roots.len()
                && intent
                    .marker_namespaces
                    .iter()
                    .zip(&intent.roots)
                    .all(|(marker, root)| marker.root == *root),
            "native intent marker roots differ"
        );
        let prior = self.prior(&intent.prior_state)?;
        let (plan, patches) = self.plan(&prior, &intent.capture)?;
        ensure!(
            plan == intent.plan && patches == intent.configuration_patches,
            "native intent differs from replayed captured plan"
        );
        let mut markers_complete = true;
        for marker in &intent.marker_namespaces {
            let Some(store) = self.marker_store(&marker.root.root, false)? else {
                markers_complete = false;
                continue;
            };
            ensure!(
                store.namespace() == marker.namespace,
                "native marker directory binding differs"
            );
            match store.read_record("pending.json")? {
                None => markers_complete = false,
                Some(bytes) => {
                    let actual: PendingMarker = json(&bytes)?;
                    ensure!(
                        actual == pending(&intent, &journal, marker),
                        "native pending marker differs from intent"
                    );
                }
            }
        }
        self.roots.verify_bindings()?;
        Ok(NativeOperationInspection {
            intent,
            journal,
            markers_complete,
        })
    }

    fn marker_store(
        &self,
        root: &InstallationRoot,
        create: bool,
    ) -> Result<Option<NativeEvidenceStore<'_>>> {
        let (scope, guard) = self.roots.scope_and_guard(root)?;
        NativeEvidenceStore::open_marker(root, scope, guard, create)
    }
}
#[cfg(test)]
type Hook = Box<dyn FnMut(&'static str, usize, &str) -> Result<()>>;
#[cfg(test)]
thread_local! { static TEST_HOOK: std::cell::RefCell<Option<Hook>> = std::cell::RefCell::new(None); }
fn checkpoint(label: &'static str, index: usize, id: &str) -> Result<()> {
    #[cfg(test)]
    {
        TEST_HOOK.with(|hook| match hook.borrow_mut().as_mut() {
            Some(hook) => hook(label, index, id),
            None => Ok(()),
        })
    }
    #[cfg(not(test))]
    {
        let _ = (label, index, id);
        Ok(())
    }
}

#[cfg(test)]
mod process_tests;
