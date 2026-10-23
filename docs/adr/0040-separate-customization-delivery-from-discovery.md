# AD-0040 — Separate customization delivery from discovery and activation

Status: accepted direction from the user's explicit architecture correction on
2026-10-19. Implementation and native qualification remain pending.

## Context

The roadmap calls for independent Codex/OpenCode rule delivery through qualified
plugins beside team `AGENTS.md`. The earlier M1 plan and Gate 6 draft instead
made managed `AGENTS.md` contributions an activation requirement. AD-0039 and
its implementation establish a bounded library experiment with reader reports;
they do not justify changing the production delivery architecture. The same
ambiguity affects shared skills, resources, package exports, agents and MCP
registrations through compatibility directories and imported configuration.

## Decision

Adopt the
[customization delivery contract](../topics/0009-customization-delivery.md) for
all customization/client cells. Independent Codex/OpenCode rules require a
qualified plugin route. Team `AGENTS.md` is preserved; rewriting it is not a
fallback. Native Claude/Copilot rules and native skills remain intended where
they satisfy the consumer/isolation contract. Missing runtime capabilities block
the route without weakening the four-client release scope.

Delivery, discovery, activation and ownership are separate contracts. Classify
known readers as intended, unintended or unresolved, including compatibility
directories, imports, globs, package exports and shared roots. Equal bytes and
reported incidental readers do not authorize sharing. Require successful
positive controls, unintended-reader negatives and duplicate-activation
controls. Reject unsafe profiles before effects rather than modifying manual
discovery settings or silently enabling another client's content.

Reuse source capture, per-owner contracts, complete preflight, qualified
locking, exact backup and recovery foundations. Native registration outcomes
require their own observed state and recovery evidence. Preserve all existing
content kinds and configuration through migration and existing commands. General
hooks, tools, permissions and expanded MCP management retain their later scope;
existing restrictions and integrations remain regression obligations.

## Reconciliation and consequences

This decision supersedes any production implication of AD-0039's managed
instruction path, earlier M1-B/M1-C instructions and the original Gate 6
canonical-file activation gates. Their algorithms, tests and historical evidence
remain valid within their explicitly bounded experimental/library scope.
Historical archive bytes are not rewritten. Active plans now point to the
corrected contract and retain an explicit contradiction inventory.

Production schema-2 activation is blocked until the corrected all-kind,
four-client, reader-isolation and lifecycle requirements have applicable native
and command evidence. Canonical instruction discovery is background evidence,
not qualification of independent plugin delivery. This ADR changes architecture
and acceptance criteria only; it neither introduces new product code nor claims
an implemented plugin, selector or migration.

## Shared-root evaluation direction (2026-10-20)

The user's subsequent evaluation request treats `.agents/skills` as
intentionally shared discovery space within the candidate
`.agents/{rules,skills,agents}` content root. The
[delivery contract](../topics/0009-customization-delivery.md#shared-content-root-evaluation)
now distinguishes that declared sharing from incidental readers and ownership.
Client-specific variants remain outside shared catalogs, and each logical item
must have one effective native exposure per client, including direct/plugin and
compatibility paths. Native scoping, permissions, manual content and existing
ownership remain binding.

The [evaluation](../technotes/shared-content-root-evaluation.md) is not a layout
adoption decision. CD-11 requires four-client native feasibility before adapter
selection and integrated coexistence before adopting the route. Gate 6 keeps
selected M1 rule qualification and existing-kind preservation; new owned-skill
lifecycle stays after Gate 6 and blocks that new layout's adoption and the
combined MVP release. No unadopted layout may acquire production effects.
Independent Codex/OpenCode plugin rules beside team instructions remain the
intended delivery architecture. No experimental code or earlier native
observation supplies this new evidence.

The comparison also includes intentional `.claude` sharing and `.agents` sharing
with selective Copilot/OpenCode compatibility exclusions. A documented skill
exclusion is not a decision that the whole `.claude` tree is private. Effective
configuration scope, preservation of unrelated manual content and native
positive/negative controls remain prerequisites for each candidate.

The next experiment order is: portable `.agents/skills` with one self-contained
Claude adapter first; intentional `.claude` sharing remains an alternative,
selective exclusions an optional provisioned profile. Rules and subagents retain
capability-aware native adapters, with source outside scanned catalogs unless
separately qualified. Native rule scope and delegated permission boundaries
cannot be replaced by prompt prose or broader plugin settings. This direction
selects experiments, not installed paths.

## Proposed amendments (2026-10-23)

The
[customization delivery proposal](../wip/specs/2026-10-23-customization-delivery-proposal.md)
proposes three changes to this direction. They are proposals under review, not
adopted changes; the accepted direction above stands until each is approved.

1. Default to one self-contained upskill-managed package per client for skills
   and subagents, with `.agents/skills` as an optional qualified profile rather
   than the first experiment. Reason: Claude direct `.agents/skills` discovery
   failed on 2.1.303, while the package route passed on Claude, Copilot and
   Codex; OpenCode has catalog evidence only, with invocation blocked at 2.1.4.
2. Include subagents in the MVP delivery scope. Reason: user direction on
   2026-10-23. Topic 0009's statement that new owned-agent management is not
   added to M1 stands until the roadmap is re-sequenced.
3. Deliver OpenCode skills as package directories registered as explicit skill
   sources, instead of the `.opencode/skills` native candidate.

The proposal keeps every other element of this decision: qualified plugin rules
for Codex and OpenCode beside unchanged team `AGENTS.md`, native Claude and
Copilot rules, reader classification, exactly-once activation controls and
preserved manual configuration. It adds a requirement that upskill never creates
`CLAUDE.md`, which retires the shipped `@AGENTS.md` bridge file.
