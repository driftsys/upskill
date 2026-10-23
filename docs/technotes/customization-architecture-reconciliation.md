# Customization architecture reconciliation

Task [#333](https://github.com/driftsys/upskill/issues/333), 2026-10-19. This
record explains corrected architecture and planning, not new product support.
The baseline is main `4d26d64`; the earlier state-rejection preparation commit
`3eca09e` remains separate and unpublished. Production schema-2 activation is
blocked. Original dirty Gate 6 drafts remain preserved in the prior planning
worktree; the revised design is now the current planning copy.

## Contradictions and resolutions

| Record                                            | Contradiction or ambiguity                                                                  | Resolution                                                                                                                                    |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Product roadmap vs M1-B and original Gate 6 draft | Independent plugin rules vs contributions in team `AGENTS.md`                               | AD-0040 and the delivery contract require independent qualified Codex/OpenCode plugin routes; contributions remain experimental               |
| AD-0039 and library integration                   | Successful aggregate editing could be mistaken for a production route decision              | Keep the bounded library decision, reader reports and recovery tests; explicitly supersede any production interpretation                      |
| Foundation shared discovery                       | Reader compatibility/equal bytes could imply permission to share                            | Require explicit intended consumers and successful native unintended-reader and duplicate controls; incidental readers gain no ownership      |
| Older lifecycle/roadmap scope                     | Three priority clients plus OpenCode preservation                                           | Four equal initial clients; OpenCode independent rule delivery is required, not just historical-output preservation                           |
| Canonical-file qualification note                 | A passing file-discovery pair could imply plugin rule qualification                         | Historical probes remain background evidence; installed package/event/trust/activation controls are separate                                  |
| Shared-updater and category plans                 | Category work could inherit an unqualified route or advance general hooks/tools/permissions | Add the same delivery/reader/preservation/evidence prerequisite; retain release ordering and current scope                                    |
| Schema-2 planning                                 | Available file/recovery foundation could imply native-effect and all-command readiness      | All installed-state commands and native registrations need compatible preservation/recovery; file-only receipts do not qualify native effects |

The [delivery contract](../topics/0009-customization-delivery.md) records every
customization/client route, possible readers, intent, preservation strategy and
I/X/O/Q/P status. The
[corrected Gate 6 design](../wip/specs/2026-10-18-gate6-activation-design.md)
and
[implementation plan](../wip/plans/2026-10-19-customization-delivery-reconciliation.md)
sequence version/reader inventory, installed-plugin probes, opt-in qualified
adapters, all-kind compatibility and finally combined dispatch/migration.
Historical archive bytes were not rewritten. No runtime code changed.

## Corrected qualification work started

After the architecture/builder check, installed command interfaces were probed
without installing a plugin, calling a model or modifying team/personal
instruction files. The
[sanitized interface manifest](agent-customization/plugin-interface-2026-10-19.json)
records exact commands, exit codes, SHA-256 and external local receipts under
`/private/tmp/upskill-plugin-interface-20261019/`. These help/version receipts
are local evidence of available CLI verbs only, not portable retained
transcripts or proof of native activation.

| Client       | Installed version | Observed interface                                                   | Remaining route proof                                                                     |
| ------------ | ----------------- | -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Claude Code  | 2.1.292           | Plugin install/enable/disable and component inventory commands exist | Installed content discovery, consumer isolation and duplicate activation                  |
| Copilot CLI  | 1.0.98            | Plugin lifecycle commands exist, with native enable/disable          | Compatibility/IDE exposure, activation and scoped behavior                                |
| Codex CLI    | 0.163.0           | Plugin add/list/remove and marketplace commands exist                | Local installed package, trust and independent context lifecycle beside team instructions |
| OpenCode CLI | 2.0.25            | Plugin add/list/update/remove commands exist                         | V2 package API/context activation and provider-backed positive/negative controls          |

The installed OpenCode interface expects an explicit `plugin add` verb, while
`src/plugin.rs` still invokes the historical `plugin <module>` form. This is a
compatibility candidate to cover in the corrected adapter plan, not permission
to change it during this architecture-only task.

[Official Codex hook documentation](https://learn.chatgpt.com/docs/hooks)
describes additional context at session start and lifecycle variants including
resume/compaction. That is a candidate internal rule-delivery mechanism; this
installed version, packaged hook trust, precedence and exactly-once behavior
remain unqualified.
[OpenCode v2 plugin documentation](https://dev.opencode.ai/v2/docs/plugins/)
describes package registration separately from runtime behavior; configuration
syntax alone does not prove independent instruction injection.

The
[upstream release inventory](agent-customization/upstream-release-inventory-2026-10-19.json)
records latest stable release metadata checked through the official
repositories: Codex `rust-v0.163.0`, Claude Code `v2.1.303` and Copilot CLI
`v1.0.98`, each with macOS arm64 and Linux x86_64 assets. These are available
release artifacts, not execution on both platforms. Installed Claude `2.1.292`
is older than that candidate baseline. OpenCode's GitHub default release line
was v1 rather than the required v2; no v2 baseline is inferred from it. The
documented v2 update endpoint could not be resolved in the retained metadata
request. No client was upgraded.

The campaign has not frozen verified common stable releases for all four
clients, provisioned isolated authenticated user roots or run corrected plugin
model controls. None of these interface probes passes CD-02 through CD-09. The
previous canonical-file probes do not fill those gaps. Four-client scope and all
applicable native requirements remain intact.

## Review and local validation

The
[isolated architecture/builder review receipt](evidence/2026-10-19-customization-architecture/architecture-review.md)
records no substantive gaps in the corrected architecture and acceptance plan.
`just fmt`, `just verify`, `just sanity`, `just docs-check` and `just book`
passed locally. Full verification ran 1418 Rust tests with 2 ignored across 89
result groups, lint/offline checks, documentation validation and the final
build. The
[receipt manifest](evidence/2026-10-19-customization-architecture/manifest.json)
links compressed outputs and hashes. Hosted checks and native plugin
qualification remain separate. Active WIP intentionally remains active because
the user requested revised plans for unfinished implementation and
qualification.
