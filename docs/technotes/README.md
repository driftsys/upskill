# Technical notes

These notes are informative. Requirements and architecture belong in the
[concern reference](../topics/README.md); decisions belong in
[ADRs](../adr/README.md). Note filenames identify their subject. Dates,
revisions, sources and limitations inside each note identify the evidence
snapshot; a stable name does not imply that external research remains current.

## Research

- [Codex resume-only retry: failed positive](gate6-codex-resume-only.md): Q9
  retry on Codex 0.164.0; HTTP 401 blocks the positive and resume remains unrun.
- [OpenCode explicit-path retry](gate6-opencode-explicit-paths.md): Q5, OpenCode
  2.1.4; failed native positive, with downstream controls not run.

- [OpenCode current-route controls](gate6-opencode-current-routes.md): Q5,
  OpenCode 2.1.4; failed native positives, unrun component controls and rejected
  local-directory installation.

- [Claude unscoped rule hook controls](gate6-claude-rule-hooks.md): Q4, Claude
  2.1.308; startup, compaction and absence observations, with resume attribution
  limits.

- [Codex rule lifecycle controls](gate6-codex-rule-lifecycle.md): Q9, Codex
  0.164.0; repeated native bodies in a combined reopened-session/compaction
  window.

- [Copilot plugin component controls](gate6-copilot-plugin-components.md): Q3,
  Copilot 1.0.103; rules component failures at both scopes and bounded agent
  observations.

- [Claude managed-root controls](gate6-claude-managed-root.md): Q6, Claude
  2.1.308; local marketplace, fresh-session edit visibility and measured
  residue.

- [Copilot reader of restricted Claude agents](gate6-copilot-claude-agents-reader.md):
  Q2 catalog and real-child observations; restriction enforcement remains
  unqualified.
- [Copilot reader of Claude rules](gate6-copilot-claude-rules-reader.md) — Q1,
  Copilot 1.0.103; plain rule read, paths variant fallback in a tool-free
  request.

- [Customization delivery choice evidence](evidence/2026-10-23-delivery-choice/assessment.md):
  current skill experiments, verified retained native records and focused
  ownership checks; supports a proposal without claiming production
  qualification.

- [Shared customization roots and client adapters](shared-content-root-evaluation.md):
  alternative catalogs, compatibility exclusions and adoption controls.

- [Customization adapter capability refresh](customization-adapter-evidence.md):
  source-pinned client mapping and packaging evidence.
- [Updater plugin startup probes](updater-plugin-probes.md): partial
  installed-client evidence for issue #285.

- [Agent customization](agent-customization/README.md): client reports,
  standards and schema snapshots.
- [Documentation lifecycle comparison](agent-customization/documentation-refactor.md):
  historical audit and migration rationale.

## Filesystem qualification

- [Filesystem qualification validation](filesystem-validation.md): historical
  scope, source and destination checks.

## Execution and recovery validation

- [Guarded lifecycle application](m1-guarded-lifecycle-application.md):
  first-write backup and executor recovery integration.

- [M1 gate 4 validation evidence — 2026-10-15](m1-gate4-validation.md)
- [Guarded original-state backup validation](m1-guarded-backup-validation.md)

## State and lifecycle validation

- [Gate 6 Codex user rule session lifecycle observations](gate6-codex-user-session-controls.md):
  explicit resume policy, native compaction and failed duplicate-delivery
  control.

- [Gate 6 OpenCode user-configuration rule observations](gate6-opencode-user-rule-controls.md):
  local global configuration, paired removals and manual-settings preservation.

- [Gate 6 Codex user-registration rule observations](gate6-codex-user-rule-controls.md):
  ordinary hook trust, two-project native controls and selected preservation.

- [Gate 6 four-client skill coexistence observations](gate6-four-client-skill-coexistence.md):
  shared and installed-plugin resource routes, paired removals and frozen failed
  calibration controls.

- [Gate 6 Codex shared skills and runtime resources](gate6-codex-shared-resources.md):
  native skill expansion, relative reads, relocation and strict-output failures.

- [Gate 6 Codex project compaction observations](gate6-codex-compaction.md):
  retained native compaction, next-turn rule messages and preservation limits.

- [Gate 6 OpenCode shared skills and runtime resources](gate6-opencode-shared-resources.md):
  native relative-resource reads, missing-file controls and shared-route limits.

- [Gate 6 OpenCode policy-parity observations](gate6-opencode-policy-parity.md):
  child execution and path permission controls with unchanged declarations.

- [Gate 6 OpenCode child controls](gate6-opencode-child-controls.md): actual
  child execution, permission observations and calibration limits.

- [Gate 6 skill coexistence and duplicate observations](gate6-skill-coexistence-controls.md):
  bounded Claude/Copilot controls, failed expectations and helper preservation
  limits.

- [Gate 6 Claude skill-route observations](gate6-claude-skill-controls.md):
  bounded native, development and installed-plugin controls with configuration
  residue.

- [Gate 6 shared-skill and exclusion observations](gate6-shared-skill-controls.md):
  bounded Copilot invocation/resource controls and failed explicit isolation.

- [Gate 6 pinned profiles and reader inventory](gate6-plugin-profile-inventory.md)

- [Customization architecture reconciliation](customization-architecture-reconciliation.md)

- [Gate 6 native instruction discovery qualification](m1-gate6-native-qualification.md)

- [Gate 5 captured rule desired-set validation](m1-gate5-desired-validation.md)
- [Gate 5 selected-source resolution validation](m1-gate5-sources-validation.md)
- [Gate 5 reference reconciliation validation](m1-gate5-reference-validation.md)
- [Gate 5 reference removal validation](m1-gate5-retirement-validation.md)
- [Gate 5 selection validation — 2026-10-16](m1-gate5-selection-validation.md)
- [M1 Gate 5 combined-state validation](m1-gate5-state-validation.md)
- [Gate 5 upsert validation — 2026-10-16](m1-gate5-upsert-validation.md)
- [Per-owner contract validation](m1-owner-contracts-validation.md)
- [Explicit shared-output reconciliation validation](m1-shared-reconciliation-validation.md)
- [Read-only lifecycle inspection validation](m1-lifecycle-inspection.md)
- [Read-only lifecycle preview validation](m1-lifecycle-preview.md)
- [Captured lifecycle command adapter validation](m1-captured-lifecycle-commands.md)
- [Source-backed lifecycle command validation](m1-gate5-orchestration-validation.md)
- [Gate 5 final acceptance evidence](m1-gate5-final-acceptance.md)
- [Lifecycle state origin validation](m1-lifecycle-state-origin.md)

Validation records retain their individual revision and platform limits. They
are not proof of production activation or complete lifecycle qualification. Keep
schema bytes, licenses and provenance manifests together. Preserve captured
evidence and its hashes; update research explicitly when upstream contracts
change. Technical notes and evidence assets are outside the published book.

- [Gate 6 independent plugin fixture preparation](gate6-plugin-rule-probes.md) —
  Offline controls, bounded native installation observations and activation
  blockers.
- [Gate 6 authenticated project rule observations](gate6-authenticated-project-controls.md)
  — Bounded native controls; no profile qualification.

- [Codex resume-only attribution](gate6-codex-resume-attribution.md): Codex
  0.164.0 adds one body per rule on resume, before compaction; recorded history
  only.
