# M1 gate 4 file application and recovery Implementation Plan

Durable successors:
[requirements](../../specification/specification.md#412-qualified-file-execution-and-recovery),
[as-built design](../../design/qualified-file-execution.md), and
[AD-0033](../../adr/0033-qualified-file-execution-and-journals.md). This raw
approved working document preserves planning history. Hosted qualification
remains pending; archival does not complete gate 4 or M1.

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans`
> to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for
> tracking.

**Goal:** Apply and recover qualified file operations with durable receipts
while preserving unrelated edits.

**Architecture:** Add an opt-in executor under `filesystem`, borrowing the
existing scope writer guard and consuming complete planned files plus a final
state action. Private descriptor-based Unix operations provide file effects; a
strict journal and restricted desired-output payloads support recovery through
`operations::recovery::plan_recovery`. Existing CLI dispatch stays unchanged.

**Tech Stack:** Rust 2024, MSRV 1.85, existing rustix 1.1.4/fs, serde_json and
sha2; Linux/macOS local qualified filesystems. No new dependencies.

**Spec:** [Gate-4 design](../../design/qualified-file-execution.md), approved in
conversation.

## Global Constraints

- No production CLI activation, schema-2 codec, ownership adoption, native
  effects, or automatic maintenance.
- Require one qualified scope and its matching writer guard throughout
  application or recovery.
- Validate all preconditions before output effects; reobserve each destination
  immediately before its action.
- Preserve selected ordinary permissions; never recursively remove output
  directories.
- Journal under `<scope-root>/.upskill/state/`; private directories use 0o700
  and journal/payload/staging files use 0o600 until selected output mode is
  applied.
- Use forward recovery and separately report recorded and currently verified
  prefixes.
- No global/home writes from tests; use temporary scopes and existing fake-home
  CLI harness when invoking the CLI.
- Run `just fmt`, `just verify`, MSRV and supported-host checks before PR
  delivery.
- Existing untracked executor drafts in other worktrees remain outside this
  task.

## Review Focus

- A journal, payload, parent directory, or root replaced with an alias must
  reject without escaping the scope.
- A crash after replacement but before a receipt must acknowledge observed
  post-state rather than replay the write.
- A crash around final-state replacement must preserve the distinction between
  committed state and an incomplete receipt.
- A non-cooperating editor can change a file between actions; report the applied
  prefix and preserve unexpected observed bytes.
- An unchanged action or removal-only plan must retain correct modes and
  sequencing without unnecessary file replacement.

## Interfaces and persistence decisions

Create `src/filesystem/execution/mod.rs` with these public interfaces,
re-exported from `filesystem.rs`:

```rust
pub struct ExecutionRequest {
    pub files: Vec<crate::files::plan::PlannedFile>,
    pub state: crate::files::plan::PlannedFile,
}
pub struct ExecutionReport {
    pub recorded_prefix: usize,
    pub verified_post_prefix: usize,
    pub outcome: ExecutionOutcome,
}
pub enum ExecutionOutcome {
    Complete,
    Reconcile { index: usize },
}
impl QualifiedScope {
    pub fn apply_operation(
        &self,
        guard: &ScopeWriterGuard<'_>,
        sources: &[&SourceSnapshot<'_>],
        request: ExecutionRequest,
    ) -> anyhow::Result<ExecutionReport>;
    pub fn recover_operation(
        &self,
        guard: &ScopeWriterGuard<'_>,
    ) -> anyhow::Result<Option<ExecutionReport>>;
}
```

Require state to be present after the operation, to occupy a distinct location,
and to be the sole final `State` action. Reject duplicate/ancestor destinations,
reserved journal paths, cross-scope actions, invalid paths and unsupported
modes. `PlannedFile` is a planned before/after value, not ownership authority:
the caller remains responsible for deriving a complete ownership-aware plan.

Private modules: `request.rs` validates and normalizes action inputs;
`backend.rs` handles descriptor-relative operations; `journal.rs` owns the
strict schema-1 execution journal; `run.rs` drives application/recovery. Use
separate `*_tests.rs` files if needed.

Journal `operation.json` contains a random exclusive operation identity,
qualified root identity, generation, ordered relative action paths/kinds,
before/after byte digests/lengths/modes or absence, desired-payload identities,
and `Queued/Pending/Done` receipts. Use strict serde tagged representations,
reject unknown fields and invalid complete sequences before effects. Payloads
contain desired owned outputs and final state only; retain no old user-file
copies. Bind every payload by digest, size and exclusive file identity.

Persist each journal generation by write + file sync + atomic rename + parent
directory sync. Persist and sync desired payloads before publishing the initial
journal. For an effectful action: persist `Pending`, reobserve destination,
perform atomic replace or individual unlink, sync the destination directory,
reobserve post-state, then persist `Done`. No-op actions use the recovery
model's checkpoint-only step. Completed journals remain recognizable until
qualified cleanup succeeds. Recovery uses persisted desired payloads, not a
fresh source fetch; source snapshots are verified before initial application.

Document the namespace race limit: advisory locks exclude cooperating upskill
writers, not external editors. Rechecks and pinned handles reject observed
changes; no cross-file atomicity or exclusion of every possible external race is
claimed. Reuse existing filesystem qualification and reject any operation whose
durability/containment requirements cannot be established.

---

### Task 1: Validate complete execution requests

**Files:** Create `src/filesystem/execution/mod.rs`, `request.rs`; modify
`src/filesystem.rs`.

**Interfaces:** `request::validate(&ExecutionRequest) -> Result<()>` is private.
Public types are defined above; command dispatch remains unchanged.

- [ ] Write unit tests `rejects_duplicate_or_ancestor_actions`,
      `rejects_cross_scope_or_reserved_paths`,
      `requires_distinct_present_final_state`, and
      `retains_noop_and_removal_actions`. Assert invalid requests yield errors
      before any filesystem setup; valid requests retain exact bytes, modes and
      action order.
- [ ] Run `cargo test filesystem::execution::request`; expect missing
      module/types before implementing them.
- [ ] Implement the request types and validator using existing path/mode
      validation. Reject unknown Unix modes instead of guessing; enforce the
      existing permission-policy contract.
- [ ] Run the focused tests; expect all pass. Add tests for state-only
      operations and an empty file-action list.
- [ ] Commit: `feat: validate complete file execution requests`.

### Task 2: Add qualified per-file effects

**Files:** Create `src/filesystem/execution/backend.rs`, `backend_tests.rs`;
modify `src/filesystem/scope.rs` only for private guard/scope checks.

**Interfaces:** Private
`backend::apply(scope: &QualifiedScope, action: &PlannedFile) -> Result<()>`;
private
`backend::observe(scope: &QualifiedScope, locations: &[FileLocation]) -> Result<Vec<ObservedFile>>`.
Public observations continue using `DestinationSnapshot`.

- [ ] Write real-filesystem tests `replacement_preserves_selected_mode`,
      `removal_retains_extra_files_and_directory`, `noop_preserves_identity`,
      `parent_symlink_or_root_swap_rejects`, and
      `changed_destination_before_action_rejects`. Assert exact contents/modes
      and outside-scope sentinels survive rejected operations.
- [ ] Run `cargo test filesystem::execution::backend`; confirm failures reflect
      absent effects.
- [ ] Implement descriptor-relative traversal and exclusive sibling staging
      using existing rustix qualification. Qualify and create missing output
      parents safely; sync new directory bindings. Set selected mode before
      publication, sync staged bytes, rename relative to qualified parent, and
      sync that parent. Unlink individual unchanged files only. Reject special
      entries, aliases, unsupported mounts and mode preservation that cannot be
      qualified.
- [ ] Run focused tests on Linux/macOS. Assert unsupported-host paths return
      explicit errors and do not emulate weaker effects.
- [ ] Commit: `feat: apply qualified file effects`.

### Task 3: Persist strict journals and desired payloads

**Files:** Create `src/filesystem/execution/journal.rs`, `journal_tests.rs`;
extend private backend helpers.

**Interfaces:** Private
`Journal::create(scope: &QualifiedScope, request: &ExecutionRequest) -> Result<Journal>`,
`Journal::load(scope: &QualifiedScope) -> Result<Option<Journal>>`, and
`Journal::record(&mut self, index: usize, checkpoint: operations::recovery::Checkpoint) -> Result<()>`.
Journal owns its validated operation identity and payload bindings.

- [ ] Write tests `journal_roundtrip_retains_complete_action_sequence`,
      `corrupt_or_unknown_journal_rejects`,
      `payload_digest_or_identity_mismatch_rejects`,
      `existing_unowned_state_directory_rejects`, and
      `journal_permissions_are_restricted`. Assert no destination changes on
      rejection and no journal paths can refer outside the qualified scope.
- [ ] Run `cargo test filesystem::execution::journal`; verify expected red
      failures.
- [ ] Implement strict journal validation, exclusive operation/payload creation,
      generated ignore-file handling and durable generation ordering defined
      above. Existing private-directory state must pass ownership/binding
      checks; never adopt conflicting manual content. Keep partial-setup cleanup
      limited to qualified files created by this operation.
- [ ] Test interrupted journal publication and failed sync with a private
      fault-injection backend. The last complete generation remains readable or
      the operation reports reconciliation; no output effect may precede
      published intent.
- [ ] Commit: `feat: persist file operation checkpoints`.

### Task 4: Coordinate application and forward recovery

**Files:** Create `src/filesystem/execution/run.rs`, `run_tests.rs`; wire public
methods in `mod.rs`.

**Interfaces:** Implement the two `QualifiedScope` methods defined above. Errors
before operation creation use `Err`; observed recovery conflicts return
`ExecutionOutcome::Reconcile` with verified/recorded prefixes. I/O errors retain
actionable context and durable journal evidence.

- [ ] Write tests `all_preconditions_checked_before_first_output`,
      `foreign_guard_rejects`, `source_changed_after_plan_rejects`,
      `later_edit_preserves_applied_prefix`,
      `pending_poststate_is_acknowledged`,
      `queued_poststate_requires_reconciliation`, and
      `state_only_and_noop_operations_complete`. Assert outputs/state and both
      prefix counts, not only return codes.
- [ ] Run `cargo test filesystem::execution::run`; verify missing orchestration
      fails.
- [ ] Verify the guard belongs to this scope, validate the complete request and
      qualified snapshots, then create durable intent. On every step reobserve
      the full action sequence, classify exact before/after bytes and modes,
      call `plan_recovery`, and perform only its permitted step. Recheck the
      selected destination immediately before an effect. Revalidate root
      bindings and retain the guard throughout.
- [ ] Implement recovery with no source/network access, validating persisted
      root/action/payload identities first. A different observed state remains
      untouched. Final state follows the same checkpoint protocol. Qualified
      cleanup failures retain a recognizable completed journal and report
      context.
- [ ] Run focused suites and inspect no-op behavior, missing files, changed
      modes, final-state conflicts and invalid checkpoint sequences.
- [ ] Commit: `feat: coordinate file application and forward recovery`.

### Task 5: Prove process interruption and concurrency behavior

**Files:** Create `tests/filesystem_execution.rs`,
`tests/common/execution_probe.rs`; keep private fault-injection support inside
the executor test modules. The probe is a test-only subprocess entrypoint, never
a production CLI flag.

**Interfaces:** Parent test runs the same integration-test executable in an
explicitly selected probe test, with temporary scope and checkpoint-stop
configuration. Child writes a readiness receipt and waits for parent
termination. Parent then invokes public recovery in a fresh process.

- [ ] Add `recovers_after_every_effect_and_receipt_boundary`, covering every
      File and final State action before/after Pending persistence,
      replacement/unlink, directory sync and Done persistence. Include two
      changed files, a removal, a no-op and final state. Assert exact desired
      bytes/modes and no replay of observed completed effects.
- [ ] Add `concurrent_writer_is_rejected_and_kill_releases_lock`,
      `unexpected_edit_after_crash_survives`, and
      `readonly_observation_creates_no_execution_artifacts`. Use real processes
      and parent-controlled readiness handshakes with bounded timeouts; no
      timing-only sleeps.
- [ ] Run `cargo test --test filesystem_execution`; confirm tests fail when
      checkpoint persistence/recovery is deliberately bypassed, restore exactly,
      then run normally.
- [ ] Add cases for crash during initial setup and qualified completed-journal
      cleanup. Recovery must not delete unknown files added by the user.
- [ ] Commit: `test: pin file execution crash and writer behavior`.

### Task 6: Document guarantees and qualify supported hosts

**Files:** Create `docs/adr/0033-qualified-file-execution-and-journals.md`,
`docs/design/qualified-file-execution.md`; modify `docs/adr/README.md`,
`docs/specification/specification.md`, `.github/workflows/ci.yml`, and the M1
delivery plan's status paragraphs.

**Interfaces:** Documentation describes the delivered opt-in API, journal
ordering, advisory-lock/race limits and unsupported platforms. Add
`macos-latest` to the existing test matrix; Windows exercises explicit
unsupported behavior.

- [ ] Record the selected durability protocol and qualified filesystem limits in
      AD-0033; check the ADR number is still free before creating it. Explain
      payload retention, final-state receipts, recovery conflicts and cleanup
      ownership without claiming CLI/schema-2 activation.
- [ ] Run `cargo test --test filesystem_execution` and executor unit tests, then
      `just fmt` and `just verify`. Run Rust 1.85 all-target checks and the
      repository's required lint commands.
- [ ] Obtain Linux/macOS hosted results for process/crash tests and record exact
      revision evidence. Unsupported hosts must compile and fail explicitly only
      when the API is invoked.
- [ ] Update M1 gate-4 status only to the extent supported by those results;
      gate 5 and gate 6 remain distinct requirements. Garden this task's
      approved spec/plan into durable records before PR delivery using the
      repository's gardening workflow.
- [ ] Commit: `docs: qualify file execution and recovery guarantees`.

## Plan self-review

The six tasks cover all ten acceptance examples. Request/backend tests cover
qualification and removal; journal tests cover corruption and restricted state;
runner tests cover preflight and exact prefix decisions; subprocess tests cover
every output/final-state interruption and lock lifetime. No task changes CLI
dispatch or creates schema-2 ownership policy. Journal and scope identities are
private validated data, never caller-provided execution authority.
