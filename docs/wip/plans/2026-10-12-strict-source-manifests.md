# Strict Source Manifests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture complete regular-file source bytes with explicit failure semantics and independently verifiable versioned evidence.

**Architecture:** Add one synchronous opt-in observer and pure validator, using a private filesystem seam for reliable fault tests. Extract the existing ownership path predicate without changing its behavior. Leave current install/discovery/hash dispatch unchanged.

**Tech Stack:** Rust 2024, Rust 1.85, standard filesystem APIs, existing `anyhow` and `sha2`; no new dependencies.

**Spec:** [Strict captured source manifests](../specs/2026-10-12-strict-source-manifests-design.md).

## Global Constraints

- Preserve Rust 1.85 and the synchronous/static-binary constraints.
- Add no dependencies, async runtime, CLI behavior, source schema, lockfile writes or native operations.
- Only main.rs writes to stdout/stderr; capture and validation return data/errors.
- Preserve legacy discovery/hash/install behavior and existing ownership validation errors/order.
- Do not claim source authenticity, atomic snapshot, race-safe containment or qualified filesystem observation.

## Review Focus

- Elevated users can read mode-000 files: inject enumeration/metadata/read failures rather than let permission fixtures silently pass.
- Symlink roots with trailing separators or dot suffixes must not evade observed-root rejection; ancestor links remain explicitly unqualified.
- Unframed path/content combinations can collide: exact version/count/length/mode framing and literal vectors must pin the new digest.
- Special entries must be rejected before opening; no FIFO/socket fixture may hang a test.
- Empty/binary files, nested entrypoint/override names and Unix mode changes are source facts, not discovery exclusions or execution authorization.

---

### Task 1: Strict capture, framed evidence and pure validation

**Files:**

- Create: `src/source_manifest.rs` — public DTOs/capture/validator, private real filesystem implementation and focused injected-failure unit tests.
- Modify: `src/lib.rs` — module export.
- Modify: `src/ownership.rs` — extract current relative-path predicate into `pub(crate) fn validate_relative_path(path: &str) -> anyhow::Result<()>`; use it at the original validation position with unchanged diagnostic.
- Create: `tests/source_manifests.rs` — public acceptance/literal vector/platform tests.
- Modify: `src/pipeline/hash.rs` — documentation only: accurately describe best-effort legacy hashing/read failures; no executable change.
- Modify: `src/pipeline/discovery.rs` — documentation only: accurately describe non-directory/non-symlink entries and skipped traversal errors; no executable change.
- Modify: `docs/specification/specification.md` — implemented library evidence boundary and limits.
- Modify: `docs/book/commands.md` — explain current commands remain unchanged; source capture is a future lifecycle prerequisite.
- Modify: `docs/wip/plans/2026-10-11-instructions-rules-m1.md` — scoped prerequisite progress/link and remaining activation gates.
- Modify: this plan's checkboxes only.
- Write report/logs in this plan's ignored SDD workspace.

**Interfaces:**

```rust
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedSourceFile {
    pub path: String,
    pub bytes: Vec<u8>,
    pub content_sha256: String,
    pub mode: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceManifest {
    pub files: Vec<CapturedSourceFile>,
    pub content_sha256: String,
}
pub fn capture_source(root: &Path) -> anyhow::Result<SourceManifest>;
pub fn validate_manifest(manifest: &SourceManifest) -> anyhow::Result<()>;
```

No serde or trusted-provenance wrapper. Capture reads; validation is pure. Public
fields can be modified, so validation checks all evidence. No resource selection,
Git revision, source-label parsing or destination operations enter this module.

- [ ] **Step 1: Write public acceptance tests before production code.**

Use `tempfile` directly for library tests; no CLI process or real home is needed.
Create two different temporary roots with these files, deliberately writing them
in different traversal orders. On Unix set every file to permission bits `0o640`.
Include an empty directory. Capture both roots and assert the complete exact
file vector and manifest digest; do not compute expected values with production
helpers. Assert file count before indexing. Use these independent vectors:

| Path             | Exact bytes     | Literal SHA-256                                                    |
| ---------------- | --------------- | ------------------------------------------------------------------ |
| `RULE.claude.md` | `b"override\n"` | `46e0313ca59003e4def743b25cd1ed4a1276f99782e5cbf8bfa849ff4cf171db` |
| `a/SKILL.md`     | `b"# Nested\n"` | `aeab417db4e43d83788b35bde1945cdfc5ac1d38879f02403798410b0e37107f` |
| `a/empty`        | `b""`           | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `z.bin`          | `[0, 255, 10]`  | `712450d3c4a79eea9509e75dc1dacdeff58034df538536cfae2da882bd8a0c50` |

