# File recovery checkpoint model

This design describes the implemented inactive pure library API.

`operations::recovery::plan_recovery` consumes a borrowed slice of supplied
`RecoveryAction` values and returns `anyhow::Result<RecoveryDecision>`. Each
action has an `ActionKind` (`File` or `State`), `Checkpoint` (`Queued`,
`Pending`, `Done`) and `ObservedRelation` (`Before`, `After`, `Both`,
`Neither`). All public model types derive `Debug`, `Clone`, `Copy`, `PartialEq`
and `Eq`.

## Supplied classifications

File represents one complete planned file edit, including an instruction
aggregate. State represents the final ownership-state checkpoint without
choosing its schema or path. Exactly one State action must be final; state-only
operations are valid. Checkpoints must follow `Done* Pending? Queued*`. The
complete sequence is structurally validated before any observation decision;
invalid structure returns a contextual error with the position or required State
condition. Diagnostic ordering is unspecified when multiple structural
conditions are invalid.

Observations compare supplied current values to exact expected before and after
values. Before and After match only their respective values; Both matches both
and represents an unchanged action; Neither matches neither. These
classifications are not qualified filesystem evidence. Native operations are not
representable.

Every Done action must match After or Both. Pending may match Before, After or
Both. Queued must match Before or Both: a coincidental unstarted After
observation does not establish application. All observations are checked,
including the suffix after the next proposed step. The first conflict in
operation order returns
`Reconcile { index, recorded_prefix, verified_post_prefix }`.

## Prefixes and next decisions

`recorded_prefix` counts leading Done checkpoints even if current observations
contradict them. `verified_post_prefix` counts only the consecutive leading Done
post-state observations, extended by one Pending action at After or Both. It
stops at the first mismatch; Queued never extends it, including Both.

After full validation, `Complete { completed }` reports the number of actions
when all are Done, including State. Otherwise `Ready` returns both prefixes and
one `RecoveryStep` at the first unfinished index:

- Pending Before or Queued Before proposes `Apply { index }`.
- Pending After or Both proposes `Acknowledge { index }`, avoiding repetition.
- Queued Both proposes `CheckpointOnly { index }`, with no physical write.

For a state-only operation supplied as Pending and After, the decision is Ready
with recorded prefix 0, verified post-state prefix 1 and Acknowledge at index 0.
The pending final state write is recognized without proposing it again. See the
[public model and planner](https://github.com/driftsys/upskill/blob/main/src/operations/recovery.rs)
and the
[literal acceptance cases](https://github.com/driftsys/upskill/blob/main/tests/recovery_plans.rs).

Apply is a proposed logical step, not permission to write. The function changes
no checkpoint and skips no action. Before another application decision, a future
caller must obtain fresh qualified facts and advance its durable checkpoint.

## Evidence and remaining contracts

[`tests/recovery_plans.rs`](https://github.com/driftsys/upskill/blob/main/tests/recovery_plans.rs)
pins all twelve checkpoint/relation combinations independently for both roles,
whole-suffix validation, first-conflict selection, distinct prefixes and final
State handling. Independent compiled semantic controls establish that these
assertions detect omitted guards and altered decisions.

This API performs no I/O, serialization, journal reading, byte comparison,
locking or effects. Qualified observers and journal producers must establish
actual file identities, containment, exact values, source availability and
source/state bindings. Credential-free reconstruction, durable receipt format,
checkpoint persistence, owner/group, ACLs, extended metadata and physical
application remain unresolved subsequent contracts. The implemented
[permission-selection policy](file-permission-policy.md) is a separate
prerequisite; physical permission application and lifecycle integration remain
pending. No actual recovery or M1 completion is claimed; M1-A2, M1-A3 and M1
remain incomplete. See
[AD-0032](../adr/0032-model-recovery-checkpoints-before-effects.md).
