# AD-0040 — Separate customization delivery from discovery and activation

Status: accepted separation of delivery and discovery from 2026-10-19, with the
user's native-first always-on rules and skills-before-rules implementation order
decided on 2026-10-24. This step-7 reconciliation records that direction for
user adoption review; implementation and complete native qualification remain
pending. Other package and release-scope amendments below remain proposals
([decision](../wip/specs/2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24),
[evidence and adoption](../topics/0009-customization-delivery.md#slice-1-evidence-and-adoption-2026-10-25)).

## Context

The 2026-10-19 roadmap called for independent Codex/OpenCode rule delivery
through qualified plugins beside team `AGENTS.md`. The later native-first
amendment below replaces that mechanism requirement with independence from team
guidance. The earlier M1 plan and Gate 6 draft instead made managed `AGENTS.md`
contributions an activation requirement. AD-0039 and its implementation
establish a bounded library experiment with reader reports; they do not justify
changing the production delivery architecture. The same ambiguity affects shared
skills, resources, package exports, agents and MCP registrations through
compatibility directories and imported configuration.

## Decision

Adopt the
[customization delivery contract](../topics/0009-customization-delivery.md) for
all customization/client cells. Independent rules must use a route independent
of team `AGENTS.md`; a plugin is required only where no native route exists.
Team guidance is preserved; rewriting it is not a fallback. Native
Claude/Copilot rules and native skill consumption remain intended where the
selected route satisfies the consumer/isolation contract. Explicit OpenCode
instruction paths are a selected candidate, pending Q5 PASS. Missing runtime
capabilities or failed positives block qualification without weakening the
four-client release scope
([native-first decision](../wip/specs/2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24),
[Q5 retry](../technotes/gate6-opencode-explicit-paths-retry.md)).

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
not qualification of independent rule delivery. This ADR changes architecture
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

The [evaluation](../technotes/shared-content-root-evaluation.md) was not a
layout adoption decision. Its original experiment order and rules-first
dependency are superseded only by the explicit sequencing amendment below.
Integrated coexistence, existing-kind preservation and the four-client combined
release gate remain required. No unadopted layout may acquire production
effects. Independent rule delivery preserves team instructions; experimental
code and earlier native observations do not supply the missing qualification
([CD-11](../topics/0009-customization-delivery.md#acceptance-and-evidence-gates)).

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

## Native-first rules and implementation order (2026-10-24)

Always-on rules are the MVP priority. Use native loading where a native route
exists; use a trusted package hook where it does not, or a separately qualified
fallback. Consistency is one authored source and per-client generation, not one
runtime mechanism. Native files should activate on clone and appear in the
client's own file inventory; Codex's hook exception requires ordinary
per-machine installation and trust. Q1, Q3, Q6 and Q9 support this direction's
evidence, onboarding and debuggability reasons without qualifying complete
adapters
([user decision](../wip/specs/2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24),
[retained dispositions](../topics/0009-customization-delivery.md#slice-1-evidence-and-adoption-2026-10-25)).

In a Claude+Copilot project, generate one unscoped `.claude/rules` file per
logical rule, declare both intended readers per item, and reject a second
`.github/instructions` copy. Other IDE, cloud, review and compatibility readers
remain unresolved until separately declared and qualified. A Claude-only rule
visible to Copilot requires compatible intended-reader consent or is unsupported
for that project. A warning does not supply consent. Scope is never silently
widened; scoped shared Claude+Copilot rules are unsupported under this policy
([Q1](../technotes/gate6-copilot-claude-rules-reader.md),
[reader policy](../topics/0009-customization-delivery.md#reader-and-activation-policy)).

The selected Codex policy is **supported with declared limitation**, not a claim
of implemented or fully qualified support. Q9's original zero-duplicate result
remains FAIL. The resume-only follow-up on 0.164.0 records one body per rule at
startup, two in cumulative history after resume and one newly appended body,
without compaction. Provider-effective input and duplicate lifetime remain
unqualified. On user adoption, #379 becomes K1 debt rather than a
route-selection blocker; the current issue labels are not changed by this
document ([combined control](../technotes/gate6-codex-rule-lifecycle.md),
[resume-only control](../technotes/gate6-codex-resume-attribution.md)).

Implement skills/resources on passed client routes before always-on rule work,
then shared automatic maintenance. This explicitly replaces the original
rules-before-new-owned-skills implementation dependency. Claude starts from Q6;
Codex has bounded Q8 PASS; OpenCode waits for Q5 skill/resource PASS. Q10/Q11,
user adoption of package defaults and per-client preservation contracts remain
entry gates. All-client release qualification and schema-2 activation remain
blocked until their own evidence passes
([handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md#slice-order-and-required-amendment),
[roadmap](../roadmap.md#implementation-dependencies)).

OpenCode Q5 is deferred after failed positives at 2.1.4. Explicit paths remain
unqualified, not disproven. Step 8 bridge/glob removal waits for Q5
explicit-path PASS and its own code PR; existing entries are not deleted. The
required future `doctor` reports are not current command behavior. Hooks read an
install-time static rule index and never invoke the upskill binary at runtime
([Q5 retry](../technotes/gate6-opencode-explicit-paths-retry.md),
[step 8](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md#slice-1-qualification-of-the-gating-routes)).

## Proposed amendments (2026-10-23)

The
[customization delivery proposal](../wip/specs/2026-10-23-customization-delivery-proposal.md)
proposes the following additional changes. They are proposals under review, not
adopted changes; the accepted direction above stands until each is approved.

1. Decision required: default to one self-contained upskill-managed package per
   client for skills and subagents, with `.agents/skills` as an optional
   qualified profile rather than the first experiment. Reason: Claude direct
   `.agents/skills` discovery failed on 2.1.303, while the package route passed
   on Claude, Copilot and Codex; OpenCode has catalog evidence only, with
   invocation blocked at 2.1.4.
2. Decision required: include subagents in the MVP delivery scope. Reason: user
   direction on 2026-10-23. Topic 0009's statement that new owned-agent
   management is not added to M1 stands until the roadmap is re-sequenced.
3. Decision required: deliver OpenCode skills as package directories registered
   as explicit skill sources, instead of the `.opencode/skills` native
   candidate.

4. Decision required: move bundles as an authorable type out of the MVP while
   retaining existing bundle composition and native plugin delivery.

Decision required: the user adopts or rejects each of these four amendments at
the step-7 PR review. No Q result adopts them. The first three numbered items
retain their original identities; authorable bundles are listed separately
([proposal amendment inventory](../wip/specs/2026-10-23-customization-delivery-proposal.md#records-superseded-or-amended-if-this-proposal-is-adopted)).

The proposal keeps independent-of-team-guidance rules, reader classification,
one planner delivery per item/client and preserved manual configuration. Only
the explicitly bounded Codex recorded-history allowance qualifies the former
unconditional duplicate requirement. The proposed retirement of shipped bridge
creation is future code work, gated with glob removal on Q5 PASS; this ADR does
not claim that current `add` or `doctor` behavior changed
([delivery contract](../topics/0009-customization-delivery.md),
[shipped ancillary handling](../specification/specification.md#34-ancillary-file-handling)).
