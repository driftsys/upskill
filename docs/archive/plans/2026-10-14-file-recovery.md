# File recovery model implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development. Steps use checkbox syntax for
> tracking.

**Goal:** Provide a pure ordered recovery decision API that checks the whole
supplied operation before proposing one checkpoint step.

**Architecture:** Add operations::recovery alongside the existing pure plan
module. Supplied checkpoint and observation classifications have no effect
authority. Keep durable journal encoding and physical file application outside
this change.

**Tech Stack:** Rust edition 2024, MSRV 1.85, anyhow, existing test
infrastructure; no new dependencies.

**Spec:** docs/wip/specs/2026-10-14-file-recovery-model-design.md

## Global Constraints

- Rust edition 2024, MSRV 1.85, existing dependencies only.
- No I/O, environment or clock input, process calls, output, mutation of inputs,
  serialization, native effects or schema changes.
- No physical qualification or execution authority follows from this API.
- Preserve all 56 inherited raw/WIP hashes and prior worktrees/evidence. The
  broader application draft remains a separate untracked design.
- Types and signatures match the spec exactly; all declared public model types
  derive Debug, Clone, Copy, PartialEq and Eq.
- Exactly one final State; checkpoints follow `Done* Pending? Queued*`; validate
  complete structure before observations and complete observations before Ready
  or Complete.
- Code, acceptance tests and current documentation belong in one task and PR.
  Use Conventional Commits and run just fmt then just verify before PR.

## Review Focus

- A late invalid checkpoint rejects even when an earlier observation conflicts;
  test structure before observation decisions.
- Both is a no-op for Queued but post-state acknowledgement for Pending; test
  both action roles across all twelve combinations.
- Recorded completion can exceed current verified post-state; test a
  contradicted completed prefix without replaying it.
- A pending final State already at post-state must be acknowledged; test its
  index and both prefixes.
- A late queued post-state cannot be adopted or overlooked; test complete-suffix
  reconciliation before an earlier next step.

## Task 1: Ordered recovery decisions and current documentation

**Files:**

- Create src/operations/recovery.rs: pure model and planner.
- Modify src/operations/mod.rs: export recovery.
- Create tests/recovery_plans.rs: literal acceptance cases, no CLI invocation or
  real HOME use.
- Update docs/design/file-recovery-model.md and
  docs/adr/0032-model-recovery-checkpoints-before-effects.md from
  selected/pending to implemented pure API; describe public decisions, prefixes
  and unresolved producer obligations.
- Update docs/specification/specification.md with the next numbered pure-model
  subsection; link current design and ADR from their existing indexes and
  docs/book/SUMMARY.md where those collections are listed.
- Modify only the current integration-status paragraph in
  docs/design/mixed-owned-artifact-planning.md for debt #278: permission
  selection is implemented; physical application and qualified integration
  remain pending. Link file-permission-policy.md.
- Do not stage docs/design/qualified-file-application.md or modify inherited
  WIP/raw archives. Controller owns issue metadata and later gardening.

**Interfaces:** Consumes a borrowed slice of RecoveryAction. Produces
plan_recovery(actions: &[RecoveryAction]) -> anyhow::Result<RecoveryDecision>
with the exact enums and fields in the spec. No lifecycle caller is activated.

- [x] **Step 1: Write literal acceptance tests and capture RED.**

Import the spec's public types from upskill::operations::recovery. Use the
following assertion helper so a changed Result is reported by a named assertion
rather than an earlier unwrap:

```rust
fn expect(actions: &[RecoveryAction], expected: RecoveryDecision, label: &str) {
    let actual = plan_recovery(actions);
    assert!(actual.as_ref().is_ok_and(|value| *value == expected),
        "{label}: expected {expected:?}; actual {actual:?}");
}
fn action(kind: ActionKind, checkpoint: Checkpoint, observed: ObservedRelation) -> RecoveryAction {
    RecoveryAction { kind, checkpoint, observed }
}
```

