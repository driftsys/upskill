# Guarded original-state backup

Date: 2026-10-17. Bounded executor prerequisite derived from the accepted
[Gate 5 lifecycle scope](2026-10-15-m1-gate5-lifecycle-integration.md),
acceptance criteria 2, 7 and 8. The umbrella remains active after this slice.

## Contract

- Add an opt-in library method to preserve the exact observed original State
  bytes before any output effect. Require a matching writer guard and all
  source/destination preconditions. Keep `ExecutionRequest` source
  compatibility.
- The backup destination must be absent or contain the identical original bytes
  in private mode 0o600. Existing matching backups retain identity and receive
  file and parent sync before their receipt completes. Mismatches reject before
  journal creation or output effects.
- Publish a new backup atomically without replacing any existing destination,
  including a file appearing after the final observation. Use existing rustix
  support; unsupported host/filesystem operations reject.
- Reuse the executor's Pending/Done ordering, payloads, final State and cleanup.
  Persist create-only policy as required first-action metadata in backup journal
  schema 2; retain ordinary schema-1 decoding. Bind the backup's original digest
  and length to final State before evidence. Reject missing/moved markers,
  invalid private modes and mismatched evidence. Older binaries reject schema 2.
- Recovery verifies the backup with all actions before progress. An interrupted
  matching Pending publication is synced and acknowledged without replacement.
  An edited completed backup requires reconciliation before later effects.
- Include the backup in report prefixes at index zero. Retain the backup after
  successful journal cleanup. Enumerate file actions; never remove directories.

## Limits

The generic executor accepts caller-authorized bytes. It does not validate JSON,
infer ownership, prepare lifecycle transitions, fetch sources, or activate
schema-2 production commands. Lifecycle application and observational adapters
remain later Gate 5 work. Native qualification remains Gate 6. Existing advisory
locking and external namespace-race limits apply; process termination checks do
not establish hardware power-loss durability.

## Evidence

Use real filesystem acceptance examples for exact bytes, modes, retained
identity, mismatches, topology and stale preconditions. Inject a late arrival
immediately before publication and verify it survives. Corrupt journal markers
and evidence; interrupt and edit a completed backup. Terminate real child
processes at output, State and backup checkpoints and verify forward recovery.
Check that removing exclusive publication, required policy validation, or
existing-backup sync causes a targeted assertion failure; restore exact source
bytes. Run full tests, formatting, verification, Rust 1.85 and independent
review.
