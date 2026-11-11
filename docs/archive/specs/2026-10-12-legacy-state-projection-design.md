# Pure legacy-state projection design

Date: 2026-10-12. Status: selected for implementation under the user's
delegation of roadmap decisions, subject to independent task and PR review.
Implementation follows source-label compatibility; this design does not activate
migration.

## Intent and scope

The approved roadmap needs safe instructions and other customization lifecycle
support for Claude, Copilot and Codex. Before a schema-2 writer can replace
schema 1, legacy state must remain inspectable without inventing ownership,
requested surfaces, or native integration outcomes. This slice interprets
current typed schema-1 records in memory only. It introduces no CLI change,
fetch, generation, filesystem inspection, adoption, save, backup, or native
operation.

Authority:
[foundation design](2026-10-11-agent-customization-foundation-design.md)
sections 2, 3 and 5; [M1-A2](../plans/2026-10-11-instructions-rules-m1.md).
Preserve Rust 1.85, the synchronous/static-binary constraints, existing OpenCode
behavior, and the fixed original defaults.

## Approaches considered

1. Immediately rewrite old lockfiles as schema 2. Rejected:
   add/update/remove/list/doctor, backup, writer locking and journal
   preservation are not integrated yet.
2. Add optional future-state fields to the current schema-1 records. Rejected:
   this mixes historical facts with new authority and risks silently losing new
   data through old writers.
3. Add a pure legacy projection with an unchanged snapshot, interpreted facts,
   diagnostics and candidate destinations. Selected: it is independently
   testable and provides the migration planner's inputs without enabling
   effects. Cost: one intermediate library model before lifecycle integration.

## Data boundary

Expose one pure projection function accepting a borrowed current `Lockfile` and
returning an owned descriptive projection. Retain an exact clone of the current
typed schema-1 snapshot, including item/bundle order, aliases, source names,
source labels, ref/hash fields, required_by/group links, client strings,
plugin/MCP targets, scopes, statuses and bundle references. This is typed-field
preservation, not a promise to recover unknown JSON fields or original
serialization bytes. Exact raw backup remains executor work.

Every item and bundle has a projected row associated with its original vector
index. Never collapse duplicate names into a map or omit a row because its
source/client/name cannot be interpreted. Diagnostics identify the row and
affected fact. Unsupported schema values produce an inspectable global
diagnostic and no interpreted destinations; they are not treated as schema 1.

Decoded source identity means parsing only. Reuse the persisted-label parser,
including its historical GitLab compatibility, to expose the canonical source
alongside the original label. Invalid labels retain their raw text and an
actionable diagnostic. No decoded source, recorded ref selector, or SSOT hash is
a verified revision or generated-byte ownership proof. Do not promote a
branch/tag string into an immutable revision claim.

All requested-surface history remains explicitly unresolved. Effective output
clients cannot reconstruct the user's original request: author audience
filtering can remove requested families, and one Copilot generation client
cannot distinguish CLI from VS Code Local. Recorded integration targets remain
separate facts and do not resolve content selection. No new target is inferred.

## Historical generation clients

Use an explicit historical client vocabulary containing exactly Claude, Copilot
and OpenCode, independent of future adapters. Empty `clients` expands to that
fixed set. Known client strings map exactly; preserve unknown strings and
diagnose unresolved interpretation rather than silently discarding them or
treating them as the latest default. Redundant known duplicates may describe one
candidate per client, while the unchanged snapshot retains the original vector.

This vocabulary describes legacy generation output. It is not a list of
requested native surfaces or a claim about current/incidental discovery.

## Candidate destinations

Candidates are a distinct type with a historical generation client, relative
entrypoint path and relative resource search base. They have no owners,
installed hashes, modes, existence flags or adoption methods and cannot
substitute for `OwnedArtifact`.

Paths are relative to the already-known legacy scope root. The projector does
not discover HOME/current directory, resolve roots, inspect existence, enumerate
resources, canonicalize paths, or follow symlinks. Missing paths remain
unobserved; the executor must check them later. A resource search base never
authorizes recursive adoption or deletion.

Freeze the nine existing kind/client path contracts. Prefer a small pure
historical-layout module reused by the current pipeline, keeping existing
wrappers and I/O intact, over duplicating the mapping or consulting a future
changing adapter table. Current generation continues using that historical
layout until a later task deliberately changes its adapter. Tests must pin the
historical table independently so future native layout changes cannot silently
redirect migration candidates.

Validate the installed name as one portable relative component before generating
candidates: reject empty, dot/dot-dot, slash, backslash, colon or
control-containing names with an item diagnostic and no destinations. Do not
impose a new slug convention on otherwise representable historical names. A
malformed source does not erase known candidate path facts, but those candidates
remain unverified and unusable for ownership changes without source
reconciliation.

Do not create candidates for CLAUDE.md, AGENTS.md, editor settings, native
config or bundle names. Those are not item-owned legacy entrypoints. Duplicate
item identities and identical entrypoint candidates from distinct rows must
remain visible with diagnostics; do not deduplicate away separate records or
resolve the conflict by choosing a winner. A resource search base does not prove
that any file or directory exists there; defer overlapping-tree analysis until
verified source resources are available.

## Legacy integrations

Provide explicitly tagged legacy plugin/MCP views retaining all current typed
fields, alongside the unchanged snapshot. Do not convert target strings into
permission to add/remove targets, upgrade status to runtime success, run
inventory, or prune a record. Native reconciliation remains M4/M5 work.

## Diagnostics and operation boundary

Diagnostics are data, never stdout/stderr. A malformed item or bundle does not
make other rows uninspectable. No projector result by itself authorizes
mutation. Later planners decide whether a particular operation needs an
unresolved fact and block only those operations; this slice does not add a
generic blocking flag or speculative migration-policy engine.

No serialized projection format or schema-2 codec is introduced. The library
boundary is an internal contract used by future plans and tests.

## Acceptance evidence

Start with failing library acceptance examples for fixed empty-client expansion,
unresolved requested selection, source compatibility, preservation of every
typed legacy field, invalid-row retention, and known historical paths. Cover
aliases versus source names, nested GitLab labels with refs/subfolders, unpinned
sources, unknown clients, invalid names, duplicate identities/locations, both
scopes' root-independent paths, and all plugin/MCP statuses. Verify candidate
hashes/ownership are never fabricated from SSOT hashes.

Keep existing pipeline layout/golden and lifecycle tests green after any pure
layout extraction. Run formatting, full repository verification, and Rust 1.85
all-targets checking. Native discovery and actual source/artifact verification
remain explicitly untested and unclaimed.

## Remaining work

Verified source resolution, reproducible historical rendering, resource
enumeration, ownership adoption, requested-selection reconciliation, schema-2
read/write, lifecycle preservation, exact schema-1 backup, filesystem
containment, preflight, writer lock and journal remain later slices. Do not mark
M1-A2 complete after this projection.
