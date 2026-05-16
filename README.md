# upskill

[![docs](https://img.shields.io/badge/docs-driftsys.github.io/upskill-blue)](https://driftsys.github.io/upskill/)

> Upskill your coding agents.

A single Rust binary for authoring and distributing AI-assistance content
— **rules**, **skills**, and **agents** — across multiple AI coding clients
(Claude Code, GitHub Copilot, opencode) from a single source of truth.

No Node.js. No npm. No runtime dependencies.

## Install (consumer)

```bash
# Linux / macOS (Windows: run inside WSL)
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh

# Or, with a Rust toolchain:
cargo install upskill

# Install everything from a source repo (auto-fans out to .claude/, .github/, .agents/)
upskill add driftsys/skills

# Or install a curated bundle — one entry, dependency-resolved
upskill add driftsys/bundles:platform-baseline.bundle.md

# Pull latest and regenerate
upskill update

# Inspect installed state
upskill list
upskill doctor
```

`.upskill-lock.json` records what was installed at which ref and hash —
commit it alongside the project.

## Author

```bash
# In a source-registry repo (where SSOT items live)
upskill new skill code-review
upskill lint --strict
upskill fmt
```

## Two roles, no overlap

- **Source registry** — repo where SSOT items live. Author commands run
  here: `new`, `lint`, `fmt`.
- **Consumer project** — repo where generated outputs land. Consumer
  commands run here: `add`, `remove`, `update`, `list`, `doctor`,
  `search`.

The same repo can be both, but SSOT and generated outputs do not mix in
the same tree.

## Bundles

A bundle is a flat manifest (`*.bundle.md`) that names the items it
includes, and optionally other bundles it depends on. Installing a
bundle resolves the transitive closure once and writes per-item output:

```bash
upskill add driftsys/bundles:platform-baseline.bundle.md
```

The lockfile records the bundle entry alongside the items, so
`remove --source <bundle-label>` and future `remove <bundle-name>`
flows know which items came from which bundle.

## Documentation

- User guide: [Getting started](docs/getting-started.md), [Commands](docs/commands.md), [Recipes](docs/recipes.md).
- Behavioural spec: [`docs/specification.md`](docs/specification.md).
- On-disk contract: [`docs/format-spec.md`](docs/format-spec.md).
- Architecture decisions: [`docs/adr/`](docs/adr/) — see
  [ADR-0001](docs/adr/0001-multi-kind-compiler-architecture.md) for the
  umbrella decision.

## License

MIT

<!-- git-std:bootstrap -->

## Post-clone setup

Run `./bootstrap` after `git clone` or `git worktree add`.
