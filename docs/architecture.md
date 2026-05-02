# upskill — Architecture

> Implementation guide for the upskill CLI.

**Status**: v0.2 redesign in progress. The v0.1 modules remain in tree
(serving the shipped binary) and are listed alongside the v0.2 SSOT-pipeline
modules added in Phase 0–2.

## 1. Stack

### 1.1 Language and edition

Rust 2024 edition. MSRV: 1.85 (first stable release to support edition 2024).

### 1.2 Dependencies

| Crate                    | Version   | Purpose                                  |
| ------------------------ | --------- | ---------------------------------------- |
| `clap`                   | 4.5       | CLI parsing (derive, subcommands).       |
| `anyhow`                 | 1         | Ergonomic error chains via `.context()`. |
| `thiserror`              | 2         | Typed parse errors (`source.rs`).        |
| `serde`                  | 1         | Serialization framework.                 |
| `serde_json`             | 1         | Lockfile and `installed.json` I/O.       |
| `serde_yaml_ng`          | 0.10      | SSOT YAML frontmatter parsing.           |
| `pulldown-cmark`         | 0.11      | Markdown body parsing for directives.    |
| `dprint-plugin-markdown` | `=0.21.1` | Embedded markdown formatter (exact pin). |
| `sha2`                   | 0.11      | Content hashing (lockfile, drift).       |
| `ureq`                   | 2         | HTTP client (skills.sh API).             |
| `ctrlc`                  | 3.4       | SIGINT handler — exit 130 on Ctrl+C.     |

**Why exact-pin `dprint-plugin-markdown`.** Per
[ADR-0003](./adr/0003-generation-pipeline.md), bumps are deliberate and gated
on re-running golden-file fixtures. `dprint.json`'s WASM plugin pin is aligned
with the embedded crate version so `just lint` matches CI.

**Deliberately excluded.** `tokio`, `reqwest` (no async needed; `ureq` is
synchronous), `git2` (~5 MB binding — shell out to `git` instead), `dialoguer`
(prompts are simple enough with raw stdin), `toml` (no config files yet).
ADR-0001 §3 relaxed the older "no large deps" stance for the items above
(notably `serde_yaml_ng`, `pulldown-cmark`, `dprint-plugin-markdown`,
`walkdir` if needed); `git2` and async runtimes remain out.

### 1.3 Build profile

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

Target: ~3 MB static binary. `panic = "abort"` saves ~200 KB by removing
unwind tables.

### 1.4 Crate layout

`upskill` is both a library (`lib.rs`) and a binary (`main.rs`). The library
exposes all modules for integration testing.

## 2. Module structure

```
src/
├── main.rs              CLI entry point, clap derive, command dispatch.
├── lib.rs               Module declarations and re-exports.
│
├── model/               SSOT data model (Phase 0).
│   ├── mod.rs
│   ├── common.rs        Schema, metadata, passthrough blocks shared by all kinds.
│   ├── rule.rs          Rule item (RULE.md) with scope.paths.
│   ├── skill.rs         Skill item (SKILL.md) — Agent Skills extended fields.
│   └── agent.rs         Agent item (AGENT.md) with mode/model/tools/preload-skills.
│
├── parse/               SSOT parsing (Phase 0).
│   ├── mod.rs
│   └── frontmatter.rs   YAML frontmatter parser (--- delimiters).
│
├── generate/            SSOT → per-client output rendering (Phase 1–2).
│   ├── mod.rs           Client enum, render_skill / render_rule / render_agent.
│   ├── claude.rs        Claude Code frontmatter mapping.
│   ├── copilot.rs       Copilot frontmatter mapping (instructions/skills/agents).
│   ├── opencode.rs      opencode frontmatter mapping (permission map).
│   ├── directives.rs    <!-- @client:X --> conditional content blocks.
│   └── format.rs        Embedded dprint-plugin-markdown wrapper.
│
├── source.rs            Source URL parsing and classification (typed errors).
├── fetch.rs             Git clone, shallow clone, local path resolution.
├── agent.rs             Agent detection, AGENT_DEFS (v0.1 — to be retired in Phase 3).
├── install.rs           v0.1 install flow (canonical target, persist, skill selection).
├── lockfile.rs          v0.1 .upskill-lock.json read/write, content hash.
├── search.rs            skills.sh API search, registry URL resolution.
├── auth.rs              Token resolution (env vars, gh/glab CLI fallback).
└── ui.rs                Interactive prompts, TTY detection, colored output.
```

### 2.1 v0.2 module status

| Module      | Status                                                                                                                                              |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `model/`    | Done (Phase 0). Full schema for rules, skills, agents.                                                                                              |
| `parse/`    | Done (Phase 0). YAML frontmatter parsing with `serde_yaml_ng`.                                                                                      |
| `generate/` | Done (Phase 1–2). All three kinds × all three clients.                                                                                              |
| `source`    | Reused from v0.1. Source format unchanged (parity with `npx skills`).                                                                               |
| `fetch`     | Reused from v0.1. Git clone via shell-out.                                                                                                          |
| `auth`      | Reused from v0.1. Token resolution unchanged.                                                                                                       |
| `install`   | v0.1 logic. Phase 3 replaces with SSOT-pipeline install over `generate/`.                                                                           |
| `lockfile`  | v0.1 schema. Phase 3 replaces with `.upskill.lock` (per-project) and `~/.upskill/installed.json` (per-user); v0.1 lockfile read once for migration. |
| `agent`     | v0.1 symlink/copy targets. Phase 3 retires in favour of per-client output paths from the format spec.                                               |
| `search`    | v0.1 skills.sh API. Carries forward unchanged.                                                                                                      |

## 3. Generation pipeline internals

