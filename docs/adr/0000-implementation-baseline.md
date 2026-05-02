# Implementation baseline

**Status**: Accepted (2026-05-02, post-v0.2.0)

## Context

ADR-0000 is the standing engineering reference for upskill: stack
choices, module layout, code conventions, and testing strategy. It is
distinct from the design-decision ADRs (0001–0005) — those record what
to build and why, this records how the codebase is laid out and how to
work in it.

Concern boundaries:

- **ADR-0001** owns the strategic v0.2 redesign and dependency
  philosophy.
- **ADR-0002** owns the on-disk SSOT contract.
- **ADR-0003** owns the SSOT-to-client generation pipeline behaviour.
- **ADR-0004** owns the CLI verb surface.
- **ADR-0005** owns ecosystem-interop deviations.

This ADR does not duplicate those decisions; it points to where they
land in code.

## 1. Stack

### 1.1 Language and edition

Rust 2024 edition. MSRV: 1.85 (first stable release to support edition
2024).

### 1.2 Dependencies

| Crate                    | Version   | Purpose                                  |
| ------------------------ | --------- | ---------------------------------------- |
| `clap`                   | 4.5       | CLI parsing (derive, subcommands).       |
| `anyhow`                 | 1         | Ergonomic error chains via `.context()`. |
| `thiserror`              | 2         | Typed parse errors (`source.rs`).        |
| `serde`                  | 1         | Serialisation framework.                 |
| `serde_json`             | 1         | Lockfile and `installed.json` I/O.       |
| `serde_yaml_ng`          | 0.10      | SSOT YAML frontmatter parsing.           |
| `pulldown-cmark`         | 0.11      | Markdown body parsing for directives.    |
| `dprint-plugin-markdown` | `=0.21.1` | Embedded markdown formatter (exact pin). |
| `sha2`                   | 0.11      | Content hashing (lockfile, drift).       |
| `ureq`                   | 2         | HTTP client (skills.sh API).             |
| `ctrlc`                  | 3.4       | SIGINT handler — exit 130 on Ctrl+C.     |

The version pin on `dprint-plugin-markdown` is deliberate; bumps are
gated on re-running golden-file fixtures so generated output stays
byte-identical. See [ADR-0003](./0003-generation-pipeline.md).

**Deliberately excluded.** `tokio`, `reqwest` (no async needed; `ureq`
is synchronous), `git2` (~5 MB binding — shell out to `git` instead),
`dialoguer` (prompts are simple enough with raw stdin), `toml` (no
config files yet). [ADR-0001](./0001-multi-kind-compiler-architecture.md)
§3 admits the well-targeted parsing/formatting deps above; `git2` and
async runtimes remain out.

### 1.3 Build profile

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

Target: ~3 MB static binary. `panic = "abort"` saves ~200 KB by
removing unwind tables.

### 1.4 Crate layout

`upskill` is both a library (`lib.rs`) and a binary (`main.rs`). The
library exposes every module so integration tests can call them
directly.

## 2. Module structure

```text
src/
├── main.rs              CLI entry point, clap derive, command dispatch.
├── lib.rs               Module declarations and re-exports.
│
├── model/               SSOT data model.
│   ├── mod.rs
│   ├── common.rs        Schema, metadata, passthrough blocks shared by all kinds.
│   ├── rule.rs          Rule item (RULE.md) with scope.paths.
│   ├── skill.rs         Skill item (SKILL.md) — Agent Skills extended fields.
│   ├── agent.rs         Agent item (AGENT.md) with mode/model/tools/preload-skills.
│   └── bundle.rs        Bundle manifest (*.bundle.md) — items + requires.
│
├── parse/               SSOT parsing.
│   ├── mod.rs
│   ├── frontmatter.rs   YAML frontmatter parser (--- delimiters).
│   └── bundle.rs        Bundle file loader + registry discovery.
│
├── generate/            SSOT → per-client output rendering (ADR-0003).
│   ├── mod.rs           Client enum, render_skill / render_rule / render_agent.
│   ├── claude.rs        Claude Code frontmatter mapping.
│   ├── copilot.rs       GitHub Copilot frontmatter mapping.
│   ├── opencode.rs      opencode frontmatter mapping (permission map).
│   ├── directives.rs    <!-- @client:X --> conditional content blocks.
│   └── format.rs        Embedded dprint-plugin-markdown wrapper.
│
├── source.rs            Source URL parsing and classification (typed errors).
├── fetch.rs             Git clone, shallow clone, local path resolution.
├── auth.rs              Token resolution (env vars, gh/glab CLI fallback).
├── search.rs            skills.sh API search, registry URL resolution.
│
├── pipeline.rs          Local + git → per-client install pipeline.
│                        Token-injected clone URLs, SSOT hashing,
│                        list / remove / update / doctor over the lockfile.
├── bundle.rs            Bundle dependency resolution (transitive items closure).
├── ancillary.rs         CLAUDE.md / opencode.json / .vscode/settings.json
│                        first-time hand-shake files.
├── lockfile.rs          .upskill-lock.json (`schema: 1`) read/write.
│
├── lint.rs              Author command — validate SSOT against the format spec.
├── fmt.rs               Author command — canonicalise YAML frontmatter.
└── scaffold.rs          Author command — `upskill new <kind> <name>`.
```

