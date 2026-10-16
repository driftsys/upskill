# Gate 5 reference removal validation

This informative note records the inactive shared-owner removal planner. The
[specification](../specification/specification.md#416-reference-driven-shared-owner-removal-planning)
and [design](../design/lifecycle-reference-removal.md) define its boundary.

## Acceptance and controls

The public `lifecycle_retirement` suite initially passed 11 focused tests. Quick
review expanded it to 15, including complete destinations with distinct values,
untouched topology, scope/raw-root ordering, association order and isolation for
equal region IDs. Examples cover shared files and regions, final-owner
retirement, retained empty aggregates, manual edits, observed aggregate mode,
recorded whole-file mode, overlapping per-owner consumer sets, bundle-path
retention, empty installations, tagged legacy duplicates, unsupported kinds,
topology conflicts and incomplete or edited evidence. A later file conflict
returns no partial plan.

Missing-interface failure preceded implementation. A separate unsupported-kind
example failed before its Rule-only retirement guard was added. Six
assertion-failing controls independently disabled whole-file hash verification,
recorded mode verification, shared-owner observation coverage, legacy
preservation, unsupported-kind rejection and per-owner union computation.
Sources were restored and SHA-256 values recorded after the experiments. Seven
later assertion controls checked untouched topology, action destinations, legacy
order, agent-kind rejection, scope identity, artifact order and region location
isolation. The first legacy-order attempt survived because its fixture was
symmetrical; that attempt is excluded. Distinguishable rows in a non-symmetrical
order made the corrected control fail as intended. The cumulative passing
assertion-control count was 13. Three final controls cross-wired before values,
reordered retained associations and reordered actions by relative path. Each
failed an assertion; the cumulative count is 16.

`just fmt`, full `just verify` and locked Rust 1.85 all-target checking passed
after the 11 acceptance examples and six controls. These are local checks.

Implementation evidence is kept in the ignored plan workspace
`.superpowers/sdd/2026-10-16-m1-gate5-retirement/`: acceptance logs,
`controls.json`, `review-controls.json`, `final-controls.json`, individual
control logs, command-exit receipts and `final-implementation-review.md`. The
excluded symmetrical attempt is marked as a reconstructed diagnostic in
`review-controls-initial-diagnostic.json`; its original stdout was overwritten.
Incomplete runs are not completion evidence. Independent implementation review
of `e3f0146..b5c9537` found no production defects. The implementation was
rebased as `911915d`; production and acceptance bytes were preserved from
`69cf8f7` to `eeaa8b8`, with hashes in `rebase-preservation.json`. Its misplaced
specification section was corrected; quick review strengthened tests and
corrected scoped documentation. The later focused run passed all 15 tests after
the 16 valid controls and exact production-source restoration. Final formatting,
full verification and MSRV checking after gardening remain delivery gates, as do
hosted checks and PR review. No merge or native target qualification follows
from this local evidence.

## Limits

This removal-only API establishes no new sharing contract. Add/update planning,
selection diagnostics, native compatibility and source qualification remain
separate work. Supplied observations do not establish qualified destinations.
Guarded first-write backup, execution, recovery and observational adapters
remain later Gate 5 deliveries. Production commands still use schema 1; this
slice neither completes Gate 5 nor activates M1.

## Numbered PR review corrections

Pass1 of PR298 found three regression-test gaps; production passed the new
probes. The expanded 18-test suite now exercises refreshed separator metadata
and sequential retirement from manual text without a final newline, consumer
isolation between retained regions at one location, and mode mismatch rejection
on final-owner deletion. It also covers invalid UTF-8 outside a region,
unsupported owned kinds with populated artifacts, and absent aggregate modes.
Six independent controls disabled those checks and each failed an intended
assertion. Source bytes were restored; `pr-pass1-controls.json` records hashes.
The cumulative assertion-control count is23. Related live documentation now
distinguishes implemented removal planning from remaining add/update work.
