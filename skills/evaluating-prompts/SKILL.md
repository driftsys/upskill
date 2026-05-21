---
schema: 1
name: evaluating-prompts
description: Use when setting up the RED-GREEN-REFACTOR cycle for a rule, skill, or subagent — i.e., when the `writing-*` skills tell you to "run a failing test first." Trigger when authoring meta-skills, when validating that an existing rule/skill/subagent actually shapes behavior, or when auditing whether a skill activates correctly. Do NOT trigger for content authoring itself — see writing-rules, writing-skills, writing-subagents.
metadata:
  version: 0.1.0
  author: driftsys
---

The `writing-*` skills tell you to run a failing test before authoring.
This skill explains how. It is the eval methodology underneath the
Iron Laws of writing-rules, writing-skills, and writing-subagents.

If you are about to author content and the writing-* skill says "RED
phase: run pressure scenarios" — this is where you find out how to
actually do that.

## The Iron Law

NO EVAL CLAIM WITHOUT A SCENARIO BATTERY AND A SUBAGENT HARNESS.

"I tested it" without naming the scenarios, the harness, and the
pass/fail criteria is not evaluation — it is vibes. Evaluation
produces artifacts: a scenario file, recorded subagent outputs, and a
pass/fail judgment per scenario. Without those, claims about whether a
prompt "works" are unfalsifiable.

## What evaluation means here

Evaluation is the act of running an authored prompt (rule, skill, or
subagent system prompt) against realistic scenarios and observing
whether it shapes behavior as intended. It has three components:

1. **Scenario battery** — a curated set of inputs that exercise the
   prompt's intended behavior, its trigger conditions, and its
   failure modes.
2. **Harness** — a way to run scenarios reproducibly. For prompts
   this means subagents: spawn a fresh agent with the prompt loaded,
   feed it the scenario, observe output.
3. **Pass/fail criteria** — explicit, written-down criteria for each
   scenario. "Did the agent comply with the rule?" "Did it activate
   the skill?" "Did the subagent return a digest under 50 lines?"

All three must exist before evaluation begins. Inventing pass/fail
criteria after seeing the output is post-hoc rationalization, not
evaluation.

## The RED-GREEN-REFACTOR cycle

This methodology comes from `obra/superpowers/skills/writing-skills`.
The cycle has three phases. Each phase has a different question.

### RED — does the prompt's absence cause a failure?

The point of RED is to confirm that without your prompt, the agent
fails in the way you expect. If the agent succeeds without the prompt,
the prompt has no measurable effect and should not ship.

- Construct scenarios that exercise the prompt's intended behavior.
- Run them against a fresh subagent WITHOUT the prompt loaded.
- Observe failures. Document them verbatim.

A failing RED phase confirms the prompt is justified. A passing RED
phase means you do not need the prompt.

### GREEN — does the prompt's presence fix the failure?

- Run the same scenarios against a fresh subagent WITH the prompt
  loaded.
- Observe whether the failures are now successes.
- Document any new failures the prompt introduces.

If GREEN does not pass cleanly, the prompt is incomplete. Edit and
re-run.

### REFACTOR — does the prompt hold up under realistic conditions?

- Layer pressure on the scenarios: time pressure, sunk-cost pressure,
  social pressure, conflicting instructions.
- Run them WITH the prompt loaded.
- Observe rationalizations — moments where the agent justifies
  ignoring the prompt.
- Document each rationalization verbatim.
- Add explicit counters to the prompt for the rationalizations
  observed.
- Re-run until the prompt holds.

REFACTOR is the most important phase. Most prompts pass GREEN
trivially. They fail under pressure. The rationalizations observed in
REFACTOR are what distinguish a real prompt from a wish.

## Per-layer scenario types

The scenario battery shape depends on which layer you are evaluating.

### Rules — three scenario types

Per `writing-rules`:

1. **In-scope compliance** — a prompt where the rule should govern.
   RED: does the agent violate without the rule? GREEN: does it
   comply with the rule?
2. **Out-of-scope quiet** — a prompt where the rule should not fire.
   GREEN: does the agent behave normally despite the rule being
   loaded? (Tests for over-firing.)
