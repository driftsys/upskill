# Explicit shared-output reconciliation implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans
> inline. User delegated decisions; no implementation workers or intermediate
> approval pause.

**Goal:** Produce explicit compiler contracts and plan shared owned outputs
safely through the existing physical planner.

**Architecture:** Keep original APIs restrictive. Add a contracted producer and
opt-in shared upsert, sharing common lifecycle preflight and graph authority.
Validate candidate real ownership/contracts before a private single-artifact
physical projection and remap only physical content metadata.

**Tech Stack:** Rust2024, MSRV1.85, existing dependencies only.

**Spec:** ../specs/2026-10-17-m1-shared-reconciliation.md;
../specs/2026-10-15-m1-gate5-lifecycle-integration.md.

## Global constraints

- No production schema2 writes, native effects, default revisions, ownership
  inference or new writer.
- Preserve unrelated drafts/worktrees, raw identity and complete state/reference
  authority.
- Exact-head reviews/CI; independent whole-branch reviewer, exactly two numbered
  PR reviews, no trials; named merge approval.

## Review focus

- Physical projection must not bypass real immutable-owner binding or let
  synthetic owners escape.
- Missing retained proof and differing source/profile semantics block sharing
  despite equal bytes.
- All-selected updates and partial release preserve graph/consumer/contract
  authority and order.
- Shared instruction neighbors preserve unknown body evidence without
  authorizing changed content.
- Observation, topology, legacy and strict whole-state errors expose no partial
  success.

### Task 1: Explicit captured contract producer

**Files:** create src/lifecycle/desired/contracts.rs,
tests/lifecycle_contract_producer.rs; modify desired/mod.rs exports; durable
docs alongside implementation.

**Interfaces:**
`ContractedDesiredSet { set: DesiredInstallationSet, contracts: Vec<OwnerContract> }`;
`compile_rule_desired_set_with_contracts(request: RuleDesiredSetRequest<'_>, profiles: &[TargetProfile]) -> Result<ContractedDesiredSet>`.

- [ ] Public acceptance examples for exact capture/content/profile binding,
      invalid explicit profiles and audience exclusions. Expected
      compiler-producer assertion failure before implementation.
- [ ] Implement producer using existing compile result and supplied profiles;
      never surface.profile().
- [ ] Run `cargo test --test lifecycle_contract_producer`; expected all pass;
      discriminate guards with assertion controls.

### Task 2: Opt-in shared upsert and physical projection

**Files:** create src/lifecycle/sharing/{mod.rs,artifacts.rs,physical.rs},
tests/lifecycle_sharing.rs; modify lifecycle/upsert.rs shared common
orchestration and lifecycle/mod.rs exports. Keep operations/files/instructions
public guards unchanged.

**Interfaces:**
`plan_shared_upsert(prior: &CombinedState, request: UpsertRequest<'_>, observations: &[ObservedFile]) -> Result<UpsertOutcome>`;
internal artifact reconciliation consumes prior rows, complete
InstallationChange values and changed IDs, returns real candidate artifacts plus
desired values indexed by raw artifact identity; private physical planner
consumes prior/candidate state and observations, returns real content metadata
and PlannedFile values.

- [ ] Examples: two owners new shared file; retained owner add/no-op;
      coordinated update; unknown/unmatched proof; release and complete removal;
      differing bytes/modes; immutable bindings; unrelated malformed rows; exact
      observation requirements; legacy/topology; instruction manual
      text/neighbors/modes/final empty aggregate; real owner/profile roundtrip
      and no projection leaks. Existing plan_upsert must still reject these
      sharing requests. Expected RED at missing shared API/unsupported sharing.
- [ ] Implement common lifecycle orchestration with private shared flag; rebuild
      real rows and contracts before calling private sharing planner. Validate
      full candidate, then require matching agreements at affected multi-owner
      identities.
- [ ] Project each artifact identity to one internal owner, delegate existing
      mixed physical planner, remap exact final keys/normalized content and new
      instruction layout onto real records. Explicitly validate real immutable
      owner bindings first.
- [ ] Run producer/sharing and existing lifecycle/operation test targets;
      expected all pass. Add targeted fault controls for every new authority
      guard and key propagation rule.
- [ ] Update durable design/spec/validation, garden only this exact bounded pair
      after green tests. Run `just fmt`, `just sanity`, `just verify`, and
      `cargo +1.85.0 check --all-targets --locked`; expect exit0.
- [ ] Commit conventional changes, mandatory commit QUICK reviews, independent
      whole-branch review and two numbered PR passes. Fix user-policy
      Small/critical/major findings, retain rationalized deferred issues where
      needed. Push, monitor exact-head hosted CI; request named merge approval
      only when concrete result is ready.
