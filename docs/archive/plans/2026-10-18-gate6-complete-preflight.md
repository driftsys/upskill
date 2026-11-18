# Gate 6 complete preflight and conflict reporting implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for
> tracking.

**Goal:** Report every independently observable selected-item failure and
destination conflict before creating an executable operation or publishing
changes.

**Architecture:** Collect source preparation outcomes in the opt-in lifecycle
orchestration layer. Run a reporting-only preflight over successful captures,
retained ownership and qualified destination observations, then use the existing
complete shared planner and opaque prepared-operation boundary only when the
report has no blockers. Keep pure planners' first-error APIs and production
schema-1 dispatch unchanged.

**Tech Stack:** Rust edition 2024, MSRV 1.85, existing synchronous source
capture, qualified filesystem sessions and ownership planners; no new
dependencies.

**Spec:** [Gate 6 design](../specs/2026-10-18-gate6-activation-design.md),
especially G6-03 and Diagnostic boundary;
[lifecycle contract](../../topics/0005-lifecycle-planning.md).

## Global Constraints

- Preserve the four-client MVP: Claude Code, Copilot CLI, Codex CLI and
  OpenCode.
- Existing pure planners may retain first-error results; the orchestration layer
  owns operation-level collection and stable ordering by installation identity
  and diagnostic category.
- A failed source must not hide independently observable conflicts in other
  selected items.
- Do not invent downstream diagnostics for an item that failed resolution or
  compilation.
- Root qualification, writer-lock acquisition and invalid authoritative state
  remain operation-level failures: stop when safe inspection cannot proceed.
- Introduce no automatic legacy ownership adoption.
- Keep schema-1 production routing until activation gates pass.
- Only `main.rs` writes to stdout/stderr; this library slice returns structured
  reports.
- Preserve qualified root, writer guard, exact backup, recovery-first behavior
  and capture revalidation. Preflight never publishes, creates a journal or
  executes a successful subset.
- Native qualification, compatibility adapters, external-home authority
  enforcement and production activation remain separate Gate 6 tasks. This slice
  supplies G6-03 library evidence only.

## Review Focus

1. A missing source beside two conflicting valid sources yields all three
   failures, without invented destinations for the missing source (Task 2 and
   Task 3).
2. Conflicts involving unchanged or unselected owners remain visible; incidental
   readers never acquire ownership (Task 3 and Task 4).
3. Two valid contributions at one canonical instruction file coexist;
   whole-file/contribution collisions and ancestor destinations fail (Task 3).
4. Reordered inputs, duplicate requests and failed normalization yield
   reproducible identities and report order (Task 1 and Task 2).
5. Pending recovery, unsafe observations and post-preparation drift cannot be
   bypassed by diagnostic collection (Task 2 and Task 4).

## File map and delivery boundary

- Create `src/lifecycle/preflight.rs`: structured diagnostics, ordering and
  reporting-only selected-set conflict checks.
- Modify `src/lifecycle/mod.rs`: expose the preflight report types.
- Modify `src/lifecycle/orchestration.rs`: collect independent source outcomes
  and retain captures; return blocked reports without invoking the prepared
  callback.
- Modify `src/lifecycle/commands.rs`: collect qualified observations and run
  reporting before complete planning; preserve existing prepared-operation
  lifetimes and apply authority.
- Reuse `src/lifecycle/desired/batch.rs` lexical conflict semantics,
  `src/lifecycle/sharing/` compatibility and physical identity checks, and
  `src/operations/plan.rs` mixed artifact validation. Do not replace these with
  a second writer or relax their checks.
- Modify `tests/lifecycle_orchestration.rs`,
  `tests/lifecycle_codex_orchestration.rs` and `tests/lifecycle_commands.rs`;
  create `tests/lifecycle_preflight.rs` for pure conflict fixtures.
- Update `docs/topics/0005-lifecycle-planning.md` and
  `docs/specification/specification.md` with the opt-in reporting contract. No
  production support claim in usage docs.

