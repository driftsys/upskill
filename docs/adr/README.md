# Architecture Decision Records

Numbered sequentially. Each record owns its status and rationale. Proposed
records remain proposals even where related code exists. Amend or supersede a
decision explicitly; documentation moves do not change its status.

| Decision                                                                                                                  | Status   |
| ------------------------------------------------------------------------------------------------------------------------- | -------- |
| [0000 — Implementation baseline](0000-implementation-baseline.md)                                                         | Accepted |
| [0001 — Multi-kind compiler architecture — v0.2 redesign umbrella](0001-multi-kind-compiler-architecture.md)              | Proposed |
| [0002 — Portable content format for AI-assistance items and bundles](0002-portable-content-format.md)                     | Proposed |
| [0003 — SSOT-to-client generation pipeline](0003-generation-pipeline.md)                                                  | Proposed |
| [0004 — upskill CLI surface](0004-cli-surface.md)                                                                         | Proposed |
| [0005 — skills.sh ecosystem interop](0005-skills-sh-ecosystem-interop.md)                                                 | Proposed |
| [0006 — Flat item layout — drop kind subdirectories](0006-flat-item-layout.md)                                            | Proposed |
| [0007 — Bundle file format — YAML, not Markdown-with-frontmatter](0007-bundle-yaml-format.md)                             | Proposed |
| [0008 — Plugin installation — extend Bundle, shell out to client CLIs](0008-plugin-install-shellout.md)                   | Accepted |
| [0009 — Coupling tiers and directed dependencies](0009-coupling-tiers-and-dependencies.md)                                | Proposed |
| [0010 — MCP server configuration — bundle `mcps:`, CLI-first config-write](0010-mcp-config-write.md)                      | Accepted |
| [0011 — Generic git-URL source — replace GitLab-specific source with host-agnostic https](0011-generic-git-url-source.md) | Accepted |
| [0012 — Consumer-side client filtering — per-invocation flags and config](0012-consumer-client-filtering.md)              | Accepted |
| [0013 — Agent customization targets — Codex, Cursor, and Gemini CLI](0013-agent-customization-targets.md)                 | Accepted |
| [0014 — Versioned model mappings for custom agents](0014-versioned-agent-model-mappings.md)                               | Accepted |

[0015 — Pure projection before legacy-state migration](0015-pure-legacy-state-projection.md)
records the implemented library boundary and its documented alternatives.

[0016 — Pure instruction planning before lifecycle effects](0016-pure-instruction-planning.md)
records the implemented bounded choice.

[0017 — Require owner evidence before changing shared instruction regions](0017-shared-instruction-owner-evidence.md)
records the implemented bounded choice.

[0018 — Retain aggregates after final-region removal](0018-retain-empty-instruction-aggregates.md)
records the implemented bounded choice.

[0019 — Separate strict source capture from legacy discovery](0019-separate-strict-source-capture.md)
records the opt-in observation boundary.

[0020 — Version and frame captured source evidence](0020-framed-source-manifest-evidence.md)
records its separate digest encoding.

## Owned whole-file planning

- [AD-0021 — Pure whole-file planning before lifecycle effects](0021-pure-file-planning.md)
- [AD-0022 — Require conservative file ownership and before-state evidence](0022-conservative-file-ownership-evidence.md)
- [AD-0023 — Reject affected file and descendant topology](0023-reject-file-topology-transitions.md)

## Captured item compilation

- [AD-0024 — Pure captured item compilation with current generation compatibility](0024-pure-captured-item-compilation.md)
- [AD-0025 — Reject active captured overrides until precedence is implemented](0025-reject-active-captured-overrides.md)
- [AD-0026 — Separate captured resource modes from generated permission policy](0026-separate-generated-permission-policy.md)

## Mixed owned-artifact planning

- [AD-0027 — Compose complete owned-artifact plans before lifecycle effects](0027-compose-complete-owned-artifact-plans.md)

## Qualified scope directories

- [AD-0028 — Lock qualified scope directories before owned lifecycle effects](0028-lock-qualified-scope-directories.md)

## Qualified destination observation

- [AD-0029 — Capture qualified destination evidence](0029-capture-qualified-destination-evidence.md)

## Qualified source observation

- [AD-0030 — Capture qualified source evidence](0030-capture-qualified-source-evidence.md)

## Explicit file permission selection

- [AD-0031 — Select explicit file permissions before file effects](0031-select-explicit-file-permissions.md)

## Ordered recovery checkpoints

- [AD-0032 — Model recovery checkpoints before file effects](0032-model-recovery-checkpoints-before-effects.md)

## Qualified file execution

- [AD-0033 — Qualified file execution with durable forward recovery](0033-qualified-file-execution-and-journals.md)

- [AD-0034 — Record explicit per-owner contract evidence](0034-record-explicit-owner-contracts.md)

- [AD-0035 — Project shared artifacts for physical planning](0035-project-shared-artifacts-for-physical-planning.md)

- [AD-0036 — Separate explicit sharing entrypoints](0036-separate-explicit-sharing-entrypoints.md)

- [AD-0037 — Require create-only backup journals](0037-require-create-only-backup-journals.md)

- [AD-0038 — Bind lifecycle origin to the original backup](0038-bind-lifecycle-origin-to-original-backup.md)

- [AD-0039 — Compile captured Codex instructions with a separate reader report](0039-compile-captured-codex-instructions.md)

- [AD-0040 — Separate customization delivery from discovery and activation](0040-separate-customization-delivery-from-discovery.md)

- [AD-0041 — Separate client representations as the design baseline](0041-separate-client-representations-as-design-baseline.md)
  — Accepted organizing baseline; native qualification remains gated.
