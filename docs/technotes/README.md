# Technical notes

These notes are informative. Requirements and architecture belong in the
[concern reference](../topics/README.md); decisions belong in
[ADRs](../adr/README.md). Note filenames identify their subject. Dates,
revisions, sources and limitations inside each note identify the evidence
snapshot; a stable name does not imply that external research remains current.

## Research

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