Before implementation, reconcile the live M1 tracking hierarchy and create one
bounded child task under its existing human-owned epic; update the parent list.
No remote issue state is asserted by this plan. Execute in an isolated worktree
with `./bootstrap`; preserve these uncommitted planning documents. Commit only
explicit task files, never blanket-stage unrelated WIP.

### Task 1: Define deterministic structured preflight reports

**Files:** Create `src/lifecycle/preflight.rs`; modify `src/lifecycle/mod.rs`,
`src/lifecycle/commands.rs`; test `tests/lifecycle_preflight.rs`.

**Interfaces:**

- Produce `PreflightCategory` with explicit ordering: `Binding`, `Resolution`,
  `Capture`, `Compilation`, `Capability`, `Destination`, `Ownership`.
- Produce
  `PreflightDiagnostic { installation_ids: Vec<String>, category: PreflightCategory, location: Option<FileLocation>, message: String }`
  and
  `PreflightReport { diagnostics: Vec<PreflightDiagnostic>, instruction_readers: Vec<SourceInstructionReaders> }`.
- Add `CommandDiagnostic::Preflight(PreflightReport)` so existing
  `Blocked(Vec<CommandDiagnostic>)` outcomes retain their outer shape and
  blocked operations can expose reader reports.
- Produce `normalize_report(report: &mut PreflightReport)`: sort/deduplicate
  participant identities; order diagnostics by identities, explicit category
  rank, raw scope/root/path and message. Deduplicate exact records only. Sort
  reader entries by installation identity. Never use input position or hash-map
  iteration as ordering authority.

- [ ] Write `report_order_is_input_independent` with two owners and two
      categories. Assert identical reports for reversed inputs, sorted
      participant IDs, and exact-duplicate removal without merging distinct
      locations/categories.
- [ ] Write `report_preserves_raw_root_identity` using lexically distinct roots.
      Assert no lossy string conversion merges roots.
- [ ] Run `cargo test --test lifecycle_preflight`; expect failure because the
      report API is absent.
- [ ] Implement the types and normalizer. Keep messages descriptive, but use
      typed categories and locations for assertions and sorting.
- [ ] Run `cargo test --test lifecycle_preflight`; expect all report tests to
      pass.
- [ ] Commit explicit files with
      `feat: define structured lifecycle preflight reports`.

### Task 2: Collect independent resolution and compilation outcomes

**Files:** Modify `src/lifecycle/orchestration.rs`; test
`tests/lifecycle_orchestration.rs`, `tests/lifecycle_codex_orchestration.rs`.

**Interfaces:** Consume Task 1 reports. Preserve public `with_prepared`,
`with_prepared_codex`, `SourcePrepared::apply` and callback signatures.
Successful entries retain the existing `ResolvedRuleSource` and capture
lifetimes until the callback and application finish.

- [ ] Add `preflight_collects_independent_source_failures`: select a missing
      local source, malformed RULE.md and unsupported capability plus a valid
      control. Assert exactly one applicable failure per failed preparation
      stage, the valid control is inspected, and callback count is zero. Assert
      destination/state/backup bytes and modes unchanged and no pending
      operation.
- [ ] Add `preflight_binding_identity_survives_failed_resolution`: reverse add
      requests and include duplicate aliases/bindings. Assert stable reports.
      For add identity, use normalized requested source label and the existing
      tuple-framed installation identity before fetching; when normalization
      fails, use the original typed source label in the same framing solely as a
      diagnostic identity. Never persist this fallback or infer resolved source
      evidence from it. Updates always use recorded IDs.
- [ ] Run
      `cargo test --test lifecycle_orchestration --test lifecycle_codex_orchestration`;
      expect new assertions to fail on current first-error behavior.
- [ ] Replace fail-fast source iterator collection and per-item binding/compile
      propagation with per-item results. Stop downstream work for each failed
      item; continue other items. Classify typed `CapabilityError` separately
      from compilation failures. Retain reader reports for successfully compiled
      Codex items, including zero-output items. Collect duplicate binding
      conflicts without silently replacing a request.
- [ ] Preserve session opening and recovery precedence. Invalid shared
      profile/selection or authoritative state remains an operation-level error
      where no safe per-item inspection exists. Source-local capture failures
      are collected; unsafe authoritative/destination access stops the
      operation.
