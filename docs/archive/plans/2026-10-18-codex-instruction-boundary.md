# Captured Codex instruction boundary implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development to
> implement this plan task by task. Routine decisions and delivery are
> delegated.

**Goal:** Compile the restricted Codex instruction subset with mandatory reader
validation and separate exposure metadata, without production activation.

**Architecture:** Add a family-aware source vocabulary while preserving frozen
generation clients. A pure Codex producer derives versioned location contracts,
validates captured source bindings and returns intentional ownership separately
from potential readers. Existing captured lifecycle commands provide acceptance
coverage; source orchestration remains a later slice.

**Tech stack:** Existing synchronous Rust crate and dependencies, edition 2024,
MSRV 1.85. No dependency additions.

**Spec:** `docs/wip/specs/2026-10-18-codex-instruction-boundary.md`.

## Global constraints

- Task #322; base main `d8bd822810cf122fd070f95fa72a4e1497948089`.
- Work only in `.claude/worktrees/codex-instruction-boundary`; bootstrap ran.
- Only `main.rs` writes production stdout/stderr. New compiler code is pure.
- Do not extend generation `Client`, legacy layouts, CLI flags or defaults.
- Production schema stays 1; no source-orchestration adapter integration.
- Versioned location contract and intentional target revision are exactly 1.
- All five project/two global readers are mandatory potential readers; only
  explicit Codex CLI is intentional. Native qualification is never inferred.
- Reuse existing manifest, state, naming, ignore and Markdown validation.
- Reject unsupported semantics with typed capability errors; never flatten
  scope.
- Code, tests and docs ship together after full review and hosted CI.
- Use `CARGO_TARGET_DIR=/private/tmp/upskill-gate6-target` for Rust checks.

## Review focus

Check incomplete reader inventories, audience exclusion with equal bytes,
identity changes during update, binding substitutions and accidental native
qualification claims. Complete operation conflict aggregation remains an
activation requirement rather than a claim of this producer.

### Task 1: Family-aware source audience and directives

Own `src/model/common.rs`, `src/generate/directives.rs` and focused tests in
those modules. Add `Audience::Codex` and strict family conversion/parsing needed
by `process_for_family(body: &str, target: TargetFamily) -> Result<String>`.
Keep existing `process(body, Client)` as a byte-compatible wrapper. Unsupported
Cursor/Gemini targets and unknown directive identifiers remain errors. Extend
only tightly related helper sites if compilation requires them.

- [ ] Write failing examples for Codex audience parsing, positive/negative/mixed
      family directives and unknown/nested/unbalanced input. Pin that
      `Client::ALL` and parsing still contain only the historical three
      generation clients.
- [ ] Run focused tests, observe behavioral or compile RED, implement minimally.
- [ ] Run directive/model tests and existing generation golden tests. Expected:
      all historical bytes remain unchanged; Codex blocks are filtered
      correctly.
- [ ] Commit with a Conventional Commit and write a task report.

### Task 2: Pure Codex producer, reader report and lifecycle acceptance

Own new `src/lifecycle/desired/codex.rs`, its export in `desired/mod.rs`,
`tests/codex_instruction_boundary.rs`, and feature-scoped documentation. Do not
remove existing source orchestration's instruction-owner rejection. The existing
instruction contribution validator may be exposed within the crate in
`src/instructions/aggregate.rs` for direct reuse; preserve its behavior.

Public interface in the new module:

```rust
pub struct CodexInstructionRequest<'a> {
    pub installation: &'a Installation,
    pub source: &'a SourceManifest,
    pub folder_name: &'a str,
    pub profile: TargetProfile,
    pub location: &'a InstructionLocation,
}
pub enum InstructionReaderRole { Requested, PotentialIncidental }
pub struct InstructionReader {
    pub profile: TargetProfile,
    pub role: InstructionReaderRole,
}
pub enum InstructionCompatibility { NotEvaluated, UnconditionalMarkdown }
pub struct InstructionReaderReport {
    pub location: InstructionLocation,
    pub contract_revision: u32,
    pub readers: Vec<InstructionReader>,
    pub compatibility: InstructionCompatibility,
}
pub struct CompiledCodexInstructions {
    pub desired: ContractedDesiredSet,
    pub readers: InstructionReaderReport,
}
pub fn compile_codex_instructions(
    request: CodexInstructionRequest<'_>,
) -> anyhow::Result<CompiledCodexInstructions>;
```

Use derives consistent with surrounding public types. Mandatory reader profiles
come from this module's reviewed location contract, not caller-selected readers.
Choose deterministic surface-name ordering. The contribution ID is lowercase
SHA-256 of the JSON tuple `("upskill-codex-contribution-v1", installation_id)`;
do not include body, alias or revision separately. No new cross-owner sharing.

- [ ] Start with the dedicated integration test target and representative RED
      examples before implementation. Missing APIs may provide the first compile
      RED, followed by behavioral negative examples.
- [ ] Cover generic project acceptance with exactly five report readers and only
      Codex CLI ownership; Codex-only project audience rejection even with equal
      body; reader-divergent directives rejection; Codex-only global acceptance
      with exactly two report readers; stable ID and changed digest after body
      update.
- [ ] Cover ignored/effective resources, nonempty scope, override/native fields,
      mismatched manifest/name/root, unsupported revisions, intentional
      IDE/multiple selection and non-native/custom/external destination paths.
      Cover an excluded Codex result without an artifact or a false
      compatibility claim. Empty/unrepresentable bodies and managed-marker
      injection must fail before a desired contribution is returned.
- [ ] Implement pure binding validation, canonical body processing, capability
      rejection, complete desired contribution, explicit owner contract and
      report. Both selected and known-reader override/native-field semantics
      must fail closed; never silently omit required extensions.
- [ ] Through existing captured lifecycle commands and real qualified temporary
      roots, add two separately identified generic rules beside manual AGENTS
      text, update/remove one, and assert exact retained manual/other
      contribution bytes, mode, state ownership and contracts. Keep the report
      separate from refs.
- [ ] Run `cargo test --test codex_instruction_boundary -- --nocapture` and
      covering unit tests; omission of OpenCode/VS Code readers or insertion of
      incidental readers into consumers must break independent literal
      assertions.
- [ ] Update the generation concern, format/usage boundaries and a focused
      ADR-0039 only where the implementation creates a durable decision. Record
      library-only support and remaining gates; do not advertise a CLI flag.
- [ ] Commit and write the report with RED/GREEN and mutation evidence.

### Task 3: Review, garden and deliver

- [ ] Run task reviews and resolve confirmed findings, then `just fmt`,
      `just verify`, `just sanity` and `just msrv-check`.
- [ ] Garden only this exact bounded spec/plan pair into durable records and
      archive, preserving all unrelated WIP and its existing sequencing.
- [ ] Publish the PR, run both numbered full review passes and final-head hosted
      checks. Fix blockers and track advisory debt under epic #283.
- [ ] Squash merge through the authorized driftsys policy, verify main's exact
      tree and hosted runs, then archive/remove only this owned finished
      worktree.
- [ ] Carry the explicit reader report into the next source-adapter design; do
      not enable instruction-source updates until that integration is complete.
