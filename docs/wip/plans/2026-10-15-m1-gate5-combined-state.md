# Gate 5 combined state implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> to implement this plan task by task. Steps use checkbox syntax for tracking.

**Goal:** Deliver the pure combined-state foundation for Gate 5, including
authoritative installation references, per-owner consumers and lossless typed
legacy retention.

**Architecture:** Add an opt-in `lifecycle::state` library module. A strict
schema-2 envelope contains the authoritative graph and existing owned-artifact
values; legacy rows retain original array order and labels. No production
dispatcher or filesystem writer consumes this module yet.

**Tech Stack:** Rust edition 2024, Rust 1.85, existing serde/serde_json and
sha2; no new dependencies.

**Spec:**
[Gate 5 lifecycle design](../specs/2026-10-15-m1-gate5-lifecycle-integration.md).

## Global constraints

- Keep `lockfile::CURRENT_SCHEMA` at 1 and current command behavior unchanged.
- Preserve unresolved legacy rows without adopting their outputs or fabricating
  requested selection.
- Only explicit new M1 rule/instruction installations become owned through later
  planning and executor slices.
- No file I/O, source fetch, migration backup or journal in this slice.
- All validation is lexical and pure; it supplies no containment or native
  support evidence.
- Keep original unknown schema-1 JSON fields and exact bytes outside the typed
  projection; the later first-write backup preserves those bytes.
- Every task includes acceptance tests, focused regressions and documentation.
  Tests use library calls because the production path remains inactive.

## Review focus

1. An owner appearing in an artifact must exactly match its authoritative
   installation binding, including revision; conflicting duplicates fail.
2. Bundle cycles, dangling references and unreachable installations must fail
   rather than implicitly retain or delete content.
3. Union consumer metadata must equal explicit per-owner associations; never
   assign another owner's consumers by inference.
4. Audience exclusions may leave an explicitly referenced installation with no
   artifact; graph validation must allow this without losing its requested
   surfaces.
5. Legacy duplicates, malformed source labels and native statuses must survive
   codec round trips without sorting or normalization.

## Files and boundaries

- Create `src/lifecycle/mod.rs`: opt-in module documentation and
  `pub mod state`.
- Create `src/lifecycle/state/mod.rs`: public envelope and graph types; no
  effects.
- Create `src/lifecycle/state/validate.rs`: graph and artifact consistency.
- Create `src/lifecycle/state/codec.rs`: strict decode, deterministic encode and
  typed legacy projection.
- Modify `src/lib.rs`: expose `pub mod lifecycle` only.
- Create `tests/lifecycle_state.rs`: public acceptance tests.
- Create `docs/design/combined-lifecycle-state.md`: implemented format and
  limits.
- Update `docs/specification/specification.md` and `docs/wip/README.md`: opt-in
  boundary and active plan.

## Chosen state model

All new records derive serde with unknown-field rejection; enums use explicit
kebab-case tags. Reuse `ArtifactOwner`, `InstallationRoot`, `TargetSurface`,
`OwnedArtifact` and the schema-1 `Lockfile` types. Do not change their existing
wire representations.

- `CombinedState { schema: u32, installations: Vec<Installation>, bundles: Vec<BundleNode>, references: Vec<Reference>, artifacts: Vec<OwnedArtifact>, owner_consumers: Vec<OwnerConsumers>, legacy: Option<Lockfile> }`.
- `Installation { owner: ArtifactOwner, root: InstallationRoot, requested: Vec<TargetSurface>, source_evidence: SourceEvidence }`.
- `SourceEvidence::Git { resolved_revision: String, manifest_sha256: String }`
  and `SourceEvidence::Local { manifest_sha256: String }`. Persist captured
  manifest identity; execution consumes the independently validated complete
  capture, not this digest alone. Local ownership revision is absent; Git
  revision equals the authoritative owner revision. Resolved revision is
  required, but no authenticity claim follows from a string.
- `BundleNode { id: String, source: String, name: String, source_revision: Option<String> }`.
- `Reference::DirectInstallation { installation_id: String }`,
  `Reference::DirectBundle { bundle_id: String }`,
  `Reference::BundleInstallation { bundle_id: String, installation_id: String }`,
  `Reference::BundleDependency { bundle_id: String, dependency_id: String }`.
- `OwnerConsumers { root: InstallationRoot, path: String, contribution_id: Option<String>, installation_id: String, consumers: Vec<TargetSurface> }`.
  One association per artifact selector and owner; its surfaces must be a subset
  of that owner's requested surfaces. Its exact union must equal the existing
  artifact's consumers.

Identifiers are caller-supplied nonempty ASCII alphanumeric, hyphen or
underscore values, unique within each node kind. Reject conflicting identities
rather than merging them. Later lifecycle identity allocation is a separate
slice. Bundle and installation namespaces are explicitly typed, so equal strings
across the two kinds are not collisions.

## Task 1: Public combined-state records and graph validation

**Interfaces:** `validate_state(state: &CombinedState) -> anyhow::Result<()>`.

- [ ] Write public acceptance tests
      `rejects_dangling_duplicate_and_cyclic_references`,
      `preserves_shared_installation_references`, and
      `allows_referenced_audience_excluded_installation`. Construct two directly
      requested bundles referencing one rule, plus a bundle dependency. Assert
      valid sharing, then separately corrupt each edge kind and assert rejection
      without input mutation.
