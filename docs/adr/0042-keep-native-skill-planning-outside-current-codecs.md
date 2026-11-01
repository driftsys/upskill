# AD-0042 — Keep native skill planning outside current codecs

Status: reviewed inactive library design. Date: 2026-11-01.

## Context

[AD-0041](0041-separate-client-representations-as-design-baseline.md) and the
approved skills/resources plan require portable selection intent, immutable
revision evidence and attributable native receipts. The current combined-state
codec has one active installation owner per ID and no native receipts or
retained session resources. Its graph cannot represent detached revisions as
active installations: unreachable installations reject. Production commands
remain schema 1. The retained #332 work rejects unsupported state before source
resolution; it does not supply native state persistence.

An additional authoritative installation lock or silent schema-2 extension would
hide these incompatibilities. Native success cannot be inferred from generated
files, a desired graph or file rollback.

## Decision

Implement a pure planning boundary, with no serializer, migration, state writer
or CLI route. Keep both existing state codecs unchanged. A future persisted
format must be reviewed and admitted separately before production application;
this record chooses no schema number or public migration.

Use a single caller-supplied `SkillDeliveryState` view containing a selected
scope, the existing `CombinedState` graph, immutable `PackageRevision` captures
and latest `NativeBindingReceipt` records. This is an in-memory planning view,
not a second installation lock. `CombinedState` remains the only logical
installation/reference graph. It carries no skill-package artifacts in this
view; revision captures supply package-file evidence, so the same skill file
cannot acquire two independent ownership representations. Unrelated existing
artifacts and legacy state remain unchanged.

Each revision carries its captured `Installation`, `SelectedSkill` and
`SkillPackage`. Validate captured source evidence, recompile the package and
require equality. Historical capture ownership is distinct from the current
logical owner. Retained files continue to carry the current logical owner when
one exists. After logical detachment, the projected file owner has no current
`source_revision`; exact historical revisions remain in the immutable captures.
This avoids conflicting current owner bindings across several retained Git
revisions without inventing an active graph node. Installation IDs cannot be
rebound to a different source, kind, source name or alias while historical
captures remain; apply the existing immutable binding check against those
captures even if an ID reappears after leaving the active graph. Such detached
captures require a confirmed detachment receipt for that same
installation/client. They establish no native activation and are not
automatically garbage-collected.

`SkillDeliveryIntent` contains installation ID, target and either an explicit
body origin or `Unavailable`; it contains no scope path or native binding.
Desired intent is complete for every surviving skill/target. It does not add
references or revive unreachable installations. Use existing reference
reconciliation for direct and bundle reachability. Removing a replacement means
explicit unavailability; no default candidate is selected automatically.
Duplicate intents/candidates and conflicting revisions or aliases reject.

`plan_skill_delivery(&SkillDeliveryInput)` consumes the current view, graph
changes, complete desired intent, selected package candidates, exact file
observations and attributable native observations. It returns a
`SkillDeliveryPlan` with a reconciled graph, validated desired intent, retained
and new revision evidence, proposed file ownership, separate immutable staging
and selector-file changes, and native transition requests. It returns no
completed state and creates no successful receipts. File planning reuses the
whole-file planner, including rejection of local edits and identical unowned
files. Retain all existing revisions; this slice authorizes no resource
deletion.

Native receipt and observation records include installation/client/scope,
version/platform, operation or observation identity, before/after selection and
captured native prestate. Outcomes distinguish confirmed, rejected and unknown.
Every non-confirmed outcome, including explicit rejection, and every unknown
observation requires recovery before a new plan. Rejection does not authorize an
automatic retry; recovery must separately establish its safe disposition.
Confirmed receipt poststate must match the native observation. Unowned native
bindings reject instead of being adopted. These are captured claims; validation
does not establish their authenticity or qualify a native profile.

Any refresh/detach, including reinstallation over retained revisions, requires
explicit session clearance or native-cache resource preservation evidence.
Export retention alone does not satisfy this guard. A fresh installation
requires an attributed absent baseline. A no-op revalidates both files and
native state.

A current view and every captured revision/receipt must bind to the selected
scope. Copying absolute-root state into another checkout rejects. Explicit clone
materialization starts with a fresh local view, portable intent, source evidence
and newly compiled packages for that checkout. It copies no native receipt or
ownership authority from the previous machine.

## Boundaries and later application

The resulting graph and file ownership cannot simply be serialized with the
schema-2 codec: detached revisions and native receipts require a separately
admitted integrated representation. The later coordinator must keep staging,
selector changes, native effects, observations and final ownership commit in a
recoverable protocol. It must not claim transaction completion by serializing a
preview. Native receipts remain in memory until that contract supplies a durable
operation format. This is compatible with retaining #332 as a production
admission dependency, not a declaration that #332 is complete.

The existing rules-only desired producer and retirement API remain unchanged.
This planner rejects graph changes that would alter non-skill installations. No
new dependency, hook, native registration, schema-2 command, paid experiment or
qualification waiver follows from this decision.
