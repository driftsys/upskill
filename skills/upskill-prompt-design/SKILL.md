---
schema: 1
name: upskill-prompt-design
description: Use as the entry point to the upskill framework's prompt-engineering discipline. Trigger when someone is new to the framework, when an author is not sure which meta-skill to activate, when reviewing how a team is using rules/skills/subagents, or when cross-cutting concerns (portability across clients, classification, token economics, composition patterns) come up. Do NOT trigger for general one-shot prompt-writing guidance — that is an onboarding concern outside the framework. Do NOT trigger for specific authoring tasks — hand off to upskill-prompt-distilling, upskill-writing-rules, writing-skills, upskill-writing-subagents, or upskill-using.
metadata:
  version: 0.1.0
  author: driftsys
---

The umbrella for how driftsys engineers durable agent content — rules,
skills, and subagents that shape Claude Code, Copilot, and opencode
across our repos. This skill routes; the writing-\* skills do the work.

**Out of scope:** one-shot prompt-writing guidance. That is general LLM
literacy and belongs in engineering onboarding. The framework stops at
the boundary between durable agent content (what it manages) and
per-turn user prompts (what users send).

## The mental model

Five layers carry durable agent behavior. Three are managed by upskill:

| Layer          | What it is                                                                | Authored via                               |
| -------------- | ------------------------------------------------------------------------- | ------------------------------------------ |
| **Rule**       | A line in CLAUDE.md / AGENTS.md, loaded every turn                        | `upskill-writing-rules`                    |
| **Skill**      | A markdown procedure activated on demand by description matching          | `superpowers:writing-skills`               |
| **Subagent**   | A bounded sub-task with its own system prompt, tools, and return contract | `upskill-writing-subagents`                |
| **MCP tool**   | Live capability exposed by an on-prem server                              | Separate MCP server repo authoring process |
| **RAG corpus** | Searchable static content fronted by an MCP search tool                   | The corpus's MCP tool authoring process    |

Most authoring questions involve the first three. MCP and RAG sit
adjacent — referenced from the framework but governed by their own
processes.

## Where to start — the routing table

| Situation                                                                  | Activate                                                 |
| -------------------------------------------------------------------------- | -------------------------------------------------------- |
| "I want to add a convention / encode behavior / shape what the agent does" | `upskill-prompt-distilling`                              |
| "I know it's a rule, how do I write it well"                               | `upskill-writing-rules`                                  |
| "I know it's a skill, how do I write it well"                              | `superpowers:writing-skills`                             |
| "I know it's a subagent, how do I write it well"                           | `upskill-writing-subagents`                              |
| "I need to run upskill commands / vendor a bundle / audit content"         | `upskill-using`                                          |
| "I need to set up the RED-GREEN-REFACTOR cycle for something I authored"   | `upskill-evaluating-prompts`                             |
| "I want to write an MCP server"                                            | `mcp-builder` (vendored) — out of upskill's direct scope |
| "I want to write good one-shot prompts to send to Claude Code"             | Not this framework — see onboarding doc                  |

The umbrella does not replace these — it routes to them.

## Cross-cutting concerns

Knowledge that applies across all three managed layers lives here
because it has no other home.

### Cross-client portability

A single canonical body in a source registry's `skills/` directory is
emitted to per-client files by `upskill add` / `upskill update`. Two
divergence mechanisms exist: **override files** (`SKILL.claude.md`,
`SKILL.copilot.md`, `SKILL.opencode.md` — when the procedure itself
diverges) and **inline directives** (`<!-- @client:claude -->` ...
`<!-- @endclient -->` — when 90%+ of the body is shared and only a
tactical swap is needed).

**Principle:** write to the most capable client (usually Claude Code),
then down-shift via inline directives for less capable ones. Writing
to the lowest common denominator and up-shifting produces blander
skills.

Client capability matrix (v0.1.0). `upskill doctor` warns about
portability misuse (overrides where directives would do, or
directives where overrides are required):

