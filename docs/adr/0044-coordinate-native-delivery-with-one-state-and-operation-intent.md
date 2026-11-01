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

Use an explicitly versioned schema-3 envelope at the existing authoritative
`.upskill-lock.json` location when this opt-in infrastructure is called. It
holds one `CombinedState` logical graph, complete desired skill intent, retained
revision captures, latest confirmed native binding receipts, applied skill-file
ownership and configuration contribution ownership. The embedded graph keeps its
existing schema-2 validation and legacy/origin semantics. Skill package
ownership is represented once, outside the logical graph, including captures
whose logical owner has detached. Unrelated graph artifacts and legacy data
remain unchanged. Do not create a separate authoritative skills lock.

The new codec must reject unsupported versions, unknown structural fields,
duplicate identities, copied scope roots, dangling selections, inconsistent
captures/packages and conflicting file ownership. Recompile captured packages
and check complete intent against the graph and confirmed native selections.
Validate applied file ownership separately from source permission evidence.
Serialization and parsing establish no filesystem or native authority.

The completed state stores confirmed outcomes only. Unknown/rejected outcomes
and planned inverses remain in operation intent until recovery resolves them.
Never serialize a desired preview and call it completed state. An operation may
commit state only after files and native selections are observed as intended.
Existing schemas are not migrated automatically, and #332 remains a production
admission dependency. Older readers must fail closed on this new envelope.

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

Current implementation: only the pure permission projection exists. The schema-3
codec, qualified multi-root store and durable coordinator specified here remain
unimplemented; Stage 5 is open.

Implement the permission projection first, then the validated state codec,
qualified multi-root store and coordinator, then native adapters. These are
substeps of Stage 5; no substep alone closes it. Exercise crashes around intent,
markers, file effects, native effects, observations and final commit, including
shared secondary-root contention after a crash. No CLI or four-client release
admission follows from these infrastructure tests.
