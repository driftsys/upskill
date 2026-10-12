# Mixed owned-artifact planning

Date: 2026-10-12. Status: Proposed implementation contract.
Base: `d886873` (PR #262).
Authority: [operational lifecycle design](2026-10-12-m1-owned-lifecycle-design.md)
and [M1 delivery plan](../plans/2026-10-11-instructions-rules-m1.md).

## Intent

One installation can contain instruction contributions and dedicated resources.
The existing pure planners each replace one artifact form and reject affected
records of the other form. Calling them independently with incomplete ownership
cannot establish a complete mixed installation change. Provide one pure
composition boundary that preserves manual text, untouched records and exact
supplied file evidence. It does not connect the compiler to native adapters or
activate installation effects.

The user delegated routine design decisions and the continuing delivery loop.
Success means deterministic complete records and individual physical file actions,
or an error without a successful partial plan. Filesystem qualification, reference
authority, state, execution and native support remain separate gates.

## Alternatives

1. Select a coordinator that validates the complete mixed operation, partitions
   it by artifact form, calls both existing planners and validates their union.
   This reuses the reviewed editors and evidence checks while adding the missing
   cross-form contracts.
2. Have callers independently invoke the planners and concatenate results.
   Reject: omitted forms, changing owner bindings and physical collisions would
   depend on each caller implementing the same missing checks.
3. Replace both planners with one implementation. Reject: this duplicates or
   rewrites reviewed behavior before lifecycle integration needs it.

## Public boundary

Add `operations::plan`, exported through `lib.rs`. Expose two new value types:

```rust
pub enum DesiredArtifact {
    File(DesiredFile),
    Instruction(DesiredInstructionContribution),
}

pub enum InstallationChange {
    Set {
        owner: ArtifactOwner,
        artifacts: Vec<DesiredArtifact>,
    },
    Remove {
        installation_id: String,
    },
}

pub fn plan_owned_artifacts(
    prior: &[OwnedArtifact],
    changes: &[InstallationChange],
    observations: &[ObservedFile],
) -> anyhow::Result<FileOperationPlan>;
```

The referenced DTOs come from `files::plan`, `instructions::plan` and
`ownership`. Both new types derive `Debug`, `Clone`, `PartialEq` and `Eq`, with
no serialization. Reuse `FileOperationPlan`, `PlannedFile`, `FileLocation` and
`FileValue` for the combined result; dedicated and aggregate actions both have
exact optional before/after values. The API performs no I/O or input mutation.

## Complete installations and bindings

Validate all prior ownership records before partitioning. Each installation has
exactly one explicit change; reject duplicates, unknown removals and empty Set
values. A Set is the complete desired set across both forms. Omitting a previously
owned form removes that form through its planner; it never silently retains stale
files or contributions. An empty complete set requires explicit Remove.

An existing installation retains its source, item kind, source name and installed
alias. Its source revision may change, consistently across all desired records.
Check this binding against the full prior set, including when the old and desired
forms have no overlap. Existing shared-owner restrictions remain: reject any
affected multi-owner record rather than infer missing reference associations.
Untouched shared records survive unchanged.

Partition prior rows by form only after global validation. Produce a Set for each
form with nonempty desired values. When an installation previously owned that
form but no longer desires it, produce its Remove. When it never owned the absent
form, produce no change for that form. A global Remove reaches every previously
owned form. Preserve the original prior-row order within each partition.

## Locations and evidence

Locations use `(scope, raw root OsStr, portable relative path)` equality. They
do not normalize roots or prove containment, host equivalence or realizability.
Validate supplied observation roots, relative paths and optional permission bits.
Require exactly one observation per affected physical location, including no-ops,
and reject duplicates, missing observations and extras. Affected locations include
every old row belonging to a changed installation and every desired location.

A physical location cannot participate in both instruction and dedicated-file
forms anywhere in the union of prior and desired state. Reject even an explicit
remove-and-replace conversion: contribution ownership does not prove ownership
of the surrounding aggregate or manual text. Same-form handoffs retain the
underlying planners' existing ownership rules.

Reject ancestor/descendant file-location pairs in the prior/desired union when
either endpoint is affected, across both forms. This includes removal-only
operations and file/directory transitions. Unaffected pre-existing topology is
preserved; it is not qualified as safe. Component boundaries distinguish `a/b`
from `ab`; compare scopes and raw roots exactly.

Partition observations by the validated physical form. Dedicated observations
reach the existing whole-file planner with bytes and modes intact. Instruction
observations reach the existing aggregate planner as exact optional bytes; the
coordinator retains their optional modes separately for physical actions. Invalid
UTF-8 in an affected aggregate remains a conflict.

## Combined result

Invoke each planner with its complete form partition. Preserve untouched records
and manual bytes using those existing implementations. Reassemble records in
original prior-row order, replacing retained record identities at their former
positions; remove absent identities. Append new records sorted by exact location
and selector, with whole-file records distinguished from contribution selectors.
Validate the final combined ownership records, including owner revision coherence.

Keep every affected physical action, including byte-identical no-ops. Sort actions
by exact location and reject duplicate action locations. Whole-file actions retain
the underlying exact before/after values, including optional modes. Aggregate
actions use the exact observed before value and the editor's after bytes. Preserve
the observed aggregate mode; an absent aggregate receives after mode `None`.
Retain an empty aggregate after its last contribution is removed. No deletion or
permission authority follows from these supplied values.

Any failure returns no successful combined plan. The first version may return
the first contextual conflict; collecting all operation diagnostics remains a
lifecycle activation gate. Inputs and results are owned values without serialized
execution authority, state writes, source provenance or native-success claims.

## Acceptance examples

| ID  | Required behavior                                                                                                                                        |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C01 | Create one installation with a contribution beside manual bytes and a binary resource; return exact complete records/actions and preserved modes.        |
| C02 | Update and remove mixed installations; omitted forms are removed, manual text and untouched owners survive, last-region removal keeps the aggregate.     |
| C03 | Convert an installation between forms at distinct locations with the same stable binding; allow a coherent revision change.                              |
| C04 | Reject changed stable bindings even when old and desired forms are disjoint; reject duplicate changes, unknown removal and empty complete Set.           |
| C05 | Reject same-location cross-form takeovers in both directions, including explicit removal/handoff; preserve unchanged inputs on errors.                   |
| C06 | Reject affected cross-form ancestor topology, including removal-only conflicts and old/new transitions; preserve unrelated historical pairs.             |
| C07 | Require exact observations for the physical affected union; reject missing, duplicate, extra, invalid-root/path/mode evidence.                           |
| C08 | Preserve exact aggregate bytes and modes for no-ops; reject invalid UTF-8, edited markers, missing owned files and changed file byte/mode evidence.      |
| C09 | Reject affected shared records but preserve untouched shared rows; retain same-form handoff semantics and exact scope/root distinctions.                 |
| C10 | Preserve prior interleaved row order, sort new rows/actions, produce deterministic output under change/observation permutations and never mutate inputs. |

## Delivery and remaining gates

Use an isolated worktree, acceptance-first integration tests, focused implementation,
actual semantic controls, task/final reviews, ordered formatting/verification,
hosted Linux/Windows checks and the two numbered PR reviews. Keep code, tests and
documentation together. Record durable decisions and garden only this completed
design/plan pair. Refresh current archive successor wording for debt #264 while
preserving explicitly historical closeout quotations and totals.

No new dependencies, asynchronous runtime, native commands, source fetching,
filesystem observation, lock, journal, schema-2 persistence, CLI selector or target
qualification belongs to this slice. Complete reference graphs, per-owner consumer
association, source/target adapters, immutable source evidence, filesystem safety,
permission policy and apply/recovery remain required before activation. This
prerequisite does not complete M1-A2, M1-A3 or M1.