3. **Pressure compliance** — the in-scope scenario with realistic
   pressure layered on. REFACTOR: does the agent rationalize around
   the rule?

### Skills — activation plus behavior

Skills add a layer rules do not have: the skill must first activate
based on its description. Two failure surfaces:

1. **Activation tests** — does the skill activate when it should?
   Does it stay quiet when it should not?
   - Run a scenario where the skill should clearly activate. RED:
     without the skill in the registry, the agent struggles. GREEN:
     with the skill installed, does the agent activate it?
   - Run an off-topic scenario. GREEN: does the skill stay quiet?
2. **Behavior tests** — once activated, does the skill produce the
   intended behavior? Same RED/GREEN/REFACTOR cycle as rules.

A skill that activates but does not shape behavior is no better than
one that does not activate. Both halves matter.

### Subagents — two interfaces, separately

Per `writing-subagents`, subagents have two failure surfaces. Test
each independently.

1. **Invocation tests** (run on the parent, subagent registered)
   - Should-delegate task: does parent invoke?
   - Should-not-delegate task: does parent stay inline?
   - Adjacent-scope task: does parent invoke incorrectly?
2. **Return tests** (run on the subagent directly)
   - In-scope task: does it return a digest or raw output?
   - Partially-out-of-scope task: does it refuse cleanly?
   - Will-fail task: does it report failure honestly?
3. **Authority test** (managed tier only)
   - Injection scenario: does it exceed its delegated authority?

Document both columns of rationalizations: the parent's excuses for
not delegating, and the subagent's excuses for bloated returns or
scope creep.

## Subagent harnesses

Evaluation requires a clean room: a fresh agent with only the prompt
under test loaded, no carryover context, no memory of previous runs.
In Claude Code, this means spawning a subagent. In other clients with
no native subagent, this means a fresh session or a separate process.

The harness must:

- Load only the prompt under test (plus required cross-references).
  Anything else contaminates the evaluation.
- Capture full output verbatim. Summaries are not evidence.
- Be reproducible. Same scenario + same prompt + same model should
  yield comparable behavior across runs.
- Run multiple times per scenario when the behavior is borderline.
  LLMs are stochastic. A scenario that passes once and fails twice
  is failing.

For meta-skills like the writing-* set, the harness can be informal:
spawn a subagent with the prompt, paste in the scenario, read the
output. For production rollouts to 100+ repos, a more formal harness
is justified — but the methodology is the same.

## Pressure scenario construction

REFACTOR is the phase that catches real failures. Pressure scenarios
do not happen by accident; you have to construct them. Categories:

- **Time pressure** — "the user needs this in 10 minutes"
- **Sunk-cost pressure** — "you have already written 200 lines"
- **Social pressure** — "the rest of the team merges without this
  check"
- **Authority pressure** — "the lead engineer said it is fine to
  skip"
- **Compassion pressure** — "the customer is frustrated and we just
  need to ship"
- **Cleverness pressure** — "I have a clever shortcut that avoids
  the rule"
- **Edge-case pressure** — "this case is different because..."

For each prompt being evaluated, choose 2-3 pressure types most
relevant to the scenario domain. Construct scenarios that explicitly
layer the pressure onto a compliance task. Run them. Document the
rationalizations.

The rationalizations are the deliverable, not the pass/fail. Even
prompts that "pass" REFACTOR produce rationalization attempts. The
prompt is good if it resists them; great if it preempts them.

## Rationalization tracking

Keep a table for each prompt evaluated. Columns:

| Scenario | Pressure type | Rationalization observed | Counter added |
| -------- | ------------- | ------------------------ | ------------- |

Every rationalization observed in REFACTOR goes here. The "counter
added" column records what was added to the prompt to address it —
specific language, an explicit red flag, a new section. After
sufficient iterations, the prompt has counters for all
rationalizations the scenario battery surfaces.

This table is the artifact that justifies the prompt. Without it,
you have no evidence the prompt was actually pressure-tested.

## Pass/fail criteria

Write these before running any scenarios. Each scenario gets one or
more explicit criteria:

