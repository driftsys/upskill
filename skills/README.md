# driftsys prompt-engineering meta-skills

This directory is the source registry for the v0.1 draft set of
meta-skills for the driftsys prompt-engineering discipline, managed via
upskill.

## Layout

The directory follows the upskill conventional source-registry layout
(items and bundles flat under `skills/`; see
<https://driftsys.github.io/upskill/conventions.html>):

```text
skills/
├── README.md                       (this file)
├── NOTICE                          attribution for vendored methodology
├── prompt-engineering.bundle.yaml  one-shot install of the discipline
├── upskill-prompt-design/          umbrella / orientation
│   └── SKILL.md
├── upskill-prompt-distilling/      decompose authoring intent + place across layers
│   └── SKILL.md
├── upskill-writing-rules/          per-layer authoring methodology
│   └── SKILL.md
├── upskill-writing-subagents/      per-layer authoring methodology
│   └── SKILL.md
├── upskill-evaluating-prompts/     RED-GREEN-REFACTOR cycle + scenario batteries
│   └── SKILL.md
├── upskill-writing-bundles/        .bundle.yaml authoring + plugin declarations
│   └── SKILL.md
└── upskill-cli/                  lifecycle operations
    └── SKILL.md
```

## Status

All skills are v0.1.0 except `upskill-prompt-distilling` (v0.3.0, renamed from
`picking-the-layer`) and `upskill-cli` (v0.2.0, rewritten against the
real CLI surface). None have yet been through their own
RED-GREEN-REFACTOR cycle. Each carries an "Honest caveats" section
flagging this.

## Not yet present

- `writing-skills` — to be vendored from `obra/superpowers` (MIT).
  Tracked in [driftsys/upskill#146](https://github.com/driftsys/upskill/issues/146).
  Until it lands, skill authoring methodology is deferred to the
  upstream superpowers source; the handoff tables in the other skills
  reference `writing-skills` as a forward pointer.

## Install

This bundle pairs with [`obra/superpowers`](https://github.com/obra/superpowers).
Install both for the full stack:

- **superpowers** — engineering discipline (TDD, debugging, brainstorming,
  plans, code review, worktrees).
- **prompt-engineering** (this bundle) — authoring discipline for durable
  agent content (rules, skills, agents).

### 1. Install superpowers

Superpowers ships as a per-harness plugin; the install command differs by
client. Pick the one matching your setup:

| Client      | Command                                                         |
| ----------- | --------------------------------------------------------------- |
| Claude Code | `/plugin install superpowers@claude-plugins-official`           |
| Codex CLI   | `/plugins` → search "Superpowers"                               |
| Gemini CLI  | `gemini extensions install https://github.com/obra/superpowers` |
| Cursor      | `/add-plugin superpowers`                                       |
| Copilot CLI | `copilot plugin install superpowers@superpowers-marketplace`    |

If you use more than one harness, install superpowers separately for each.

### 2. Install upskill's prompt-engineering bundle

`upskill add` resolves a bundle when the `source` argument points at the
`.bundle.yaml` file directly. From the upskill repo root (treating this
directory as a local source registry):

```bash
upskill add ./skills/prompt-engineering.bundle.yaml
```

Or, once published, from the canonical source (using the `:subfolder`
syntax to point at the bundle file inside the repo):

```bash
upskill add driftsys/skills:skills/prompt-engineering.bundle.yaml
```

To install one item without the rest of the bundle, point at the
registry directory and pass the item name as a filter:

```bash
upskill add ./skills upskill-prompt-design   # just the umbrella skill
```

See `upskill-cli/SKILL.md` for the full lifecycle.

## Recommended adoption sequence

1. Vendor `writing-skills` from superpowers.
2. Run `upskill-evaluating-prompts`' methodology against `upskill-prompt-distilling`
   (most-activated skill) to validate the meta-skill methodology.
3. Iterate based on findings.
4. Roll out to pilot team(s).
5. Iterate based on real authoring use.

## Out of scope here

- General one-shot prompting guidance (belongs in engineering
  onboarding documentation, not in the framework)
- MCP server authoring (separate repo's authoring process; see
  `mcp-builder` from `anthropics/skills`)
- RAG corpus management (lives behind specific MCP tools)
- Domain-specific skills (Android, IVI, Rust cluster) — these meta-skills
  help authors create those domain skills
