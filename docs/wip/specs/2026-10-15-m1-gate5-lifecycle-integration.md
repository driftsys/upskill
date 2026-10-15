# M1 gate 5: combined owned-content lifecycle

Date: **2026-10-15**. Status: **scope accepted for planning; gate 4 merged in PR
#293**. This task scopes gate 5 in the
[M1 owned-lifecycle design](2026-10-12-m1-owned-lifecycle-design.md) and the
[M1 delivery plan](../plans/2026-10-11-instructions-rules-m1.md).

## Problem

The merged prerequisites provide pure ownership and mixed-artifact planners,
captured compilation, qualified observations and locking, permission selection,
and qualified file application and checkpointed recovery. PR #293 merged these
executor contracts as `a5e28b8`; hosted Linux, macOS and Windows checks passed.
The production command lifecycle still uses schema 1.

Gate 5 connects these pieces through one combined state and lifecycle path.
Unresolved schema-1 history stays tagged alongside explicitly owned artifacts.
The lifecycle consumes the reviewed executor rather than introducing another
writer or recovery protocol.

## Goal

Implement one ownership-aware state and operation path for `add`, `update`,
`remove`, `list`, and `doctor`, including preview, mixed legacy/new state,
qualified file application, and interruption recovery. Keep the new write path
inactive until gate 6 has passed its Codex adapter qualification.

## Scope

- Persist owned artifacts and tagged unresolved legacy records in one schema-2
  lockfile. Preserve each legacy row and its source label exactly.
- Connect captured source compilation and complete desired sets to the mixed
  owned-artifact planner. Establish ownership only from an explicit new
  installation; never infer ownership from a schema-1 source digest.
- Integrate all five lifecycle commands with shared preflight, qualified scope
  locking, exact before-state checks, and typed operation outcomes.
- Complete the compiler-to-lifecycle desired-set connection, authoritative
  reference graph, and per-owner consumer association required by the M1
  lifecycle design.
- On the first schema-2 write, create and verify a restricted, non-overwritten
  byte-exact backup of the observed schema-1 lockfile before any output effect.
- Apply only unchanged owned files or contribution regions. Preserve unrelated
  owners, manual bytes, unselected legacy records, and individually enumerated
  extra resources. Never use recursive legacy output removal.
- Persist and recover ordered file and state checkpoints. An interrupted or
  stale operation must not repeat an unknown effect or discard unrelated edits.
- Keep list, doctor, and preview observational: no migration, backup, source
  fetch, journal, or state rewrite.
- Represent every current content kind in the combined state. Apply owned
  effects only to M1 rule and instruction artifacts; preserve other kinds as
  tagged legacy rows. If an operation selects unsupported legacy work, report it
  and stop the affected operation without falling back to a schema-1 writer.

## Out of scope

- Enabling schema-2 writes in production commands. Gate 6 must qualify the Codex
  instruction adapter and selector first; until then commands stay on schema 1
  and the ownership-aware write path remains inactive.
- Codex instruction selection, client-native discovery claims, and scoped-rule
  semantics. These remain in the later M1 adapter gates.
- Migrating, adopting, or deleting legacy outputs based on source hashes,
  inferred refs, or path names. Legacy adoption requires a later explicit,
  evidence-based operation.
- Migrating MCP or plugin registrations. Preserve them as tagged legacy rows.
- Changes to #284 or the shared updater implementation.

## Acceptance criteria

1. **New ownership beside legacy history:** adding an explicitly selected item
   creates owner records while preserving unrelated schema-1 rows in the tagged
   legacy snapshot. No legacy output is adopted.
2. **Exact first-write backup:** before the first schema-2 output effect, the
   backup bytes equal the observed schema-1 lockfile. A mismatched existing
   backup blocks the operation without changing state or outputs.
3. **Manual-content preservation:** adding, updating, and removing owned
   contributions preserves manual bytes and every untouched contribution.
   Editing an affected owned region or whole file produces a conflict with no
   partial write.
4. **Complete selection preflight:** selecting ambiguous legacy work together
   with owned work reports the conflict before any effect. Explicitly disjoint
   owned work can proceed while unrelated legacy rows remain byte-identical.
5. **Safe removal:** removal changes only unchanged owned artifacts with no
   remaining reference. Extra resource files, edited outputs, and unowned
   directories survive; deletion enumerates individual owned files.
