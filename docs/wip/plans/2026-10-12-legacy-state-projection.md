# Legacy-state projection implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task by task. Steps use checkbox syntax for tracking.

**Goal:** Describe schema-1 state without inventing requested selections or artifact ownership, preserving every typed legacy record.

**Architecture:** A pure public library projection retains a cloned `Lockfile` and indexed interpreted rows. A frozen historical-layout module provides the nine legacy paths and is reused by current pipeline wrappers. There are no CLI, persistence or filesystem effects.

**Tech stack:** Rust edition 2024, Rust 1.85, existing dependencies only.

**Spec:** [Legacy-state projection design](../specs/2026-10-12-legacy-state-projection-design.md).

## Global Constraints

- Preserve Rust 1.85, the synchronous/static-binary constraints, existing OpenCode behavior, and the fixed original defaults.
- This slice interprets current typed schema-1 records in memory only. It introduces no CLI change, fetch, generation, filesystem inspection, adoption, save, backup, or native operation.
- This is typed-field preservation, not a promise to recover unknown JSON fields or original serialization bytes. Exact raw backup remains executor work.
- All requested-surface history remains explicitly unresolved.
- No projector result by itself authorizes mutation.
- No serialized projection format or schema-2 codec is introduced.
- Only main.rs writes to stdout/stderr. Add no dependencies and preserve unrelated worktrees and edits.

## Review Focus

- Empty effective clients must always mean exactly the historical three clients; future Codex support must not change this expansion.
- A source parser success or recorded hash/ref must not acquire ownership or verified revision semantics.
- Invalid names, duplicate rows and unknown clients must remain inspectable, never disappear through map conversion or early return.
- Unsupported schema values must retain their snapshot but produce no interpreted destinations.
- Extraction must preserve all current path/resource behavior on Unix and Windows while projection paths use forward slashes.

---

### Task 1: Pure legacy state and frozen candidate layout

**Files:**

- Create: `src/legacy/mod.rs` for projection data, decoding and diagnostics.
- Create: `src/legacy/layout.rs` for historical client vocabulary and pure path contracts.
- Modify: `src/lib.rs` to expose `legacy`.
- Modify: `src/pipeline/output.rs` to delegate its existing pure layout helpers; keep copy/write/remove behavior intact.
- Test: `tests/legacy_projection.rs` for public-boundary examples.
- Modify: `docs/specification/specification.md` and `docs/book/commands.md` to describe the internal boundary and unchanged CLI/schema behavior.
- Modify: `docs/wip/plans/2026-10-11-instructions-rules-m1.md` to record this partial prerequisite without completing M1-A2.
- Update this task's checkboxes and ignored task report.

**Interfaces:**

- Consumes `crate::lockfile::{Lockfile, LockedItem, LockedBundle, LockedPlugin, LockedMcp}`, `crate::pipeline::ItemKind`, and `crate::source::parse_install_source_label`.
- Produces `pub fn project_legacy(lockfile: &Lockfile) -> LegacyProjection`.
- `LegacyProjection` retains `snapshot: Lockfile`, `items: Vec<LegacyItem>`, `bundles: Vec<LegacyBundle>`, `integrations: Vec<LegacyIntegration>`, `diagnostics: Vec<LegacyDiagnostic>`.
- Item and bundle rows retain `index: usize`, their cloned `record`, and `source: Option<InstallSource>`. Each item additionally has `requested_selection: RequestedSelection::Unresolved`, historical known clients, and `candidates: Vec<LegacyCandidate>`.
- `LegacyIntegration` has explicitly named `Plugin { index, record: LockedPlugin }` and `Mcp { index, record: LockedMcp }` variants. Preserve raw client/scope/status values, including unknown target strings.
- `LegacyCandidate` contains only `client: LegacyClient`, `entrypoint: String`, `resource_search_base: String`.
- `LegacyClient` has only `Claude`, `Copilot`, `OpenCode`. Its fixed `ALL` is independent of `generate::Client::ALL`; implement explicit conversion for current pipeline wrappers.
- Diagnostics are structured row locations (`Global`, `Item(usize)`, `Bundle(usize)`) plus distinguishable kinds: unsupported schema, invalid source, unknown client, invalid installed name, duplicate item identity, identical candidate entrypoint. Invalid-source diagnostics carry the existing actionable parser message. Collision diagnostics identify all involved row indices. Do not add a generic blocking-policy flag.
- No type in this module derives Serialize/Deserialize or contains `OwnedArtifact`, owners, artifact hashes, modes, source verification or I/O handles. `Clone`, `Debug`, `PartialEq`, `Eq` are suitable for data assertions.

