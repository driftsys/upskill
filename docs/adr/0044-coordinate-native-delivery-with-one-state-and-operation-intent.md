# AD-0044 — Coordinate native delivery with one state and operation intent

Status: reviewed inactive coordination contract. Date: 2026-11-01.

## Context

The approved skills/resources plan requires durable native coordination.
[AD-0042](0042-keep-native-skill-planning-outside-current-codecs.md) leaves its
persistence format open: retained captures and native receipts cannot be
inserted as active schema-2 graph nodes. The existing executor journals
File/State actions under one root. It neither records native effects nor
protects another root after a process crashes. The captured skill planner also
carries source resource modes; those are not applied destination modes.

## Decision

Implement Stage 5 as inactive infrastructure. Keep production CLI dispatch and
both existing codecs unchanged. Admit no native profile or migration here.

### One integrated state

Use an explicitly versioned schema-4 envelope at the existing authoritative
`.upskill-lock.json` location when this opt-in infrastructure is called. It
holds one `CombinedState` logical graph, complete desired skill intent, retained
revision captures, latest confirmed native binding receipts, applied skill-file
ownership and configuration contribution ownership. The embedded graph keeps its
existing schema-2 validation and legacy/origin semantics. Skill package
ownership is represented once, outside the logical graph, including captures
whose logical owner has detached. Unrelated graph artifacts and legacy data
remain unchanged. Do not create a separate authoritative skills lock.

Raw native prestate and configuration snapshots, including rollback baselines,
can contain unrelated manual settings. Never embed these bytes in the project
lockfile. Its schema-4 envelope stores content-addressed, typed references to
private evidence under a qualified user root. The evidence store has a local
namespace identity; parsing requires the expected namespace and root context, so
copied machine receipts do not become authority in another store. Evidence
contains no independently authoritative installation graph. The lockfile remains
the only selector of the current graph and current evidence references.

The pure codec returns lockfile JSON plus the required immutable private blobs;
it writes neither. Parsing requires every referenced blob, checks kind, length
and framed digest, then validates the reconstructed view. Missing/corrupt
private evidence rejects. The later qualified store must create private blobs
before committing their references, exclude them from tracked project content,
and retain referenced evidence. Private evidence uses mode `0600`; it does not
live in a committed project artifact. Native configuration ownership is
validated against a private captured document and exact per-entry target
associations.

The new codec must reject unsupported versions, unknown structural fields,
duplicate identities, copied scope roots, dangling selections, inconsistent
captures/packages and conflicting file ownership. Recompile captured packages
and check complete intent against the graph and confirmed native selections.
Validate applied file ownership separately from source permission evidence.
Selected skill captures are encoded as typed YAML documents inside the JSON
envelope, preserving supported YAML metadata structures and non-string mapping
keys without lossy JSON conversion. Custom YAML tags on additional metadata
values, such as `!fixture`, are not admitted by the portable source parser; this
codec adds no support for them. Decoded structural fields must survive typed
round trips; duplicate JSON keys reject before typed decoding. The codec
preserves the supplied logical graph and legacy ordering. Private evidence uses
the bytes `b"upskill:native-evidence:v1\0"`, including one trailing NUL byte,
followed by a big-endian u64 kind-byte length, the UTF-8 kind bytes, a
big-endian u64 payload-byte length and the exact bytes. References carry kind,
digest and length. Serialization and parsing establish no filesystem or native
authority.

The completed state stores confirmed outcomes only. Unknown/rejected outcomes
and planned inverses remain in operation intent until recovery resolves them.
Never serialize a desired preview and call it completed state. An operation may
commit state only after files and native selections are observed as intended.
Existing schemas are not migrated automatically, and #332 remains a production
admission dependency. The earlier inactive schema-3 proposal is not accepted by
the schema-4 reader; it had no representation for exports staged without a
confirmed native binding. Older readers must fail closed on this new envelope.

### Staging ownership and context transitions

Keep retained staging distinct from native-bound revision history.
`StagedSkillRevision` records one captured revision, a canonical operation ID
and a nonempty sorted subset of its immutable revision-file paths. Catalogs,
configuration selectors, duplicate captures and foreign paths reject. The single
applied ownership table includes those paths, validated against the recompiled
package and fixed file policy. A staging record proves no native activation,
unavailability or detachment; native-bound history still requires its existing
confirmed receipt lineage.

