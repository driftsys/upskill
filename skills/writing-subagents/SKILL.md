---
schema: 1
name: writing-subagents
description: Use when designing a new subagent or modifying an existing one's system prompt, tool surface, or return contract. Trigger when the parent is observed doing work inline that should be delegated, OR when a subagent is observed dumping raw output, OR when subagent scope is drifting. Also trigger when designing managed/project-scoped/user-global subagent tiers with different permission models. Do NOT trigger for skill or rule authoring; see writing-skills and writing-rules.
metadata:
  version: 0.1.0
  author: driftsys
---

A subagent earns its overhead only if it (a) does bounded work and
(b) returns a digest small enough that the parent's context is cleaner
than if the parent had done the work inline. If either fails, you have
a tool call dressed up as a subagent. Confirm the routing decision via
`prompt-distilling` before continuing here.

## The two interfaces

Every subagent has two failure surfaces. They fail independently. Both
must be tested.

1. **Invocation interface** — the name, description, and tool
   advertisement that the parent sees. This determines whether the
   parent decides to call the subagent.
2. **Return interface** — what the subagent emits when it finishes.
   This determines whether the parent's context stays clean after
   invocation.

Most subagent failures observed in practice are return-contract
failures: the subagent itself worked, but it returned 3KB of tool noise
instead of a digested summary, defeating the entire point.

## The Iron Law

NO SUBAGENT WITHOUT A FAILING TEST ON BOTH INTERFACES.

You must observe (a) the parent failing to delegate when it should, OR
the parent delegating when it should not, AND (b) the return contract
polluting parent context or hiding failure. If neither failure is
observed without the subagent, the work probably belongs in a skill.

## Failure modes this skill prevents

| Failure mode                                                                             | Interface                         |
| ---------------------------------------------------------------------------------------- | --------------------------------- |
| **Under-firing** — parent does the work inline, pollutes own context                     | Invocation                        |
| **Over-firing** — parent delegates trivial work, pays overhead                           | Invocation                        |
| **Scope creep** — subagent does more than its remit                                      | Subagent body                     |
| **Return-contract failure** — subagent dumps raw output instead of digesting             | Return                            |
| **Silent failure** — subagent reports success without verifying; parent acts on bad info | Return                            |
| **Tool surface mismatch** — too many tools → wanders; too few → cannot complete          | Subagent body                     |
| **Authority leak** — subagent has permissions parent did not intend to delegate          | Subagent body (managed tier only) |

## RED phase — scenarios per interface

<!-- @client:claude -->

Run each scenario by spawning a fresh subagent with only the relevant
prompt loaded. The parent's invocation tests run in the parent session
with the subagent registered. The return tests run on the subagent
directly in isolation.

<!-- @endclient -->

### Invocation tests (run on the parent, subagent present)

- **Should-delegate task** — does parent invoke?
- **Should-not-delegate task** (too small) — does parent stay inline?
- **Adjacent-scope task** — does parent invoke incorrectly?

### Return tests (run on the subagent directly)

- **In-scope task** — does it return a digest or raw output?
- **Partial-out-of-scope task** — does it refuse cleanly, or silently
  expand scope?
- **Will-fail task** — does it report the failure honestly, or
  rationalize success?

### Authority test (managed tier only)

- **Injection scenario** — parent or user attempts to get the subagent
  to exceed its delegated authority. If it complies, the authority
  model is broken regardless of how good the rest of the design is.

Document the rationalizations in two columns: parent's excuses for not
delegating (or for delegating wrong), and subagent's excuses for
bloated returns or scope creep.

## GREEN phase — writing the subagent

### System prompt

- **Scope in one sentence.** If it takes two, split the agent.
- **Refusal conditions stated explicitly.** "If asked to do X, return
  REFUSED with reason." Implicit refusal does not survive pressure.
- **Return format stated explicitly.** Do not leave digest length to
  taste.

### Description (the invocation surface)

- Same rules as skills: trigger conditions, not workflow summary.
- Include **negative triggers** when over-firing is the dominant
  failure: "Do NOT invoke for X."
- Treat the description as the activation contract. Vague descriptions
  cause silent under-firing.

### Tool surface

- Default to fewer tools. Add a tool only when a RED scenario fails
  for lack of one. Every extra tool is a wandering opportunity.
- For the **managed tier**, audit the tool surface against the
  delegated authority. Elevated tools (anything that mutates real
  systems) require a refusal-conditions section.

### Return contract

- **Maximum length.** "Return at most 10 lines summarizing the
  finding. Raw tool output goes in scratch, not the return."
- **Required fields.** A return without an explicit success/failure
  field is a silent-failure risk. Make the field mandatory.
- **No raw dumps.** If the parent needs the raw output, the subagent
  should be a tool call, not a subagent.

## REFACTOR phase

Re-run all scenarios. Watch for:

- Parent now over-delegates → tighten description with negative triggers.
- Subagent now returns bloated digests → tighten return-contract length.
- Subagent refuses too aggressively → loosen refusal conditions.
- Authority injection succeeds → revoke tools, not just patch the prompt.
- New rationalizations in either direction → counter explicitly in the
  system prompt or description.

## Red flags — STOP and reconsider

- Description that summarizes the workflow rather than triggers.
- Return contract longer than 30 lines for a "summary" return.
- A subagent that calls another subagent (re-entrancy is a separate
  design problem; address it explicitly or refuse to design it).
- "The parent will know what to do with the output" — no, specify it.
- A subagent that needs the parent's full context to function — it
  is a skill, not a subagent.
- A managed-tier subagent without a refusal-conditions section.

## The three scope tiers

The three tiers each carry additional design requirements:

- **Managed** (CI-deployed, elevated permissions, non-overridable):
  requires the authority test. Tool surface audited against delegated
  permissions. Refusal conditions stated and tested under injection.
- **Project-scoped** (domain-specific per repo): description should
  reference the repo or domain explicitly so it does not over-fire
  in adjacent contexts.
- **User-global** (via `devex-setup`): broader description, but tool
  surface must not include anything that mutates org-shared state.

## Composition with other layers

- A subagent often loads skills inside its own context. Pair
  `writing-subagents` work with `superpowers:writing-skills` when the
  subagent has a non-trivial procedure.
- A subagent that needs live data uses MCP tools — those go through
  the MCP server authoring process, not here.
- Rules that govern the subagent's behavior live in the subagent's
  own system prompt, not in the parent's CLAUDE.md.

## Honest caveat

This skill is v0.1.0. The two-interface decomposition and the
managed-tier authority test are extensions specific to subagents; they
build on the testing methodology in
`obra/superpowers/skills/writing-skills` but have not been validated
against an eval set the way superpowers' original methodology has.
Before declaring this skill ready, run it through its own
RED-GREEN-REFACTOR cycle using real subagent designs from the
framework's priority list (explorer, test-runner, git-assistant first;
license-checker and classification-guard next).

## You Are Done When

- The subagent system prompt exists as an AGENT.md with valid frontmatter
- The authority test passes (subagent stays in scope)
- The return contract is explicit and the parent can consume the output
- `upskill lint` passes
