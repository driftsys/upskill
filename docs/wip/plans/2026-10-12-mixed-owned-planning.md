# Mixed Owned Artifact Planning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Plan one complete installation containing instruction contributions and dedicated files without losing manual bytes, ownership bindings or physical evidence.

**Architecture:** Validate the whole operation, partition complete changes and observations by artifact form, invoke the existing pure planners, then rebuild and validate one deterministic result. Reuse existing physical file DTOs; reject cross-form physical takeovers and affected ancestor topology before composition.

**Tech Stack:** Rust edition 2024, Rust 1.85 minimum, existing standard-library and anyhow facilities; no new dependencies.

**Spec:** [Mixed owned-artifact planning design](../specs/2026-10-12-mixed-owned-planning-design.md).

## Global Constraints

- The API performs no I/O or input mutation.
- A Set is the complete desired set across both forms.
- An empty complete set requires explicit Remove.
- Locations use `(scope, raw root OsStr, portable relative path)` equality.
- Existing shared-owner restrictions remain: reject any affected multi-owner record rather than infer missing reference associations.
- Keep every affected physical action, including byte-identical no-ops.
- Retain an empty aggregate after its last contribution is removed.
- Any failure returns no successful combined plan.
- No new dependencies, asynchronous runtime, native commands, source fetching, filesystem observation, lock, journal, schema-2 persistence, CLI selector or target qualification belongs to this slice.
- This prerequisite does not complete M1-A2, M1-A3 or M1.
- Only main.rs presents output. Preserve Rust 1.85 and zero warnings.

## Review Focus

- A resource disappears from a complete mixed Set: delete every old dedicated file omitted from the new set, not only the first row.
- An installation changes forms at distinct locations: retain stable binding checks even when one subplanner has no prior owner.
- A contribution path is an ancestor of a dedicated path: reject the physical union even on removal-only operations.
- An aggregate carries mode `Some(0o640)`: keep its exact mode through update, no-op and last-region removal.
- Prior rows interleave forms and installations: preserve their order when rebuilding and append only genuinely new sorted rows.

---

### Task 1: Compose complete mixed installation plans

**Files:**
- Create `src/operations/mod.rs` and `src/operations/plan.rs`.
- Modify `src/lib.rs` to export `operations`.
- Create `tests/operation_plans.rs` for public API acceptance examples.
- Modify `docs/specification/specification.md`, `docs/book/commands.md`, `docs/wip/plans/2026-10-11-instructions-rules-m1.md` and the module map in `AGENTS.md` for the implemented boundary.
- Modify only current successor paragraphs in `docs/archive/README.md` for debt #264, preserving historical quotations and totals.
- Update this plan's own checkboxes. Leave all other WIP pairs untouched; gardening is controller-owned after task approval.

**Interfaces consumed:**

```rust
upskill::files::plan::plan_files(
    prior: &[OwnedArtifact],
    changes: &[FileInstallationChange],
    observations: &[ObservedFile],
) -> anyhow::Result<FileOperationPlan>;
upskill::instructions::plan::plan_instructions(
    prior: &[OwnedArtifact],
    changes: &[InstructionInstallationChange],
    observations: &[ObservedInstructionFile],
) -> anyhow::Result<InstructionOperationPlan>;
```

Use the actual DTOs from these modules, `ownership::validate_artifacts` and
crate-private `ownership::validate_relative_path`. Existing helpers and planner
bodies remain unchanged. The two new public types and function have exactly the
design's signatures and derive Debug, Clone, PartialEq and Eq without Serde.

- [ ] **Step 1: Add a creation acceptance test before the API exists.**

Use this independently specified fixture and literal expected bytes in
`tests/operation_plans.rs` (imports include the types used here):