- [ ] Pass successful captures to Task 3's reporting path even when source
      diagnostics exist. Never send an incomplete set to ordinary upsert
      preparation or reconcile it as the next state.
- [ ] Re-run the two integration targets and the existing recovery-first unit
      test with `cargo test codex_pending_recovery`; expect passes, including
      zero callback invocations on blocked requests.
- [ ] Commit explicit files with
      `feat: collect independent lifecycle source diagnostics` after Task 3
      integration makes the acceptance path executable. Do not commit a broken
      intermediate API.

### Task 3: Report complete destination and ownership conflicts

**Files:** Modify `src/lifecycle/preflight.rs`, `src/lifecycle/commands.rs`,
`src/lifecycle/orchestration.rs`; test `tests/lifecycle_preflight.rs`,
`tests/lifecycle_commands.rs`, `tests/lifecycle_orchestration.rs`.

**Interfaces:** Produce
`collect_conflicts(prior: &CombinedState, successful: &CapturedUpsert, observations: &[ObservedFile]) -> Result<Vec<PreflightDiagnostic>>`
in `preflight.rs`. It reports only; it cannot return an execution plan. Reuse
Task 1 normalization. Qualified observation remains owned by `LifecycleSession`;
expose no raw-path writer or caller-supplied authoritative state.

- [ ] Add `invalid_source_does_not_hide_two_destination_conflicts`: one missing
      source and two valid sources targeting independently occupied
      destinations. Assert the sorted inventory contains exactly the resolution
      failure and two destination failures, no destination for the missing
      source, unchanged manual bytes/modes, unchanged state/backup, no journal
      and no callback.
- [ ] Add table-driven pure fixtures named `complete_conflict_inventory`:
      duplicate installation identities, same-path incompatible file
      bytes/modes/contracts, ancestor/descendant paths, file/instruction
      collisions, duplicate contribution IDs, retained unselected owners, legacy
      reservations and reserved lifecycle paths. Assert exact participants,
      locations and categories for two independent conflicts in each applicable
      case. Include identical shareable files and distinct valid contributions
      as zero-conflict controls.
- [ ] Add `unchanged_selection_does_not_hide_changed_conflict`: mixed
      changed/unchanged selected items and unrelated survivor state. Assert both
      conflicts remain visible and survivor bytes, modes, ownership, references
      and contracts are preserved.
- [ ] Run
      `cargo test --test lifecycle_preflight --test lifecycle_commands --test lifecycle_orchestration`;
      expect new conflict inventories to fail before implementation.
- [ ] Build a complete location/participant index from successful desired
      artifacts plus retained ownership and selected old destinations. Check
      cross-item structure before per-location content checks. Preserve legal
      sharing and distinct contributions. Use existing validators/planners per
      independent location or connected conflict group, with the group's
      complete owners/contracts; never validate each incoming owner against an
      artificially empty prior state.
- [ ] Observe the union once through the qualified session, including old
      destinations needed for update/removal. Reject reserved paths before
      observation. If a location cannot safely be observed, stop with an
      operation-level error; do not follow unsafe paths to gain more
      diagnostics.
- [ ] Collect one typed diagnostic per independently checked conflict
      group/category. A location whose structural validation failed must not
      produce speculative content diagnostics. Do not repeatedly delete failed
      items and rerun the whole planner; that changes ownership context and can
      fabricate failures.
- [ ] Merge source and conflict diagnostics, normalize, and return `Blocked`
      with all available readers when nonempty. Only an entirely clean selected
      set enters existing `prepare_upsert`, `plan_shared_upsert` and opaque
      preparation. A remaining pure-planner invariant error still aborts without
      publication; never convert it to a fabricated item failure or proceed
      anyway.
- [ ] Re-run the three targets plus
      `cargo test --test lifecycle_upsert --test lifecycle_sharing`; expect
      exact inventories and legal-sharing controls to pass.
- [ ] Commit explicit files with
      `feat: report complete lifecycle destination conflicts`.

### Task 4: Pin reader exposure, authority and no-effect guarantees