For this complete sorted tree, digest with all modes `Some(0o640)` is
`d277aa179cdcdfc083aab28ccace1714cdf239315f9bdf6767764f4c9e27ded1`;
with all modes `None` it is
`24ff8dddb80b667251644a4d68ff5bd8715e1df34fb132b3406b5153ea821cbf`.
An empty root digest is
`ecbfd2081fcd5a8c1c993491058a1913b092eb5cb7d16034f12807969ab4619b`.
Assert Unix mode evidence and non-Unix absence explicitly.

```rust
#[test]
fn empty_source_has_versioned_framed_evidence() {
    let root = tempfile::tempdir().unwrap();
    let captured = upskill::source_manifest::capture_source(root.path()).unwrap();
    assert!(captured.files.is_empty());
    assert_eq!(captured.content_sha256,
        "ecbfd2081fcd5a8c1c993491058a1913b092eb5cb7d16034f12807969ab4619b");
    upskill::source_manifest::validate_manifest(&captured).unwrap();
}
```

Add literal single-file vectors with `mode: None` for pure validation:
path `a`, bytes `bc`, file hash
`1e0bbd6c686ba050b8eb03ffeedc64fdc9d80947fce821abbe5d6dc8d252c5ac`,
manifest hash `1cb8488ccbda1fcef669daabf5ceab19c98708642534db354eacc4c3d4202b14`;
path `ab`, bytes `c`, file hash
`2e7d2c03a9507ae265ecf5b5356885a53393a2029d241394997265a1a25aefc6`,
manifest hash `c8ca3f0b05498df16c899154aec285d419ccdac53d7d10e48db3323aec27187c`.
Both validate, their hashes differ, and substituting either hash into the other
manifest fails its digest diagnostic. These vectors share the same unframed
`path || bytes` stream; framing must distinguish them.

Cover each independent class with an assertion of its diagnostic family so an
unrelated error cannot mask a missing validator:

| Input or operation                                              | Required result                                                                                   |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Rename, byte edit, Unix mode-only edit                          | Expected path/bytes/mode change and manifest digest difference; each resulting manifest validates |
| Empty directory plus binary/empty/nested entrypoints            | No exclusions; exact owned bytes; empty directory has no record                                   |
| Missing root or regular-file root                               | Contextual directory/root error                                                                   |
| Unix root symlink, plus trailing `/` and `/.`                   | Root symlink error for every spelling                                                             |
| Unix regular/directory/broken link and cycle inside root        | Entry symlink error before traversal/read                                                         |
| Unix FIFO via `mkfifo`, Unix socket via `UnixListener`          | Special-entry error before any file read; no blocking open                                        |
| Unix invalid UTF-8 filename                                     | Non-UTF-8 path error                                                                              |
| Unix names containing backslash/colon/control characters        | Lexical-path diagnostic; Windows uses pure validator fixtures for uncreatable names               |
| Duplicate paths or reversed file order                          | Strict-order/duplicate diagnostic, even if manifest hash is stale                                 |
| Empty/dot/parent/absolute/backslash/colon/Unicode-control paths | Lexical-path diagnostic before file/manifest digest rejection                                     |
| Wrong file hash with otherwise valid bytes/mode                 | File-digest diagnostic                                                                            |
| Mode `0o10000`                                                  | Permission-bit diagnostic before manifest digest rejection                                        |
| Wrong manifest hash with valid sorted file evidence             | Manifest-digest diagnostic                                                                        |
| Validating success and every rejection                          | Input values unchanged                                                                            |

Use `#[cfg(unix)]` for actual symlink/mode/special/non-UTF-8 fixtures. On Unix
`mkfifo` is test setup through the OS command, not a product dependency; check its
status. Socket listeners stay alive while capturing. Windows executes portable
capture/validator tests. Do not claim device-entry, native-client, alias or
concurrent replacement qualification.

- [ ] **Step 2: Record compile RED, then genuine assertion RED.**

Before adding the module, run `cargo test --test source_manifests`; save expected
missing-module compile failure. Add only DTOs/module export and minimal signatures:

```rust
pub fn capture_source(_: &std::path::Path) -> anyhow::Result<SourceManifest> {
    Ok(SourceManifest { files: Vec::new(), content_sha256: String::new() })
}
pub fn validate_manifest(_: &SourceManifest) -> anyhow::Result<()> { Ok(()) }
```

Serialize all Cargo runs and source edits. Run focused tests and save actual
assertion failures for complete file count/empty-root literal digest and invalid
manifest validation. Do not draft real observation/validation helpers before
these valid assertions complete. Bounds panics, compile errors and incorrect
fixtures are not assertion RED evidence. Report any deviation honestly.

- [ ] **Step 3: Implement the narrow private filesystem seam and strict capture.**

