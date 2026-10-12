# Owned Whole-File Planning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce deterministic whole-file creation, replacement and deletion plans from explicit ownership changes and exact supplied observations.

**Architecture:** Add a pure `files::plan` boundary alongside `instructions::plan`. Validate existing ownership, complete replacement sets and supplied preconditions, then return owned records and individual file actions. Source classification, mixed-plan composition and filesystem application stay separate.

**Tech Stack:** Rust 1.85, existing `anyhow` and `sha2`, standard collections, existing ownership and target types.

**Spec:** [Owned whole-file planning design](../specs/2026-10-12-owned-file-planning-design.md).

## Global Constraints

- Rust 1.85, synchronous standard library plus existing dependencies, no stdout or stderr outside `main.rs`, and no new dependency or runtime.
- Inputs are borrowed and unchanged. Outputs own their bytes and records.
- Keep the historical pipeline, existing instruction planner semantics, CLI flags and lockfile behavior unchanged.
- Locations compare scope, exact root `OsStr` spelling and portable path. Do not use `Path` equality to normalize roots.
- Do not canonicalize, fold case, resolve aliases or claim containment.
- A `Set` is the complete nonempty whole-file set for its installation. Reject empty sets; use `Remove` for the last file.
- A repeated installation ID cannot change source, kind, source name or installed alias. Its revision may change, consistently across all replacement records.
- Require exactly one observation for every affected location, including metadata-only changes and byte-identical no-ops.
- Reject affected Markdown-contribution records and affected multi-owner whole-file records.
- Require exact equality of the optional recorded and observed modes.
- At a location with no prior ownership, require absence. Identical existing bytes are still unowned and conflict.
- Deletion identifies only the recorded file; it never grants directory ownership or recursive removal authority.
- Failure returns no successful partial plan.
- Code, acceptance tests and canonical specification/usage notes land together; run `just fmt` followed by `just verify` before opening a PR.

## Review Focus

1. Unmarked equal bytes are a conflict, and an edited/missing owned file or changed optional mode cannot be updated, removed or accepted as a no-op.
2. Complete-set replacement preserves unrelated files, shared rows and contributions; same-location ownership handoff requires both old and new owners to be selected.
3. Empty bytes, binary resources, Unicode paths, mode boundaries and exact lexical root variants remain valid without normalization or lossy conversion.
4. Missing/duplicate/extraneous observations and invalid desired/observed locations or modes fail meaningfully, including no-op operations.
5. File/descendant conflicts reject affected topology transitions without rejecting component-prefix siblings or disjoint untouched pairs.

## File structure

| File                                                 | Responsibility                                                 |
| ---------------------------------------------------- | -------------------------------------------------------------- |
| `src/files/mod.rs`                                   | Export focused planning module                                 |
| `src/files/plan.rs`                                  | Pure DTOs, validation, indexing and whole-file operation plans |
| `src/lib.rs`                                         | Export `files` module                                          |
| `tests/file_plans.rs`                                | Public acceptance matrix and independent expected records      |
| `docs/specification/specification.md`                | As-built public contract and pending lifecycle gates           |
| `docs/book/commands.md`                              | State that existing commands remain on their current lifecycle |
| `docs/wip/plans/2026-10-11-instructions-rules-m1.md` | Record this bounded prerequisite without marking M1 complete   |

---

### Task 1: Plan owned whole-file changes from explicit observations

**Files:** Create `src/files/mod.rs`, `src/files/plan.rs`, `tests/file_plans.rs`; modify `src/lib.rs`, canonical specification, command guide and M1 plan above. Update only this plan's checkboxes as steps complete. Do not edit the parent lifecycle design or other worktrees.

**Interfaces:**

- Consume `ownership::{ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope, validate_artifacts}` and the existing crate-private `ownership::validate_relative_path(&str) -> anyhow::Result<()>`.
- Consume `target::TargetSurface` and `sha2::{Digest, Sha256}`.
- Produce the exact seven DTOs and `plan_files` signature in the spec's Public interface. Read that section before declaring types; do not rename fields or add serialization.
- The planner consumes complete desired whole-file sets, not source directories or a manifest authenticity token. No filesystem or current pipeline callers are added.

- [ ] **Step 1: Write public acceptance tests and capture compile RED.**

