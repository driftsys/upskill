# Technical notes

Technical notes are informative. They preserve investigations, comparisons and
implementation explanations without establishing new product requirements.
External research records its date, sources, uncertainty and version limits.
Accepted choices belong in [ADRs](../adr/README.md); contracts belong in
[specifications](../specification/README.md).

- [Agent customization research, 2026-10-11](2026-10-11-agent-customization/README.md):
  six first-class design targets, eight customization categories, official
  documentation and schema evidence.
- [Customization adapter capability refresh, 2026-10-14](2026-10-14-customization-adapter-evidence.md):
  six-client mapping, Gemini/Cursor capabilities, VS Code runtime boundaries,
  and source-pinned Superpowers packaging examples.
- [Updater plugin startup probes, 2026-10-14](2026-10-14-updater-plugin-probes.md):
  partial installed-client evidence for the first updater entrypoints in MVP
  issue #283.
- [RIDL documentation lifecycle comparison](2026-10-11-agent-customization/documentation-refactor.md):
  the investigation behind this repository's documentation organization.

Keep schema bytes, licenses and provenance manifests together. Update dated
research explicitly or add a successor note when upstream contracts change.
These files are not part of the published book or runtime dependencies.

- [Per-owner contract validation, 2026-10-16](2026-10-16-m1-owner-contracts-validation.md):
  supplied-value examples, fault controls and qualification limits.

- [Explicit shared-output reconciliation validation](2026-10-17-m1-shared-reconciliation-validation.md):
  contracted production, private projection examples and supplied-value limits.
