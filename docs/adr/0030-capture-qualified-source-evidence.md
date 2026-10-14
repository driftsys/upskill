# AD-0030: Capture qualified source evidence

Inactive library implementation complete; hosted Linux and Windows scope/compile
checks passed at `a393263` on 2026-10-13. Final-head hosted checks remain
required.

## Context

Strict source manifests supply exact freely mutable values to pure compilation.
Their existing ambient-path capture establishes neither descriptor-relative
containment nor later source revalidation. Qualified destination and scope-lock
boundaries are implemented independently; lifecycle effects remain inactive.

## Options

Ambient path rereads are simple but do not bind descendant access to inspected
parents. Creating platform snapshots or probe files adds writes and
dependencies. Select descriptor-relative full-tree capture with retained handles
and complete membership revalidation, using the existing qualified scope and
mount policy.

## Decision

Implement the
[qualified source contract](../design/qualified-source-observation.md) as a
separate opaque non-Clone snapshot borrowing a QualifiedScope. Expose only an
immutable existing SourceManifest and non-refreshing verify_unchanged. Preserve
complete source inclusion and hard-link paths; reject links, special entries and
mount changes. Record empty-directory topology privately. Share native
qualification without changing destination policy or strict manifest digests.
Keep the current CLI, schema 1, dependencies and legacy capture behavior
unchanged.

## Consequences

Retained descriptors and repeated exact checks cost memory, descriptors and I/O.
Recoverable errors return contextual failures without a partial snapshot;
ordinary allocation or OS termination is not a recoverable exhaustion guarantee.
A sampled check cannot promise an atomic snapshot or exclusion against hostile
writers. Source and destination roots may use separate qualified mounts.
Permission application, Git revision/authenticity, lifecycle state, effects,
recovery and Windows qualification remain separate gates.

Local implementation checks pass.
[CI run 38346202172](https://github.com/driftsys/upskill/actions/runs/38346202172)
and
[CodeQL run 38346200154](https://github.com/driftsys/upskill/actions/runs/38346200154)
succeeded at `a393263` on 2026-10-13. That Windows run checked scope rejection
and source API compilation, not the source method inside the previously uncalled
helper. The new private unsupported-host test executes that method. Final-head
hosted checks, including native execution of this assertion on Windows, remain
merge gates. The
[design evidence](../design/qualified-source-observation.md#local-implementation-evidence)
records local coverage, independent-control limits and matched release-size
measurements. Initial public binding failures preserve source-root/verification
context and their original scope cause; retained nested paths are independently
pinned. No as-built binding-context exception remains. No lifecycle or CLI
activation follows.

Satisfies:
[specification §4.9](https://github.com/driftsys/upskill/blob/a4e84d9334f7717bfb170d59afcbaf7f829d1e82/docs/specification/specification.md#49-qualified-source-tree-observations).
Related: [AD-0028](0028-lock-qualified-scope-directories.md),
[AD-0029](0029-capture-qualified-destination-evidence.md).

The binding-context fix measured at `a3932633e9cc9a269f8ba8d8b6dd2ec26d0b828e`
refreshes the candidate CLI and live public-API probe under the same Rust 1.97.1
locked dependencies, release profile and runtime inputs. Exact hash-verified
prior baseline/control binaries are reused: CLI baseline/candidate remain
3,215,120 bytes (delta zero); control is 303,120 bytes and live
capture/revalidation is 353,840 bytes (delta 50,720). Both runtime-input
fixtures pass. No byte-identity, performance or final-head hosted success claim
follows from these size measurements.

The later test-seam and fixture closeout does not repeat these size builds or
claim these measurements for a future head. Independent final revalidation
binding omission coverage now complements the earlier capture binding controls.
