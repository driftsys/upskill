---
schema: 1
name: prompt-distilling
description: Use BEFORE authoring any rule, skill, or subagent. Trigger when someone says "we should encode X", "the agent keeps forgetting Y", "let's add a rule for Z", or any variant where new behavior needs to live somewhere in the framework. Also trigger when reviewing existing content that is misbehaving — wrong-layer placement is a common silent root cause. Do NOT trigger for refining content already correctly placed; hand off to writing-rules, writing-skills, or writing-subagents.
metadata:
  version: 0.3.0
  author: driftsys
---

Before authoring anything, distill the intent down to its components.
Most authoring intents are not atomic — they mix invariants, procedures,
enforcement, live data, and reference material. Distilling means
separating those strands and placing each one in the layer that fits it.

Skipping this step produces single-layer answers to multi-layer questions,
which is the most common framework failure. It does not alarm; wrong-layer
content quietly underperforms while looking healthy in CI.

## The Iron Law

NO AUTHORING WITHOUT DISTILLING FIRST.

Distill the intent into atomic parts, then place each part in its layer.
State which layer each part is targeting and why, in terms of the
decision dimensions below. "It felt right" and "that's where we usually
put these" are not reasons. Bias toward the layer you know best is the
dominant failure mode this skill exists to prevent. The second most
common failure is routing a multi-part intent as if it were single-part.

## The five layers, at a glance

| Layer          | Loaded                  | Cost shape                        | Best for                                                               |
| -------------- | ----------------------- | --------------------------------- | ---------------------------------------------------------------------- |
| **Rule**       | Every turn              | Per-turn token tax                | Small invariants relevant on most turns                                |
| **Skill**      | On activation           | Description-routing cost only     | Procedures and domain knowledge                                        |
| **Subagent**   | On invocation           | Overhead + return digest          | Bounded sub-tasks whose intermediate work would pollute parent context |
| **MCP tool**   | Every turn (definition) | Per-turn tax + server maintenance | Live data or side effects on real systems                              |
| **RAG corpus** | On retrieval            | Indexing + retrieval              | Searchable static-ish content too large to inline                      |

## Distill before placing

Most authoring intents are not atomic. "We need license-checking when
adding new dependencies" looks like one request but contains:

- An invariant ("all dependencies must have approved licenses") → rule
- A list of approved licenses → rule, skill, or MCP tool depending on
  size and liveness
- A procedure ("how to add a dep including the check") → skill
- An enforcement mechanism ("scan and flag violations") → subagent or
  MCP tool

Distillation comes BEFORE layer selection. Skipping it produces
single-layer answers to multi-layer questions, which is the most
common framework failure after default-layer bias.

### The distillation questions

Apply to the authoring intent, in order. Multiple "yes" answers means
the intent spans multiple layers — place each part separately.

1. **Is there an invariant?** A statement of the form "X must always
   hold" or "Y must never happen" → rule candidate.
2. **Is there a procedure?** A sequence of steps to accomplish
   something → skill candidate.
3. **Is there enforcement or scanning?** Something that needs to run
   over content and report findings → subagent or MCP tool candidate.
4. **Is there live state or data?** Lookups against systems whose
   answer can change → MCP tool candidate.
5. **Is there bulk reference material?** A large corpus of supporting
   docs → RAG candidate.

### The instruction/subagent split

When the decomposition includes a subagent, draw the parent/subagent
boundary explicitly. The most common misplacement is putting the
subagent's HOW in the parent's instructions, bloating the parent's
always-loaded context with content it never needs.

**Goes in parent instructions** (CLAUDE.md / AGENTS.md):

- Universal invariants the parent must respect even when no subagent
  is active
- One-line awareness that the subagent exists ("a test-runner
  subagent is available for running and interpreting test failures")
- Rules governing when NOT to delegate, if over-firing is a concern

**Goes in the subagent's system prompt**:

- The subagent's scope (one sentence)
- The subagent's tools and refusal conditions
- The subagent's return contract
- Skills the subagent loads
- HOW the subagent does its work

**Goes in the subagent's description** (the activation surface the
parent reads):

- WHEN the parent should invoke the subagent
- Negative triggers (when NOT to invoke)

The principle: the parent must not know HOW the subagent works. It
needs only WHAT the subagent does (description) and WHEN to call it
(description). Anything beyond that in parent instructions is bloat
paid every turn.

**Symptoms of misplacement**:

- Parent CLAUDE.md contains a paragraph explaining how a subagent
  interprets its inputs → move to the subagent's prompt.
- Subagent system prompt says "invoke me when X happens" → that
  belongs in the subagent's description, not its prompt. The parent
  controls invocation; the subagent's prompt only governs behavior
  once invoked.
- Universal invariant ("never commit secrets") lives only in a
  security-check subagent's prompt → it also needs to be in parent
  instructions, because the subagent isn't always active.
- Subagent's procedural detail duplicated in parent instructions
  "for completeness" → delete from parent.

### Common decomposition patterns

A quick reference for recurring multi-layer authoring intents:

