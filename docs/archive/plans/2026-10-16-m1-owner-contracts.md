# Per-owner contract evidence implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> inline under the accepted Gate 5 delegation.

**Goal:** Persist and validate per-owner contracts and expose conservative
agreement without enabling shared mutation.

**Architecture:** Keep evidence inside CombinedState. Bind normalized
content/source/profiles to existing owner associations. Propagate explicit
evidence through pure upsert/retirement and preserve unknown history. All
existing writer and sharing rejections remain.

**Tech Stack:** Rust 2024, Rust 1.85, existing serde/anyhow only.

**Spec:** ../specs/2026-10-16-m1-owner-contracts.md;
../../wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md.

## Global constraints

- No new dependencies, CLI/schema-1 changes, native effects or qualification
  claims.
- Exact raw roots; no ownership adoption or inferred revisions.
- Keep other WIP and worktrees intact.

## Review focus

- Unknown prior evidence cannot become version 1 by default.
- Equal bytes cannot mask differing source semantics or target contracts.
- Changed selected metadata cannot leave stale evidence attached.
- Unselected shared contracts survive retirement and manual aggregate edits.
- Duplicate, extra and partially mismatched evidence fails without partial
  output.

### Task 1: Recorded evidence, agreement and lifecycle propagation

**Files:** Create src/lifecycle/contracts.rs and tests/lifecycle_contracts.rs;
update lifecycle/state/mod.rs, validate.rs, codec.rs; lifecycle/upsert.rs and
retire/mod.rs; initialize empty contract vectors in existing constructors/test
fixtures; add durable design/specification/validation records.

**Interfaces:**

- Consumes CombinedState, OwnedArtifact, SourceEvidence, OwnerConsumers,
  UpsertRequest.
- Produces ConsumerContract { surface: TargetSurface, contract_revision: u32 },
  ContractContent::File { content_sha256: String, mode: Option<u32> } /
  Instruction { content_sha256: String }, OwnerContract { root, path,
  contribution_id, installation_id, source_manifest_sha256, content, profiles },
  ContractAgreement::Matching / Blocked(Vec<ContractConflict>),
  check_contract_agreement(state, root, path, contribution_id) ->
  Result<ContractAgreement>.
- Adds CombinedState.owner_contracts: Vec<OwnerContract> with serde
  default/skip-empty and UpsertRequest.contracts: &[OwnerContract].

- [ ] Write public examples for missing and complete evidence, source/profile
      disagreements, exact bindings, serialization defaults/order, instruction
      body metadata, upsert replace/clear/untouched propagation, and shared
      retirement.

```rust
assert_eq!(parsed.owner_contracts, Vec::new());
assert!(matches!(check_contract_agreement(&state, &root(), "same.md", None)?, ContractAgreement::Blocked(_)));
```

- [ ] Run `cargo test --test lifecycle_contracts`; expected clean
      missing-interface RED.
- [ ] Implement records/validation/checker; update explicit constructor fields
      with empty defaults; propagate complete selected evidence through upsert
      and filter retirement evidence.
- [ ] Run `cargo test --test lifecycle_contracts`; expected all examples pass.
      Add assertion controls for missing evidence, source/profile mismatches,
      bindings, stale proof, retained proof and deterministic encoding,
      restoring exact sources after each.
- [ ] Update durable design/spec/usage, garden only this bounded spec/plan, run
      just fmt, just sanity, just verify and Rust1.85 locked all-target checks;
      expected exit0.
- [ ] Commit `feat: record per-owner content and target contracts`, independent
      whole-branch review, required QUICK/numbered PR reviews and hosted checks.
      Request specific PR merge approval only after concrete delivery is ready.
