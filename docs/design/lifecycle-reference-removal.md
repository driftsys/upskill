# Lifecycle reference removal planning

The opt-in `lifecycle::retire::plan_reference_removal` boundary connects
[reference reconciliation](lifecycle-reference-reconciliation.md) to complete
shared-owner retirement. It accepts validated combined state, exact reference
removals, and caller-supplied file observations. It returns complete combined
state and individual `PlannedFile` values without I/O or input mutation.

## Retention and ownership

Graph reachability decides which installations retire. Artifact counts and
consumer counts establish no graph authority. Removing one bundle path retains
an installation reached by another path. Referenced empty installations survive;
retiring empty installations produces no file action.

Only rule installations may retire through this M1 boundary. Unselected
unsupported kinds and tagged legacy rows remain unchanged. Additions, updates
and new sharing contracts are outside this API.

For each retained artifact, owner order is preserved and consumers are the union
of surviving authoritative per-owner associations, in prior consumer order. No
equality of rendered bytes is used to establish new sharing. Final combined
state validation includes graph, ownership, associations and tagged legacy.

## File evidence and effects

Each affected physical location requires exactly one present observation,
including shared-owner changes whose bytes are unchanged. Location identity uses
exact scope and raw root spelling. Duplicate, missing, extraneous, malformed and
invalid-mode observations conflict. Ancestor/descendant artifact paths involving
an affected location conflict.

Whole-file hashes and recorded modes must match the observation. A surviving
owner retains the observed file unchanged; final-owner retirement emits an
absent after-value for that individual file.

For each aggregate instruction file, every recorded region is inspected,
including untouched regions at that location. The aggregate editor runs once
with the retained bodies. It preserves manual bytes and retained body text,
updates retained marker metadata, and leaves an empty aggregate present.
Aggregate mode is preserved from valid observed evidence: individual region
records store no prior mode. Unknown markers, missing markers and edited owned
regions conflict. Manual text may have changed before observation.

These supplied values establish no qualified filesystem provenance. Application
must later use guarded, revalidated observations and the existing executor. No
production CLI dispatch, schema-2 write, backup, recovery or native target
activation is introduced. Existing mixed/instruction planners retain their
conservative multi-owner rejection.

## Components

- `src/lifecycle/retire/mod.rs`: graph-driven state and per-owner retirement.
- `src/lifecycle/retire/files.rs`: location evidence and aggregate/file actions.
- `tests/lifecycle_retirement.rs`: public acceptance examples.
