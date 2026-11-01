# Native coordination verification evidence

Status: offline implementation evidence, 2026-11-01. No native client or
provider was exercised and no profile is qualified by these checks.

This records the evidence behind Stage 5 in the
[delivery plan](../wip/plans/2026-10-31-four-client-skills-resources-delivery.md).
The original plan named `native_delivery_recovery` for coordinator acceptance;
implementation split the coordinator cases into `native_coordinator` and kept
the state, permission and store cases in `native_delivery_recovery`.

## Retained RED observations

The earlier local `native-coordinator-red.log` records compilation of
`tests/native_coordinator.rs` failing because `native_delivery::adapter` and
`native_delivery::coordinator` did not exist. Its diagnostic ends:

```text
error: could not compile `upskill` (test "native_coordinator") due to 2 previous errors
```

A later process-boundary test failed at `forward/after_intent/0`: observed
`recovery-required`, expected `complete`, with reason
`recovery requires every root marker`. Its result was:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 543 filtered out; finished in 37.34s
```

The subsequent process-boundary run passed:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 543 filtered out; finished in 310.94s
```

These are excerpts recovered from local logs, not a reconstructed command
transcript. The historical argument vectors and exact source snapshots were not
retained in those logs. Their SHA-256 values identify the inspected files:

- `native-coordinator-red.log`:
  `d76a8965b935f827c92c1c1e33da0c8a605605afb3925b89da3f19601adf2fb8`
- `native-coordinator-process-red.log`:
  `f5a4312946ef261f6b8f41eacbcba5dc5ed081038311d26df1fd1a3ada425609`
- `native-coordinator-process-green.log`:
  `f10560c6d2fdbac270cbdaaab8a99d72fe23a5346365783ced0bb8252d605ddf`

## Fresh GREEN verification

The full `cargo test` phase of `just verify` passed against `c0e8c67`'s
production source on 2026-11-01. This included:

- `native_coordinator`: 25 passed, including timeout-after-effect, inverse,
  retention, file-only exposure, contention and effective-binding controls.
- `native_delivery_recovery`: 28 passed, covering state, permission projection,
  qualified evidence and durable intent.
- The fresh-process sweep
  `every_forward_and_inverse_boundary_recovers_in_fresh_processes_without_duplicate_calls`.
- Existing file-execution recovery sweeps, including
  `recovers_after_every_effect_and_receipt_boundary`,
  `journal_generation_crashes_retain_published_intent` and
  `backup_recovers_after_every_effect_and_receipt_boundary`.

The enclosing `just verify` then failed only at Markdown formatting of the
private progress ledger. After formatting, `just lint docs-check assemble`
passed, as did all-target Clippy and the Rust 1.85 all-target check. The source
and test inputs did not change between the full test phase and those follow-up
checks. This does not claim that the original `just verify` command exited zero.
A final uninterrupted `just verify` remains required before opening a PR.

Full run log: `native-claude-command-plan-verify.log`, SHA-256
`e7a93deb463ee9f08f7b47f90315bcce379919658cce768ac5a84b5285b661bd`. Follow-up
log: `native-claude-command-plan-final-checks.log`, SHA-256
`f6f70e663745817bbd857ae319ce22e8b320c397e7cecce55e68705d50d5804d`.

Temporary full logs remain local; these bounded excerpts and their limitations
are retained here so coverage source is not confused with observed execution.

## Review-fix verification

After correcting the review findings, `just fmt` and an uninterrupted
`just verify` passed on 2026-11-01. All-target Clippy and the Rust 1.85
all-target check then passed. The default Cargo suite includes the new fixture
runtime tests; its ignored relative-root helper is explicitly exercised by a
parent test in an owned temporary working directory.

The corrected command tests caught three actual validation-removal mutations.
The fixture tests caught nine actual mutations spanning unsafe roots, overwrite,
missing resource/catalog publication, user scope, a paid-call allowance and
private permissions. Production source was restored before the passing checks.

- Full verification: `native-claude-review-fixes-final-verify.log`, SHA-256
  `2328a8ddd9cacedf0174d8e85d71921cc464cc3efc4a9a772a4b7e0f97ced94f`.
- All-target Clippy: `native-claude-review-fixes-final-clippy.log`, SHA-256
  `dfea6a7127fffcffab30efcb791df8bc1969c1b216f6d8493371d102a46f5011`.
- Rust 1.85: `native-claude-review-fixes-final-msrv.log`, SHA-256
  `edb38cc4c1802c8e8e513f6c4952d5886053ec43b89715cf4d6d704d7f294c22`.

This is offline verification of the inactive infrastructure. It does not change
native qualification, production admission, resource-retention or isolation
results. No native profile, model call or paid provider was used.