Write two independent literal tables of twelve expected outcomes, using the
spec's state-only table and the File table below. Do not compute expectations
from checkpoint predicates or introduce a second oracle algorithm. Literal enum
constructors can be aliased locally for readability.

For File cases append State/Queued/Before. File Queued/Before yields Ready
Apply0 prefixes0/0; Queued/Both yields CheckpointOnly0 prefixes0/0; Queued/After
and Neither reconcile0 prefixes0/0. File Pending/Before yields Apply0
prefixes0/0; Pending/After and Both yield Acknowledge0 prefixes0/1;
Pending/Neither reconciles0 prefixes0/0. File Done/After and Both yield Ready
Apply1 prefixes1/1; Done/Before and Neither reconcile0 prefixes1/0.

Add the ten literal multi-action examples from the spec, plus File Done/After +
State Done/Before -> Reconcile index1, recorded_prefix2, verified_post_prefix1.
Both recorded checkpoints remain Done even when the final State observation
contradicts completion.

Add File Queued/After + State Queued/Neither -> Reconcile index0
recorded0/verified0, and File Done/Before + File Pending/After + State
Queued/Before -> Reconcile index0 recorded1/verified0. These independently pin
first-conflict selection and contiguous-prefix stopping.

Invalid-structure cases are empty; Files only; State then File; two State
actions; Queued then Done; Pending then Done; Queued then Pending; two Pending.
Include valid final State where needed to isolate checkpoint errors. Assert Err
and diagnostic context (position or final State requirement), without requiring
diagnostic ordering for multiple structural defects. Add a late invalid Done
checkpoint after an earlier conflicting observation; it must still return Err.

Run via the unique-log runner:

```bash
python3 .superpowers/sdd/2026-10-14-file-recovery/recovery-run.py task1-acceptance-red cargo test --test recovery_plans
```

Expected: initial unresolved API failure. This is acceptance-first RED, not a
semantic mutation control.

- [x] **Step 2: Implement the public types and minimal pure planner.**

Copy exact public types from the spec. Add pub mod recovery to
operations/mod.rs. Implement this algorithm, retaining contextual anyhow errors
for structural defects:

```rust
pub fn plan_recovery(actions: &[RecoveryAction]) -> anyhow::Result<RecoveryDecision> {
    anyhow::ensure!(actions.last().is_some_and(|a| a.kind == ActionKind::State),
        "recovery requires a final State action");
    anyhow::ensure!(actions.iter().filter(|a| a.kind == ActionKind::State).count() == 1,
        "recovery requires exactly one State action");
    let recorded_prefix = actions.iter().take_while(|a| a.checkpoint == Checkpoint::Done).count();
    for (index, action) in actions.iter().enumerate().skip(recorded_prefix) {
        anyhow::ensure!(action.checkpoint != Checkpoint::Done,
            "recovery action {index}: Done checkpoint is outside completed prefix");
        anyhow::ensure!(action.checkpoint != Checkpoint::Pending || index == recorded_prefix,
            "recovery action {index}: Pending checkpoint must immediately follow completed prefix");
    }
    let mut verified_post_prefix = 0;
    let mut conflict = None;
    for (index, action) in actions.iter().enumerate() {
        let post = matches!(action.observed, ObservedRelation::After | ObservedRelation::Both);
        if index == verified_post_prefix && action.checkpoint != Checkpoint::Queued && post {
            verified_post_prefix += 1;
        }
        let valid = match action.checkpoint {
            Checkpoint::Done => post,
            Checkpoint::Pending => action.observed != ObservedRelation::Neither,
            Checkpoint::Queued => matches!(action.observed, ObservedRelation::Before | ObservedRelation::Both),
        };
        if !valid && conflict.is_none() { conflict = Some(index); }
    }
    if let Some(index) = conflict {
        return Ok(RecoveryDecision::Reconcile { index, recorded_prefix, verified_post_prefix });
    }
    if recorded_prefix == actions.len() {
        return Ok(RecoveryDecision::Complete { completed: actions.len() });
    }
    let index = recorded_prefix;
    let step = match (actions[index].checkpoint, actions[index].observed) {
        (Checkpoint::Pending, ObservedRelation::After | ObservedRelation::Both) => RecoveryStep::Acknowledge { index },
        (Checkpoint::Pending, ObservedRelation::Before) => RecoveryStep::Apply { index },
        (Checkpoint::Queued, ObservedRelation::Both) => RecoveryStep::CheckpointOnly { index },
        (Checkpoint::Queued, ObservedRelation::Before) => RecoveryStep::Apply { index },
        _ => anyhow::bail!("recovery action {index}: inconsistent validated checkpoint"),
    };
    Ok(RecoveryDecision::Ready { recorded_prefix, verified_post_prefix, step })
}
```

