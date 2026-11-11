//! Maintainer diagnostic only; no admitted production profile or owned-skills CLI.
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use upskill::native_delivery::copilot::probe::{CopilotProbePreflight, CopilotProbeRequest};
fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1).peekable();
    let execute = arguments.peek().is_some_and(|value| value == "--execute");
    let marketplace_only = arguments
        .peek()
        .is_some_and(|value| value == "--execute-marketplace-check");
    if execute || marketplace_only || arguments.peek().is_some_and(|value| value == "--check") {
        arguments.next();
    }
    let root = PathBuf::from(arguments.next().context(
        "usage: copilot_contract_probe [--check] ROOT PLAN_SHA256 PREPARATION_SHA256\n       copilot_contract_probe --execute|--execute-marketplace-check ROOT PLAN_SHA256 PREPARATION_SHA256 DRIVER_SHA256",
    )?);
    let plan = arguments
        .next()
        .context("missing reviewed plan digest")?
        .into_string()
        .map_err(|_| anyhow::anyhow!("invalid plan digest"))?;
    let preparation = arguments
        .next()
        .context("missing reviewed preparation digest")?
        .into_string()
        .map_err(|_| anyhow::anyhow!("invalid preparation digest"))?;
    if execute || marketplace_only {
        // The reviewed readiness receipt binds this executable's digest.
        // A rebuilt driver must stop before any packet or native effect.
        let expected = arguments
            .next()
            .context("missing reviewed driver digest")?
            .into_string()
            .map_err(|_| anyhow::anyhow!("invalid driver digest"))?;
        let path = std::env::current_exe().context("locating the probe driver")?;
        let bytes = std::fs::read(&path).context("reading the probe driver")?;
        let actual: String = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        ensure!(
            actual == expected,
            "probe driver digest {actual} does not match the reviewed digest {expected}"
        );
    }
    ensure!(arguments.next().is_none(), "unexpected probe arguments");
    let request = CopilotProbeRequest {
        packet_root: root,
        plan_sha256: plan,
        preparation_sha256: preparation,
    };
    let preflight = CopilotProbePreflight::capture(&request)?;
    if execute || marketplace_only {
        let cancellation = Arc::new(AtomicBool::new(false));
        let signal = cancellation.clone();
        ctrlc::set_handler(move || signal.store(true, std::sync::atomic::Ordering::SeqCst))?;
        let mut evidence = preflight.begin_evidence()?;
        let summary = if marketplace_only {
            evidence.collect_marketplace_sources(&cancellation)?
        } else {
            evidence.collect(&cancellation)?
        };
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!(
            "{}",
            serde_json::json!({"fixture_files":preflight.fixture_file_count(),"executed":false,"qualified":false})
        );
    }
    Ok(())
}
