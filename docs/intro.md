# upskill

`upskill` lets you author and distribute AI-assistance content —
**rules**, **skills**, and **agents** — across multiple AI coding
clients (Claude Code, GitHub Copilot, opencode) from a single source of
truth. One CLI, one set of files, per-client output generated on
demand.

```bash
upskill add owner/repo               # install everything from a source repo
upskill update                       # pull latest, regenerate per-client files
upskill list                         # see what's installed
upskill new skill code-review        # scaffold a new SSOT skill (in a registry)
```

## Two roles

| Role         | Lives in                          | Commands                                              |
| ------------ | --------------------------------- | ----------------------------------------------------- |
| **Consumer** | A project that uses AI assistance | `add`, `remove`, `update`, `list`, `doctor`, `search` |
| **Author**   | A source-registry repo            | `new`, `lint`, `fmt`                                  |

Consumer projects only ever contain **generated** per-client files.
Source registries hold the canonical SSOT items (`RULE.md`,
`SKILL.md`, `AGENT.md`, `*.bundle.md`). The same repo can play both
roles; SSOT and generated outputs do not mix in the same tree.

## Where to next

- **[Getting started](./getting-started.md)** — install, first add,
  first scaffold.
- **[Commands](./commands.md)** — full reference for every verb.
- **[Recipes](./recipes.md)** — CI usage, private repos, pinning.
- **[Specification](./specification.md)** — the contract.
- **[Portable format](./format-spec.md)** — the on-disk SSOT format.
