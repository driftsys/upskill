# Gate 5 reference removal validation

This informative note records the inactive shared-owner removal planner. The
[specification](../specification/specification.md#416-reference-driven-shared-owner-removal-planning)
and [design](../design/lifecycle-reference-removal.md) define its boundary.

## Acceptance and controls

The public `lifecycle_retirement` suite initially passed 11 focused tests. Quick
review expanded it to 14, including complete destinations, untouched topology
conflicts and location isolation for equal region IDs. Examples cover shared
files and regions, final-owner retirement, retained empty aggregates, manual
edits, observed aggregate mode, recorded whole-file mode, overlapping per-owner
consumer sets, bundle-path retention, empty installations, tagged legacy
duplicates, unsupported kinds, topology conflicts and incomplete or edited
evidence. A later file conflict returns no partial plan.

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
assertion-control count is 13.

`just fmt`, full `just verify` and locked Rust 1.85 all-target checking passed
after the 11 acceptance examples and six controls. These are local checks.

Implementation evidence is kept in the ignored plan workspace
`.superpowers/sdd/2026-10-16-m1-gate5-retirement/`: acceptance logs,
`controls.json`, individual control logs and command-exit receipts. Incomplete
runs are not completion evidence. Independent implementation review of
`e3f0146..b5c9537` found no production defects. Its misplaced specification
section was corrected; quick review strengthened tests and corrected scoped
documentation. The later focused run passed all 14 tests after the 13 valid
controls and exact production-source restoration. Final formatting, full
verification and MSRV checking after gardening remain delivery gates, as do
hosted checks and PR review. No merge or native target qualification follows
from this local evidence.

## Limits

This removal-only API establishes no new sharing contract. Add/update planning,
selection diagnostics, native compatibility and source qualification remain
separate work. Supplied observations do not establish qualified destinations.
Guarded first-write backup, execution, recovery and observational adapters
remain later Gate 5 deliveries. Production commands still use schema 1; this
slice neither completes Gate 5 nor activates M1.
