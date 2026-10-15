# M1 Gate 5 combined-state validation

Date: 2026-10-15. Scope: the first Gate 5 slice, an opt-in pure library. This is
not evidence that Gate 5 or M1 is complete or activated.

## Delivered boundary

The combined-state model validates installation identity, direct and bundle
reference reachability, cycles, exact artifact owner/root binding and explicit
per-owner consumers. The codec validates schema 2 and preserves the typed
schema-1 snapshot without adopting outputs or normalizing legacy arrays.
Existing production command dispatch and `lockfile::CURRENT_SCHEMA` stay on
schema 1. No migration backup, application or recovery is invoked by this slice.

## Local evidence

- Acceptance tests: 33 cases in `tests/lifecycle_state.rs` passed.
- Graph controls: omitting ID syntax, duplicate installation ID, endpoint,
  duplicate edge and delegated artifact-shape checks made their targeted tests
  fail.
- Association controls: omitting requested-surface containment, nonempty owner
  consumers, authoritative root scope, surplus rejection, and swapping
  contribution selectors made their targeted tests fail.
- Codec controls: reversing legacy arrays, dropping scope/root/selector sorting
  keys, and emitting compact JSON made their targeted tests fail.
- All 22 controls failed through test assertions, not compilation errors. Each
  temporary production change was restored byte-for-byte before the green run.
- `just fmt`, `just verify` and `cargo +1.85.0 check --all-targets --locked`
  passed for the implementation. Final slice delivery repeats verification after
  the test-only strengthening and documentation gardening.

Ignored scratch logs and control scripts are under
`.superpowers/sdd/2026-10-15-m1-gate5-combined-state/` in the implementation
worktree. This record describes the observed results; those local logs are not
published artifacts.

## Review evidence

Quick advisory docs/tests reviews ran separately for each of the three task
commits. Their test gaps were classified Major/S or Minor/S and fixed in the
same slice. Documentation wording and book-link fixes were Minor/S.

The final independent review used `gpt-6-astra` with `xhigh` reasoning on
`a5e28b816a3f4a2db435753564a7cd1204f9e93c..490b8e29a27040dac8172dfc5b4f582991029f56`.
It reported no Critical, Important or Minor findings. That reviewer
independently ran 28 lifecycle-state, 29 ownership-record and 16
legacy-projection tests. The later two codec acceptance cases and stronger
fixtures were quick-review fixes, verified by the targeted controls and the
final repository checks; they were outside that independent review target. No
additional review of fixes is claimed.

## PR review follow-up

PR #295 full pass 1 reviewed
`a5e28b816a3f4a2db435753564a7cd1204f9e93c..b3e2fa71f1a9b947d8bac4027f5145bee3889ab4`
with compliance, tests, docs and independent bug seats. Production validation
was correct; the review confirmed two Major/S test gaps: accepting a consumer
association superset, and treating source revision as part of installation
identity. Two Minor/S findings covered nonempty bundle-revision retention and
stale reference-authority status wording. All four were fixed in this slice.

Six additional controls make the consumer-superset, revision/selection-identity,
bundle-revision retention, valid ID alphabet and hexadecimal digit fixtures fail
when their intended behavior is omitted. Production sources were restored
exactly. Positive fixtures also cover uppercase/digit/underscore identifiers and
digit-containing manifest hashes. Final pass 2 and hosted checks are delivery
gates on the follow-up head; this record does not substitute for their ledgers.

## Decisions and limits

- Inline execution used one final independent review rather than per-task
  independent review, as selected for this plan.
- Association consistency lives in a focused private module while
  `validate_state` remains the single public validation boundary.
- The planned text codec accepts `&str`; future byte-loading callers must
  explicitly reject invalid UTF-8. Testing the standard library's UTF-8 decoder
  would not establish that future caller behavior.
- Typed legacy retention preserves known fields and array order. Original JSON
  bytes and unknown fields require the later exact first-write backup.
- Compiler reconciliation, selection conflicts, shared-owner mutation,
  filesystem qualification, backup, execution/recovery and observational command
  adapters remain later Gate 5 slices. Native qualification and production
  activation remain Gate 6. The deferred-review rationale is this explicit pure
  state boundary; enabling it early would invalidate the delivery contract.

Hosted CI is required on the final PR head. Local checks do not qualify Linux or
Windows execution or prove native client discovery.