```rust
use upskill::files::plan::{DesiredFile, FileLocation, FileValue, ObservedFile};
use upskill::instructions::plan::{DesiredInstructionContribution, InstructionLocation};
use upskill::operations::plan::{DesiredArtifact, InstallationChange, plan_owned_artifacts};
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn location(path: &str) -> FileLocation {
    FileLocation {
        root: InstallationRoot {
            scope: OwnershipScope::Project,
            path: std::env::temp_dir().join("upskill-mixed-plan"),
        },
        path: path.into(),
    }
}
fn owner(id: &str) -> ArtifactOwner {
    ArtifactOwner {
        installation_id: id.into(),
        source: "local:./registry".into(),
        source_revision: None,
        kind: ItemKind::Rule,
        source_name: "source-demo".into(),
        installed_name: "demo".into(),
    }
}

#[test]
fn creates_mixed_installation_with_exact_manual_bytes_and_modes() {
    let instructions = location("AGENTS.md");
    let resource = location("resources/data.bin");
    let changes = vec![InstallationChange::Set {
        owner: owner("install-a"),
        artifacts: vec![
            DesiredArtifact::Instruction(DesiredInstructionContribution {
                location: InstructionLocation {
                    root: instructions.root.clone(),
                    path: instructions.path.clone(),
                },
                id: "rule-a".into(),
                markdown: "A\n".into(),
                consumers: vec![TargetSurface::CodexCli],
            }),
            DesiredArtifact::File(DesiredFile {
                location: resource.clone(),
                value: FileValue { bytes: vec![0, 255], mode: Some(0o755) },
                consumers: vec![TargetSurface::ClaudeCli],
            }),
        ],
    }];
    let observations = vec![
        ObservedFile { location: resource.clone(), value: None },
        ObservedFile {
            location: instructions.clone(),
            value: Some(FileValue { bytes: b"Manual".to_vec(), mode: Some(0o640) }),
        },
    ];
    let saved = (changes.clone(), observations.clone());
    let result = plan_owned_artifacts(&[], &changes, &observations).unwrap();
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.artifacts.len(), 2);
    assert_eq!(result.files[0].location, instructions);
    assert_eq!(result.files[0].before, observations[1].value);
    assert_eq!(result.files[0].after, Some(FileValue {
        bytes: b"Manual\n<!-- upskill:begin rule-a -->\nA\n<!-- upskill:end rule-a -->\n".to_vec(),
        mode: Some(0o640),
    }));
    assert_eq!(result.files[1].location, resource);
    assert_eq!(result.files[1].before, None);
    assert_eq!(result.files[1].after, Some(FileValue { bytes: vec![0, 255], mode: Some(0o755) }));
    assert_eq!((changes, observations), saved);
}
```

Run `cargo test --test operation_plans creates_mixed_installation_with_exact_manual_bytes_and_modes -- --exact`.
Save actual missing-API compile RED. Do not call that an assertion failure.

- [ ] **Step 2: Add only DTOs/module exports and an empty-result stub, then record genuine assertion RED.**

`src/operations/mod.rs` contains `pub mod plan;`. Export `pub mod operations;`
from lib.rs. Copy the exact enum declarations from the design and import their
existing field types. Initially the function body is only:

```rust
Ok(FileOperationPlan { artifacts: Vec::new(), files: Vec::new() })
```

Use underscore-prefixed stub parameters to keep compilation warning-free, with
the same public signature types. Repeat Step 1: require an actual file-count
assertion failure. Add an invalid-operation test with an empty Set and no
observations; `assert!(result.is_err(), "empty complete set must conflict")`
must fail against the stub. Save both logs and append a report checkpoint before
writing real composition logic. Fix test-fixture compile errors before recording
behavioral RED.

- [ ] **Step 3: Extend acceptance examples before implementing composition.**

Create named tests for every design example C01–C10. Expected actions, manual
bytes, modes, ordering and ownership lists are independent literals. A setup may
use existing planners to construct valid prior metadata, but expected combined
outcomes must not be computed by the new API under test. Every error test checks
context and unchanged inputs. Cover these exact matrices:

| Test group | Cases and assertions |
| --- | --- |
| Complete replacement | Mixed update omits two old resources; remove both and keep/update the contribution. Mixed update omits the contribution; preserve exact manual bytes and retain desired resources. Global removal reaches both forms and retains empty aggregate. |
| Cross-form binding | File-only prior to instruction-only Set and reverse at distinct paths. Reject changes independently to source, kind, source_name and installed_name; accept coherent source_revision changes. |
| Complete request | Duplicate Set/Remove ID, duplicate Sets, unknown Remove, empty Set, duplicate desired file, duplicate contribution selector, malformed owner and consumers. |
| Physical collision | Both forms desired at one location; old file to desired contribution and reverse; explicit A removal/B replacement across forms; untouched owner at the other form. All conflict. Same-form file and contribution handoffs keep existing semantics. |
| Topology | Contribution `a` and file `a/b`, reverse forms, nonadjacent lexical ancestor `a`, `a-keep`, `a/b`; removal-only prior conflict; old/new transitions. Reject affected pairs; unrelated historical pairs and siblings `a/b` versus `ab` survive. |
| Evidence | Missing either form observation, duplicate location, extra observation, relative/empty root, invalid portable path, mode above `0o7777`, absent owned file, edited bytes, changed or missing file mode, invalid aggregate UTF-8 and edited/unrecorded markers. |
| Modes and no-ops | Aggregate Some(0o640) survives update/no-op/last removal; absent aggregate after mode None. Dedicated Some(0o755), Some(0o600) and None remain exact and distinct; all no-op preconditions remain. |
| Sharing and identity | Affected shared file or contribution conflicts, untouched shared records survive; roots with raw `.` components remain distinct and project/user scopes remain distinct. |
| Stable order | At least four prior rows interleaving forms and installations, two old files removed, two retained rows replaced in place, sorted new selectors/paths appended. Reversing changes, desired entries and observations yields the same result. |

