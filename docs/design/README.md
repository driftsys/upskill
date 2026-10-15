# Design records

Design records describe architecture as implemented. Code and tests establish
current behavior; [ADRs](../adr/README.md) own binding decisions and rationale.
Exploratory future designs remain in [WIP](../wip/README.md). Selected contracts
may be recorded here before implementation when their status is explicit.

- [Generation and installation](generation-pipeline.md)
- [Owned instruction operation planning](owned-instruction-planning.md)

- [Strict captured source manifests](strict-source-manifests.md)

- [Owned whole-file operation planning](owned-file-planning.md)

- [Captured item compilation](captured-item-compilation.md)

- [Mixed owned-artifact operation planning](mixed-owned-artifact-planning.md)

- [Qualified scope roots and advisory writer locking](qualified-scope-locking.md)

- [Qualified destination observation](qualified-destination-observation.md) —
  implemented with APFS and hosted Linux evidence; hosted Windows unsupported
  boundary checked, Windows qualification pending.

- [Qualified source observation](qualified-source-observation.md) — implemented
  as an inactive read-only library capability; hosted Linux and Windows
  scope/compile checks passed at `a393263` on 2026-10-13. Final-head hosted
  checks, new Windows source-method runtime coverage and lifecycle integration
  remain pending.

- [Explicit file permission selection](file-permission-policy.md) — implemented
  as an inactive pure supplied-value policy, consumed by the opt-in file
  executor.

- [File recovery checkpoint model](file-recovery-model.md) — implemented as an
  inactive pure supplied-classification API, consumed by durable file recovery.

- [Qualified file execution and durable recovery](qualified-file-execution.md) —
  implemented as an opt-in library capability; hosted Linux/macOS qualification
  and combined lifecycle integration remain pending.

- [Combined lifecycle state](combined-lifecycle-state.md): opt-in reference
  authority; production commands remain on schema 1.

- [Captured rule desired sets](captured-rule-desired-sets.md): opt-in complete
  compiler outputs bound to explicit installation consumers.
