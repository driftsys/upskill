# Changelog

## [0.7.8] (2026-06-10)

### Features

- **bundle:** resolve cross-source bundle requires ([f335d36])
- **source:** support GitLab subgroup paths in install sources ([2ee7780])

[0.7.8]: https://github.com/driftsys/upskill/compare/v0.7.7...v0.7.8
[f335d36]: https://github.com/driftsys/upskill/commit/f335d36
[2ee7780]: https://github.com/driftsys/upskill/commit/2ee7780

## [0.7.7] (2026-06-04)

### Bug Fixes

- **plugin:** use marketplace name for claude/copilot install ref ([#227])
  ([d7dc394])

[0.7.7]: https://github.com/driftsys/upskill/compare/v0.7.6...v0.7.7
[d7dc394]: https://github.com/driftsys/upskill/commit/d7dc394
[#227]: https://github.com/driftsys/upskill/issues/227

## [0.7.6] (2026-06-04)

### Bug Fixes

- **generate:** match percent-encoded resource links; harden walkers ([#203])
  ([90b2fc2])

### Refactoring

- **skills:** rename upskill-using skill to upskill-cli ([2c672c4])

### Documentation

- **spec:** fix broken ADR-0010 relative link in MCP v2 spec ([897cfca])

### Features

- **pipeline:** relocate resources and links when aliasing with --as ([#200])
  ([383e701])

[0.7.6]: https://github.com/driftsys/upskill/compare/v0.7.5...v0.7.6
[90b2fc2]: https://github.com/driftsys/upskill/commit/90b2fc2
[#203]: https://github.com/driftsys/upskill/issues/203
[2c672c4]: https://github.com/driftsys/upskill/commit/2c672c4
[897cfca]: https://github.com/driftsys/upskill/commit/897cfca
[383e701]: https://github.com/driftsys/upskill/commit/383e701
[#200]: https://github.com/driftsys/upskill/issues/200

## [0.7.5] (2026-06-04)

### Refactoring

- **lint:** avoid fabricating empty frontmatter in body-format check ([46efa72])
- **lint:** guard bodyless files; dedupe body-format ATDD fixture ([c17912e])

### Features

- **lint:** add body-format rule using shared canonical_body helper ([6315814])
- **fmt:** format item markdown body via shared canonical_body helper
  ([90af065])

### Documentation

- fmt formats the body; add body-format lint rule ([d83eda6])
- **plan:** implementation plan for fmt formatting the source body ([6b250ee])
- **spec:** correct fmt body-format design for the frontmatter seam ([2d0d41b])
- **spec:** design for fmt formatting the source body ([625c422])

[0.7.5]: https://github.com/driftsys/upskill/compare/v0.7.4...v0.7.5
[46efa72]: https://github.com/driftsys/upskill/commit/46efa72
[c17912e]: https://github.com/driftsys/upskill/commit/c17912e
[6315814]: https://github.com/driftsys/upskill/commit/6315814
[90af065]: https://github.com/driftsys/upskill/commit/90af065
[d83eda6]: https://github.com/driftsys/upskill/commit/d83eda6
[6b250ee]: https://github.com/driftsys/upskill/commit/6b250ee
[2d0d41b]: https://github.com/driftsys/upskill/commit/2d0d41b
[625c422]: https://github.com/driftsys/upskill/commit/625c422

## [0.7.4] (2026-06-03)

### Documentation

- cover MCP server support ([#218]) ([99c70a6])
- cover plugins + co-location, drop ADRs from book nav ([f1e0e5c])
- **recipes:** MCP local-install convention recipe + distilling pattern
  ([4588087])
- **spec:** MCP local-install convention design (v2, docs-only) ([1673d82])
- **mcp:** correct requires-env warning timing and adr index gap ([2d4b651])
- **mcp:** ADR-0010 and format-spec mcps: sub-shape ([fc8d26d])
- **plan:** MCP server support v1 implementation plan ([28e7177])
- **spec:** add MCP server support design ([dcdd8c7])

### Bug Fixes

- **fmt:** canonicalise bundle mcps: key after plugins ([5b71288])
- **cli:** surface failed MCP uninstall, doctor drift exit code, help ordering
  ([f2f7fdd])
- **pipeline:** drop side-effecting MCP unit test; gitignore .mcp.json
  ([621eb92])

### Features

- **lint:** surface mcps validation errors ([a6d9a1a])
- **cli:** report, remove, and doctor-reconcile MCP servers ([5ec551e])
- **pipeline:** configure MCP servers from bundles + record in lockfile
  ([f76bfc0])
- **lockfile:** record MCP servers (LockedMcp + lifecycle) ([1d90c09])
- **ancillary:** .mcp.json config-write fallback for MCP servers ([da6d88e])
- **mcp:** claude mcp add/remove/list shellout ([b5f3e01])
- **model:** validate mcps transport fields ([1ddf52c])
- **model:** add mcps: descriptor types to Bundle ([8e1ce16])

### Refactoring

- **skills:** namespace-prefix item names with upskill- ([f4d723d])
- **plugin:** expose command helpers as pub(crate) ([628c5f8])
- **model:** re-export MCP types from model module ([eded6f5])

[0.7.4]: https://github.com/driftsys/upskill/compare/v0.7.3...v0.7.4
[99c70a6]: https://github.com/driftsys/upskill/commit/99c70a6
[#218]: https://github.com/driftsys/upskill/issues/218
[f1e0e5c]: https://github.com/driftsys/upskill/commit/f1e0e5c
[4588087]: https://github.com/driftsys/upskill/commit/4588087
[1673d82]: https://github.com/driftsys/upskill/commit/1673d82
[2d4b651]: https://github.com/driftsys/upskill/commit/2d4b651
[fc8d26d]: https://github.com/driftsys/upskill/commit/fc8d26d
[28e7177]: https://github.com/driftsys/upskill/commit/28e7177
[dcdd8c7]: https://github.com/driftsys/upskill/commit/dcdd8c7
[5b71288]: https://github.com/driftsys/upskill/commit/5b71288
[f2f7fdd]: https://github.com/driftsys/upskill/commit/f2f7fdd
[621eb92]: https://github.com/driftsys/upskill/commit/621eb92
[a6d9a1a]: https://github.com/driftsys/upskill/commit/a6d9a1a
[5ec551e]: https://github.com/driftsys/upskill/commit/5ec551e
[f76bfc0]: https://github.com/driftsys/upskill/commit/f76bfc0
[1d90c09]: https://github.com/driftsys/upskill/commit/1d90c09
[da6d88e]: https://github.com/driftsys/upskill/commit/da6d88e
[b5f3e01]: https://github.com/driftsys/upskill/commit/b5f3e01
[1ddf52c]: https://github.com/driftsys/upskill/commit/1ddf52c
[8e1ce16]: https://github.com/driftsys/upskill/commit/8e1ce16
[f4d723d]: https://github.com/driftsys/upskill/commit/f4d723d
[628c5f8]: https://github.com/driftsys/upskill/commit/628c5f8
[eded6f5]: https://github.com/driftsys/upskill/commit/eded6f5

## [0.7.2] (2026-06-02)

### Documentation

- **commands:** document supporting-resource copying ([#199]) ([22722a4])
- **plan:** implementation plan for supporting-resource copy ([#199])
  ([ab60837])
- **spec:** design for copying item supporting resources ([#199]) ([494c2bb])

### Bug Fixes

- **pipeline:** skip symlinks in dir walkers to prevent recursion on cycles
  ([#199]) ([28a02d0])
- **pipeline:** address code-review findings ([#199]) ([7df4064])
- **pipeline:** delete resource trees on remove and update-orphan ([#199])
  ([fb555ea])

### Features

- **pipeline:** guard --as against resource-bearing items ([#199]) ([8ff8540])
- **pipeline:** copy item resources and rewrite flat-kind links on install
  ([#199]) ([f65d97f])
- **generate:** rewrite relative resource links for flat kinds ([#199])
  ([103c620])
- **pipeline:** resource base paths and copy helper ([#199]) ([3baf08f])
- **pipeline:** enumerate item supporting resources ([#199]) ([d5a8377])

[0.7.2]: https://github.com/driftsys/upskill/compare/v0.7.1...v0.7.2
[22722a4]: https://github.com/driftsys/upskill/commit/22722a4
[#199]: https://github.com/driftsys/upskill/issues/199
[ab60837]: https://github.com/driftsys/upskill/commit/ab60837
[494c2bb]: https://github.com/driftsys/upskill/commit/494c2bb
[28a02d0]: https://github.com/driftsys/upskill/commit/28a02d0
[7df4064]: https://github.com/driftsys/upskill/commit/7df4064
[fb555ea]: https://github.com/driftsys/upskill/commit/fb555ea
[8ff8540]: https://github.com/driftsys/upskill/commit/8ff8540
[f65d97f]: https://github.com/driftsys/upskill/commit/f65d97f
[103c620]: https://github.com/driftsys/upskill/commit/103c620
[3baf08f]: https://github.com/driftsys/upskill/commit/3baf08f
[d5a8377]: https://github.com/driftsys/upskill/commit/d5a8377

## [0.7.1] (2026-05-30)

### Bug Fixes

- **update:** re-resolve bundle sources in dry-run; guard empty sources ([#196])
  ([24a8965])

[0.7.1]: https://github.com/driftsys/upskill/compare/v0.7.0...v0.7.1
[24a8965]: https://github.com/driftsys/upskill/commit/24a8965
[#196]: https://github.com/driftsys/upskill/issues/196

## [0.7.0] (2026-05-24)

### Bug Fixes

- isolate HOME in all integration tests to prevent writes to real $HOME
  ([c195555]), closes [#193]
- P0 baseline bugs — path traversal, atomic writes, search hint ([#189])
  ([71104b2]), closes [#177], closes #178, closes #179

### Refactoring

- extract pipeline/install.rs ([fa111e4]), refs [#180]
- extract pipeline/lifecycle.rs ([23c81b2]), refs [#180]
- extract pipeline/output.rs ([2d28326]), refs [#180]
- extract pipeline/git.rs ([62249df]), refs [#180]
- extract pipeline/discovery.rs ([42dcca4]), refs [#180]
- extract pipeline/hash.rs ([274749b]), refs [#180]
- extract pipeline/report.rs ([d657a92]), refs [#180]
- scaffold pipeline/ module directory ([de0018b]), refs [#180]
- P1 batch 2 — unify install, docs fixes, UX hints ([b97d7d7]), closes [#181],
  closes #187, closes #188

### Features

- P1 batch 3 — CLI discoverability, skill output contracts ([67e8eef]), closes
  [#184], closes #186

[0.7.0]: https://github.com/driftsys/upskill/compare/v0.6.4...v0.7.0
[c195555]: https://github.com/driftsys/upskill/commit/c195555
[#193]: https://github.com/driftsys/upskill/issues/193
[71104b2]: https://github.com/driftsys/upskill/commit/71104b2
[#189]: https://github.com/driftsys/upskill/issues/189
[#177]: https://github.com/driftsys/upskill/issues/177
[fa111e4]: https://github.com/driftsys/upskill/commit/fa111e4
[#180]: https://github.com/driftsys/upskill/issues/180
[23c81b2]: https://github.com/driftsys/upskill/commit/23c81b2
[2d28326]: https://github.com/driftsys/upskill/commit/2d28326
[62249df]: https://github.com/driftsys/upskill/commit/62249df
[42dcca4]: https://github.com/driftsys/upskill/commit/42dcca4
[274749b]: https://github.com/driftsys/upskill/commit/274749b
[d657a92]: https://github.com/driftsys/upskill/commit/d657a92
[de0018b]: https://github.com/driftsys/upskill/commit/de0018b
[b97d7d7]: https://github.com/driftsys/upskill/commit/b97d7d7
[#181]: https://github.com/driftsys/upskill/issues/181
[67e8eef]: https://github.com/driftsys/upskill/commit/67e8eef
[#184]: https://github.com/driftsys/upskill/issues/184

## [0.6.4] (2026-05-24)

### Documentation

- **spec:** add design for comment-preserving fmt ([#169]) ([59c0e7a])

### Features

- multi-registry search with local index ([#175]) ([91a64d4]), closes [#63]
- item conflict resolution and source locking ([#174]) ([499393c])
- upgrade UX — orphan removal, clean-and-regenerate, interactive confirm
  ([#172]) ([801cb95])
- **model:** extend bundle plugins schema with instructions-only and
  config-write modes ([#171]) ([a851308])

### Bug Fixes

- **fmt:** comment-preserving YAML key reordering ([#169]) ([1297202])

[0.6.4]: https://github.com/driftsys/upskill/compare/v0.6.3...v0.6.4
[59c0e7a]: https://github.com/driftsys/upskill/commit/59c0e7a
[#169]: https://github.com/driftsys/upskill/issues/169
[91a64d4]: https://github.com/driftsys/upskill/commit/91a64d4
[#175]: https://github.com/driftsys/upskill/issues/175
[#63]: https://github.com/driftsys/upskill/issues/63
[499393c]: https://github.com/driftsys/upskill/commit/499393c
[#174]: https://github.com/driftsys/upskill/issues/174
[801cb95]: https://github.com/driftsys/upskill/commit/801cb95
[#172]: https://github.com/driftsys/upskill/issues/172
[a851308]: https://github.com/driftsys/upskill/commit/a851308
[#171]: https://github.com/driftsys/upskill/issues/171
[1297202]: https://github.com/driftsys/upskill/commit/1297202

## [0.6.3] (2026-05-22)

### Features

- **skills:** add writing-skill-bundles skill ([7f46b4c])

### Refactoring

- **skills:** rename prompt-engineering skill to prompt-design ([0a2a83a])

### Documentation

- **skill:** update using-upskill doctor section with plugin reconciliation
  ([3cb96db])
- **readme:** add CI, crates.io, and version tag badges ([bbd91b0])

[0.6.3]: https://github.com/driftsys/upskill/compare/v0.6.2...v0.6.3
[7f46b4c]: https://github.com/driftsys/upskill/commit/7f46b4c
[0a2a83a]: https://github.com/driftsys/upskill/commit/0a2a83a
[3cb96db]: https://github.com/driftsys/upskill/commit/3cb96db
[bbd91b0]: https://github.com/driftsys/upskill/commit/bbd91b0

## [0.6.2] (2026-05-22)

### Features

- **doctor:** plugin reconciliation — report missing/skipped plugins ([#151])
  ([8fc9944])
- **bundle:** add copilot: plugin descriptor for GitHub Copilot CLI ([20a4079]),
  closes [#158]
- **fetch:** use sparse clone for subfolder installs ([994d121]), closes [#60]
- **pipeline:** bundle-by-name discovery for upskill add ([#160]) ([d5be7fb])
- Windows support — USERPROFILE fallback + CI matrix ([#157]) ([a88b010])

### Documentation

- **doctor:** document plugin reconciliation in commands, spec, and format-spec
  ([b62a2ee])
- fix documentation gaps from PR review ([#157], [#160], [#166]) ([4f8b36d])

### Bug Fixes

- **doctor:** detect Windows cmd.exe 'is not recognized' via stderr ([80d09fd])
- **doctor:** cross-platform fake CLI in tests (Windows .bat + correct PATH sep)
  ([ba05f4f])
- **install:** detect items in sibling-layout registries ([79b4f4b]), closes
  [#161]
- use --initial-branch=main for bare repos in tests ([c255d60])
- **fetch:** ensure test repo uses deterministic branch name ([32332b6])
- **lint:** discover entrypoints when pointing at an item directory ([ee20d09]),
  closes [#159]

[0.6.2]: https://github.com/driftsys/upskill/compare/v0.6.1...v0.6.2
[8fc9944]: https://github.com/driftsys/upskill/commit/8fc9944
[#151]: https://github.com/driftsys/upskill/issues/151
[20a4079]: https://github.com/driftsys/upskill/commit/20a4079
[#158]: https://github.com/driftsys/upskill/issues/158
[994d121]: https://github.com/driftsys/upskill/commit/994d121
[#60]: https://github.com/driftsys/upskill/issues/60
[d5be7fb]: https://github.com/driftsys/upskill/commit/d5be7fb
[#160]: https://github.com/driftsys/upskill/issues/160
[a88b010]: https://github.com/driftsys/upskill/commit/a88b010
[#157]: https://github.com/driftsys/upskill/issues/157
[b62a2ee]: https://github.com/driftsys/upskill/commit/b62a2ee
[4f8b36d]: https://github.com/driftsys/upskill/commit/4f8b36d
[#166]: https://github.com/driftsys/upskill/issues/166
[80d09fd]: https://github.com/driftsys/upskill/commit/80d09fd
[ba05f4f]: https://github.com/driftsys/upskill/commit/ba05f4f
[79b4f4b]: https://github.com/driftsys/upskill/commit/79b4f4b
[#161]: https://github.com/driftsys/upskill/issues/161
[c255d60]: https://github.com/driftsys/upskill/commit/c255d60
[32332b6]: https://github.com/driftsys/upskill/commit/32332b6
[ee20d09]: https://github.com/driftsys/upskill/commit/ee20d09
[#159]: https://github.com/driftsys/upskill/issues/159

## [0.6.1] (2026-05-22)

### Documentation

- ADR-0008 plugin install shellout + format-spec §3.7 plugins ([#153])
  ([13bbabb]), closes [#152]

### Features

- **plugin:** add src/plugin.rs client CLI shellout module ([#155]) ([4a2e927]),
  closes 148

* feat(plugin): add src/plugin.rs client CLI shellout module

New
module implementing the plugin install/uninstall surface per
ADR-0008. Shells
out to native client CLIs:

- claude plugin marketplace add + claude plugin
  install --scope
- code --install-extension / --uninstall-extension
- opencode
  plugin / plugin remove

Key design:

- PluginOutcome enum: Success |
  CliNotFound | Failed
- PluginScope enum: Project | User (maps to claude
  --scope flag)
- is_cli_available() presence detection via
  ErrorKind::NotFound
- No new dependencies (std::process::Command only)
- Never
  writes to stdout/stderr (presentation in main.rs)

Tests cover:

- Scope flag
  mapping
- CLI availability detection (true for sh, false for nonexistent)
- CliNotFound path for all three clients
- Success/failure outcome from
  run_command (true/false binaries)
- Predicate methods on PluginOutcome, 149

* feat(pipeline): wire plugin install into install_with_lockfile

Add
LockedPlugin struct to lockfile schema with upsert/remove methods.
Wire plugin
shellout into install_with_lockfile via
install_plugins_from_bundles
orchestrator. Only successful installs are
recorded in the lockfile;
CLI-not-found and failures are reported to
the user via structured
PluginResult entries., [#150]

- **bundle:** add plugins: map to Bundle schema with typed descriptors ([#154])
  ([8b6deeb]), closes [#148]

[0.6.1]: https://github.com/driftsys/upskill/compare/v0.6.0...v0.6.1
[13bbabb]: https://github.com/driftsys/upskill/commit/13bbabb
[#153]: https://github.com/driftsys/upskill/issues/153
[#152]: https://github.com/driftsys/upskill/issues/152
[4a2e927]: https://github.com/driftsys/upskill/commit/4a2e927
[#155]: https://github.com/driftsys/upskill/issues/155
[#150]: https://github.com/driftsys/upskill/issues/150
[8b6deeb]: https://github.com/driftsys/upskill/commit/8b6deeb
[#154]: https://github.com/driftsys/upskill/issues/154
[#148]: https://github.com/driftsys/upskill/issues/148

## [0.6.0] (2026-05-21)

### Documentation

- **agents:** list just book / just book-serve in AGENTS.md ([913450e])
- refactor source-registry layout page into an Upskill conventions doc
  ([ca09ef2])
- surface source-registry layout in a Conventions annexe ([32e6f93])

### Features

- **skills:** add v0.1 prompt-engineering meta-skills bundle ([b597100])

### Refactoring

- **bundle:** switch bundle manifest format to pure YAML (.bundle.yaml)
  ([b1d8573])

[0.6.0]: https://github.com/driftsys/upskill/compare/v0.5.1...v0.6.0
[913450e]: https://github.com/driftsys/upskill/commit/913450e
[ca09ef2]: https://github.com/driftsys/upskill/commit/ca09ef2
[32e6f93]: https://github.com/driftsys/upskill/commit/32e6f93
[b597100]: https://github.com/driftsys/upskill/commit/b597100
[b1d8573]: https://github.com/driftsys/upskill/commit/b1d8573

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
