# Durable lifecycle origin and fresh-state application

> **For agentic workers:** Use superpowers:executing-plans for this bounded
> prerequisite; decisions and approvals are delegated by the user.

**Goal:** Bind later application to the exact initial schema-1 backup and
support fresh initialization without a pre-existing lockfile.

**Architecture:** Add optional strict `StateOrigin` metadata to the
combined-state envelope. Preserve it through existing planners. Add a bound
application entrypoint that validates fresh/migrated provenance, backup evidence
and invariant origin before delegating to the existing qualified
executor/application helpers. No new journal or writer.

**Tech stack:** Existing Rust library; Rust 1.85, no new dependencies.

**Spec:**
[Gate 5 umbrella](../../wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md).

## Acceptance and execution

- [ ] Add failing public tests for strict origin codec/validation and
      preservation through upsert; fresh creation with absent state/backup and
      mode 0600; exact migration bytes/backup binding; later reopened writes
      rejecting a changed backup even when caller supplies its current bytes;
      origin changes and unknown provenance rejected before effects.
- [ ] Add `StateOrigin::{Fresh, Migrated { backup_sha256, backup_length }}` plus
      strict validation and a pure schema-1 byte binding constructor.
      `CombinedState.origin` defaults to `None` and skips serialization when
      absent; typed legacy projection cannot fabricate byte provenance.
- [ ] Preserve origin in all planner-produced combined state. Update existing
      Rust struct literals mechanically with `None`, retaining their serialized
      behavior.
- [ ] Add `BoundSchema2Write` and `apply_bound_schema2_write`, consuming
      optional observed state/backup, complete file plan, next state, locations
      and retained source snapshots. Reject unbound later state, origin changes
      and inconsistent backup evidence. Fresh origin includes a journaled
      backup-absence no-op and private initial state; migrated origin reuses the
      existing backup protocol.
- [ ] Record AD-0038 and concern/validation docs. Keep production
      commands/schema 1 inactive.
- [ ] Run focused tests, just fmt/verify/MSRV, independent review, scoped
      gardening, PR/full review/hosted CI and squash merge before
      command-adapter work.

The active completion lane and Gate 5 umbrella remain WIP after this
prerequisite.
