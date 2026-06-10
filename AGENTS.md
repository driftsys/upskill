# AGENTS.md

Instructions for AI coding agents working in this repository.

## Project

upskill is a Rust CLI for authoring and distributing AI-assistance content
(rules, skills, agents) across multiple AI coding clients (Claude Code,
Copilot, opencode) from a single source of truth. The central abstraction is
**generation** — SSOT in, per-client output out.

**v0.2.0 shipped.** The redesign around the SSOT-to-client generation
pipeline is complete and released. See
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
just book               # Build the mdBook docs into book/
just book-serve         # Serve the mdBook with live reload (opens in browser)
```

After `git clone` or `git worktree add`, run `./bootstrap` once. It installs
`git-std` (from `driftsys/git-std`) into `~/.local/bin` and runs
`git std bootstrap`. Release tagging (`just release`) uses `git std bump`.

## Architecture

Primary crate:

- `upskill` — library/CLI implementation and domain logic in Rust.

### Module layout

```text
src/
├── main.rs              CLI entry point, clap derive, command dispatch
├── lib.rs               Module declarations and re-exports
│
├── model/               SSOT data model: Rule, Skill, Agent, Bundle + common
├── parse/               SSOT parsing: YAML frontmatter + bundle loader/discovery
├── generate/            SSOT → per-client rendering:
│                        Client enum + claude/copilot/opencode + directives + dprint
│
├── source.rs            Source URL parsing and classification (typed errors)
├── fetch.rs             Git clone, shallow clone, local path resolution
├── search.rs            skills.sh API search
│
├── pipeline.rs          Local + git → per-client install pipeline,
│                        git-config clone URLs, SSOT hashing,
│                        list / remove / update / doctor over the lockfile
├── bundle.rs            Bundle dependency resolution
├── lint.rs              Author command — validate SSOT against the format spec
├── fmt.rs               Author command — canonicalise YAML frontmatter
├── scaffold.rs          Author command — `upskill new <kind> <name>`
├── ancillary.rs         CLAUDE.md / opencode.json / .vscode/settings.json
│                        first-time hand-shake files
└── lockfile.rs          .upskill-lock.json (`schema: 1`) read/write
```

Core docs (published as mdBook at <https://driftsys.github.io/upskill/>):

- `docs/intro.md` — book entrypoint
- `docs/getting-started.md` / `docs/commands.md` / `docs/recipes.md` — user guide
- `docs/conventions.md` — upskill conventions, including the recommended `skills/` layout
- `docs/specification.md` — upskill specification
- `docs/format-spec.md` — portable on-disk content format
- `docs/adr/` — architecture decision records (0000 baseline, 0001 umbrella, 0002–0005)

### Key conventions

- **Error handling**: `anyhow::Result<T>` + `.with_context()` everywhere except
  `source.rs`, which uses `thiserror` for typed `SourceParseError`.
- **`main.rs` only does I/O orchestration** — call modules, handle errors, print
  results. Business logic lives in the library modules.
- **Only `main.rs` writes to stdout/stderr.** Every other module returns
  data structures or `Result<T>`; presentation belongs in `main.rs`.
- **Zero warnings policy** — compiler, clippy, and docs tooling. `-D warnings`
  is enforced in CI.
- **Clippy `too_many_arguments`** — group related flags into structs
  (e.g. `AddContext`) when a function would exceed 7 params.

### Install layout

Per-item generated output, copy only (no symlinks). One lockfile
shape (`.upskill-lock.json`, `schema: 1`) in two possible locations:
`<cwd>/` (project scope, committed) or `$HOME/` (global scope, not
committed). Per-client output paths and ancillary files (`CLAUDE.md`,
`.vscode/settings.json`, `opencode.json`) are specified in
[ADR-0003](docs/adr/0003-generation-pipeline.md) and
[format-spec §7](docs/format-spec.md).

### Source format

`upskill add` accepts:

- `owner/repo` — GitHub shorthand
- `owner/repo@ref` — pinned ref/tag/branch
- `owner/repo:path/to/skill` — subfolder
- `owner/repo@ref:path` — combined
- `https://github.com/owner/repo[...]` — full URL
- `https://<host>/<path>[...]` — any other https git host (GitLab including
  self-hosted and subgroups, Bitbucket, Gitea, …)
- `./path`, `../path`, `/abs/path`, `~/path` — local paths

### Authentication

upskill never injects credentials into clone URLs. Clones use the bare
`https://<host>/...` URL and rely entirely on git's own configuration —
credential helpers (keychain, manager), `url.<base>.insteadOf` rewrites,
and SSH. There are no token env vars or `gh` / `glab` CLI fallbacks;
configure git the way you would for any manual `git clone`.

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
  `assert_cmd` + `tempfile`. Construct the binary through
  `common::upskill_cmd(&fake_home)` — never a raw `Command::cargo_bin("upskill")`.
  The harness points `HOME`/`USERPROFILE` at a tempdir so a stray global-scope
  write can't land in the developer's real `$HOME` (`add` defaults to global
  outside a git repo). `tests/cli_test_harness_guard.rs` enforces this; a file
  that must control `HOME` itself opts out with an
  `upskill-allow-raw-cargo-bin: <reason>` comment. Pattern:

  ```rust
  mod common;

  let tmp = tempfile::tempdir().unwrap();
  let home = tmp.path().join("home");
  std::fs::create_dir_all(&home).unwrap();
  common::upskill_cmd(&home)
      .current_dir(tmp.path())
      .args(["add", "owner/repo", "--claude"])
      .assert()
      .success();
  ```

