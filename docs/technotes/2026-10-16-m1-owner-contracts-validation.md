# Per-owner contract validation

The bounded delivery has 16 passing public acceptance examples in
[the contract suite](../../tests/lifecycle_contracts.rs). The dispatcher records
1,191 passing tests across 74 result groups. The initial RED contained 27
missing-interface compiler errors; it is compilation evidence, not an assertion
control. The first 10 passing examples expanded to 16. An empty-output fixture
initially failed requested-surface accounting and was corrected to record its
Claude audience exclusion explicitly.

Examples cover absent historical evidence, exact identity/source/content/profile
bindings, deterministic serialization without input mutation, complete
disagreement reporting, profile order, instruction body/layout separation,
explicit upsert replacement/clearing and untouched evidence, empty-output
history, and surviving shared proof through retirement. The instruction example
also changes preceding newline bookkeeping while retaining proof and preserving
final manual bytes.

Fourteen temporary source controls reached assertion/panic failures for missing
evidence, source agreement, profile agreement, content binding, nonzero
revisions and source binding, exact surface membership, partially missing
evidence, rule-only evidence preservation of prior contract positions,
selected-proof clearing, retained and retired proof, and canonical contract
ordering. The dispatcher recorded byte-for-byte source restoration after each.
These controls target fourteen checks; they do not claim independent mutation
coverage of every propagation and ordering rule.

This record reconciles the local implementation before commit. Before gardening,
the dispatcher recorded successful `just fmt`, `just sanity`, `just verify` and
`cargo +1.85.0 check --all-targets --locked`. These local results precede the
durable documentation edits. Independent reviews, final documentation checks and
exact-head hosted CI remain separate delivery gates. Supplied state and
observations establish neither native client behavior nor source authenticity,
physical containment or filesystem qualification. Existing shared-mutation
rejection remains. Production commands and schema 1 are unchanged; this slice
does not complete Gate 5 or activate M1.
