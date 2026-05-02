# upskill CLI surface

**Status**: Proposed (2026-05-01)

## Context

upskill is the developer-facing tool for the entire content lifecycle:
authoring, validating, distributing, installing. The CLI surface is the
contract developers actually touch — it must be discoverable, predictable,
and free of overlapping verbs.

v0.1's surface (`add` / `list` / `remove` / `check` / `search` / `update`)
was shaped around the skills-installer use case. v0.2's broader scope
(create, manage, prolong content for three kinds across three clients)
needs a surface that fits author workflows and bundle composition.

This ADR is one of four child ADRs of [ADR-0001](./0001-multi-kind-compiler-architecture.md).

## Decision

### Nine commands, no overlap

```text
upskill add <source> [items...]    Install content from any source.
upskill remove [<name>]            Remove installed content.
upskill update [<name>]            Pull latest, regenerate changed items.
upskill list [--available]         Show installed (or available) content.
upskill info <name>                Show item/bundle details.
upskill new <kind> <name>          Scaffold a new rule/skill/agent.
upskill doctor                     Verify installation consistency.
upskill lint [paths...]            Validate SSOT files.
upskill fmt [paths...]             Canonicalise YAML frontmatter.
```

### Author commands vs consumer commands

The nine commands split by where they run:

- **Author commands** — run inside a **source registry** working tree
  (where SSOT items live and are edited):
  - `new <kind> <name>` scaffolds a new SSOT item into the source repo.
  - `lint [paths...]` validates SSOT files.
  - `fmt [paths...]` canonicalises SSOT YAML frontmatter.
- **Consumer commands** — run inside a **consumer project** (where only
  generated outputs live):
  - `add`, `remove`, `update`, `list`, `info`, `doctor`.

Per [ADR-0002](./0002-portable-content-format.md), SSOT items only exist
in source registries; consumer projects never contain SSOT. Author
commands therefore have no meaning in a consumer project, and consumer
commands have no meaning in a source registry. Implementations MAY
detect the wrong context and emit a clear error.

### `add` unifies bundle and ad-hoc installation

Source resolution is automatic: upskill checks the registry index first;
if no match, treats the source as a git repo or local path. Developers
never decide between two install verbs. Source format parity with
`npx skills add` is the subject of
[ADR-0005](./0005-skills-sh-ecosystem-interop.md).

### Install all by default

`upskill add <source>` installs everything the source contains. Item
names after the source filter to a subset. No interactive picker, no
confirmation prompts. Bundles are curated — their contents belong
together.

### `update` absorbs sync

`update` always fetches latest sources before regenerating. There is no
separate `sync` command. No `--offline` flag in v1; offline use is
covered by passing local paths to `add`/`update`.

### `lint` absorbs validate

`upskill lint --strict` is the CI mode (warnings become errors). No
separate `validate` command. Lint scope is deliberately small (schema +
structural markdown checks; cross-file consistency); content-quality
review is delegated to AI workflows outside upskill.

### `fmt` is frontmatter only

`upskill fmt` canonicalises YAML frontmatter (key ordering, version
quoting, indentation). Markdown body formatting is dprint's job. The two
tools don't overlap.

### Default scope: `--project`, fall back to `--global`

`upskill add` writes into the current repo's `.agents/...` by default
(`--project`). Falls back to `$HOME/.agents/...` (`--global`) if the
current directory is not inside a git repo. Users can pass `--global`
or `--project` explicitly to override.

### Project lock file

Consumer commands (`add`, `remove`, `update`) read and write
**`.upskill-lock.json`** at the consumer project root. The lock file is
**committed alongside the project** — it records the bundles and items
added, their source-registry URLs, resolved git refs, and per-item
content hashes. Plays the same role as `package-lock.json`: deterministic
regeneration on another developer's machine or in CI. State design
detailed in [ADR-0003](./0003-generation-pipeline.md).

### "Prolong" is a CLI behaviour contract

Two distinct things, both surfaced through `update`:

- **Absorb client drift.** When Copilot or Claude Code change a path or
  frontmatter field, `update` regenerates outputs from the same SSOT.
  Authors don't have to react.
- **Track upstream evolution.** `update` git-pulls source repos and
  regenerates items whose source hash changed.

A single command covers both because both rerun the same generation
pipeline; differentiating them at the CLI level would force users to
remember which one they want.

### Aliases

`add` does **not** alias to `install`. `remove` does **not** alias to
`uninstall`. v0.1's `install` and `uninstall` are dropped — the unified
verb names are the canonical ones. Migration covered in v0.2.0 release
notes.

## Consequences

**Positive.** Predictable surface; no decision burden between two install
verbs. Bundle install is just `add <bundle>`. Small command count is
discoverable from `upskill --help`.

**Negative.** v0.1 users who scripted `upskill install ...` need to
update. Some users may expect interactive prompts (Vercel's `npx skills
add` doesn't have them either, so this aligns with the wider ecosystem).

## Alternatives considered

**(a) Keep `install`/`uninstall` aliases for one minor version**
(original ADR-0001 plan). Rejected: keeping aliases prolongs migration
ambiguity; better a clean break with release notes.

**(b) Separate `sync` and `update`.** Rejected: `update` is always-fetch;
`--offline` is not a v1 use case. Splitting forces users to learn two
verbs for what's effectively one operation.

**(c) Separate `validate` and `lint`.** Rejected: `--strict` is
sufficient differentiation; one command is easier to document and
discover.

**(d) Interactive multi-select prompt for partial install.** Rejected:
install-all-by-default fits bundle curation. Named-item filter handles
partial cases without TTY interaction (also: scriptable, CI-friendly).

## References

- Parent ADR: [ADR-0001](./0001-multi-kind-compiler-architecture.md)
- Sibling ADRs: [ADR-0002](./0002-portable-content-format.md),
  [ADR-0003](./0003-generation-pipeline.md),
  [ADR-0005](./0005-skills-sh-ecosystem-interop.md)
