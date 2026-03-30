# AGENTS.md

Instructions for AI coding agents working in this repository.

## Project

upskill is a Rust CLI project for managing Agent Skills packages across coding
agents. It focuses on a lightweight binary workflow and shared repository
conventions used across driftsys projects.

## Build commands

```bash
cargo test <test_name>  # Run a single test
just assemble           # Compile
just test               # Run all tests
just lint               # Lint + format check
just check              # Run all checks (test + lint)
just build              # Assemble + check
just verify             # Commit check + build — run before PR
just fmt                # Format Rust + Markdown
```

## Architecture

Primary crate:

- `upskill` — library/CLI implementation and domain logic in Rust.

Core docs:

- `docs/specification.md`
- `docs/architecture.md`

## Workflow

Workflow model:

```text
Story/Task -> ATDD -> TDD -> Implement -> Update SPEC/USAGE -> PR -> Review -> Merge
```

1. Start from acceptance criteria. Read the issue and write acceptance tests
   first.
2. Work by example: start with ATDD integration tests using CLI/snapshot
   testing, then move to TDD with focused unit tests.
3. Update specification and usage docs with implementation changes.
4. One PR per story/task with code, tests, and docs together.
5. Use Conventional Commits (`feat`, `fix`, `refactor`, `docs`, `test`,
   `chore`).
6. Before opening a PR, run `just fmt` then `just verify`.
7. After opening a PR, fix CI issues first, then respond to review comments.
8. Fix critical findings immediately.
9. Track non-critical follow-up work as debt in a story.
10. Merge with a merge commit (not squash) to preserve PR linkage in history.

Agent-specific rules:

- Start from acceptance criteria first.
- Work by example: start with ATDD integration tests using CLI/snapshot testing,
  then move to TDD with focused unit tests.
- Every branch must be sandboxed in its own git worktree.
- Keep code, tests, and docs in the same PR.
- Use Conventional Commits (`feat`, `fix`, `refactor`, `docs`, `test`, `chore`).
- Before opening a PR, run `just fmt` then `just verify`.
- After opening a PR, fix CI issues first, then respond to review comments on the
  PR.
- Fix critical findings immediately.
- Track non-critical follow-up work as debt in a story.

## Issue Model

Issue hierarchy:

```text
Initiative (label only - initiative:<name>)
  -> Epic (issue + epic + epic:<name> labels)
         -> Story  (user-facing requirement)
         -> Task   (technical requirement)
         -> Debt   (refactoring/review findings)
```

Issue types and labels:

- Epic: `epic`
- Story: `story`
- Task: `task`
- Debt: `debt`
- Bug: `bug`

Severity:

- `K0`: Must-have
- `K1`: Should-fix
- `K2`: Nice-to-have

Effort:

- `XS`: Trivial
- `S`: Small
- `M`: Medium
- `L`: Large
- `XL`: Extra large

Priority matrix:

```text
          XS   S    M    L     XL
K0     P0   P0   P0   P1    P1
K1     P0   P1   P1   P2    drop
K2     P1   P2   P2   drop  drop
```

Issue rules:

1. Every story/task/debt starts with `Epic:` as the first non-blank body line
   (`Epic: #N` or `Epic: org/repo#N`).
2. Use one `epic:<name>` label plus one issue-type label.
3. When creating a child issue, update the parent epic task list.
4. Epics are created by humans; agents create stories, tasks, and debt.

Review findings policy:

- `K0`: fix in the PR immediately (or open a bug issue if blocked).
- `K1` / `K2`: open a debt issue with severity, effort, and priority labels.

Reference process: [fast-track](https://github.com/driftsys/fast-track)

## Conventions

- Zero warnings policy for compiler, clippy, and docs tooling.
- Use `cargo fmt` and `clippy`; prefer `just fmt` before committing.
- Keep modules focused; avoid generic helper buckets.
- Prefer typed errors and clear user-facing messages.
- Add comments only where logic is non-obvious.

<!-- git-std:bootstrap -->

## Post-clone setup

Run `./bootstrap` after `git clone` or `git worktree add`.
