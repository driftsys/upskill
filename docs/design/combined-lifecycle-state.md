# Combined lifecycle state

The opt-in `lifecycle::state` library describes schema-2 state for the future
owned-content lifecycle. Current commands still use schema 1. These records do
not migrate state, fetch sources, claim ownership or write output files. The
[specification §4.13](../specification/specification.md#413-combined-lifecycle-state-foundation)
owns the requirements; [`lifecycle::state`](../../src/lifecycle/state/mod.rs)
exposes the records and public functions.

## Public records and validation

`CombinedState` holds the schema number, installations, bundles, references,
owned artifacts, per-owner consumers and optional typed legacy snapshot. An
`Installation` reuses `ArtifactOwner` and `InstallationRoot`, and records its
complete requested target surfaces and `SourceEvidence`. A `BundleNode` records
its ID, source, name and optional source revision. `Reference` distinguishes
direct installations, direct bundles, bundle membership and bundle dependencies.
`OwnerConsumers` binds an installation ID to a root, relative path, optional
contribution selector and explicit consumers. Existing ownership, target and
schema-1 types retain their wire representations. No new dependency is required.

[`validate_state`](../../src/lifecycle/state/validate.rs) checks the complete
envelope and reuses `ownership::validate_artifacts` for artifact shape.
[`associations`](../../src/lifecycle/state/associations.rs) supplies the
internal owner/selector checks within that same public validation boundary.
Parsing and serialization both call this validator. Inputs remain unchanged on
rejection.

## Reference authority

Installations bind an explicit ID to source, item kind, source name, installed
alias, root, requested surfaces and captured source-manifest identity. Git
records require their resolved revision to match the owner binding; local
records have no revision. These recorded strings do not prove source
authenticity. Manifest digests must contain exactly 64 lowercase hexadecimal
characters; installation sources and names are nonempty. Roots must be absolute,
and requested surfaces must be nonempty and unique. Git evidence requires a
nonempty revision; an optional bundle revision cannot be empty.

Node IDs are caller supplied, nonempty ASCII alphanumeric, hyphen or underscore
strings, unique within each node kind. Bundle and installation namespaces are
typed, so the same string may occur in both. Identity allocation for lifecycle
operations remains later work.

Direct installation and bundle references are roots of a directed graph. Bundle
membership and dependencies retain existing installations without duplicating
artifacts. Validation rejects duplicate IDs and edges, missing endpoints, bundle
cycles and unreachable nodes. An iterative topological traversal rejects bundle
cycles without recursive graph traversal, and traversal from direct references
establishes reachability. Installations with no generated output remain valid
when referenced, for example after audience exclusion.

Distinct IDs cannot describe the same source/kind/name/alias installation under
the same exact root. Requested surfaces and resolved revisions may change on
update without changing that identity. Root comparisons are lexical and retain
raw spellings; they establish no filesystem equivalence or containment.

The combined graph is the single authority for new installation bindings and
references. Reusing existing ownership and legacy types preserves their reviewed
contracts. Independent writable sidecars and automatic legacy adoption were
rejected because they would separate authority or invent ownership evidence.

## Delivery boundary

This foundation is separate from command activation, source resolution, compiler
reconciliation, migration backup, application and recovery. Gate 5 requires all
lifecycle slices; Gate 6 supplies target qualification and activation.

## Owners and consumers

Every artifact owner must exactly match its authoritative installation,
including revision and alias. Artifact roots must match the installation scope
and raw root spelling. Existing artifact validation still rejects ambiguous
whole-file/contribution locations and malformed content metadata.

Each artifact selector and owner has exactly one explicit consumer association.
Whole files use no contribution selector; Markdown regions use their recorded
contribution ID. Associations must name existing owners, contain unique nonempty
surfaces within that owner's requested selection, and have no surplus entries.
The union of these associations equals the artifact consumer set exactly.
Validation never infers individual consumers from that union.

## Codec and retained legacy data

`parse_state` and `serialize_state` both validate the complete envelope.
Schema-2 records reject unknown fields. Encoding sorts owned graph records,
selectors, owners and consumer lists in a cloned value and emits pretty JSON
with one trailing newline. It does not change the supplied state.

`project_legacy` requires schema 1 and retains its typed `Lockfile` snapshot in
`legacy`, with an empty owned graph. Legacy arrays, duplicate rows, labels,
recorded refs, optional fields and integration statuses keep their original
values and order. They are unresolved historical data, never ownership evidence.
Unknown original JSON fields and serialization bytes are outside this typed
projection. The applying lifecycle must preserve them in an exact first-write
backup; this pure slice does not create that backup. The text codec accepts
UTF-8 `&str`; future byte-loading callers must reject invalid UTF-8 explicitly.

An empty envelope has `schema: 2`, empty `installations`, `bundles`,
`references`, `artifacts` and `owner_consumers` arrays, and `legacy: null`. New
references are tagged as `direct-installation`, `direct-bundle`,
`bundle-installation` or `bundle-dependency`; source evidence is tagged `git` or
`local`. Legacy fields keep their existing schema-1 names rather than being
normalized to new tags.

## Verification boundary

[`tests/lifecycle_state.rs`](../../tests/lifecycle_state.rs) exercises graph
sharing, every reference kind, reachability and cycles, identity collisions,
source revision/hash bindings, audience-excluded installations, exact
owner/consumer associations, lexical roots and selectors, strict decoding,
deterministic owned ordering and typed legacy retention. Independent controls
omit validation or sorting checks and require assertion failures after each
omission, followed by exact restoration. The
[validation record](../technotes/2026-10-15-m1-gate5-state-validation.md)
records the checked revision and scope of the evidence.

This completes the pure state slice. Separate
[captured desired-set production](captured-rule-desired-sets.md),
[reference reconciliation](lifecycle-reference-reconciliation.md) and
[shared-owner removal planning](lifecycle-reference-removal.md) are implemented.
Complete add/update planning, exact original-JSON backup, guarded application,
checkpoint recovery integration and observational lifecycle adapters remain
later work. Native and complete lifecycle qualification remain required; this
record does not declare Gate 5 or M1 complete.