Use clear rustdoc to describe supplied classifications and absence of effect
authority. Do not add a redundant pending counter or a second computed oracle.

- [x] **Step 3: Run focused GREEN and independent semantic controls.**

```bash
python3 .superpowers/sdd/2026-10-14-file-recovery/recovery-run.py task1-focused-green cargo test --test recovery_plans
```

Expected: all acceptance tests pass. Perform thirteen independent one-at-a-time
compiled controls: remove final-State position guard; remove single-State count
guard; remove completed-prefix ordering guard; remove Pending position guard;
accept Done/Before; accept Queued/After; return Apply for Pending/After; return
Acknowledge for Pending/Before; return Apply for Queued/Both; exclude Pending
from verified-post extension; truncate observation scan through
recorded_prefix + 1; overwrite the first conflict with a later conflict; extend
the verified prefix for Pending post-state despite an earlier broken completed
prefix. Each must fail a named behavioral assertion. A guard may enforce
multiple related constraints; do not distort production code merely to isolate
redundant syntax. Use unique logs and save mutated sources, exact original
hashes and compiled failure names in scratch. Compiler failures do not count;
retain and disclose invalid attempts. Restore exact original source after each
control; rerun focused GREEN on restoration.

- [x] **Step 4: Update current docs and debt #278 wording.**

Document implemented pure classification and remaining qualified-producer,
identity, reconstruction, receipt, metadata and effects contracts. Add a concise
public API example using a Pending State/After -> Acknowledge result. Clarify
that Apply is a proposed logical step, not permission to write. Do not claim
actual recovery, journal or M1 completion. Index links must target durable
documents from the start, not WIP paths.

- [x] **Step 5: Run complete verification and inspect preservation.**

Run just fmt through the runner, then just verify with a new unique log. Run
locked all-target cargo check with toolchain1.85 and a fresh target directory
using a separately named runner environment; retain exact command and result.
Run all-target clippy with -D warnings if not already covered by just verify.
Check the 56 protected hashes against protected-before.json, the copied broad
draft against inherited-application-draft.json, and git diff --check. No claim
of hosted platform validation before CI runs. Count outer test results
separately from nested captured fixtures if reporting totals.

- [x] **Step 6: Commit, self-review and report.**

Stage only assigned code/tests/current durable docs/indexes and the #278
paragraph; keep broad application draft untracked. Commit with feat: model
ordered recovery checkpoints. Self-review the diff against every spec section
and Review Focus row. Save task-1-report.md in this plan's scratch with status,
commit, files, acceptance RED/GREEN, thirteen controls, restored hashes,
complete gate evidence, test counts and limitations. No worker-spawned agents.
Controller dispatches the task reviewer after the report.

## Controller closeout

After clean task review: mark these steps complete, garden only this new
spec/plan pair with exact raw hashes, preserve foreign WIP and broad draft, run
one whole-branch architecture review, and at most one combined final fix wave
plus scoped re-review. Open a single PR closing the new architecture-debt issue
and #278, perform the two numbered PR reviews, wait for exact-head CI success
and squash merge under standing delegated delivery authorization. Preserve
scratch and earlier worktrees.
