# M1 Gate 5 final acceptance evidence

Gate 5 remains an opt-in library path. Source-backed rule operations use
`orchestration::with_prepared`; supplied instruction contributions use
`commands::LifecycleSession`. These are the adapters available for Gate 6. There
is no production schema-2 activation or native Codex qualification.

## Acceptance matrix

The names below identify executable tests, rather than inferred guarantees.

| Criterion                                | Evidence                                                                                                                                                                                                                                                                                                                                 |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC1: ownership beside legacy             | `lifecycle_orchestration::migrated_disjoint_history_survives_and_complete_legacy_selection_blocks_before_source_work`; `lifecycle_commands::fresh_and_migrated_add_update_remove_preserve_extra_files_and_exact_backup`                                                                                                                  |
| AC2: exact backup and fresh absence      | `lifecycle_application::first_schema2_write_backs_up_exact_schema1_bytes_before_outputs`; `mismatched_existing_backup_rejects_transition_without_changing_state`; process acceptance below compares exact backup bytes and modes                                                                                                         |
| AC3: manual content and conflicts        | `lifecycle_commands::instruction_add_update_remove_preserves_manual_bytes_and_ordinary_mode`; `updating_and_removing_one_instruction_preserves_an_unselected_contribution`; `shared_file_contracts_retain_unselected_owner_and_edited_outputs_block`; `lifecycle_orchestration::manual_output_conflict_preserves_entire_state_and_files` |
| AC4: complete selection                  | `lifecycle_commands::complete_selection_blocks_before_builder_and_wrong_guard_is_rejected`; `all_selection_rejects_legacy_before_building_desired_values`; source orchestration migration test above removes the source before blocked selection                                                                                         |
| AC5: safe individual removal             | `lifecycle_orchestration::resource_policy_is_applied_to_output_and_contract_and_manual_neighbors_survive`; `lifecycle_commands::fresh_and_migrated_add_update_remove_preserve_extra_files_and_exact_backup`; `lifecycle_retirement::shared_whole_file_survives_until_final_owner_retires`                                                |
| AC6: observational inspection            | `lifecycle_inspection::qualified_inspection_does_not_create_storage_backup_or_lock`; `qualified_doctor_reports_drift_without_changing_state_or_outputs`; `scope_inspection_remains_read_only_while_writer_guard_is_held`; `lifecycle_preview::preview_preserves_physical_state_sources_outputs_and_unowned_resources`                    |
| AC7: stale evidence and writer rejection | `lifecycle_commands::stale_state_source_or_destination_rejects_without_partial_output`; `lifecycle_orchestration::stale_source_after_preparation_rejects_without_destination_effects`; process acceptance rejects a second writer while the child is paused                                                                              |
| AC8: checkpoint recovery                 | `lifecycle::acceptance_tests::every_observed_lifecycle_checkpoint_recovers_exact_captured_intent`, complemented by executor process tests in `src/filesystem/execution/process_tests.rs`                                                                                                                                                 |
| AC9: retained and unsupported records    | `lifecycle_inspection::legacy_inspection_retains_exact_bytes_unknown_fields_and_duplicate_rows`; `mixed_state_keeps_unresolved_native_and_legacy_rows_visible`; `lifecycle_retirement::unsupported_owned_kind_blocks_retirement_but_survives_disjoint_work`; captured command selection tests above                                      |
| AC10: inactive production boundary       | `lifecycle_end_to_end::production_add_retains_schema_one` invokes the actual CLI with isolated HOME and checks schema 1 and backup absence                                                                                                                                                                                               |

## Real process coverage

