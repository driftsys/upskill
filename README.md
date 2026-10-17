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

```bash
# Linux / macOS (Windows: run inside WSL)
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh

# Or, with a Rust toolchain:
cargo install upskill

# Install everything from a source repo (auto-fans out to .claude/, .github/, .agents/)
upskill add driftsys/skills

# Or install a curated bundle — one entry, dependency-resolved
upskill add driftsys/bundles:platform-baseline.bundle.yaml

# Pull latest and regenerate
upskill update

# Inspect installed state
upskill list
upskill doctor
```

`.upskill-lock.json` records what was installed at which ref and hash — commit
it alongside the project.

## Author

```bash
# In a source-registry repo (where SSOT items live)
upskill new skill code-review
upskill lint --strict
upskill fmt
```

## Two roles, no overlap

- **Source registry** — repo where SSOT items live. Author commands run here:
  `new`, `lint`, `fmt`.
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
