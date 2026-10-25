# Q11 cross-client package isolation implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for
> tracking.

**Attempt disposition:** The offline harness and the first failed-positive
packet are implemented and verified. Both target positives are blocked, so the
absence baselines and foreign-package controls remain unrun. This plan stays
active for the unfinished Q11 controls; it is not an archived claim of completed
qualification. The retained attempt is described in the
[technote](../../technotes/gate6-cross-client-package-isolation.md).

**Goal:** Retain bounded evidence for Claude reading a Copilot package and
Copilot reading a Claude package, under issue #410.

**Architecture:** Extend the existing qualification harness with offline
fixtures and strict classification. Capture native evidence privately, then
retain sanitized receipts and a technote. No production routing changes.

**Tech Stack:** Python standard library, existing offline unittest suite, native
Claude and Copilot CLIs on macOS arm64.

**Spec:**
[Implementation handoff](../specs/2026-10-23-customization-delivery-implementation-spec.md),
Q11, and [issue #410](https://github.com/driftsys/upskill/issues/410).

## Global constraints

- Fresh four-client campaign pins before execution; only Claude and Copilot
  execute Q11 controls. Q5 remains deferred.
- Neutral fixture outside discovery roots; no team instruction files, native
  cache writes, copied credentials or existing discovery-setting changes.
- Same content bytes and logical item identity in each target-native positive
  and its foreign-package candidate. Record actual package and runtime paths.
- Positive native catalog and namespaced invocation before each negative. Failed
  positive blocks the target result.
- Use documented explicit package selectors. An unavailable selector is
  unavailable, not an isolation PASS. Conversion or installation that activates
  foreign content is activation, not isolation.
- Restore measured profile bytes and modes after native cleanup; retain residue
  separately. Raw source and model selection remain private.
- Q10/#409 has separate evidence. Independent worktrees permit parallel offline
  work. Native phases must not mutate a profile concurrently: use separately
  provisioned authenticated profiles or serialize access to the shared profile.
- No production route, schema activation or durable architecture amendment.

## Review focus

- Failed positive, missing catalog evidence or incomplete invocation must never
  become an isolation PASS.
- Shared content paths, mismatched bytes or identities can invalidate a paired
  control even when the final response looks correct.
- Existing broad imports and compatibility readers remain active and must be
  inventoried before running a candidate.
- Native conversion or implicit installation can activate foreign content and
  must be recorded as FAIL rather than selector absence.
- Interrupted cleanup and concurrent Q10 registration changes must be retained
  as preservation failures, not overwritten by snapshot restoration.

## Task 1: Offline paired fixtures and classification

**Files:** Modify `scripts/qualify-plugin-rules.py`; create
`scripts/tests/test_qualify_cross_client_isolation.py`.

**Interfaces:**

- `prepare_cross_client_isolation(root, *, pins)` returns target-keyed fixture
  records for `claude` and `copilot`, including native and foreign package
  paths, intended client, logical skill identity, exact content SHA-256,
  positive marker, absent marker and distinct disposable project paths.
- `classify_cross_client_isolation(observation, fixture)` returns separate
  `isolation` and `preservation` fields. Isolation is one of `blocked_positive`,
  `invalid_pair`, `selector_unavailable`, `incomplete_evidence`,
  `activation_observed`, `control_metadata_failed`, `catalog_exposure`, or
  `isolation_observed`. Preservation is `unchanged` or `preservation_failed`.
  `isolation_observed` is a bounded result, not general support.
- Observation includes pin/platform, selector route and availability, content
  hashes and identities, positive catalog/invocation success, candidate catalog
  and invocation evidence, activation evidence and preservation status.

- [x] Write failing offline tests for byte/identity equality, neutral-root
      rejection including dangling symlinks, distinct projects and unexpected
      discovery paths. Do not create team guides even in tests.
- [x] Write classification tests for each return value. Assert that missing
      catalog alone, a failed invocation or an unmatched fallback cannot pass;
      observed activation fails even when catalog presence is false. Require
      positive completion and paired integrity before interpreting negatives.
- [x] Run `just lint`; retain failure from the missing Q11 interfaces.
- [x] Implement fixtures using the existing neutral-root and environment guards.
      Use the existing Claude and Copilot manifest conventions; vary package
      envelope and selector only, preserving the logical content bytes.
- [x] Implement classification without native calls or transcript mutation.
- [x] Run the focused test file, then `just lint`; retain passing output.

## Task 2: Native controls and retained packet

**Files:** Update `docs/technotes/gate6-plugin-profile-inventory.md` and
`docs/technotes/README.md`; create
`docs/technotes/gate6-cross-client-package-isolation.md` and
`docs/technotes/evidence/2026-10-25-cross-client-package-isolation/`. If
execution occurs on a later date, use that date for the evidence directory.
Private native capture helpers remain outside the repository.

**Interfaces:** Consume Task 1 fixtures and classifier. Produce target-specific
receipts, indexed summary, validation, restoration/residue evidence, receipt
hashes and private-source index in the existing packet format.

- [ ] Record four installed client pins, platform and documented selectors from
      official client documentation and installed help. Inventory existing broad
      imports/globs and compatibility readers without changing them.
- [ ] Reserve exclusive use of any profile shared with Q10. Snapshot measured
      settings and registration bytes, modes and presence; retain source hashes.
- [ ] For each target, register its native positive using supported commands,
      capture native catalog and exact namespaced invocation, then clean up the
      fixture registration. Stop that target on a failed positive.
- [ ] Point the target at the foreign package through its documented selector.
      Record unavailable setup without inventing configuration. Capture catalog,
      explicit invocation, actual paths and any installation/conversion
      behavior.
- [ ] Classify each target separately. Remove only fixture-owned registrations
      through native commands, inventory residue and restore measured state only
      after confirming unrelated state was preserved. Retain failures unchanged.
- [ ] Write the technote with exact client, package shape, selector and scope
      limits, separate Q10 status, indexed summary and replayable classification
      tests against sanitized retained observations.
- [ ] Run `just fmt`, then `just verify`; review the complete diff and packet
      hashes before committing and opening one task PR. Do not merge without the
      required review and authorization.

## Accepted review corrections and execution decisions

The user authorized execution after the independent plan review. The binding
issue criteria take priority over suggestions that broaden Q11 into Q10.

- Research selectors before fixtures: installed help documents `--plugin-dir` on
  both clients. The first bounded profile uses this session-only selector for
  native positive and foreign candidate. Marketplace/install selectors are
  recorded as not run, not covered. Both packages keep the same plugin and skill
  names and identical skill-tree bytes and modes. Different targets use
  independent nonces, paths and projects. Claude uses its Q6 manifest shape;
  Copilot uses the Agent Plugins 1.0 root manifest.
- The selected scope is a fresh process in a disposable trusted project, with
  normally provisioned isolated user authentication. This is a narrower selector
  control than persisted project/user registration; it qualifies neither Q10 nor
  complete slice-2 package ownership. No skill component is substituted by an
  agent or rule control.
- After the native positive exits, run a selector-free catalog and explicit
  invocation baseline in the candidate project. Require absence and no skill
  activation before applying the foreign selector. A failed positive or baseline
  blocks that target. Raw untrimmed results and native identities are retained.
- Capture process completion, parsed stream completeness, pin, platform and cwd.
  Verify package tree fingerprints before and after native phases. Enumerate
  required catalog/invocation evidence in the classifier and its tests.
- Classifier order: metadata and pair integrity, positive and baseline gates,
  observed activation, foreign catalog exposure, unavailable selector,
  incomplete evidence, bounded isolation. Activation wins over missing catalog
  evidence. Catalog exposure is retained separately as failure of isolation,
  without equating exposure with execution. A documented selector rejection is
  distinct from no documented selector.
- Preservation is a separate computed field. Hash and mode receipts compare
  measured state; unexpected differences fail preservation independently of the
  isolation outcome. Session-only selection permits no settings residue and does
  not require a new Copilot settings-residue exemption. Auth-bearing
  configuration bytes remain private and are never exported.
- First executing task owns its campaign revision. Q11 records a separate dated
  revision with exact command receipts; Q10's existing campaign remains intact.
  Native phases run serially under a private OS advisory profile lock shared
  between this task's processes. Preflight also checks other active native
  controls; an advisory lock cannot exclude a nonparticipating process.
- Q10 registrations are pre-existing state and never cleanup targets. Existing
  discovery settings remain enabled. Audit native and compatibility skill roots,
  manifests and broad selectors before choosing the neutral root.
- Claude authentication is known to have failed in Q10. No credential copying,
  trust bypass or silent profile substitution is permitted. Retain a failed
  positive or preflight blocker without claiming a negative PASS.
- Replay tests precede any new replay helper and retain both isolation and
  preservation outcomes. State unexercised selectors/readers, Codex/OpenCode and
  package profiles explicitly in the technote.

## Post-implementation review corrections

- Separate process completion, explicit invocation attempt and native
  resolution. A not-found response must correlate to one exact namespaced native
  request; stream completion and a prompted fallback marker alone are
  insufficient.
- Record any observed fixture-marker disclosure, bare-name content load and
  selector installation/conversion as activation. Preserve raw response text;
  exact marker matching still governs native positive calibration.
- Distinguish `baseline_contaminated` from `blocked_baseline` and
  `blocked_positive`. Fingerprint complete package envelopes for future runs.
- Retain the original measurement limitation: Claude `.claude.json` and managed
  settings were not snapshotted before this attempt. After-only residue cannot
  establish which files changed. Do not relabel the old preservation receipts.
- The original native fixtures lack before-envelope hashes; the reviewed builder
  captures them for future controls. Both original classifications remain
  blocked.
