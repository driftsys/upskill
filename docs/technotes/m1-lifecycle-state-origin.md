# Lifecycle state origin validation

This bounded inactive prerequisite adds durable initial-state evidence and a
bound applying entrypoint. Production commands remain on schema 1.

Public acceptance examples in `tests/lifecycle_state_origin.rs` cover strict
origin decoding/validation, a known SHA-256 result for original bytes, optional
unbound inspection, planner preservation, fresh initialization/private state/no
invented backup, and reopened fresh writes. Migration creates the exact private
backup; reopened migrated writes succeed with original evidence and reject
same-length changed unknown JSON bytes or non-private modes. Unknown origin,
changed origin evidence and a pre-existing fresh backup block before outputs.

The initial acceptance compile failed because the new APIs and origin field were
absent. A strict-field regression then exposed internally tagged unit variant
handling; using an empty struct variant made the unknown-field test pass. Six
public origin examples plus existing application, state and preview regressions
passed. The fresh interruption unit example covers successful checkpointed
recovery and a late backup arrival that preserves the unexpected bytes, leaves
outputs/state absent and retains the pending journal for reconciliation.
Migrated reopened-write examples explicitly reject changed and missing origin
without output effects. Independent review found no runtime defect; these
additional assertions address its acceptance-coverage requests.

Validation on 2026-10-18 applies to the local origin lane based on
`aeb92256fec4a3939b25984ef35bc69b67a0e949`. Final `just verify` passed after
gardening, including the complete test suite, Clippy, Rust/Markdown formatting,
documentation checks and build. `just msrv-check` passed with Rust 1.85.0 and
locked all-targets checking. Hosted CI, final PR review and merge remain
delivery gates; this record claims no hosted or merged result.

These checks do not authenticate state against a party editing state and backup
together, establish hardware power-loss durability, finish source resolution or
complete Gate 5/Gate 6. Original unknown schema-1 fields stay in the exact
backup, not the typed projection. See AD-0038 for the interface and
compatibility limits.
