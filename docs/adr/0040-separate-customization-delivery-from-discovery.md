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
