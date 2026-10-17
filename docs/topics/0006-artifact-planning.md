# Artifact planning

This concern owns the requirements and implemented architecture below. Moving
these records does not change their implementation or approval status.
Library-only capabilities remain separate from production CLI activation; each
section records its own limits. Accepted decisions remain in
[ADRs](https://github.com/driftsys/upskill/blob/main/docs/adr/README.md).

## Requirements

<a id="42-pure-owned-instruction-planning"></a>

### Pure owned instruction planning

The library function `instructions::plan::plan_instructions` joins validated
owned-artifact records with the Markdown aggregate editor. Callers supply
complete installation contribution replacements or removals and exact
observations for every affected file. The result contains the complete resulting
artifact vector and file plans with exact optional before bytes and after bytes,
including content no-ops. Absent files and existing empty files have distinct
preconditions. Removing the last region retains manual bytes and keeps the
aggregate file, which is empty only when no content remains. The plan has no
deletion action.

Set replaces one installation's complete contribution set. Existing installation
IDs retain their source, item kind, source name and installed alias; recorded
revision may change. Untouched selectors retain their bodies, owners and
consumers and cannot be taken over, even with identical content. Affected
regions require one owner; affected whole files or shared regions conflict.
Unaffected records, including whole files and shared regions, survive exactly.
At affected locations, the editor rebuilds separator metadata for retained
regions after other removals.

`instructions::aggregate::inspect_owned` reads exact verified Markdown bodies in
document order through the same private marker/span validator as reconciliation.
Edited, missing, duplicate, overlapping, unknown or malformed regions conflict.
Bytes outside managed regions remain unchanged. Locations use scope, raw lexical
root spelling and portable relative path; file plans sort by that key. Surviving
artifact rows retain prior order and new selectors append in location/ID order.

Planning performs no I/O and validates prior and final ownership plus all
desired data. Observations must match affected locations exactly; missing,
duplicate or extraneous observations conflict. Supplied bytes do not establish
source or filesystem evidence. Callers must resolve direct/bundle references and
native compatibility first. The separate pure whole-file planner is implemented
in [owned whole-file operation plans](#44-owned-whole-file-operation-plans);
captured compilation in
[captured item compilation](0002-generation.md#45-captured-item-compilation)
derives complete per-client entrypoint and resource outputs. The complete mixed
coordinator is implemented in
[mixed owned-artifact operation plans](#46-mixed-owned-artifact-operation-plans).
Reference-graph authority and per-owner consumer associations now support
removal planning
([reference-driven shared-owner removal planning](0005-lifecycle-planning.md#416-reference-driven-shared-owner-removal-planning)).
Integrating captured-rule desired sets
([captured rule desired sets](0002-generation.md#414-captured-rule-desired-sets))
with complete add/update planning, state persistence and transactional execution
remain pending. This prerequisite changes no CLI behavior, source format or
lockfile format and enables no native client support.

See the
[planner design](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#architecture-owned-instruction-planning)
and its decision records for implementation boundaries and rationale.

<a id="44-owned-whole-file-operation-plans"></a>

### Owned whole-file operation plans

The pure library module `files::plan` exports these public values, all deriving
`Debug`, `Clone`, `PartialEq` and `Eq` without serialization:

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

`FileLocation` uses the ownership model's `InstallationRoot`; desired files use
explicit `TargetSurface` consumers and changes use an explicit `ArtifactOwner`.
Inputs are borrowed and unchanged. Outputs own their bytes and ownership
records. The planner performs no I/O. An observed `None` means an absent
directory entry, not an unreadable file or directory; supplied observations are
unqualified before-state evidence and do not establish provenance.

Prior and desired records pass `ownership::validate_artifacts`. Observed and
desired locations require host-absolute nonempty roots and portable relative
paths; optional modes must be in `0..=0o7777`. Invalid locations and modes are
rejected before observation membership diagnostics. Location identity and
sorting compare scope, the exact root `OsStr` spelling and portable path. There
is no component normalization, canonicalization, case folding, alias resolution
or containment claim.

A `Set` supplies the complete nonempty whole-file set for its installation;
`Remove` removes its last files. Duplicate changes, unknown removals and
duplicate desired locations conflict. Reusing an installation ID cannot change
source, kind, source name or installed alias. Its revision may change
consistently across all replacement records. Equal content establishes neither
compatibility nor shared ownership.

Affected locations include prior records owned by selected installations and all
desired locations. Each requires exactly one observation, including
revision-only changes and byte-identical no-ops. Duplicate, missing and
extraneous observations conflict. Affected Markdown contributions and
multi-owner whole files conflict. Unrelated records, including shared files and
contributions, survive exactly. A desired file cannot take over an untouched
installation; explicit removal of the old owner plus selection of the new owner
permits handoff only when the recorded before-state is verified.

Owned locations require present observed bytes matching the recorded SHA-256 and
exact equality of the optional recorded and observed modes. `None` differs from
`Some(0)`. Edited or missing files conflict on update, removal and no-op.
Unowned locations require absence even when present bytes and modes equal
desired values. The planner does not reinterpret platform mode evidence or
authorize permission application.

An affected location cannot be a strict slash-component ancestor or descendant
of any prior or desired artifact within the same exact root and scope.
Contributions count as files here. `a` and `ab` are distinct siblings; removing
`a` while creating `a/b` still conflicts. Disjoint untouched topology pairs do
not block other work.

The result contains one `PlannedFile` per affected location in lexical order,
with exact observed `before` and desired `after` values, including no-ops.
Creation has absent `before`; deletion has absent `after` and identifies only an
individual recorded file, granting no directory ownership or recursive removal
authority. Desired ownership records contain SHA-256 of exact desired bytes,
optional mode, explicit owner and explicit consumers. Untouched rows and
same-location replacements retain prior row order; removed rows disappear and
new rows append in lexical order. The final record set is validated before
return. Any failure returns no successful partial plan.

This prerequisite leaves existing commands, CLI flags, lockfile persistence and
native-client behavior unchanged. Integrating captured-rule desired sets
([captured rule desired sets](0002-generation.md#414-captured-rule-desired-sets))
with lifecycle planning, qualified observation, destination
equivalence/containment, writer locking, permission policy, durable state,
apply/recovery and Codex activation remain pending. The mixed coordinator in
[mixed owned-artifact operation plans](#46-mixed-owned-artifact-operation-plans)
composes complete sets across both forms; the individual planners do not
authorize partial application of mixed installations. This API does not complete
M1-A2, M1-A3 or M1.

See the
[whole-file planning design](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#architecture-owned-file-planning)
and the active
[operational lifecycle design](https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-10-12-m1-owned-lifecycle-design.md).

<a id="46-mixed-owned-artifact-operation-plans"></a>

### Mixed owned-artifact operation plans

The pure library module `operations::plan` composes the existing dedicated-file
and instruction planners into one complete installation operation. Its two new
value types derive `Debug`, `Clone`, `PartialEq` and `Eq`, without
serialization:

```rust
pub enum DesiredArtifact {
    File(DesiredFile),
    Instruction(DesiredInstructionContribution),
}

pub enum InstallationChange {
    Set {
        owner: ArtifactOwner,
        artifacts: Vec<DesiredArtifact>,
    },
    Remove {
        installation_id: String,
    },
}

pub fn plan_owned_artifacts(
    prior: &[OwnedArtifact],
    changes: &[InstallationChange],
    observations: &[ObservedFile],
) -> anyhow::Result<FileOperationPlan>;
```

The existing `files::plan` and `instructions::plan` modules supply the desired
values. `ownership` supplies records and owners. The result reuses
`FileOperationPlan`, `PlannedFile`, `FileLocation` and `FileValue` from
`files::plan`; both physical forms therefore carry exact optional before and
after values.

A Set replaces the installation's complete desired set across both forms.
Omitted old files and contributions are removed through their respective
planners; a global Remove reaches every old form. Empty Sets require explicit
Remove. Duplicate changes, unknown removals, duplicate desired file locations or
contribution selectors, invalid owners and invalid consumers conflict. An
existing installation retains its source, item kind, source name and installed
alias even when the old and desired forms are disjoint. Its revision may change
coherently across all desired records. Any multi-owner record at an affected
location conflicts; untouched shared rows survive exactly.

Locations compare `(scope, raw root OsStr, portable relative path)`. Raw dot
components and project/user scopes remain distinct. Roots must be nonempty and
absolute; paths must use portable relative components. Supplied optional modes
must be within `0..=0o7777`. Every affected physical location requires exactly
one observation, including no-ops: the affected union includes every old row
belonging to a changed installation and every desired location. Missing,
duplicate or extra observations conflict, and all observations are validated
before membership checks.

A location cannot participate in both forms anywhere in the prior/desired union.
Even explicit old-owner removal and new-owner replacement cannot convert a
contribution into a whole file or the reverse at that location: contribution
ownership does not establish authority over manual aggregate bytes. Same-form
handoffs retain the underlying planners' rules. An ancestor/descendant pair in
the union conflicts when either location is affected, within the same exact root
and scope. This includes cross-form, removal-only and old/new transitions.
Slash-component boundaries distinguish `a/b` from `ab`; unrelated historical
pairs survive without a filesystem safety claim.

Dedicated actions retain exact optional before/after bytes and modes. Owned
bytes and optional modes must still match prior evidence on update, removal and
no-op; an unowned destination must be absent. Aggregate actions use the exact
observed before value and the editor's after bytes, preserving the observed
optional mode. An absent aggregate receives after mode `None`. Invalid UTF-8,
edited or unrecorded markers and missing owned aggregates conflict. Manual bytes
survive; removing the last contribution retains the aggregate, even when empty.
No permission authority follows from the supplied mode values.

Every affected action remains in the result, including byte-identical no-ops.
Actions sort by exact location and duplicate physical actions conflict. Retained
artifact identities replace rows at their original positions across the original
interleaved forms and installations. Removed identities disappear; new rows
append in exact location/selector order, with whole files distinguished from
contribution selectors. Final combined ownership is validated, including
revision coherence. Reordering changes, desired entries or observations does not
change the result. The API performs no I/O or input mutation; any failure
returns no successful partial plan. It may return the first contextual conflict
rather than all diagnostics.

This library prerequisite activates no CLI selector, native adapter, source
fetch, filesystem observation, lock, journal, schema-2 persistence or execution.
Captured-rule desired sets are implemented in
[captured rule desired sets](0002-generation.md#414-captured-rule-desired-sets);
reference authority and per-owner associations support removal planning in
[reference-driven shared-owner removal planning](0005-lifecycle-planning.md#416-reference-driven-shared-owner-removal-planning).
Complete add/update integration with immutable source evidence, source/target
adapters, qualified observations, permission/metadata policy, writer locking,
guarded persistence and apply/recovery remains later lifecycle work. Pure
[permission selection](0007-filesystem-access.md#architecture-file-permission-policy)
is implemented as a separate inactive supplied-value capability. It does not
complete M1-A2, M1-A3 or M1. The public declarations are in
[`operations::plan`](https://github.com/driftsys/upskill/blob/main/src/operations/plan.rs).

## Architecture

<a id="architecture-owned-instruction-planning"></a>

<a id="architecture-owned-instruction-planning-owned-instruction-operation-planning"></a>

### Owned instruction operation planning

This record describes the pure library planner implemented in
[`instructions::plan`](https://github.com/driftsys/upskill/blob/main/src/instructions/plan.rs).
The [pure owned instruction planning](#42-pure-owned-instruction-planning) owns
its requirements. The planner has no CLI dispatch, state codec or I/O.

<a id="architecture-owned-instruction-planning-public-boundary"></a>

#### Public boundary

`plan_instructions` borrows prior ownership records, installation changes and
observations, and returns an owned `InstructionOperationPlan` through
`anyhow::Result`. The public DTOs derive equality, cloning and debug support;
they have no serialization contract. An `InstructionLocation` combines an
`InstallationRoot` and portable relative path. `ObservedInstructionFile` holds
that location and optional exact bytes; absence differs from an existing empty
file. A set change carries an `ArtifactOwner` and its complete desired
contribution set, including Markdown, selector and consuming surfaces. A remove
change names an installation ID.

The result contains the complete artifact vector and `PlannedInstructionFile`
entries with exact optional before bytes and resulting bytes. File plans include
content no-ops and retain aggregates after final-region removal. Caller-supplied
observations establish preconditions for a later executor, without establishing
filesystem or source authority.
[AD-0016](https://github.com/driftsys/upskill/blob/main/docs/adr/0016-pure-instruction-planning.md)
records this boundary.

<a id="architecture-owned-instruction-planning-validation-and-location-grouping"></a>

#### Validation and location grouping

The planner validates prior records with `ownership::validate_artifacts`,
indexes installation bindings, and validates changes and desired contributions
before editing. Existing bindings compare source, item kind, source name and
installed alias explicitly; revision may change. Desired data passes through the
existing editor and ownership validator, including paths, owner metadata and
consumers. Duplicate changes, empty sets and unknown removals fail with context.

Affected locations are the union of prior locations involving changed owners and
desired locations. Their key is scope, native `OsString` root spelling and
portable relative path. Ordered maps and sets retain exact lexical identity
without canonicalization, alias detection or containment checks. Observations
must match that set exactly. A prior owned region cannot be reconciled from
absent bytes; observed aggregates must decode as UTF-8.

<a id="architecture-owned-instruction-planning-verified-body-inspection-and-editing"></a>

#### Verified body inspection and editing

[`aggregate::inspect_owned`](https://github.com/driftsys/upskill/blob/main/src/instructions/aggregate.rs)
shares the private span validator with `aggregate::reconcile`. Verified spans
retain body start offsets and end-marker positions, so the reader takes exact
body slices and returns contributions in document order. It rejects duplicate
IDs, overlapping spans and invalid marker, hash or separator state through the
existing editor validation. There is no second marker parser.

At each affected location, the planner reads all verified bodies, retains
untouched selectors and combines them with the desired complete replacements. A
desired selector cannot take over an untouched owner. It calls reconciliation
once for the complete aggregate edit and joins returned selectors to the
retained or desired owner and consumer metadata. Returned contribution metadata
includes new separator context even for retained bodies. This allows subsequent
removal to restore manual bytes exactly.

A changed multi-owner contribution conflicts because the stored consumer union
does not identify each owner's selection. Untouched shared contributions remain,
including within an affected aggregate; separately owned regions are supported.
An affected whole-file record conflicts.
[AD-0017](https://github.com/driftsys/upskill/blob/main/docs/adr/0017-shared-instruction-owner-evidence.md)
records the shared-region restriction. Empty aggregates remain files because
prior file origin is unknown; see
[AD-0018](https://github.com/driftsys/upskill/blob/main/docs/adr/0018-retain-empty-instruction-aggregates.md).

<a id="architecture-owned-instruction-planning-result-assembly-and-downstream-limits"></a>

#### Result assembly and downstream limits

Surviving prior selectors retain their relative artifact-row order. Removed
selectors disappear; genuinely new selectors append in location and ID order.
Unaffected locations retain exact records. File plans sort by location key. The
complete result passes ownership validation before return; errors return no
partial plan and inputs remain borrowed and unchanged.

Callers must resolve direct/bundle references and native compatibility before
planning. The separate
[pure whole-file planner](#44-owned-whole-file-operation-plans) is implemented
for explicitly supplied complete file sets. The separate
[captured item compiler](0002-generation.md#architecture-captured-item-compilation)
derives complete per-client entrypoint and resource outputs from validated
captured bytes. Qualified scope roots, writer locking,
[destination observation](0007-filesystem-access.md#architecture-qualified-destination-observation)
and
[qualified source observation](0003-sources.md#architecture-qualified-source-observation)
are implemented as separate inactive capabilities. Final-head hosted source
qualification remains required. The
[combined-state validator](0004-state-and-ownership.md#architecture-combined-lifecycle-state)
provides reference-graph authority and explicit per-owner consumer associations.
Removal planning integrates those associations and returns complete combined
state. Integrating qualified source evidence and compiled outputs with complete
add/update planning, persistence and guarded application remain pending. The
opt-in
[qualified file executor and recovery](0008-execution-and-recovery.md#architecture-qualified-file-execution)
are implemented separately; hosted final-head qualification remains required.
The implemented [mixed coordinator](#architecture-mixed-owned-artifact-planning)
composes complete explicit sets across both forms; neither individual planner
authorizes partial application of a mixed installation. Current commands,
lockfile/source formats and native client support are unchanged.

[`tests/instruction_plans.rs`](https://github.com/driftsys/upskill/blob/main/tests/instruction_plans.rs)
covers owner retention, complete replacements and moves, exact preconditions,
conflicts, consumer propagation, lexical scope/root identity and row ordering.
[`tests/instruction_contributions.rs`](https://github.com/driftsys/upskill/blob/main/tests/instruction_contributions.rs)
covers the shared reader and editor validation. These tests establish the pure
boundary, without native discovery or filesystem execution claims.

The opt-in
[combined-state validator](0004-state-and-ownership.md#architecture-combined-lifecycle-state)
now checks reference authority and explicit per-owner consumer associations.
Reference-driven removal planning now connects this state to shared-owner
retirement. The
[single-owner upsert coordinator](0005-lifecycle-planning.md#architecture-lifecycle-upsert)
now connects complete add/update sets to graph and mixed-artifact planning. The
separate inactive `plan_shared_upsert` entrypoint consumes explicit
shared-output contracts. Guarded lifecycle execution remains pending.

<a id="architecture-owned-file-planning"></a>

<a id="architecture-owned-file-planning-owned-whole-file-operation-planning"></a>

### Owned whole-file operation planning

This record describes the pure planner implemented in
[`files::plan`](https://github.com/driftsys/upskill/blob/main/src/files/plan.rs).
[owned whole-file operation plans](#44-owned-whole-file-operation-plans) owns
its requirements. The implementation consumes supplied values and performs no
I/O.
[AD-0021](https://github.com/driftsys/upskill/blob/main/docs/adr/0021-pure-file-planning.md)
records the bounded delivery choice.

<a id="architecture-owned-file-planning-public-boundary"></a>

#### Public boundary

`plan_files` borrows prior `OwnedArtifact` records, `FileInstallationChange`
values and `ObservedFile` values. It returns an owned `FileOperationPlan`
through `anyhow::Result`. The seven public DTOs derive `Debug`, `Clone`,
`PartialEq` and `Eq` without serialization. `FileLocation` combines an
`InstallationRoot` with a portable relative path; `FileValue` carries exact
bytes and optional permission mode. Desired files additionally carry explicit
consumers. A set change carries one explicit owner and its complete nonempty
desired file set; removal names an installation ID. An observed absent value
means an absent directory entry; it does not represent an unreadable file or
directory.

The result contains the complete ownership vector and individual `PlannedFile`
entries with exact optional before/after values. Byte-identical no-ops retain
preconditions. Deletions identify individual files and grant no directory or
recursive removal authority. Caller-supplied evidence establishes neither source
provenance nor qualified filesystem authority.

<a id="architecture-owned-file-planning-validation-and-indexing"></a>

#### Validation and indexing

The planner validates prior records with `ownership::validate_artifacts`, then
validates observed roots, portable paths and optional modes. It indexes changes,
validates desired records through the ownership boundary and rejects duplicate
installation changes, empty sets, unknown removals and duplicate desired
locations. Invalid observed or desired locations and modes fail before
observation membership diagnostics. Desired record hashes use SHA-256 of exact
desired bytes, rendered as lowercase hexadecimal through the existing digest
dependency.

Location keys combine scope, exact native root `OsString` spelling and portable
path in ordered maps and sets. No `Path` component normalization, lossy string
conversion, canonicalization, case folding, alias resolution or containment
qualification occurs.
[AD-0021](https://github.com/driftsys/upskill/blob/main/docs/adr/0021-pure-file-planning.md)
records location identity, complete replacements, stable bindings and
deterministic ordering as parts of the pure API boundary.

<a id="architecture-owned-file-planning-affected-locations-and-before-state-verification"></a>

#### Affected locations and before-state verification

Affected locations are the union of desired locations and prior locations with
any changed owner. Affected contributions and multi-owner files fail before
planning; a desired location cannot take over an untouched owner. Unrelated
rows, including shared files and contributions, remain exact. Both old and new
owners must be selected for a same-location handoff.
[AD-0022](https://github.com/driftsys/upskill/blob/main/docs/adr/0022-conservative-file-ownership-evidence.md)
records the composition restriction.

Observation keys must match the affected set exactly: duplicates, missing
entries and extraneous entries fail, including for revision-only changes and
no-ops. At a prior owned location, observed bytes must be present and match its
stored SHA-256, and optional modes must match exactly. At an unowned location
the observation must establish absence even when existing bytes equal the
desired bytes. These choices are recorded in
[AD-0022](https://github.com/driftsys/upskill/blob/main/docs/adr/0022-conservative-file-ownership-evidence.md).

The topology check compares the union of prior and desired locations in the same
exact root and scope. It rejects a strict slash-component ancestor pair whenever
one side is affected, including removal of `a` followed by creation of `a/b`.
Contributions count as file locations. Component-prefix siblings and disjoint
untouched pairs remain allowed.
[AD-0023](https://github.com/driftsys/upskill/blob/main/docs/adr/0023-reject-file-topology-transitions.md)
records why directory transitions await a qualified contract.

<a id="architecture-owned-file-planning-result-assembly-and-downstream-limits"></a>

#### Result assembly and downstream limits

Each affected location produces one action in lexical key order. The planner
walks prior rows to preserve unaffected rows and the positions of same-location
replacements, drops removed rows, then appends new desired rows in lexical
order. Replacement records contain the explicit owner, consumers, desired mode
and exact byte hash. Final ownership validation precedes return; any error
returns no successful partial plan. Inputs remain unchanged.
[AD-0021](https://github.com/driftsys/upskill/blob/main/docs/adr/0021-pure-file-planning.md)
records the ordering choice.

[`tests/file_plans.rs`](https://github.com/driftsys/upskill/blob/main/tests/file_plans.rs)
covers exact creation, replacement, removal, owner handoff, no-op preconditions,
untouched-record retention, metadata validation, mode boundaries,
binary/empty/Unicode values, raw-root and scope isolation, ancestor conflicts
and deterministic output. The critical creation, update and empty-content hash
assertions use independent literal SHA-256 values. Other fixture-derived
expected records use test-side `Sha256::digest`. The tests establish
supplied-value behavior, without proving actual filesystem observation or
execution.

The
[captured item compiler](0002-generation.md#architecture-captured-item-compilation)
already derives complete per-client resource and entrypoint outputs from
validated captured sources. The
[mixed coordinator](#architecture-mixed-owned-artifact-planning) composes
complete explicit file/contribution sets; neither individual planner authorizes
partial application of a mixed installation. Captured-rule desired sets are
implemented in
[captured-rule desired sets](0002-generation.md#architecture-captured-rule-desired-sets);
lifecycle planning and reviewed adapters remain pending. Qualified scope roots,
writer locking,
[destination observation](0007-filesystem-access.md#architecture-qualified-destination-observation)
and
[qualified source observation](0003-sources.md#architecture-qualified-source-observation)
are implemented as separate inactive capabilities, including conservative
destination overlap rejection and descriptor-relative source and destination
observation. Final-head hosted source qualification remains required.
Integrating qualified source evidence into the lifecycle, combined durable state
and Codex activation remain pending. Pure
[permission selection](0007-filesystem-access.md#architecture-file-permission-policy)
and opt-in
[qualified file execution and recovery](0008-execution-and-recovery.md#architecture-qualified-file-execution)
are implemented as separate library capabilities. This prerequisite leaves
current commands, schema-1 lockfiles and native client behavior unchanged and
does not complete M1-A2, M1-A3 or M1.

<a id="architecture-mixed-owned-artifact-planning"></a>

<a id="architecture-mixed-owned-artifact-planning-mixed-owned-artifact-operation-planning"></a>

### Mixed owned-artifact operation planning

This record describes the implemented pure coordinator in
[`operations::plan`](https://github.com/driftsys/upskill/blob/main/src/operations/plan.rs).
[mixed owned-artifact operation plans](#46-mixed-owned-artifact-operation-plans)
owns its requirements.
[AD-0027](https://github.com/driftsys/upskill/blob/main/docs/adr/0027-compose-complete-owned-artifact-plans.md)
records the accepted composition choice. The coordinator performs no I/O and is
not called by current installation commands.

<a id="architecture-mixed-owned-artifact-planning-public-boundary"></a>

#### Public boundary

`plan_owned_artifacts` borrows prior `OwnedArtifact` records,
`InstallationChange` values and `ObservedFile` values, returning an owned
`FileOperationPlan` through `anyhow::Result`. `DesiredArtifact` distinguishes
`File(DesiredFile)` from `Instruction(DesiredInstructionContribution)`.
`InstallationChange` holds either a Set with one owner and desired artifacts, or
a Remove naming an installation. Both new enums derive `Debug`, `Clone`,
`PartialEq` and `Eq` without serialization. Existing file planner DTOs represent
both forms' physical actions with exact optional before/after bytes and modes.

A Set describes the installation's complete desired state across both forms. The
coordinator validates and partitions that operation before invoking the reviewed
[file planner](#architecture-owned-file-planning) and
[instruction planner](#architecture-owned-instruction-planning). It then
reconstructs one complete ownership vector and one sorted physical action
vector. Failure at any stage returns no successful combined plan; inputs remain
unchanged.

<a id="architecture-mixed-owned-artifact-planning-complete-installations-and-bindings"></a>

#### Complete installations and bindings

Complete prior ownership validation precedes indexing. Global installation
bindings are checked before forms split, so changing from file-only to
instruction-only ownership at a distinct location cannot bypass the source,
kind, source-name or installed-alias binding. Source revision may change
coherently. Duplicate changes, unknown removals and empty Sets conflict.

The original plan required existing helpers to remain unchanged. The accepted
implementation ruling instead exposes `files::plan::validate_binding` as
`pub(crate)` and imports it into the coordinator. Its body and existing planner
behavior remain unchanged. This narrow visibility expansion avoids duplicating
stable-binding policy, at the cost of crate-internal coupling to the file
planner.

Prior rows partition by form while preserving their relative order. Each form
receives a nonempty Set when desired, a Remove when previously owned but now
omitted, or no change when both absent. A global Remove reaches every previously
owned form. Both planners receive complete form partitions rather than partial
mixed requests. Any multi-owner record at an affected physical location
conflicts, including another selector there; untouched shared rows survive.

<a id="architecture-mixed-owned-artifact-planning-locations-and-supplied-evidence"></a>

#### Locations and supplied evidence

Ordered keys combine ownership scope, raw root `OsString` and portable relative
path. Roots must be nonempty and absolute; lexical identity does not normalize
dot components or prove host equivalence, containment or realizability. The
coordinator indexes form membership throughout the prior/desired union and marks
old locations of changed installations plus every desired location as affected.

Every supplied observation is validated before membership checks, including
root, portable path and permission-bit range. Exactly one observation must match
each affected location, even for no-ops; missing, duplicate and extra evidence
fail. A physical location cannot have both forms anywhere in the union,
including an explicit removal and replacement. Contribution ownership does not
prove authority over surrounding manual bytes. Same-form handoffs retain each
planner's rules.

Ancestor/descendant locations within the same exact root and scope conflict when
either endpoint is affected. This check covers both forms, removal-only changes
and old/new transitions. Slash-component boundaries distinguish `a/b` from `ab`.
Unrelated historical topology is preserved without a filesystem safety claim.

Observations partition using validated form membership. Dedicated values retain
bytes and optional modes intact. Instruction observations pass optional exact
bytes to the aggregate planner; physical mode evidence stays with the
coordinator. The existing planners enforce owned byte/mode evidence and verified
aggregate markers, including invalid UTF-8 rejection and exact manual-byte
preservation.

<a id="architecture-mixed-owned-artifact-planning-combined-result"></a>

#### Combined result

The coordinator indexes planned ownership by exact location plus optional
contribution selector. It walks the original interleaved prior rows, replacing
retained identities at their former positions and dropping removed identities.
Remaining new identities append in key order. Final combined ownership
validation checks the union, including revision coherence.

Dedicated actions retain their exact before/after values. Aggregate actions use
the supplied observed before value and the editor's after bytes. Their after
mode is the observed optional mode, or `None` for an absent aggregate. Removing
the last contribution retains the aggregate, even when empty. Modes grant no
permission or execution authority. All affected actions, including
byte-identical no-ops, remain, sort by exact location and pass a
duplicate-location guard.

<a id="architecture-mixed-owned-artifact-planning-evidence-and-remaining-integration"></a>

#### Evidence and remaining integration

[`tests/operation_plans.rs`](https://github.com/driftsys/upskill/blob/main/tests/operation_plans.rs)
covers mixed creation, complete replacements and removals, disjoint-form binding
checks, coherent and conflicting revisions, physical collisions and topology,
exact observations, manual bytes and aggregate modes, shared ownership,
raw-root/scope identity, interleaved row reconstruction and deterministic
permutations. Literal hashes and bytes pin changed contribution metadata and a
subsequent no-op roundtrip. The tests establish supplied-value behavior rather
than filesystem execution.

The
[captured compiler](0002-generation.md#architecture-captured-item-compilation)
already derives complete per-client resource and entrypoint outputs. Connecting
those outputs to lifecycle ownership, roots, consumers and destination
observations remains pending. The
[combined-state library](0004-state-and-ownership.md#architecture-combined-lifecycle-state)
now validates reference authority and per-owner consumer associations. The
separate
[removal planner](0005-lifecycle-planning.md#architecture-lifecycle-reference-removal)
integrates those records for shared-owner retirement. Complete add/update
planning, integration of immutable source evidence and source/target adapters,
qualification, writer locking, guarded durable state and physical apply/recovery
remain later lifecycle gates. The pure
[permission-selection policy](0007-filesystem-access.md#architecture-file-permission-policy)
is implemented;
[qualified file execution and recovery](0008-execution-and-recovery.md#architecture-qualified-file-execution)
consume selected modes as an opt-in library capability. Complete lifecycle
integration and native qualification remain pending. The API may return the
first contextual conflict; complete diagnostic aggregation remains an activation
gate. No CLI selector, native support, schema-2 persistence or installation
effects are activated. M1-A2, M1-A3 and M1 remain incomplete.
