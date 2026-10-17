# Gate 5 captured rule desired-set validation

Validation record date: 2026-10-15. This date identifies the original evidence
record; consolidation does not rerun or extend its qualification.

This informative note records local evidence for completed task 2a: the inactive
captured-rule producer and complete batch preflight. The requirements remain in
[specification §4.14](../topics/0002-generation.md#414-captured-rule-desired-sets)
and the implemented interfaces in the
[desired-set design](../topics/0002-generation.md#architecture-captured-rule-desired-sets).

## Local evidence

Implementation and initial tests span `ff4671f..9228cc1`. The focused suite had
16 passing acceptance tests at `9228cc1`; subsequent test-only quick-review
fixes strengthened payload, topology and diagnostic assertions and brought it to
18 passing tests without production changes. Subsequent PR-review coverage adds
independent empty-extra-binding rejection, all incompatible supported
family/surface pairs and audience-excluded Codex rejection, bringing the suite
to 20 passing tests. Three additional assertion-failing controls cover those
checks; all 19 controls were restored before green runs.

Full `cargo test`, `just verify` and the locked Rust 1.85 all-target check
passed for the implementation. The restored focused suite passed 18 tests after
the last quick-review fixes. Final verification after documentation gardening
and hosted checks on the delivery head remain separate delivery gates.

Six original producer controls, five first quick-review/batch controls and five
final batch controls failed through their intended assertions. The controls
covered source binding, consumer/exclusion accounting, resource preservation,
complete batch outcomes, collision detection, payload retention and diagnostic
ordering. Production sources were restored exactly before the green runs.
Missing-interface RED preceded each producer and batch implementation.

The ignored implementation-worktree evidence is under
`.superpowers/sdd/2026-10-15-m1-gate5-rule-desired-sets/`, including
`progress.md`, `task2-suite.log`, `final-verify.log`, `final-msrv.log`,
`quick2-restored-green.log` and the control reports. This note records the
observed results; the scratch files are not published artifacts.

## Review and limits

The independent `gpt-6-astra` review with `xhigh` reasoning examined
`ff4671f..9228cc1` and reported no findings. Later test-only quick-review fixes
were outside that review target; their assertion controls and restored focused
run provide separate evidence. No review of the later fixes is implied.

This slice establishes desired values from captures, with lexical collision
preflight. Cross-request identity reconciliation, authoritative references,
shared ownership, physical qualification, permission application, backup,
execution/recovery and native discovery remain later work. Existing commands and
schema 1 remain unchanged. Local checks make no hosted platform, PR merge, Gate
5 completion or production-activation claim.