- [ ] **Step 1: Add public-boundary failing acceptance tests.**

Create the fixture using actual typed state (no CLI or filesystem fixture is required):

```rust
use upskill::legacy::{LegacyClient, RequestedSelection, project_legacy};
use upskill::lockfile::{LockedItem, Lockfile};
use upskill::pipeline::ItemKind;

fn item() -> LockedItem {
    LockedItem {
        kind: ItemKind::Skill,
        name: "installed alias".into(),
        source: "gitlab+gitlab.example.com:org/team/repo@v1:skills/tool".into(),
        git_ref: Some("v1".into()),
        hash: Some("a".repeat(64)),
        source_name: Some("tool".into()),
        required_by: vec!["rule:parent".into()],
        group: Some("skills/tool".into()),
        clients: vec![],
    }
}

#[test]
fn empty_clients_preserve_state_and_expand_only_historical_clients() {
    let mut lock = Lockfile::new();
    lock.items.push(item());
    let before = lock.clone();
    let projected = project_legacy(&lock);
    assert_eq!(lock, before);
    assert_eq!(projected.snapshot, before);
    assert_eq!(projected.items[0].index, 0);
    assert_eq!(projected.items[0].record, before.items[0]);
    assert_eq!(projected.items[0].requested_selection, RequestedSelection::Unresolved);
    assert_eq!(projected.items[0].clients,
               vec![LegacyClient::Claude, LegacyClient::Copilot, LegacyClient::OpenCode]);
    assert_eq!(projected.items[0].candidates.len(), 3);
    assert_eq!(projected.items[0].candidates[0].entrypoint,
               ".claude/skills/installed alias/SKILL.md");
    assert_eq!(projected.items[0].source.as_ref().unwrap().to_string(),
               "https://gitlab.example.com/org/team/repo@v1:skills/tool");
}
```

Add independent literal expected entrypoint/resource-base pairs for all nine `(ItemKind, LegacyClient)` combinations. Use the installed alias, never the source name. Pin directory-backed skills and OpenCode rules, plus sibling resource bases for all flat kinds. For example Claude skill is `.claude/skills/x/SKILL.md` with `.claude/skills/x`; Copilot rule is `.github/instructions/x.instructions.md` with `.github/instructions/x`; OpenCode agent is `.opencode/agents/x.md` with `.opencode/agents/x`.

Add separate assertions for each case below, with exact diagnostic variant/location and preserved rows/snapshot:

1. Explicit `claude` alone still has unresolved requested selection; `copilot` alone cannot identify a native surface.
2. `clients = ["claude", "codex", "claude", "COPILOT"]` retains all raw values, gives one Claude candidate, and diagnoses both unknown spellings.
3. Invalid source retains its item and still describes known layout facts; invalid bundle source retains its bundle/index and diagnostic. Bundle names never create candidates.
4. Unpinned remote, pinned selector, modern HTTPS, Github and local labels decode using the existing parser. No fake immutable revision or generated hash field exists in the API.
5. Invalid installed names `""`, `"."`, `".."`, `"a/b"`, `"a\\b"`, `"C:x"`, `"a\n"` each produce no candidates. Spaces and Unicode names remain representable.
6. Duplicate `(kind, name)` rows preserve both records and report duplicate identity and identical candidate entrypoints. Same kind/name with disjoint clients diagnoses identity but does not invent a path collision. Different kinds with the same name remain separate valid identities.
7. Unsupported schema values 0 and 2 preserve all item/bundle/integration records, report unsupported schema globally and produce no candidates. Retain projected indexed rows without pretending their sources/clients are schema-1 interpretations.
8. Plugin statuses Installed/Skipped/Instructions and MCP Installed/Skipped, arbitrary client/scope strings, bundle references and duplicate integration rows survive exact typed equality. Include all fields and vector order in snapshot assertions.
9. Projection has no scope input: identical typed state gives identical relative paths regardless of a future project/user scope root.