Per [ADR-0003](./adr/0003-generation-pipeline.md). Pure rendering: SSOT model
→ per-client markdown string. No filesystem writes — those land in Phase 3.

```text
SSOT file (RULE.md / SKILL.md / AGENT.md)
  │
  ▼
parse::frontmatter::extract  (split --- frontmatter from body)
  │
  ▼
serde_yaml_ng → model::{Rule, Skill, Agent}
  │
  ▼
generate::render_{rule,skill,agent}(item, body, Client)
  │
  ├── directives::process(body, client)   process @client conditionals
  ├── {claude,copilot,opencode}::*_frontmatter(item)   build YAML
  └── format::format_markdown(combined)   dprint formatter
       │
       ▼
    String (idempotent under dprint)
```

### 3.1 The `Client` enum

`generate::Client` has three variants — `Claude`, `Copilot`, `OpenCode`. Each
client owns one module (`claude.rs`, `copilot.rs`, `opencode.rs`) that
implements `*_frontmatter(item)` for each kind. The dispatch lives in
`generate::render_*`.

### 3.2 Frontmatter mapping rules

Per format-spec Appendix B and ADR-0003 §"Per-client output paths":

- **Claude.** `name`, `description`, plus extended Agent Skills fields. Rules
  emit `paths:` array from `scope.paths`. Agents emit `model`, `tools`
  (capitalized), `skills:` (renamed from `preload-skills`).
- **Copilot.** `name`, `description`. Rules emit `applyTo:` (comma-joined
  from `scope.paths`, default `"**"`). Agents emit `tools` filtered by the
  documented §4 capability mapping (unmapped tools dropped, not
  passed through). `copilot.*` passthrough block merged when present.
- **opencode.** `name`, `description`. Rules drop scope (no per-rule scoping
  in opencode). Agents emit `mode` (default `subagent`), `model`,
  `permission` (capability → allow map; `write` rolls into `edit`). `name` is
  in the filename per Appendix B and not emitted in agent frontmatter.

### 3.3 Idempotence (dprint embedding)

`format::format_markdown` calls
`dprint-plugin-markdown::format_text(combined, &cfg, |_, _, _| Ok(None))`.
Notes:

- Returns `Ok(None)` when input was already canonical — must be unwrapped to
  the original string, not panicked on.
- Default `text_wrap` is `Maintain`; we leave it that way to avoid surprising
  body re-wrapping.
- Code-block callback returns `Ok(None)` to leave fenced blocks untouched.

## 4. Key conventions

- **Error handling.** `anyhow::Result<T>` with `.with_context()` everywhere
  except `source.rs`, which uses `thiserror` for typed `SourceParseError`
  (callers branch on the error variant).
- **`main.rs` only does I/O orchestration.** Call modules, handle errors,
  print results. Business logic lives in library modules.
- **Only `main.rs` and `ui.rs` write to stdout/stderr.** Every other module
  returns data structures or `Result<T>`.
- **Zero warnings.** Compiler, clippy, docs tooling. `-D warnings` enforced
  in CI.
- **Clippy `too_many_arguments`.** Group related flags into structs (e.g.
  `AddContext`) when a function would exceed 7 params.
- **No comments explaining what code does.** Comments only for the WHY of
  non-obvious logic.

## 5. Testing strategy

### 5.1 Unit tests

Live alongside modules. Coverage focuses on pure logic:

- `source.rs` — parsing exhaustively (every format variant, every error).
- `model/` — round-trip serde for each item kind, schema-version handling.
- `parse/frontmatter.rs` — delimiter edge cases, missing/malformed YAML.
- `generate/{claude,copilot,opencode}.rs` — frontmatter mapping per kind.
- `generate/directives.rs` — conditional block parsing, negation, nesting.
- `generate/format.rs` — dprint idempotence on already-formatted input.
- `lockfile.rs`, `auth.rs` — flag resolution, env-var precedence, etc.

### 5.2 Integration tests

Live in `tests/` as one file per concern:

```
tests/
├── cli_add.rs           v0.1 add behaviour
├── cli_check.rs         v0.1 check
├── cli_ci_mode.rs       TTY detection, no-color, no-prompt
├── cli_dryrun.rs        --dry-run for update
├── cli_exit_codes.rs    0 / 1 / 2 / 130
├── cli_gitlab.rs        GitLab fetch and auth
├── cli_global.rs        --global scope
├── cli_list.rs
├── cli_lockfile.rs      v0.1 lockfile read/write
├── cli_moddetect.rs     content-hash drift detection
├── cli_remove.rs
├── cli_search.rs        skills.sh API
├── cli_update.rs
│
├── generate_skills.rs   render_skill across all clients
├── generate_rules.rs    render_rule across all clients
├── generate_agents.rs   render_agent across all clients
│
└── fixtures/            golden SSOT files and expected per-client output
```

Pattern: `assert_cmd::Command::cargo_bin("upskill")` + `tempfile::TempDir`.

When adding behaviour, prefer extending the matching file or creating a new
`cli_<area>.rs` / `generate_<kind>.rs`.

## 6. Build and verify

```bash
just assemble    # cargo build
just test        # cargo test
just lint        # cargo clippy + fmt --check + dprint check
just check       # test + lint
just build       # assemble + check
just verify      # commit check + build (run before opening a PR)
just fmt         # cargo fmt + dprint fmt
```

After `git clone` or `git worktree add`, run `./bootstrap` once. It installs
`git-std` from `driftsys/git-std` and runs `git std bootstrap`. Release
tagging (`just release`) uses `git std bump`.

## 7. References

- Behavioural spec: [`docs/specification.md`](./specification.md)
- Format contract: [`docs/format-spec.md`](./format-spec.md)
- Architecture decisions: [`docs/adr/`](./adr/README.md)