- [ ] Run `cargo test --test lifecycle_state`; require a
      missing-module/interface failure before implementation.
- [ ] Add the public types and validator. Require schema exactly 2, unique IDs,
      nonempty source/name values, valid absolute roots, unique nonempty
      requested surfaces, valid 64-character lowercase manifest hashes, and
      consistent source-evidence revision bindings. Check all edge endpoints,
      reject duplicate edges and bundle cycles, and require every node reachable
      from a direct reference. Reuse `ownership::validate_artifacts` for
      artifact shape validation.
- [ ] Add focused tests for each invalid field, self-cycles, transitive cycles,
      disconnected nodes, multiple direct references, and byte-identical
      conflicting owner identities. Referenced installations without artifacts
      remain valid.
- [ ] Run `cargo test --test lifecycle_state`; require all tests passing.
- [ ] Document graph authority and the absence of effects in the design and
      specification; review the task, then commit as
      `feat: validate combined lifecycle state references`.

## Task 2: Exact owner bindings and consumer associations

**Interfaces:** Extend `validate_state`; no second authority or independent
validator that callers can bypass.

- [ ] Write `rejects_artifact_owner_binding_drift` and
      `requires_exact_per_owner_consumer_union`. Assert rejection for changed
      revision/root/alias, missing or duplicate associations, an unknown
      selector, extra consumers, and consumers outside requested surfaces.
      Assert valid independent owners with different consumers on one unchanged
      artifact.
- [ ] Run the two focused tests and require failure for the unimplemented
      checks.
- [ ] Validate each artifact owner against its installation and root. Require
      exactly one association per owner/selector, no unreferenced association,
      nonempty unique consumers, and exact union equality. Whole-file selectors
      use `None`; Markdown selectors use the recorded contribution ID. Preserve
      all input vectors and raw root spellings.
- [ ] Add regressions for same path under different roots/scopes, multiple
      contributions at one path, and identical owner IDs with different aliases
      or source evidence.
- [ ] Run `cargo test --test lifecycle_state`; require all tests passing.
- [ ] Update the design with the association contract, review this task, and
      commit as `feat: validate lifecycle owner consumer associations`.

## Task 3: Strict codec and typed schema-1 retention

**Interfaces:** `parse_state(json: &str) -> anyhow::Result<CombinedState>`,
`serialize_state(state: &CombinedState) -> anyhow::Result<String>`, and
`project_legacy(lockfile: &Lockfile) -> anyhow::Result<CombinedState>` in
`lifecycle::state`.

- [ ] Write `round_trip_preserves_legacy_rows_labels_and_statuses`,
      `rejects_unknown_schema2_fields`, and
      `serialization_is_deterministic_without_input_mutation`. Include duplicate
      legacy item rows, malformed source labels, bundles, plugins with every
      status, and MCP rows. Assert typed equality after round trip and unchanged
      production `CURRENT_SCHEMA == 1`.
- [ ] Run these tests; require missing codec failures before implementation.
- [ ] Implement strict schema-2 parsing followed by `validate_state`. Reject
      unsupported versions and unknown fields in new schema-2 records. Retained
      legacy uses the existing typed schema-1 format; its unknown raw fields are
      deliberately not reconstructed. Require legacy schema exactly 1 but
      preserve its unresolved diagnostics and duplicates.
- [ ] Implement projection with empty owned graph/artifacts and
      `Some(lockfile.clone())`; never create ownership from labels or hashes.
      Serialize a validated clone as pretty JSON with one trailing newline. Sort
      graph nodes/edges, associations, artifact selectors, owner lists and
      consumer lists deterministically using explicit variant/location keys. Do
      not sort or normalize any retained legacy array or source label.
- [ ] Add tests for source-label punctuation, pins, absent optional fields,
      legacy unknown-field limitations, invalid UTF-8 input handling by callers,
      schema-1 supplied to the schema-2 parser, empty combined state, and
      permutation-equivalent owned graphs. Ensure whole-file and contribution
      metadata survive exactly.
- [ ] Run `cargo test --test lifecycle_state` and related existing
      ownership/legacy tests; require all passing.
- [ ] Record codec examples and evidence limits in durable docs; review the
      task, then commit as
      `feat: encode combined lifecycle state with retained legacy rows`.

## Slice completion

- [ ] Confirm every review-focus item has a named test and each state
      requirement maps to a task above.
- [ ] Run `just fmt`, then `just verify`; require zero warnings and passing
      tests.
- [ ] Run `cargo +1.85.0 check --all-targets --locked`; require success.
- [ ] Review the complete slice, classify findings using the user's
      severity/effort policy, and address immediate fixes.
- [ ] Garden only this completed slice's working documents into durable records.
      Preserve the active Gate 5 umbrella and unrelated WIP.
- [ ] Open one slice PR; monitor hosted CI and complete review before merge. Do
      not claim Gate 5 complete.

## Remaining Gate 5 plans

After this slice's reviewed interfaces land, prepare bounded plans in dependency
order:

1. Complete compiler-to-desired-set reconciliation, selection conflict
   aggregation and shared-owner planning.
2. Guarded exact first-write backup and executor integration, with
   backup/output/final-state interruption tests and stale evidence rejection.
3. Observational list/doctor/preview and end-to-end lifecycle acceptance,
   including extra resource retention and inactive production dispatch.

These later slices consume the reviewed state interfaces. This plan does not
invent their exact executor extensions before the state and reference model is
reviewed.
