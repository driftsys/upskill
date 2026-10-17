# Per-owner contract validation

Validation record date: 2026-10-16. This date identifies the original evidence
record; consolidation does not rerun or extend its qualification.

The initial implementation at `d809fe2f10d221874fdf0af8c7cc5636ad40adf1` had 16
passing public examples and 1,191 passing Rust tests across 74 result groups.
The review fixes add fifteen examples, giving 31 in
[the contract suite](../../tests/lifecycle_contracts.rs). Captured reports and
control recipes are retained as compressed files alongside a
[content-addressed manifest](evidence/2026-10-16-m1-owner-contracts/manifest.json).
It identifies the parent commit and exact production/test source hashes. The
later snapshot includes uncommitted review fixes at capture time. These are
dispatcher-captured reports, not independently authenticated execution.

## Examples and controls

The initial RED contained 27 missing-interface compiler errors. This is
compilation evidence, not an assertion control.

Examples cover historical absence, exact source/content/profile bindings,
complete file/instruction codec preservation and identity ordering, unknown
nested fields, exact query roots, nonlexical all-owner diagnostics, Git capture
binding, instruction layout independence, no-op omitted-proof clearing, new
evidence insertion, unchanged-owner rejection without duplicate masking, graph
retirement, and shared proof and relative order surviving removal.

The unified retained `all-controls.py.gz` recipe reruns 33 assertion-failing
controls against the captured source and test hashes, including the previously
missing surface-membership recipe. Each run retains separate patch and output
files listed in the manifest. `all-control-receipts.json.gz` records the source
path, test identity and before/after restoration hashes. The two
surface-membership runs deliberately repeat the same guard. Earlier recipes and
reports remain historical context; the unified recipe supplies the current
reproducible set.

Controls cover missing proof, source/profile/content binding, revisions, exact
surfaces, rule-only evidence, propagation, ordering, distinct per-owner profile
preservation, complete file/instruction codec preservation, unrelated invalid
contract evidence, whole-state validation, exact query roots and strict
decoding. This is targeted fault coverage, not exhaustive mutation
qualification.

## Reading the retained evidence

Files use gzip compression. For example, from the repository root:

```bash
gzip -dc docs/technotes/evidence/2026-10-16-m1-owner-contracts/verify-final.log.gz
```

The manifest gives SHA-256 hashes and byte lengths of the uncompressed files.
`verify-baseline.log.gz` retains the initial implementation run;
`verify-final.log.gz` retains the final review-fix run. Formatting, sanity and
Rust 1.85 all-target checks have separate reports. Reproduce control source
edits only in a disposable isolated checkout. Create the output directory before
running the recipe from that checkout's repository root:

```bash
mkdir -p .superpowers/sdd/2026-10-16-m1-owner-contracts
gzip -dc docs/technotes/evidence/2026-10-16-m1-owner-contracts/all-controls.py.gz | python3 -
```

Manifest integrity was checked at capture; this delivery does not add a
permanent evidence-integrity CI gate.

## Limits

Independent reviews and exact-head hosted CI are separate delivery gates.
Supplied state and observations establish neither native client behavior nor
source authenticity, physical containment or filesystem qualification. Existing
shared-mutation rejection remains. Production commands and schema 1 are
unchanged; this slice does not complete Gate 5 or activate M1.
