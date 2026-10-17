# Gate 5 reference reconciliation validation

Validation record date: 2026-10-16. This date identifies the original evidence
record; consolidation does not rerun or extend its qualification.

This informative note records local evidence for the inactive authoritative
reference graph boundary, the first bounded part of task 2b. Requirements remain
in
[specification §4.15](../topics/0005-lifecycle-planning.md#415-authoritative-lifecycle-reference-reconciliation)
and implemented interfaces in the
[reference reconciliation design](../topics/0005-lifecycle-planning.md#architecture-lifecycle-reference-reconciliation).

## Local evidence

Initial implementation spans `b01a67d..dd53bd4`. The focused
[acceptance suite](../../tests/lifecycle_references.rs) passed nine tests before
final review. Review found that an explicitly added edge from a detached bundle
could be silently pruned. A failing example preceded the survival check; both
membership and dependency edges are now rejected when they would disappear.
Further examples independently exercise reachable duplicate installation and
bundle upserts and installation roots with same-named bundles. The focused suite
passed 13 tests after those corrections and an additional example for prior
installation and removed-ID ordering and successful membership/dependency edge
ordering.

Seven initial assertion-failing controls exercised retention traversal,
installation binding, raw root immutability, complete candidate validation,
unreachable upsert rejection, duplicate edge edits and prior state validation.
Sources were restored before green verification. Missing-interface failure
preceded implementation.

Five later controls independently removed the installation duplicate guard,
bundle duplicate guard and added-edge survival check, and made an installation
root retain the same-named bundle. A mixed-batch control checked only the first
added edge; both addition orders now reject discarded work. Each failed an
intended acceptance assertion with exit code 101. Exact source bytes were
restored after those controls.

`just fmt`, full `just verify` and `cargo +1.85.0 check --all-targets --locked`
passed for the initial delivery. Full `just verify` passed again after final
review corrections. The existing negative-fetch CLI fixture was changed to a
missing local file-protocol remote after its external GitHub clone blocked
verification. Its diagnostic assertion failed before the fixture correction, and
the complete CLI UX suite passed afterward. Production fetch behavior did not
change.

Ignored implementation-worktree evidence lives under
`.superpowers/sdd/2026-10-16-m1-gate5-reference-reconciliation/`: `progress.md`,
`delivery-receipts.json`, `controls.json`, the individual control logs,
`review-controls.json`, `review-control-*.log`, `verify-clean.log`,
`verify-review.log`, `msrv-clean.log` and the focused suite logs. Earlier
interrupted or interleaved verification logs are diagnostic only. These scratch
files are not published artifacts.

## Limits and remaining work

This delivery validates recorded graph identity and retention. Filesystem alias
equivalence and source authenticity still require later qualification. Shared
artifact ownership reconciliation, complete lifecycle planning, persistence,
backup, application, recovery and observational adapters remain separate work.
The graph result grants no deletion authority and contains neither artifacts nor
tagged legacy data. Production commands remain on schema 1.

Local checks establish no hosted platform qualification, PR merge, native
activation, task 2b completion, Gate 5/6 completion or M1 completion. Delivery
head review and hosted checks remain separate gates. Final verification after
gardening and the ordering example remains a delivery gate.

## Deterministic plugin test isolation

A later full verification run exposed inherited library plugin tests invoking
installed Claude/Copilot CLIs and network subprocesses. The owned test process
and its descendants were stopped; that interrupted run is diagnostic only. Five
plugin/doctor tests now execute in isolated current-executable child processes
with an empty PATH, temporary HOME/USERPROFILE and a 30-second bound. The
17-test pipeline lockfile suite passed in 0.16 seconds. A local stand-in CLI
control verified that the isolated test never invoked the stand-ins; removing
PATH isolation invoked them and failed the intended assertion. Production plugin
orchestration did not change. This control brings the cumulative count to 16.

The child also asserts HOME and USERPROFILE against an independently passed
parent-created temporary path. Removing either override failed that assertion
under an inherited temporary home. These two controls raise the cumulative count
to 18. The timeout is configured in the harness; normal passing fixtures do not
exercise its expiration.

Final numbered review required proof that the intended child test completed. The
harness now checks its successful test-result line; selecting a nonexistent
filter failed the parent assertion. A distinct new installed alias also exposed
alias normalization in a separate control. Both assertions failed with exit101,
and sources were restored. The cumulative control count is20. These final review
fixes receive verification without a third numbered review cycle.
