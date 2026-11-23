# Stale documentation inventory

Date: 2026-11-23. Status: **working note for the roadmap and `docs/wip/` cleanup
session**. Discard or archive it when the items below are resolved.

Source: a read-only sweep of the shipped documentation against `src/` at
`origin/main` (4c07051). `docs/wip/`, `docs/archive/` and superseded ADRs were
out of scope. Items 1 and 3 were checked by hand.

## Checked and correct

- Output is copied, never symlinked: AGENTS.md, specification §3.2,
  `docs/book/commands.md`, `skills/upskill-cli/SKILL.md`.
- Per-client override files are documented as removed (#580): topics/0001,
  ADR-0002.
- upskill writes only `CLAUDE.md` (`@AGENTS.md`), never `AGENTS.md`:
  `docs/book/support-levels.md`, specification.
- Codex is documented as unsupported: `docs/book/support-levels.md`,
  `skills/upskill-cli/SKILL.md`.

## Stale or false statements

| # | Location                                                                            | Statement                                                                                                                                                           | Why it is false                                                                                                         | Proposed fix                                                                                 |
| - | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| 1 | `docs/topics/0001-content-format.md` §7, lines 1224-1238                            | "one shared always-on Claude/Copilot file when both are selected, with per-item reader consent and no second Copilot copy"; "Explicit OpenCode paths await Q5 PASS" | ADR-0042 replaced this plan. Each client gets its own copy (ADR-0041). The Q-gates are withdrawn.                       | Delete the paragraph, or replace it with one sentence that cites ADR-0041 and ADR-0042.      |
| 2 | `docs/topics/0001-content-format.md` lines 1221-1226                                | "Planned ancillary transition" block                                                                                                                                | Planned work under the withdrawn ADR-0040 gating, inside a shipped document.                                            | Remove it, or move it to `docs/wip/` if the transition is still wanted.                      |
| 3 | `docs/specification/specification.md` lines 1048-1056                               | "The separate step-8 code PR removes bridge creation and the OpenCode glob write only after Q5 explicit-path PASS"                                                  | Same withdrawn plan. The bridge and the OpenCode glob write are shipped and kept.                                       | Remove the block. Decide in the roadmap session whether the bridge removal is still planned. |
| 4 | `docs/topics/0009-customization-delivery.md` lines 30-60, 104-135, 255-440, 470-560 | Present-tense Codex and OpenCode delivery routes, "four-client MVP release gate" (line 159), "current four-client support range" (line 390)                         | Only a header banner says ADR-0042 replaced them. The code has three clients; Codex is unsupported (`src/support.rs`).  | Mark each withdrawn section, or move the file to `docs/technotes/` as history.               |
| 5 | `docs/topics/0002-generation.md` lines 94, 321, 371                                 | Codex qualification "pending", `compile_codex_instructions`                                                                                                         | Other Codex sections carry "Withdrawn 2026-11-11" banners; these lines may sit outside them. The code is not in `src/`. | Check each line for a banner; add one or delete.                                             |
| 6 | `docs/topics/0001-content-format.md` lines 37-39                                    | Lists Codex with the clients in prose                                                                                                                               | Codex is an `audience` identifier only (`src/model/common.rs`), not a generation client (`src/generate/mod.rs`).        | Add "Codex is an audience identifier only".                                                  |
| 7 | `skills/upskill-prompt-design/SKILL.md` lines 67-75                                 | "Client capability matrix (v0.1.0)"                                                                                                                                 | Version label is stale; cells not verified.                                                                             | Re-verify the cells and drop the version label.                                              |
| 8 | `skills/README.md` line 67                                                          | Install table lists Codex CLI, Gemini CLI, Cursor                                                                                                                   | These are third-party (Superpowers) install commands, not upskill clients.                                              | Confirm the table is labelled as third-party.                                                |

## Stubs and duplicates to decide on

- `docs/commands.md`, `conventions.md`, `format-spec.md`, `getting-started.md`,
  `intro.md`, `recipes.md`, `specification.md` at the `docs/` root: 5-line
  "Documentation moved" redirects to `docs/book/`. Intentional
  (`docs/README.md`); remove only if the old URLs are no longer promised.
- `docs/design/*`: stubs; all pages except `generation-pipeline.md` describe
  withdrawn library code.
- `docs/topics/0005` to `0008`: fully withdrawn history kept in `topics/`.
  Candidates for `docs/archive/` or `docs/technotes/`.
- `docs/specification/format-spec.md` anchors for removed override files:
  compatibility anchors (`docs/compatibility.json`); keep.

## Not covered by the sweep

- Status lines and current-behavior statements of active ADRs (0013, 0039, 0040,
  0041 and others).
- `docs/technotes/` beyond keyword search.
- A full read of `docs/book/commands.md`, `docs/specification/specification.md`
  and topics 0002 to 0004; only keyword matches were read.
- Whether `pack` is described as experimental everywhere it is documented.