This permits retaining partial exports when restoring an operation that never
became active. Restore must independently confirm the prior native binding or
absence. Never manufacture a detachment receipt to retain staged files. A later
plan retains the owned subset using actual observations, requires cache/session
clearance before a transition involving retained staging and plans promotion
only for a matching candidate. Native observation remains required before
committing that promotion. No resource garbage collection follows from either
role.

The authoritative state records its declared additional roots, while an
operation can hold a larger set for a context transition. The integrated store
checks stable principal/private roots and private namespace, then requires every
declared root to be in the held lock set before loading its private evidence.
New state can expand or contract the declared set only within those held roots.
The pure codec still requires exact equality with its supplied expected context.
Recovery of an operation still requires the exact root bindings recorded in that
operation.

### Fixed private skill-file projection

Keep `SelectedSkill` and `SkillPackage` source modes and hash protocols
unchanged. The existing captured-only `plan_skill_delivery` remains an inactive
pure view. Add an explicit native-file planning entrypoint using the same graph,
selection and receipt checks with destination-mode projection before whole-file
planning. It consumes real captured destination observations; never normalize
observations to pretend that applied modes equal source modes.

For native skill exports, policy `native-skill-files-v1` is fixed: generated
files use `0600`; resources use `select_file_mode` with `OwnerExecutable`,
giving `0700` when captured owner execution is present and `0600` otherwise.
Unknown resource modes reject. No source file is executed. New skill export
paths cannot adopt manual files, and existing managed exports must retain their
recorded modes. A mode edit is drift, including on a no-op. Configuration edits
preserve existing ordinary modes and create private files under AD-0043.

This fixed policy introduces no user-selectable execution toggle for an existing
immutable revision. A later policy change must change payload identity and pass
new qualification before existing paths can be reused. Source-mode evidence
remains part of immutable package identity; applied ownership records actual
selected modes. All retained revisions use the same admitted policy.

### Multi-root durable operation intent

Acquire all qualified writer locks in deterministic physical-root order before
prestate capture or effects. Reject aliased/duplicate roots and verify each
selected binding. Lock contention returns without effects. Hold every guard
through completion or the recovery-required return. Advisory locks do not
exclude manual editors or establish cross-root atomicity.

Use one private operation journal and pending markers in every affected root.
Each marker binds operation ID, primary journal location and exact root
identity. Publish and sync intent and every marker before staging files or
invoking a native mutation. Read-only adapter capture happens under the locks
before intent. A later operation sharing only a secondary root must see the
marker and require recovery, even after the original process has exited. A
partial marker publication has no external effects and can only be reconciled
under all locks. Use qualified descriptor-based primitives and existing
metadata/link/mount controls; do not add path-only journal writes that bypass
those controls.

Intent contains the exact prior state, intended graph/intent/revisions, all
staging/selector/configuration file preconditions, native prestate, planned
inverse, profile attribution, resource-retention evidence and ordered
checkpoints. The journal is recovery evidence for one operation, not a second
installation lock. Durably record pending native effects before invoking them,
and retain uncertain outcomes after timeouts or exceptions. Apply inactive
staging first; change selectors and native bindings under their qualified
transition profile; observe both; only then commit the integrated state. Release
markers only after durable state and completion evidence are verified. Retain
old revisions; this stage authorizes no resource garbage collection.

### Recovery and completion

`NativeDeliveryAdapter` supplies capture, apply, observe and inverse operations
with distinct Confirmed, Rejected and Unknown outcomes. A successful process
exit alone proves no binding. Adapter observations bind operation, target,
client version, platform, scope and exact selection. Require native cache
preservation or session clearance before refresh/detach, independently of intact
exports.

Recovery opens the same qualified roots and observes before acting. An unknown
outcome never authorizes automatic replay. If observed state matches the
intended poststate and all file preconditions/ownership agree, recovery may
finish the recorded forward operation. A confirmed before-state requires an
explicit recovery disposition before retry or inverse. Manual divergence, failed
inverse and unverifiable retention remain `RecoveryRequired`; preserve evidence
and manual bytes. Never use file-only rollback to claim native recovery.

`Complete` requires observed files, confirmed native outcomes, durable
integrated state and terminal journal/marker evidence. Every other interrupted
or uncertain case returns `RecoveryRequired`. Tests must use a real durable
store and fresh processes at interruption boundaries, in addition to fake
adapter outcomes.

## Implementation and admission

