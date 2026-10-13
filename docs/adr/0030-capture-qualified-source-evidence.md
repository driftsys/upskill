# AD-0030: Capture qualified source evidence

Selected design; implementation and qualification evidence pending.

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
Windows qualification remain separate gates. Implementation and hosted evidence
must be recorded before changing this document's pending status.