The unit driver records actual executor events from successful source-backed
operations, restores their exact before-state, then starts a fresh test process
for each event. It preserves the fresh and migrated first-transition add, update
and remove matrix with source-backed rule and resource outputs. An additional
captured-adapter matrix covers instruction add, update and remove with a manual
prefix and distinct selected/retained contributions sharing `AGENTS.md`. The
retained installation, contribution ownership, consumer association, direct
reference and ordinary mode 0640 are checked independently of the
successful-operation image. An unrelated manual resource neighbor is checked
independently too. Migration contains duplicate unresolved legacy rows; the
instruction setup establishes the retained owner before the interrupted
operation, while rule/resource cases retain first-transition coverage.

Every emitted `before_pending`, `after_pending`, `before_effect`,
`after_effect`, `after_effect_sync`, `before_done`, `after_done` and
`after_ack_sync` event is interrupted, including the final-state action and
migration backup creation. An absence-only backup precondition has no physical
effect checkpoints; the matrix follows emitted events rather than inventing
those checkpoints.

The parent verifies concurrent writer rejection, terminates the child, opens a
new writer session and recovers through `orchestration::with_prepared`. A new
request names a missing source and its preparation callback must never run;
recovery returns before source work and stops that request. Every observed
checkpoint already has a persisted journal, so a ready result fails. Source
bytes change after termination. Recovery must still match the successful
operation's exact regular-file image outside executor storage, including
combined-state bytes, output bytes, backup bytes and modes. A second session
must be ready without replay. In every instruction case the parent edits the
unrelated manual neighbor after termination; only that neighbor's expected image
is adjusted, and its new bytes must survive recovery. Manual prefix, retained
body/records/reference and mode must still match independent assertions. For
source-backed update, an additional late external edit must produce
reconciliation, preserve the edited output and old combined state, and retain
pending recovery.

The ignored probe is invoked explicitly by the driver and clears inherited
`GIT_*` variables. No production hook or command-line flag is introduced. This
tests process termination and persisted recovery, not hardware power loss.
Instruction desired sets are explicitly caller-authorized supplied payloads
bound to actual qualified source captures. The snapshot guards source evidence;
it does not prove an instruction compiler or native client discovery.

## Completion boundary

Local validation results are reported by the delivery agent. Hosted CI,
independent final review and merge remain required before declaring Gate 5
complete. Compiler/native qualification and production activation remain Gate 6
work.

## Accounted executor staging residue