| Capability                         | Claude                      | Copilot        | opencode            |
| ---------------------------------- | --------------------------- | -------------- | ------------------- |
| Native subagents                   | ✓                           | ✗              | ✓ (different shape) |
| Skills with progressive disclosure | ✓                           | ✓              | ✓                   |
| AGENTS.md native                   | ✓ (via `@AGENTS.md` stub)   | ✓              | ✓                   |
| Path-scoped instructions           | ✓ (`paths:` in frontmatter) | ✓ (`applyTo:`) | partial             |
| MCP servers                        | ✓                           | ✓              | ✓                   |

### Classification

Default to the lowest classification that fits. Never include
classified specifics (customer names, IP-sensitive details) in skills
shared across teams — those belong in team- or repo-scoped skills.
Tag sensitive content explicitly: `metadata.classification: <level>`.
upskill itself does not bake in a classification scheme; allow-list
gating is org-specific.

### Token economics

Rules are paid every turn. Skills cost on activation. Subagents cost
invocation overhead plus return payload. MCP tool definitions are paid
every turn (the schemas, not the responses). Description quality is the
dominant cost variable for skills, not body length — a vague
description silently fails to activate, which is worse than no skill
at all. `upskill-writing-rules` carries the full token-budget discipline.

### Composition patterns

Most authoring intents span multiple layers (rule + skill, skill +
MCP tool, subagent + skill, and so on). `upskill-prompt-distilling`
enumerates the recurring patterns and handles the decomposition. If
your intent fits one, route to every relevant writing-\* skill, not
just one.

## The methodology in one paragraph

Every durable prompt — rule, skill, subagent — is authored against a
scenario battery, evaluated under pressure, and shipped only when it
holds up. The cycle is RED (confirm the failure exists without the
prompt), GREEN (confirm the prompt fixes it), REFACTOR (confirm the
prompt survives realistic pressure). Prompts that have not been
pressure-tested look identical on the page to ones that have, and
fail silently in production. The evaluation discipline is what
distinguishes durable agent content from wishful thinking;
`upskill-evaluating-prompts` covers the methodology in detail.

## The lifecycle in one paragraph

Content lives in a source registry's `skills/` directory as SSOT.
`upskill add` and `upskill update` fetch the SSOT and generate
per-client files in the consumer project at install time.
`upskill lint` catches schema and structural errors in source
registries; `upskill doctor` verifies installed state against
`.upskill-lock.json`. Generated per-client files are never hand-edited
— drift is silent. Vendored bundles (like `superpowers`) carry NOTICE
files and attribution. `upskill-using` covers the workflows.

## Symptoms that suggest framework-level problems

If you notice any of these, this skill is the right entry point for
diagnosis:

- "The agent keeps doing X even though we have a rule against it" →
  rule under-firing or being rationalized. Activate
  `upskill-evaluating-prompts` for diagnosis, then `upskill-writing-rules` for
  revision.
- "We have a skill for that but nobody uses it" → description not
  activating. Activate `superpowers:writing-skills` or hand off via
  `upskill-prompt-distilling` (the skill may be in the wrong layer).
- "The subagent's output is bigger than what the parent would have
  done inline" → return-contract failure. Activate `upskill-writing-subagents`.
- "We added a rule and now every interaction feels slower" → token
  budget exceeded. Activate `upskill-using` (`doctor`) and
  `upskill-prompt-distilling` (the content may not deserve to be a rule).
- "Different teams handle the same situation differently" →
  cross-cutting convention without a home. Route via
  `upskill-prompt-distilling`.

## Composition with onboarding

This framework covers the _durable agent content_ layer. The
_human-side discipline_ — how engineers phrase requests, structure
context, give feedback — lives in the engineering onboarding
documentation. A well-prompted engineer using a well-configured agent
produces good work; either side alone underperforms.

## Honest caveats

v0.1.0, deliberately scoped to the framework-side of prompt
engineering (durable agent content authoring); the user-side
(one-shot prompting effectiveness) is explicitly excluded. The
cross-cutting sections are short by design and each may grow into its
own skill later. This skill has not yet been through its own
RED-GREEN-REFACTOR cycle; the bet is that an explicit entry point
reduces the activation cost of the discipline enough to be worth its
slot.

## You Are Done When

- The author's intent is classified (new content vs. debugging vs. cross-cutting)
- You have routed to the correct downstream skill
