# Owned instruction operation planning

Date: 2026-10-12. Status: **Ready for bounded implementation**.
Baseline: `f7d36bf`.
Parent direction: [M1 owned lifecycle](2026-10-12-m1-owned-lifecycle-design.md).

## Outcome and boundaries

Join the existing aggregate editor and validated ownership model into a pure
add/update/remove planner. A future lifecycle supplies resolved installation
changes and file observations. This task establishes no source verification,
reference-graph authority, native compatibility, filesystem observation or
permission to write. Keep CLI behavior, schemas, defaults and renderers unchanged.

The lifecycle resolves direct/bundle references before asking to remove an
installation owner. The planner protects owners outside its explicit changes;
it does not infer bundle-reference counts from artifact records. An installation
change replaces that installation's complete contribution set, or removes it.

## Public contract

Add `instructions::plan::plan_instructions(prior, changes, observations)` returning
an `InstructionOperationPlan` through `anyhow::Result`. Inputs are borrowed slices;
the planner returns owned data without modifying them.

A location contains an existing `InstallationRoot` and portable relative path.
An observation contains a location and optional exact byte content (`None` means
absent). A set change contains an `ArtifactOwner` and one or more desired
contributions, each with location, ID, Markdown and consuming surfaces. A remove
change contains an installation ID. The plan contains the complete resulting
artifact records and ordered file plans with exact before bytes and after bytes.
Shared aggregate file plans always retain a file, even when after bytes are empty.

Validate all prior records with the existing validator. Reject duplicate change
IDs, empty set changes, unknown removals and changes that reinterpret an existing
ID's source/kind/source-name/alias binding. A set may change the recorded revision.
Validate final records using the existing validator as well.

Use exact lexical root/scope/path identities, consistent with current ownership
validation. Do not normalize paths, inspect case/aliases or claim containment.
Each affected location needs exactly one observation; duplicate or extraneous
observations conflict. No changes requires no observations and preserves prior
records. Invalid desired locations, IDs, owners or consumers must fail, including
when a malformed desired location would otherwise become empty after editing.

## Editing and ownership

A contribution affected by a changed owner must have exactly one prior owner.
Reject affected whole files and multi-owner contributions explicitly. Do not
silently drop unsupported records; unrelated records of either form survive.
The multi-owner restriction avoids guessing each owner's consumers from a stored
union. It does not restrict multiple separately owned contributions in one file.

An unchanged owner's contribution cannot be taken over by reusing its selector,
even with identical Markdown. Duplicate desired selectors at a location conflict.
Move/update/removal of a selected owner's contributions is allowed when all old
regions verify. New selectors in existing manual text are allowed, but unrecorded
markers never become owned. Non-UTF-8 observed aggregates conflict.

Expose `aggregate::inspect_owned(existing, prior)` to read verified existing
contribution bodies, using the same marker/span validation as `reconcile`.
Refactor shared private validation rather than introduce a second marker parser.
Return bodies in document order. Reject duplicate/overlapping, missing, edited or
unrecorded markers using the existing editor's contract.

At each affected location, retain verified bodies for untouched selectors,
combine desired replacements and call `aggregate::reconcile` once for the edit.
Rebuild returned contribution metadata from that edit, including for untouched
selectors whose separator context changed. Preserve their bodies, owners and
consumers. Unaffected-location records remain exactly unchanged.

Output file plans sort by scope, raw root spelling and portable path. Resulting
artifact rows retain prior relative order where selectors survive; append new
rows sorted by location/selector. Desired owner/consumer data is validated and
preserved; do not sort caller inputs in place. Equal inputs yield equal results.
File plans include unchanged-byte locations touched by an ownership operation,
so a future executor can still recheck their exact preconditions.

## Acceptance examples

1. Start with manual bytes and add two separately owned regions. Update or remove
   one, retaining the other's body/ownership and exact manual bytes. Recomputed
   separator metadata must support a subsequent removal restoring manual bytes.
2. An explicit owner replacement removes its omitted old regions and adds/moves
   desired ones; another installation at the same location survives.
3. Edited/missing regions, unknown markers, invalid UTF-8, duplicate changes or
   selectors, invalid locations/owners/consumers, missing/duplicate/extraneous
   observations and unknown removals all fail without partial output.
4. Whole-file and multi-owner records conflict only when affected. Unrelated
   records survive; no source, client, dependency or legacy record is invented.
5. A no-op content update still yields exact before-state bytes. Last-region
   removal leaves empty file content rather than a deletion action.
6. Different lexical roots remain distinct; input vectors and observations are
   unchanged; ordering is deterministic. Reader and planner acceptance tests use
   actual public APIs and independent literal expected values.

## Delivery constraints

- Preserve Rust 1.85 and the synchronous/static-binary constraints.
- Add no dependencies, CLI changes, codecs, schema-2 writes or native operations.
- Only main.rs writes to stdout/stderr; planner and reader perform no I/O.
- Preserve existing aggregate editor behavior and unrelated worktrees/edits.

Run behavioral assertion RED after compilation is available, then GREEN and
focused semantic mutations that prove owner preservation and before-state
preconditions. Finish with ordered `just fmt`, `just verify`, minimum-compiler
checking and independent task/final/PR review. This task does not complete M1-A2,
M1-A3 or the mixed-state activation gates.