- [ ] **Step 2: Run the acceptance test and record the expected failure.**

Run `cargo test --test legacy_projection`. Before adding the module, expect unresolved `upskill::legacy`; save the compiler failure. After minimal public type declarations compile, run again and record assertion failures proving missing interpretation/layout behavior. Do not count only compile failure as behavioral RED evidence.

- [ ] **Step 3: Implement the pure projection and frozen layout.**

Build the owned snapshot first. Enumerate original vectors without reordering or dropping rows. Check `schema == 1` before interpreting values. For schema 1, decode sources with the persisted-label parser; resolve the fixed historical client strings; validate the portable single-component installed name; compute candidates for distinct known clients. Collect diagnostics while continuing other rows. Detect duplicate identities and identical entrypoints across distinct rows without choosing a winner. Use borrowed-key grouping or simple bounded loops for diagnostics, never replace source vectors with maps.

Implement the historical path mapping once in `src/legacy/layout.rs`; keep the fixed client enum explicit. Return portable strings. Existing `output_path`, `is_dir_backed` and `resource_base_path` wrappers convert `generate::Client` explicitly and construct `PathBuf` from the historical paths. Keep existing filesystem operations and pipeline caller signatures intact. Preserve or adapt existing layout tests to literal expectations, avoiding tests that merely compare a wrapper to its own implementation.

```rust
// Pure boundary: no root, filesystem, fetch or native client access.
pub fn project_legacy(lockfile: &Lockfile) -> LegacyProjection {
    // Construct rows and diagnostics as specified above. The snapshot is
    // always retained even when interpretation cannot safely proceed.
    project_records(lockfile)
}
```

`project_records` is a private decomposition choice, not a required extra layer; implement directly if clearer. The concrete row/diagnostic types above are the contract. Do not return `Result` for malformed historical facts that must remain inspectable.

- [ ] **Step 4: Run focused GREEN checks and a semantic mutation.**

Run `cargo test --test legacy_projection` and `cargo test --lib pipeline::output::tests`. Temporarily alter empty-client expansion to omit OpenCode, run only `empty_clients_preserve_state_and_expand_only_historical_clients`, and require an assertion failure. Restore production bytes and show focused GREEN again. Record exact commands and outputs; never commit the mutant. Existing pipeline golden/resource tests must also stay green in full verification.

- [ ] **Step 5: Update canonical docs and the M1 plan.**

Document the pure library prerequisite, typed preservation, fixed historical defaults, unresolved requested selection and candidate-only semantics in the specification. In the book, state that current commands and schema remain unchanged; avoid advertising an unavailable migration command. In M1, identify this completed prerequisite and retain pending source verification, historical reproducibility/resources, adoption, schema-2 command preservation, exact backup and transactional execution work.

- [ ] **Step 6: Verify and commit.**

Run `just fmt`, `just verify`, and `cargo +1.85.0 check --locked --all-targets`. Use the authorized escalation only where the existing formatter cache, npm resolution, shared git index or loopback test restrictions require it. Report genuine failures separately from environment failures. Update checkboxes, write the complete ignored task report with RED/GREEN/mutation/gate evidence and scope limits, then commit `feat: project legacy lockfile state without adoption` with code, tests and docs together. Do not push or open a PR; controller handles task/final/PR review, CI and merge.

## Controller preflight and review

Rebase this unpublished branch onto the merged legacy-label compatibility fix before implementation. Read the spec and plan, record the one-task self-consistency and shared interface/file checks in the ignored SDD ledger, and generate the task brief. Dispatch a fresh implementer, then fresh task review, whole-branch review, the standard PR reviews and hosted CI. Retain the active foundation design and M1 plan: this slice does not complete their lifecycle work.
