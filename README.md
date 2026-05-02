# upskill

[![docs](https://img.shields.io/badge/docs-driftsys.github.io/upskill-blue)](https://driftsys.github.io/upskill/)

> Upskill your coding agents.

A single Rust binary for authoring and distributing AI-assistance content
— **rules**, **skills**, and **agents** — across multiple AI coding clients
(Claude Code, GitHub Copilot, opencode) from a single source of truth.

No Node.js. No npm. No runtime dependencies.

```bash
cargo install upskill

upskill add owner/repo               # install everything from a source repo
upskill update                       # pull latest, regenerate per-client files
upskill list                         # see what's installed
```

## Status

- **v0.1.x** — shipped on `main`, `cargo install upskill`. Skills-only
  installer with the original fetch-and-copy model.
- **v0.2** — in progress on `v0.2-redesign`. Adds rules and agents alongside
  skills, with an SSOT-to-client generation pipeline. Phases 0–2 (model,
  parser, generation) have shipped on the branch; Phase 3+ (install / update
  over the pipeline, bundles, lockfile migration) is in flight.

See [`docs/specification.md`](docs/specification.md) for the v0.2 design and
[`docs/adr/`](docs/adr/) for the architecture decisions
([ADR-0001](docs/adr/0001-multi-kind-compiler-architecture.md) is the
umbrella).

## License

MIT

<!-- git-std:bootstrap -->

## Post-clone setup

Run `./bootstrap` after `git clone` or `git worktree add`.
