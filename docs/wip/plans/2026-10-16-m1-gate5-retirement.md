# Gate 5 shared-owner retirement implementation plan

> Use superpowers:executing-plans for inline execution. The user delegated
> continuing implementation without another planning approval checkpoint.

**Goal:** Plan explicit reference removals over complete state, preserving
surviving shared owners and requiring observed content before any output change.

**Architecture:** Pure removal-only lifecycle boundary: reconcile graph first,
filter owners and authoritative consumer associations, then plan each affected
physical location once. Aggregate edits preserve manual bytes and retained
regions; whole-file deletion requires the final owner to retire. Return complete
validated combined state plus file actions. No I/O, native sharing
establishment, new owners, CLI activation or executor effects.

**Spec:**
[Gate 5 lifecycle integration](../specs/2026-10-15-m1-gate5-lifecycle-integration.md)
and [M1 owned lifecycle](../specs/2026-10-12-m1-owned-lifecycle-design.md).

## Constraints

- Existing mixed/instruction planner rejection remains unchanged. Only rule
  installations may retire; unselected unsupported kinds remain unchanged.
- Only explicit reference removals; no installation upserts or new sharing.
- Validate prior state and every supplied observation before planning.
- Affected locations include metadata-only shared-owner retirement. Require
  exactly one present observation, verify owned bytes/markers and recorded
  whole-file mode, and reject duplicate, extraneous, missing, malformed or
  alias-spelled evidence.
- Retain owner order, union consumers from surviving per-owner associations,
  preserve tagged legacy data exactly, and validate the final complete state.
- Whole files disappear only when their last owner retires. Aggregate files
  remain present even when empty; preserve observed mode and manual bytes.
- Referenced installations with no output remain. No-op graph edits do not
  invent affected files. Input state and observations remain unchanged on error.
- No filesystem qualification or deletion authority beyond explicit returned
  individual file actions. Later guarded executor must revalidate captures.

## Review focus

1. Different owners contribute different surfaces to one region: removing one
   owner recomputes the exact union without deleting surviving content.
2. Metadata-only retirement rejects edited owned bytes and changed recorded
   whole-file mode; aggregates preserve valid observed mode (no prior mode is
   stored for individual regions).
3. Two retired installations in the same aggregate require one observation and
   one aggregate edit, preserving a third untouched region and manual text.
4. Empty installations can retire without artifact actions; referenced empty
   installations survive. Bundle diamonds delay owner retirement.
5. Invalid evidence or final-state association errors return no partial plan;
   tagged legacy duplicates and invalid source labels remain untouched.

## Interfaces and files

New src/lifecycle/retire/mod.rs and private files.rs; expose lifecycle::retire.
Public ReferenceRemovalPlan { state: CombinedState, files: Vec<PlannedFile> }.
plan_reference_removal(prior: &CombinedState, remove: &[Reference],
observations: &[ObservedFile]) -> anyhow::Result<ReferenceRemovalPlan>. New
tests/lifecycle_retirement.rs; design/specification and validation records.

## Task 1: Reference-driven shared-owner removal planning

- [ ] Write public acceptance examples before implementation; observe missing
      interface RED.
- [ ] Reconcile remove-only GraphChanges, index retired IDs and affected raw
      locations. Preserve graph output and typed legacy snapshot.
- [ ] Validate observation syntax/modes and exact affected membership; reject
      topology overlaps involving affected paths before file planning.
- [ ] Derive retained owners and per-owner consumer union from validated prior
      state, retaining row order and all untouched rows.
- [ ] Verify whole-file hash/mode; emit unchanged bytes for partial retirement,
      absent after-value for final retirement.
- [ ] For each aggregate, inspect all prior regions, invoke aggregate::reconcile
      once with retained bodies, update retained region metadata and preserve
      observed mode. Final empty aggregate remains present.
- [ ] Validate complete final CombinedState and return no partial success.
- [ ] Cover shared whole files, shared regions, distinct regions sharing a file,
      edited/missing bytes, marker tampering, mode changes,
      extra/duplicate/missing observations, raw root differences, invalid prior
      associations, empty installations, retained graph paths and tagged legacy
      preservation.
- [ ] Run focused tests and assertion-failing fault controls; restore source.
- [ ] Update scoped records; full fmt/verify/MSRV; commit; final independent
      implementation review; fix quick wins; garden only this completed plan.
