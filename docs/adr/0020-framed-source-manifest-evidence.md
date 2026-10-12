# Version and frame captured source evidence

AD-0020

## Context

A source manifest needs a deterministic digest that distinguishes path/content
boundaries and available permission evidence. Legacy unframed hashing cannot
serve as verification of this new captured representation.

## Options

Reuse legacy hashing or define separate domain-separated, versioned and
length-framed evidence. The former retains ambiguous path/content concatenation
and existing best-effort semantics. Other alternatives are not documented.

## Decision

Hash the exact versioned stream specified in specification §4.3: a domain tag,
file count, each sorted path and content with checked lengths, and optional mode
presence and value. Verify per-file byte hashes independently; they are not
additional manifest digest inputs. Do not reinterpret legacy digests.

## Consequences

Paths, bytes and observed mode evidence affect the digest; roots, timestamps and
enumeration order do not. Different available mode evidence can produce different
cross-platform digests. This new internal evidence version must retain its exact
encoding. A valid digest verifies supplied values rather than source provenance.

Satisfies: [Specification §4.3](../specification/specification.md#43-strict-captured-source-manifests).
Related: [source manifest design](../design/strict-source-manifests.md),
[AD-0019](0019-separate-strict-source-capture.md).
Rationale: [archived spec](../archive/specs/2026-10-12-strict-source-manifests-design.md).
