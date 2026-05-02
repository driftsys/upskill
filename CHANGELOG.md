# Changelog

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
