# File Application Permission Policy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox syntax for tracking. The user's standing delegation preserves this execution method.

**Goal:** Add an inactive pure file permission-selection API without changing compiler, planner or CLI behavior.

**Architecture:** Separate exact supplied destination/source mode evidence from the explicit resource execution choice. The pure function validates evidence, preserves supported existing ordinary modes and selects private modes for absent output. Future qualified writers consume its result only after separate ownership/metadata/effect qualification.

**Tech Stack:** Existing Rust edition 2024, anyhow, integration test harness; no dependency additions.

**Spec:** ../specs/2026-10-13-file-permissions-design.md. Durable records: docs/design/file-permission-policy.md and AD-0031.

## Global Constraints

- Rust edition 2024 and MSRV 1.85.
- No new dependencies, async runtime, subprocess, environment reads, filesystem I/O or stdout/stderr outside main.rs.
- No changes to source capture, manifest framing, compiler output, existing planner validation or lockfile schemas.
- No production CLI behavior, state persistence, migration, file effects, journal or recovery activation.
- Public supplied values convey no physical qualification or execution authority.
- Preserve the inherited 54 WIP/raw archive hashes, the broader application draft and all scratch/control evidence. No cleanup.

## Review Focus

- Unknown present mode must not become an absent-file private default: unknown_present_rejects_while_absent_selects_private.
- Group/other execute must not grant owner execution: resource_execution_requires_owner_bit_and_explicit_policy.
- Invalid unused captured evidence must not bypass validation: invalid_evidence_rejects_even_when_existing_mode_would_win.
- Existing special bits must not be silently cleared or copied: existing_special_modes_reject_without_a_result.
- Resource execution must not affect generated output or existing modes: generated_output_is_private and existing_ordinary_modes_are_exact.

## Files and ownership

Create src/files/permissions.rs for the pure public policy, tests/file_permissions.rs for public acceptance tests. Export the module from src/files/mod.rs. Update docs/specification/specification.md with a new subsection 4.10 after 4.9 and before section5. Update the durable permission design/AD-0031 from selected to implemented only after passing evidence. Add their links to docs/design/README.md, docs/adr/README.md and docs/book/SUMMARY.md where the existing design/ADR sections belong. Keep broader docs/design/qualified-file-application.md untracked and unchanged; it is an owned draft for the subsequent executor work and must not be staged in this PR. No other product files are assigned.

### Task 1: Select explicit file modes from supplied evidence

**Files:** Create src/files/permissions.rs; modify src/files/mod.rs; create tests/file_permissions.rs; update the current documentation/index paths above.

**Interfaces:**

- Consumes: DestinationMode::Absent or Present(Option<u32>); PermissionSource::Generated or Resource { captured_mode: Option<u32> }; ResourceExecutionPolicy::Disabled or OwnerExecutable.
- Produces: files::permissions::select_file_mode(destination, source, resource_execution) -> anyhow::Result<u32>; all three enums derive Debug, Clone, Copy, PartialEq, Eq. No serialized representation or effect capability.

- [x] **Step 1: Write public acceptance tests before implementation.**

Use this concrete starting suite in tests/file_permissions.rs, extending its diagnostic assertions where the spec distinguishes contextual errors:

```rust
use upskill::files::permissions::{
    DestinationMode::{Absent, Present}, PermissionSource::{Generated, Resource},
    ResourceExecutionPolicy::{Disabled, OwnerExecutable}, select_file_mode,
};

#[test]
fn generated_output_is_private() {
    for policy in [Disabled, OwnerExecutable] {
        assert_eq!(select_file_mode(Absent, Generated, policy).unwrap(), 0o600);
    }
}

#[test]
fn existing_ordinary_modes_are_exact() {
    for mode in [0o000, 0o400, 0o640, 0o755, 0o777] {
        for source in [Generated, Resource { captured_mode: Some(0o7777) }] {
            for policy in [Disabled, OwnerExecutable] {
                assert_eq!(select_file_mode(Present(Some(mode)), source, policy).unwrap(), mode,
                           "existing ordinary mode was not preserved exactly");
            }
        }
    }
}

#[test]
fn unknown_present_rejects_while_absent_selects_private() {
    assert!(select_file_mode(Present(None), Generated, Disabled).is_err(),
            "unknown existing mode was treated as absence");
    assert_eq!(select_file_mode(Absent, Generated, Disabled).unwrap(), 0o600);
}

#[test]
fn existing_special_modes_reject_without_a_result() {
    for mode in [0o1000, 0o2000, 0o4000, 0o2640] {
        assert!(select_file_mode(Present(Some(mode)), Generated, Disabled).is_err(),
                "unsupported existing special bits produced an application mode");
    }
}

#[test]
fn disabled_resource_execution_never_copies_source_mode() {
    for captured_mode in [None, Some(0o000), Some(0o755), Some(0o7777)] {
        assert_eq!(select_file_mode(Absent, Resource { captured_mode }, Disabled).unwrap(), 0o600,
                   "disabled resource execution copied captured permissions");
    }
}

#[test]
fn resource_execution_requires_owner_bit_and_explicit_policy() {
    for (captured_mode, expected) in [(0o100, 0o700), (0o755, 0o700), (0o4777, 0o700),
                                     (0o011, 0o600), (0o644, 0o600)] {
        assert_eq!(select_file_mode(Absent, Resource { captured_mode: Some(captured_mode) }, OwnerExecutable).unwrap(), expected,
                   "resource execution did not follow only the captured owner-execute bit");
    }
}

#[test]
fn unknown_resource_mode_cannot_grant_new_execution() {
    assert!(select_file_mode(Absent, Resource { captured_mode: None }, OwnerExecutable).is_err(),
            "unknown resource mode granted new executable output");
    assert_eq!(select_file_mode(Present(Some(0o640)), Resource { captured_mode: None }, OwnerExecutable).unwrap(), 0o640);
}

#[test]
fn invalid_evidence_rejects_even_when_existing_mode_would_win() {
    for mode in [0o10000, u32::MAX] {
        assert!(select_file_mode(Absent, Resource { captured_mode: Some(mode) }, Disabled).is_err(),
                "invalid captured resource mode was accepted");
        assert!(select_file_mode(Present(Some(0o640)), Resource { captured_mode: Some(mode) }, Disabled).is_err(),
                "unused invalid captured resource mode bypassed validation");
        assert!(select_file_mode(Present(Some(mode)), Generated, Disabled).is_err(),
                "invalid destination mode was accepted");
    }
}
```

