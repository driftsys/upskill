# Owned whole-file planning design

Date: 2026-10-12. Status: **Implementation contract**.
Baseline: `d170f2b` (PR #260).
Authority: [M1 lifecycle design](2026-10-12-m1-owned-lifecycle-design.md)
and [M1 delivery plan](../plans/2026-10-11-instructions-rules-m1.md).

## Intent and boundary

Plan explicitly enumerated whole-file outputs from supplied observations while
preserving untouched ownership and refusing to overwrite unowned content. This
supplies the pure file-operation prerequisite for dedicated entrypoints and
supporting resources. It does not discover, select, render or copy resources.
The user delegated the continuing design, implementation, review and merge loop.

The merged instruction planner handles marked Markdown contributions. The strict
source observer supplies captured values. Neither supplies whole-file operation
plans. Existing commands still use schema 1 and the historical pipeline.

## Alternatives and selected approach

1. Add filesystem writes directly to the historical pipeline. This would expose
   ownership-aware output before locking, recovery and combined state are ready.
2. Build a pure whole-file planner first. Select this approach: explicit input
   evidence, complete replacement sets and individually enumerated actions can
   be reviewed independently of application authority.
3. Combine file planning, source classification, rendering and both instruction
   and file execution now. This spans independent qualification and persistence
   contracts. Defer that composition to later lifecycle work.

## Public interface

Add `files::plan` and export `files` from the library. DTOs derive
`Debug`, `Clone`, `PartialEq` and `Eq`, without serialization:

```rust
pub struct FileLocation {
    pub root: InstallationRoot,
    pub path: String,
}
pub struct FileValue {
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}
pub struct ObservedFile {
    pub location: FileLocation,
    pub value: Option<FileValue>,
}
pub struct DesiredFile {
    pub location: FileLocation,
    pub value: FileValue,
    pub consumers: Vec<TargetSurface>,
}
pub enum FileInstallationChange {
    Set { owner: ArtifactOwner, files: Vec<DesiredFile> },
    Remove { installation_id: String },
}
pub struct PlannedFile {
    pub location: FileLocation,
    pub before: Option<FileValue>,
    pub after: Option<FileValue>,
}
pub struct FileOperationPlan {
    pub artifacts: Vec<OwnedArtifact>,
    pub files: Vec<PlannedFile>,
}
pub fn plan_files(
    prior: &[OwnedArtifact],
    changes: &[FileInstallationChange],
    observations: &[ObservedFile],
) -> anyhow::Result<FileOperationPlan>;
```

Inputs are borrowed and unchanged. Outputs own their bytes and records. An absent
observation means an absent directory entry, not an unreadable file or directory.
Only a later qualified observer can establish that fact. The planner performs no
I/O and treats supplied values as evidence, not trusted provenance.

## Validation and identity

Validate prior records with `ownership::validate_artifacts`. Validate every desired
and observed location: host-absolute nonempty root, existing portable relative-path
predicate, and optional mode in `0..=0o7777`. Validate every desired owner/consumer
record through the existing ownership boundary. Invalid locations or modes must
not be hidden behind missing or extraneous observation diagnostics.

A `Set` is the complete nonempty whole-file set for its installation. Reject
empty sets; use `Remove` for the last file. A repeated installation ID cannot
change source, kind, source name or installed alias. Its revision may change,
consistently across all replacement records. Reject duplicate changes, unknown
removals and duplicate desired locations, including across installations. This
slice does not establish shared-file compatibility from equal bytes.

Locations compare scope, exact root `OsStr` spelling and portable path. Do not
use `Path` equality to normalize roots. Do not canonicalize, fold case, resolve
aliases or claim containment. Sorting uses that same lexical location key.

## Affected locations and preconditions

A location is affected when a prior row has a changed installation owner or a
new desired file names it. Require exactly one observation for every affected
location, including metadata-only changes and byte-identical no-ops. Reject
missing, duplicate and extraneous observations.

Reject affected Markdown-contribution records and affected multi-owner whole-file
records. Their composition or per-owner consumer authority is not available at
this boundary. Preserve every unrelated row, including shared files and Markdown
contributions, exactly. A desired file cannot take over an untouched owner.

At an owned whole-file location, require present observed bytes whose SHA-256
matches the recorded file digest. Require exact equality of the optional recorded
and observed modes. Missing mode evidence does not prove a recorded mode, and
changing optional evidence is a conflict. An edited or missing owned file conflicts
on update, removal and no-op. This deliberately conservative boundary does not
reinterpret cross-platform mode evidence; later qualification must resolve it.

At a location with no prior ownership, require absence. Identical existing bytes
are still unowned and conflict. When a selected owner leaves a verified location,
another explicitly selected installation may replace it; both changes and all
before-state checks remain necessary. No implicit sharing or adoption occurs.

Check lexical file/descendant conflicts within an exact root and scope: an affected
file cannot be an ancestor or descendant of another prior or desired artifact
location. Contribution locations count as files for this check. Use component
boundaries, so `a` and `ab` do not conflict. Same-location replacement is allowed
under the preceding ownership rules. Reject even removal-to-directory transitions
such as removing file `a` while adding `a/b`; directory topology changes need a
separate qualified contract. Disjoint pre-existing topology pairs do not block an
operation unless at least one location is affected.

## Result contract

For every affected location, emit one `PlannedFile`, sorted by lexical location.
Creation has absent `before`; deletion has absent `after`; replacements and no-ops
retain exact before/after bytes and modes. No-op plans still carry preconditions.
Deletion identifies only the recorded file; it never grants directory ownership
or recursive removal authority.

Rebuild desired `ArtifactContent::File` records using SHA-256 of exact desired
bytes and the desired optional mode. Each has the explicit selected owner and
consumers. Keep prior row order for untouched rows and same-location replacements;
append new locations in lexical order. Removed rows disappear. Validate the final
record set before returning it. The planner may return the first contextual
conflict; complete operation-level diagnostic aggregation remains a coordinator
gate. Failure returns no successful partial plan.

## Acceptance examples

1. Creating an entrypoint and binary resource at absent destinations returns two
   sorted actions and exact hashes/modes. Removing them returns two individual
   deletions and preserves an unrelated manual-contribution record.
2. A pre-existing unmarked file conflicts even when bytes and modes equal the
   desired output. An edited owned file conflicts before a replacement plan.
3. Updating one installation preserves another installation's records and a
   shared untouched file. Revision-only updates retain byte-identical actions.
4. A changed alias/source binding, affected shared file, affected contribution,
   missing observation or `a`/`a/b` topology transition conflicts explicitly.
5. Empty bytes, optional modes, modes `0` and `0o7777`, Unicode relative paths,
   exact lexical root variants and distinct scopes remain representable. Invalid
   modes and portable paths reject. Inputs remain unchanged.

## Implementation constraints and remaining gates

Rust 1.85, synchronous standard library plus existing dependencies, no stdout or
stderr outside `main.rs`, and no new dependency or runtime. Keep the historical
pipeline, existing instruction planner semantics, CLI flags and lockfile behavior
unchanged. Code, acceptance tests and canonical specification/usage notes land
together; run `just fmt` followed by `just verify` before opening a PR.

This API does not validate source manifests, resolve Git revisions, classify
entrypoints/resources, apply ignore rules or rewrite links. A future compiler must
explicitly derive complete desired sets from captured source values and reviewed
adapters. A future coordinator must compose instruction and file plans for mixed
installations; neither planner authorizes partial application on its own.

Qualified source/destination observation, filesystem equivalence/containment,
writer locking, permission policy, apply/recovery, reference graphs, combined
state and all lifecycle adapters remain pending. No schema-2 writer, resource
compatibility, native-client or Codex activation is claimed. Completing this
prerequisite does not complete M1-A2, M1-A3 or M1.