- Existing test files:
  - **CLI:** `cli_add`, `cli_ci_mode`, `cli_exit_codes`, `cli_search`.
  - **Pipeline:** `pipeline_local`, `pipeline_source`, `pipeline_lockfile`.
  - **Generation (v0.2 pipeline):** `generate_skills`, `generate_rules`,
    `generate_agents`. Golden fixtures in `tests/fixtures/`.
  - **Harness:** `tests/common/mod.rs` (`upskill_cmd`) and
    `cli_test_harness_guard` (enforces the fake-`$HOME` convention).
  - When adding behavior, prefer extending the matching file or creating
    a new `cli_<area>.rs` / `pipeline_<area>.rs` / `generate_<area>.rs`.

### Available skills

Skills provide specialized instructions and workflows for specific tasks.
Use the skill tool to load a skill when a task matches its description.

<available_skills>
<skill>
<name>upskill-evaluating-prompts</name>
<description>Use when setting up the RED-GREEN-REFACTOR cycle for a rule, skill, or subagent — i.e., when the `writing-*` skills tell you to "run a failing test first." Trigger when authoring meta-skills, when validating that an existing rule/skill/subagent actually shapes behavior, or when auditing whether a skill activates correctly. Do NOT trigger for content authoring itself — see upskill-writing-rules, writing-skills, upskill-writing-subagents.</description>
<location>skills/upskill-evaluating-prompts/SKILL.md</location>
</skill>
<skill>
<name>upskill-prompt-design</name>
<description>Use as the entry point to the upskill framework's prompt-engineering discipline. Trigger when someone is new to the framework, when an author is not sure which meta-skill to activate, when reviewing how a team is using rules/skills/subagents, or when cross-cutting concerns (portability across clients, classification, token economics, composition patterns) come up. Do NOT trigger for general one-shot prompt-writing guidance — that is an onboarding concern outside the framework. Do NOT trigger for specific authoring tasks — hand off to upskill-prompt-distilling, upskill-writing-rules, writing-skills, upskill-writing-subagents, or upskill-cli.</description>
<location>skills/upskill-prompt-design/SKILL.md</location>
</skill>
<skill>
<name>upskill-prompt-distilling</name>
<description>Use BEFORE authoring any rule, skill, or subagent. Trigger when someone says "we should encode X", "the agent keeps forgetting Y", "let's add a rule for Z", or any variant where new behavior needs to live somewhere in the framework. Also trigger when reviewing existing content that is misbehaving — wrong-layer placement is a common silent root cause. Do NOT trigger for refining content already correctly placed; hand off to upskill-writing-rules, writing-skills, or upskill-writing-subagents.</description>
<location>skills/upskill-prompt-distilling/SKILL.md</location>
</skill>
<skill>
<name>upskill-cli</name>
<description>Use when working in an upskill-consumer repo and needing to add, modify, vendor, audit, or remove installed content (rules, skills, agents, bundles). Trigger when `upskill lint` or `upskill doctor` reports issues. Trigger when adding a third-party bundle or updating one from upstream. Trigger when generated per-client files (e.g., `.claude/`, `.github/skills/`) look stale or wrong. Do NOT trigger for actual authoring decisions — hand off to upskill-prompt-distilling, upskill-writing-rules, writing-skills, or upskill-writing-subagents. Do NOT trigger for cosmetic typo fixes in skill bodies; raw edits to source registry files plus `upskill lint` are sufficient.</description>
<location>skills/upskill-cli/SKILL.md</location>
</skill>
<skill>
<name>upskill-writing-rules</name>
<description>Use when adding or editing rules in CLAUDE.md, AGENTS.md, or any always-loaded instructions file. Trigger when authoring repo conventions, invariants, or behavioral guardrails. Also trigger when an existing rule is being violated despite being present — that means the rule needs refactoring, not the agent. Do NOT trigger for skill or subagent authoring; see writing-skills and upskill-writing-subagents.</description>
<location>skills/upskill-writing-rules/SKILL.md</location>
</skill>
<skill>
<name>upskill-writing-bundles</name>
<description>Use when authoring or editing upskill .bundle.yaml manifests — declaring items, plugins, requires dependencies, naming conventions, or troubleshooting bundle resolution errors. Also use when adding plugin install declarations for client CLIs (Claude Code, Copilot, VS Code, opencode).</description>
<location>skills/upskill-writing-bundles/SKILL.md</location>
</skill>
<skill>
<name>upskill-writing-subagents</name>
<description>Use when designing a new subagent or modifying an existing one's system prompt, tool surface, or return contract. Trigger when the parent is observed doing work inline that should be delegated, OR when a subagent is observed dumping raw output, OR when subagent scope is drifting. Also trigger when designing managed/project-scoped/user-global subagent tiers with different permission models. Do NOT trigger for skill or rule authoring; see writing-skills and upskill-writing-rules.</description>
<location>skills/upskill-writing-subagents/SKILL.md</location>
</skill>
</available_skills>

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
