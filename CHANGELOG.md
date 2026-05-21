# Changelog

## Unreleased

### Breaking Changes

- **bundle format:** bundle manifests are pure YAML files named
  `<name>.bundle.yaml` (was Markdown-with-frontmatter `<name>.bundle.md`).
  The schema is unchanged; only the file format changes. Discovery
  matches the `.bundle.yaml` suffix and gates on a top-level integer
  `schema:` key — YAML files without it are silently skipped. A bundle
  MAY have an optional sibling `<name>.bundle.md` carrying human-readable
  documentation; the parser ignores it. See
  [ADR-0007](docs/adr/0007-bundle-yaml-format.md).
  Migration (pre-1.0, no back-compat): rename `<name>.bundle.md` →
  `<name>.bundle.yaml` and strip the `---` delimiters and Markdown body.
  Move any worth-keeping prose to a sibling `<name>.bundle.md`.

## [0.5.1] (2026-05-21)

### Documentation

- document recommended skills/ source-registry layout ([87e437f])

[0.5.1]: https://github.com/driftsys/upskill/compare/v0.5.0...v0.5.1
[87e437f]: https://github.com/driftsys/upskill/commit/87e437f

## [0.5.0] (2026-05-20)

### Refactoring

- **registry:** remove git-native registry feature ([#141]) ([870dd21])

### Documentation

- reconcile ADR-0003 and registry spec/plan with code ([038c977])

[0.5.0]: https://github.com/driftsys/upskill/compare/v0.4.2...v0.5.0
[870dd21]: https://github.com/driftsys/upskill/commit/870dd21
[#141]: https://github.com/driftsys/upskill/issues/141
[038c977]: https://github.com/driftsys/upskill/commit/038c977

## [0.4.2] (2026-05-20)

### Features

- **registry:** git-native registry standard ([#63]) ([711a241])

### Documentation

- drop unimplemented bundle-name source form from spec ([#139]) ([9cffdeb])

[0.4.2]: https://github.com/driftsys/upskill/compare/v0.4.1...v0.4.2
[711a241]: https://github.com/driftsys/upskill/commit/711a241
[#63]: https://github.com/driftsys/upskill/issues/63
[9cffdeb]: https://github.com/driftsys/upskill/commit/9cffdeb
[#139]: https://github.com/driftsys/upskill/issues/139

## 0.4.1 (2026-05-16)

### Bug Fixes

- install.sh resolves install dir without requiring HOME (#136) (9daeaca)

## 0.4.0 (2026-05-16)

### Features

- curl|sh installer and release workflow (#135) (d9c4c01)
- **layout:** flat item layout — drop kind subdirectories (#134) (5d46dc6)

## [0.3.1] (2026-05-05)

### Documentation

- strike §4.2, simplify lockfile to one shape in two locations ([94efa95]),
  closes 110.

Following the `npx skills` reference model: the lockfile is the
same
shape (`.upskill-lock.json`, `schema: 1`) regardless of scope. It
just
lives in `<cwd>/` for project scope or `$HOME/` for global scope. No
need
for a separate `~/.upskill/installed.json` aggregator.

### Features

- **cli:** implement [items...] subset filter on add ([#132]) ([b8ea586]),
  closes [#124]
- **cli:** UX polish bundle — confirm, progress, timeout, signal, short flags
  ([#131]) ([bdac6c3]), closes [#118]
- **cli:** --json output for list and doctor ([#128]) ([424f180]), closes [#114]
- **cli:** generate man pages via clap_mangen ([#127]) ([b69d339]), closes
  [#117]
- **cli:** add -q/--quiet global flag ([cef9dde]), closes [#115]
- **help:** add bug-report URL, examples per subcommand, drop ADR refs
  ([b5ea91c]), closes 116.

clig.dev §Help findings:

- **Bug-report URL** —
  top-level `upskill --help` now ends with a
  `DOCUMENTATION:` and `REPORT
  BUGS:` block via clap's `after_help`.
- **Examples per subcommand** — every
  subcommand's `--help` now shows
  an `EXAMPLES:` section with 2–6
  representative invocations. Covers

  add/remove/update/list/doctor/search/lint/fmt/new.
- **ADR refs out of help
  text** — five doc comments referenced
  ADR-0003/0004 or `format-spec §`.
  End users don't have those
  documents loaded; trimmed to user-facing prose.
  ADRs remain the
  source of truth in `docs/adr/` for contributors.
- **style:** color output with clig.dev disable chain ([7aa1608]), closes
  108.

Adds the universal palette and the five-signal disable chain promised
by
spec §6.2 / §6.3. Previously `NO_COLOR` was listed in the spec but
no
color logic existed at all — the test asserting it gave false
confidence.

## Color crate

`colored = "3"` — one new dep, idiomatic API.
Auto-detects TTY via the
crate's defaults; the disable chain wraps that with
explicit overrides.

## Disable chain (clig.dev)

In `src/style.rs::init()`,
applied in order:

1. `--no-color` flag (CLI)
2. `NO_COLOR` env var
   (non-empty)
3. `UPSKILL_NO_COLOR` env var (app-specific override)
4.

`TERM=dumb`
5. `!isatty(target)` — handled by `colored`
defaults

`FORCE_COLOR` (or `CLICOLOR_FORCE`) re-enables color even when
piped.

## Universal palette

Applied across all 9 commands' output:

- **red
  bold** — error labels, missing outputs in `doctor`, error
  severity in
  `lint`
- **yellow** — warnings, `would change` in `update --dry-run`,
  `formatted:` in `fmt`, hash drift in `doctor`
- **green** — `Installed`,
  `Removed`, `updated`, `scaffolded`, `doctor: clean`
- **dim/gray** —
  secondary info: source labels, kind labels, paths, descriptions, orphan
  reasons
- **bold** — primary identifiers: item names, file paths, bundle
  names

## Centralised error printing

New `print_error(...)` and
`print_error_chain(...)` helpers replace 18
direct `eprintln!("error: ...")`
sites with a uniform red-bold label
shape, while keeping the same plain text
under disable.

## Tests

- 6 unit tests in `src/style.rs` exhaustively cover
  the disable chain
  with env var precedence (`NO_COLOR`, `UPSKILL_NO_COLOR`,
  `TERM=dumb`, `FORCE_COLOR`, empty-`NO_COLOR`-doesn't-disable).
- 6 ATDD tests
  in `tests/cli_ci_mode.rs` (was the misleading
  single-test file) pin the
  contract through the actual binary:
  - `NO_COLOR` env strips ANSI
  -
  `--no-color` flag strips ANSI
  - `UPSKILL_NO_COLOR` strips ANSI
  -
  `TERM=dumb` strips ANSI
  - piped output strips ANSI by default (no FORCE
    override)
  - `FORCE_COLOR=1` re-enables ANSI even when piped
- **scope:** -p/--project flag, auto-fallback, positive --global tests
  ([ff92224]), closes 109.

Completes the partial `--global` implementation.
Before: `-g` was wired
to `install_target(global: bool)` but no positive
integration test
proved it wrote to `$HOME`. Behind: spec §2.1 promised
auto-fallback
to global when `cwd` is not in a git repo, and `-p/--project` as
an
explicit override (parity with `npx skills update -p`). Neither
was
implemented.

- **cli:** --version flag and HTTPS_PROXY support in search ([f607955]), closes
  [#106], #107.

`--version` / `-V` now prints `upskill <version>` to stdout and
exits 0, via clap's `#[command(version)]` reading the package version.
Two
integration tests pin the contract.

`search` now configures `ureq` with
`HTTPS_PROXY` (or lowercase
`https_proxy`) so corporate users can reach
skills.sh through their
proxy. `git`/`gh`/`glab` already honor proxy env vars
implicitly; this
closes the gap for the only `ureq` call site. NO_PROXY host
bypass is
not implemented — corporate users with host-specific exclusions
should
configure that at the system level.

[0.3.1]: https://github.com/driftsys/upskill/compare/v0.3.0...v0.3.1
[94efa95]: https://github.com/driftsys/upskill/commit/94efa95
[b8ea586]: https://github.com/driftsys/upskill/commit/b8ea586
[#132]: https://github.com/driftsys/upskill/issues/132
[#124]: https://github.com/driftsys/upskill/issues/124
[bdac6c3]: https://github.com/driftsys/upskill/commit/bdac6c3
[#131]: https://github.com/driftsys/upskill/issues/131
[#118]: https://github.com/driftsys/upskill/issues/118
[424f180]: https://github.com/driftsys/upskill/commit/424f180
[#128]: https://github.com/driftsys/upskill/issues/128
[#114]: https://github.com/driftsys/upskill/issues/114
[b69d339]: https://github.com/driftsys/upskill/commit/b69d339
[#127]: https://github.com/driftsys/upskill/issues/127
[#117]: https://github.com/driftsys/upskill/issues/117
[cef9dde]: https://github.com/driftsys/upskill/commit/cef9dde
[#115]: https://github.com/driftsys/upskill/issues/115
[b5ea91c]: https://github.com/driftsys/upskill/commit/b5ea91c
[7aa1608]: https://github.com/driftsys/upskill/commit/7aa1608
[ff92224]: https://github.com/driftsys/upskill/commit/ff92224
[f607955]: https://github.com/driftsys/upskill/commit/f607955
[#106]: https://github.com/driftsys/upskill/issues/106

## [0.3.0] (2026-05-02)

### Refactoring

- drop pre-1.0 back-compat shims and rename lockfile module ([81a2dda])

### Documentation

- **book:** make book standalone, drop v0.1 migration mentions ([f85ad3d])
- **book:** restructure as user-focused book; architecture → ADR-0000
  ([f8e4d5e])

[0.3.0]: https://github.com/driftsys/upskill/compare/v0.2.0...v0.3.0
[81a2dda]: https://github.com/driftsys/upskill/commit/81a2dda
[f85ad3d]: https://github.com/driftsys/upskill/commit/f85ad3d
[f8e4d5e]: https://github.com/driftsys/upskill/commit/f8e4d5e

## [0.2.0] (2026-05-02)

### Features

- **new:** scaffold new rule / skill / agent items ([#98]) ([8bb4687])
- **fmt:** canonicalise YAML frontmatter in SSOT files ([#97]) ([a7c259b])
- **lint:** implement `upskill lint` with five rules ([#96]) ([ac64b54])
- **list:** implement `upskill list` over schema-2 lockfile ([#95]) ([e364042])
- **bundle:** resolve and install bundles end-to-end ([#94]) ([f139e26])
- **bundle:** parse schema and discover *.bundle.md files ([#93]) ([e810f1b])
- **cli:** implement `upskill doctor` consistency check ([#92]) ([28e6ef8])
- **cli:** implement `upskill update` with --dry-run ([#91]) ([20b93fc])
- **cli:** implement `upskill remove` over schema-2 lockfile ([#90]) ([3abcb31])
- **cli:** replace v0.1 add wholesale with v0.2 pipeline as default ([#89])
  ([afba448])
- **pipeline:** inject GITHUB_TOKEN/GITLAB_TOKEN into clone URLs ([#88])
  ([89c7770])
- **ancillary:** register .github/instructions in .vscode/settings.json ([#87])
  ([29d3a27])
- **ancillary:** register opencode rules glob in opencode.json ([#86])
  ([63a9f82])
- **ancillary:** create CLAUDE.md bridge after pipeline install ([#85])
  ([a7e8c6c])
- **pipeline:** wire GitLab fetch through install_from_source ([#84])
  ([8092576])
- **model:** promote audience to top-level field per format-spec §3.1 ([#83])
  ([b2f9cc4])
- **lockfile:** in-place v0.1 → v0.2 lockfile migration on first load ([#82])
  ([f0e449d])
- **lockfile:** write schema-2 .upskill-lock.json after pipeline install
  ([7dd7993])
- **cli:** hidden --pipeline flag on add routes to v0.2 pipeline ([0c1ccf2])
- **pipeline:** install_from_source dispatches over InstallSource ([06fd667])
- **pipeline:** install local SSOT to per-client output on disk ([e64a090])
- **generate:** extend pipeline to agents with mode/tools/preload-skills
  ([76d06a0])
- **generate:** extend pipeline to rules with path-scoping ([36e7573])
- **generate:** add skills generation pipeline for Claude/Copilot/opencode
  ([268e8b8])
- **parse:** add YAML frontmatter parser ([5b46368])
- **model:** add SSOT data model for rules, skills, agents ([13f7513])

### Bug Fixes

- **generate:** drop unmapped capabilities from copilot agent tools ([96ef485])
- **generate:** always emit name: in rule and agent frontmatter ([b40e2bc])
- **generate:** correct opencode agent frontmatter to use permission map
  ([b59bf82])

### Documentation

- align book + AGENTS with .upskill-lock.json (post-[#75] / [#76] cleanup)
  ([0f779d4])
- **format-spec:** apply review findings — consistency, scope, balance
  ([56b2998])
- rewrite book to v0.2 model and refresh AGENTS/README ([ac63989])
- align book with v0.2 ADRs and rename umbrella ADRs ([4f66a71])
- address review on doc-reconciliation PR ([ef74751])
- copilot tool mapping is strict ([15029b5])
- reconcile opencode agent format with permission map ([d0440bb])
- opencode rules generate to .agents/rules/ ([1a6b290])
- add .upskill.lock per-project state design ([54bdfcd])
- clarify SSOT lives in source registry only ([7b6cee4])
- **adr:** add ADR-0005 vercel skills.sh interop ([d5cda00])
- **adr:** add ADR-0004 cli surface ([0af10f4])
- **adr:** add ADR-0003 generation pipeline ([295b589])
- **adr:** add ADR-0002 portable content format ([039e412])
- **adr:** record v0.2 architectural reset decision (ADR-0001) ([f0d068d])
- add portable format spec for AI-assistance content ([c016d65])
- **agents:** merge repo conventions into AGENTS.md, add CLAUDE.md import
  ([466895e])

### Refactoring

- **adr:** slim ADR-0001 to umbrella scope ([9895672])

[0.2.0]: https://github.com/driftsys/upskill/compare/v0.1.0...v0.2.0
[8bb4687]: https://github.com/driftsys/upskill/commit/8bb4687
[#98]: https://github.com/driftsys/upskill/issues/98
[a7c259b]: https://github.com/driftsys/upskill/commit/a7c259b
[#97]: https://github.com/driftsys/upskill/issues/97
[ac64b54]: https://github.com/driftsys/upskill/commit/ac64b54
[#96]: https://github.com/driftsys/upskill/issues/96
[e364042]: https://github.com/driftsys/upskill/commit/e364042
[#95]: https://github.com/driftsys/upskill/issues/95
[f139e26]: https://github.com/driftsys/upskill/commit/f139e26
[#94]: https://github.com/driftsys/upskill/issues/94
[e810f1b]: https://github.com/driftsys/upskill/commit/e810f1b
[#93]: https://github.com/driftsys/upskill/issues/93
[28e6ef8]: https://github.com/driftsys/upskill/commit/28e6ef8
[#92]: https://github.com/driftsys/upskill/issues/92
[20b93fc]: https://github.com/driftsys/upskill/commit/20b93fc
[#91]: https://github.com/driftsys/upskill/issues/91
[3abcb31]: https://github.com/driftsys/upskill/commit/3abcb31
[#90]: https://github.com/driftsys/upskill/issues/90
[afba448]: https://github.com/driftsys/upskill/commit/afba448
[#89]: https://github.com/driftsys/upskill/issues/89
[89c7770]: https://github.com/driftsys/upskill/commit/89c7770
[#88]: https://github.com/driftsys/upskill/issues/88
[29d3a27]: https://github.com/driftsys/upskill/commit/29d3a27
[#87]: https://github.com/driftsys/upskill/issues/87
[63a9f82]: https://github.com/driftsys/upskill/commit/63a9f82
[#86]: https://github.com/driftsys/upskill/issues/86
[a7e8c6c]: https://github.com/driftsys/upskill/commit/a7e8c6c
[#85]: https://github.com/driftsys/upskill/issues/85
[8092576]: https://github.com/driftsys/upskill/commit/8092576
[#84]: https://github.com/driftsys/upskill/issues/84
[b2f9cc4]: https://github.com/driftsys/upskill/commit/b2f9cc4
[#83]: https://github.com/driftsys/upskill/issues/83
[f0e449d]: https://github.com/driftsys/upskill/commit/f0e449d
[#82]: https://github.com/driftsys/upskill/issues/82
[7dd7993]: https://github.com/driftsys/upskill/commit/7dd7993
[0c1ccf2]: https://github.com/driftsys/upskill/commit/0c1ccf2
[06fd667]: https://github.com/driftsys/upskill/commit/06fd667
[e64a090]: https://github.com/driftsys/upskill/commit/e64a090
[76d06a0]: https://github.com/driftsys/upskill/commit/76d06a0
[36e7573]: https://github.com/driftsys/upskill/commit/36e7573
[268e8b8]: https://github.com/driftsys/upskill/commit/268e8b8
[5b46368]: https://github.com/driftsys/upskill/commit/5b46368
[13f7513]: https://github.com/driftsys/upskill/commit/13f7513
[96ef485]: https://github.com/driftsys/upskill/commit/96ef485
[b40e2bc]: https://github.com/driftsys/upskill/commit/b40e2bc
[b59bf82]: https://github.com/driftsys/upskill/commit/b59bf82
[0f779d4]: https://github.com/driftsys/upskill/commit/0f779d4
[#75]: https://github.com/driftsys/upskill/issues/75
[#76]: https://github.com/driftsys/upskill/issues/76
[56b2998]: https://github.com/driftsys/upskill/commit/56b2998
[ac63989]: https://github.com/driftsys/upskill/commit/ac63989
[4f66a71]: https://github.com/driftsys/upskill/commit/4f66a71
[ef74751]: https://github.com/driftsys/upskill/commit/ef74751
[15029b5]: https://github.com/driftsys/upskill/commit/15029b5
[d0440bb]: https://github.com/driftsys/upskill/commit/d0440bb
[1a6b290]: https://github.com/driftsys/upskill/commit/1a6b290
[54bdfcd]: https://github.com/driftsys/upskill/commit/54bdfcd
[7b6cee4]: https://github.com/driftsys/upskill/commit/7b6cee4
[d5cda00]: https://github.com/driftsys/upskill/commit/d5cda00
[0af10f4]: https://github.com/driftsys/upskill/commit/0af10f4
[295b589]: https://github.com/driftsys/upskill/commit/295b589
[039e412]: https://github.com/driftsys/upskill/commit/039e412
[f0d068d]: https://github.com/driftsys/upskill/commit/f0d068d
[c016d65]: https://github.com/driftsys/upskill/commit/c016d65
[466895e]: https://github.com/driftsys/upskill/commit/466895e
[9895672]: https://github.com/driftsys/upskill/commit/9895672
