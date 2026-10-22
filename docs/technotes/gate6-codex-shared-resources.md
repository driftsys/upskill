# Gate 6 Codex shared-skill and runtime-resource observations

Task [#369](https://github.com/driftsys/upskill/issues/369) continues the
bounded [#337](https://github.com/driftsys/upskill/issues/337) campaign and the
[shared-core criteria](../wip/plans/2026-10-19-customization-delivery-reconciliation.md#shared-core-authoring-clarification-324).
All 160 cells remain unqualified; no shared route is adopted and production
stays schema 1.

## Candidate and native context

Current
[official Codex skill documentation](https://learn.chatgpt.com/docs/build-skills)
describes repository `.agents/skills` discovery and explicit skill mentions.
That rolling source supplies a candidate, not qualification of the frozen
binary. The actual experiment used Codex `0.163.0`, model `gpt-6.1-sol` and
provider `openai` on macOS arm64. Five distinct sessions identify the actual
version, model, project and native `read-only` sandbox policy. Commands use
`--no-daemon exec --json` through the normally provisioned isolated profile.
Authentication contents were not inspected or copied; ordinary trust and native
registrations were not changed. Profile use is not user-scope delivery evidence.

One portable, consumption-ready skill contains the common instruction to read
`references/review-procedure.md` relative to its directory. Only the resource
contains the unpredictable response marker. There are no client directives,
variants, scripts or include syntax. The prompt mentions `$<skill-name>`, not a
physical path or the resource marker. Manual guidance defines an absent-skill
fallback and prohibits searching for substitutes.

The entry starts in project A's `.agents/skills`. After missing-resource,
missing-entry and restored controls, the same package moves to project B, a
second fresh Git repository. The projects are siblings; the fixture does not
expose that package in both simultaneously. Broader configured/global/ancestor
sources and other readers were not inventoried or suppressed to force a result.

## Observations and failed expectations

| Control                             | Native skill/resource evidence                                                                              | Response assessment                                                      |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Shared positive                     | One matching fixture metadata item and native `<skill>` expansion; actual relative resource command exits 0 | Final answer matches; whole-message sequence fails because of commentary |
| Resource temporarily missing        | Native skill expansion; actual relative `cat` exits 1 with `No such file or directory`                      | Exact missing-resource fallback; whole-message sequence matches          |
| Entrypoint temporarily missing      | No matching fixture metadata or skill expansion; no read command                                            | Exact absent fallback; no native failed-load or permission-denial event  |
| Shared entry/resource restored      | Native skill expansion; relative resource command exits 0                                                   | Final answer matches; whole-message sequence fails because of commentary |
| Same package relocated to project B | Native expansion identifies B; actual absolute command reads B's relative resource                          | Final answer and whole-message sequence match                            |

The [summary](evidence/2026-10-22-gate6-codex-shared-resources/summary.json)
retains five completed turns, four native skill expansions and four resource
commands: three successful reads and one missing-file failure. Each successful
read returns the exact procedure bytes. Native session records retain the
runtime-expanded skill body and its actual path; these are not generated-file
checks or a claim that a separate skill-reading tool ran.

The original capture compared the entire sequence of `agent_message` events with
one expected response. Shared-positive and restored-shared remain failed under
that strict expectation. Independent native session records identify
`commentary` and `final_answer` phases; all five final-answer phases match. This
additional phase evidence does not rewrite the two failed original expectations
or qualify a complete paired route. A final marker alone would not establish
discovery or resource access.

The absent-entry control observes a changed native context and successful
fallback, with no failed file-load attempt. It does not establish permission
enforcement, exhaustive absence across all sources or duplicate-free discovery.
One matching fixture metadata item is not a global catalog/activation count.

## Preservation, receipts and remaining limits

The
[fixture assessment](evidence/2026-10-22-gate6-codex-shared-resources/fixture-and-assessment.json)
retains before/after manual team files and isolated profile configuration
hashes/modes, all exactly equal. Logical entry/resource bytes and modes also
match. The final package intentionally remains in project B; this is not
restoration of the initial location or an installed upskill lifecycle result.
Temporary missing files were restored in guarded cleanup. The model's four
commands only read the expected fixture resource paths.

Selected
[CLI traces](evidence/2026-10-22-gate6-codex-shared-resources/shared-positive.jsonl)
and
[native session projections](evidence/2026-10-22-gate6-codex-shared-resources/native-session-selections.json)
bind thread/turn identities, metadata, expansions, actual commands, responses
and completion. Native source paths resolve the `/var` and `/private/var`
aliases to the same fixture. Projections omit account identifiers, unrelated
skill metadata, base instructions, provider/accounting state and world state.
Original owned sessions remain at their native profile paths; full exports were
not copied into these receipts.

The
[private source index](evidence/2026-10-22-gate6-codex-shared-resources/private-source-index.json)
and
[retained manifest](evidence/2026-10-22-gate6-codex-shared-resources/receipt-hashes.json)
retain distinct original/selected hashes and helper fingerprints. All seventeen
original manifest entries, five owned session hashes and nine selected file
hashes were checked. Private helpers are not shipped or qualified runners; this
packet is informative data, not product input.

This slice does not test native permission-denial mapping, delegated review
children, wrappers, embedded composition or override execution. Direct/plugin
duplicates, compatibility exclusions, four-client coexistence, user scope,
Linux, IDE, repeated discovery/cache behavior, timeout cleanup and integrated
ownership/update/removal/recovery remain required. Task 2/CD-11 and schema-2
activation remain blocked.