Cross-references:

- The on-disk shape consumed by `model/` and `parse/` is specified in
  [`docs/format-spec.md`](../format-spec.md) and motivated by
  [ADR-0002](./0002-portable-content-format.md).
- The pipeline implemented in `generate/` and `pipeline.rs` follows
  [ADR-0003](./0003-generation-pipeline.md).
- The CLI verbs `main.rs` dispatches to are fixed by
  [ADR-0004](./0004-cli-surface.md).

## 3. Conventions

- **Error handling.** `anyhow::Result<T>` with `.with_context()`
  everywhere except `source.rs`, which uses `thiserror` for a typed
  `SourceParseError` (callers branch on the error variant).
- **`main.rs` is I/O orchestration only.** It calls library modules,
  handles errors, and prints results. Business logic lives in the
  library.
- **Only `main.rs` writes to stdout/stderr.** Every other module
  returns data structures or `Result<T>`; presentation belongs in
  `main.rs`.
- **Zero warnings.** Compiler, clippy, and docs tooling. `-D warnings`
  is enforced in CI.
- **Clippy `too_many_arguments`.** Group related flags into structs
  (e.g. `AddContext`) when a function would exceed seven params.
- **Comments only for the WHY of non-obvious logic.** Don't restate
  what the code does.

## 4. Testing strategy

### 4.1 Unit tests

Live alongside modules. Coverage focuses on pure logic:

- `source.rs` — every format variant and every error.
- `model/` — round-trip serde for each kind, schema-version handling.
- `parse/frontmatter.rs` — delimiter edge cases, missing or malformed
  YAML.
- `parse/bundle.rs` — bundle file load + registry discovery.
- `generate/{claude,copilot,opencode}.rs` — frontmatter mapping per
  kind.
- `generate/directives.rs` — conditional block parsing, negation,
  nesting.
- `generate/format.rs` — dprint idempotence on already-formatted
  input.
- `pipeline.rs` — install / remove / update / doctor / list against
  in-memory lockfiles.
- `lockfile.rs` — schema rejection, upsert/remove ordering.
- `lint.rs`, `fmt.rs`, `scaffold.rs` — per-rule findings, canonical
  round-trip, name validation.
- `auth.rs`, `bundle.rs`, `ancillary.rs` — per-module logic.

### 4.2 Integration tests

Live in `tests/`, one file per concern:

```text
tests/
├── cli_add.rs              add behaviour
├── cli_bundle_install.rs   bundle resolution + install end-to-end
├── cli_ci_mode.rs          TTY detection, no-color, no-prompt
├── cli_doctor.rs           drift report (3 buckets)
├── cli_exit_codes.rs       0 / 1 / 2 / 130
├── cli_fmt.rs              `upskill fmt`
├── cli_lint.rs             `upskill lint`
├── cli_list.rs             `upskill list`
├── cli_new.rs              `upskill new <kind> <name>`
├── cli_remove.rs           `upskill remove`
├── cli_search.rs           skills.sh API
├── cli_update.rs           `upskill update --dry-run`
│
├── parse_bundles.rs        bundle parser end-to-end
├── pipeline_local.rs       install_from_local_path
├── pipeline_lockfile.rs    install + lockfile + ancillary
├── pipeline_source.rs      install_from_source dispatch
│
├── generate_skills.rs      render_skill across all clients
├── generate_rules.rs       render_rule across all clients
├── generate_agents.rs      render_agent across all clients
│
└── fixtures/               golden SSOT files and expected per-client output
```

Pattern: `assert_cmd::Command::cargo_bin("upskill")` +
`tempfile::TempDir`. When adding behaviour, prefer extending the
matching file or creating a new `cli_<area>.rs` /
`pipeline_<area>.rs` / `generate_<kind>.rs`.

## 5. Build and verify

```bash
just assemble    # cargo build
just test        # cargo test
just lint        # cargo clippy + fmt --check + dprint check
just check       # test + lint
just build       # assemble + check
just verify      # commit check + build (run before opening a PR)
just fmt         # cargo fmt + dprint fmt
```

After `git clone` or `git worktree add`, run `./bootstrap` once. It
installs `git-std` from `driftsys/git-std` and runs
`git std bootstrap`. Release tagging (`just release`) uses
`git std bump`.
