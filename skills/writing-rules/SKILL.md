---
schema: 1
name: writing-rules
description: Use when adding or editing rules in CLAUDE.md, AGENTS.md, or any always-loaded instructions file. Trigger when authoring repo conventions, invariants, or behavioral guardrails. Also trigger when an existing rule is being violated despite being present — that means the rule needs refactoring, not the agent. Do NOT trigger for skill or subagent authoring; see writing-skills and writing-subagents.
metadata:
  version: 0.1.0
  author: driftsys
---

A rule is a line that costs tokens on every turn in exchange for shaping
behavior across the whole scope. If you cannot justify the per-turn cost,
it is a skill, not a rule. Confirm the routing decision via
`prompt-distilling` before continuing here.

## The Iron Law

NO RULE WITHOUT A FAILING TEST FIRST.

Run a pressure scenario WITHOUT the rule and document the exact violation
before authoring. If the agent does not fail without the rule, the rule has
no measurable effect and should not ship.

## Failure modes this skill prevents

| Failure mode              | Signature                                                                                                         |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Bloat blindness**       | Rule buried in a long instructions file; agent skims past it.                                                     |
| **Soft-language drift**   | "Consider running X" → agent does not.                                                                            |
| **Negation-as-prompt**    | "Never auto-merge MRs" surfaces auto-merge as an option the agent would not otherwise have considered.            |
| **Pressure capitulation** | Agent rationalizes around the rule when given time, sunk-cost, or social pressure.                                |
| **Over-firing**           | Rule applies broadly but is relevant on 20% of work; agent invokes it on the other 80% and slows everything down. |
| **Conflict ambiguity**    | Rule contradicts another rule, the user's prompt, or observed code. Agent picks arbitrarily.                      |
| **Staleness**             | Codebase moved on; rule still references the old way.                                                             |

The dominant failure is not "agent fails to load the rule" (rules always
load). It is "agent loads the rule and ignores it under pressure" or
"agent applies the rule where it should not."

## RED phase — three scenario types per rule

Rules need three test scenarios, not one. Run each with a subagent.

1. **In-scope compliance** — a prompt where the rule should govern.
   Without the rule, does the agent violate? (Should fail.)
2. **Out-of-scope quiet** — a prompt where the rule should not fire.
   Without the rule, the agent behaves normally; with the rule, does
   it still behave normally? (Should not over-fire.)
3. **Pressure compliance** — the in-scope prompt with realistic
   pressure layered on: time ("user needs this in 10 minutes"), sunk
   cost ("you already wrote 200 lines"), social ("the rest of the
   team merges without this check"). Does the agent rationalize?

Document every rationalization verbatim. Those become your counters in
the GREEN phase. `evaluating-prompts` covers harness construction,
pressure typology, and rationalization tracking — activate it before
running the scenarios.

## GREEN phase — writing the rule

- **Imperative voice.** "Run `cargo deny check` before publishing", not
  "consider running `cargo deny check`".
- **State the trigger condition explicitly.** Rules that say what to do
  without saying when fire too broadly.
- **Prefer positive form to negation.** "Always run `cargo deny` before
  publish" beats "never publish without running `cargo deny`" — the
  positive form does not surface the bad option as a candidate.
- **One rule per line.** Multi-clause rules with exceptions ("never X
  except when Y and not Z") get partially applied. Split them.
- **Two-sentence ceiling.** A rule longer than two sentences is either
  a skill in disguise or several rules glued together.
- **Justify the per-turn cost in writing, outside the rule file**
  (the PR description, an ADR, or the registry's review record —
  inline comments in CLAUDE.md still load every turn). Authors who
  cannot write the justification often discover the rule does not
  deserve to be a rule.

## REFACTOR phase

Re-run all three scenarios WITH the rule. Watch for:

- New rationalizations the rule did not anticipate → add explicit
  counters in a red-flags section attached to the rule.
- Over-firing on out-of-scope prompts → tighten the trigger condition.
- Pressure capitulation → strengthen language and add a red-flag list
  of symptoms-of-about-to-violate.
- Conflict with other rules → resolve explicitly, do not leave
  ambiguous.

Repeat until all three scenarios pass cleanly.

## Red flags — STOP and rewrite

- "Consider", "try to", "where appropriate", "if possible", "where
  relevant" — all signal soft language; agents treat them as optional.
- A rule longer than two sentences.
- A rule that requires a sub-clause to disambiguate.
- A rule whose description (or the comment justifying it) matches more
  turns than its actual relevance.
- A rule added "to be safe" without a failing test.
- Negation in the imperative where a positive form would work.

## Token budget discipline

Every rule line is paid on every turn. Before adding one, compute the
cost:

```text
lines × avg_turns_per_session × active_users
```

For an org-level framework propagating to ~100 repos, this compounds
fast. If a rule fires correctly only 1 in 20 turns, it almost certainly
belongs in a skill with a sharp description, not in the always-on
rule layer.

## Composition with other layers

- A rule states an invariant. The skill explaining the procedure that
  respects that invariant goes alongside it. Pair `writing-rules` work
  with `superpowers:writing-skills` when both apply.
- A rule cannot describe live state. If the invariant references "today's
  classification policy" or "the current owner of repo X", the live part
  belongs in an MCP tool; the rule should reference the tool, not the
  data.

## Honest caveat

This skill is v0.1.0, adapted from the methodology in
`obra/superpowers/skills/writing-skills` (which is anchored in tested
agent behavior under pressure). The three-scenario format and the
over-firing failure mode are extensions specific to rules; they have not
yet been validated against an eval set the way superpowers' original
methodology has. Before declaring this skill ready, run it against its
own RED-GREEN-REFACTOR cycle using a battery of real rule-authoring
requests from the org framework.

## You Are Done When

- A pressure scenario WITHOUT the rule produced a documented violation
- The rule is written in the target instructions file
- A pressure scenario WITH the rule produces correct behavior
- The rule passes `upskill lint` (if in SSOT format)