These are library integration tests with no binary invocation or HOME mutation. Do not add a CLI harness call for a pure API. P10 input immutability follows from Copy value inputs; inspect purity rather than writing a tautological copy test.

- [x] **Step 2: Run acceptance RED.** Run cargo test --test file_permissions before adding the module. Preserve the expected unresolved-import compile RED separately from later semantic assertion RED controls.

- [x] **Step 3: Implement the minimal public policy.**

Add `pub mod permissions;` beside plan in files/mod.rs. Create the three enum definitions from the spec and a function following this complete algorithm; keep diagnostics contextual and document the inactive supplied-value boundary.

```rust
use anyhow::{Result, bail};

fn validate_mode(mode: u32, context: &str) -> Result<()> {
    if mode > 0o7777 {
        bail!("{context} mode must contain permission bits only (0..=0o7777)");
    }
    Ok(())
}

pub fn select_file_mode(
    destination: DestinationMode,
    source: PermissionSource,
    resource_execution: ResourceExecutionPolicy,
) -> Result<u32> {
    if let DestinationMode::Present(Some(mode)) = destination {
        validate_mode(mode, "destination")?;
    }
    if let PermissionSource::Resource { captured_mode: Some(mode) } = source {
        validate_mode(mode, "captured resource")?;
    }
    match destination {
        DestinationMode::Present(None) => bail!("existing destination mode is unknown"),
        DestinationMode::Present(Some(mode)) => {
            if mode & 0o7000 != 0 {
                bail!("destination special permission bits are unsupported for application");
            }
            return Ok(mode);
        }
        DestinationMode::Absent => {}
    }
    match (source, resource_execution) {
        (PermissionSource::Resource { captured_mode }, ResourceExecutionPolicy::OwnerExecutable) => {
            let mode = captured_mode.ok_or_else(|| anyhow::anyhow!(
                "resource owner-execute policy requires captured mode"
            ))?;
            Ok(if mode & 0o100 != 0 { 0o700 } else { 0o600 })
        }
        _ => Ok(0o600),
    }
}
```

- [x] **Step 4: Run focused GREEN and independently failing semantic controls.** Run cargo test --test file_permissions. Then perform one independent compiled omission per assertion: existing-mode preservation; unknown-present rejection; copying source mode under Disabled; any execute bit instead of owner bit; ignoring explicit Disabled; rejecting existing special bits; destination range validation; captured resource range validation; unknown executable-resource mode rejection. Preserve actual assertion output, exact original/restored SHA and focused GREEN after each. Keep temporary controls in owned scratch or /tmp, never commit them. Record any invalid control honestly and do not count compiler failures as semantic RED. Use the production code unchanged after exact restoration.

- [x] **Step 5: Complete current documentation.** Document exact API/selection/validation examples and remaining metadata/application/recovery boundaries in specification 4.10 and the existing durable records. Add the corresponding design/ADR/book links. Do not link durable documentation to docs/wip and do not rewrite prior raw archives or the broader application draft.

- [x] **Step 6: Verify and commit.** Run just fmt followed by just verify; run fresh cargo +1.85.0 check --locked --all-targets and cargo clippy --all-targets --locked -- -D warnings. Use isolated HOME and a task-specific CARGO_TARGET_DIR; preserve logs and machine-readable evidence. Check every inherited protected hash. Stage only assigned code/tests/current permission docs and assigned raw spec/plan (never the broader draft or scratch). Commit normally with `feat: select explicit file permission policy`; do not bypass hooks. No push or PR by the implementer.

- [x] **Step 7: Return complete evidence for the independent task gate.** Write task-1-report.md under .superpowers/sdd/2026-10-13-file-application with commands, outcomes, semantic controls, restoration hashes, actual touched files, commit and remaining limits. Controller runs task review and the required whole-branch review, scoped gardening of only this spec/plan pair, then hosted CI and the numbered PR passes. Do not spawn an implementer-owned broad reviewer.

## Controller handoff

Standing user delegation selects this scope and the preserved subagent-driven execution method after controller self-review of the spec and this plan. The delegation overrides skill prompts to reconfirm each already-authorized stage; record that ruling rather than claiming the human reviewed artifacts they did not see. The full executor draft remains separate design work.