**Files:** Modify `tests/lifecycle_codex_orchestration.rs`,
`tests/lifecycle_commands.rs`, `tests/lifecycle_orchestration.rs`; adjust
implementation from Tasks 1–3 only where tests require it.

**Interfaces:** Consume `CommandDiagnostic::Preflight`, existing reader roles
and `SourcePrepared::instruction_readers`. Do not change owner-consumer or
owner-contract authority based on the reader inventory.

- [ ] Add `blocked_preflight_keeps_excluded_and_incidental_readers`: two
      source-backed instruction items, including an
      audience-excluded/zero-output item, plus an independent blocker. Assert
      exact compiled reader inventories and roles on the blocked report. Compare
      state ownership and consumer associations byte-for-byte with the baseline.
- [ ] Add `clean_preflight_retains_capture_guards`: prepare successfully, then
      independently change source bytes, Git revision, destination bytes/mode
      and state. Each mutation must reject apply before a new journal or
      publication; an unchanged control must apply successfully.
- [ ] Add `operation_failures_precede_source_inspection`: invalid authoritative
      state and held writer lock with an invalid source. Assert operation-level
      failure and no source-resolution callback; retain existing
      recovery-complete and reconcile controls. Recovery may perform its own
      guarded effects and stops the new operation, so compare against the
      recovery contract rather than asserting no recovery writes.
- [ ] Run
      `cargo test --test lifecycle_codex_orchestration --test lifecycle_commands --test lifecycle_orchestration --test lifecycle_state_origin --test lifecycle_application`;
      expect new assertions to fail where reporting integration is incomplete.
- [ ] Complete report threading and retain all existing revalidation and
      recovery boundaries. Removal continues to use recorded roots/state without
      resolving sources or requiring an available compiler adapter.
- [ ] Re-run those targets plus `cargo test --test lifecycle_end_to_end`; expect
      all tests to pass.
- [ ] Commit explicit files with
      `test: qualify complete lifecycle preflight boundaries`.

### Task 5: Document and verify the bounded deliverable

**Files:** Modify `docs/topics/0005-lifecycle-planning.md`,
`docs/specification/specification.md`; update this plan's checkboxes only as
execution completes.

- [ ] Document diagnostic categories, stable ordering, failed-item suppression,
      partial observation limits, blocked reader reports, operation-level
      failures and no successful-subset execution. State that production
      routing/native activation remain pending. Record G6-03 example names and
      commands without claiming unrun platform or native evidence.
- [ ] Run `just fmt`, then inspect the diff and revert only unintended formatter
      changes to unrelated files; never overwrite pre-existing WIP edits.
- [ ] Run `just verify`, `just sanity`, `just msrv-check` and `just book`.
      Expect exit 0 and no warnings; inspect rendered links to the updated
      contract. Record unavailable checks as blockers rather than passes.
- [ ] Self-review the implementation against G6-03 and this plan, then obtain
      the repository's required independent review before delivery. Fix critical
      findings and file confirmed non-critical debt under the existing epic.
- [ ] Commit explicit documentation files with
      `docs: specify complete lifecycle preflight reporting`. Before opening a
      PR, garden only completed task WIP through the repository gardening
      workflow; retain the overarching activation design because its remaining
      gates are unfinished.
- [ ] Open one PR for this bounded task after local gates pass. Inspect
      exact-head hosted Linux/macOS checks and review evidence. Do not enable
      production schema-2 routes or claim Gate 6 completion from this PR.

## Coverage and handoff

G6-03 selected-set diagnostics are covered by Tasks 1–3; blocked and ready
reader reporting by Task 4. Existing G6-04/G6-05 safety boundaries receive
regression controls, not new activation qualification. G6-01/02/06–10
production/native obligations, broad compatibility adapters and settled Codex
home enforcement remain in the parent design's delivery sequence. No CLI switch
or schema migration is added here, so integration acceptance exercises the
actual opt-in library entry points rather than inventing a temporary production
command.

Self-review: every diagnostic-boundary requirement maps to a task; all five
Review Focus cases have named tests. Task 2 and Task 3 are sequential parts of
one deliverable and must be verified together before their commits. No
implementation or native qualification was performed while writing this plan.
