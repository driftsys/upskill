# AGENTS.md

Instructions for AI coding agents working in this repository.

## Project

upskill is a Rust CLI for authoring and distributing AI-assistance content
(rules, skills, agents) across multiple AI coding clients (Claude Code,
Copilot, opencode) from a single source of truth. The central abstraction is
**generation** — SSOT in, per-client output out.

**v0.2 redesign in progress.** The shipped binary (`v0.1.x`, on `main`) is a
skills-installer with the old fetch-and-copy model. The current branch
(`v0.2-redesign`) is rebuilding around the SSOT-to-client pipeline. See
[ADR-0001](docs/adr/0001-multi-kind-compiler-architecture.md) for the
umbrella decision and the rest of `docs/adr/` for concern-specific design.

**Design priority**: single static binary, ~3 MB target. No Node.js, no npm,
no async runtime, no `git2` (shell out to `git` instead). Other deps are
admitted on a case-by-case basis — see ADR-0001 §3 (dependency philosophy
relaxed for `serde_yaml_ng`, `pulldown-cmark`, `dprint-plugin-markdown`).

## Stack

- **Language**: Rust, edition 2024. **MSRV: 1.85** (first stable to support
  edition 2024).
- **Crate**: `upskill` is both library (`lib.rs`) and binary (`main.rs`).
- **Dependencies**: `clap` 4 (derive), `anyhow` 1, `thiserror` 2, `serde` 1,
  `serde_json` 1, `serde_yaml_ng` 0.10, `pulldown-cmark` 0.11,
  `dprint-plugin-markdown` `=0.21.1` (exact pin — see
  [ADR-0003](docs/adr/0003-generation-pipeline.md)), `sha2` 0.11, `ureq` 2,
  `ctrlc` 3.
- **Out**: `tokio`, `reqwest` (no async, `ureq` is sync), `git2` (~5 MB —
  shell out to `git`), `dialoguer` (raw stdin is enough). Don't add these
  without a strong reason recorded in an ADR.
- **Release profile**: `opt-level = "z"`, `lto`, `codegen-units = 1`,
  `strip`, `panic = "abort"`.

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

After `git clone` or `git worktree add`, run `./bootstrap` once. It installs
`git-std` (from `driftsys/git-std`) into `~/.local/bin` and runs
`git std bootstrap`. Release tagging (`just release`) uses `git std bump`.

## Architecture

Primary crate:

- `upskill` — library/CLI implementation and domain logic in Rust.

### Module layout

```
src/
├── main.rs              CLI entry point, clap derive, command dispatch
├── lib.rs               Module declarations and re-exports
│
├── model/               SSOT data model (Phase 0): Rule, Skill, Agent + common
├── parse/               SSOT parsing (Phase 0): YAML frontmatter
├── generate/            SSOT → per-client rendering (Phase 1–2):
│                        Client enum + claude/copilot/opencode + directives + dprint
│
├── source.rs            Source URL parsing and classification (typed errors)
├── fetch.rs             Git clone, shallow clone, local path resolution
├── auth.rs              Token resolution (env vars, gh/glab CLI fallback)
├── search.rs            skills.sh API search
├── ui.rs                Interactive prompts, TTY detection, colored output
│
├── agent.rs             v0.1 agent detection / symlink targets (retired in Phase 3)
├── install.rs           v0.1 install flow (replaced in Phase 3)
└── lockfile.rs          v0.1 .upskill-lock.json schema (Phase 3 keeps
                         the filename, bumps schema, adds ~/.upskill/installed.json)
```

