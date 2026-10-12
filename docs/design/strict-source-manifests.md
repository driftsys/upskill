# Strict captured source manifests

This record describes the implemented [`source_manifest`](../../src/source_manifest.rs)
library boundary. [Specification §4.3](../specification/specification.md#43-strict-captured-source-manifests)
owns its requirements. Capture observes source files; validation checks supplied
values without I/O. Neither function participates in current command dispatch.
[AD-0019](../adr/0019-separate-strict-source-capture.md) records that separation.

## Public interface and evidence

`capture_source` takes a borrowed filesystem path and returns an owned
`SourceManifest` through `anyhow::Result`. Its `files` vector contains
`CapturedSourceFile` values with a portable relative path, exact owned bytes,
lowercase SHA-256 of those bytes, and optional observed permission bits. The
manifest also carries its framed SHA-256. Public values derive debugging,
cloning and equality; they have no serialization or trusted-provenance wrapper.
Callers can alter them, so consumers must validate supplied values.

`validate_manifest` borrows the manifest without mutation or sorting. For each
record it checks the shared ownership lexical-path rule, strict ascending path
order and uniqueness, mode range, and exact byte digest, then checks the complete
manifest digest. Invalid input returns contextual errors rather than a partial
validated result. Ownership uses the same extracted crate-private path predicate
at its original validation position with unchanged diagnostics.

## Observation and assembly

Capture reconstructs the root from standard path components before metadata
inspection. This lexical step removes redundant trailing separators and dot
suffixes without canonicalizing the root or resolving its ancestors. Root
metadata must identify an existing directory rather than an observed symlink.

A private synchronous `ObservationIo` boundary supplies directory entries,
symlink metadata and file reads. Directory enumeration preserves individual
entry results inside an outer enumeration result, so traversal can report both
directory-open and iterator errors. Metadata and read errors also propagate with
operation and path context; no failure returns a partial manifest. The private
fault delegate exercises these paths after an earlier successful file read.

Traversal rejects observed symlinks before following or reading them, validates
relative names before either directory recursion or regular-file capture, and
rejects other entry types before opening them. It applies no ignore rules or
entrypoint exclusions. Hidden files, gitignore-matched files, binary and empty
files, nested entrypoints and overrides are captured. Empty directories create
no records; invalid directory names still fail validation. Hard-linked regular
files produce a record for each path, without a topology guarantee.

Each regular file is read once. Unix permission evidence comes from its observed
metadata masked to `0o7777`; other hosts record no mode. These bits authorize no
permission application or execution. The complete vector sorts by Rust string
path ordering, receives its framed digest and passes pure validation before
return. [AD-0020](../adr/0020-framed-source-manifest-evidence.md) records the
versioned digest choice; its exact encoding belongs to specification §4.3.

## Integration and evidence limits

Captured bytes let later planning avoid rereading mutable source paths. They do
not establish authenticity, an immutable Git revision or a single observation
instant. Ordinary synchronous standard-library APIs provide no atomic snapshot,
descriptor-relative containment or protection against concurrent replacement.
Root ancestors can be symlinks. Lexical path checks do not qualify aliases, case
collisions, reserved names or destination filesystem limits.

Legacy hashing and discovery retain their existing runtime behavior and persisted
digest meanings. No CLI, lockfile, source-schema or native-client activation is
introduced. Source stabilization, qualified observation, whole-file/resource
planning, destination qualification and lifecycle execution remain later work.

[`tests/source_manifests.rs`](../../tests/source_manifests.rs) pins literal file
and manifest vectors, path/content/mode changes, complete inclusion, validation
diagnostics and supported platform rejection fixtures. Private module tests pin
fault propagation and read counts. Actual non-UTF-8 filesystem-name fixtures are
limited to Unix excluding macOS: the development macOS filesystem rejected their
creation, so a private pure path-conversion fixture covers rejection there.
FIFO and socket tests do not claim device-entry or concurrent-containment
qualification. Platform-gated tests describe coverage, not proof that each host
has executed them.