Start `tests/file_plans.rs` with the following fixture and creation test. The byte hashes are independent Python `hashlib.sha256` results; use these literal expected values rather than calling production hashing to construct expectations.

```rust
use upskill::files::plan::{
    DesiredFile, FileInstallationChange, FileLocation, FileValue, ObservedFile,
    plan_files,
};
use upskill::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnershipScope,
};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn location(path: &str) -> FileLocation {
    FileLocation {
        root: InstallationRoot {
            scope: OwnershipScope::Project,
            path: std::env::temp_dir().join("upskill-file-plan"),
        },
        path: path.to_owned(),
    }
}
fn owner(id: &str) -> ArtifactOwner {
    ArtifactOwner {
        installation_id: id.to_owned(),
        source: "local:./content".to_owned(),
        source_revision: None,
        kind: ItemKind::Skill,
        source_name: "source-demo".to_owned(),
        installed_name: "demo".to_owned(),
    }
}
fn desired(path: &str, bytes: &[u8]) -> DesiredFile {
    DesiredFile {
        location: location(path),
        value: FileValue { bytes: bytes.to_vec(), mode: None },
        consumers: vec![TargetSurface::ClaudeCli],
    }
}

#[test]
fn creates_sorted_entrypoint_and_binary_resource_with_literal_evidence() {
    let entry = desired("demo/SKILL.md", b"abc");
    let resource = desired("demo/assets/data.bin", &[0, 255]);
    let observations = vec![
        ObservedFile { location: resource.location.clone(), value: None },
        ObservedFile { location: entry.location.clone(), value: None },
    ];
    let changes = vec![FileInstallationChange::Set {
        owner: owner("install-a"), files: vec![resource.clone(), entry.clone()],
    }];
    let before = (changes.clone(), observations.clone());
    let plan = plan_files(&[], &changes, &observations).unwrap();
    assert_eq!(plan.files.len(), 2);
    assert_eq!(plan.artifacts.len(), 2);
    for (index, expected) in [entry, resource].iter().enumerate() {
        assert_eq!(plan.files[index].location, expected.location);
        assert_eq!(plan.files[index].before, None);
        assert_eq!(plan.files[index].after, Some(expected.value.clone()));
        assert_eq!(plan.artifacts[index].root.path.as_os_str(), expected.location.root.path.as_os_str());
        assert_eq!(plan.artifacts[index].path, expected.location.path);
        assert_eq!(plan.artifacts[index].owners, vec![owner("install-a")]);
        assert_eq!(plan.artifacts[index].consumers, vec![TargetSurface::ClaudeCli]);
    }
    assert_eq!(plan.artifacts[0].content, ArtifactContent::File {
        content_sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_owned(), mode: None,
    });
    assert_eq!(plan.artifacts[1].content, ArtifactContent::File {
        content_sha256: "06eb7d6a69ee19e5fbdf749018d3d2abfa04bcbd1365db312eb86dc7169389b8".to_owned(), mode: None,
    });
    assert_eq!((changes, observations), before);
}
```

Run `cargo test --test file_plans creates_sorted_entrypoint_and_binary_resource_with_literal_evidence`. Save its compile failure for the missing public module/API. Do not start real implementation before Step 2's valid behavioral assertion failures.

- [ ] **Step 2: Add only compile scaffolding and capture genuine assertion RED.**

Declare the spec's DTOs, expose `pub mod plan;` in `files/mod.rs` and `pub mod files;` in `lib.rs`. Use this deliberately empty stub temporarily:

```rust
pub fn plan_files(
    prior: &[OwnedArtifact],
    _changes: &[FileInstallationChange],
    _observations: &[ObservedFile],
) -> anyhow::Result<FileOperationPlan> {
    Ok(FileOperationPlan { artifacts: prior.to_vec(), files: Vec::new() })
}
```

Re-run the creation test and save its `plan.files.len()` assertion failure (0 versus 2), distinct from compile RED. Add a second test that supplies an existing unowned file with identical desired bytes/mode and requires an error containing `unowned`; run it against the stub and save the failed rejection assertion. Run Cargo commands serially and wait for completion. If a fixture fails to compile, correct it and rerun until these valid behavioral RED cases are recorded before drafting real logic.

- [ ] **Step 3: Implement validation and deterministic planning.**