Run the compiled matrix against the stub and preserve its actual failures. Do
not claim every matrix failure is assertion-only unless the log proves that.

- [ ] **Step 4: Implement the coordinator in operations/plan.rs.**

Use private focused helpers in the same module. Keep existing planner bodies
unchanged. Implement these stages in order:

1. Validate prior records; index full installation bindings and unique changes.
   Enforce stable fields globally before splitting. Build all prior/desired
   physical form membership and the affected location union using raw OsString
   root keys, OwnershipScope and portable path strings.
2. Validate every observation and exact affected membership. Reject mixed-form
   membership at a physical location in the prior/desired union, and affected
   ancestor pairs with component-boundary comparison. No PathBuf-normalized key.
3. Partition prior rows, desired values and observations. Produce each form's
   Set/Remove/no-change exactly as the design specifies. Preserve full owner
   values and consumers. Calls to plan_files and plan_instructions consume the
   resulting complete form partitions, with contextual errors.
4. Rebuild original row identities in prior order from the two resulting maps.
   New identities append in location/selector order. Validate the combined
   ownership to check coherent revisions and shared locations.
5. Map aggregate plans back to physical values with exact observed before bytes
   and modes, keeping after mode equal to observed mode or None if absent.
   Combine with dedicated actions, reject duplicate locations and sort them.

The core explicit physical checks use component boundaries and preserved modes:

```rust
fn is_descendant(parent: &str, child: &str) -> bool {
    child.strip_prefix(parent).is_some_and(|suffix| suffix.starts_with('/'))
}
```

```rust
// Inside aggregate-action conversion, with observed: &ObservedFile:
let after_mode = observed.value.as_ref().and_then(|value| value.mode);
```

Do not infer modes, delete aggregates, normalize roots, adopt equal-byte files,
drop omitted old forms or silently retain shared affected ownership. Run
`cargo test --test operation_plans` until the specified matrix is GREEN.

- [ ] **Step 5: Execute six meaningful semantic controls, restoring exact bytes.**

Mutate one production behavior at a time: retain omitted dedicated files; bypass
global stable binding validation for disjoint-form changes; bypass cross-form
location collision rejection; skip cross-form affected topology validation;
erase aggregate observed modes; stop reconstructing after the first prior row.
For each, run the matching complete-set/binding/collision/topology/mode/order
test and require an intended assertion failure, not a compile/fixture failure.
Use finally restoration and compare source bytes to the saved snapshot. After
all controls, require the full focused suite GREEN and no production mutation.

- [ ] **Step 6: Update documentation and self-review the complete task diff.**

Document the exact API, complete mixed Sets, cross-form safety boundary, aggregate
mode evidence, deterministic ordering and remaining activation gates in the
canonical specification. Add a brief library-only usage note, module-map entry
and bounded M1 status. Refresh archive current successor prose for #264; preserve
historical quotations, totals and unrelated WIP dispositions. No native/CLI/schema-2
activation claim. Read every changed file against the spec before final gates.

- [ ] **Step 7: Run final gates once, commit and report.**

Use `PATH=/tmp/upskill-captured-check-bin:/Users/sebastientasson/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin`
for the test/gate processes; it contains required tools but excludes native agent
CLIs invoked by legacy plugin tests. Use a task-specific CARGO_TARGET_DIR for
mutations and final verification; no reviewer may contaminate that target.

Run focused regression suites `cargo test --test instruction_plans --test file_plans --test compile_items --test ownership_records`.
Then run ordered `just fmt` and `just verify`, followed by
`cargo +1.85.0 check --all-targets --locked` and `git diff --check`.
Stage only owned files and commit `feat: compose mixed owned artifact plans`.
Write full actual RED/GREEN/control/gate evidence, file scope, self-review and
commit/clean status to the controller-specified report. No push, PR, gardening,
nested agents or discretionary review; the controller supplies independent
task/final/numbered review gates. An explicit automatic QUICK trigger is handled
only as instructed, with no extra cycle over review fixes.
