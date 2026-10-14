# File recovery checkpoint model

Status: selected pure prerequisite; implementation pending.

## Intent and delivery boundary

Define the ordered recovery decisions before implementing a durable journal or
file effects. The model distinguishes recorded completion from currently
verified post-state, prevents adoption of an unstarted action's coincidental
post-state, and checks the whole supplied operation before returning a next
step. Include the final ownership-state write as an ordinary checkpointed action
with an explicit final kind.

This is a pure library API. It consumes supplied classifications; it does not
qualify those classifications, read a journal, compare file bytes, acquire a
guard, write a file or authorize effects. There is no serialization or durable
receipt format. Future qualified observers, journal producers and lifecycle
adapters must establish actual authority. The permission-selection prerequisite
is already implemented in AD-0031. M1-A2, M1-A3 and M1 remain incomplete.

## Interfaces

Add operations::recovery with these public types, all deriving Debug, Clone,
Copy, PartialEq and Eq:

```rust
pub enum ActionKind { File, State }
pub enum Checkpoint { Queued, Pending, Done }
pub enum ObservedRelation { Before, After, Both, Neither }
pub struct RecoveryAction {
    pub kind: ActionKind,
    pub checkpoint: Checkpoint,
    pub observed: ObservedRelation,
}
pub enum RecoveryStep {
    Apply { index: usize },
    Acknowledge { index: usize },
    CheckpointOnly { index: usize },
}
pub enum RecoveryDecision {
    Ready {
        recorded_prefix: usize,
        verified_post_prefix: usize,
        step: RecoveryStep,
    },
    Complete { completed: usize },
    Reconcile {
        index: usize,
        recorded_prefix: usize,
        verified_post_prefix: usize,
    },
}
pub fn plan_recovery(actions: &[RecoveryAction]) -> anyhow::Result<RecoveryDecision>;
```

File represents each complete planned file edit, including an instruction
aggregate. State represents the final state checkpoint without selecting its
schema or path. Native operations are not representable. The caller supplies the
action order from its approved complete operation; the model does not produce or
validate physical file identities or source bindings.

ObservedRelation describes current supplied evidence relative to an action's
exact expected before/after values: Before matches only before; After only
after; Both matches both; Neither matches neither. Both is distinct from two
arbitrary labels and represents an unchanged action. These labels are untrusted
supplied values, not filesystem evidence capabilities.

## Structural validation

Validate the complete sequence before classifying observations. Empty sequences
reject. Exactly one State action must exist and it must be last. Zero File
actions are allowed for a state-only operation. Checkpoints must follow
`Done* Pending? Queued*`: a completed prefix, at most one pending action
immediately after it, then queued actions. Reject Done after unfinished work,
Pending after a queued action and multiple pending actions. Errors identify the
invalid position or required final-state condition; no partial successful
decision is returned.

## Observation validation and prefixes

- Every Done action must match After or Both. Before and Neither require
  reconciliation; never repeat an action recorded as complete.
- A Pending action may match Before, After or Both. Neither requires
  reconciliation.
- Every Queued action must match Before or Both. After and Neither require
  reconciliation; an unstarted action's coincidental post-state does not
  establish application.
- Examine every supplied observation, including actions after the next step.
  Return the first conflicting index in operation order. A later conflict
  prevents Ready even when the earliest uncompleted action could proceed.
- recorded_prefix is the number of leading Done checkpoints. It reports the
  supplied journal history, even if current evidence contradicts it.
- verified_post_prefix counts the leading Done actions whose observations match
  post-state, extended by one Pending action only when that action matches After
  or Both. Stop at the first mismatch. Queued actions never extend this prefix,
  including Both.
- A conflict returns Reconcile with its index and both prefixes. Do not silently
  replace recorded history with current observations or conflate either count
  with physical execution authority.

## Next-step selection

After all structural and observation checks succeed:

- If every action is Done, return Complete with the action count. This includes
  the final State action.
- Pending Before returns Apply for that index.
- Pending After or Both returns Acknowledge for that index, never Apply.
- Queued Before returns Apply for that index.
- Queued Both returns CheckpointOnly for that index. Its before-state must still
  have been checked; do not manufacture a physical write for an unchanged
  action.

Return Ready with the selected step and both prefix counts. Never skip to a
later queued action or return more than one next step. The caller must obtain
fresh qualified facts and advance its durable checkpoint before another
application decision; this function changes no state.

## Acceptance table: state-only operation

For one State action, the required literal outcomes are:

