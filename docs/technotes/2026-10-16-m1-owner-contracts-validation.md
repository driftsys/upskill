# Per-owner contract validation

The initial implementation at `d809fe2f10d221874fdf0af8c7cc5636ad40adf1` had 16
passing public examples and 1,191 passing Rust tests across 74 result groups.
The review fixes add eleven examples, giving 27 in
[the contract suite](../../tests/lifecycle_contracts.rs). The complete captured
reports and mutation recipes are retained as compressed files alongside a
[content-addressed manifest](evidence/2026-10-16-m1-owner-contracts/manifest.json).
The manifest identifies the parent commit and exact production/test source
hashes; the later snapshot includes the uncommitted review fixes at capture
time. These are dispatcher-captured reports, not independently authenticated
execution.

## Examples and controls

The initial RED contained 27 missing-interface compiler errors; it is
compilation evidence, not an assertion control. The first ten passing examples
expanded to sixteen before review. An empty-output fixture initially failed
requested-surface accounting and was corrected to record its Claude audience
exclusion explicitly.

Examples cover historical absence, exact source/content/profile bindings,
complete codec preservation and ordering, unknown nested fields, exact query
roots, all-owner diagnostics with nonlexical owner order, Git capture binding,
instruction layout independence, no-op omitted-proof clearing, previously absent
evidence insertion and ordering, unchanged-owner rejection without duplicate
masking, graph retirement, and retained shared proof.

Fourteen initial fault controls and twelve review-fix controls reached failing
assertions. The original recipes and output are retained; the twelve later
controls also retain exact patches and before/after source-hash receipts. Each
source was restored after its control. The controls exercise missing proof,
source/profile/content binding, revisions, exact surfaces, rule-only evidence,
propagation, ordering, complete codec preservation, whole-state agreement
validation, exact query roots and nested strict decoding. This is targeted fault
coverage, not exhaustive mutation qualification.

## Reading the retained evidence

Files use gzip compression. For example, from the repository root:

```bash
gzip -dc docs/technotes/evidence/2026-10-16-m1-owner-contracts/verify-final.log.gz
```

The manifest gives SHA-256 hashes and byte lengths of the uncompressed files.
`verify-baseline.log.gz` retains the initial implementation run;
`verify-final.log.gz` retains the review-fix run. Formatting, sanity and Rust
1.85 all-target checks have separate reports. Control recipes are historical
execution inputs; use a disposable isolated checkout when reproducing their
source edits.

## Limits

Independent reviews and exact-head hosted CI are separate delivery gates.
Supplied state and observations establish neither native client behavior nor
source authenticity, physical containment or filesystem qualification. Existing
shared-mutation rejection remains. Production commands and schema 1 are
unchanged; this slice does not complete Gate 5 or activate M1.
