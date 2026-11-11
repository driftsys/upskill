# Customization storage and delivery candidates

Date: 2026-10-23. Status: discussion draft; no candidate selected or approved
for implementation. Purpose: retain the side-conversation alternatives for
comparison once the applicable native qualification evidence is available.
Related work:
[native qualification #337](https://github.com/driftsys/upskill/issues/337) and
[customization MVP epic #283](https://github.com/driftsys/upskill/issues/283).

This document records alternatives. It changes no authoritative contract,
installation, native registration, active implementation plan or activation
gate. Existing experimental code is evidence to assess, not an architecture
decision. Any selected route must first be reconciled with the roadmap,
contracts and Gate 6 plans in the main development conversation.

## Evaluation acceptance criteria

Evaluate both candidates against the same logical rules, skills, supporting
resources and subagents across Claude Code, Copilot, Codex and OpenCode. Declare
project/user scope, platform, pinned client version, configuration and intended
readers. Retain existing Linux, macOS and IDE obligations. Generated-file tests
alone cannot establish native support.

A candidate is eligible for adoption only when the applicable evidence proves:

1. Intended delivery, discovery and activation mechanisms are explicit for every
   client and customization. Distinguish generated artifacts, plugin packages,
   configuration pointers, native registration and actual runtime consumption.
2. Each logical item has exactly one active representation per intended client.
   Inventory direct/plugin routes, aliases, compatibility directories,
   ancestor/nested roots, imports, globs and configured shared paths. Declare
   whether each cross-client consumer is intentional.
3. Successful calibrated positives precede negative controls for unintended
   readers, unavailable content, disabled/removed plugins and duplicate
   activation. A failed positive or a missing tool attempt is not an absence or
   permission-enforcement pass.
4. Native scope, tool, permission and delegation requirements survive
   translation. Reject unsupported mappings rather than substituting behavioural
   prose for enforcement or widening session-wide authority.
5. Native resource reads bind the actual intended file/package identity. Equal
   response text alone cannot prove which source was consumed.
6. Installation, update, removal and recovery preserve manual content, existing
   configuration and unrelated ownership. Reuse ownership, preflight and
   recovery foundations; declare native cache/configuration residue and
   incomplete preservation inventories.
7. Repeated requests establish stable content/order and measured cache/token
   behaviour for the declared configuration. Avoid assuming either glob or
   plugin delivery has superior caching without a controlled comparison.
8. Findings distinguish implemented behaviour, experimental observations,
   qualified profiles and planned work. Applicable missing evidence continues to
   block production schema-2 activation.

## Source, generation and installation are separate

The install source already contains the SSOT. It may be a remote repository, a
local authoring directory or deliberately vendored content. A consumer normally
needs installed outputs and ownership/provenance records, not another visible
`.agents/ssot` authoring copy.

Capture and internal source retention may still be necessary for reproducible
generation, update or recovery. Record source identity, captured revision,
content hashes and output ownership through the applicable contracts. This
proposal does not remove required captured evidence or change the lock schema.

All layouts below describe generated or installed consumer content. Directory
names do not establish native discovery, access isolation or support. Client
configuration and plugin caches may remain in client-managed native locations.
Preserve the existing copy-only installation contract unless a separate decision
explicitly revises it.

## Candidate A: separate generated outputs for each client

Use one upstream authored source and generate a separate client representation
for every installed item. A possible consumer namespace is:

```text
.agents/
├── claude/
├── copilot/
├── codex/
└── opencode/
```

Each client subtree contains its generated packages/artifacts and supporting
resources. Plugins, explicit configuration pointers or native files expose the
appropriate outputs through qualified client mechanisms. These namespaces are
not a claim that clients natively discover arbitrary `.agents/<client>` trees.

For skills, evaluate self-contained client plugin packages containing the
rendered `SKILL.md` and its resources. Plugin installation must actually
register the skill; package acceptance alone is insufficient. Do not also expose
a shared `.agents/skills` copy to the same client. Plugin-based skill
registration and resource access remain client-specific qualification questions,
particularly for OpenCode.

For rules and subagents, choose qualified native delivery or plugin packaging
according to the capabilities required by the item. Native files may still be
necessary outside the namespace. Do not require universal plugin packaging at
the cost of native scope or restrictions.

Expected benefits to evaluate:

- Uniform generation, provenance, ownership and update/removal handling.
- Clear placement for client-specific metadata, bodies and resources.
- Fewer dependencies on shared-default selection and exclusion mechanisms.

Expected costs to evaluate:

- More generated copies and native registration/configuration artifacts.
- Repeated package/resource storage and more generated output to inspect.
- Continued compatibility-reader risks: for example, generating Claude artifacts
  does not by itself prevent another client from discovering them.

## Candidate B: shared installed defaults and client-specific variants

Generate consumption-ready portable defaults into shared locations. Generate
client-specific replacements outside those shared catalogs when required:

```text
.agents/
├── skills/foo/SKILL.md
├── rules/bar/RULE.md
├── claude/
│   ├── skills/foo/SKILL.md
│   └── rules/bar/RULE.md
├── copilot/
├── codex/
└── opencode/
```

A qualified plugin selects and loads the applicable representation, or upskill
maintains explicit native configuration pointers where supported. For each
logical item, declare which clients consume the shared default and which consume
a client-specific replacement. The shared installed default is not a second
consumer SSOT.

Selection must be enforced, not merely described. A plugin pointer to a variant
does not necessarily suppress a shared skill that the same client automatically
discovers. Require qualified native per-item exclusion or precedence, including
negative and duplicate-activation controls. If that cannot be established, keep
the affected item out of shared discovery and use separate client delivery.

Define variant removal behaviour before execution: restoration of a shared
default versus unavailability must be an intentional, tested policy. Do not
silently fall back to a weaker variant or different permissions.

Whole-body overrides replace content; they do not compose with a shared core. A
shared procedure referenced by a wrapper requires an actual runtime resource
read. Embedded composition/includes would require an explicit separate feature.
Keep source-only directives and variants outside consumption-ready catalogs.

Expected benefits to evaluate:

- Fewer installed copies for identical portable content and resources.
- Direct shared discovery where it is intentional and qualified.
- Explicit client-specific exceptions without duplicating every item.

Expected costs to evaluate:

- More complex selection, exclusion, ownership and shared-owner retirement.
- Greater risk of direct-plus-plugin or compatibility duplicate activation.
- A shared update can affect several clients, requiring complete reader and
  failure/recovery analysis.

## Delivery choices to compare within both candidates

These are mechanisms to evaluate, not new native-support claims:

| Kind/client         | Candidate mechanism                                                                                          | Qualification boundary                                                                                                                               |
| ------------------- | ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rules: Codex        | Trusted plugin reads selected explicit installed rule paths and injects context beside team `AGENTS.md`      | Prove actual reads, activation count, trust, scope and lifecycle; startup injection alone does not establish native path conditions.                 |
| Rules: OpenCode     | Explicit instruction-file paths without globs, compared with independent plugin loading of the same content  | Confirm the exact frozen client's loading and cache behaviour; reconcile any architecture change before selecting the native reader.                 |
| Rules: Claude       | Generated native `.claude/rules` output                                                                      | Preserve native conditions and nested scope; inventory other clients that may read this output.                                                      |
| Rules: Copilot      | Generated native `.github/instructions` output                                                               | Preserve native applicability and inventory canonical, imported and compatibility readers.                                                           |
| Skills: all four    | Separate client plugin delivery, or intentional shared discovery with a qualified adapter/variant route      | Each client needs successful actual invocation/resource controls and exactly one active representation.                                              |
| Subagents: all four | Shared authored role/requirements with generated client-native definitions or qualified plugin registrations | Preserve actual delegation, model/tool restrictions and native permission boundaries; native discovery may require files outside `.agents/<client>`. |

### OpenCode explicit paths and cache behaviour

The user reports that `.agents/rules/**/RULE.md` glob loading increases token
usage and disrupts cache reuse, while explicit file paths without globs work in
their experience. Preserve this observation as user-reported evidence. It does
not yet establish the underlying cause or qualify every OpenCode version.

Compare an ordered explicit path list and plugin delivery with the same rule
bytes and task sequence. Record versions, prompt/context identity, ordering,
repeat and resumed requests, cached/uncached token accounting, rule activation
and update/removal behaviour. Retain failed controls. Determine whether glob
expansion changes order/content or other context; do not assume that explanation
without evidence.

The earlier discussion identified a historical OpenCode configuration reader for
`.agents/rules/**/RULE.md`. Inventory the actual installed configuration before
treating that directory as inert or adding plugin injection. Preserve manual
configuration; preflight must expose conflicting readers and require a safe,
owned transition.

### Contract contradiction requiring an explicit decision

The recorded Gate 6 architecture requires independent Codex/OpenCode plugin rule
delivery alongside team `AGENTS.md`. Selecting an OpenCode explicit-path native
reader would revise that requirement. This WIP retains the alternative without
silently approving that revision. Native instructions, rule injection and team
files remain separate mechanisms in the evaluation.

## Subagents, models, tools and permissions

A shared authored subagent can express role, procedure, evaluation criteria,
return contract and required capabilities. Client adapters must translate native
model identifiers, supported effort, tools, delegation and permission settings.
A common Markdown body cannot establish equivalent enforcement.

Evaluate shared model profiles as a separate proposed resolver: one intent maps
to client-specific identifiers and supported settings. Provider configuration,
authentication and native availability remain client concerns. Distinguish
selection during generation from runtime fallback and define failure behaviour;
do not promise universal fallback or silently downgrade a required model.

Retain the known distinction between inherited sandbox/approval authority and
instructions requesting read-only behaviour. A client unable to enforce a
required child boundary must reject that mapping. Verify actual child execution
and denied operations, not just catalog registration or generated frontmatter.

This evaluation does not add general subagent, model-profile, hook, tool,
permission or MCP management to the current implementation scope. Keep those
planned capabilities visible. Bundles, external plugins and MCP servers retain
separate dependency, registration, ownership and compatibility obligations; the
layout proposal does not absorb or qualify them automatically.

## Comparison and decision record

| Question             | Candidate A                                    | Candidate B                                                 |
| -------------------- | ---------------------------------------------- | ----------------------------------------------------------- |
| Consumer content     | Separate generated client representations      | Shared generated defaults plus client-specific replacements |
| Source of truth      | Upstream/local install source                  | Upstream/local install source                               |
| Selection complexity | Per-client route selection                     | Per-client route plus shared/variant selection              |
| Duplication          | Higher generated storage                       | Potentially lower, dependent on qualified readers           |
| Isolation proof      | Native/compatibility readers and registrations | The same proof plus shared-default exclusion/precedence     |
| Ownership/recovery   | Client artifacts and native configuration      | Client artifacts, shared owners and selection configuration |
| Present disposition  | Unapproved candidate baseline                  | Unapproved alternative                                      |

Do not assign numerical weights or declare a winner before evidence can support
the comparison. When sufficient data is available, record the observed
tradeoffs, remaining gaps, selected route per kind/client and reasons for
rejecting the alternatives. Update authoritative contracts and acceptance
criteria first; continue implementation and native qualification against that
corrected plan. Production activation remains blocked until the applicable gates
have evidence.

## Related repository contracts

- [Content format](../../topics/0001-content-format.md).
- [Generation](../../topics/0002-generation.md).
- [State and ownership](../../topics/0004-state-and-ownership.md).
- [Lifecycle planning](../../topics/0005-lifecycle-planning.md).
- [Execution and recovery](../../topics/0008-execution-and-recovery.md).
- [Roadmap](../../roadmap.md).
