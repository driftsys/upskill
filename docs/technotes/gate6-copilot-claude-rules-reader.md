# Gate 6 Copilot reader of Claude rules

Task [#382](https://github.com/driftsys/upskill/issues/382), Q1 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
records bounded project discovery on 2026-10-24. Production remains schema 1.

## Method and pins

The [campaign revision](gate6-plugin-profile-inventory.md) pins Copilot CLI
1.0.103, Claude Code 2.1.308, Codex CLI 0.164.0 and OpenCode 2.1.4 on macOS
arm64. Only Copilot ran model requests in this packet.

The existing `scripts/qualify-native-instructions.py` constructs Q1 fixtures and
classifies reader outcomes. Every fixture is a separate disposable Git
repository with a distinct empty root commit, hooks and signing disabled, and
inherited `GIT_*` selectors removed. The positive has only
`.github/copilot-instructions.md`. The candidates have only
`.claude/rules/probe.md`, separately without frontmatter and with
`paths: "**/*.rs"`. Each has independent unpredictable instruction and absent
markers. Prompts contain only fallback markers. The paths variant performs no
matching-file access; it cannot establish native scoping or ignored frontmatter.

Private capture helpers use the previously provisioned isolated Copilot profile,
an allowlisted environment, disabled tools and built-in MCPs, disabled automatic
updates and structured JSON output. A successful request requires native exit 0,
a completed result, an assistant response and no observed tool execution or
session error. The positive must return its exact marker before candidates run.
No team instructions, discovery settings or client plugin caches were written by
the helpers. No `CLAUDE.md` or `AGENTS.md` was created.

## Attempts and controls

The initial positive failed with HTTP 402 `quota_exceeded`. After the user
identified available provisioned routes, an explicitly selected route failed
with HTTP 400 and a deployment-availability error. Neither failed positive ran
candidates or yielded a reader conclusion. Both remain retained.

A different normally provisioned route completed the positive and both
candidates. Model selection and identifiers remain private. This is an
observation on that account route, not a provider-wide claim.

| Control                                                 | Pin             | Native result                                  | Interpretation                                                                        |
| ------------------------------------------------------- | --------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------- |
| Initial native positive                                 | Copilot 1.0.103 | FAIL: exit 1, HTTP 402                         | Quota failure; candidates not run                                                     |
| First selected-route retry positive                     | Copilot 1.0.103 | FAIL: exit 1, HTTP 400                         | Deployment unavailable; candidates not run                                            |
| Final native `.github/copilot-instructions.md` positive | Copilot 1.0.103 | PASS: exact marker, exit 0, no tools           | Calibrates the final candidate pair                                                   |
| `.claude/rules/probe.md` without frontmatter            | Copilot 1.0.103 | PASS: exact candidate marker, exit 0, no tools | **Reads** in this project request                                                     |
| `.claude/rules/probe.md` with `paths:`                  | Copilot 1.0.103 | PASS: exact fallback, exit 0, no tools         | **Does not read** in this tool-free request; matching-file activation remains unknown |

The [summary](evidence/2026-10-24-copilot-claude-rules-reader/summary.json)
records five requests: two failed positives followed by three successful
controls. The
[initial failure summary](evidence/2026-10-24-copilot-claude-rules-reader/initial-failure-summary.json)
preserves the earlier checkpoint. Native responses, distinct session IDs,
fixture hashes and root commits are retained. Failed positives are never
reclassified as absence observations.

The exact affected row is the
[proposal Rules section's `.claude/rules`
gate](../wip/specs/2026-10-23-customization-delivery-proposal.md#rules).
Unscoped files are a demonstrated Copilot reader and duplicate-delivery risk.
The two native defaults therefore cannot be assumed independent. Option (a) can
be considered only for an unscoped rule shared verbatim; option (b) still needs
Q4's Claude hook qualification, and option (c) still needs Q3's Copilot plugin
qualification. This packet does not establish a selective exclusion or that
Copilot ignores `paths:`. Scoped delivery for both clients remains gated.
Durable records await step 7; the Codex rules row remains blocked by #379.

## Preservation, provenance and residue

The
[initial preservation receipt](evidence/2026-10-24-copilot-claude-rules-reader/preservation.json)
and
[retry receipt](evidence/2026-10-24-copilot-claude-rules-reader/retry-preservation.json)
record unchanged isolated-profile `config.json` bytes and mode `0600`. The
initial check also observed absent `settings.json` and `mcp-config.json`. No
measured configuration required restoration. Authentication stores were not
read, copied or exported. Native session, log and search state may remain; exact
whole-profile restoration is unqualified. No real user profile was selected.

Raw transcripts, fixtures and capture helpers remain outside the repository. The
[private source index](evidence/2026-10-24-copilot-claude-rules-reader/private-source-index.json)
identifies them and their hashes. Selected retained events omit model
identifiers, thinking, usage, unrelated metadata and error text. Public command
receipts explicitly redact the private model selection. Raw-output hashes refer
to complete private bytes, not sanitized retained events.

## Offline validation and limits

Tests were written first. `just lint` failed on the two missing Q1 helper
attributes before implementation, then passed all 42 offline tests. Follow-up
regressions cover complete fixture directives, frontmatter, fresh separated
markers, distinct empty root commits and inherited Git selector isolation.
Classification requires an exact successful positive before either candidate
marker or fallback can yield a conclusion. Mutation checks reject positive-guard
bypass, incomplete frontmatter and inverted candidate instructions.

These tests never call a client and cannot qualify discovery. Linux, user scope,
IDE and hosted surfaces remain unqualified. No plugin, subagent, update, resume,
compaction or production lifecycle route was tested. This bounded Q1 result
promotes no production adapter or complete Gate 6 profile.
