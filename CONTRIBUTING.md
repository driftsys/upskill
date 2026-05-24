# Contributing to upskill

## Prerequisites

| Tool       | Version                    | Install                        |
| ---------- | -------------------------- | ------------------------------ |
| Rust       | ≥ 1.85 (edition 2024 MSRV) | [rustup.rs](https://rustup.rs) |
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

Not enforced by hook — just documented.

## PR checklist

1. Run `just fmt && just verify` before opening.
2. One PR per story/task.
3. Keep code + tests + docs together.

## AI agents

See [AGENTS.md](AGENTS.md) for AI-specific conventions, issue model, and
workflow rules.
