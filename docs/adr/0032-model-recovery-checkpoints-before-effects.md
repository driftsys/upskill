# Model recovery checkpoints before file effects

AD-0032

This decision is implemented as an inactive pure library API.

## Context

Owned lifecycle recovery must distinguish recorded completion from current
observations, recognize an interrupted final state write and preserve manual
edits. Neither supplied values nor the existing pure permission policy grant
file-effect authority.

## Decision

Expose `operations::recovery::plan_recovery(&[RecoveryAction])` as a separate
pure ordered checkpoint model before durable journals or file effects. Require
one final State action and a completed prefix, followed by at most one Pending
action and queued work. Validate all supplied observation relations before
returning a next step. A pending post-state is acknowledged, queued post-state
requires reconciliation, and an unchanged queued action performs no physical
write. Report recorded and verified post-state prefixes separately.

## Alternatives and cost

Embedding these rules into effectful code couples logical defects to file
mutation. Inferring completion from desired values alone adopts unstarted
changes and can repeat the final state write. The selected model adds an
interface and requires future qualified producers and durable checkpoint
protocols. It does not select journal serialization, credential storage, native
metadata preservation or lifecycle activation.

Related:
[recovery model](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#architecture-file-recovery-model),
[permission policy](0031-select-explicit-file-permissions.md).

## Implemented boundary

`RecoveryDecision` returns Ready with one Apply, Acknowledge or CheckpointOnly
step, Complete including the final State, or Reconcile at the first observation
conflict. Structural errors return contextual errors before observation
decisions. The recorded prefix counts leading Done checkpoints even when
contradicted; the verified post-state prefix stops at a mismatch and may extend
through one Pending post-state action. Queued Both never extends it. Complete
observation validation prevents a later conflict from being hidden by an earlier
next step.

Apply conveys no write authority. Producers must qualify physical identities,
exact values, containment, source/state bindings and source availability. The
opt-in executor in [AD-0033](0033-qualified-file-execution-and-journals.md)
supplies desired-payload reconstruction, durable receipts, checkpoint
persistence and physical effects within a restricted metadata envelope. No
lifecycle caller or schema-2 writes are activated; hosted executor
qualification, M1-A2, M1-A3 and M1 remain incomplete. Literal acceptance cases
and independent compiled semantic controls pin both action roles and ordering
boundaries.

Satisfies:
[specification §4.11](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#411-ordered-recovery-checkpoint-model).
