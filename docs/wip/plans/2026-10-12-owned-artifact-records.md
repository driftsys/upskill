# Owned artifact records implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development to implement this task, then the Codex PR review workflow before merge.

**Goal:** Give the M1 lifecycle a validated, serializable record of exact owned files and Markdown contributions, without activating schema-2 writes before lifecycle integration.

**Architecture:** Add a pure `ownership` library module. An artifact belongs to a concrete installation root and relative path, records either whole-file bytes/mode or the existing Markdown editor's contribution record, and carries explicit installation owners and consuming target surfaces. Validation and a deterministic JSON codec supply the boundary later state integration will use; they perform no filesystem operations.

**Tech Stack:** Rust edition 2024, existing serde/serde_json/anyhow; no new dependency.

**Spec:** [Foundation design §3 and §5](../specs/2026-10-11-agent-customization-foundation-design.md), [M1 plan](2026-10-11-instructions-rules-m1.md).

## Global constraints

- Keep the single static binary architecture and Rust MSRV 1.85; add no dependencies.
- Only `main.rs` writes to stdout/stderr. This module returns records or `Result<T>`.
- Preserve exact source item names separately from installed aliases and installation identities.
- Whole-file ownership and ownership of an aggregate region are distinct contracts.
- Record each owned file and mode; never infer directory ownership.
- Schema 1 remains readable. Schema 2 writing begins only once content lifecycle can preserve these records on add, update, remove, list, and doctor.
- This task does not change CLI behavior, lockfile schema, native discovery, or installed files. Filesystem canonicalization, case/alias checks, symlink containment, source verification, and write locking belong to the executor/state integration tasks.

## Review focus

1. Distinct installation roots with equal relative paths are different artifacts; equal roots and selectors collide.
2. A whole-file owner cannot coexist with any contribution owner for the same destination.
3. A repeated installation identity must retain the same source/item/alias binding across records.
4. Invalid paths and unknown serialized fields cannot enter validated records through the codec.
5. Contribution hashes and both newline counts survive round trips, including zero owned newlines and a manual prefix.

## Task 1: Validated owned artifact records and codec

**Files:**

- Create `src/ownership.rs` and `tests/ownership_records.rs`.
- Modify `src/lib.rs` to export `ownership`.
- Modify `src/target.rs` to serialize target surfaces as their existing stable names and reject unknown names on deserialization.
- Modify `src/instructions/aggregate.rs` to derive strict serde traits for `OwnedContribution`; retain its existing field semantics.
- Update `docs/wip/plans/2026-10-11-instructions-rules-m1.md` with the delivered M1-A2a boundary and pointers to this plan.

**Interfaces:**

Consume `TargetSurface::name`/`FromStr`, `ItemKind`, and `OwnedContribution`. Produce these public definitions in `upskill::ownership`:

```rust
pub enum OwnershipScope { Project, User }
pub struct InstallationRoot {
    pub scope: OwnershipScope,
    pub path: std::path::PathBuf,
}
pub struct ArtifactOwner {
    pub installation_id: String,
    pub source: String,
    pub source_revision: Option<String>,
    pub kind: upskill::pipeline::ItemKind,
    pub source_name: String,
    pub installed_name: String,
}
pub enum ArtifactContent {
    File { content_sha256: String, mode: Option<u32> },
    MarkdownContribution { contribution: upskill::instructions::aggregate::OwnedContribution },
}
pub struct OwnedArtifact {
    pub root: InstallationRoot,
    pub path: String,
    pub content: ArtifactContent,
    pub owners: Vec<ArtifactOwner>,
    pub consumers: Vec<upskill::target::TargetSurface>,
}
pub fn validate_artifacts(artifacts: &[OwnedArtifact]) -> anyhow::Result<()>;
pub fn parse_artifacts(json: &str) -> anyhow::Result<Vec<OwnedArtifact>>;
pub fn serialize_artifacts(artifacts: &[OwnedArtifact]) -> anyhow::Result<String>;
```

