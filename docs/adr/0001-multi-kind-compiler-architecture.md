# Multi-kind compiler architecture — v0.2 redesign umbrella

**Status**: Proposed (2026-05-01)

## Context

upskill v0.1 is a Rust CLI that installs Agent Skills (`SKILL.md` packages)
from git source repos for use across multiple AI coding clients. Its
surface is `add`, `list`, `remove`, `check`, `search`, `update`. Its fetch
model is `owner/repo[@ref][:subfolder]`. Its state is a per-project
lockfile.

The new requirement is broader: help authors **create, manage, and
prolong** AI-assistance content of three kinds — rules, skills, agents —
across three clients (Claude Code, Copilot, opencode). The expansion is
not additive: the new central abstraction is **generation** (SSOT →
per-client output), and v0.1's `install.rs` (fetch-and-copy) does not
have a place in that pipeline.

This ADR is the **umbrella** for the v0.2 redesign. The substantive
design is split across four concern-focused child ADRs:

- [ADR-0002](./0002-portable-content-format.md) — Portable content format
  (the on-disk SSOT contract).
- [ADR-0003](./0003-generation-pipeline.md) — Generation pipeline (SSOT
  → per-client output mechanics, including the dprint embedding decision
  that originated as Phase 0's spike).
- [ADR-0004](./0004-cli-surface.md) — CLI surface (the user-facing
  contract).
- [ADR-0005](./0005-skills-sh-ecosystem-interop.md) — Compatibility with the
  Vercel/skills.sh ecosystem (cross-cutting strategic alignment).

This ADR records the project-level decisions only: the pivot itself, the
dependency philosophy shift, the build-order plan. Read the children for
"what exactly we're doing"; read this for "why we're doing it and how
the work is sequenced."

## Decision

### 1. Tag and branch

Tag current `main` as `v0.1.0` and rebuild for v0.2 on a `v0.2-redesign`
branch. v0.1 stays supported via patch releases for the
skills-installer use case.

### 2. Redesign as a multi-kind portable-format compiler

The substantive design — content format, generation pipeline, CLI
surface, ecosystem alignment — is delegated to ADR-0002 through ADR-0005.
At the project level, the decision is to redesign rather than extend:
v0.1's mental model (fetch-and-copy of `SKILL.md`) does not fit rules
and agents at all, so any extension would have required this redesign
anyway.

### 3. Dependency philosophy relaxed

[AGENTS.md](../../AGENTS.md) previously listed `tokio`, `reqwest`,
`git2`, `walkdir`, `dialoguer`, `serde_yaml`, `toml` as deliberately not
used. v0.2 admits `serde_yaml_ng`, `walkdir`, `pulldown-cmark`,
`dprint-plugin-markdown`, and similar. `git2` stays out (shell out to
`git`). The "no runtime deps, size-optimized" framing in AGENTS.md is
revised to "minimal but reasonable — prefer one focused dep over
hand-rolled equivalents."

## Consequences

**Positive.** Single tool covers the full lifecycle of rules / skills /
agents across all three clients. Migration story for existing v0.1 users
is explicit (translator from per-project lockfile to global state file
per [ADR-0003](./0003-generation-pipeline.md)).

**Negative.** ~7 engineer-weeks. Binary size grows. Small migration for
v0.1 users. README and architecture docs need rewrite. The redesign
introduces concrete trade-offs documented per concern in the child ADRs.

## Alternatives considered

**(a) Layer rules and agents onto v0.1's existing model without a
redesign.** Rejected: the generation pipeline is fundamentally different
from fetch-and-copy; grafting onto `install.rs` produces churn without
clarity.

**(b) Keep size-optimized minimalism strict, hand-roll YAML / markdown
traversal.** Rejected: high cost, low benefit. The deps in question are
well-maintained and individually small.

**(c) Defer the redesign and continue extending v0.1.** Rejected: v0.1's
fetch-and-copy mental model does not accommodate rules or agents.
Extension would have required this redesign anyway, just later and with
more sunk cost.

## Build order

- **Phase 0** (1w) — tag, branch, deps, this ADR, dprint spike, model
  skeleton. **Done.**
- **Phase 1** (1.5w) — SSOT parser + generation pipeline for skills × all
  three clients. **Done.**
- **Phase 2** (1w) — extend pipeline to rules and agents.
- **Phase 3** (~2.5w) — install / update / remove on top of pipeline;
  bundle support; v0.1 lockfile migration; ancillary file generation
  (CLAUDE.md, opencode.json, .vscode/settings.json). Original estimate
  was 1.5w; bundles promoted from "deferred" to MVP per
  [ADR-0002](./0002-portable-content-format.md), and ancillary file
  handling per [ADR-0003](./0003-generation-pipeline.md) added scope.
- **Phase 4** — _deleted._ `sync` absorbed into `update` per
  [ADR-0004](./0004-cli-surface.md).
- **Phase 5** (1w) — `lint` + `fmt`.
- **Phase 6** (0.5w) — scaffold / `new`.
- **Phase 7** (~1w) — polish + release of v0.2.0.

## Open questions

Project-level only. Concern-specific opens live in the child ADRs.

- **Multi-host auth growth (GitLab + GitHub).** Phase 3 — when
  `add`/`update` need to authenticate against multiple host families.
- **v0.2.0 release timing relative to internal rollout phases.** Phase 7
  — coordinated with documentation cutover and v0.1 patch-release
  sunset.
- **Windows support level for v0.2.0.** Stretch goal; not blocking MVP.
  Copy-only installation per [ADR-0003](./0003-generation-pipeline.md)
  removes the main symlink obstacle, but full Windows CI verification
  is deferred.
