# Owned whole-file operation planning

This record describes the pure planner implemented in
[`files::plan`](../../src/files/plan.rs). [Specification §4.4](../specification/specification.md#44-owned-whole-file-operation-plans)
owns its requirements. The implementation consumes supplied values and performs
no I/O. [AD-0021](../adr/0021-pure-file-planning.md) records the bounded delivery choice.

## Public boundary

`plan_files` borrows prior `OwnedArtifact` records, `FileInstallationChange`
values and `ObservedFile` values. It returns an owned `FileOperationPlan` through
`anyhow::Result`. The seven public DTOs derive `Debug`, `Clone`, `PartialEq` and
`Eq` without serialization. `FileLocation` combines an `InstallationRoot` with a
portable relative path; `FileValue` carries exact bytes and optional permission
mode. Desired files additionally carry explicit consumers. A set change carries
one explicit owner and its complete nonempty desired file set; removal names an
installation ID. An observed absent value means an absent directory entry; it
does not represent an unreadable file or directory.

The result contains the complete ownership vector and individual `PlannedFile`
entries with exact optional before/after values. Byte-identical no-ops retain
preconditions. Deletions identify individual files and grant no directory or
recursive removal authority. Caller-supplied evidence establishes neither source
provenance nor qualified filesystem authority.

## Validation and indexing

The planner validates prior records with `ownership::validate_artifacts`, then
validates observed roots, portable paths and optional modes. It indexes changes,
validates desired records through the ownership boundary and rejects duplicate
installation changes, empty sets, unknown removals and duplicate desired locations.
Invalid observed or desired locations and modes fail before observation membership
diagnostics. Desired record hashes use SHA-256 of exact desired bytes, rendered
as lowercase hexadecimal through the existing digest dependency.

Location keys combine scope, exact native root `OsString` spelling and portable
path in ordered maps and sets. No `Path` component normalization, lossy string
conversion, canonicalization, case folding, alias resolution or containment
qualification occurs. [AD-0021](../adr/0021-pure-file-planning.md) records
location identity, complete replacements, stable bindings and deterministic
ordering as parts of the pure API boundary.

## Affected locations and before-state verification

Affected locations are the union of desired locations and prior locations with
any changed owner. Affected contributions and multi-owner files fail before
planning; a desired location cannot take over an untouched owner. Unrelated rows,
including shared files and contributions, remain exact. Both old and new owners
must be selected for a same-location handoff. [AD-0022](../adr/0022-conservative-file-ownership-evidence.md)
records the composition restriction.

Observation keys must match the affected set exactly: duplicates, missing entries
and extraneous entries fail, including for revision-only changes and no-ops.
At a prior owned location, observed bytes must be present and match its stored
SHA-256, and optional modes must match exactly. At an unowned location the
observation must establish absence even when existing bytes equal the desired
bytes. These choices are recorded in [AD-0022](../adr/0022-conservative-file-ownership-evidence.md).

The topology check compares the union of prior and desired locations in the same
exact root and scope. It rejects a strict slash-component ancestor pair whenever
one side is affected, including removal of `a` followed by creation of `a/b`.
Contributions count as file locations. Component-prefix siblings and disjoint
untouched pairs remain allowed. [AD-0023](../adr/0023-reject-file-topology-transitions.md)
records why directory transitions await a qualified contract.

## Result assembly and downstream limits

Each affected location produces one action in lexical key order. The planner
walks prior rows to preserve unaffected rows and the positions of same-location
replacements, drops removed rows, then appends new desired rows in lexical order.
Replacement records contain the explicit owner, consumers, desired mode and exact
byte hash. Final ownership validation precedes return; any error returns no
successful partial plan. Inputs remain unchanged. [AD-0021](../adr/0021-pure-file-planning.md)
records the ordering choice.

[`tests/file_plans.rs`](../../tests/file_plans.rs) covers exact creation,
replacement, removal, owner handoff, no-op preconditions, untouched-record
retention, metadata validation, mode boundaries, binary/empty/Unicode values,
raw-root and scope isolation, ancestor conflicts and deterministic output. The
critical creation, update and empty-content hash assertions use independent
literal SHA-256 values. Other fixture-derived expected records use test-side
`Sha256::digest`. The tests establish supplied-value behavior, without proving
actual filesystem observation or execution.

A future compiler must derive complete desired resource and entrypoint sets from
captured sources and reviewed adapters. Mixed instruction/file installations
need a coordinator; neither planner authorizes partial application. Qualified
source/destination observation, equivalence and containment, writer locking,
permission policy, durable state, apply/recovery and Codex activation remain
pending. This prerequisite leaves current commands, schema-1 lockfiles and native
client behavior unchanged and does not complete M1-A2, M1-A3 or M1.
