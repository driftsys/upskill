---
schema: 1
name: using-upskill
description: Use when working in an upskill-consumer repo and needing to add, modify, vendor, audit, or remove installed content (rules, skills, agents, bundles). Trigger when `upskill lint` or `upskill doctor` reports issues. Trigger when adding a third-party bundle or updating one from upstream. Trigger when generated per-client files (e.g., `.claude/`, `.github/skills/`) look stale or wrong. Do NOT trigger for actual authoring decisions — hand off to prompt-distilling, writing-rules, writing-skills, or writing-subagents. Do NOT trigger for cosmetic typo fixes in skill bodies; raw edits to source registry files plus `upskill lint` are sufficient.
metadata:
  version: 0.2.0
  author: driftsys
---

upskill manages the lifecycle of installable agent content. It does not make
authoring decisions for you. This skill covers operations: add, update,
remove, list, doctor, lint, fmt, new. For authoring decisions (where does
this go, how do I write it), hand off to `prompt-distilling` and the
writing-* skills.

## Two sides of the workflow

upskill files live in two places. The commands you reach for depend on which
side you are on.

- **Source registry** — the repo where SSOT content is authored. Holds a
  `skills/` directory with item subfolders (`skills/<name>/SKILL.md`,
  `RULE.md`, `AGENT.md`) and bundle manifests (`skills/<name>.bundle.yaml`). The
  layout is documented in
  [Conventions](https://driftsys.github.io/upskill/conventions.html). Author
  commands: `new`, `fmt`, `lint`.
- **Consumer project** — a downstream repo that installs from one or more
  source registries. Holds generated per-client output under `.claude/`,
  `.github/`, `.opencode/`, etc., plus a `.upskill-lock.json` recording what
  is installed. Consumer commands: `add`, `update`, `remove`, `list`,
  `doctor`.

`upskill lint` refuses to run inside a consumer project, and `upskill add`
infers consumer scope from the current directory. Knowing which side you are
on tells you which command surface is even available.

## The Iron Law

GO THROUGH UPSKILL FOR LIFECYCLE OPERATIONS. HAND OFF TO writing-*
SKILLS FOR AUTHORING DECISIONS.

This Iron Law is a routing law, not a discipline law (no "NO X
WITHOUT Y FIRST" shape like the other meta-skills), because
`using-upskill` governs which tool to reach for, not whether to
pressure-test content before shipping it.

The boundary is sharp. upskill handles fetching, frontmatter
canonicalization, per-client generation, validation, and lockfile tracking.
It does not decide whether a piece of content should be a rule, skill, or
agent — that is `prompt-distilling`'s job. Mixing these concerns is the most
common usage error.

## When to use upskill vs raw edits

| Operation                                              | Use upskill    | Raw edits OK                                  |
| ------------------------------------------------------ | -------------- | --------------------------------------------- |
| Authoring a new item in a source registry              | Yes (`new`)    | No                                            |
| Adding installable content to a consumer project       | Yes (`add`)    | No                                            |
| Updating installed content from its source             | Yes (`update`) | No                                            |
| Removing installed content                             | Yes (`remove`) | No                                            |
| Fixing a typo in a source SSOT body                    | Optional       | Yes, then `upskill lint`                      |
| Editing frontmatter in source SSOT                     | Yes (`fmt`)    | No — manual edits break canonicalization      |
| Auditing installed content for drift                   | Yes (`doctor`) | No                                            |
| Hand-editing a generated `.claude/` or `.github/` file | Never          | No — these are regenerated and drift silently |

When in doubt, use upskill. Raw edits are a sharp tool — only reach for them
on content you authored and understand, and always lint after.

## Common workflows

### Authoring a new skill in a source registry

1. Run `prompt-distilling` first to confirm the content belongs in a skill
   (not a rule, agent, or MCP tool).
2. Hand off to `superpowers:writing-skills` to draft the SKILL.md.
3. From inside the registry's `skills/` directory, run
   `upskill new skill <name>`. This scaffolds `skills/<name>/SKILL.md` with
   the right shape.
4. Edit the SKILL.md body to match your draft.
5. Run `upskill fmt` to canonicalize frontmatter and run dprint over the
   body.
6. Run `upskill lint` (or `upskill lint --strict` for CI) to catch schema or
   structural errors.
7. Commit the SSOT. Consumer projects pick up the change at next
   `upskill update`.

### Adding installable content to a consumer project

1. Identify the source: either an org repo (e.g. `driftsys/skills`), a
   GitLab path (`gitlab:team/repo`), a full https URL, or a local path
   (`./local-source`).
2. From the consumer project root, run `upskill add <source>` to install
   every item in the source, or `upskill add <source> item-a item-b` to
   install a named subset.
3. Verify with `upskill list` — the new items appear with their hashes and
   their generated per-client output paths.
4. Commit the changed `.upskill-lock.json` and the generated per-client
   files your repo policy says to commit.

### Updating installed content

1. Run `upskill update` to refresh everything, or `upskill update item-a`
   to refresh one item. The command always re-fetches the source.
2. Use `upskill update --dry-run` first if you want a preview. It hashes
   what the new install would produce and reports what would change without
   writing.
3. Review the diff. Pay attention to:
   - Changed `description` fields — these affect skill activation behavior
     and may need eval before accepting.
   - Removed items — confirm no downstream content depends on them.
   - New items pulled in via bundle expansion.

### Vendoring a third-party bundle (e.g., superpowers)

upskill has no separate `vendor` command; vendoring is just `add` from the
upstream source. The vendoring discipline lives in what you record
alongside the install.

1. Confirm the upstream license is compatible with your repo's licensing
   policy.
2. Run `upskill add <upstream-spec>` to fetch the bundle into the consumer
   project.
3. Add a `NOTICE` file at the consumer-project root with full attribution
   to the upstream author and the license text. Most upstream licenses
   require this.
4. Commit the lockfile, the generated per-client output, and the NOTICE.

### Removing installed content

1. Run `upskill remove <item-name>` (or `--bundle <name>` for a whole
   bundle, see `upskill remove --help`).
2. Verify with `upskill list` that the item is gone and `upskill doctor`
   that nothing else depended on it.
3. Commit the changed lockfile and the deletion of the now-removed
   per-client files.

### Auditing for drift or misrouted content

1. Run `upskill doctor`. It verifies the installed state matches the
   lockfile (per-client files present, hashes consistent, no orphans).
2. For each finding, decide whether the drift is in the consumer project
   (someone hand-edited generated output → re-run `upskill update`) or in
   the source registry (an item changed shape upstream → review and
   `upskill update`).
3. If `doctor` surfaces a content-shape concern (vague description,
   oversized rule), the fix lives in the source registry — hand off to
   `prompt-distilling`'s audit section there.

## Interpreting upskill output

### `upskill lint`

Validates SSOT files in a source registry against the format spec. Refuses
to run in a consumer project (detected by `.upskill-lock.json`). Default
mode emits warnings and exits 0 unless an error rule fires; `--strict`
promotes warnings to errors for CI use.

Common findings:

- Missing `schema:` / `name:` / `description:` in frontmatter
- `metadata.version` not quoted (YAML float coercion risk)
- Invalid directory layout (entry file not in its own folder, etc.)
- Bundle `items:` list references a name not present in the registry
- Inline directive syntax errors (unbalanced `<!-- @client:X -->` blocks)

### `upskill fmt`

Canonicalizes frontmatter and runs dprint over the body. Idempotent.
Always safe to run; CI should fail if `fmt` changes anything.

### `upskill doctor`

Verifies the installed state of a consumer project against its
`.upskill-lock.json`. Pure consistency check — no judgment on content
shape. Common findings:

- Generated per-client file missing or hash-changed (someone hand-edited
  it → re-run `update`)
- Lockfile references an item whose source is no longer reachable
- Orphan per-client files not recorded in the lockfile (a previous
  install or hand-creation that needs reconciling)
- Plugin recorded as installed but missing from the client (uninstalled
  out-of-band → `update` reinstalls)
- Plugin skipped at install time because the client CLI was not on PATH
  (informational warning — install the CLI then run `update`)

Exits 0 when clean. Exits 1 when drift is found (missing outputs, stale
hashes, orphan entries, or missing plugins). Skipped plugins are
warnings only and do not affect the exit code.

## Conventions to remember

### Source side vs consumer side

- **Source registry**: holds `skills/<name>/SKILL.md`, `RULE.md`,
  `AGENT.md`, and `skills/<name>.bundle.yaml`. Author here; commit here.
- **Consumer project**: holds generated `.claude/`, `.github/`,
  `.opencode/` content plus `.upskill-lock.json`. Generated files are
  produced by `upskill add` / `upskill update`. **Never hand-edit
  generated files** — `upskill update` will overwrite them and the
  intervening drift is silent.

### `.upskill-lock.json`

The state of truth for what is installed in a consumer project. Records
each item's source, ref, hash, and generated file paths. Edit only via
`upskill add`, `update`, and `remove`. Commit it. Default location is the
consumer project root; with `--global`, it moves to `$HOME/`.

### Cross-platform portability

upskill uses file copies, not symlinks, so generated content works on
Windows. Generated copies are regenerated at `add`/`update` time. Don't
rely on inode identity or hard links between source and generated.

### Org-specific access control

Some orgs gate skill installation through a classification or
allow-list mechanism (an MCP-served registry, a private GitLab group,
an org-internal upskill registry). If `upskill add` refuses to fetch
from a particular source, check whether the source is part of your
org's allow-list before suspecting a bug. The mechanism is org-specific;
this skill does not assume one exists.

## Handoff matrix

| Situation                                              | Hand off to                                              |
| ------------------------------------------------------ | -------------------------------------------------------- |
| Newcomer to the framework needing orientation          | `prompt-design`                                          |
| Deciding where new content belongs                     | `prompt-distilling`                                      |
| Authoring a new rule                                   | `writing-rules`                                          |
| Authoring a new skill                                  | `superpowers:writing-skills`                             |
| Authoring a new subagent / agent                       | `writing-subagents`                                      |
| Setting up RED-GREEN-REFACTOR for any authored content | `evaluating-prompts`                                     |
| Authoring a new MCP server                             | the MCP server repo's authoring process (out of scope)   |
| Adding to a RAG corpus                                 | the corpus's MCP tool's authoring process (out of scope) |
| Routing decision on existing content showing drift     | `prompt-distilling` audit section                        |

## Red flags — STOP

- About to hand-edit a generated `.claude/`, `.github/`, or `.opencode/`
  file → don't. Edit the source registry and run `upskill update` in the
  consumer project.
- About to skip `upskill lint` in a source registry because "it's just a
  small change" → don't. Lint is cheap and catches frontmatter
  corruption before commit.
- About to commit generated artifacts because "the next person won't
  have upskill installed" → that is the right reason to commit them, but
  it is also the reason `update` is mandatory after any source change.
  Drift between SSOT and generated output is the dominant silent
  failure mode.
- About to vendor a bundle without a NOTICE file → MIT and similar
  licenses require attribution. Don't ship without it.
- About to write a skill body explaining HOW upskill works (every flag,
  every exit code) — that belongs in `upskill --help` and the upskill
  user guide, not in a skill. Skills explain _workflows and decisions_,
  not CLI invocations.

## Honest caveats

This skill is v0.2.0 and tracks upskill's current command surface
(`add`, `remove`, `update`, `list`, `doctor`, `search`, `lint`, `fmt`,
`new`). As upskill evolves, this skill needs maintenance —
specifically, it should NOT enumerate every CLI flag (that's
`upskill --help`'s job). It should remain at the workflow and
decision-boundary level, which is more stable.

This skill has not yet been through its own RED-GREEN-REFACTOR cycle.
Before declaring it ready, run a battery of real upskill-operation
scenarios (author a new skill, add a bundle, update from upstream,
audit, hand-edit a generated file) past a subagent without the skill,
then with, and measure whether the agent makes correct workflow
decisions in both cases.
