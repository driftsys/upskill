# AD-0030: Capture qualified source evidence

Inactive library implementation complete locally; final-head hosted qualification pending.

## Context

Strict source manifests supply exact freely mutable values to pure compilation.
Their existing ambient-path capture establishes neither descriptor-relative
containment nor later source revalidation. Qualified destination and scope-lock
boundaries are implemented independently; lifecycle effects remain inactive.

## Options

Ambient path rereads are simple but do not bind descendant access to inspected
parents. Creating platform snapshots or probe files adds writes and dependencies.
Select descriptor-relative full-tree capture with retained handles and complete
membership revalidation, using the existing qualified scope and mount policy.

## Decision

Implement the [qualified source contract](../design/qualified-source-observation.md)
as a separate opaque non-Clone snapshot borrowing a QualifiedScope. Expose only
an immutable existing SourceManifest and non-refreshing verify_unchanged.
Preserve complete source inclusion and hard-link paths; reject links, special
entries and mount changes. Record empty-directory topology privately. Share native
qualification without changing destination policy or strict manifest digests.
Keep the current CLI, schema 1, dependencies and legacy capture behavior unchanged.

## Consequences

Retained descriptors and repeated exact checks cost memory, descriptors and I/O.
Recoverable errors return contextual failures without a partial snapshot; ordinary
allocation or OS termination is not a recoverable exhaustion guarantee. A sampled
check cannot promise an atomic snapshot or exclusion against hostile writers.
Source and destination roots may use separate qualified mounts. Permission
application, Git revision/authenticity, lifecycle state, effects, recovery and
Windows qualification remain separate gates.

Native APFS focused evidence currently covers 7 public source acceptance tests,
25 source-private tests, 24 unchanged strict-manifest acceptance tests and 28
destination acceptance tests. Independent omission controls retain exact
restoration evidence. Complete release checks, MSRV and warning-denying all-target
Clippy are required at task closeout; final-head hosted Linux missing-mask and
Windows unsupported-host checks remain pending. See the design record for the
local coverage and evidence limits. No lifecycle or CLI activation follows.

## Matched local release size

Rust 1.97.1 release builds with identical pinned root dependency resolution and
release profile produce byte-identical baseline/candidate CLI binaries of
3,215,120 bytes (delta 0). Minimal public-API probes use the same pinned package
versions and profile: control 303,120 bytes, source capture plus revalidation
353,840 bytes (delta 50,720). Both runtime-input probe fixtures succeed. This
records binary size only, not performance; inactive API code is stripped from the
current CLI. Final-head hosted Linux/Windows qualification remains pending.
