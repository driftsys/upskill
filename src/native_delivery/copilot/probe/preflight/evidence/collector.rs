//! Bounded diagnostic execution. No production admission or native effect receipts.
use super::{CopilotEvidenceError as Error, CopilotProbeEvidence, files, publication};
use crate::native_delivery::process::{NativeCommandSpec, NativeExecutable, run_native_process};
use crate::ownership::{InstallationRoot, OwnershipScope};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

mod environment;
mod metadata;
mod observe;

/// Diagnostic completion only, never native activation, binding or qualification.
#[derive(Debug, Clone, Serialize)]
pub struct CopilotCollectionSummary {
    complete: bool,
    client_commands: usize,
    setup_commands: usize,
    model_requests: u8,
    qualified: bool,
    stop: Option<&'static str>,
}
impl CopilotCollectionSummary {
    fn new() -> Self {
        Self {
            complete: false,
            client_commands: 0,
            setup_commands: 0,
            model_requests: 0,
            qualified: false,
            stop: None,
        }
    }
}
impl CopilotProbeEvidence<'_> {
    /// Execute the reviewed zero-model diagnostic envelope once. One bounded
    /// offline Git setup precedes the eleven fixed client commands. Calling
    /// this explicitly performs native effects; real runs require their own
    /// approval. Unknown results stop without a further command or cleanup.
    pub fn collect(
        &mut self,
        cancellation: &AtomicBool,
    ) -> Result<CopilotCollectionSummary, Error> {
        if self.collection_started || self.stopped {
            return Err(Error::Stopped);
        }
        self.collection_started = true;
        let result = self.collect_inner(cancellation);
        if result.as_ref().map_or(true, |summary| !summary.complete) {
            self.stopped = true;
        }
        result
    }
    fn record(&mut self, name: &str, value: &Value) -> Result<String, Error> {
        self.verify_store()?;
        if self.captures.contains_key(name) {
            return Err(Error::PublicationUncertain);
        }
        let bytes = serde_json::to_vec_pretty(value).map_err(|_| Error::PublicationUncertain)?;
        let capture = publication::write_new(&self.directory, name, &bytes)?;
        let digest = files::digest(&bytes);
        self.captures.insert(name.into(), capture);
        self.verify_store()?;
        Ok(digest)
    }
    fn collect_inner(
        &mut self,
        cancellation: &AtomicBool,
    ) -> Result<CopilotCollectionSummary, Error> {
        self.verify_unchanged()?;
        let plan_bytes = &self
            .preflight
            .fixtures
            .get("plan.json")
            .ok_or(Error::Preconditions)?
            .bytes;
        let plan: Value = serde_json::from_slice(plan_bytes).map_err(|_| Error::Preconditions)?;
        let environment = environment::values(self)?;
        let cwd = InstallationRoot {
            scope: OwnershipScope::Project,
            path: self.root.path.join("project"),
        };
        let mut summary = CopilotCollectionSummary::new();
        let mut previous = files::digest(
            &self
                .captures
                .get("header.json")
                .ok_or(Error::Preconditions)?
                .bytes,
        );
        for sequence in 0usize..12 {
            if cancellation.load(Ordering::SeqCst) {
                self.record(
                    "stopped.json",
                    &json!({"qualified":false,"reason":"cancelled","previous_sha256":previous}),
                )?;
                summary.stop = Some("cancelled");
                return Ok(summary);
            }
            self.verify_unchanged()?;
            let index = sequence.checked_sub(1);
            let (executable, arguments) = if let Some(index) = index {
                let executable = self
                    .preflight
                    .assets
                    .first()
                    .ok_or(Error::Preconditions)?
                    .executable()
                    .map_err(|_| Error::Changed)?;
                let arguments = plan["commands"][index]
                    .as_array()
                    .ok_or(Error::Preconditions)?
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .map(std::ffi::OsString::from)
                            .ok_or(Error::Preconditions)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                (executable, arguments)
            } else {
                let executable = NativeExecutable::capture(Path::new("/usr/bin/git"))
                    .map_err(|_| Error::Preconditions)?;
                let arguments = [
                    "-c",
                    "init.defaultBranch=main",
                    "-c",
                    "core.hooksPath=/dev/null",
                    "-c",
                    "init.templateDir=",
                    "init",
                    "--quiet",
                    "--template=",
                    ".",
                ]
                .map(std::ffi::OsString::from)
                .to_vec();
                (executable, arguments)
            };
            let argv = arguments
                .iter()
                .map(|value| value.to_str().ok_or(Error::Preconditions))
                .collect::<Result<Vec<_>, _>>()?;
            let pending = json!({"schema":1,"sequence":sequence,"client_index":index,"qualified":false,
                "kind":if index.is_some(){"client"}else{"git_setup"},"arguments":argv,
                "executable":executable.code_identity(),"previous_sha256":previous,
                "timeout_seconds":20,"output_limit_bytes":1048576,"model_requests":0,"maximum_authorized_spending":0});
            let pending_name = format!("step-{sequence:02}.pending.json");
            let pending_sha = self.record(&pending_name, &pending)?;
            if index.is_none() && environment::prepare(self).is_err() {
                self.record(&format!("step-{sequence:02}.result.json"),&json!({"schema":1,"sequence":sequence,
                    "pending_sha256":pending_sha,"accepted":false,"qualified":false,"reason":"environment_preparation"}))?;
                summary.stop = Some("environment_preparation");
                return Ok(summary);
            }
            // Revalidate after durable Pending and environment preparation.
            if self.preflight.verify_unchanged().is_err() {
                self.record(&format!("step-{sequence:02}.result.json"),&json!({"schema":1,"sequence":sequence,
                    "pending_sha256":pending_sha,"accepted":false,"qualified":false,"reason":"input_changed"}))?;
                summary.stop = Some("input_changed");
                return Ok(summary);
            }
            self.verify_store()?;
            let spec = NativeCommandSpec {
                executable,
                arguments,
                environment: environment.clone(),
                timeout: Duration::from_secs(20),
                output_limit: 1048576,
            };
            let (scope, guard) = self
                .locks
                .scope_and_guard(&cwd)
                .map_err(|_| Error::Changed)?;
            if index.is_some() {
                summary.client_commands += 1;
            } else {
                summary.setup_commands += 1;
            }
            let output = run_native_process(scope, guard, &spec, cancellation);
            let inputs_unchanged = self.preflight.verify_unchanged().is_ok();
            let (facts, observation) = match output {
                Ok(output) => {
                    let facts = observe::facts(&output);
                    let observation = if inputs_unchanged {
                        observe::inspect(index, &output, &plan)
                    } else {
                        Err("input_changed")
                    };
                    (facts, observation)
                }
                Err(_) => (
                    json!({"invocation_returned_output":false}),
                    Err("invocation"),
                ),
            };
            let (observation, native_files) = match observation {
                Ok(observation) => match metadata::sample(self) {
                    Ok(files) => (Ok(observation), Some(files)),
                    Err(reason) => (Err(reason), None),
                },
                Err(reason) => (Err(reason), None),
            };
            let accepted = observation.is_ok();
            let reason = observation.as_ref().err().copied();
            let result = json!({"schema":1,"sequence":sequence,"client_index":index,
                "pending_sha256":pending_sha,"accepted":accepted,"qualified":false,"reason":reason,
                "inputs_unchanged":inputs_unchanged,"transport":facts,"observation":observation.ok(),
                "model_requests":0,"native_files":native_files});
            previous = self.record(&format!("step-{sequence:02}.result.json"), &result)?;
            if !accepted {
                summary.stop = reason;
                return Ok(summary);
            }
        }
        summary.complete = true;
        Ok(summary)
    }
}
