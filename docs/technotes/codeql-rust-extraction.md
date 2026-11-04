# Rust CodeQL extraction coverage

Issue: [#344](https://github.com/driftsys/upskill/issues/344). Evidence captured
on 2026-11-04. This note concerns static analysis extraction; it does not
qualify native customization delivery or every platform branch.

## Baseline

The default-setup
[Rust job 125192822607](https://github.com/driftsys/upskill/actions/runs/41419948210/job/125192822607)
analyzed main commit `1a291a7de62481b14360ddcc09cf19cc93ccbdc2` on 2026-11-02.
Its
[retained baseline](evidence/2026-11-04-codeql-rust-extraction/baseline.json)
records CodeQL 2.27.3, `codeql/rust-queries` 0.1.45, `codeql/rust-all` 0.2.24
and the complete pack revision. All 193 Rust files were scanned: 188 were
extracted without error and five with errors. The SARIF analysis (26 listed
rules, including metrics) 2075675383 was ingested with zero security findings. A
green job and zero findings did not establish clean extraction.

Seven warnings identify exactly those five files at the baseline revision:

| File                               | Line   | Failing macro               | Inactive condition on the Linux runner |
| ---------------------------------- | ------ | --------------------------- | -------------------------------------- |
| `src/filesystem/destinations.rs`   | 98     | `$crate::__anyhow`          | host is neither Linux nor macOS        |
| `src/filesystem/scope.rs`          | 73, 89 | `$crate::__anyhow`          | host is neither Linux nor macOS        |
| `src/filesystem/sources.rs`        | 51, 84 | `$crate::__anyhow`          | host is neither Linux nor macOS        |
| `tests/filesystem_destinations.rs` | 765    | `$crate::format_args`       | host is macOS                          |
| `tests/source_manifests.rs`        | 137    | `$crate::panic::panic_2021` | host is not Unix                       |

The job uses build mode `none`, the runner default stable Linux toolchain, and
no explicit sysroot or configuration overrides. The log does not record the
exact rustc version. No repository CodeQL workflow or custom CodeQL
configuration participates in default setup.

## Cause and bounded workaround

These are conditionally disabled macro expressions in otherwise compiled files.
This differs from a dependency resolution failure or an unsupported macro on an
active path. The upstream discussion in
[github/codeql#19966](https://github.com/github/codeql/issues/19966) explains
that disabled-code macro expansion can generate diagnostics;
[github/codeql#19982](https://github.com/github/codeql/issues/19982) also
includes other causes, so its standard-library workaround is not assumed to
apply here.

The
[extractor source at the analyzed pack revision](https://github.com/github/codeql/blob/a1575b98a8cc19bcfa7bb6cb41be688cb3dae65c/rust/extractor/src/translate/generated.rs)
checks `should_be_excluded` before translating a block expression. Its
macro-expression translation does not perform that check before visiting the
macro call. A direct expression attribute can therefore leave a nested macro
expansion attempt without valid semantics. Moving the same condition onto an
enclosing block gives the extractor a node it can discard before visiting that
macro. This source reading is the hypothesis tested by the hosted before/after
metrics below.

The baseline also contains matching block-wrapped unsupported-host errors in
`QualifiedScope::open` and `QualifiedScope::observe_destinations` without
corresponding warnings. These are existing same-run controls, not a separate
minimal-extractor execution.

The change wraps only the seven affected macro statements in blocks. It
preserves every condition, error message, assertion operand and semicolon inside
the conditional branch. It does not exclude files, suppress warnings, pin a
different sysroot, change dependencies or change GitHub security settings.

## Verification

[Local verification receipts](evidence/2026-11-04-codeql-rust-extraction/local-verification.json)
retain the failed baseline acceptance check, commands, observed exit codes,
source revisions, platform, log hashes and result summaries. The baseline Rust
suite, the four focused integration targets (78 parent-process tests plus
subprocess controls), `just fmt`, full `just verify` and Rust 1.85 MSRV checks
passed. The
[independent candidate review](evidence/2026-11-04-codeql-rust-extraction/candidate-review.md)
found no issues; full PR review pass 1 also kept no findings.

| Case                     | Commit    | CodeQL | Errored files | Clean files | Macro warnings |
| ------------------------ | --------- | ------ | ------------- | ----------- | -------------- |
| Historical main baseline | `1a291a7` | 2.27.3 | 5             | 188         | 7              |
| Wrapped candidate        | `f6fdb61` | 2.27.4 | 0             | 193         | 0              |
| Original-source control  | `eacff60` | 2.27.4 | 5             | 188         | 7              |

The
[candidate receipt](evidence/2026-11-04-codeql-rust-extraction/candidate.json)
binds
[Rust job 126864425559](https://github.com/driftsys/upskill/actions/runs/41912780563/job/126864425559)
and ingested analysis 2098365348 to exact commit
`f6fdb611b8ba04b1354f1558073309dfe483fe05`. It records zero macro-expansion
warnings and 193 files extracted without error. Linux, macOS and Windows tests,
quality, documentation and MSRV checks passed at that candidate commit.

GitHub selected CodeQL 2.27.4 and updated query packs for the candidate, so the
historical comparison alone cannot isolate the source change. The
[same-bundle control](evidence/2026-11-04-codeql-rust-extraction/same-bundle-control.json)
restored the original five Rust file blobs in a temporary commit. Its
[Rust job 126868956541](https://github.com/driftsys/upskill/actions/runs/41914065659/job/126868956541)
and analysis 2098431849 reproduce all seven exact warning paths, lines, columns
and messages with the same CodeQL 2.27.4/query-pack versions as the wrapped
candidate. This control rejects the tool update alone as the cause of the clean
result. All 193 tracked Rust paths remain unchanged. Both runs ingest SARIF
successfully with zero security findings despite different extraction quality.

Default setup does not emit its separate file-coverage baseline on PR runs. The
PR evidence therefore uses the extraction metrics and unchanged tracked
inventory; it does not claim a PR file-coverage baseline was emitted. No local
minimal-extractor run was performed. GitHub refused both attempts to rerun the
historical default-setup job; the temporary source control supplied the matched
comparison instead. A later comment-only control marker was unnecessary and is
removed with the original-source control before delivery.

The implementation and
[plan reconciliation](../archive/plans/2026-11-04-codeql-extraction-coverage.md)
are complete. The five restored Rust file blobs match the verified `f6fdb61`
candidate exactly. The final delivery commit and its exact-head hosted checks
are tracked in [PR #462](https://github.com/driftsys/upskill/pull/462). This
note records the controlled extraction experiment and does not assert a merge.

## Limits

The clean Linux extraction metric shows that the tracked Rust inventory was
processed without the reported diagnostics on that runner. It does not establish
analysis of all macOS or Windows-only branches, complete semantic modeling,
absence of vulnerabilities, or any native delivery qualification. Default setup
remains responsible for future tool updates; a later extractor or toolchain
change requires a fresh check of metrics and diagnostics.
