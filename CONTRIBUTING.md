# Contributing to upskill

## Prerequisites

| Tool       | Version                    | Install                        |
| ---------- | -------------------------- | ------------------------------ |
| Rust       | ≥ 1.85 (edition 2024 MSRV) | [rustup.rs](https://rustup.rs) |
| git-std    | latest stable              | `./bootstrap`                  |
| prim       | latest stable              | `./bootstrap`                  |
| just       | latest                     | `cargo install just`           |
| dprint     | latest                     | `cargo install dprint`         |
| shellcheck | latest                     | `brew install shellcheck`      |
| mdbook     | latest                     | `cargo install mdbook`         |

## Quick start

```bash
git clone https://github.com/driftsys/upskill.git
cd upskill
./bootstrap
just build
```

## Daily workflow

| Command       | Purpose                            |
| ------------- | ---------------------------------- |
| `just sanity` | Fast pre-push code checks          |
| `just test`   | Run all tests                      |
| `just fmt`    | Format Rust + Markdown             |
| `just lint`   | Clippy + shellcheck + dprint check |
| `just verify` | Full pre-commit check              |
| `just build`  | Compile + check                    |

## Branch model

Normal git branches for humans. The `.claude/worktrees/` model is for AI
agents only — see [AGENTS.md](AGENTS.md) for details.

## Commit convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — new feature
- `fix:` — bug fix
- `refactor:` — code restructuring
- `docs:` — documentation only
- `test:` — test additions/changes
- `chore:` — tooling, CI, dependencies

The `commit-msg` hook enforces this convention with `git std lint`.

The `pre-commit` hook checks staged whitespace with `git diff --cached --check`.
It does not format files or compile code. The `pre-push` hook runs `just sanity`:
Rust formatting, Prim hygiene, and `cargo check --all-targets --locked`. CI runs
the full tests and Clippy checks.

Bootstrap installs or updates `git-std` and `prim` from their latest stable
GitHub releases, verifies archive checksums, and activates the hooks. It requires
network access; commit and push hooks do not download tools.

## PR checklist

1. Run `just fmt && just verify` before opening.
2. One PR per story/task.
3. Keep code + tests + docs together.

## AI agents

See [AGENTS.md](AGENTS.md) for AI-specific conventions, issue model, and
workflow rules.
