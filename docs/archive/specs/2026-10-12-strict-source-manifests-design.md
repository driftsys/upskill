# Strict captured source manifests

Date: 2026-10-12. Status: **Design for bounded implementation**.
Baseline: `2a3c3b5`.
Parent direction: [M1 owned lifecycle](../../wip/specs/2026-10-12-m1-owned-lifecycle-design.md).

## Intent and boundary

The future owned lifecycle needs complete captured source bytes before planning
individual output files and resources. Existing legacy hashing and discovery can
skip read errors and do not provide this evidence. Add a separate opt-in library
capture boundary, preserving the current pipeline, hashes, commands and schemas.
A successful capture describes the regular files actually read. It neither
verifies source authenticity nor authorizes installation or ownership adoption.

This is the source-observation prerequisite of operational delivery step 2.
Dedicated-file/resource planning still follows separately. Git resolution,
entrypoint/resource classification, client rendering, ignore selection,
reference resolution, destination qualification, execution and state activation
remain outside this task. Capture does not execute source content.

## Alternatives and decision

1. Add strict behavior to legacy discovery/hash helpers. This changes current
   installs and persisted hash meanings; it also couples new evidence to existing
   best-effort behavior. Reject for this slice.
2. Introduce only a pure manifest model. This can validate supplied values but
   leaves source read failures and skipped entries unresolved. Insufficient alone.
3. Add a strict captured-file observer plus a pure manifest validator. Select
   this option: future planning consumes captured bytes rather than rereading
   mutable source paths, while existing commands remain unchanged.

The user delegated routine design decisions and continuing the reviewed delivery
loop. Written spec and plan stages remain required before implementation.

## Public contract

Add `source_manifest` as a library module, separate from source-label parsing.
Public DTOs derive `Debug, Clone, PartialEq, Eq`, without serialization derives:

- `CapturedSourceFile`: `path: String`, `bytes: Vec<u8>`,
  `content_sha256: String`, `mode: Option<u32>`.
- `SourceManifest`: `files: Vec<CapturedSourceFile>`,
  `content_sha256: String`.
- `capture_source(root: &Path) -> anyhow::Result<SourceManifest>`.
- `validate_manifest(manifest: &SourceManifest) -> anyhow::Result<()>`.

For the root metadata check, reconstruct the supplied path from its standard
`Path::components()` without canonicalization. This removes redundant trailing
separators and dot suffixes, so `link/` and `link/.` do not evade the observed-root
symlink rejection. Parent components and ancestor symlinks are not resolved by
this lexical step. The checked root must be an existing directory; reject an
observed root symlink and non-directory root. Root spelling does not
enter the digest. Walk every entry recursively, without ignore rules or
entrypoint exclusions; include binary files and nested entrypoint names.
Empty directories carry no file record. An empty root is a valid empty manifest.

Fail on enumeration, entry metadata, file-open/read or mode-observation errors.
Reject every observed symlink, including broken links, directory links and cycles;
do not follow or silently skip them. Reject observed special entries such as
FIFOs, sockets and devices before reading them. Do not return a partial manifest.
Hard-linked regular files are captured by each path; no topology claim is made.

Each record has a UTF-8 relative path joined with `/`. Reject non-UTF-8 components,
empty/dot/parent components, backslashes, colons and Unicode control characters.
These are lexical component rules, not proof of destination compatibility: case
collisions, reserved names, aliases and filesystem limits remain qualification
work. Reuse the existing ownership path predicate through a small crate-private
helper extraction, preserving its public errors and behavior; do not duplicate
that rule or introduce an unrelated path framework.

Read each regular file once into owned bytes. Its lowercase content hash is
SHA-256 of exactly those bytes, including empty/binary files. On Unix, capture
`PermissionsExt::mode() & 0o7777`; on other hosts record `None`. This is observed
permission evidence, not permission to apply or execute those bits. There is no
cross-platform equality claim when available mode evidence differs.

Sort captured files by their path's Rust string ordering. Return unique paths in
strict ascending order. The pure validator checks order, uniqueness, lexical
paths, each byte hash, optional mode range `0..=0o7777` and the complete manifest
hash. It does not reorder or mutate inputs. Public fields are inspectable data,
not a trusted provenance token; callers must validate supplied/modified values.

## Digest version and framing

Define the manifest digest as SHA-256 over this exact byte stream:

1. ASCII `upskill:source-manifest:v1` followed by one NUL byte.
2. File count as an unsigned 64-bit big-endian integer.
3. For each sorted file: UTF-8 path byte length as unsigned 64-bit big-endian,
   path bytes, content byte length in the same encoding, exact content bytes,
   then mode-presence byte (`0` absent, `1` present). A present mode follows as
   an unsigned 32-bit big-endian integer.

Use checked integer conversion and contextual failure, rather than truncating
lengths. The per-file hash field is verified but is not an additional digest
input. Directory root names, timestamps and traversal order are not inputs.
Mode and path changes are inputs. This is a new internal versioned evidence
format; do not reinterpret or rewrite existing legacy digests.

## Filesystem evidence limits

Capture uses synchronous standard-library filesystem APIs on Rust 1.85. It
rejects symlinks and unsupported types when observed, but does not provide an
atomic filesystem snapshot, descriptor-relative containment or protection
against concurrent entry replacement. An ancestor of the supplied root may be
a symlink. Do not label these APIs race-safe or containment-qualified.

A successful result owns the exact bytes captured, so later planning can use
them without rereading the source. It does not prove every byte came from one
instant or that no entry appeared during traversal. Source stabilization and
qualified observation remain activation gates before security or migration
claims. Tests must not imply those unsupplied guarantees.

## Acceptance and evidence

- Capture nested text, binary and empty files in deterministic path order; verify
  exact bytes, independent literal per-file and framed manifest digests, and
  available permission evidence. Equivalent trees at different roots match.
- Rename a file, edit bytes or change observed mode: relevant evidence changes.
  Distinguish ambiguous unframed path/content combinations using literal vectors.
- Empty roots produce the specified framed digest. Empty directories create no
  record. Nested `SKILL.md` and top-level override names are captured normally.
- Missing/file roots, observed root/entry symlinks (including root trailing
  separator/dot spellings), broken links/cycles, special
  entries and non-UTF-8/invalid lexical names fail where the platform permits
  constructing those fixtures. Unsupported fixtures are explicitly OS-scoped.
- Inject or otherwise reliably force enumeration/metadata/read failure without
  relying on mode bits under an elevated user. Verify contextual error and no
  successful partial result. Keep test injection crate-private and narrow if
  needed; do not expose filesystem mocking as a public feature.
- Pure validation rejects bad order, duplicates, invalid paths, wrong file or
  manifest hashes and out-of-range modes. Inputs remain unchanged. Tests use
  independent expected values rather than computing expectations through the
  implementation under test.
- Establish compile RED, then valid behavioral assertion RED before implementing
  capture/validation. Serialize test runs during temporary code changes. Kill
  semantic mutations that skip an entry/read failure and remove digest framing,
  restore byte-for-byte and rerun focused GREEN.

Finish with ordered `just fmt`, `just verify`, Rust 1.85 all-target checking,
fresh task/final/PR reviews and hosted CI. No new dependencies, async runtime,
stdout/stderr presentation, CLI behavior, source schema, lockfile writes or
native operations. Documentation must distinguish captured evidence from source
authenticity and qualified filesystem observation.
