# Gate 5 reference reconciliation validation

This informative note records local evidence for the inactive authoritative
reference graph boundary, the first bounded part of task 2b. Requirements remain
in
[specification §4.15](../specification/specification.md#415-authoritative-lifecycle-reference-reconciliation)
and implemented interfaces in the
[reference reconciliation design](../design/lifecycle-reference-reconciliation.md).

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

Four later controls independently removed the installation duplicate guard,
bundle duplicate guard and added-edge survival check, and made an installation
root retain the same-named bundle. Each failed an intended acceptance assertion
with exit code 101. Exact source bytes were restored after those controls.

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
