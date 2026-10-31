# AD-0041 — Separate client representations as the design baseline

Status: **proposed, awaiting explicit user adoption**. Date: 2026-10-31. Task
[#452](https://github.com/driftsys/upskill/issues/452). The
[decision brief](../technotes/customization-baseline-adoption-decision.md)
summarizes the proposed choice and its boundaries. This record has no production
effects.

## Context

The [A/B comparison](../technotes/customization-delivery-policy-comparison.md)
recommends separate generated client representations (A) over shared installed
defaults with client replacements (B). A needs fewer default-suppression and
fallback policies. Neither candidate has a complete qualified native route.
Q11/Q11b demonstrate bounded unintended activation; client-named directories do
not establish isolation. Their failures remain unchanged.

In Q11b's macOS project profile, a Claude project-marketplace registration made
Copilot catalog and invoke the Claude plugin. Its separate native-preservation
failure also blocks lifecycle claims for that route under A. Neither failure is
reclassified by adopting an organizing baseline.

The organizing choice can be explicit before every native route passes. This
requires keeping design adoption separate from permission to implement or
activate a route.
[AD-0040](0040-separate-customization-delivery-from-discovery.md) and
[Topic 0009](../topics/0009-customization-delivery.md) retain that boundary.

## Proposed decision

If adopted, use **separate generated representations per intended client as the
design baseline**, with explicitly declared, separately qualified shared
representations as exceptions. This selects A's organization; it neither selects
installed paths nor makes every item a plugin.

### Source and selection

The upstream or local install source remains the single authoring source.
Consumer outputs are copied, consumption-ready artifacts with captured source
identity, revision, hashes and ownership. Internal capture for reproducibility
is separate from an additional visible authoring tree. Preserve the existing
copy-only installation contract; this is not a new symlink-policy decision.

For each logical item and intended client, select exactly one representation and
one effective native exposure. Here, a default means the generated
representation of the item's authored default for that client, not an installed
shared catalog. An item/client need not have any default. A client-specific body
replaces its default; it is not loaded in addition to it. Inventory aliases,
shared roots, native files, plugins, compatibility readers, imports and
ancestor/nested discovery. Missing scope, permission or delegation mappings
reject the affected plan. This decision does not implement active source
overrides or a universal model resolver; their existing contracts still apply.

Supporting resources belong to the selected representation, retain relative
references and must be bound to actual native reads. Equal bytes or final
response text do not establish which representation was consumed.

### Native routes and sharing

Preserve native-first rule delivery and independence from manual team guidance.
A native file or configuration route is preferred where it satisfies the
contract; an internal rule-delivery hook/adapter can be selected within
AD-0040's existing scope where needed, subject to its evidence gates. This does
not authorize general hook authoring or new hook management. No universal plugin
packaging requirement follows from A.

Preserve the adopted one-copy unscoped Claude/Copilot rule policy: when both are
explicit intended readers, one `.claude/rules` representation serves both and a
second Copilot copy is rejected. This is an explicit sharing exception, not a
claim that `.claude` is private or all of it is shared.

Other sharing requires per-item intended readers, compatible content and
resources, one effective exposure per reader, permission/scope parity and
qualified lifecycle/negative controls. Equal content alone does not authorize
sharing. Divergent bodies require exclusion of the alternate body from each
client. If that exclusion is unqualified, both A and B reject effects for that
profile. No compatibility setting is disabled to manufacture support.

### Ownership and lifecycle

Reuse capture, owner/reference contracts, complete preflight, qualified locking,
backups and recovery. Direct and bundle references identify owners; incidental
readers do not become owners. A shared physical artifact retains every surviving
owner/reference, and final retirement requires observed native detachment and
the applicable live-resource controls.

Updates revalidate selection, intended readers, resource identity, local edits
and all affected native effects before changing anything. They cannot expose a
second body or widen permissions. Removal releases only the selected owner's
contribution and preserves surviving owners, manual files and configuration.
Native registration/cache outcomes need their own evidence and safe inverses;
file rollback alone is insufficient. Existing preservation classes and failure
outcomes are unchanged.

The proposed default on removing a selected client replacement is
**unavailability**, not implicit loading of a shared/default body. Switching to
a default is an explicit new selection, subject to the same preflight and
qualification. This is a proposed future lifecycle policy, not a change to
existing schema-1 remove behavior.

## Adoption and implementation boundaries

Adoption of this record would settle A versus B as the organizing baseline and
the selection/lifecycle policies above, including default unavailability after
replacement removal. Approval must explicitly cover that future removal policy;
it does not change schema-1 behavior. It would not settle:

- An installed `.agents/<client>` path contract, package granularity, per-client
  default packaging, source override syntax or a new lock schema.
- OpenCode package-skill selection, Codex distribution/trust, or any currently
  unqualified native delivery route.
- New owned-subagent MVP scope, removal of authorable bundles, expanded MCP,
  general hooks/tools/permissions or model-profile management.
- A preservation waiver, new public sharing profile, replacement Q11 gate,
  production schema-2 activation or a reduced four-client release scope.

Those are separate decisions or implementation designs. A's selection rules do
not pre-empt the pending owned-subagent scope, OpenCode package-skill source or
Codex distribution/trust decisions. Existing schema-1 production behavior and
supported kinds remain unchanged. Existing native observations retain their
exact version, scope, OS and surface limits. A configured Copilot overlay result
would qualify only its configured profile; it would not satisfy the retained
explicit foreign-selector Q11 gate.

Before owned implementation, require the existing step-7 user adoption,
preservation contracts and applicable Q10/Q11 and CD-01 through CD-11 evidence.
Skills/resources remain first, followed by rules and then automatic maintenance
under the adopted roadmap. Existing agent support stays a regression obligation;
new owned-agent delivery is not added to that sequence by this record.

## Consequences and reconsideration

A makes client-specific metadata/resources and selection responsibilities
explicit, at the cost of more generated copies and native lifecycle effects. It
does not demonstrate lower runtime risk. B remains a possible explicit sharing
profile; it is not the default replacement model.

Reconsider the baseline if a bounded native selector proves shared/default
replacement, exclusion, duplicate prevention and removal behavior with fewer
effects and preserved capabilities, or if independent client representations
cannot satisfy the required reader contract. A route failure blocks that route;
it does not silently narrow gates or authorize the alternative.
