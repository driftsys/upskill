//! Maintainer diagnostic only; no admitted production profile or owned-skills CLI.
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use upskill::native_delivery::copilot::probe::{CopilotProbePreflight, CopilotProbeRequest};
fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1).peekable();
    let execute = arguments.peek().is_some_and(|value| value == "--execute");
    if execute || arguments.peek().is_some_and(|value| value == "--check") {
        arguments.next();
    }
    let root = PathBuf::from(arguments.next().context(
        "usage: copilot_contract_probe [--check|--execute] ROOT PLAN_SHA256 PREPARATION_SHA256",
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
    ensure!(arguments.next().is_none(), "unexpected probe arguments");
    let request = CopilotProbeRequest {
        packet_root: root,
        plan_sha256: plan,
        preparation_sha256: preparation,
    };
    let preflight = CopilotProbePreflight::capture(&request)?;
    if execute {
        let cancellation = Arc::new(AtomicBool::new(false));
        let signal = cancellation.clone();
        ctrlc::set_handler(move || signal.store(true, std::sync::atomic::Ordering::SeqCst))?;
        let mut evidence = preflight.begin_evidence()?;
        let summary = evidence.collect(&cancellation)?;
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!(
            "{}",
            serde_json::json!({"fixture_files":preflight.fixture_file_count(),"executed":false,"qualified":false})
        );
    }
    Ok(())
}
