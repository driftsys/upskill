# Owned Instruction Planning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Plan complete installation-owner contribution changes without losing manual bytes, untouched owners or exact before-state preconditions.

**Architecture:** Add a pure planner beside the existing aggregate editor. Reuse one private span validator for verified body inspection and reconciliation, group caller-supplied observations by exact lexical location, and retain the complete resulting ownership set. No effects are enabled.

**Tech Stack:** Existing Rust 2024 crate, `anyhow`, current ownership/editor APIs; Rust 1.85; no new dependencies.

**Spec:** [Owned instruction operation planning](../specs/2026-10-12-owned-instruction-planning-design.md).
The [operational direction](../specs/2026-10-12-m1-owned-lifecycle-design.md) remains active future work.

## Global Constraints

- Preserve Rust 1.85 and the synchronous/static-binary constraints.
- Add no dependencies, CLI changes, codecs, schema-2 writes or native operations.
- Only main.rs writes to stdout/stderr; planner and reader perform no I/O.
- Preserve existing aggregate editor behavior and unrelated worktrees/edits.

## Review Focus

- Consumer unions do not reveal each owner's selection: affected multi-owner contributions must conflict; unrelated shared records survive.
- Removing an early region changes later separator context: returned metadata must support subsequent removal restoring literal manual bytes.
- Ownership-only/no-op edits still need stale-plan protection: file plans retain exact observed before bytes.
- An ID belongs to one source/item/alias binding: set cannot reinterpret that binding; recorded revision may change.
- Empty or malformed desired data must not erase prior state: reject empty sets, invalid records and selector takeover; no partial plan escapes.

---

### Task 1: Owner-aware instruction operation plans and verified body inspection

**Files:**

- Create: `src/instructions/plan.rs` — public pure operation planner and its DTOs.
- Modify: `src/instructions/mod.rs` — module export and accurate module description.
- Modify: `src/instructions/aggregate.rs` — shared private span validation and public verified body reader; retain current reconciliation semantics.
- Create: `tests/instruction_plans.rs` — public planner acceptance tests.
- Modify: `tests/instruction_contributions.rs` — reader acceptance/regression tests.
- Modify: `docs/specification/specification.md` — describe implemented pure planning contract and limits.
- Modify: `docs/book/commands.md` — library prerequisite only; current commands/state unchanged.
- Modify: `docs/wip/plans/2026-10-11-instructions-rules-m1.md` — scoped progress and remaining activation work.
- Modify: this plan's task checkboxes only.
- Write ignored report/logs under this plan's SDD workspace.

**Interfaces:**

Consumes existing `ArtifactOwner`, `InstallationRoot`, `OwnedArtifact`,
`ArtifactContent`, `TargetSurface`, `ownership::validate_artifacts`,
`aggregate::{Contribution, OwnedContribution, AggregateEdit, reconcile}`.
Produces the following public types and functions, all DTOs deriving
`Debug, Clone, PartialEq, Eq` without serialization derives:

```rust
pub struct InstructionLocation {
    pub root: InstallationRoot,
    pub path: String,
}
pub struct ObservedInstructionFile {
    pub location: InstructionLocation,
    pub content: Option<Vec<u8>>,
}
pub struct DesiredInstructionContribution {
    pub location: InstructionLocation,
    pub id: String,
    pub markdown: String,
    pub consumers: Vec<TargetSurface>,
}
pub enum InstructionInstallationChange {
    Set {
        owner: ArtifactOwner,
        contributions: Vec<DesiredInstructionContribution>,
    },
    Remove {
        installation_id: String,
    },
}
pub struct PlannedInstructionFile {
    pub location: InstructionLocation,
    pub before: Option<Vec<u8>>,
    pub after: Vec<u8>,
}
pub struct InstructionOperationPlan {
    pub artifacts: Vec<OwnedArtifact>,
    pub files: Vec<PlannedInstructionFile>,
}
pub fn plan_instructions(
    prior: &[OwnedArtifact],
    changes: &[InstructionInstallationChange],
    observations: &[ObservedInstructionFile],
) -> anyhow::Result<InstructionOperationPlan>;

// Added to aggregate.rs:
pub fn inspect_owned(
    existing: &str,
    prior: &[OwnedContribution],
) -> anyhow::Result<Vec<Contribution>>;
```

The caller resolves direct/bundle references and native compatibility before
supplying changes. This planner does not establish those facts or infer reference
counts. Set replaces one owner's complete contribution set, not the whole input
ownership vector. An absent file is distinct from an existing empty file.

- [ ] **Step 1: Write acceptance tests before production changes.**

Use public APIs and actual editor/record values. Include these complete core
examples, plus the explicit table below. Build platform-valid roots through
`if cfg!(windows) { "C:/workspace" } else { "/workspace" }`.

