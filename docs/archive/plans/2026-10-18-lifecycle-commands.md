# Captured lifecycle command adapters

Status: implementation in isolated `feat/lifecycle-commands` worktree.

## Accepted boundaries

- Recovery under the supplied matching guard precedes canonical state capture.
  Any recovery report stops the request, including completed recovery.
- Sessions retain opaque state/backup evidence. Prepared operations borrow their
  session and exact source snapshots and privately retain output evidence.
- Complete selection preflight precedes desired-set construction. Supplied
  complete sets are bound to captured manifests; plain values grant no source
  authenticity without their retained snapshot.
- Apply revalidates source, state, backup and destinations, then uses the
  existing bound schema-2 writer and its journal. Production dispatch remains
  inactive.
- Removal derives only selected explicit direct roots. Bundle expansion must not
  remove an independent direct installation or dependency reference.
- File permission selection updates desired content and matching owner contracts
  together; generated instructions use exact before modes. Resources default to
  disabled owner execution.

## Implementation and verification

1. Add failing lifecycle acceptance tests for guarded fresh and migrated writes,
   updates/removal, selection rejection, stale evidence and writer contention.
2. Implement session, captured requests, preparation and guarded application.
3. Extend examples for preserved manual instructions, extra resources and graph
   roots. Test reserved paths and source substitution.
4. Run focused acceptance, format, repository verify and explicit Rust 1.85.
5. Record remaining source-resolution and whole-lane recovery acceptance limits
   in a dedicated technote. No commit, push or PR belongs to this worker.

## Delivered library slice

Implemented `lifecycle::commands` with private captured sessions/prepared
writes, complete selection preflight, source snapshot association,
permission/contract selection and explicit direct-root retirement. Ruling
**R-G5-CAPTURED-1** permits caller-authorized typed instruction desired values
without claiming compiler origin. See the
[adapter note](../../technotes/m1-captured-lifecycle-commands.md).

Acceptance evidence: 11 integration examples plus a recovery-stop unit example;
`just fmt`, `just verify`, all-target Clippy with denied warnings and explicit
Rust 1.85 `just msrv-check` pass. Full verification requires escalation because
existing FIFO and process recovery examples are denied by the local sandbox.
Logs are retained under `/private/tmp/upskill-lifecycle-commands-*`.

No production activation, source fetching, commit, push or PR was performed.
Source orchestration, whole-lane interruption acceptance, hosted CI and
independent review remain the parent completion lane's responsibility.
