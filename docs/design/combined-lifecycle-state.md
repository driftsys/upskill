# Combined lifecycle state

The opt-in `lifecycle::state` library describes schema-2 state for the future
owned-content lifecycle. Current commands still use schema 1. These records do
not migrate state, fetch sources, claim ownership or write output files.

## Reference authority

Installations bind an explicit ID to source, item kind, source name, installed
alias, root, requested surfaces and captured source-manifest identity. Git
records require their resolved revision to match the owner binding; local
records have no revision. These recorded strings do not prove source
authenticity.

Direct installation and bundle references are roots of a directed graph. Bundle
membership and dependencies retain existing installations without duplicating
artifacts. Validation rejects duplicate IDs and edges, missing endpoints, bundle
cycles and unreachable nodes. Installations with no generated output remain
valid when referenced, for example after audience exclusion.

Distinct IDs cannot describe the same source/kind/name/alias installation under
the same exact root. Requested surfaces and resolved revisions may change on
update without changing that identity. Root comparisons are lexical and retain
raw spellings; they establish no filesystem equivalence or containment.

## Delivery boundary

This foundation is separate from command activation, source resolution, compiler
reconciliation, migration backup, application and recovery. Gate 5 requires all
lifecycle slices; Gate 6 supplies target qualification and activation.