```rust
use upskill::instructions::aggregate::{self, Contribution};
use upskill::instructions::plan::*;
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

fn location(path: &str) -> InstructionLocation {
    InstructionLocation {
        root: InstallationRoot {
            scope: OwnershipScope::Project,
            path: if cfg!(windows) { "C:/workspace" } else { "/workspace" }.into(),
        },
        path: path.into(),
    }
}
fn owner(id: &str) -> ArtifactOwner {
    ArtifactOwner {
        installation_id: id.into(), source: "local:./registry".into(),
        source_revision: None, kind: ItemKind::Rule,
        source_name: id.into(), installed_name: id.into(),
    }
}
fn set(id: &str, markdown: &str) -> InstructionInstallationChange {
    InstructionInstallationChange::Set {
        owner: owner(id),
        contributions: vec![DesiredInstructionContribution {
            location: location("AGENTS.md"), id: id.into(),
            markdown: markdown.into(), consumers: vec![TargetSurface::CodexCli],
        }],
    }
}
#[test]
fn separate_owner_removal_preserves_manual_bytes_and_later_metadata() {
    let manual = b"Manual instructions without final newline".to_vec();
    let first = plan_instructions(&[], &[set("a", "A\n"), set("b", "B\n")],
        &[ObservedInstructionFile { location: location("AGENTS.md"), content: Some(manual.clone()) }]).unwrap();
    let second = plan_instructions(&first.artifacts,
        &[InstructionInstallationChange::Remove { installation_id: "a".into() }],
        &[ObservedInstructionFile { location: location("AGENTS.md"), content: Some(first.files[0].after.clone()) }]).unwrap();
    assert_eq!(second.artifacts.len(), 1);
    assert_eq!(second.artifacts[0].owners, vec![owner("b")]);
    assert_eq!(second.files[0].after, b"Manual instructions without final newline\n<!-- upskill:begin b -->\nB\n<!-- upskill:end b -->\n");
    let third = plan_instructions(&second.artifacts,
        &[InstructionInstallationChange::Remove { installation_id: "b".into() }],
        &[ObservedInstructionFile { location: location("AGENTS.md"), content: Some(second.files[0].after.clone()) }]).unwrap();
    assert_eq!(third.files[0].after, manual);
}
#[test]
fn no_op_update_keeps_exact_before_state() {
    let first = plan_instructions(&[], &[set("a", "A\n")],
        &[ObservedInstructionFile { location: location("AGENTS.md"), content: None }]).unwrap();
    let before = first.files[0].after.clone();
    let second = plan_instructions(&first.artifacts, &[set("a", "A\n")],
        &[ObservedInstructionFile { location: location("AGENTS.md"), content: Some(before.clone()) }]).unwrap();
    assert_eq!(second.files.len(), 1);
    assert_eq!(second.files[0].before, Some(before.clone()));
    assert_eq!(second.files[0].after, before);
}
#[test]
fn verified_reader_preserves_document_order() {
    let edit = aggregate::reconcile("manual", &[], &[Contribution::new("a", "A\n"), Contribution::new("b", "B\n")]).unwrap();
    let mut records = edit.owned.clone();
    records.reverse();
    assert_eq!(aggregate::inspect_owned(&edit.content, &records).unwrap(),
        vec![Contribution::new("a", "A\n"), Contribution::new("b", "B\n")]);
}
```

Extend with independently asserted cases (fixtures may use the real editor to
construct prior records, but expected ownership/body/manual-byte results must
not be computed by the planner under test):

| Input                                                                                                     | Expected public result                                                                                                  |
| --------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Two owners, update `a` body                                                                               | `b` body/owner/consumers unchanged, literal manual prefix/suffix unchanged                                              |
| Set `a` omits an old region and moves another to a new file                                               | Old omitted region gone; new destination present; other owners survive; each touched location has exact before bytes    |
| Remove final region from previously absent file                                                           | One file plan with empty after bytes, no deletion action                                                                |
| Set same ID with different revision only                                                                  | Allowed; every retained owner binding for that ID updated consistently                                                  |
| Set changes source, kind, source name or alias separately                                                 | Each rejected with installation-ID context                                                                              |
| Desired ID already belongs to untouched owner, including identical body                                   | Conflict, no implicit takeover/sharing                                                                                  |
| Affected whole file or multi-owner region                                                                 | Conflict, original inputs unchanged                                                                                     |
| Unaffected whole file and unaffected multi-owner region, including shared region in an affected aggregate | Ownership/body survive; unaffected-location records are exact; affected-region separator metadata comes from the editor |
| Duplicate change IDs (set/set or set/remove), empty set, unknown remove                                   | Each rejected                                                                                                           |
| Duplicate desired selectors, empty/duplicate consumers, invalid owner ID/source/name/revision             | Each rejected before returning a plan                                                                                   |
| Invalid path/root and invalid contribution ID/body                                                        | Each rejected, including removal/addition combinations                                                                  |
| Missing, duplicate or extraneous observations                                                             | Each rejected; no-op changes require the affected observation too                                                       |
| Absent observed file with prior owned region                                                              | Conflict; missing bytes are not an empty owned region                                                                   |
| Invalid UTF-8, edited/missing/duplicate marker, unknown marker                                            | Each rejected by reader/planner; manual bytes never normalized                                                          |
| Distinct raw lexical root spellings and shuffled new changes                                              | Distinct location plans sorted by documented key; inputs unchanged                                                      |
| Empty changes and observations                                                                            | Complete prior vector unchanged; no file plans                                                                          |

