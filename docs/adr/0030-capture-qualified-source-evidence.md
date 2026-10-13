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

Local implementation checks pass; final-head hosted Linux and Windows
qualification remains pending. The [design evidence](../design/qualified-source-observation.md#local-implementation-evidence)
records local coverage, independent-control limits and
matched release-size measurements. Initial public binding failures preserve
source-root/verification context and their original scope cause; retained nested
paths are independently pinned. No as-built binding-context exception remains. No lifecycle or CLI activation follows.

Satisfies: [specification §4.9](https://github.com/driftsys/upskill/blob/a4e84d9334f7717bfb170d59afcbaf7f829d1e82/docs/specification/specification.md#49-qualified-source-tree-observations).
Related: [AD-0028](0028-lock-qualified-scope-directories.md),
[AD-0029](0029-capture-qualified-destination-evidence.md).

The final binding-context fix refreshes the candidate CLI and live public-API
probe under the same Rust 1.97.1 locked dependencies, release profile and runtime
inputs. Exact hash-verified prior baseline/control binaries are reused: CLI
baseline/candidate remain 3,215,120 bytes (delta zero); control is 303,120 bytes and
live capture/revalidation is 353,840 bytes (delta 50,720). Both runtime-input
fixtures pass. No byte-identity, performance or final-head hosted success claim
follows from these size measurements.
