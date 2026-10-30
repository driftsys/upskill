# Q11b native-name calibration implementation plan

**Goal:** Attribute one Copilot native-name skill load without requiring the
catalog namespace to be a model-tool argument.

**Architecture:** Add a separate pure replay function to the existing
qualification script. Keep the historical Q11 parser and classifiers unchanged.
Require captured native history and immutable fixture bytes; never access
credentials or invoke a client from replay.

**Spec:** The user-approved native-name revision described in chat on
2026-10-30, recorded in `../specs/2026-10-30-q11b-native-name-profile.md`.

- [x] Write failing replay tests for a supported native-name positive, wrong
      plugin/path/content/hash, name collisions, session substitution, missing
      or duplicate receipts, tool ordering, disclosure and exact response
      whitespace.
- [x] Add `inspect_copilot_native_positive` in
      `scripts/qualify-plugin-rules.py`; expose a distinct profile and
      affirmative provenance flag, without changing historical parsing behavior.
- [x] Record the complete revised profile and account preflight. Retain earlier
      failures. A historical namespace-to-bare fallback does not pass this
      revision.
- [x] Run focused/offline checks, `just fmt`, then `just verify`; obtain review
      before one fresh Haiku request. Native negatives require the native name
      and exact not-found evidence, with catalog and alias/collision controls
      retained.

Review focus: forged or disconnected native invocation records; hidden second
loads; colliding bare names; omitted stdout or history events; marker
disclosure.

## Result

The pre-native reviewer found two gaps: acceptance of a load before the user
request, and exceptions on missing native record data. Both were reproduced in
failing tests and fixed. [#437](https://github.com/driftsys/upskill/issues/437)
tracks the noncritical review finding; it is fixed in this branch. The reviewer
ran no native clients.

One fresh Haiku request as `stasson` passed the revised positive. The initial
producer passed Python path objects to the JSON-fixture validator, which
correctly rejected them. Replaying the unchanged captured JSON fixture and raw
native stream passed. Preserve both the original producer summary and corrected
offline replay; no native request was repeated. Owner and negative controls
remain unrun, pending their separate capture review and credit budget.