- **Rule + Skill** — invariant ("never commit secrets") plus a
  procedure that respects it ("how to onboard a credential via
  Vault").
- **Skill + MCP tool** — procedure that calls live tools ("how to
  triage a Jira incident" plus `jira_search`).
- **MCP + installer Subagent + Skill** — an MCP server whose local
  binary must be installed on the consumer machine (no `npx`/`uvx`
  launcher). Configure the server in a bundle's `mcps:`; put the
  install recipe — detect OS, version-check, install the binary — in a
  per-MCP `<name>-mcp-installer` subagent (the HOW); put the trigger in
  the skill that uses the MCP, as one authored line ("if the `<name>`
  MCP is not responding, run the `<name>-mcp-installer` subagent
  first"). upskill never runs the installer — the client's agent does,
  under its own permission prompts. See the recipe "Shipping an MCP
  that needs a local install".
- **Subagent + Skill** — subagent loads skills inside its own context
  (test-runner subagent plus a "how to interpret cargo-nextest
  failures" skill).
- **RAG + Skill** — skill describes when and how to query the corpus
  ("when to consult the ADR archive" plus `search_adrs`).
- **Rule + Subagent** — invariant in parent instructions plus an
  enforcement subagent that scans for violations.

If the intent fits one of these patterns, hand off to all relevant
authoring skills, not one.

## Decision dimensions, in priority order

Apply these to each distilled part separately. The first dimension
that resolves the decision for a given part wins.

1. **Liveness** — Can the answer change between this morning and this
   afternoon? Does this need to act on a real system? If yes → **MCP tool**.
2. **Corpus size** — Is the relevant content too large to inline as a
   skill? If yes → **RAG**, fronted by an MCP tool.
3. **Context isolation** — Would the intermediate work pollute parent
   context if done inline, and does the parent only need a digested result?
   If yes → **Subagent**.
4. **Frequency × size** — Does this apply to >70% of turns in scope AND
   fit in 1–3 imperative lines? If yes → **Rule**. If high frequency but
   bulky, the scope is wrong — narrow it and reconsider.
5. **Default** — **Skill**.

## Cost-of-placement, computed before authoring

Write the cost down before choosing. Refusing to do this is itself a red
flag.

- **Rule**: `lines × avg_turns_per_session × users`. A 5-line rule across
  10 teams of 8 engineers averaging 40 turns/day is roughly 16,000
  rule-line-turns per day. Justify it.
- **Skill**: near-zero ambient cost. The real cost is description
  quality — vague descriptions cause silent activation failure, which is
  worse than a clear refusal.
- **Subagent**: invocation overhead plus return-contract design cost.
  Justify by showing the parent context pollution that occurs without it.
- **MCP tool**: per-turn definition tax plus server maintenance, on-prem
  hosting, and auth surface. Justify by showing the data is genuinely
  live or the action genuinely needs to happen.
- **RAG**: indexing pipeline plus retrieval-quality tuning. Justify by
  showing the corpus is too large to inline as one or several skills.

## Red flags — STOP and re-route

- "Just put it in CLAUDE.md to be safe" — about to bloat the rule layer.
  Compute the cost first.
- "Let's make a skill for this" with no description draft — if you
  cannot write the trigger condition in one sentence, the skill will
  silently fail to activate.
- "Spin up a subagent" for work that returns more than ~50 lines — that
  is not isolation, that is relocation.
- "Add it to the MCP server" for content that does not need live data —
  building infrastructure for a markdown file.
- Authoring intent treated as atomic when it contains an invariant AND
  a procedure AND enforcement — distill first, then place each part.
- Placement decision made before running the distillation questions —
  high risk of single-layer answer to multi-layer question.
- A subagent's HOW being added to parent instructions for "clarity" —
  parent doesn't need it, pays the token cost every turn.
- Placement decided without referencing the decision dimensions — bias,
  not analysis.

## Misplaced-content audit

This skill triggers for audit, not just initial placement. Symptoms of
misplacement:

- A rule that fires correctly on <30% of turns → demote to skill.
- A skill whose description has been rewritten 3+ times trying to get it
  to activate → it is probably a rule masquerading as a skill, OR its
  scope is too broad.
- A subagent whose return values are routinely >50 lines → the parent
  should load it as a skill instead.
- An MCP tool that returns the same answer every time → it is a skill,
  not a live capability.
- A skill containing stepwise instructions with current MR numbers or
  today's policies → the live parts belong in MCP.
- Parent instructions contain procedural detail about how a subagent
  does its work → move that detail into the subagent's own system
  prompt. The parent only needs to know WHAT and WHEN.
- A subagent's system prompt contains its own invocation triggers
  ("invoke me when X") → move them to the subagent's description; the
  parent reads the description to decide invocation.
- A universal invariant lives only inside one subagent's prompt → also
  add it to parent instructions, since the subagent isn't always active.
- A single authoring artifact (one rule, one skill) tries to cover an
  invariant AND a procedure AND enforcement → it was a multi-layer
  intent that should have been distilled first.

When audit reveals misplacement, the fix is layer migration or
re-distillation, not content editing.

## Handoff

Once distilled and placed, hand off:

- Rule → `writing-rules`
- Skill → `superpowers:writing-skills`
- Subagent → `writing-subagents`
- MCP tool → the MCP server repo's authoring process (out of scope here)
- RAG corpus → the corpus that the relevant `search_*` MCP tool fronts
  (out of scope here)

## Why this skill exists

The other writing-* skills make individual artifacts better. This one
makes the system better, because misplaced content is invisible — it
does not fail loudly, it just quietly underperforms. A bloated rule
layer slows every turn by a few percent. A skill with a vague
description fails to activate but nothing alarms. A subagent dumping
raw output looks like it "worked." None of these show up in CI.

The audit clause is in the body, not a separate document, because
distilling and placing is not a one-time decision. It happens again at
every review.

## Honest caveat

This skill is v0.3.0 (renamed from `picking-the-layer`) and has not yet
been put through its own RED-GREEN-REFACTOR cycle. Before declaring it
ready, assemble a battery of 15–20 real authoring requests with
expert-classified correct distillations and placements, run them past
an author subagent without the skill present, then with, and measure
the delta. If authors already distill and place correctly without the
skill, the skill itself is unjustified.

## You Are Done When

- The behavior is classified into exactly one layer (rule, skill, or subagent)
- The rationale for placement is documented
- You have handed off to the appropriate `writing-*` skill
