# M1 explicit shared-output reconciliation

User delegated implementation decisions on2026-10-17 under the accepted Gate5
scope. Implement one inactive pure opt-in path. Production commands/schema1,
native qualification, qualified execution/backup and existing unqualified
planner sharing rejections remain unchanged.

## Contract production

Add `compile_rule_desired_set_with_contracts(request, profiles)` returning
`ContractedDesiredSet { set, contracts }`. Reuse captured compilation, bind
source manifest, normalized content and explicit target revisions to every
produced artifact. Profiles must exactly cover requested surfaces, match their
target families, be unique and nonzero. Never obtain revisions from surface
defaults. Audience exclusions produce no artifact contracts. Existing producer
and batch rejection remain unchanged. Instruction compilation remains separate
work.

## Explicit shared upsert

Add `plan_shared_upsert(prior, request, observations)` alongside `plan_upsert`,
using the same selection, legacy checks, graph retention, complete desired-set
validation and contract propagation. No alternate reference authority or writer.

Reconcile candidate artifacts by exact scope/raw-root/path/contribution
selector. Preserve surviving artifact/owner/association/contract relative order;
new identities append in key order. Changed owners require complete sets;
unchanged owners retain content/consumers/unknown proof exactly. Conflicting
desired bytes/modes/bodies at one identity reject; retained owners cannot
acquire changed content implicitly. Real existing owner bindings use the
original immutable-binding validation.

Every affected artifact with multiple final owners requires complete valid
recorded contracts and exact source/profile agreement. A missing or conflicting
owner blocks the whole operation. Complete recompiled evidence can replace
unknown selected history; unknown unselected evidence never becomes revision1.
Release/removal retains existing reference-removal safety: dropping owners alone
does not require inventing their missing contracts. No selected work yields a
partial ready plan.

## Physical planning

Use a private physical projection: one deterministic internal planning owner per
logical artifact identity. Validate real owner contracts before projection,
reuse `operations::plan_owned_artifacts` for observations, no-op verification,
topology, manual aggregate preservation, permissions and marker handling, then
map only the reconciled content metadata back onto authoritative real owners.
Internal owner identities never escape to state, contracts, references or public
outcomes.

All affected physical locations require exact supplied observations, even
no-ops. Unknown dedicated files and unmanaged managed markers cannot be adopted.
Untouched shared instruction neighbors may keep unknown evidence while their
body stays unchanged; aggregate layout metadata may refresh. Final complete
state validation runs after physical reconciliation. Inputs remain unchanged on
success/failure. This supplied-value plan grants no filesystem containment,
source authenticity, native behavior or execution authority; guarded application
remains later work.

## Acceptance examples

- Explicit compiler profiles bind exact source/content/consumers, including
  empty audience, bad family, zero/duplicate/missing/extra profiles, aliases and
  resources.
- New sharing, adding to retained ownership, coordinated shared updates and
  owner release work only with the required complete evidence;
  contract/source/profile, content or mode disagreements reject with no partial
  plan.
- Unknown unselected owners block shared mutation; selected complete
  requalification is explicit. Single-owner opt-in plans match existing
  behavior.
- Real binding edits, unselected reference/owner edits, invalid unrelated state,
  duplicate desired/proof/observations, raw-root aliases, missing/edited
  observations, topology and legacy conflicts reject.
- Instructions preserve exact manual bytes, unknown unchanged shared neighbors
  and observed modes; changed bodies require fresh owner contracts. Final
  removal keeps empty aggregates. New unmanaged marker collisions reject.
- Full codec roundtrip retains each distinct real owner/profile/source;
  projection tokens never escape. Existing unqualified APIs still reject
  sharing.