Killing fresh add at `before_effect` index 0 can leave a sibling
`.claude/rules/.upskill-stage-<nonce>` after successful recovery. The matrix
also observes analogous resource-parent and final-state-parent stage files. This
is tracked in [debt #316](https://github.com/driftsys/upskill/issues/316).

The comparison accounts for each observed stage file explicitly: it must be a
regular file and match the bytes, selected mode and parent directory of a known
expected final regular file in that parent. The helper does not bind that match
to a particular interrupted action token; backend inspection establishes that
staging writes and verifies the action's after value before `before_effect`.
Fresh outputs and backups use mode 0600 in these fixtures; the migrated state
retains the observed mode 0644. Every other regular file remains in the exact
comparison. Directory modes, symlinks and filesystem identity are outside this
image comparison. The inode-bound executor storage under `.upskill` is outside
the content image; successful recovery explicitly requires no pending operation,
while reconciliation retains one.

This allowance describes observed disk residue. It makes no claim about native
client discovery or whether those clients ignore staging names. Gate 6 must
qualify that exposure. No production cleanup based on a name prefix is added.

## Local validation snapshot

On 2026-10-18, the process matrix passed 162 emitted checkpoint interruptions
plus two late-edit reconciliation cases in 74.48 seconds. The dedicated process
log was `/private/tmp/upskill-gate5-process-acceptance.log`.

The actual CLI boundary test passed. Fifty focused integration tests passed
across application (8), captured commands (11), inspection (10), source
orchestration (11), and preview (10). `just fmt`, all-target Clippy with
`-D warnings`, `just msrv-check` with Rust 1.85, Markdown lint, and diff
whitespace checks passed. These checks used a dedicated target directory;
integration launchers removed inherited `GIT_*` variables.

The combined final source and adapter tree subsequently passed full
`just verify`; its log is `/private/tmp/upskill-gate5-final-verify.log`. That
run includes Rust tests, Clippy, Rust and Markdown format checks, Markdown lint,
shell and hook checks, documentation regression checks, rendered book validation
and build. The combined tree also passed explicit Rust 1.85 `just msrv-check`;
its log is `/private/tmp/upskill-gate5-final-msrv.log`. At that checkpoint the
command suite contained fifteen examples, bringing the five focused integration
suites to fifty-four examples; the eleven-command and fifty-total counts above
describe the earlier snapshot. Hosted CI, independent final delivery review and
merge remain pending. Local verification does not substitute for those delivery
requirements.

## Independent local acceptance audit

A separate read-only audit checked the implementation, bounded plans and process
log against all ten umbrella acceptance criteria. It found no concrete Gate 5
implementation blocker. It classified AC8 as covered with the explicit #316
residual: additional copies of known intended bytes can remain, while recorded
destination effects and retained state recover correctly. AC8 does not promise
removal of every temporary file.

The audit did not rerun tests or establish full-suite, MSRV or hosted CI results
independently. It requested the regular-file image qualification and the precise
staging-match description recorded above. The coordinating delivery verified the
combined tree separately as recorded above. Hosted review/CI and merge remain
delivery gates.

## Umbrella reconciliation and archive

The locally implemented umbrella now joins combined state, compiler-to-desired
sets and reference authority, guarded origin/backup/executor recovery, captured
inspection and preview, applying adapters, and explicit local/Git rule-source
orchestration. It does not adopt schema-1 outputs, register native integrations,
activate production writes or qualify the Gate 6 Codex adapter and selector.

The dispatcher reports preview PR #312 and origin PR #314 merged, followed by
source PR #315 at `01fbe9a384a21e98483ac2a7e3116dd3f9c04d88`. Captured-command
PR #317 merged as `6e2978b2ff566b706b6c9a7604ed99a129a71c3a` after two full
review passes and final-head hosted checks. Its transient bootstrap HTTP403
passed on retry. Final orchestration delivery review, hosted CI and merge remain
separate delivery checkpoints. This record claims local implementation and
acceptance completion only.

The unchanged
[umbrella specification](../archive/specs/2026-10-15-m1-gate5-lifecycle-integration.md)
and [completion plan](../archive/plans/2026-10-18-gate5-completion.md) retain
historical planning status and pending-task prose as provenance. Durable
successors are
[combined requirements and architecture](../topics/0005-lifecycle-planning.md#combined-owned-content-lifecycle-contract),
[selected sources](../topics/0003-sources.md#selected-lifecycle-rule-sources),
[state and origin](../topics/0004-state-and-ownership.md#durable-lifecycle-origin),
and
[backup and recovery](../topics/0008-execution-and-recovery.md#combined-lifecycle-backup-and-recovery).
Only this exact umbrella pair is archived. All other WIP remains at its original
path with unresolved disposition.

Archive SHA-256 digests:

- Umbrella specification:
  `916606a7078be19fca4cafab69b0ae9b118d476bcdc9fd88a2441a5c08bef5ec`.
- Completion plan:
  `816ac4d108884b6ccc8d76d5713363f92757c7211db49969ec44d17a556d248f`.

## Quick-review acceptance correction

The final test wave requires recovery through `orchestration::with_prepared`
with an unavailable new source and a preparation callback that must not run.
Every observed effect/receipt checkpoint already has a persisted journal; a
ready result now fails instead of retrying the request. The strict session
recovery control passed in 104.90 seconds before the orchestration-entrypoint
correction. The full corrected tree passed `just verify` in
`/private/tmp/upskill-gate5-wave-verify.log` and Rust 1.85 in
`/private/tmp/upskill-gate5-wave-msrv.log`.

The orchestration suite now contains twelve examples. Repeated add must refresh
changed content and source evidence while preserving identity and references.
Apply-time Git identity is pinned by changing a retained checkout HEAD to
another commit with the same tree: application rejects without state/output
changes or pending intent. Independent controls deleting Git revalidation and
making repeated add a successful no-op both failed the strengthened assertions.
Duplicate target rejection and complete two-source add/update/remove are also
asserted. These controls do not replace final hosted qualification or review.

## Integrated delivery checkpoint

The final branch includes the captured adapter's sixteen examples and the
orchestration suite's twelve examples: the five focused suites contain 56 tests.
Full integrated verification passed in
`/private/tmp/upskill-gate5-integrated-verify.log`. The subsequent parent change
only reverses supplied contract order in the new permission fixture; its sixteen
adapter tests and Rust 1.85 check passed again, with no production-code change.
The final parent-integrated branch's verification is recorded in
`/private/tmp/upskill-gate5-delivery-verify.log` and its Rust 1.85 check in
`/private/tmp/upskill-gate5-delivery-msrv.log`.

This note records pre-merge local checkpoints. The reviewed pull requests and
MVP epic #283 carry the exact hosted-head and merge evidence. Gate 5 delivery
requires those checks and merges; no local result activates production schema 2
or establishes native Codex discovery. Stage residue #316 and advisory debt
#313, #311 and #309 remain tracked separately.

## Final delivery review fix validation

The final review fix wave closes C1, T1, T2, T3, B1, B2, D1 and D2.
Source-backed upserts reject instruction-backed owners before source work, and
add rejects owned bundle alias collisions. The captured instruction adapter
remains the update path for instruction contributions; recorded removal remains
source-free. No production activation, dependencies, native-discovery claims or
stage-file cleanup are introduced. Canonical topic and command documentation now
describe the implemented opt-in compiler/lifecycle connection and target
precedence. Historical archived plans and the umbrella pair remain unchanged.

The orchestration integration suite has sixteen examples, plus the private
identity example. Focused regression runs passed. Three independent negative
controls failed the strengthened tests: reversing source specs fails each
alias's source assertion; omitting the alias from identity fails the same-source
two-alias lifecycle; ignoring add target overrides fails the actual add
precedence witness. B1 and B2 each had a failing regression before the guards
were added. Logs are `/private/tmp/upskill-pr318-fixes-{b1,b2}-red.log`,
`/private/tmp/upskill-pr318-fixes-{t1,t2,t3}-negative.log`,
`/private/tmp/upskill-pr318-fixes-orchestration.log` and
`/private/tmp/upskill-pr318-fixes-unit.log`.

The expanded process matrix passed on macOS in 99.29 seconds: 270 discovered
checkpoint interruptions and two additional late-edit reconciliation cases, 272
killed processes in total. This retains all 162 source-backed first-transition
rule/resource checkpoints and adds 108 captured instruction checkpoints across
fresh and migrated add/update/remove. Each instruction checkpoint preserves the
manual prefix, retained contribution/owner/reference, ordinary mode and a manual
neighbor edited after termination. Counts come from the emitted event vectors,
not assumed action counts. The log is
`/private/tmp/upskill-pr318-fixes-process.log`. Unix sockets require an
escalated run in this environment; the sandbox attempt stopped at socket
creation.

The complete wave passed `just fmt`, escalated `just verify`, `just sanity`
(including all-target Clippy with `-D warnings`) and explicit Rust 1.85
`just msrv-check`, using
`CARGO_TARGET_DIR=/private/tmp/upskill-gate5-acceptance-target` and an empty
`RUSTC_WRAPPER`. Logs are
`/private/tmp/upskill-pr318-fixes-{fmt,verify,sanity,msrv}.log`. Full
verification includes the expanded process matrix and the actual production
schema-1 CLI boundary. Hosted CI, final full review and merge remain separate
delivery gates. Stage residue #316 remains explicitly accounted by regular-file
bytes, mode and parent; no cleanup or native qualification claim follows from
these results.