6. **Read-only inspection:** `list`, `doctor`, and preview preserve exact state
   bytes and create no backup, migration, journal, or output change.
7. **Stale-plan and writer rejection:** changed state, source, or destination
   evidence after planning rejects application. A second upskill writer cannot
   apply concurrently in the same qualified scope.
8. **Checkpoint recovery:** interruption at every output and final-state
   checkpoint recovers without duplicating unknown effects, losing retained
   records, or overwriting unrelated edits.
9. **Tagged-row preservation:** each lifecycle operation retains unrelated owned
   references and all tagged native or legacy rows. Unsupported or malformed
   rows remain visible and are never silently pruned. Operations requiring
   unsupported selected legacy work stop before effects; disjoint owned work can
   proceed.
10. **Inactive production boundary:** tests exercise the ownership-aware
    operation path, but no production command writes schema 2 until the Codex
    adapter/selector gate is complete.

## Design constraints

- Use the existing `operations`, `ownership`, `compile`, `filesystem`, and
  `recovery` contracts; do not introduce an independent ownership sidecar.
- Keep source schema independent from lockfile schema. Preserve historical
  source labels and pins in tagged legacy data.
- A conflict affecting selected work prevents partial effects for that
  operation. An unrelated unresolved legacy row does not block a disjoint owned
  operation.
- Filesystem and native outcomes remain distinct. Successful file application
  does not imply native registration, trust, authentication, or discovery.
- Limit supported owned effects to the M1 instruction and rule artifacts
  explicitly represented by the qualified planners. Preserve every other current
  content kind as tagged legacy data.

## Dependencies and evidence

This task consumes `operations::plan`, `ownership`, `compile::item`, qualified
source and destination observations, and `QualifiedScope::apply_operation` /
`recover_operation` with their matching writer guard. See
[qualified file execution](../../design/qualified-file-execution.md) and
[AD-0033](../../adr/0033-qualified-file-execution-and-journals.md).

## Implementation direction

- Add an opt-in library lifecycle boundary. Keep `main.rs`, existing command
  dispatch and `lockfile::CURRENT_SCHEMA` on their current production path. Gate
  5 acceptance calls the same library adapters that Gate 6 will activate.
- Use one schema-2 envelope containing installation bindings, direct and bundle
  references, per-owner consumers, owned artifacts and tagged legacy data.
  Validate reference targets and ownership bindings before planning effects.
  Deterministic serialization and strict schema-2 validation are required; exact
  Rust types and identifier encoding are builder decisions with collision tests.
- Preserve the indexed typed legacy snapshot, including duplicates and native
  status. Preserve unknown original JSON fields through the exact schema-1
  backup; do not claim that the typed projection itself retains those fields.
- Prepare complete desired sets from captured compiler results and explicit
  selections. Reconcile the authoritative graph before calling the mixed
  planner. Affected shared contributions need per-owner consumer association; do
  not weaken the existing planner's rejection without that evidence.
- Separate observational inspection and preparation from guarded application.
  Preview consumes supplied captured evidence and never fetches; applying source
  resolution is explicit orchestration. Return typed diagnostics and outcomes,
  leaving presentation in `main.rs`.
- Include the exact first-write backup as a guarded, create-only prerequisite
  before executor output effects. Never overwrite it. Recheck its bytes on
  retry/recovery. Any executor extension required for this prerequisite must
  retain the existing checkpoint protocol and receive focused review.
- Reuse qualified executor recovery for file and final-state actions. Lifecycle
  metadata must be reconstructible from the persisted operation; no independent
  sidecar becomes authoritative. Inspection reports pending recovery without
  performing it.

## Delivery slices

Implement in dependency order, each with acceptance examples and a reviewable PR
containing code and documentation:

1. Combined state codec, authoritative references and schema-1 projection.
2. Compiler-to-desired-set reconciliation and pure lifecycle selection/planning.
3. Guarded backup, executor integration and interrupted lifecycle recovery.
4. Read-only list/doctor/preview adapters and complete lifecycle acceptance.

Gate 5 is complete only when all four slices pass together. Slice completion
never enables schema-2 production writes or claims native target qualification.

The task is complete only after the full acceptance suite, `just fmt`,
`just verify`, Rust 1.85 evidence, hosted CI, and independent task/final review
pass. Completion prepares gate 5; it does not claim that M1 is activated or that
#284's prerequisite has passed until gate 6 is also complete.