| Checkpoint | Observation | Decision                        | Recorded / verified prefix |
| ---------- | ----------- | ------------------------------- | -------------------------- |
| Queued     | Before      | Ready Apply at index 0          | 0 / 0                      |
| Queued     | Both        | Ready CheckpointOnly at index 0 | 0 / 0                      |
| Queued     | After       | Reconcile at index 0            | 0 / 0                      |
| Queued     | Neither     | Reconcile at index 0            | 0 / 0                      |
| Pending    | Before      | Ready Apply at index 0          | 0 / 0                      |
| Pending    | After       | Ready Acknowledge at index 0    | 0 / 1                      |
| Pending    | Both        | Ready Acknowledge at index 0    | 0 / 1                      |
| Pending    | Neither     | Reconcile at index 0            | 0 / 0                      |
| Done       | After       | Complete with count 1           | completed count 1          |
| Done       | Both        | Complete with count 1           | completed count 1          |
| Done       | Before      | Reconcile at index 0            | 1 / 0                      |
| Done       | Neither     | Reconcile at index 0            | 1 / 0                      |

## Multi-action examples

1. File Done/After, File Pending/After, State Queued/Before: Ready Acknowledge
   at index 1, recorded prefix 1, verified prefix 2.
2. The same prefix followed by State Queued/Neither: Reconcile at index 2,
   recorded prefix 1, verified prefix 2. Do not acknowledge or apply before
   reporting the later conflict.
3. File Done/After, File Pending/Before, State Queued/After: Reconcile at index
   2, recorded prefix 1, verified prefix 1.
4. File Done/Before, File Done/After, State Queued/Before: Reconcile at index 0,
   recorded prefix 2, verified prefix 0. Preserve the difference between
   recorded history and current post-state.
5. File Queued/Both, State Queued/Before: Ready CheckpointOnly at index 0,
   recorded prefix 0, verified prefix 0.
6. File Done/Both, State Queued/Both: Ready CheckpointOnly at index 1, recorded
   prefix 1, verified prefix 1.
7. File Done/After, State Pending/After: Ready Acknowledge at index 1, recorded
   prefix 1, verified prefix 2; do not duplicate the final state write.
8. File Done/After, State Done/After: Complete with count 2.
9. All Queued/Before: Apply at index 0; never skip to the final State action.
10. All valid until a late Queued/Neither or Queued/After: reconcile the late
    action before any Ready result.

Invalid examples include no State, a nonfinal State, two State actions, Done
after Queued or Pending, Pending after Queued, two Pending actions and an empty
slice. Invalid structure in a suffix must return Err before any Ready or
Reconcile result.

## Verification

Acceptance tests use literal enums and counts from the table, not a second
implementation of the algorithm. Separate tests pin full-suffix validation,
recorded versus verified prefix, final-state recognition and unchanged-action
handling. Do not write tautological Copy tests.

Independent compiled semantic controls must pin: final State position; exactly
one State; completed-prefix ordering; pending positioning/uniqueness; rejection
of Done/Before; rejection of Queued/After; Pending/After acknowledgement;
Pending/Before application; Queued/Both checkpoint-only handling; pending-post
prefix extension; and rejection of a late conflicting observation before
returning Ready. Restore exact source after each control and retain named
assertion RED and focused GREEN. No compiler failure counts as a semantic
control.

## Constraints and subsequent work

- Rust edition 2024, MSRV 1.85, existing dependencies only.
- No I/O, environment or clock input, process calls, output, mutation of inputs,
  serialization, native effects or schema changes.
- No physical qualification or execution authority follows from this API.
- Preserve all 56 inherited raw/WIP hashes and prior worktrees/evidence. The
  broader application draft remains a separate untracked design.
- Actual journal contents, credential-free reconstruction, source availability,
  native metadata/identity qualification, checkpoint persistence and file
  application remain subsequent contracts. This model neither resolves nor
  conceals them.

The alternatives are to embed recovery decisions directly into effectful code,
or infer completion from desired bytes alone. The first couples logical mistakes
to data mutation; the second adopts unstarted post-state and can repeat a
pending state write. Select the separate pure model. Its cost is an additional
reviewed interface and a future qualified producer for every supplied
observation.

When several structural conditions are invalid, diagnostic ordering is
unspecified. File and State action observations must both follow the same
relation rules; tests must cover all twelve checkpoint/relation combinations for
each action role, with a valid queued State suffix for File cases.

Additional distinguishing examples: File Queued/After followed by State
Queued/Neither returns Reconcile at index 0, prefixes 0/0; the first conflict
wins. File Done/Before, File Pending/After, State Queued/Before returns
Reconcile at index 0, prefixes 1/0; later pending post-state cannot extend a
broken verified prefix. Compiled controls must independently pin both conditions
in addition to the original eleven.
