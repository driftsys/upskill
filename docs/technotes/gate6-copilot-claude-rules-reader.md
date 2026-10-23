# Gate 6 Copilot reader of Claude rules: failed positive

Task [#382](https://github.com/driftsys/upskill/issues/382), Q1 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains a failed native positive on 2026-10-24. This packet does not decide
whether Copilot reads `.claude/rules`. Production remains schema 1.

## Method and pins

The [new campaign revision](gate6-plugin-profile-inventory.md) pins Copilot CLI
1.0.103, Claude Code 2.1.308, Codex CLI 0.164.0 and OpenCode 2.1.4 on macOS
arm64. Only Copilot ran a model request in this packet.

The existing `scripts/qualify-native-instructions.py` now constructs Q1 fixtures
and classifies reader outcomes. Each fixture is a separate disposable Git
repository with a distinct empty root commit, hooks and signing disabled, and
inherited `GIT_*` selectors removed. The positive has only
`.github/copilot-instructions.md`; the candidates have only
`.claude/rules/probe.md`, separately without frontmatter and with
`paths: "**/*.rs"`. Each fixture has independent unpredictable instruction and
absent markers. The prompt contains only its fallback marker. No file access or
matching-path capability is inferred from the `paths:` candidate.

The private capture helper uses the previously provisioned isolated Copilot
profile, an allowlisted environment, tools disabled, built-in MCPs disabled,
auto-update disabled and JSON output. No team instructions, discovery settings
or client plugin caches were written by the helper. It creates no `CLAUDE.md` or
`AGENTS.md`. Raw outputs, fixtures and helper remain outside the repository. The
[private source index](evidence/2026-10-24-copilot-claude-rules-reader/private-source-index.json)
records their location and hashes. Retained events omit model identifiers,
usage, unrelated metadata and error text; raw stdout/stderr hashes cover the
original private bytes.

## Controls

| Control                                           | Pin             | Result                                   | Interpretation                         |
| ------------------------------------------------- | --------------- | ---------------------------------------- | -------------------------------------- |
| Native `.github/copilot-instructions.md` positive | Copilot 1.0.103 | FAIL: exit 1, HTTP 402, `quota_exceeded` | Positive could not calibrate discovery |
| `.claude/rules/probe.md` without frontmatter      | Copilot 1.0.103 | NOT RUN                                  | Blocked by failed positive             |
| `.claude/rules/probe.md` with `paths:`            | Copilot 1.0.103 | NOT RUN                                  | Blocked by failed positive             |

The [summary](evidence/2026-10-24-copilot-claude-rules-reader/summary.json)
retains one native request and zero successful positives. Neither candidate has
a "reads" or "does not read" result. The quota failure is not evidence of absent
instructions. Q1 remains open and requires a fresh pinned run with a successful
positive before either candidate can be interpreted.

The exact affected row is the
[proposal Rules section's `.claude/rules`
gate](../wip/specs/2026-10-23-customization-delivery-proposal.md#rules): this
packet selects none of options (a), (b), (c), and does not establish that both
native defaults can coexist. Durable records await step 7. The Codex rules row
remains blocked by #379 independently.

## Preservation and residue

The
[preservation receipt](evidence/2026-10-24-copilot-claude-rules-reader/preservation.json)
records identical before/after bytes and mode `0600` for the isolated profile's
`config.json`; `settings.json` and `mcp-config.json` remained absent. No
measured configuration needed restoration. Authentication stores were not read,
copied or exported. Native session, log and search state may remain in the
disposable profile; these were not inventoried before the run and exact
whole-profile restoration is unqualified. No real user profile was selected.

## Offline validation and limits

Tests were written first. `just lint` failed with the two missing Q1 helper
attributes before the harness change, then passed all 42 offline tests after it.
Earlier sandbox and formatting failures are not the red test evidence. Fixture
tests enforce one selected instruction, separate markers and distinct root
commits; classification tests require an exact successful positive and reject
failures or nonmatching candidate responses. These tests never call a client and
cannot qualify native discovery.

Linux, user scope, IDE and hosted surfaces remain unqualified. No plugin,
subagent, update, resume, compaction or production lifecycle route was tested.
The `paths:` capability is still unknown. One failed native request establishes
no client reader behavior and no support promotion.