Use exact lexical keys and SHA-256 of captured desired bytes:

```rust
use std::ffi::OsString;
use sha2::{Digest, Sha256};
type LocationKey = (OwnershipScope, OsString, String);
fn location_key(location: &FileLocation) -> LocationKey {
    (location.root.scope, location.root.path.as_os_str().to_owned(), location.path.clone())
}
fn content_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn is_descendant(parent: &str, child: &str) -> bool {
    child.strip_prefix(parent).is_some_and(|suffix| suffix.starts_with('/'))
}
```

Implement the following algorithm in focused private helpers; all helpers remain synchronous and private:

1. Validate `prior` with `validate_artifacts`. Index its installation bindings. For every observation validate root absolute/nonempty, the shared relative-path predicate and present mode range before membership diagnostics. Validate desired rows with newly computed file hashes, one explicit owner and their explicit consumers through `validate_artifacts`.
2. Build a unique changed-ID set; reject duplicate changes, unknown removals and empty `Set` lists. Compare stable owner fields individually (source/kind/source_name/installed_name), allowing revision replacement. Index desired files by `LocationKey`; reject duplicate locations globally.
3. Build the affected location set from prior rows owned by changed IDs plus desired locations. Reject any affected contribution or multi-owner file. Reject desired takeover of a prior untouched owner. Index observations, rejecting duplicate/extraneous locations and missing affected locations.
4. Compare every prior/desired lexical location pair within the same exact root/scope when at least one location is affected. A strict slash-component ancestor relation conflicts in either direction. Equal locations do not trigger this check; ownership/duplicate rules handle them. This union check deliberately rejects removal-to-directory transitions. Do not fail solely on a disjoint untouched pair.
5. For each sorted affected key, inspect the prior whole-file row. Present prior ownership requires observed `Some(FileValue)`, exact SHA-256 equality to its recorded content, and exact `Option<u32>` mode equality. No prior row requires observed absence; identical unowned output still conflicts. Keep diagnostic context naming the installation/location where available.
6. Emit one action with exact observed `before` and cloned desired `after`, or absent `after` for removal. Emit even byte/mode-identical no-ops. Build desired ownership records from exact desired content hash/mode and explicit owner/consumer metadata.
7. Walk prior rows in input order: retain untouched rows exactly; replace same-location rows from desired records; omit removed rows. Append remaining new desired records by sorted location. Validate the final combined artifacts. Return the complete plan only after all checks succeed.

Use `BTreeMap`/`BTreeSet` for deterministic indexing. No iteration over unordered hash collections may determine output order. Never infer directory ownership, selected consumers, source revisions or compatibility from equal content. No fallback to lossy UTF-8, filesystem access or legacy resource walkers.

- [ ] **Step 4: Complete the acceptance matrix and focused GREEN.**

Extend the matching public test file with these concrete cases, checking full before/after values and ownership metadata where meaningful:

