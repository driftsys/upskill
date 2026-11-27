# upskill

[![CI](https://github.com/driftsys/upskill/actions/workflows/ci.yml/badge.svg)](https://github.com/driftsys/upskill/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/upskill)](https://crates.io/crates/upskill)
[![GitHub tag](https://img.shields.io/github/v/tag/driftsys/upskill?sort=semver)](https://github.com/driftsys/upskill/releases)
[![docs](https://img.shields.io/badge/docs-driftsys.github.io/upskill-blue)](https://driftsys.github.io/upskill/)

> Upskill your coding agents.

A single Rust binary for authoring and distributing AI-assistance content —
**rules**, **skills**, and **agents** — across multiple AI coding clients
(Claude Code, GitHub Copilot, opencode) from a single source of truth.

No Node.js. No npm. No runtime dependencies.

## Install (consumer)

Set up upskill in Claude Code or Copilot CLI with the `upskill` plugin from the
`driftsys/metapowers` marketplace. It gives you the upskill skills, prints the
command that installs the CLI when it is missing, and prints update notices:

```bash
claude plugin marketplace add driftsys/metapowers
claude plugin install upskill@metapowers

copilot plugin marketplace add driftsys/metapowers
copilot plugin install upskill@metapowers
```

If you installed the upskill skills before with `upskill add driftsys/upskill`,
the plugin now provides them, and the old install keeps the old names. Run
`upskill remove` on the seven old skills (add `--global` for a global install)
before your next `upskill update`:

```bash
upskill remove --global upskill-cli upskill-prompt-design upskill-prompt-distilling upskill-writing-rules upskill-writing-subagents upskill-evaluating-prompts upskill-writing-bundles
```

`update` no longer stops for a lockfile that records the bundle
`prompt-engineering`: it drops the bundle record and removes the bundle's items.
A lockfile written by an earlier upskill does not record which items you added
by name (the `direct` field), so `update` removes every item of that bundle,
also an item that you added by name. To keep such an item, run
`upskill add <source> <item>` again. For an install by item name, `update`
removes the old `upskill-cli` and keeps the other six old skills beside the
plugin's copies. If you already ran `update`, run `upskill list` and give
`upskill remove` only the old names that it still lists: `remove` fails for a
name that is not in the lockfile. See
[Getting started](docs/book/getting-started.md#set-up-the-upskill-plugin).

For opencode, or when you want no plugin, install the CLI and add the skills
with `upskill add`:

```bash
# Linux / macOS (Windows: run inside WSL)
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh

# Or, with a Rust toolchain:
cargo install upskill

# opencode, or no plugin: install the upskill skills
upskill add driftsys/upskill --global --opencode

# Install everything from a source repo (auto-fans out to .claude/, .github/, .agents/)
upskill add driftsys/skills

# Or install a curated bundle — one entry, dependency-resolved
upskill add driftsys/bundles:platform-baseline.bundle.yaml

# Pull latest and regenerate
upskill update

# Inspect installed state (both scopes: project, then global)
upskill list
upskill doctor

# Report sources that have a newer version, without a fetch
upskill update --check
```

`.upskill-lock.json` records what was installed at which ref and hash — commit
it alongside the project.

## Updates in a session

`upskill update --notice` prints one line for each scope that has updates. The
`upskill` plugin for Claude Code and Copilot CLI, published by the
`driftsys/metapowers` marketplace, runs it when a session starts. See
[Recipes](docs/book/recipes.md) for the install commands and what each client
shows.

## Support levels

Each client and kind has a support level (supported, experimental or
unsupported). `upskill doctor` prints the table; see
[Support levels](docs/book/support-levels.md).

## Author

```bash
# In a source-registry repo (where SSOT items live)
upskill new skill code-review
upskill lint --strict
upskill fmt
```

## Two roles, no overlap

- **Source registry** — repo where SSOT items live. Author commands run here:
  `new`, `lint`, `fmt`, `pack`.
- **Consumer project** — repo where generated outputs land. Consumer commands
  run here: `add`, `remove`, `update`, `list`, `doctor`, `search`.

The same repo can be both, but SSOT and generated outputs do not mix in the same
tree.

## Bundles

A bundle is a flat YAML manifest (`*.bundle.yaml`) that names the items it
includes, and optionally other bundles it depends on. Installing a bundle
resolves the transitive closure once and writes per-item output:

```bash
upskill add driftsys/bundles:platform-baseline.bundle.yaml
```

The lockfile records the bundle entry alongside the items, so
`remove --source <bundle-label>` and future `remove <bundle-name>` flows know
which items came from which bundle.

## Documentation

See the [documentation map](docs/README.md) for design records, technical notes,
working documents and archives.

- User guide: [Getting started](docs/book/getting-started.md),
  [Commands](docs/book/commands.md), [Recipes](docs/book/recipes.md).
- Specification:
  [`docs/specification/specification.md`](docs/specification/specification.md).
- On-disk contract: [content format](docs/topics/0001-content-format.md).
- Architecture decisions: [`docs/adr/`](docs/adr) — see
  [ADR-0001](docs/adr/0001-multi-kind-compiler-architecture.md) for the umbrella
  decision.

## License

MIT

<!-- git-std:bootstrap -->

## Post-clone setup

Run `./bootstrap` after `git clone` or `git worktree add`.
