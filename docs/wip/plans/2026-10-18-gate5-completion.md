# Gate 5 completion delivery lane

Authority: the user delegated decisions and approvals on 2026-10-18 to deliver
preview through PR/review/CI/merge, then finish add/update/remove adapters,
source-resolution orchestration and Gate 5 acceptance. Preserve the accepted
inactive-production boundary; Gate 6 activation/native qualification is outside
this lane.

Spec:
[Gate 5 umbrella](../../wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md).

## Delivery order

1. Publish and squash-merge captured preview after full review and exact-head
   hosted CI. Existing non-critical review debt remains #311/#309.
2. Record durable initial-state provenance in combined state and support an
   explicitly absent initial lockfile, without inventing a schema-1 backup.
   Reopened migrated states must verify the original backup
   digest/length/private mode rather than trusting current backup observations.
3. Provide guarded prepared add/update/remove adapters: recover before
   preparation, preflight the complete selection, retain exact
   state/destination/source evidence, map selected direct roots to explicit
   reference removals, and apply through the existing executor. Unsupported
   selected legacy work blocks before effects.
4. Resolve local/git sources through git-native authentication, select explicit
   captured rule roots, record actual resolved revisions and manifest evidence,
   compile complete desired sets/contracts and retain source snapshots until
   guarded application. Keep source resolution separate from read-only preview.
5. Run complete lifecycle acceptance across fresh/migrated scopes, manual/shared
   content, resource files, stale evidence, writer contention and process
   interruption/recovery; independently audit every umbrella acceptance
   criterion.
6. Deliver each bounded task with tests/docs, just fmt/verify/MSRV, independent
   review, PR/hosted CI and squash merge. Garden this lane and umbrella only
   after Gate 5 acceptance is actually complete. Do not close MVP/runtime
   stories or claim Gate 6 complete.

## Decisions

- R-G5-ORIGIN-1: An optional strict origin record distinguishes freshly
  initialized state from migrated schema-1 state. Existing unbound schema-2
  fixtures remain inspectable, but applying adapters reject unknown provenance
  rather than treating today's backup as canonical. Cost if wrong: an explicit
  reconciliation path may later be needed for internal unbound snapshots.
- R-G5-ORIGIN-2: Store exact original backup SHA-256/length in the authoritative
  combined state; private mode remains 0600. No independently authoritative
  sidecar is introduced.
- R-G5-FRESH-1: Fresh initialization requires both absent state and absent
  canonical backup, rechecked as executor preconditions; state mode is 0600 and
  no imaginary schema-1 backup is created.
- R-G5-RECOVERY-1: Pending executor recovery runs under the writer guard before
  source work. A reconciliation outcome stops the requested operation. The exact
  recovered/new-operation outcome contract belongs to the adapter task.
- R-G5-SCOPE-1: Existing supported rule compiler families remain the
  source-backed M1 scope. Codex instruction selection and native discovery
  remain Gate 6; supplied instruction contributions must still pass complete
  lifecycle preservation/recovery acceptance.

- R-G5-CAPTURED-1: Supplied typed instruction contributions remain supported
  caller-authorized inputs to the guarded captured adapters. Separate
  source-backed rule orchestration binds compiler outputs to retained captures.
  Cost if wrong: a stricter authoring API may be needed later; this does not
  claim that public desired values authenticate source content.

## Status

- Preview PR #312: squash-merged as `aeb92256fec4a3939b25984ef35bc69b67a0e949`;
  full pass-1 ledger
  `/Users/sebastientasson/.codex/review-ledgers/driftsys-upskill/pr312-pass1.md`;
  all hosted checks passed. T1-T3 remain deferred in #311, status prose in #309.
- Origin/fresh-state prerequisite: PR #314 squash-merged as
  `77181b90222473345dab6b46a3665828649361b9` after full review and all hosted
  checks passed. Remaining advisory origin coverage is #313; minor migration
  wording in #309 is reconciled by the captured adapter documentation.
- Source prerequisite: `feat/lifecycle-sources` contains `b0e1593` and verified
  quick-review fixes in `dbdddbe`. Distinct refs, containment rejection and
  stale compilation are pinned with mutation controls. Full verification and
  MSRV passed; PR/hosted delivery follows.
- Captured command adapters: implemented in `feat/lifecycle-commands`, stacked
  on source commit `b0e1593`. Eleven examples, recovery-stop coverage, final
  stacked verification/MSRV and independent task review passed. Bounded
  gardening is complete. Quick-review test assertions are being strengthened
  before PR delivery.
- Source-backed command orchestration: worker implementing in
  `feat/lifecycle-orchestration`, based on captured commit `38647b4`. Whole-lane
  recovery acceptance and final Gate 5 qualification remain pending.
