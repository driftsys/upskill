# M1 owned-content lifecycle design

Date: 2026-10-12. Status: **Proposed implementation contract**.
Baseline: `f7d36bf` (PR #258).
Authority: [foundation](2026-10-11-agent-customization-foundation-design.md)
and [M1 delivery plan](../plans/2026-10-11-instructions-rules-m1.md).

## Intent and current boundary

Deliver instruction installation for Claude Code, Copilot and Codex while
preserving manual content and existing OpenCode behavior. The user delegated
routine design and delivery decisions and the continuing review/merge loop.
This document selects the operational direction and decomposes its prerequisites;
it does not claim a schema-2 reader, writer, executor or Codex CLI flag exists.

The merged library supplies target identities, a pure Markdown contribution
editor, validated owned-artifact records and a pure legacy-state projection.
Current command lifecycles still read and write schema 1. None of the library
records establishes filesystem ownership by itself.

## Alternatives and selected direction

1. Verify and adopt every legacy installation before new owned installations.
   This delays new instructions behind unavailable original revisions and renderer
   versions. A legacy SSOT digest cannot recover either fact. Reject as the
   universal prerequisite; retain verified adoption as a later explicit operation.
2. Preserve unresolved legacy records alongside new, explicitly established
   ownership. Select this direction. It permits disjoint new installations while
   keeping unresolved history inspectable and blocking operations that need it.
3. Keep separate writable legacy and ownership stores indefinitely. Reject as the
   activation architecture: independent writers could lose references or use
   legacy recursive removal against newly owned destinations.

Until the activation gates below pass, keep current commands on schema 1 and all
new planning machinery inactive. Do not expose an intermediate installation mode
that mixes ownership-aware writes with legacy recursive removal.

## State and identity contract

The eventual schema-2 state contains installation roots, explicit requested
surfaces, installation/item identities, direct and bundle reference edges, owned
artifacts, operation outcomes and a tagged legacy snapshot. Source schema remains
independent. Exact Rust types and serialized fields belong to bounded slice plans;
this document is not a speculative wire-format schema.

Preserve every current typed schema-1 field, row, duplicate and integration status
inside the tagged legacy representation. Preserve original source labels and
recorded refs independently. Parsing a label does not verify a source revision.
Unknown raw JSON fields and original serialization bytes are not recovered by a
typed snapshot; the first-write backup preserves those bytes separately.

An installation identity binds its source, kind, source name and installed alias.
Requested surfaces and resolved source revisions can change on update without
accidentally creating a different reference graph. Bundle membership and direct
installation roots are references, not extra copies of an artifact. Any chosen
identifier algorithm must detect collisions rather than silently merge entries.

A new Git installation records the actually resolved immutable revision and its
source-content evidence. A local installation records a strict captured source
manifest, without fabricating a Git revision. Source observation must fail on
unreadable files; the current legacy digest is not that verification primitive.
No decision here allows automatic adoption from a legacy digest or parsed ref.

New ownership requires an explicit planned source/target binding and observed
artifact preconditions. An existing unmarked dedicated output conflicts, even
when its bytes match. A shared instruction aggregate may contain manual text:
only new marked contributions become owned, and all other bytes stay user-owned.

The native surface set and default selection remain the foundation's fixed
contract. An explicit Codex selector is enabled only with its instruction adapter
and complete lifecycle. Incidental discovery is exposure metadata, not an owner
or an additional retention reference.

## Mixed-state operation contract

Resolve the operation's selected installations and references before effects.
Report all relevant conflicts together. A conflict in selected work prevents its
application; do not perform an owned subset and then report a selected legacy
operation as failed. Explicit name selection can isolate disjoint owned work.

| Operation | New owned state                                                                                            | Retained legacy state                                                                                                                    |
| --------- | ---------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Add       | Establish explicit owners and references after source, capability and destination validation               | Preserve existing rows; reject same installed identity or a known conflicting destination rather than replacing it                       |
| Update    | Resolve the foundation's flag/config/history precedence, plan ownership changes, preserve untouched owners | Inspect and preview without adoption; an applying operation needing unresolved selection or ownership stops with reconciliation guidance |
| Remove    | Remove selected references; edit or delete only unchanged owned artifacts with no remaining reference      | Preserve unowned output and state when removal needs unknown ownership; do not call recursive legacy output removal                      |
| List      | Show installations, references, requested surfaces and ownership facts                                     | Show original indexed rows and unresolved diagnostics; no fetch, writes or adoption                                                      |
| Doctor    | Observe owned bytes/selectors and report changes, missing files or unresolved recovery                     | Remain observational; report known candidates and unresolved facts without treating source hashes as installed-byte evidence             |

An unresolved row does not block a disjoint operation merely because it exists.
The planner must establish the identities and destinations its operation needs;
when that cannot be established, it reports that specific missing fact. Unknown
or malformed legacy data is retained, never pruned as a cleanup side effect.

All current kinds must remain representable by the new lifecycle before general
activation, including dedicated files and individually enumerated resources.
Using the current renderer does not establish new native conformance. New Codex
support initially covers the M1 instruction contract; unsupported categories or
required semantics receive preflight diagnostics, never silent omission.

MCP/plugin records stay tagged legacy data until their category migrations.
Retain the foundation's explicit-removal policy and report retained integrations
when content selection narrows. Native effects require explicit typed operation
outcomes; file transaction success cannot be used as proof that registration,
trust, authentication or discovery succeeded.

## Pure instruction planning boundary

The next bounded implementation joins the existing contribution editor and
ownership records into operation plans. It receives caller-supplied observations
and explicit installation changes; it does not verify the filesystem or fetch
sources. It returns deterministic desired records, exact before-state
preconditions and per-location desired content, or contextual conflict errors. The first pure slice may stop at its first
conflict; complete operation-level diagnostic aggregation remains an activation
gate for the lifecycle coordinator.

Untouched installations retain their owners and contributions. A caller cannot
remove another owner's reference by supplying an incomplete replacement list.
Changing a contribution shared with an untouched owner conflicts unless that
owner's content contract remains unchanged. Equal rendered bytes alone do not
establish compatible native behavior; adapter validation precedes sharing.

The first planner supports changes to single-owner contributions. Affected
multi-owner contributions require authoritative per-owner consumer data, which
the current union-only artifact record cannot supply; reject those changes
explicitly rather than guess. Unaffected multi-owner records remain intact.
Later state integration must supply the missing association before claiming
shared-contribution reconciliation.

Group edits at a location and invoke the existing aggregate editor once. Preserve
manual bytes, reject edited or unrecorded markers, and retain the editor's full
hash/newline metadata. Non-UTF-8 aggregate content is a conflict, not a lossy
conversion. Inputs and outputs do not promise canonical roots, containment or
case/alias equivalence; an executor must establish those facts.

Preserve an empty shared aggregate after removing its last contribution. The
existing ownership record does not prove the aggregate file was created by
upskill, so an empty result alone cannot authorize deleting that file. Dedicated
whole-file removal and resource enumeration need their separate planning slice.

Plans include before-state preconditions even when only ownership changes and
file bytes stay identical. They do not authorize mutation or expose a serialized
user-editable script. Reject unsupported affected artifact forms rather than
silently reinterpreting them; preserve unrelated records.

## Executor and recovery contract

The executor establishes concrete roots and filesystem-equivalent destinations,
rejects traversal, symlink escapes and aliases, and checks all selected source,
state and destination preconditions under one scope writer lock before the first
write. Recheck each destination immediately before its action. A later external
edit can still interrupt a multi-file operation; the journal must report the
applied prefix rather than claim cross-file atomicity.

Use atomic per-file replacement where qualified and preserve permissions. Never
remove resource directories recursively because their name matches an item.
Deletion operates on individually owned, unchanged files; extra user files stay.
Source, root and resource enumeration must supply containment evidence before
these actions become eligible for application.

A journal under `<scope-root>/.upskill/state/` records operation identity,
expected old/new state hashes, action identities, pre/post hashes, owned
instruction values and checkpointed outcomes. It does not retain credential
values or backup copies of unrelated user configuration. Necessary atomic-write
temporary files are restricted transient output, not a general rollback archive.
The state directory supplies its own generated ignore file.

Prefer forward recovery: compare each current observation with its recorded pre
and post states. Pre-state permits reconstructing the planned owned edit; post
state permits recognizing an already-applied action. A different state requires
reconciliation and remains untouched. Do not blindly repeat an unknown native
operation. The final state write is a checkpointed action; a crash immediately
before or after it must not duplicate an installation or lose retained records.

A scope lock coordinates upskill writers and recovery, not external editors.
Its eventual implementation must work on Rust 1.85 and the supported local
filesystems. A dependency choice requires an ADR, minimum-compiler evidence,
cross-process contention/release tests and binary-size measurement. This document
selects no dependency and makes no network-filesystem locking promise.

Before the first schema-2 write, retain the exact schema-1 bytes in a restricted,
non-overwritten backup and verify the backup matches the observed source state.
Reading, listing, doctor and preview never create a migration backup or rewrite
state. Restoring the backup alone cannot undo output or native operations; do not
offer downgrade without a verified inverse plan.

## Activation gates and delivery order

Each implementation task gets its own isolated worktree, acceptance-first tests,
code/docs together, task and final review, repository verification, PR review,
hosted CI and squash merge. Later tasks consume explicit reviewed interfaces.

1. Pure installation/reference-aware instruction planning, using the existing
   editor and ownership model; no new CLI or persistence.
2. Dedicated-file/resource planning and strict source observation, with per-file
   evidence and compatibility checks for all existing content kinds.
3. Qualified filesystem observation, destination equivalence/containment and
   scope-lock implementation; dependency/OS evidence before claims.
4. File apply/recovery with interruption tests at every action/state checkpoint
   and preservation of manual bytes; no native-success inference.
5. Combined state and all five lifecycle adapters, including tagged native
   records, preview, exact migration backup and no legacy recursive removal.
6. Codex unconditional instruction adapter and selector, native target qualification,
   fixed defaults and documented incidental readers. Scoped rules remain M1-C.

Do not turn a completed prerequisite into a claim that M1-A2, M1-A3 or M1 is
complete. Schema-2 file writes begin only after all lifecycle paths can preserve
the state; activation additionally requires qualified apply/recovery and adapter gates.

## Acceptance examples

| ID  | Example and required result                                                                                                                  |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| L01 | Add two owned contributions to manual text; update/remove one preserves the other and all manual bytes                                       |
| L02 | Remove one of two references; shared content survives; changing content for an untouched owner conflicts                                     |
| L03 | Edited region, unknown marker, invalid UTF-8 or affected unsupported artifact form yields a conflict with no partial plan                    |
| L04 | New owned install beside an unrelated unresolved legacy row preserves that row; selected ambiguous legacy update/remove performs no effects  |
| L05 | Bare update selecting owned and blocked legacy work reports the complete conflict before any write; named disjoint update can proceed        |
| L06 | List, doctor and preview preserve exact state bytes and create no backup/journal/migration                                                   |
| L07 | First migration backup matches exact original schema-1 bytes; a pre-existing different backup conflicts and is not overwritten               |
| L08 | Extra resource files and edited owned files survive removal; no name-based recursive deletion                                                |
| L09 | Changed state/source/destination after preview rejects the plan; a second upskill writer cannot apply concurrently                           |
| L10 | Interrupt every file/state checkpoint; recovery recognizes pre/post states, preserves unrelated edits and does not duplicate unknown effects |
| L11 | Schema-2 add/update/remove/list/doctor retain all tagged legacy/native rows and all unrelated owned references                               |
| L12 | Explicit Codex instructions use qualified native roots; implicit defaults remain unchanged and incidental discovery is reported              |

## Evidence limits

This design records future implementation requirements. It adds no native target
version claim, filesystem guarantee, model mapping update, codec, CLI flag,
source verification, migration, ownership adoption or dependency. Exact native
versions and OS/scope qualification remain adapter evidence gates. Cursor and
Gemini remain design targets for later implementation.
