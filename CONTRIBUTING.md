# Contributing to upskill

## Prerequisites

| Tool       | Version                    | Install                                         |
| ---------- | -------------------------- | ----------------------------------------------- |
| Rust       | ≥ 1.85 (edition 2024 MSRV) | [rustup.rs](https://rustup.rs)                  |
| git-std    | latest stable              | `./bootstrap`                                   |
| prim       | latest stable              | `./bootstrap`                                   |
| just       | latest                     | `cargo install just`                            |
| shellcheck | latest                     | `brew install shellcheck`                       |
| Python     | ≥ 3.9                      | System package manager                          |
| mdbook     | 0.5.4                      | `cargo install mdbook --version 0.5.4 --locked` |

## Quick start

```bash
git clone https://github.com/driftsys/upskill.git
cd upskill
./bootstrap
just build
```

## Daily workflow

| Command       | Purpose                                                                    |
| ------------- | -------------------------------------------------------------------------- |
| `just sanity` | Fast pre-push code checks                                                  |
| `just test`   | Run all tests                                                              |
| `just fmt`    | Format Rust + Markdown                                                     |
| `just lint`   | Clippy + Rust format check + Prim lint + ShellCheck + offline Python tests |
| `just verify` | Full test, lint, docs, and build checks                                    |
| `just build`  | Compile + check                                                            |

## Branch model

Normal git branches for humans. The `.claude/worktrees/` model is for AI agents
only — see [AGENTS.md](AGENTS.md) for details.

## Commit convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — new feature
- `fix:` — bug fix
- `refactor:` — code restructuring
- `docs:` — documentation only
- `test:` — test additions/changes
- `chore:` — tooling, CI, dependencies

The `commit-msg` hook enforces this convention with `git std lint`.

The `pre-commit` hook formats staged files with Prim and runs `rustfmt` on each
staged Rust file. Golden files under `tests/fixtures/` are excluded from
formatting. It does not run lint, tests, or builds. The `pre-push` hook runs
`just sanity`: Prim lint on the upstream diff, Clippy, and
`cargo check --all-targets --locked`. Full tests, repository lint, docs checks,
and builds run through `just verify` and CI.

Bootstrap installs or updates `git-std` and `prim` from their latest stable
GitHub releases, verifies archive checksums, and activates the hooks. It
requires network access; commit and push hooks do not download tools.

## PR checklist

1. Run `just fmt && just verify` before opening.
2. One PR per story/task.
3. Keep code + tests + docs together.

## AI agents

See [AGENTS.md](AGENTS.md) for AI-specific conventions, issue model, and
workflow rules.

## Documentation lifecycle

Use the [documentation map](docs/README.md) to select the owning record. CLI
requirements belong in the tool specification. Other contracts and implemented
architecture belong in `docs/topics/`, with separate Requirements and
Architecture sections. Use the existing eight concerns before adding a new one.
Four-digit prefixes establish a reading sequence; keep assigned filenames stable
rather than renumbering them when adding a topic. Accepted decisions and
rationale remain in ADRs. User instructions belong in `docs/book/`.

Technical notes use descriptive filenames without date prefixes. Record research
or validation dates, checked revisions, sources and qualification limits inside
the note. Keep immutable evidence assets, schema snapshots, licenses and
manifests together; evidence snapshot directories may retain capture dates. The
staged formatter also preserves the captured schema pack: Prim stdin formatting
does not apply the whole-tree exclusions. Index every note in
`docs/technotes/README.md`. New proposals and implementation plans belong in
`docs/wip/specs/` and `docs/wip/plans/`.

When a feature is complete, reconcile its working documents with code and tests,
update the durable records and book, then archive its raw spec and plan with
delivery evidence and successor links. Keep unfinished scope in WIP. The initial
migration preserves historical statuses; it does not ratify Draft specifications
or Proposed ADRs.

`just docs-check` checks documentation links and builds the book in a temporary
directory. It checks rendered local links, missing chapters/includes and
publication isolation. The [compatibility inventory](docs/compatibility.json)
also pins historical repository anchors and captured asset hashes, including
licenses and provenance manifests. It runs in `just verify` and PR CI. Python
3.9 or newer and mdBook 0.5.4 are development tools only. The local and CI
version is pinned because mdBook 0.5 changed the edit-link path template.
External URLs and historical WIP/archive body references are outside the link
check; their indexes are checked.

The book's reference chapters publish canonical concern and specification
sources. Keep published chapter filenames and anchors stable, and check both
repository links and the rendered book when changing includes. Preserve schema
snapshot bytes and their provenance checksums when moving research.