Current implementation: pure permission projection, the validated codec and live
multi-root writer exclusion exist. The codec emits schema-4 JSON plus private
evidence blobs; it performs no I/O, migration or publication.
`native_delivery::roots` binds exact logical root paths to revalidated directory
identities, rejects physical aliases and acquires nonblocking writer guards in
ascending device/inode order. Failure releases every acquired guard. These
advisory locks release on process exit; they provide no pending markers or
post-crash exclusion. Filesystem-profile admission remains separate from those
locks.

`native_delivery::store::NativeEvidenceStore` now stores immutable typed blobs
under `.upskill/native-evidence` in an explicitly qualified user root. It
borrows the matching writer guard, reuses descriptor-based execution primitives
and qualifies the existing filesystem profiles, mount binding, metadata and link
controls. Its owned directory uses mode `0700`, its files use `0600`, and its
ignore file excludes all entries. Tracked evidence rejects, including when an
ancestor alias reaches a repository. Root and directory identities plus a random
local nonce bind `owner.json`; a domain-separated digest of those exact owner
bytes supplies the expected namespace. Copied, unowned or partially initialized
stores require reconciliation. Equal blobs are verified and synced; conflicting
bytes remain untouched. No evidence garbage collection is supplied.

`store_state_evidence` validates supplied state, syncs all required private
blobs and returns reference-only JSON. `load_state` checks the expected private
root/namespace and envelope declarations before reading exactly the referenced
blobs, then performs complete codec validation. Unreferenced evidence remains
untouched and is not another source of graph authority.

`NativeStateStore` holds the complete root-lock set, derives context from its
primary/private roots and orders additional logical roots independently of
physical lock order. It captures only the existing `.upskill-lock.json` as the
authoritative graph. `prepare_state_file` rejects unsupported prior state,
validates the supplied context, syncs private evidence and returns an exact file
precondition plus JSON. New state files select `0600`; existing ordinary modes
are preserved. It verifies captured state and all root bindings before returning
and never applies the state action. Principal/additional roots supplied directly
to the lower evidence API remain claims; the integrated wrapper binds them to
the held root set.

The state APIs do not publish the authoritative lockfile. Interrupted
blob-publication tests establish process restart behavior, not power-loss
durability or native recovery.

`NativeOperationStore::prepare` now writes a typed YAML schema-2 intent and an
initial JSON journal into the qualified private evidence directory. The intent
retains exact prior state, captured graph edits, selected revisions, native
observations, configuration snapshots/associations and the replayable
skill/configuration plans. YAML preserves supported non-string source metadata
keys. Strict typed round trips and replay reject a different preview even when
its stored hashes are internally consistent. The intent digest uses
`b"upskill:native-intent:v1\0"`, a big-endian u64 byte length and the exact YAML
bytes. The initial journal has generation zero and status `Prepared`.

Each held root receives `.upskill/native-pending/pending.json` after intent and
journal synchronization. This separately owned directory uses `0700`, its files
use `0600`, and its ignore file excludes all entries. Its owner record requires
purpose `native-pending-v1`; it cannot adopt an evidence directory. Marker
namespaces use `b"upskill:native-marker-namespace:v1\0"`, then a big-endian u64
owner-byte length and exact owner bytes. Existing evidence owner bytes and
namespace framing remain unchanged. Markers contain only the operation ID,
physical/logical root binding, marker namespace, private journal locator,
private namespace and intent digest. They contain no captured native/config
snapshot bytes.

Preparation scans all held roots for pending markers before creating marker
storage elsewhere, recaptures file observations under the locks and verifies
captured files and root bindings before returning. A new operation sharing only
a marked secondary root requires recovery, without opening that prior journal
through an unheld root. `inspect` requires exactly the recorded roots, validates
intent/journal bindings and reports missing markers without retry, cleanup or
application effects. Marker coverage is metadata visibility, not a future effect
lease or native completion. Fresh processes exercise interruption after intent
and after every marker, including shared-root exclusion after process exit.

The store does not stage application files, invoke a client, publish installed
state or retire markers/evidence. Native effect profiles, ordered effect
checkpoints, inverse execution and the observation/recovery coordinator remain
unimplemented. Stage 5 is open.

Implement the permission projection first, then the validated state codec,
qualified multi-root store and coordinator, then native adapters. These are
substeps of Stage 5; no substep alone closes it. Exercise crashes around intent,
markers, file effects, native effects, observations and final commit, including
shared secondary-root contention after a crash. No CLI or four-client release
admission follows from these infrastructure tests.
