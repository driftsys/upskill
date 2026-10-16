# Complete lifecycle installation upsert planning

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

## Selection and references

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

## Complete output sets and associations

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

## Destination evidence and sharing

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
[owner-contract boundary](lifecycle-owner-contracts.md) now records and checks
explicit source, content and adapter-revision evidence. The separate
`plan_shared_upsert` entrypoint consumes that evidence with supplied
observations as described below. Shared retirement also remains available
through the separate removal API.

## Explicit shared upsert

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

## Private physical projection

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
scope/raw-root attribution; retained output extraction also adds real owner,
location and selector context. Literal destination and cause text are preserved.
Final complete validation precedes return. Errors expose no partial plan and
inputs remain unchanged. This projection supplies no execution,
source-authenticity, physical-containment or native authority.

## Remaining Gate 5 work

This is complete planning for the supported single-owner boundary. Graph and
artifact errors currently report the first error; full operation-level conflict
aggregation remains pending. Qualified source/destination integration,
source/scoped selectors, CLI precedence, instruction compiler adapters, guarded
first-write backup, executor/recovery integration and observational adapters
remain. Gate 6 must qualify native targets before schema-2 production
activation. This pure plan grants no execution, source authenticity, physical
containment or native support authority.

## Components

- [Coordinator](../../src/lifecycle/upsert.rs)
- [Acceptance examples](../../tests/lifecycle_upsert.rs)
- [Validation record](../technotes/2026-10-16-m1-gate5-upsert-validation.md)

- [Shared reconciliation implementation](../../src/lifecycle/sharing/mod.rs)
- [Shared acceptance examples](../../tests/lifecycle_sharing.rs)
- [Shared validation limits](../technotes/2026-10-17-m1-shared-reconciliation-validation.md)
