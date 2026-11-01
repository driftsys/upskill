# AD-0043 — Parse native configuration with preserved spans

Status: reviewed dependency admission for an inactive library boundary. Date:
2026-11-01.

## Context

Stage 4 of the approved skills/resources plan needs pure contributions to shared
JSONC and TOML configuration. Manual comments, unrelated keys and file bytes
must survive. Duplicate keys, ambiguous selectors, inherited masking and unowned
entries must reject. Existing ancillary JSON behavior stays unchanged. A partial
TOML parser would give uncertain ownership boundaries on legal TOML.

## Decision

Admit exactly `toml_edit =0.22.27`, with default features disabled and only
`parse` enabled, for the inactive native configuration planner. Its immutable
`ImDocument` parser validates full TOML and supplies original key/value spans.
Never serialize an entire manual document. Encode only a changed value or new
assignment, splice validated spans, and reparse the complete result. Unsupported
selected value shapes must fail explicitly; the parser must still accept legal
unrelated TOML, including dates, multiline strings and arrays of tables.

Keep the existing `serde_json` features. For JSONC, mask comments and valid
trailing separators without changing byte offsets, then validate JSON with a
recursive duplicate-rejecting visitor. Recover token spans from that validated
document; do not replace JSON validation with a permissive scanner. Reject
unterminated comments and decoded duplicate names, including escaped spellings.
Reparse every planned result. JSONC is a separate opt-in boundary, not a change
to legacy strict JSON inputs.

Do not enable `serde_json/raw_value`. A compatibility probe using the legal
object `{"$serde_json::private::RawValue":"0"}` retained that object with
current features but returned the number `0` with `raw_value` enabled. Cargo
feature unification would expose that semantic change to existing consumers.

The planner consumes captured layers, file identity, exact selector ownership
and expected prestate. It returns complete before/after file preconditions and
updated in-memory ownership evidence. It performs no file writes and supplies no
production state codec. An identical manual value is not ownership evidence.
Owned array elements must be separate contributions; never claim a complete
manual array to register one source. Reject overlapping selectors. Independent
array entries in the same array may coexist. Scalar selectors reject occurrences
in any inherited layer. Higher arrays reject conservatively because an
unqualified profile may replace rather than append them; lower arrays may
contain distinct manual entries. Concrete native layer semantics remain an
adapter qualification obligation.

The in-memory ownership view binds the file location, surface and format and
contains per-entry selector, semantic value and raw-token digest. Updates and
removals require an exact expected ownership record. A shared rollback baseline
restores the original bytes or absence after the last removal only while the
complete expected file bytes and mode still match. New documents select private
mode `0600`; existing modes are preserved. Unrelated manual edits invalidate
that shortcut; minimal token removals preserve the remaining bytes, comments and
any empty parent containers. Do not delete those containers by guessing that
they are still exclusively managed.

TOML scalar, array and inline-table value tokens can be owned. Whole explicit
tables and implicit inline-table selectors without original token spans are
unsupported edit targets and must return errors. Legal unrelated syntax is still
accepted. Missing paths use dotted assignments at the nearest explicit table
anchor and the complete result must parse.

Capture native layer ordering explicitly and reject contributions masked by a
higher layer or duplicated across effective array sources. Project OpenCode
configuration must use an ignored, untracked local overlay. Reject a write root
outside the selected installation root, including an external XDG root. Layer
capture and lexical root checks do not establish filesystem qualification;
application must use qualified identities and complete before-state checks.
Native profile qualification remains a separate gate.

## Dependency evidence

The cached upstream package declares edition 2021 and Rust 1.66. A private
runtime-selected parser probe links both parsers using the CLI release profile
(`z`, LTO, one codegen unit, strip, panic abort). Measurements on macOS arm64
with Rust 1.97.1 are:

| Build                           |     Bytes |
| ------------------------------- | --------: |
| Existing Stage 3 CLI            | 3,215,120 |
| JSON parser probe               |   302,784 |
| Same probe with parse-only TOML |   402,672 |
| Incremental linked probe cost   |    99,888 |

The two-parser probe passes `cargo +1.85.0 check --offline --locked`. These are
dependency admission measurements, not a prediction of final CLI size. LTO can
discard an uncalled library boundary, so a zero CLI difference alone would hide
the linked parser cost. After adoption, the workspace `just msrv-check` passes
on Rust 1.85.0 with `--locked --all-targets`. The existing resolution is
preserved; the added packages are `toml_edit 0.22.27`, `toml_datetime 0.6.11`
and `winnow 0.7.15`. The parser reuses the existing `indexmap 2.13.0`
resolution. The release CLI remains 3,215,120 bytes, a zero-byte change because
this inactive library boundary is not called by the CLI and LTO removes it. The
fully linked probe cost above remains the relevant admission estimate. These
checks do not qualify native behavior.

Evidence is retained in private parser-probe build receipts. The upstream
[0.22.27 package source](https://docs.rs/crate/toml_edit/0.22.27/source/Cargo.toml)
and
[immutable document API](https://docs.rs/toml_edit/0.22.27/toml_edit/struct.ImDocument.html)
identify the exact implementation under review.

## Consequences and limits

The measured linked cost is accepted for full TOML parsing and duplicate/span
validation. Pinning avoids an accidental parser/MSRV upgrade; revising the pin
requires renewed compatibility evidence. Formatting, serde and display features
are unnecessary for this boundary.

This record admits neither client registrations nor a public configuration
command. Existing codecs, schemas, ancillary writers and qualification gates
remain unchanged. It does not resolve the later durable integrated-state format.