| Case                                    | Required assertions                                                                                                                                                                                                                                                    |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Update `abc` to `changed`, then remove  | Literal new hash `d67e2e944994496c8d8ec76eed0cf9f09679448d584b532bebf941852a37f5ed`; old bytes/mode in before; individual deletion after `None`; final artifacts omit only selected files                                                                              |
| Two-file revision-only update           | Both actions retain identical bytes/modes and before values; owner revision changes on both; source/name/alias stay literal; no input mutation                                                                                                                         |
| Alias/source/kind/source_name collision | Each changed stable field separately fails for existing ID; a new ID may bind a distinct source name/alias at absent locations                                                                                                                                         |
| Complete-set replacement                | Old omitted location becomes one deletion; new location becomes one creation; unrelated rows keep original order and full consumers/owners/content                                                                                                                     |
| Untouched shared and contribution rows  | Both preserved exactly; affected shared file and affected contribution each reject on both set/removal paths                                                                                                                                                           |
| Explicit ownership handoff              | Remove A plus Set B at A's verified location succeeds; Set B alone fails; edited A's file fails even with both selected                                                                                                                                                |
| Unowned present destination             | Equal and different bytes each conflict; no implicit adoption even with identical mode                                                                                                                                                                                 |
| Changed or missing owned bytes          | Update, remove and byte-identical desired no-op reject; assert meaningful diagnostic, not generic `is_err` alone                                                                                                                                                       |
| Optional-mode identity                  | Recorded `None`/observed `Some(0)` and reverse each reject; `Some(0)` and `Some(0o7777)` round-trip successfully; changed mode rejects; invalid desired and observed `0o10000` reject                                                                                  |
| Empty/binary/Unicode values             | Empty bytes have literal hash `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`; Unicode path retained exactly; binary bytes retained without string decoding                                                                                         |
| Observation membership                  | Missing, duplicate and extraneous observations reject separately; no-op still requires observation; invalid roots/paths/modes reject before a membership diagnostic                                                                                                    |
| Invalid metadata                        | Empty/duplicate consumers, invalid owner/revision, invalid prior hash, duplicate desired paths and duplicate changes reject; empty sets and unknown removes reject                                                                                                     |
| Lexical roots and scopes                | Two raw root spellings differing by a `/.` component stay distinct using `as_os_str()` assertions; same relative path in project/user scopes or distinct roots produces distinct actions                                                                               |
| Topology                                | Prior file `a` against desired `a/b`, reverse ancestor, and affected file against a contribution descendant each reject; explicit remove `a` plus create `a/b` rejects; `a` and `ab` succeed; disjoint untouched conflicting pair does not block another root/location |
| Determinism                             | Reverse changes, desired files and observations: sorted actions and new rows stay identical; existing prior row order preserved; input bytes/modes/metadata unchanged                                                                                                  |
| Empty operation                         | Valid prior with no changes/observations returns exact prior rows and no actions; extraneous observation still rejects                                                                                                                                                 |

A helper may construct valid prior fixture hashes from independent `sha2` in test code, but critical creation/update/empty expected hashes above remain literal. Do not use the production planner or production hashing to derive expected post-state values. Assert root raw spellings explicitly because `PathBuf` structural equality normalizes components.

Run `cargo test --test file_plans` and `cargo test --test instruction_plans`, plus `cargo test --test ownership_records`; save command/output logs. Extend meaningful existing regression coverage only if this feature changes a shared boundary.

- [ ] **Step 5: Demonstrate semantic sensitivity and exact restoration.**

One mutation at a time, with saved original production bytes and `finally` restoration plus byte comparison:

1. Skip owned before-hash mismatch rejection. The edited-owned-file assertion must fail because an unsafe plan succeeds.
2. Permit present unowned equal-byte output. The unowned-equal-byte rejection assertion must fail.
3. Omit preservation of an untouched prior row. Complete-set preservation assertions must fail against exact record/owner/consumer values.
4. Skip optional-mode equality. Both recorded/observed evidence mismatch directions must become accepted and fail their rejection assertions.

Require intended assertion failures, not compilation errors or panics in unrelated fixtures. Save failed command/output, restoration comparison and restored focused GREEN for each. Restore source exactly and run the full file-plan acceptance suite once after all controls. Do not treat these later mutants as a substitute for Step 2's genuine assertion RED.

- [ ] **Step 6: Update canonical records and run ordered final gates.**

Add a specification subsection after strict source manifests describing the exact public API, ownership/precondition/mode/topology contracts, sorted actions and record-order guarantees. State supplied observations are unqualified, source/resource classification and mixed-plan composition are pending, and no existing command/native behavior changes. Use GitHub links for non-book design documents and the actual `specification.html` published path for book links.

Add a short command-guide note that current commands do not call this planner, and an M1 prerequisite paragraph preserving pending state/executor/Codex gates. Keep the operational lifecycle design active. Do not garden or archive during implementation; controller dispatches scoped gardener after task review.

Run, in order:

```text
just fmt
just verify
cargo +1.85.0 check --all-targets --locked
git diff --check
```

Save exit statuses and test counts, including unswept OS cases. Stage only the assigned code/tests/docs and this plan's checkbox changes, then commit:

```text
git add src/files src/lib.rs tests/file_plans.rs docs/specification/specification.md docs/book/commands.md docs/wip/plans/2026-10-11-instructions-rules-m1.md docs/wip/plans/2026-10-12-owned-file-planning.md
git commit -m 'feat: plan owned whole-file changes'
```

Append report with RED/GREEN/mutation/restoration evidence, final commands/output, commit, deviations and remaining qualification limits. No push or PR creation by the implementer.