Reader tests in `instruction_contributions.rs` also pin duplicate prior IDs,
overlapping spans, inline/malformed markers and separator edits through the same
validation used by reconciliation. Reuse focused existing fixtures where useful.

- [ ] **Step 2: Establish compile RED then behavioral assertion RED.**

Run `cargo test --test instruction_plans` before defining the module; save the
expected missing API compile failure. Introduce only the DTOs, module export and
minimal signatures; a stub returning unchanged prior records and no file plans
is sufficient to compile. Run focused tests again and save actual assertion
failures for owner/new-region plans and the no-op precondition. Compiler failures
are not behavioral RED evidence. Tests must be meaningful before full code.

- [ ] **Step 3: Implement shared inspection and pure planning.**

In `aggregate.rs`, extract current span/marker verification into one private
helper returning sorted verified spans. Keep `find_owned_span` hash/separator
checks. Store its already-computed body start on `OwnedSpan`; use that and
`end_marker_start` to read exact Markdown. Reject overlaps before returning spans.
Both public entrypoints use this helper; keep reconciliation's existing desired
validation order and existing output behavior. The reader constructs real
`Contribution` values from verified body slices, never reparses markers separately.

In `plan.rs`, use the public DTOs above and focused private functions. A complete
native key helper can be written as:

```rust
use std::ffi::OsString;
use crate::ownership::OwnershipScope;
type LocationKey = (OwnershipScope, OsString, String);
fn location_key(location: &InstructionLocation) -> LocationKey {
    (location.root.scope, location.root.path.as_os_str().to_owned(), location.path.clone())
}
```

Implement in this order:

1. Validate prior artifacts. Index IDs and their bindings; validate unique changes
   and known removals. Compare existing set binding fields explicitly, allowing
   revision to change. Set with no contributions is an error.
2. Derive affected locations from every prior record involving a changed ID and
   every desired location. A prior whole file at an affected location fails. A multi-owner
   region fails only when one of its owners is changed; an untouched multi-owner
   region at the same aggregate location is retained. Index observations and require exact equality of their location key set
   with the affected set. No filesystem access or canonicalization.
3. At each affected location decode exact observed UTF-8 (`None` starts empty
   only when no prior region exists). Collect all prior contribution records at
   that location, and inspect verified bodies. Retain bodies/ownership outside
   changed IDs. Reject a desired selector belonging to an untouched owner or any
   duplicate desired selector. Replace selected owners' complete sets.
4. Reconcile once with retained and desired bodies. Match every returned record
   by selector to its retained or desired owner/consumer metadata; update full
   contribution metadata from the editor. Build a file plan containing cloned
   exact before bytes even for identical after bytes.
5. Preserve all unaffected records. Traverse prior rows in order, replacing
   surviving affected selectors with rebuilt rows and omitting removed ones;
   append only genuinely new selectors sorted by key/ID. Sort file plans by key.
   Validate the complete resulting artifacts. Return only a complete valid plan.

Do not return partial plans on any error. Add contextual errors naming the
installation, path or selector involved. Validate desired data even if it could
otherwise be optimized out; no idempotence fast path may bypass ownership checks.

- [ ] **Step 4: Run focused GREEN and semantic negative controls.**

Run `cargo test --test instruction_plans` and
`cargo test --test instruction_contributions`; all existing editor tests remain
green. Temporarily mutate retention to drop untouched owner `b` and run the
literal three-operation acceptance test; it must fail the owner/body assertion.
Separately replace planned before bytes with `None`; the exact no-op-before test
must fail. Restore production byte-for-byte in `finally` and confirm with `cmp`.
Save command, exit status and intended assertion excerpt; exclude any accidental
compiler failures from semantic RED evidence. Re-run focused GREEN after restore.

- [ ] **Step 5: Update canonical docs and M1 progress.**

Document the pure public planner, supplied-observation limit, single-owner affected
region restriction, untouched owner preservation, shared body reader, exact
before bytes and empty aggregate retention. Explain that complete reference-graph,
per-owner consumer association, source/filesystem evidence, whole-file/resource
plans, state/lifecycle and transactional execution remain pending. Add links to
the bounded plan and operational design from the M1 plan. Do not claim CLI/Codex
support or change source/lockfile formats. Operational design is active WIP and
must not be archived when this bounded pair is later completed.

- [ ] **Step 6: Verify, self-review, commit and report.**

Run ordered `just fmt` then `just verify`, followed by
`cargo +1.85.0 check --locked --all-targets`. Full access permits existing tool
caches and loopback integration tests; do not supply `sandbox_permissions`.
Run `git diff --check`. Self-review each acceptance row, shared validator/editor
reuse and input immutability. Commit only task-owned files with
`feat: plan owned instruction contribution changes`. Write full report beside
the task brief: scope, exact commit, RED/GREEN/mutation commands and excerpts,
restoration, gate outcomes, deviations and unswept native/OS/effect boundaries.
Return status, SHA, one-line tests and concerns. Controller owns fresh task and
whole-branch reviews, bounded-pair gardening, PR/FULL passes, CI and merge.
