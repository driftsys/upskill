# Lifecycle planning

Architecture reconciliation (2026-10-25): the
[delivery, discovery and activation contract](0009-customization-delivery.md)
controls intended production routes and reader intent. Existing Codex
`AGENTS.md` contribution APIs described here are experimental library
boundaries, not production independent-rule delivery. Independent Codex/OpenCode
rules remain beside unchanged team guidance through a selected native or hook
route. The native-first amendment permits explicit OpenCode paths after Q5 PASS
and selects the trusted Codex hook with its declared recorded-history
limitation. Reusable ownership, preflight and recovery mechanisms do not select
or qualify a runtime route, and schema-2 production activation remains blocked
([AD-0040 amendment](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md#native-first-rules-and-implementation-order-2026-10-24)).

This concern owns the requirements and implemented architecture below. Moving
these records does not change their implementation or approval status.
Library-only capabilities remain separate from production CLI activation; each
section records its own limits. Accepted decisions remain in
[ADRs](https://github.com/driftsys/upskill/blob/main/docs/adr/README.md).

## Requirements

<a id="415-authoritative-lifecycle-reference-reconciliation"></a>

### Authoritative lifecycle reference reconciliation

The inactive `lifecycle::reconcile::reconcile_references` API accepts validated
combined state and explicit installation/bundle upserts plus reference edits. It
returns graph records and removed IDs without I/O or input mutation. Only actual
direct roots and transitive bundle edges retain nodes; empty output or
artifact/consumer count does not remove a referenced installation.

All supplied nodes and edges are validated before iterative pruning, including
unreachable dangling edges and cycles. Explicit upserts and added references
must survive reconciliation; otherwise the request is rejected. Duplicate,
absent, already-existing or overlapping reference edits conflict. Existing
installation IDs cannot change source/kind/name/alias or scope/raw root;
existing bundle IDs retain source/name. Valid evidence/revision and requested
surface updates remain subject to later complete artifact planning.

Surviving rows retain order; new nodes and references append in ID/enum order.
Removed IDs follow prior order. Graph results contain neither artifacts nor the
legacy snapshot and do not authorize deletion or state persistence. Later
reconciliation must preserve tagged legacy data and unrelated owners, then
validate the complete final combined state. No CLI or schema-1 behavior changes.
See
[lifecycle reference reconciliation](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#architecture-lifecycle-reference-reconciliation).

<a id="416-reference-driven-shared-owner-removal-planning"></a>

### Reference-driven shared-owner removal planning

The inactive `lifecycle::retire::plan_reference_removal` API accepts complete
combined state, exact reference removals and supplied file observations. It
reconciles graph retention first, then retires rule installations whose final
reference ended. Unsupported owned kinds cannot retire through this boundary;
unselected kinds and tagged legacy rows remain unchanged.

Surviving artifact owners retain their order. Consumers are recomputed from
surviving per-owner associations. Every affected physical location requires one
exact observation, including metadata-only changes. Invalid, duplicate, missing
or extraneous evidence and affected topology overlaps conflict. Whole-file
hash/mode changes and edited or malformed owned instruction regions conflict
before returning a plan.

Whole files remain unchanged while an owner survives and are individually
removed after final-owner retirement. Each affected aggregate is edited once,
retaining manual bytes, surviving bodies and the observed valid mode. Empty
aggregates remain present. Region metadata is updated and the complete final
state is validated. Referenced empty installations survive; empty retired
installations need no file action.

This pure boundary grants no qualified execution or native compatibility claim.
Add/update sharing is a separate opt-in supplied-value boundary in
[explicit shared-output reconciliation](0005-lifecycle-planning.md#420-explicit-shared-output-reconciliation).
The opt-in guarded application boundary now covers first and subsequent schema-2
writes, exact schema-1 backup validation, captured-source freshness, and
recovery through the qualified executor journal. The opt-in source-backed rule
adapter resolves explicit local/Git sources. Production commands and native
qualification remain later work.

<a id="417-captured-lifecycle-selection-preflight"></a>

### Captured lifecycle selection preflight

The inactive `lifecycle::selection::select_owned_nodes` library boundary
validates complete supplied combined state, then resolves all-record or
literal-name selection. Names match installed aliases, bundle names and
historical native names/declaring bundle labels. Selected owned bundles include
their complete reference-graph dependency and installation closure, retaining
prior node order.

Every selected legacy row, unsupported owned kind, unknown name or ambiguous
name blocks the complete selection. Unselected malformed history survives and
does not block explicitly disjoint owned work. Native history declaring any
selected owned bundle name blocks, including transitively selected bundles.
Duplicate legacy rows retain separate category/index diagnostics. Named requests
must contain nonempty unique names and must not be empty.

Ready results contain owned installation/bundle identifiers. Blocked results
contain descriptive candidates and all typed diagnostics; no subset is ready to
apply. Name diagnostics follow lexical order, legacy diagnostics follow original
category/index order, and unsupported kinds follow prior installation order. The
boundary performs no I/O, state mutation, ownership adoption, reference edit or
output planning. Source/scoped selection and CLI precedence are not represented.
It neither changes production schema 1 commands nor establishes native support.
See
[lifecycle selection](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#architecture-lifecycle-selection).

<a id="418-complete-single-owner-lifecycle-upsert-planning"></a>

### Complete single-owner lifecycle upsert planning

The opt-in `plan_upsert` coordinator accepts complete desired installation sets,
explicit prior selection, exact bundle/reference edits and supplied file
observations. It reconciles graph retention before planning mixed owned outputs
and validates the complete resulting state. Every selected surviving
installation requires a complete set; existing changes and graph retirements
require selection.

Empty audience-filtered sets retire outputs while retaining referenced
installation/source/requested-surface history. Desired roots must match the
installation exactly. Output consumers and disjoint exclusions must exactly
account for requested surfaces. Changed owner associations are rebuilt from the
complete sets; untouched ownership, references and tagged legacy rows survive.

Selection and new-name legacy/native blockers expose no successful subset. Known
frozen legacy entrypoint/resource namespace collisions block even when observed
files are absent. Graph/artifact errors return no partial plan. All affected
destinations, including no-op bytes, require exact observations. Affected shared
artifacts and new sharing remain unsupported; equal bytes do not establish an
adapter contract. Manual aggregate bytes and observed modes retain the mixed
planner's preservation contract.

This inactive pure boundary provides no source authenticity, qualified
filesystem or native authority. The separate shared entrypoint is specified in
[explicit shared-output reconciliation](0005-lifecycle-planning.md#420-explicit-shared-output-reconciliation).
This pure API retains first-error results; the opt-in session layer supplies
[complete selected-set reporting](#421-complete-selected-set-preflight-reporting).
Production CLI precedence remains pending. Opt-in inspection and preview provide
observational adapters. Guarded backup, application and recovery are available
through the opt-in
[application boundary](0008-execution-and-recovery.md#guarded-lifecycle-application).
Production schema and commands stay unchanged. See the
[lifecycle upsert design](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#architecture-lifecycle-upsert).

<a id="420-explicit-shared-output-reconciliation"></a>

### Explicit shared-output reconciliation

The inactive `compile_rule_desired_set_with_contracts` producer accepts explicit
target profiles covering exactly the installation's requested surfaces. Profiles
are unique, nonzero and match their target families. It reuses captured rule
compilation and returns the desired set plus exact per-artifact contracts
binding source manifest, normalized content and only that artifact's consumers
to the supplied revisions. Audience exclusions produce no artifact contracts. It
never fills revisions from defaults. Original producer and batch behavior is
unchanged. The separate
[bounded Codex instruction producer](0002-generation.md#captured-codex-instruction-boundary)
now returns instruction contributions, exact owner contracts and a reader report
for captured unconditional rules. The explicit Codex source route now connects
that producer to guarded orchestration; production command activation remains
separate work.

The inactive `plan_shared_upsert` entrypoint uses the same complete desired
sets, selection, graph authority, legacy checks and evidence propagation as
`plan_upsert`. Exact scope, raw root, path and optional contribution selector
identify an artifact. Surviving artifacts and owners retain relative order; new
identities append in key order. Immutable real-owner bindings remain enforced.
Conflicting desired bytes, modes or instruction bodies reject. Untouched owners
cannot implicitly acquire changed content or inferred proof.

Every affected artifact receiving desired output with multiple final owners
requires complete valid contracts with exact source-manifest and complete
surface/revision-map agreement. Unknown unselected evidence blocks; complete
newly supplied evidence may replace unknown selected history. Single-owner plans
require no invented evidence. Retirement removes released owners' evidence while
retaining surviving history. Pure owner release with no desired value at the
identity preserves surviving content, associations and evidence unchanged,
without requiring missing contracts, even when multiple owners remain.

Private per-artifact projection reuses the mixed physical planner after real
ownership and agreement validation. Exact supplied observations are required for
all affected locations, including no-ops. Unknown dedicated files and unmanaged
markers cannot be adopted. Instruction changes preserve manual bytes and
observed modes; unchanged shared neighbors may retain unknown body proof while
aggregate layout metadata refreshes. Final contribution removal keeps an empty
aggregate. Only reconciled content metadata maps back onto authoritative real
owners; internal projection identities never escape to state, contracts or
references or error diagnostics. Physical errors name authoritative real owners
and preserve actionable location/cause text. Complete final validation precedes
return and failures expose no partial plan.

Inputs remain unchanged. This supplied-value path grants no filesystem
containment, source authenticity, native behavior or execution authority.
Production commands, schema 1, original upsert and public mixed-planner sharing
rejections remain unchanged. Guarded application and executor recovery are
available through an opt-in library boundary; command activation, client
adapters and native qualification remain later work. See the
[shared upsert design](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#architecture-lifecycle-upsert-explicit-shared-upsert).

### Combined owned-content lifecycle contract

The opt-in library lifecycle creates ownership only through explicit new
installation. Fresh and migrated add, update and remove preserve unrelated owned
references and indexed tagged legacy/native rows without adopting legacy
outputs. Unsupported or malformed retained rows remain visible. Selection that
requires unsupported or ambiguous legacy work stops before effects; explicitly
disjoint owned work can proceed.

Complete selection and planning precede application. Changes to affected owned
whole files or contribution regions conflict without partial application.
Instruction edits retain manual bytes and untouched contributions. Removal
changes only unchanged individually enumerated owned files whose final reference
has ended; extra resources and unowned directories survive.

List, doctor and preview remain observational: they preserve exact state bytes
and create no migration, backup, journal or output changes. Applying operations
reject changed state, source or destination evidence and require the matching
qualified scope writer guard. A competing cooperating writer cannot apply in
that scope concurrently. Backup and interruption requirements are recorded in
[combined lifecycle backup and recovery](0008-execution-and-recovery.md#combined-lifecycle-backup-and-recovery).

The source-backed rule adapter and supplied instruction adapter exercise this
contract without activating production commands. Production writes remain schema
1 until the Codex adapter and selector gate is complete. Local acceptance
coverage and its limits are recorded in the
[Gate 5 acceptance matrix](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate5-final-acceptance.md).
The combined implementation passed local full `just verify`, explicit Rust 1.85
checking and an independent audit of all ten acceptance criteria. Final delivery
review, hosted CI and merge remain delivery requirements; this contract does not
declare Gate 5 delivered.

## Architecture

### Combined lifecycle integration

The library connects one authoritative combined-state envelope, reference graph
and owner contracts to captured compilation, complete desired sets and the mixed
owned-artifact planner. It introduces neither an ownership sidecar nor a second
writer or recovery protocol. The typed indexed legacy projection preserves each
original row and source label; unknown original JSON fields remain in the exact
schema-1 backup rather than the typed projection.

For applying requests, the caller acquires the qualified scope writer guard. The
captured session recovers any pending executor operation before source work;
every recovery report stops the new request. Complete selection preflight then
rejects unsupported selected work before resolution. Explicit rule-source
orchestration retains local/Git captures, actual commits, compiler bindings and
owner contracts throughout scoped preparation and consuming application.
Explicit Codex instruction sources now use the same guarded orchestration engine
through `with_prepared_codex`. Caller-supplied contributions remain available
through the captured adapter. Both paths observe affected destinations, plan the
complete operation and revalidate exact state, source and destination evidence
before applying through the same origin-bound qualified executor.

Migrated origin binds the original schema-1 bytes to a restricted create-only
backup. Fresh origin requires absent state and backup without creating imaginary
history. Ordered executor actions persist file effects followed by combined
state; recovery uses those persisted actions without fetching or recompiling
sources. Inspection and captured preview are separate observational boundaries
and never recover, fetch, migrate or write. See the component interfaces below
and
[origin-bound application](0008-execution-and-recovery.md#origin-bound-lifecycle-application).

The
[acceptance evidence](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate5-final-acceptance.md)
maps all ten criteria to executable examples, including 162 process
interruptions and two late-edit reconciliation cases. Accounted staging residue
remains explicit debt #316; this evidence does not prove hardware power-loss
durability or native client discovery. Production command dispatch and
`CURRENT_SCHEMA = 1` stay unchanged. Gate 6 must qualify the independent
Codex/OpenCode plugin rule routes and selectors, all customization
reader/isolation controls and compatible commands before production activation;
the experimental contribution adapter is not the production route.

Satisfies:
[combined owned-content lifecycle contract](#combined-owned-content-lifecycle-contract).

<a id="architecture-lifecycle-selection"></a>

<a id="architecture-lifecycle-selection-captured-lifecycle-selection-preflight"></a>

### Captured lifecycle selection preflight

The opt-in `lifecycle::selection::select_owned_nodes` library boundary accepts
borrowed validated `CombinedState` and `Selection::All` or `Selection::Names`.
It returns `SelectionOutcome::Ready(SelectedNodes)` or `Blocked` with
descriptive candidates and typed diagnostics. Invalid entire state or malformed
requests return errors. It performs no I/O or input mutation.

<a id="architecture-lifecycle-selection-literal-selection-and-graph-closure"></a>

#### Literal selection and graph closure

Names match installed aliases, owned/legacy bundle names, and legacy plugin/MCP
names or declaring bundle labels. Matching is literal: source names, source
labels, scoped identities, glob patterns and CLI syntax are not interpreted. A
request must contain nonempty unique names and must not be empty. Unknown names
and names matching multiple rows block instead of choosing a first match. One
native row counts once even when its own and bundle names both match.

Selecting an owned bundle includes its authoritative dependency and installation
closure. Shared dependency paths are visited once. Node results retain prior
installation/bundle order, independently of request or traversal order.
Selecting an installation alone does not select its parent bundles or other
members.

<a id="architecture-lifecycle-selection-complete-blockers-and-preserved-history"></a>

#### Complete blockers and preserved history

All selection includes every owned node and typed legacy item, bundle, plugin
and MCP row. Every selected legacy row blocks, including duplicate, malformed,
unsupported or apparently identical rows. Legacy indices address the unchanged
original vectors; no source label is reparsed and no ownership is inferred.
Historical native rows declaring a selected owned bundle name also block,
including bundles reached transitively. This conservative label association is
not proof of native identity or an instruction to apply historical native
effects.

Unselected malformed legacy history does not block explicitly disjoint work.
Only selected Rule installations pass the owned-kind check; selected Skill or
Agent installations block even with no artifacts. Unsupported descendants of a
selected owned bundle block too. All blockers are collected before returning;
Blocked candidates never identify an independently ready subset.

Name diagnostics are in lexical requested-name order, followed by selected
legacy items/bundles/plugins/MCPs in their original category/index order, then
unsupported installations in prior order. Candidates include all matching owned
nodes and their bundle closure even when other selection work blocks.

<a id="architecture-lifecycle-selection-boundary-and-later-integration"></a>

#### Boundary and later integration

Ready means this selection preflight passed. It establishes no source
authenticity, filesystem containment, native support, compatible sharing, or
authority to remove references or apply outputs. Opt-in orchestration enforces
this outcome and connects complete desired-set/reference planning, qualified
observation and execution. Its target precedence is nonempty invocation,
nonempty project configuration, nonempty global configuration, then recorded
update targets or caller-supplied add defaults. Empty lists do not override the
fallback. Production source/scope selectors and CLI selection behavior remain
unchanged. Production schema remains 1. The captured command adapter now
connects this preflight to guarded application and executor recovery. Local full
Gate 5 acceptance is recorded in the
[acceptance matrix](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate5-final-acceptance.md);
final delivery, M1 activation and Codex qualification remain pending.

<a id="architecture-lifecycle-selection-components"></a>

#### Components

- [Pure selector](https://github.com/driftsys/upskill/blob/main/src/lifecycle/selection.rs)
- [Public acceptance examples](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_selection.rs)
- [Dated validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate5-selection-validation.md)

Satisfies:
[captured lifecycle selection preflight](#417-captured-lifecycle-selection-preflight).

<a id="architecture-lifecycle-reference-reconciliation"></a>

<a id="architecture-lifecycle-reference-reconciliation-lifecycle-reference-reconciliation"></a>

### Lifecycle reference reconciliation

`lifecycle::reconcile::reconcile_references` is an inactive pure graph boundary.
It consumes a validated `CombinedState` and explicit `GraphChanges`, and returns
`ReconciledGraph`. It performs no I/O, source fetch, ownership adoption,
migration, backup, application or schema-2 production write.

<a id="architecture-lifecycle-reference-reconciliation-interfaces-and-components"></a>

#### Interfaces and components

The public
[reconciliation module](https://github.com/driftsys/upskill/blob/main/src/lifecycle/reconcile/mod.rs)
exposes `GraphChanges`, which carries node upserts and exact edge edits, and
`ReconciledGraph`, which carries surviving graph records and removed IDs. The
private
[reachability traversal](https://github.com/driftsys/upskill/blob/main/src/lifecycle/reconcile/reachability.rs)
indexes bundle dependencies and memberships and follows direct roots using an
iterative queue. It keeps installation and bundle reachability separate even
when IDs have the same spelling.

The boundary reuses the combined-state validator and whole-file binding checks.
Validation-only graph values omit artifacts, consumer associations and legacy
state after the complete prior state has been validated. Those temporary values
do not replace the complete state or grant authority to apply effects.

<a id="architecture-lifecycle-reference-reconciliation-authority-and-retention"></a>

#### Authority and retention

Only `DirectInstallation`, `DirectBundle`, `BundleInstallation` and
`BundleDependency` edges establish retention. An installation survives while any
direct or transitive bundle path reaches it. Removing one root of a diamond
retains shared dependencies reached through another root. Artifact count,
audience-filtered empty output, consumers and matching source labels are not
retention references. Bundle and installation IDs remain separate namespaces.

The result contains installations, bundles, references and removed IDs. It
contains no artifacts or tagged legacy snapshot and is not a final combined
state. A removed installation ID does not authorize deleting its files. Later
lifecycle planning must reconcile complete ownership, retain unrelated owners
and legacy data, and validate the complete final state before effects.

<a id="architecture-lifecycle-reference-reconciliation-explicit-edits"></a>

#### Explicit edits

`GraphChanges` supplies installation/bundle upserts and exact reference
additions and removals. Duplicate upserts/edits, adding an existing reference,
removing an absent reference or adding/removing the same reference in one
request conflict. An empty changes value is a no-op; this exact edit API does
not infer intent from an incomplete replacement reference list.

Existing installation IDs retain source, kind, source name, installed alias,
scope and raw root spelling. Even a spelling-only root change conflicts.
Requested surfaces and captured evidence/revision may change subject to state
validation; later artifact planning must establish matching complete outputs.
Existing bundle IDs retain source and name, while revision evidence may change.
New IDs cannot bypass installation identity uniqueness.

Every supplied node and edge is validated before reachability pruning. Private
validation-only direct roots allow the existing state validator to check all
candidate nodes, dangling edges and cycles without treating intentional detaches
as unreachable-state errors. Those roots never escape or participate in actual
retention. Explicitly upserted nodes and added references must survive in the
result; conflicting update-and-final-detach requests fail rather than silently
discard updates.

Iterative traversal uses indexed dependency and membership edges. Unreachable
prior nodes and their outgoing edges are removed. Surviving prior nodes and
references retain order; new nodes append in lexical ID order and new references
in enum order. Removed ID lists follow prior record order. Final graph
validation runs again without validation-only roots. Inputs remain unchanged on
success and failure; there is no successful partial result after an error.

<a id="architecture-lifecycle-reference-reconciliation-remaining-integration"></a>

#### Remaining integration

The complete prior state is validated, including artifact ownership/consumer
associations and tagged legacy schema. Graph reconciliation never mutates those
prior values and does not reconstruct them in its graph-only result.

The separate [removal planner](#architecture-lifecycle-reference-removal)
reconciles shared-owner retirement. The pure
[selection preflight](#architecture-lifecycle-selection) aggregates selected
legacy and native conflicts for all/literal-name requests. The pure
[single-owner upsert coordinator](#architecture-lifecycle-upsert) now connects
complete desired sets and selected graph changes to mixed-artifact planning. The
separate
[explicit shared upsert](#architecture-lifecycle-upsert-explicit-shared-upsert)
now consumes recorded contracts and supplied observations. Source/scoped
selectors and CLI precedence and complete command lifecycle integration remain
task 2b work. Codex rule source integration now has an explicit library route.
Existing mixed and instruction planners keep their affected multi-owner
rejection. Guarded lifecycle backup/application/recovery and observational
adapters are available through separate library boundaries; complete command
integration and native activation remain separate gates. This boundary alone
does not complete task 2b, Gate 5, M1 or the updater prerequisite.

<a id="architecture-lifecycle-reference-reconciliation-trace"></a>

#### Trace

Requirements are recorded in
[authoritative lifecycle reference reconciliation](#415-authoritative-lifecycle-reference-reconciliation).
The
[public acceptance suite](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_references.rs)
establishes retention, edit rejection, binding immutability, deterministic
output and input preservation.
[Validation evidence](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate5-reference-validation.md)
records local checks and their limits.

<a id="architecture-lifecycle-reference-removal"></a>

<a id="architecture-lifecycle-reference-removal-lifecycle-reference-removal-planning"></a>

### Lifecycle reference removal planning

The opt-in `lifecycle::retire::plan_reference_removal` boundary connects
[reference reconciliation](#architecture-lifecycle-reference-reconciliation) to
complete shared-owner retirement. It accepts validated combined state, exact
reference removals, and caller-supplied file observations. It returns complete
combined state and individual `PlannedFile` values without I/O or input
mutation. The public `ReferenceRemovalPlan` contains `state: CombinedState` and
`files: Vec<PlannedFile>`; the function takes borrowed prior state, a slice of
exact `Reference` removals and a slice of `ObservedFile` values.

<a id="architecture-lifecycle-reference-removal-retention-and-ownership"></a>

#### Retention and ownership

Graph reachability decides which installations retire. Artifact counts and
consumer counts establish no graph authority. Removing one bundle path retains
an installation reached by another path. Referenced empty installations survive;
retiring empty installations produces no file action.

Only rule installations may retire through this M1 boundary. Unselected
unsupported kinds and tagged legacy rows remain unchanged. Additions, updates
and new sharing contracts are outside this API.

For each retained artifact, owner order is preserved and consumers are the union
of surviving authoritative per-owner associations, in prior consumer order. No
equality of rendered bytes is used to establish new sharing. Final combined
state validation includes graph, ownership, associations and tagged legacy.
Retained artifact and association rows keep their prior order. Tagged legacy
rows retain their order, duplicate entries and historical source labels exactly.

<a id="architecture-lifecycle-reference-removal-file-evidence-and-effects"></a>

#### File evidence and effects

Each affected physical location requires exactly one present observation,
including shared-owner changes whose bytes are unchanged. Location identity uses
exact scope and raw root spelling. Duplicate, missing, extraneous, malformed and
invalid-mode observations conflict. Ancestor/descendant artifact paths involving
an affected location conflict, including conflicts with untouched artifacts.
Actions are ordered by scope, raw root and relative path and retain the complete
observed destination and before-value. A conflict at any location returns an
error without exposing a partial plan.

Whole-file hashes and recorded modes must match the observation. A surviving
owner retains the observed file unchanged; final-owner retirement emits an
absent after-value for that individual file.

For each aggregate instruction file, every recorded region is inspected,
including untouched regions at that location. The aggregate editor runs once
with the retained bodies. It preserves manual bytes and retained body text,
updates retained marker metadata, and leaves an empty aggregate present.
Aggregate mode is preserved from valid observed evidence: individual region
records store no prior mode. Unknown markers, missing markers and edited owned
regions conflict. Manual text may have changed before observation.

These supplied values establish no qualified filesystem provenance. Application
must later use guarded, revalidated observations and the existing executor. No
production CLI dispatch, schema-2 write, backup, recovery or native target
activation is introduced. Existing mixed/instruction planners retain their
conservative multi-owner rejection.

<a id="architecture-lifecycle-reference-removal-components"></a>

#### Components

- [Retirement boundary](https://github.com/driftsys/upskill/blob/main/src/lifecycle/retire/mod.rs):
  graph-driven state and per-owner retirement.
- [File planning](https://github.com/driftsys/upskill/blob/main/src/lifecycle/retire/files.rs):
  location evidence and aggregate/file actions.
- [Acceptance examples](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_retirement.rs):
  public contract coverage.

Satisfies:
[reference-driven shared-owner removal planning](#416-reference-driven-shared-owner-removal-planning).

<a id="architecture-lifecycle-upsert"></a>

<a id="architecture-lifecycle-upsert-complete-lifecycle-installation-upsert-planning"></a>

### Complete lifecycle installation upsert planning

The inactive `lifecycle::upsert::plan_upsert` boundary joins complete desired
installation sets, explicit selection, authoritative reference reconciliation
and the existing mixed-artifact planner. It receives `CombinedState`, an
`UpsertRequest` and supplied `ObservedFile` values. It returns either
`UpsertOutcome::Ready(Box<UpsertPlan>)` containing complete state and file
plans, or `Blocked(Vec<SelectionDiagnostic>)`. Invalid state, graph edits,
desired sets or artifact evidence return an error with no partial plan. No I/O
or input mutation occurs. Production commands and lockfiles remain on schema 1.

The working plan declared an unboxed ready payload. The implemented payload is
boxed to satisfy the outcome-size lint; its state and file planning contract is
unchanged.

<a id="architecture-lifecycle-upsert-selection-and-references"></a>

#### Selection and references

`UpsertRequest` contains optional prior selection, complete desired sets, bundle
upserts, explicit owner contracts and exact added/removed references.
Installation graph upserts derive only from the complete sets. `None` selects no
existing nodes and permits new roots. Existing installation and bundle upserts
require selection. Edits to an existing direct root or bundle's outgoing
references require that root to be selected. Adding a new bundle may reference
an untouched existing installation; that reference does not replace its
artifacts or source history.

Every selected surviving installation needs a complete desired set. Nodes
retired by graph reachability must also be selected. Selecting a bundle uses the
existing complete dependency/member closure. Reconciliation preserves identity
bindings and retention independently of artifact counts. Unsupported selected
owned kinds block; this coordinator accepts rule installations only.

Prior-selection blockers and legacy/native blockers for supplied installed or
bundle names are collected without exposing a successful subset. Repeated
identical diagnostics appear once. Prior-selection diagnostics retain the
selector output order: lexical requested-name diagnostics, original legacy
category/index order, then unsupported installations in prior order. Additional
legacy name blockers append in original category/index order. Unknown names from
the new-name collision probe do not prevent adding new roots. This probe does
not interpret source/scoped selectors or command-line syntax.

<a id="architecture-lifecycle-upsert-complete-output-sets-and-associations"></a>

#### Complete output sets and associations

Desired files and instruction contributions must use the installation's exact
scope and raw root spelling. Each artifact has nonempty unique consumers. The
union of artifact consumers and unique audience exclusions must exactly equal
requested surfaces; an excluded surface cannot also receive output. These are
supplied-value consistency checks, not proof that the source actually declared
the exclusions or that native clients discover the outputs.

An empty set retires prior output ownership while retaining a referenced
installation, its source evidence, requested surfaces and graph edges. The mixed
planner's `Remove` value is used only for output retirement here; graph
reconciliation independently decides installation retention. A new empty set can
retain history without any file action.

The coordinator rebuilds changed per-owner consumer associations from the
complete sets. Untouched rows retain order and values. Replaced association
identities retain relative order among surviving rows; new rows append in
lexical scope, raw root, path, optional contribution ID and installation ID
order. Final combined state validation checks graph, artifact ownership and
exact consumer unions. Tagged legacy snapshots remain unchanged, including
duplicate and malformed rows and native statuses.

<a id="architecture-lifecycle-upsert-destination-evidence-and-sharing"></a>

#### Destination evidence and sharing

The existing mixed planner produces all affected file actions in one complete
plan. Exact before observations are mandatory even for no-op bytes. It preserves
manual aggregate text and observed aggregate modes and rejects edited ownership,
unmarked dedicated files, topology conflicts and malformed observations.

Known frozen legacy entrypoints and resource namespaces also conflict with
affected old or desired paths, including missing observed files and different
aliases. Component boundaries distinguish `old` from `oldish`. Historical rows
lack qualified scope/root bindings, so this check conservatively compares
relative destinations across supplied roots. Missing candidate information is
not filled in by inferring ownership; later source/destination qualification
must establish the facts needed for application.

In `plan_upsert`, affected multi-owner artifacts and new shared artifact
identities remain unsupported. Unaffected shared rows survive disjoint changes.
Equal rendered bytes cannot establish compatibility. The separate
[owner-contract boundary](0004-state-and-ownership.md#architecture-lifecycle-owner-contracts)
now records and checks explicit source, content and adapter-revision evidence.
The separate `plan_shared_upsert` entrypoint consumes that evidence with
supplied observations as described below. Shared retirement also remains
available through the separate removal API.

<a id="architecture-lifecycle-upsert-explicit-shared-upsert"></a>

#### Explicit shared upsert

`plan_shared_upsert` accepts the same request and observations and returns the
same outcome as `plan_upsert`. Both entrypoints share selection, complete-set
validation, graph retention, legacy collision checks and contract propagation.
The original entrypoint and the public mixed planner retain their conservative
sharing rejection. The shared entrypoint is an inactive supplied-value API;
source and filesystem qualification remain outside this boundary.

Candidate artifacts use exact scope, raw root, path and optional contribution
selector. Surviving artifact identities and real owners retain relative order;
new identities and owners append in key order. Changed installations supply
complete sets. Untouched owners keep their binding, body/file content and
contract evidence. Adding an owner cannot silently change retained content.
Desired file bytes and modes or instruction bodies at one identity must agree.
Real immutable owner bindings are checked before physical projection.

After candidate whole-state validation, every affected artifact receiving
desired output with multiple final owners requires complete matching contracts
for all final owners. Full source-manifest digests and complete surface/revision
maps must agree. Missing unselected evidence blocks the operation. Selected
owners may replace unknown history with complete newly supplied evidence; no
default revision is inferred. Single-owner results do not require invented
proof. Retirement removes only the released owners' proof. A pure release with
no desired value at that identity retains all surviving owners and evidence
unchanged, including unknown proof, without requiring agreement. Physical
observations remain mandatory. Unaffected shared neighbors may retain unknown
evidence.

<a id="architecture-lifecycle-upsert-private-physical-projection"></a>

#### Private physical projection

The private sharing planner assigns one deterministic internal owner to each
logical artifact identity in the union of prior and candidate state. It
delegates physical planning to `operations::plan::plan_owned_artifacts` after
validating real ownership and agreement. No alternate reference authority or
filesystem writer is introduced. For affected retained output without desired
bytes, exact supplied observations provide the existing body or file; the
existing planner verifies ownership and recorded content.

The existing planner retains observation, no-op verification, topology,
permission and marker guards. Unknown dedicated files and unmanaged markers
cannot be adopted. Shared instruction changes preserve exact manual text and
observed modes. Untouched contribution bodies retain their proof, while
aggregate whitespace metadata may refresh. Removing the final contribution keeps
an empty aggregate file.

The projection maps only normalized content and instruction layout metadata back
to the real candidate rows, requiring exact identity count and content
agreement. Internal owner identities never enter returned state, contracts or
references or public error messages. Physical errors translate complete private
owner contexts into real owner lists. Typed instruction locations retain exact
scope/raw-root attribution. Retained aggregate extraction reports the exact
physical file ownership grouped by selector, including neighbors verified by the
reader. Literal destination and cause text are preserved. Final complete
validation precedes return. Errors expose no partial plan and inputs remain
unchanged. This projection supplies no execution, source-authenticity,
physical-containment or native authority.

<a id="architecture-lifecycle-upsert-remaining-gate-5-work"></a>

#### Boundary limits and later work

These supplied-value boundaries plan supported rule outputs and explicit shared
reconciliation. Their graph and artifact APIs retain first-error results. The
opt-in session layer provides
[complete safely observable selected-set reporting](#421-complete-selected-set-preflight-reporting).
The [captured command adapter](#captured-lifecycle-command-adapters) connects
supplied source snapshots, qualified destinations and guarded application. The
[source-backed adapter](#source-backed-lifecycle-orchestration) now resolves
explicit rule roots and applies invocation/configuration/recorded target
precedence. Codex rule source integration now has an explicit library route.
Source/scoped selectors, registry discovery and production CLI integration
remain. Captured [preview](#read-only-lifecycle-preview) is available as an
opt-in supplied-value boundary. Read-only captured list/doctor and qualified
scope inspection are available through the
[inspection boundary](#read-only-lifecycle-inspection). The opt-in
[guarded application boundary](0008-execution-and-recovery.md#guarded-lifecycle-application)
now provides migration-only first-write backup, fresh backup-absence guards and
executor/recovery integration. Gate 6 must qualify native targets before
schema-2 production activation. This pure plan grants no execution, source
authenticity, physical containment or native support authority.

<a id="architecture-lifecycle-upsert-components"></a>

#### Components

- [Coordinator](https://github.com/driftsys/upskill/blob/main/src/lifecycle/upsert.rs)
- [Acceptance examples](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_upsert.rs)
- [Validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate5-upsert-validation.md)

- [Shared reconciliation implementation](https://github.com/driftsys/upskill/blob/main/src/lifecycle/sharing/mod.rs)
- [Shared acceptance examples](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_sharing.rs)
- [Shared validation limits](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-shared-reconciliation-validation.md)

## Read-only lifecycle inspection

The opt-in `lifecycle::inspection` library exposes `inspect_state`, `doctor`,
and `inspect_scope`. `inspect_state` accepts captured schema-1 or strict
schema-2 bytes, retains those exact bytes, and exposes validated combined
records alongside indexed legacy rows and diagnostics. Unknown schema-1 JSON
fields remain in the captured bytes rather than the typed projection.

`doctor` consumes supplied destination observations. It distinguishes missing
evidence from an observed missing file, checks whole-file hashes and recorded
modes, and verifies the complete set of recorded Markdown regions at each
location through the existing aggregate verifier. A shared aggregate validation
failure is reported against each recorded contribution at that location. Manual
bytes outside owned regions remain outside the content hash. Duplicate
destination observations are rejected; extra observations are permitted.
Whole-file mode checks apply only when state records an expected mode; missing
mode evidence then reports `ModeUnknown`. Markdown contribution checks do not
compare file modes. Legacy diagnostics remain visible in the inspection report;
legacy output is neither adopted nor checked as owned content.

`inspect_scope` captures the state and owned destinations through the qualified
filesystem boundary and revalidates both snapshots before returning. It reports
validated journal presence through `QualifiedScope::has_pending_operation`
without taking a writer lock, creating execution storage, fetching sources,
writing backups, or performing recovery. A completed journal awaiting cleanup
also counts as pending. Malformed journals, unsafe destinations, foreign
artifact roots, and unsupported schemas return errors. These observations sample
the namespace and do not exclude concurrent writers or prove native discovery or
source freshness. Qualified inspection is supported on Linux and macOS.

Production command dispatch and schema 1 remain unchanged. The
[captured command adapters](#captured-lifecycle-command-adapters) and
[source-backed orchestration](#source-backed-lifecycle-orchestration) are now
implemented as opt-in library boundaries. Complete lifecycle delivery remains
Gate 5 work; this inspection slice does not complete Gate 5. Captured preview is
described [below](#read-only-lifecycle-preview).

`StateInspection` contains exact original bytes, validated `CombinedState`, and
an optional descriptive `LegacyProjection`. `ScopeInspection` contains optional
state, ordered artifact diagnostics, and the sampled `pending_recovery` flag. An
absent state file yields no state or artifact diagnostics while journal
inspection still runs. Diagnostic indices address the original artifact vector;
one artifact may report both content and mode drift. No report authorizes a
repair or ownership adoption.

Components:

- [Inspection implementation](https://github.com/driftsys/upskill/blob/main/src/lifecycle/inspection.rs).
- [Read-only journal observation](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/run.rs).
- [Acceptance examples](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_inspection.rs).
- [Validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-lifecycle-inspection.md).

## Read-only lifecycle preview

The opt-in `lifecycle::preview::preview` API consumes captured schema-1 or
schema-2 state bytes, an explicit `PreviewRequest`, and supplied output
observations. It delegates restrictive upserts to `plan_upsert`, explicit shared
upserts to `plan_shared_upsert`, and exact reference removals to
`plan_reference_removal`. Reference-removal requests identify graph edges; CLI
name/source selection and source resolution remain separate work.

A ready `LifecyclePreview` retains exact original state bytes, the complete
validated proposed combined state, and ordered `FilePreview` actions. Each
action retains the planner's exact location and before/after bytes and modes.
`FileChange` distinguishes creates, content-only changes, mode-only changes,
combined content/mode changes, removals, and unchanged actions. Reference and
owner changes can change state while leaving output bytes unchanged; those no-op
output actions remain visible when returned by the planner.

`StateChange` compares canonical validated records, so differences only in
serialization whitespace or canonical record order do not imply semantic
changes. Schema-1 input instead reports `UpgradeFromSchema1`: any subsequent
applying transition would require the exact-backup prerequisite. Preview does
not create that backup or perform a migration. Output actions exclude the final
state write and backup, which remain guarded application responsibilities.

Selection blockers return `PreviewOutcome::Blocked` with the existing typed
diagnostics and no partial actions. Invalid state, missing/edited output
evidence, and graph/artifact conflicts return an error without a successful
partial preview. The underlying planners retain their first-error behavior; this
preview does not introduce operation-wide conflict aggregation. Applying
source/session reporting has its
[own contract](#421-complete-selected-set-preflight-reporting). Unrelated typed
legacy/native rows survive in the proposed state, and unknown original schema-1
fields remain available in the retained bytes rather than the typed projection.

The adapter performs no filesystem access, source fetch, writer locking, journal
creation, application or recovery. Supplied observations do not prove current
filesystem freshness or source authenticity. The preview is descriptive, not an
execution capability; application must qualify evidence under its writer guard.
Production command dispatch and schema 1 remain unchanged. Captured command
adapters and explicit rule-source orchestration are implemented as opt-in
library boundaries. Complete lifecycle delivery and Gate 6 native qualification
remain pending.

- [Preview implementation](https://github.com/driftsys/upskill/blob/main/src/lifecycle/preview.rs).
- [Acceptance examples](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_preview.rs).
- [Validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-lifecycle-preview.md).

## Captured lifecycle command adapters

The opt-in
[command adapter](https://github.com/driftsys/upskill/blob/main/src/lifecycle/commands.rs)
connects the existing selection, shared upsert and reference-removal planners to
origin-bound qualified execution. It is a library interface for applying
requests; production command dispatch remains on schema 1. Source fetching and
native client discovery are outside this boundary.

### Session capture and recovery

Callers acquire the matching `QualifiedScope` writer guard before opening
`LifecycleSession`. Opening recovers the existing executor journal before
capturing canonical lockfile and backup evidence. Every recovery report returns
`SessionOutcome::Recovery` and stops the new request, including completed
recovery; callers inspect the report and open a new session before source work.
A ready session retains opaque metadata observations under the same guard.
Absent state establishes fresh origin, schema 1 establishes migrated origin
bound to its exact bytes, and schema 2 requires known origin. Fresh state
rejects a backup; migrated state validates the retained original backup rather
than accepting today's backup as new historical authority.

### Upsert preparation and supplied authority

`prepare_upsert` runs complete existing-selection preflight before invoking its
builder. The builder supplies complete desired sets, owner contracts, bundle
nodes and exact reference edits. Every desired installation's manifest digest
must match a retained `SourceSnapshot`. This proves captured association and
freshness, not compiler origin for arbitrary supplied output. Typed instruction
contributions remain caller-authorized values under ruling **R-G5-CAPTURED-1**;
a source orchestration adapter must bind its rule values to actual compiler
results. Alternatives for this supplied-value boundary were not documented.

The adapter enumerates old and desired output locations into qualified evidence,
then supplies only affected observations to `plan_shared_upsert`. File
permission selection updates desired content and matching owner contracts
together. Generated files start private, ordinary existing modes survive,
resource owner execution follows explicit `ResourceExecutionPolicy` (normally
`Disabled`), and instruction actions select generated modes from exact observed
before values. State, backup and executor-journal destinations cannot be content
outputs.

### Direct-root removal and consuming application

`prepare_remove` preflights the complete selection and derives only direct roots
whose literal installation or bundle name was requested. Bundle
dependency/member closure does not remove independent direct roots. A named
member retained only through a bundle returns `NoDirectReference`; the caller
can identify its declaring bundle. Reachability and owner retirement reuse the
existing planners, preserving shared survivors, manual instruction bytes,
unselected graph records and individually unowned extra resources.

A ready `PreparedOperation` privately retains its next state, file plan and
qualified destinations while borrowing its session and exact source snapshots.
Read-only accessors expose the proposed state and plan. Consuming `apply` cannot
accept replacement state, snapshots or guard: it rechecks metadata, destinations
and sources before calling the existing
[origin-bound writer](0008-execution-and-recovery.md#origin-bound-lifecycle-application).
That writer owns output/state checkpoints and recovery; no second journal or
writer is introduced.

[Acceptance examples](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_commands.rs)
cover fresh and migrated writes, update/removal, complete blockers, stale
captures, direct roots, shared ownership, permissions and reserved destinations.
The
[validation note](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-captured-lifecycle-commands.md)
records qualification limits. The
[source-backed adapter](#source-backed-lifecycle-orchestration) binds compiled
rule outputs to retained captures. Whole-lane interruption acceptance, hosted
delivery and Gate 6 native qualification are separate evidence and delivery
concerns.

Satisfies:
[captured lifecycle selection preflight](#417-captured-lifecycle-selection-preflight),
[complete lifecycle upsert](#418-complete-single-owner-lifecycle-upsert-planning),
[shared-output reconciliation](#420-explicit-shared-output-reconciliation), and
[reference-driven removal](#416-reference-driven-shared-owner-removal-planning).

## Source-backed lifecycle orchestration

### Requirements

The opt-in applying boundary opens the guarded lifecycle session and completes
recovery before any source work. A recovery report stops the current request.
Complete literal/all selection and target preflight precede resolution. Selected
legacy or unsupported nodes block the complete operation; unrelated legacy rows
remain preserved. Removal uses recorded direct references and ownership without
resolving sources or interpreting current preferences as history.

Explicit add requests identify each source rule name and installed alias.
Updates preserve installation IDs, aliases and the existing reference graph
while recapturing the requested source binding and recording actual Git commits
and exact manifest digests. Repeated add of the same binding reuses its recorded
ID and adds no duplicate direct reference. A different binding cannot reuse an
installed alias, and an add alias cannot collide with an owned bundle name. The
historical source route rejects instruction-backed owners, including
artifact-free Codex installations, before parsing or resolving sources. The
explicit Codex source route accepts its supported retained representation.
Recorded removal remains source-free. Caller-supplied complete graphs and
manual/shared instruction contributions remain on the captured command API.

Nonempty invocation targets override nonempty project configuration, then
nonempty global configuration. Update otherwise uses recorded targets; add uses
caller-supplied defaults. Empty selections are absent overrides. No effective
selection or duplicate requested targets rejects rather than inventing defaults.
Requested targets, audience exclusions and actual artifact consumers remain
separate. Compilation requires explicit compiler bindings and profiles and is
limited to the existing Claude, Copilot and OpenCode families on the historical
route. The separate Codex route accepts only Codex CLI revision 1. Native Codex
qualification and production schema-2 activation remain Gate 6 work.

### Architecture

#### Interfaces and preparation lifetime

[`lifecycle::orchestration::with_prepared`](https://github.com/driftsys/upskill/blob/main/src/lifecycle/orchestration.rs)
accepts a qualified scope, its caller-retained writer guard, installation root,
`SourceRequest`, `SourceOptions` and a callback. `SourceRequest` distinguishes
explicit add roots, selected updates and selected removals. `SourceOptions`
contains target precedence inputs, add defaults and a `CompilerContext` holding
compiler bindings, contract profiles and resource execution policy.

`OrchestrationOutcome` distinguishes typed blockers, recovery and `Ready(T)`,
which contains the callback result rather than proving application. The callback
receives opaque `SourcePrepared`; consuming `apply` revalidates all retained
captures, including Git identity, before the captured operation's own metadata,
destination and snapshot checks. Discarding preparation applies nothing. The
prepared operation cannot outlive the callback or accept substituted output.
Resolvers, private temporary checkouts and captures remain alive until the
callback returns. Existing permission selection keeps owner contracts consistent
with resulting file modes. The adapter introduces no separate writer or journal.

#### Installation identity

New IDs are full lowercase SHA-256 digests of a versioned JSON tuple containing
`upskill-installation-v1`, ownership scope, raw root bytes, normalized requested
source label, rule kind, source name and installed alias. Tuple serialization
frames component boundaries; actual resolved Git commits and manifest digests
are evidence outside identity. Exact prior bindings reuse recorded IDs,
including previous caller-selected IDs. A computed ID already bound differently
or multiple prior installations with the same binding rejects. Raw root spelling
remains identity-bearing.

The adapter selects existing matching owners for complete upsert and adds only
absent direct references. Alias preflight normalizes local ancestors while
leaving the selected final component for qualification. Updates retain graph
history; this module neither discovers registries nor constructs bundle graphs.

#### Acceptance and limits

[Integration examples](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_orchestration.rs)
cover local add/update/remove, branch advancement, exact revision and manifest
evidence, ID and alias preservation, repeated add, target precedence, stale
captures, manual output conflicts, disjoint legacy preservation, complete
selection blockers, audience exclusions and resource permissions. A private
identity example pins raw-root framing, collision rejection and reuse of prior
IDs independently of resolved commit evidence. The
[validation note](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate5-orchestration-validation.md)
records local checks and review evidence. These examples alone establish neither
whole-lifecycle interruption acceptance nor hosted delivery or native behavior.

#### Explicit Codex route and retained representation

`with_prepared_codex` shares session recovery, complete selection preflight,
source resolution, captured lifetimes and application with `with_prepared`.
`CodexSourceOptions` supplies target precedence and defaults, an explicit
profile and execution policy. Profile and effective selection failures return
typed Instructions capability errors before source resolution. Recovery and
Remove precede these checks; Remove uses recorded ownership even after source
deletion or with unavailable compiler options.

Every retained selected owner, including same-alias Add refreshes, must be a
rule requested exactly for Codex CLI. Its scope and raw root path must exactly
match the declared session root before any selected source is resolved. It may
own no artifacts when excluded, or one Markdown contribution at project
`AGENTS.md` or user `.codex/AGENTS.md`. That contribution must have the
producer-derived identity, sole installation ownership, exact Codex CLI
per-owner consumers and its matching revision-1 contract. Arbitrary identities,
multiple contributions, shared ownership, whole-file conversion and malformed
bindings reject before any selected source is resolved. This conservative rule
does not migrate hand-constructed or historical adapter state.

`SourcePrepared::instruction_readers()` exposes complete per-installation
reports sorted by installation ID, including excluded output with NotEvaluated
compatibility. Historical and Remove callbacks expose no newly compiled reports.
Potential incidental readers never enter ownership, references, contracts,
combined state or execution receipts. A caller can inspect or clone reports
before applying, or discard preparation without publication.

The declared root determines the exact destination. Qualified filesystem access
does not establish native user-home selection or client discovery. All selected
sources compile before the callback, and application rechecks original capture
bytes and actual Git revision. Local/Git project/user examples, stable exclusion
and reinclusion, complete preflight failures, manual bytes/modes, and a
revision-only private-checkout mutation are covered by
[`lifecycle_codex_orchestration.rs`](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_codex_orchestration.rs).
Production commands and schema 1 remain unchanged; native qualification,
all-kind compatibility, activation and staging accounting in #316 remain
separate gates. Complete independently observable preflight reporting is
implemented at the opt-in library boundary described below.

<a id="421-complete-selected-set-preflight-reporting"></a>

## Complete selected-set preflight reporting

### Requirements

Source-backed Add and Update collect independently observable binding,
resolution, source-capture, compilation, capability, destination and ownership
failures across the complete selected set. An invalid source does not hide
conflicts in independently compiled sources. An item that fails one preparation
stage supplies no speculative downstream destination diagnostics.

Root qualification, writer-lock acquisition, recovery, invalid authoritative
state and unsafe destination observation remain operation-level boundaries.
Recovery stops the new request. Invalid retained root or representation
authority is rejected before selected sources are resolved. Collection stops
when safe inspection cannot proceed; a report is complete only for observations
that can be obtained safely. The existing selection blockers retain their own
typed outcomes.

A blocked request cannot invoke the prepared callback, create a new operation
journal, publish outputs, migrate state or execute a successful subset. Valid
sharing remains permitted under the existing complete owner-contract agreement.
Retained owners, manual instruction text, exact modes, unrelated references and
schema-1 backup evidence remain protected by the existing planner and executor.

### Architecture and report contract

`CommandDiagnostic::Preflight(PreflightReport)` carries structured diagnostics
and available compiled instruction-reader reports inside the existing `Blocked`
outcome. Diagnostics contain installation participants, category, optional
location and explanatory text. Participant IDs are sorted and deduplicated.
Diagnostics sort by participant IDs, then category in the order Binding,
Resolution, Capture, Compilation, Capability, Destination, Ownership; ties sort
by scope, raw root, relative path and message. Only exact diagnostic records are
deduplicated. Error wording is descriptive rather than a stable machine code.

Update diagnostics use recorded installation IDs. Add diagnostics normally use
the same normalized requested binding as installation identity. If normalization
fails, tuple framing of the original requested source label supplies a
report-only identity; it never establishes source evidence or persisted state.
Duplicate selected aliases or identities are reported without choosing a winner.
Their invalid desired topology is excluded consistently before observation and
reporting; their recorded prior ownership remains protected.

Structural and reserved-path checks precede filesystem observation. Conflicting
locations are excluded from observation; unrelated safe locations remain
inspectable. The qualified session captures their union once and selects modes
before comparing desired file contracts. Location-level reporting retains every
recorded owner, consumer association and contract at that location, including
owners whose selected source failed preparation. It reuses shared ownership and
physical planning checks without constructing an applying operation. Structural
failures suppress speculative content diagnostics at their locations. A mode
failure likewise suppresses diagnostics that depend on a usable desired mode.

A complete captured request projects its authorized reference edits before
location reporting so explicitly retired owners do not appear as untouched
survivors. Duplicate bindings prevent that projection; failed-source subset
reporting never infers retirements. Only a complete clean request enters opaque
preparation. It retains the qualified destination snapshot through application,
which still revalidates metadata, source bytes, Git revisions and destinations.
Pure planners retain their first-error APIs; independent diagnostic collection
belongs to the orchestration/session boundary.

Compiled reader inventories sort by installation ID on blocked and ready
outcomes. Excluded content retains its NotEvaluated report; failed compilation
creates no report. Potential incidental readers grant no ownership, consumer
association, contract, reference or execution authority. Historical compilation
and recorded removal introduce no new instruction-reader inventory.

### Executable examples and limits

The following integration examples pin the reporting contract:

- `invalid_source_does_not_hide_two_destination_conflicts` reports one
  resolution failure and two independent occupied destinations, with no callback
  or effects.
- `unchanged_selection_does_not_hide_changed_conflict` retains a source-label
  parse failure alongside changed and unchanged selected-source conflicts and an
  exact unselected survivor.
- `complete_conflict_inventory`, `mixed_artifact_and_sharing_conflict_matrix`
  and `duplicate_binding_does_not_hide_disjoint_conflicts` pin complete
  participant inventories, mixed artifacts and independent conflict groups.
- `compatible_shared_files_and_distinct_contributions_have_no_conflicts` pins
  legal identical file sharing and distinct instruction contributions.
- `blocked_preflight_keeps_excluded_and_incidental_readers` pins both compiled
  reader roles and unchanged retained ownership.
- `instruction_mode_failure_does_not_hide_an_independent_source_failure` pins
  special-mode rejection alongside an independently failed source.

Run
`cargo test --test lifecycle_preflight --test lifecycle_orchestration
--test lifecycle_codex_orchestration --test lifecycle_commands`
to exercise these examples and the existing capture/authority regressions.
Process and special-mode fixtures require an environment that permits their OS
operations.

This is library-only G6-03 reporting support. Production commands still use
schema 1; production schema-2 migration, all-kind compatibility, native client
qualification and activation remain separate gates. The roadmap remains Gate 6,
then owned-skill lifecycle support, then automatic maintenance.

## Inactive skill ownership and native receipt planning

`lifecycle::skill_delivery::plan_skill_delivery` is a separate pure library
boundary. It consumes a `SkillDeliveryState` in-memory view, graph edits,
complete desired selection intent, captured package candidates, file
observations and native observations. It does not add a serializer, migration,
writer, command or native activation route. Existing schema-1 and schema-2
codecs remain unchanged; production state compatibility remains an admission
dependency.

The view holds one existing `CombinedState` logical installation/reference
graph, immutable revision captures and latest native receipts. Skill package
files are represented through those captures, not also as graph artifacts.
Non-skill installations cannot be changed through this planner, and unrelated
owned artifacts and legacy state remain untouched. Revision capture validation
checks source evidence, target/owner/root bindings and complete recompiled
package equality. Copied absolute-root state rejects. An explicit clone starts
with a fresh local view and new packages for that checkout, carrying no native
receipt or ownership authority across machines.

Direct and bundle reachability uses the existing reconciler. Complete intent
must explicitly select a body origin or unavailability for every surviving
skill/target; selected intent requires exactly one matching candidate. No
fallback is compiled after replacement removal. Shared references keep one
installation and one native binding per intended target. Historical captures
prevent an ID being rebound to another source, kind, source name or alias after
detachment.

Native receipts and observations bind installation, client, scope, version,
platform and operation/observation identity. Confirmed receipt poststate must
match the observation; unowned native presence cannot be adopted. Every
non-confirmed receipt, including rejection, and every unknown observation
returns a typed recovery-required error before a new plan. These captured claims
are not authenticated observations or runtime qualification. Refresh, detach and
reinstallation over retained revisions require supplied cache-preservation or
session-clearance evidence. Retaining export files alone is insufficient.

The result is a preview: reconciled graph, explicit desired intent, all retained
and new revision captures, proposed file ownership, separate immutable staging
and selector-file changes, and native transition requests. It emits no completed
state or successful receipt. Whole-file planning rejects missing/edited owned
files and identical unowned files, including for no-ops. All revisions remain
retained; native detachment never authorizes resource garbage collection.
Detached file ownership has no current source revision, while immutable captures
retain their exact historical source revisions. The result cannot be serialized
by simply combining it with the current schema-2 graph codec.

The later coordinator must stage immutable files before changing selectors,
coordinate native effects and observed outcomes, and commit ownership only
through a separately admitted integrated state representation. This boundary
neither adds a second authoritative installation lock nor qualifies that future
persistence format. See
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-keep-native-skill-planning-outside-current-codecs.md).

## Inactive native configuration contribution planning

`native_config::plan_config_patch` plans one explicitly owned contribution to a
captured JSONC or TOML document. Inputs bind the target file, installation root,
client surface, effective lower/higher layers and current per-selector
ownership. The returned `ConfigPatch` contains exact before/after file values
and updated in-memory ownership evidence. It performs no I/O and adds no command
or codec. New files select private mode `0600`; edits preserve captured existing
modes. Applying a patch still requires qualified file identity and complete
preconditions. Captured layer completeness and provenance are caller evidence,
not observations made by this pure function.

A selector identifies a key path or one semantic array entry. The first edit
cannot adopt an existing manual entry, even when its value is identical. Later
edits require the exact ownership witness and compare both semantic value and
raw-token digest. Conflicting selectors reject; distinct owned entries may share
one array. All documents are fully parsed, decoded duplicate keys reject, and
planned results are reparsed. Changes splice selected spans without reformatting
unrelated configuration or comments. A no-op still validates captured ownership.

After the last managed entry is removed, an unchanged complete document can
return to its exact original bytes or initial absence. Unrelated manual changes
invalidate that rollback shortcut. Removal then preserves remaining manual bytes
and comments, including empty parent containers. Owned-token edits or missing
entries reject. Ownership and rollback records are captured claims and confer no
native or filesystem authority.

Scalar selectors present in any inherited layer reject conservatively. Array
sources cannot duplicate inherited entries, and higher array selectors reject
until concrete adapter merge semantics are qualified. A project OpenCode target
must be an ignored, untracked `.opencode/opencode.json` or `.jsonc` overlay.
External write roots, including an external XDG root, reject at this single-root
boundary. Read-only inherited layers may be outside the selected root.

The TOML parser accepts legal unrelated dates, multiline strings, dotted keys
and arrays of tables. Editing is limited to scalar, array and inline-table value
tokens; whole tables and implicit inline selectors without token spans are not
editable. JSONC support does not change existing strict JSON handling. The
parse-only dependency and compatibility evidence are recorded in
[AD-0043](https://github.com/driftsys/upskill/blob/main/docs/adr/0043-parse-native-config-with-preserved-spans.md).
Native adapters, durable integrated state and client qualification remain later
work.

## Inactive native skill-file permissions

`native_delivery::plan_native_skill_files` reuses skill graph/receipt planning
with the fixed `native-skill-files-v1` permission policy. Generated exports use
`0600`. Resources require known source mode evidence and use `0700` for captured
owner execution, otherwise `0600`. Alias paths retain the same classification.
Captured selections, resource modes, package bytes and hash identities remain
unchanged; returned file ownership records the selected destination modes.

The planner consumes actual destination observations. Existing native exports
must match the fixed private policy; mode drift rejects even on a no-op. It does
not rewrite observations to match source modes or adopt identical manual files.
The earlier `plan_skill_delivery` captured-only view remains unchanged. Both
entrypoints retain the native cache/session-clearance guard before refresh.

This is a pure Stage 5 prerequisite. It supplies no state codec, journal, native
adapter, file effect or completed transaction. The remaining coordination
contract is specified in
[AD-0044](https://github.com/driftsys/upskill/blob/main/docs/adr/0044-coordinate-native-delivery-with-one-state-and-operation-intent.md).

## Inactive integrated native state codec

`native_delivery::state` encodes one validated graph, complete desired skill
intent, retained captures, confirmed native receipts and applied export
ownership in an explicit schema-4 envelope. Existing schema-1/2 codecs and CLI
dispatch remain unchanged. It does not migrate or write a lockfile, register a
client or complete a native transaction.

`serialize_native_state` returns JSON and immutable private evidence blobs.
Native prestate and full configuration/rollback snapshots stay in those blobs;
the JSON contains typed references with framed SHA-256 and length. A later
qualified store must publish them privately under the selected user root before
publishing the lockfile references. These blobs are evidence, not another
installation graph. `parse_native_state` requires the expected scope, private
root, local namespace and referenced evidence. Missing/corrupt evidence, swapped
kinds, copied contexts, unsupported schemas/policies and unknown structural
fields reject. Namespace/digest checks validate supplied claims; physical
qualification and native observation remain coordinator responsibilities.

The codec recompiles packages, checks exact applied ownership and confirmed
selection, and validates configuration ownership against private snapshots and
per-entry target bindings. It retains detached revisions without inventing
active graph nodes, preserving historical Git revisions separately from current
file ownership. Unrelated graph artifacts, legacy order and origin records
survive. Typed selected captures use YAML inside the JSON envelope to preserve
supported metadata structures and non-string mapping keys. Custom YAML tags on
additional metadata values remain unsupported by the portable source parser.
Duplicate JSON keys reject before decoding.

The codec grants no durable coordination or client admission. The private store
and pending markers are described below; effect coordination and client adapters
remain pending under
[AD-0044](https://github.com/driftsys/upskill/blob/main/docs/adr/0044-coordinate-native-delivery-with-one-state-and-operation-intent.md).

## Inactive native root binding and locking

`QualifiedScope::binding` revalidates a pinned directory and returns its
selected path, resolved path, device and inode. The returned record carries no
filesystem capability. `native_delivery::roots::lock_native_roots` requires
exact normalized logical root paths, rejects duplicate or aliased physical
directories and holds all writer guards in ascending device/inode order.
Acquisition is nonblocking; any failure releases previously acquired guards. The
returned lock set can revalidate all bindings while held.

This is live advisory writer exclusion on Linux/macOS. It does not exclude
non-cooperating editors, freeze path bindings, qualify filesystem profiles or
record durable operation intent. Locks release after normal exit or a crash.
Pending markers and recovery coordination remain required before native effects
can be admitted.

## Inactive qualified private native evidence

`native_delivery::store::NativeEvidenceStore` opens or initializes
`.upskill/native-evidence` beneath an exact qualified user root while borrowing
its writer guard. `open` creates nothing; `initialize` rejects unowned or
partial stores. The directory uses `0700`, files use `0600`, and `.gitignore`
excludes all entries. Force-tracked content rejects, including through ancestor
aliases. The existing APFS/ext4/tmpfs profile, descriptor, mount, metadata and
link checks apply; this API admits no native client profile.

The store derives its namespace from owned root/directory identities and a local
nonce, preserves immutable typed blobs and validates reference syntax before
path selection. A blob must match its framed kind, length and digest. Equal
existing bytes are verified and synced; corruption or changed ownership rejects
without replacement. Copied store records cannot become authority in another
directory. Namespace/root equality remains required when parsing state.

This private evidence is not another installation lock. `store_state_evidence`
validates supplied state, syncs its private blobs and returns reference-only
JSON. `load_state` checks its expected root/namespace and envelope declarations,
reads only referenced blobs and performs full codec validation. Unreferenced
blobs are neither loaded nor removed. These methods consume supplied state data;
they do not observe principal/additional root capabilities or native outcomes.

`NativeStateStore` binds that codec/store boundary to the full root-lock set. It
derives the expected root context, keeps additional logical roots in stable
scope/path order and captures the authoritative `.upskill-lock.json` through the
qualified primary scope. `load` returns its validated state or absence; an
unsupported schema rejects without migration. `open` describes private-store
availability, so a missing private store does not imply an absent installed
graph. `prepare_state_file` validates prior and supplied state, syncs private
evidence and returns exact file preconditions without applying them. It selects
`0600` for a new state file and preserves an existing ordinary mode. Binding or
captured-file changes during preparation reject.

A loaded state's declared additional roots may be a subset of the held roots.
Principal/private roots and the private namespace must match, and every declared
root must be held before evidence loading. State preparation can expand or
contract that declaration within the held set; operation recovery still requires
its exact recorded root set. The pure codec's expected-context check remains
exact.

The public state APIs publish no authoritative graph or operation journal and
retire no evidence. Durable intent and markers are described below. The inactive
native coordinator described later performs effect coordination and recovery;
production native admission remains pending.

## Inactive durable native preparation

`NativeOperationStore::prepare` captures exact prior state and owned request
inputs in a private typed YAML intent. It records the replayed
skill/configuration plans and a generation-zero `Prepared` journal under
`.upskill/native-evidence`. Both records are synchronized before publishing
`.upskill/native-pending/pending.json` in every held root. Marker directories
are owned, private and ignored; their purpose and namespace are separate from
the private evidence directory. The marker contains root/operation bindings and
a private journal locator, with no native prestate or manual configuration
bytes.

The operation store rejects an existing marker before creating marker storage in
another root. A new operation sharing only one marked secondary root requires
recovery even after the previous process exits. Captured file observations are
checked against qualified observations, and reserved storage/overlapping targets
reject. Inspection validates strict intent/journal formats, namespace/root
bindings, intent hashes and replayed plans; it reports partial marker coverage
without applying, retrying or deleting anything.

Preparation is not native completion. Native observations remain supplied
claims, and marker coverage grants no future effect authority. The inactive
native coordinator described later supplies effect and inverse execution,
observation-first recovery, state publication and terminal marker retirement.
Profile admission and complete delivery review remain pending. No production CLI
route or prior-schema migration is admitted here.

## Retained staging without a native binding

`SkillDeliveryState.staged` keeps a revision capture, origin operation ID and
the exact owned subset of immutable export paths. It selects no catalog and
declares no native binding. `revisions` remains separate; a detached revision
requires confirmed receipt lineage. Reachable revisions with unavailable intent
may have no current receipt, so a revision alone proves no native activation.
This allows partial exports to survive restore without fabricated activation or
detachment evidence. The applied ownership table remains in the single
authoritative state.

Planning validates and retains these paths with their real observed values. Only
a matching desired candidate plans promotion into bound revision history;
committing that promotion still requires independent native observation. Cache
preservation or session clearance is required for a native transition involving
retained staging. Unknown native presence cannot be adopted from a staging
record. No record authorizes resource deletion.

The inactive native state format is schema 4, and the private intent format is
schema 2. Earlier inactive schema-3 state and schema-1 intent are rejected
rather than silently reinterpreted. Production schemas 1/2, CLI dispatch and
native admission remain unchanged.

## Native effect recipe and checkpoint metadata

`NativeOperationStore::prepare_execution` calls only the adapter's read-only
profile and inverse planning methods. It validates complete markers and captured
file/state preconditions, then persists one ordered recipe: immutable exports,
before-native exposures, native effects and after-native exposures. Profiles
bind exact client/version/platform/scope attribution; shared exposures reject
ambiguous ordering. Inspection checks the recipe against immutable intent.

The journal moves from generation-zero `Prepared` to generation-one `Executing`.
`begin_execution_step` syncs the next `Queued` step as `Pending` before
returning it, rejecting skips or a second begin of the pending step. It grants
no replay or filesystem preflight authority. Native attempt outcomes remain
evidence; even `Confirmed` cannot acknowledge a step. `observe_pending_step`
uses actual file bytes/mode or an independent, attributed native observation.
Unknown, diverged and failed native observations stay pending with their history
retained. An observed native effect does not prove final effective binding.
These metadata APIs apply no files/commands and publish no completed state. The
separate inactive coordinator now connects effects and recovery as described
below; Stage 5 verification and native admission remain separate gates.

## Inactive native execution and recovery

`native_delivery::coordinator::NativeDeliveryCoordinator` runs the recorded
sequence under all held qualified roots. It captures native state freshly,
stages immutable exports, applies ordered owned exposure/native changes and
requires independent final bindings before authoritative state publication. Each
native call has a durable pending record with fresh precondition and
cache/session evidence. Command outcomes never replace observation.

`ObserveOnly` does not replay calls or advance files with a native Before-state.
Explicit `Forward` or `Restore` is required then. Unknown/foreign bindings,
manual changes, profile drift or unverifiable retention remain
`RecoveryRequired`. Restore uses owned inverses, preserves prior receipts and
retains new exports as staging without inventing a binding history. File-only
profiles use exposure plus independent observation and issue no native command.

State publication has its own durable action and observation checkpoint.
Terminal/retiring evidence precedes exact own-marker deletion; interrupted
publication or retirement resumes without native replay. Forward `Complete` and
inverse `Restored` require observed files/bindings, durable state and retired
markers. All old resources and operation evidence remain retained.

Several configuration contributions to one document form one physical patch,
with unique IDs, complete owner associations and one original rollback baseline.
Omitted prior owned documents receive no-op captures, so no-op cannot bypass
owned selector/layer drift. All supplied durable capture data must satisfy D6's
approved non-secret boundary. Private storage is not permission to journal
credentials or credential-bearing shared documents. Real profiles, their
capture/writer contracts, qualification and production CLI admission remain
unadmitted.

## Closed native profile admission

`native_delivery::admission::QualifiedNativeAdapter` wraps the existing adapter
trait with an exact-profile admission check. The production registry is empty;
all four CLI surfaces reject with a typed `NativeAdmissionError` before native
capture, inverse planning, execution or observation. A profile lookup remains
read-only. Every method rechecks admission, and effect methods require the
recorded profile to match the current profile.

Matching includes surface, client version, installation scope and root,
platform, profile ID/revision, exposure order and action kind. There is no
nearest-version, user-scope or alternate-surface fallback. The private registry
cannot be populated from caller-supplied claims. Future entries still require
reviewed qualification, resolved executable and invocation/environment
contracts, preservation controls and resource-retention evidence. This wrapper
alone does not validate those facts. Concrete client adapters and their
qualification remain pending, and the production CLI continues to use its
existing dispatch.

## Inactive native subprocess runner

`native_delivery::process` provides Linux/macOS subprocess mechanics for future
adapters. It does not implement a client adapter, qualify a route or affect
legacy plugin commands. A call requires its CWD's matching qualified writer
guard, a captured absolute launcher, explicit arguments/environment, a finite
poll deadline and a bounded combined output budget. The child receives closed
stdin and a descriptor-selected CWD. No parent environment values inherit.

Launcher capture opens nonblocking without following a new final symlink, checks
an ordinary executable descriptor and bounds hashing. Revalidation rejects
changed launcher bytes/identity before spawn; post-execution scope and launcher
changes remain explicit result fields. Capture does not pin every
interpreter/delegated runtime or eliminate the final validation-to-exec race.

The runner retains an unreaped leader while it signals the owned group, reaps
that child, then independently probes group emptiness. Timeout, cancellation,
output overflow and normal leader exit all trigger group cleanup. It never
signals that group after reaping its leader. macOS may deny a signal to a
zombie-only group; a subsequent empty-group observation remains separate from
the recorded signal result. Surviving/orphaned group bookkeeping is unresolved,
not assumed absent. Group emptiness covers only that group and proves neither
absence of escaped processes nor quiescence after parent death. Kernel calls and
final reaping have no hard-real-time guarantee.

Output stays bounded in memory. Command specs and results have redacted debug
representations and no serde representation; raw stdout/stderr and argument or
environment values must not be journaled. An exit code, a stopped group or an
intact export is not a native binding/retention receipt. Real adapters still
require durable Pending-before-invocation ordering, approved non-secret capture,
independent effect/source observations and separately qualified recovery.

## Claude inventory metadata prerequisite

`native_delivery::claude::ClaudePluginInventory` decodes the retained Claude
2.1.308 plugin-list wire shape without running a command. It bounds input,
rejects malformed UTF-8/JSON and decoded duplicate keys, requires the known
metadata fields and preserves user, project and other reported scope labels.
Unknown fields and raw parse errors are discarded; Debug exposes only the entry
count, and the inventory has no serde representation.

Exact lookup distinguishes plugin/catalog identity, user versus project scope,
and the reported project path. Duplicate matches and matching entries in an
unsupported/ambiguous scope reject. A false `projectEnabled` value stays
separate from `enabled`; neither is expanded into an effective activation claim.
Sanitized receipt path placeholders remain reported metadata and cannot identify
a real absolute project. Path alias resolution is not performed here.

These records establish no ownership, observer freshness, native binding,
resource retention or profile qualification. No matching row means only that the
supplied inventory has no match. Concrete adapters must still bind a successful
complete transport observation to its qualified runtime/context, validate the
selected source and independently establish effective behavior. Client mutation
methods, production admission and CLI dispatch remain pending.