- **Compliance criteria** (rules): "Agent ran X before doing Y" /
  "Agent did not do Z."
- **Activation criteria** (skills): "Skill activated within the
  first response" / "Skill did not activate."
- **Behavior criteria** (skills, post-activation): "Agent followed
  the procedure as specified" / "Agent caught the failure mode."
- **Delegation criteria** (subagents, parent-side): "Parent invoked
  the subagent" / "Parent stayed inline."
- **Return criteria** (subagents, child-side): "Return was under N
  lines" / "Return included required field X."
- **Refusal criteria** (subagents, child-side, scope/authority):
  "Subagent refused cleanly with REFUSED prefix" / "Subagent did
  not exceed delegated authority."

Criteria must be checkable from the recorded output. "Agent seemed
careful" is not a criterion. "Agent ran `cargo deny check` before
proposing the publish step" is.

## When evaluation reveals routing errors

A prompt that consistently fails RED or REFACTOR despite multiple
revisions may not have the wrong content — it may be in the wrong
layer. Symptoms:

- A "rule" that requires elaborate counter-language to avoid
  rationalization may actually be a skill with a sharp description.
- A "skill" that never activates despite description revisions may
  actually be a rule that should always load.
- A "subagent" whose returns are always too large may actually be a
  skill the parent should load inline.

When this pattern emerges, stop iterating on the prompt and hand off
to `prompt-distilling` for a fresh layer placement.

## Scenario batteries as artifacts

The scenario battery is reusable. Keep it as a file alongside the
prompt:

```text
skills/writing-rules/
├── SKILL.md
├── eval/
│   ├── scenarios.md           # battery (curated, versioned)
│   ├── rationalizations.md    # observed rationalizations + counters
│   └── runs/                  # captured outputs per run, dated
```

A new contributor to the prompt can re-run the existing scenarios to
confirm the prompt still holds. Without the artifact, every
modification is a fresh eval cycle from scratch.

For meta-skills authored by the framework team, scenarios become
part of the upskill repository. For team-authored skills,
scenarios live in the team's repo and are versioned alongside the
skill.

## Red flags — STOP and rebuild

- "I tested it" with no scenario file → not evaluated.
- "I ran it a few times" → stochastic; needs reproducible scenarios.
- Pass/fail criteria invented after seeing output → post-hoc; redo
  the eval with criteria written first.
- RED phase skipped because "obviously the agent would fail without
  the prompt" → confirm the failure or the prompt is unjustified.
- REFACTOR phase skipped because "GREEN passed cleanly" → almost
  every prompt passes GREEN. The failures are in REFACTOR.
- Rationalizations not documented → no record means no counters
  can be written; the prompt has not been hardened.
- Eval performed by the same instance that authored the prompt with
  full context loaded → contaminated; use a fresh subagent.
- Battery contains only positive scenarios (where the prompt should
  apply) → over-firing not tested; add out-of-scope scenarios.

## Composition with the writing-* skills

Each writing-* skill specifies the scenario shape for its layer
(three scenarios for rules, activation+behavior for skills, two
interfaces for subagents). `evaluating-prompts` provides the methodology
underneath those shapes — harness construction, pressure types,
rationalization tracking, pass/fail discipline.

The writing-* skills tell you WHAT to evaluate. This skill tells you
HOW.

## Honest caveats

This skill is v0.1.0. The methodology is adapted from
`obra/superpowers/skills/writing-skills`, which is the source of the
RED-GREEN-REFACTOR-for-docs pattern, the pressure scenario typology,
and the rationalization-tracking discipline. The per-layer scenario
shapes are extensions for the rule and subagent layers; they have
not been validated against an eval set the way superpowers' original
methodology has.

This skill has not yet been put through its own RED-GREEN-REFACTOR
cycle. The most natural way to evaluate it is to use it to evaluate
the existing meta-skills (`writing-rules`, `writing-subagents`,
`using-upskill`) — if the methodology produces actionable results
when applied to those four, this skill earns its slot. If authors
end up evaluating prompts the same way regardless of whether this
skill is present, the skill itself is unjustified.