Core docs (published as mdBook at <https://driftsys.github.io/upskill/>):

- `docs/usage.md` — user-facing guide (entrypoint)
- `docs/specification.md` — v0.2 behavioral spec
- `docs/architecture.md` — implementation guide
- `docs/format-spec.md` — portable on-disk content format
- `docs/adr/` — architecture decision records (0001 umbrella + 0002–0005)

### Key conventions

- **Error handling**: `anyhow::Result<T>` + `.with_context()` everywhere except
  `source.rs`, which uses `thiserror` for typed `SourceParseError`.
- **`main.rs` only does I/O orchestration** — call modules, handle errors, print
  results. Business logic lives in the library modules.
- **Only `main.rs` and `ui.rs` write to stdout/stderr.** Every other module
  returns data structures or `Result<T>`; presentation belongs in `main.rs`.
- **Zero warnings policy** — compiler, clippy, and docs tooling. `-D warnings`
  is enforced in CI.
- **Clippy `too_many_arguments`** — group related flags into structs
  (e.g. `AddContext`) when a function would exceed 7 params.

### Install layout

**v0.1 (shipped on `main`).** Skills install to `.agents/skills/` (project)
or `~/.agents/skills/` (global) with per-agent symlinks. State in
`.upskill-lock.json`. `AGENT_DEFS` in `src/agent.rs` is the source of truth.

**v0.2 (this branch, Phase 3 in flight).** Per-item generated output, copy
only (no symlinks). State split between `.upskill-lock.json` (per-project,
committed, schema-versioned) and `~/.upskill/installed.json` (per-user).
The lockfile filename is unchanged from v0.1; v0.2 bumps a `schema:` field
and rewrites in place on first invocation. Per-client output paths and
ancillary files (`CLAUDE.md`, `.vscode/settings.json`, `opencode.json`) are
specified in
[ADR-0003](docs/adr/0003-generation-pipeline.md) and
[format-spec §7](docs/format-spec.md).

### Source format

`upskill add` accepts:

- `owner/repo` — GitHub shorthand
- `owner/repo@ref` — pinned ref/tag/branch
- `owner/repo:path/to/skill` — subfolder
- `owner/repo@ref:path` — combined
- `https://github.com/owner/repo[...]` — full URL
- `gitlab:owner/repo[...]` or `https://gitlab.com/...` — GitLab
- `./path`, `../path`, `/abs/path`, `~/path` — local paths

### Authentication

Token resolution order:

- GitHub: `GITHUB_TOKEN` → `GH_TOKEN` → `gh auth token` → none.
- GitLab: `GITLAB_TOKEN` → `GL_TOKEN` → `glab auth token` → none.

### Exit codes

| Outcome       | Code |
| ------------- | ---- |
| Success       | 0    |
| General error | 1    |
| Usage error   | 2    |
| SIGINT        | 130  |

### Testing

- **Unit tests** live alongside modules. `source.rs` is pure — test parsing
  exhaustively. Other unit tests cover flag resolution, lockfile read/write,
  hash computation, env-var precedence, fetch with subfolder, etc.
- **Integration tests** live in `tests/` as `cli_*.rs` files and use
  `assert_cmd` + `tempfile`. Pattern:

  ```rust
  Command::cargo_bin("upskill")
      .unwrap()
      .current_dir(&tmp)
      .args(["add", "owner/repo", "--claude"])
      .assert()
      .success();
  ```

- Existing test files:
  - **CLI (v0.1 surface):** `cli_add`, `cli_check`, `cli_ci_mode`,
    `cli_dryrun`, `cli_exit_codes`, `cli_gitlab`, `cli_global`, `cli_list`,
    `cli_lockfile`, `cli_moddetect`, `cli_remove`, `cli_search`, `cli_update`.
  - **Generation (v0.2 pipeline):** `generate_skills`, `generate_rules`,
    `generate_agents`. Golden fixtures in `tests/fixtures/`.
  - When adding behavior, prefer extending the matching file or creating a
    new `cli_<area>.rs` / `generate_<area>.rs`.

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
10. Merge with a squash commit to keep history clean.

Agent-specific rules:

- Start from acceptance criteria first.
- Work by example: start with ATDD integration tests using CLI/snapshot testing,
  then move to TDD with focused unit tests.
- Every branch must be sandboxed in its own git worktree, in
  `.claude/worktrees/<branch>` (already gitignored).
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
