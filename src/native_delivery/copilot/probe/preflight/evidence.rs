//! Private diagnostic publication and collection; no native effect receipts.
use super::{CopilotProbePreflight, allowed, files};
use crate::filesystem::native_effects;
use crate::native_delivery::roots::{NativeRoot, NativeRootLocks, lock_native_roots};
use crate::ownership::{InstallationRoot, OwnershipScope};
use serde_json::json;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;

mod publication;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CopilotEvidenceError {
    #[error("Copilot probe inputs changed before evidence publication")]
    Preconditions,
    #[error("Copilot probe evidence or root exclusion is unavailable")]
    Unavailable,
    #[error("Copilot probe evidence publication is uncertain; retain the packet")]
    PublicationUncertain,
    #[error("Copilot probe evidence or input bindings changed")]
    Changed,
    #[error("Copilot probe evidence handle is stopped; no further work is permitted")]
    Stopped,
}

/// Exclusive diagnostic evidence under a borrowed captured packet.
/// Initial publication invokes no executable. Explicit collection appends command
/// Pending and terminal diagnostic records. Drop releases advisory root locks
/// and retains every published file. Neither operation supplies effect/retention
/// receipts, admission or worker quiescence proof. File sync requests do not
/// qualify a filesystem profile.
pub struct CopilotProbeEvidence<'a> {
    preflight: &'a CopilotProbePreflight,
    locks: NativeRootLocks<'a>,
    root: InstallationRoot,
    directory: File,
    identity: files::Directory,
    captures: BTreeMap<String, files::Capture>,
    stopped: bool,
    collection_started: bool,
}
impl fmt::Debug for CopilotProbeEvidence<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CopilotProbeEvidence")
            .field("files", &self.captures.len())
            .field("stopped", &self.stopped)
            .finish_non_exhaustive()
    }
}
impl CopilotProbePreflight {
    /// Explicitly publish private initial evidence once. Acquire all selected
    /// root guards, recheck known and unused inputs, then exclusively create the
    /// evidence directory. Existing or partial evidence is never adopted,
    /// overwritten or deleted. No native executable is invoked.
    pub fn begin_evidence(&self) -> Result<CopilotProbeEvidence<'_>, CopilotEvidenceError> {
        use CopilotEvidenceError as Error;
        self.verify_unchanged().map_err(|_| Error::Preconditions)?;
        let roots = self
            .scopes
            .iter()
            .enumerate()
            .map(|(index, scope)| {
                let path = scope.binding().map_err(|_| Error::Preconditions)?.selected;
                Ok(NativeRoot {
                    scope,
                    root: InstallationRoot {
                        scope: if index == 2 {
                            OwnershipScope::User
                        } else {
                            OwnershipScope::Project
                        },
                        path,
                    },
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let locks = lock_native_roots(&roots).map_err(|_| Error::Unavailable)?;
        self.verify_unchanged().map_err(|_| Error::Preconditions)?;
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: self.request.packet_root.clone(),
        };
        let (scope, guard) = locks
            .scope_and_guard(&root)
            .map_err(|_| Error::Preconditions)?;
        let base =
            native_effects::command_directory(scope, guard).map_err(|_| Error::Preconditions)?;
        let permitted = allowed(self.fixtures.keys().map(String::as_str));
        if files::tree(&base, &permitted).map_err(|_| Error::Unavailable)? != self.directories {
            return Err(Error::Preconditions);
        }
        locks.verify_bindings().map_err(|_| Error::Preconditions)?;
        let mut rows = Vec::new();
        let mut payload = Vec::new();
        for (index, (source, capture)) in self.fixtures.iter().enumerate() {
            let name = format!("fixture-{index:02}.bin");
            rows.push(
                json!({"source":source,"snapshot":name,"sha256":files::digest(&capture.bytes),
                "bytes":capture.bytes.len(),"mode":0o600}),
            );
            payload.push((name, capture.bytes.clone()));
        }
        let header = json!({"schema":1,"status":"prepared; no native command pending",
            "plan_sha256":self.request.plan_sha256,"preparation_sha256":self.request.preparation_sha256,
            "maximum_native_commands":12,"maximum_client_commands":11,"maximum_setup_commands":1,"model_calls":0,"maximum_authorized_spending":0,
            "qualified":false,"inventory_source_policy":{"filesystem_paths":"packet_only",
                "unrelated_builtin_label":"count_only","owned_builtin_label":"reject",
                "unrelated_default_remote_display":"count_only","marketplace_local_display":"packet_path_checks",
                "owned_marketplace":"exact_catalog_non_default"},
            "roots":locks.bindings(),"fixtures":rows});
        let header = serde_json::to_vec_pretty(&header).map_err(|_| Error::Preconditions)?;
        payload.insert(0, ("header.json".into(), header));
        let (directory, captures) = publication::publish(&base, &payload, |_| Ok(()))?;
        let identity = files::Directory::of(&directory).map_err(|_| Error::PublicationUncertain)?;
        let mut result = CopilotProbeEvidence {
            preflight: self,
            locks,
            root,
            directory,
            identity,
            captures,
            stopped: false,
            collection_started: false,
        };
        result.verify_unchanged()?;
        Ok(result)
    }
}
impl CopilotProbeEvidence<'_> {
    /// Sample known inputs and immutable published evidence. Any failure stops
    /// this handle permanently, including if a caller repairs reversible drift.
    /// New native configuration contents are never read, copied or hashed.
    pub fn verify_unchanged(&mut self) -> Result<(), CopilotEvidenceError> {
        use CopilotEvidenceError as Error;
        if self.stopped {
            return Err(Error::Stopped);
        }
        let result = self.verify_inner();
        if result.is_err() {
            self.stopped = true;
        }
        result
    }
    fn verify_inner(&self) -> Result<(), CopilotEvidenceError> {
        self.preflight
            .verify_unchanged()
            .map_err(|_| CopilotEvidenceError::Changed)?;
        self.verify_store()
    }
    fn verify_store(&self) -> Result<(), CopilotEvidenceError> {
        use CopilotEvidenceError as Error;
        self.locks.verify_bindings().map_err(|_| Error::Changed)?;
        let (scope, guard) = self
            .locks
            .scope_and_guard(&self.root)
            .map_err(|_| Error::Changed)?;
        let base = native_effects::command_directory(scope, guard).map_err(|_| Error::Changed)?;
        let selected = publication::open_area(&base).map_err(|_| Error::Changed)?;
        if files::Directory::of(&selected).map_err(|_| Error::Changed)? != self.identity
            || files::Directory::of(&self.directory).map_err(|_| Error::Changed)? != self.identity
        {
            return Err(Error::Changed);
        }
        let permitted = self
            .captures
            .keys()
            .map(|name| (name.clone(), false))
            .collect();
        files::tree(&selected, &permitted).map_err(|_| Error::Changed)?;
        for (name, capture) in &self.captures {
            let current = files::Capture::read(
                &selected,
                name,
                &files::digest(&capture.bytes),
                capture.bytes.len() as u64,
            )
            .map_err(|_| Error::Changed)?;
            if &current != capture {
                return Err(Error::Changed);
            }
        }
        // Recheck both the held root and selected evidence namespace after reads.
        self.locks.verify_bindings().map_err(|_| Error::Changed)?;
        let bound = publication::open_area(&base).map_err(|_| Error::Changed)?;
        if files::Directory::of(&bound).map_err(|_| Error::Changed)? != self.identity {
            return Err(Error::Changed);
        }
        Ok(())
    }
}

mod collector;
pub use collector::CopilotCollectionSummary;
