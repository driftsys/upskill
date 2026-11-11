# Copilot visibility of duplicate Claude and Copilot installs

Trial T1 on 2026-11-11 records what GitHub Copilot CLI lists and applies when
upskill installs the same skill, rule and agent for both Claude and Copilot in
one project. This note is informative.

## Method and pins

- Copilot CLI: GitHub Copilot CLI 1.0.109 (`copilot --version`).
- upskill: commit `e151b25`, built with `cargo build`.
- Source: one skill (`SKILL.md`), one rule (`RULE.md`, no `scope`) and one agent
  (`AGENT.md`, `mode: subagent`), each with `schema: 1`, `name`, `description`
  and a distinct marker string in the body.
- Project A: `git init`, then
  `upskill add ../source --claude --copilot
  --project` (exit 0, 6 files
  installed).
- Project B (control): `git init`, then
  `upskill add ../source --claude
  --project` (exit 0, 3 files installed, no
  `.github/` directory).
- Commands without a model, in each project: `copilot skill list` and
  `copilot
  instruction list`.
- Model requests: `copilot -s --no-auto-update -p "<prompt>"`, one new session
  per request, no permission flags. Four prompts in project A and two control
  prompts in project B.

In project A, the installed files were `.claude/skills/t1-skill/SKILL.md`,
`.claude/rules/t1-rule.md`, `.claude/agents/t1-agent.md`,
`.github/skills/t1-skill/SKILL.md`,
`.github/instructions/t1-rule.instructions.md` and
`.github/agents/t1-agent.agent.md`. The Copilot rule has `applyTo: '**'` in its
frontmatter; the Claude rule has only `name` and `description`.

## Observations

Direct observations (command output, no model):

- `copilot instruction list` in project A lists the rule twice, as two entries:
  one from `.github/instructions/t1-rule.instructions.md` and one from
  `.claude/rules/t1-rule.md`. In project B it lists the rule once, from
  `.claude/rules/t1-rule.md`.
- `copilot skill list` lists `t1-skill` once in both projects. The command
  prints no paths.
- In project B, Copilot lists the `.claude/` skill and the `.claude/` rule, so
  Copilot reads those locations.

Statements of the model only:

- Rule: in project A the model said the rule marker appears 2 times in its
  instructions, one copy with `applyTo: '**'` and one without; it could not name
  the source files. In project B it said the marker appears once.
- Skill: the model found two `SKILL.md` files by file search in project A, but
  said the skill marker appears once, from the skill it loaded. The model said
  that the loaded skill was the `.github/skills/t1-skill` copy.
- Agent: the model named two agent files in project A by file search and said
  its tool list shows a single `t1-agent`.

| Kind  | Listed by Copilot in project A      | Applied in project A         |
| ----- | ----------------------------------- | ---------------------------- |
| skill | once (`copilot skill list`)         | once (model statement)       |
| rule  | twice (`copilot instruction list`)  | twice (model statement)      |
| agent | not determined (no listing command) | not determined (not invoked) |

## Limits

- Copilot CLI has no command without a model that lists agents, so the number of
  times Copilot lists the agent could not be determined. The two model
  statements about the agent conflict in form (two files by search, one entry in
  the tool list).
- The agent was never invoked and its marker was not retrieved. Whether Copilot
  reads `.claude/agents/` at all was not determined.
- Whether the skill reaches the model as one entry or two was checked only by
  `copilot skill list` and the model's own statements.
- The rule application count rests on the model's count of its own context.
- The model used read-only file search tools during the requests. Copilot also
  reports user-level skills from `~/.copilot`, `~/.claude` and `~/.agents`;
  these are unrelated to the trial.

## Maintainer decision

On 2026-11-11 the maintainer decided that upskill keeps both copies of a rule:
the `.claude/rules` file and the `.github/instructions` file. With both copies,
one client can be removed without an effect on the other client. The known limit
is that Copilot lists and applies such a rule twice when both clients are
selected. The decision is recorded in AD-0042, which pull request #466 adds.