Keep the seam module-private and synchronous. A suitable concrete contract is:

```rust
trait ObservationIo {
    fn entries(&self, path: &std::path::Path)
        -> std::io::Result<Vec<std::path::PathBuf>>;
    fn metadata(&self, path: &std::path::Path) -> std::io::Result<std::fs::Metadata>;
    fn read(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>>;
}
struct StdIo;
// StdIo::entries uses read_dir(path)?.map(|entry| entry.map(|e| e.path())).collect().
// StdIo::metadata uses symlink_metadata; StdIo::read uses fs::read.
// capture_source calls private capture_with(root, &StdIo).
```

The collector propagates iterator errors, never `flatten()` or `filter_map(ok)`.
Reconstruct root with `let root: PathBuf = root.components().collect();` before
root metadata. Reject observed root symlink/non-directory. Recursively inspect
entry metadata; symlink first, then directory recursion, then regular-file read;
otherwise reject special entry. No read occurs before regular-file classification.
Build each UTF-8 relative path from the walked relative components with `/`, and
call the shared lexical validator. Use contextual errors including operation and
path; return no partial result on errors. Read each file once into the record,
calculate its hash and mode evidence from metadata, sort complete records and
calculate the framed digest. Do not canonicalize or re-read captured source.

Unit tests in this module use a tiny delegating fault implementation of the
private trait. Inject one failure at a selected path for each of enumeration,
metadata and read. Use a tree containing an earlier readable file and the failing
entry; each capture must fail with its intended operation/path context rather
than return a partial manifest. Test read counts with a test-local `RefCell` map
inside the delegating implementation; each regular file read once, special/link
entries never read. Keep fault state local, not environment/global mutation.
Sort entries locally in fault fixtures if deterministic failure ordering needs it;
production correctness must not depend on directory enumeration order.

- [ ] **Step 4: Implement exact digest/validation and unchanged path-rule extraction.**

Extract the existing ownership path conditional unchanged into the crate-private
helper; keep its exact error text and validation position. Source validation
adds source path context around that existing result. Preserve existing ownership
tests and semantics. Update the two legacy helper comments to describe current
best-effort error skipping/non-directory entries accurately; no executable changes.

Use one private hashing implementation for capture and validation, with framing:

```rust
use anyhow::Context;
use sha2::{Digest, Sha256};
fn put_length(hasher: &mut Sha256, length: usize) -> anyhow::Result<()> {
    let encoded = u64::try_from(length).context("source manifest length exceeds u64")?;
    hasher.update(encoded.to_be_bytes());
    Ok(())
}
// Domain b"upskill:source-manifest:v1\0", count, each path length/path,
// byte length/bytes, then [0] or [1] + mode.to_be_bytes().
```

Format lowercase hex with existing crate patterns and no dependency. Validation
checks lexical path, strict ascending uniqueness, mode range and exact file hash
before the aggregate hash, with useful path diagnostics. It borrows inputs and
never sorts them in place. Validate captured output too. Do not introduce state
codecs or broaden ownership rules.

- [ ] **Step 5: Focused GREEN and required semantic controls.**

Run `cargo test --test source_manifests`, `cargo test source_manifest::` and the
`cargo test --test ownership_records --test instruction_plans --test
instruction_contributions` after extraction. Save clean results.
Temporarily make a file-read failure skip its entry and run the injected read-fault
test: its intended error assertion must fail. Separately remove path/content
length framing from the digest and run literal manifest/vector tests: intended
digest assertions must fail. Also omit one real file from captured output and
run the complete-vector acceptance test; file-count/body assertion must fail.
Restore byte-for-byte in `finally`, compare with `cmp`, exclude compiler failures
from semantic evidence and rerun focused GREEN. No temporary mutation is committed.

- [ ] **Step 6: Docs, ordered gates, self-review and commit.**

Describe the library API, exact digest framing, captured byte/mode evidence,
strict observed-entry failure behavior and qualification limits in canonical docs.
Mark only this prerequisite delivered in M1; whole-file/resource plans, immutable
Git evidence, source stabilization, destination qualification, executor/state and
Codex activation remain pending. The operational design remains active WIP.

Run ordered `just fmt` then `just verify`, followed by
`cargo +1.85.0 check --locked --all-targets` and `git diff --check`.
No `sandbox_permissions`. Self-review each acceptance row and no legacy behavior
change. Commit task-owned files with `feat: capture strict source manifests`.
Write full report beside the task brief: exact commit, compile/assertion RED,
GREEN/mutation/restoration commands/output, gate counts/results, OS-limited and
unswept evidence, deviations and concerns. Controller owns fresh task/final
reviews, bounded-pair gardening, PR reviews, CI and merge. Return short status,
commit(s), tests and concerns; no push/PR.