Types derive equality, clone, debug, and serde as appropriate. Use strict unknown-field rejection. `OwnershipScope` serializes as `project`/`user`; `ArtifactContent` is tagged with `kind: file` or `kind: markdown-contribution`. Target surfaces serialize through existing stable `name()`/`FromStr` behavior rather than a second string table. The codec handles an internal JSON array of artifact records, not a schema-2 lockfile.

- [ ] **Step 1: Write acceptance tests first.** Start with round-trip preservation of a source name differing from its installed alias, a source revision, two owners, and a Markdown contribution. Pin both newline counts and the known SHA-256 from the editor fixtures. Use the public codec, not a mock. The intended regression is dropping or rewriting any ownership field.

```rust
let json = serialize_artifacts(&records).unwrap();
let decoded = parse_artifacts(&json).unwrap();
assert_eq!(decoded, records);
```

Also define examples for every validation rule below before implementing its branch. Demonstrate at least one assertion failure using a compiling placeholder validator/codec, then implement the real behavior; compile errors alone are not the RED evidence. Record the command and failure in the task report.

- [ ] **Step 2: Implement the smallest pure model and boundary.** Roots must have nonempty absolute paths according to the current host's `Path::is_absolute`; this is lexical validation, not canonicalization. Artifact paths use portable forward-slash relative components: reject empty paths/components, `.`, `..`, backslashes, colons, NUL/control characters, absolute paths, and trailing slash. Require at least one owner and one consumer. Reject duplicate owner installation IDs within a record, duplicate consumers, empty owner identity/source/name fields, and empty present source revisions. IDs use ASCII letters/digits/hyphen/underscore. SHA-256 fields are exactly 64 lowercase hexadecimal characters. File modes, when present, contain permission bits only (`0..=0o7777`). Contribution IDs follow the editor marker grammar and `leading_newlines` is `0..=2`; `preceding_newlines` is preserved without imposing a guessed limit.

- [ ] **Step 3: Validate the set, not just records.** Compare exact lexical `(root scope, root path, relative path)` locations. Reject duplicate file selectors, duplicate contribution IDs at one location, and any file/contribution mixture at one location. Allow distinct contribution IDs at one location and equal relative paths under distinct roots. Across all records, the same installation ID must have an identical owner binding. Do not infer ownership from bytes, filenames, consuming surfaces, or source hashes.

- [ ] **Step 4: Make serialization deterministic and strict.** `parse_artifacts` parses then validates. `serialize_artifacts` validates then sorts a cloned record set by root scope/path, relative path, and selector; it also sorts owners by installation ID and consumers by their stable names. It emits pretty JSON with one trailing newline. It does not mutate its input. Test that differently ordered equivalent inputs produce identical output, and that malformed/unknown fields or target surfaces fail with contextual errors. Tests inspect preserved values rather than merely comparing counts.

- [ ] **Step 5: Complete the meaningful test matrix.** Exercise all five review-focus cases, empty owner/consumer sets, unknown target strings, invalid hashes/modes, path variants, and owner-binding conflicts. Pin valid multi-contribution sharing and distinct-root behavior so rejecting everything cannot satisfy the suite. Include a contribution with `leading_newlines: 0` and a nonzero preceding count.

- [ ] **Step 6: Update the boundary documentation and verify.** Document the internal record format, lack of CLI/schema activation, lexical validation limits, and follow-on lifecycle work in the M1 plan. Run `cargo test --test ownership_records`, existing target/editor tests, then `just fmt` and `just verify`. Check the diff for unrelated changes and commit with Conventional Commits.

## Follow-on sequence

The next task projects legacy lockfile state without fabricating ownership or requested surfaces. Subsequent tasks integrate the records across all content lifecycle commands, then enable schema-2 writes together with migration backup and the M1-A3 preflight/journal boundary. This task alone is not schema-2 support or complete M1-A2.
