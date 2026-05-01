# SSOT-to-client generation pipeline

**Status**: Proposed (2026-05-01)

## Context

[ADR-0002](./0002-portable-content-format.md) defines the portable on-disk
format. This ADR addresses how SSOT becomes per-client output: paths,
frontmatter mapping, ancillary file handling, and the formatting guarantee
required by the spec.

Generation must be cheap (so it can run on every install/update),
deterministic (idempotent across re-runs), and produce output that passes
markdown linting without manual touch-up.

## Decision

### On-the-fly generation, not committed `dist/`

SSOT → per-client output happens on the developer's machine when they run
an install or update command. No CI generation step. No committed `dist/`
artifacts. Per-developer client detection works because generation runs
locally.

Transform cost is negligible (milliseconds per item). Committing generated
output produces diff noise on every item change and makes per-developer
client targeting impossible.

### Embed `dprint-plugin-markdown`, exact-pinned at `=0.21.1`

`dprint-plugin-markdown` is added as a direct Rust crate dependency with
an exact version requirement (`=0.21.1`, not caret). Bumps are deliberate,
each gated on re-running golden-file fixtures. `dprint.json`'s WASM
plugin pin is aligned with the embedded crate version so contributors
running `just lint` see the same output as CI.

This decision originated as the Phase 0 spike outcome under ADR-0001 and
moves here as part of the umbrella refactor.

**Rationale.**

- Output is byte-identical to the `dprint` CLI on the 36 KB
  `docs/format-spec.md` test.
- Byte-identity holds across 4 minor versions (`0.17.8` → `0.21.1`,
  ~18 months apart) — formatter is effectively output-stable even though
  the API isn't.
- Marginal binary cost is ~+1 MB after dep overlap — inside the
  architecture target (~2-3 MB).
- Removes the `dprint` CLI as a runtime dep for upskill users.

**Phase 1 implementation notes.**

- Real signature:
  `format_text(text: &str, config: &Configuration, callback: impl FnMut(...) -> Result<Option<String>>) -> Result<Option<String>>`.
- `Ok(None)` means "input was already canonical, hand it back as-is" —
  must not be misread as "no output." A naive `unwrap()` will panic on
  already-formatted files.
- Default `text_wrap` is `Maintain`. `line_width` alone does not trigger
  re-wrapping; set `TextWrap::Always` to wrap.
- Pass `|_, _, _| Ok(None)` for the code-block callback to leave fenced
  blocks untouched.

### Per-client output paths and frontmatter mapping

Per format-spec §7:

| Item kind | Claude Code                             | Copilot                                                       | opencode                                       |
| --------- | --------------------------------------- | ------------------------------------------------------------- | ---------------------------------------------- |
| Rule      | `.claude/rules/<name>.md` with `paths:` | `.github/instructions/<name>.instructions.md` with `applyTo:` | path entry in `opencode.json` `instructions[]` |
| Skill     | `.claude/skills/<name>/SKILL.md`        | `.github/skills/<name>/SKILL.md`                              | `.agents/skills/<name>/SKILL.md` (native)      |
| Agent     | `.claude/agents/<name>.md`              | `.github/agents/<name>.agent.md`                              | `.opencode/agents/<name>.md`                   |

### Copy, not symlink

All installation is file copy or generated output. No symlinks anywhere.
One code path; Windows portability without Developer Mode or
`core.symlinks=true`. The deliberate divergence from skills.sh's
symlink-first default is documented in
[ADR-0005](./0005-vercel-skills-sh-interop.md).

### Ancillary file handling

Three ancillary files are managed idempotently:

- **`CLAUDE.md`** at repo root: created once with `@AGENTS.md` content if
  absent. Never overwritten — protects user customisations.
- **`opencode.json`**: read existing file, update `instructions[]` array
  additively, write back. Other keys preserved.
- **`.vscode/settings.json`**: read existing file, set
  `chat.instructionsFilesLocations`, write back. Other keys preserved.

### State file and v0.1 lockfile migration

State lives at `~/.upskill/installed.json`, schema-versioned (`schema: 1`).
Tracks installed items, their source, and content hash for drift
detection. Implementations refuse higher schema versions with a clear
upgrade message and a reset offer.

v0.1's per-project `.upskill-lock.json` is read once by a translator that
produces equivalent entries in `~/.upskill/installed.json`. Existing v0.1
users are not silently broken.

## Consequences

**Positive.** Generated files are linter-clean by construction. dprint
embedding removes the runtime `dprint` CLI dependency. Copy-only
installation eliminates symlink portability issues. Schema-versioned
state file enables future migrations without ambiguity.

**Negative.** A generation bug forces a tool-wide release — no CI-side
fix possible. Dual-state (state file + agent paths on disk) requires
explicit `update` to refresh; not automatic via symlink. Manual dprint
bumps with golden-file revalidation add a small but recurring maintenance
cost.

## Alternatives considered

**(a) CI-generated, committed `dist/` output.** Rejected: noisy diffs on
every item change, prevents per-developer client detection, requires CI
infrastructure per item-source repo. On-the-fly generation is simpler
given negligible transform cost.

**(b) Symlink-first installation.** Rejected: Windows portability requires
Developer Mode + `core.symlinks=true`, which cannot be guaranteed across
all developer machines.

**(c) Shell out to `dprint`.** Rejected (Phase 0 spike): forces a 6-9 MB
Homebrew dependency on users, turns formatting into a subprocess dance
with stdin/stdout error handling.

**(d) Wasmtime-host the `.wasm` plugin.** Rejected: pulls a 5-10 MB
wasmtime runtime, adds a second runtime to debug, reimplements what
`dprint` already does.

## References

- Format spec §7 (generation): [`docs/format-spec.md`](../format-spec.md)
- Parent ADR: [ADR-0001](./0001-v0.2-architectural-reset.md)
- Sibling ADRs: [ADR-0002](./0002-portable-content-format.md),
  [ADR-0004](./0004-cli-surface.md),
  [ADR-0005](./0005-vercel-skills-sh-interop.md)
