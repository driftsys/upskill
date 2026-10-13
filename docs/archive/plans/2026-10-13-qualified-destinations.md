# Qualified destination observation implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture and revalidate exact destination observations beneath one pinned physical scope without filesystem application writes.

**Architecture:** An opaque snapshot borrows QualifiedScope and retains independently opened root, directory and file evidence. Descriptor-relative no-follow reads and mount qualification reject unsafe paths; conservative component comparisons reject potentially overlapping candidates before traversal.

**Tech Stack:** Rust edition 2024, Rust 1.85 minimum, synchronous standard-library I/O, existing target-specific rustix 1.1.4 with fs support, anyhow, tempfile tests.

**Spec:** `docs/wip/specs/2026-10-13-qualified-destinations-design.md`. Binding durable contract: `docs/design/qualified-destination-observation.md`; decision: `docs/adr/0029-capture-qualified-destination-evidence.md`.

## Global constraints

- Rust 1.85, synchronous execution, zero warnings and library-only presentation.
- Reuse rustix 1.1.4; add no dependency, async runtime or handwritten unsafe syscall binding.
- Qualify APFS on macOS and ext-family filesystems and tmpfs on Linux. Reject other filesystems, cross-mount traversal and unknown qualification evidence explicitly.
- Linux requires a returned statx mount ID, not a device-number approximation.
- Windows qualification remains pending; current commands and schema 1 remain unchanged.
- Preview performs no application writes; do not create comparison probes, directories, locks, state, backups or journals.
- Preserve native root paths and exact file bytes; supplied observations convey no execution or permission authority.
- Do not modify the unrelated WIP inventory or another session's worktree.
- Existing user-delegated implementation/review/merge loop and evidence retention remain in force.

## Review focus

1. A different mount can share a device number: independently pin mount-ID/binding checks, including absent mount-ID evidence; no privileged mount experiment is authorized.
2. A relative path can be valid Unicode while its normalization aliases another name: independently pin conservative Unicode overlap and exact stored-spelling rejection.
3. A file may change bytes without changing length or its restored mtime: exact bytes and change-time evidence must reject it.
4. A missing ancestor can become a symlink or directory: retain the first missing component and fail verification on either transition without creating anything.
5. A FIFO or non-not-found syscall failure must not become an absent observation: classify only ENOENT as absence and reject static nonregular entries before opening them.

Each condition is covered in Task 1 acceptance/unit tests and semantic controls below. Privileged cross-mount behavior, hostile namespace relocation and exotic filesystems remain explicit evidence limits.

## File structure

- `src/filesystem.rs`: private destination module and public snapshot re-export.
- `src/filesystem/scope.rs`: small private capability to reopen the bound root as a fresh directory File; preserve all existing public scope/locking behavior and its tests.
- `src/filesystem/destinations.rs`: public snapshot and QualifiedScope observation method, deterministic orchestration and redacted Debug.
- `src/filesystem/destinations/candidates.rs`: portable validation, sorting and conservative overlap relation.
- `src/filesystem/destinations/backend.rs`: cfg Linux/macOS profile/mount qualification, descriptor-relative enumeration/open/read, retained identity stamps and revalidation. Split another private file by responsibility if this backend becomes difficult to review; no generic helper bucket.
- `tests/filesystem_destinations.rs`: public real-filesystem acceptance D01-D09, isolated temporary roots, bounded child only if needed for a hang-sensitive fixture.
- `docs/specification/specification.md`: implemented §4.8 contract after code passes.
- Durable design/AD-0029: truthful implementation/evidence status and limits after code passes.

## Task 1: Capture and revalidate destination evidence

**Files:** All source/test/durable documentation files listed above. Controller owns this plan, working spec, branch/GitHub coordination and scratch ledger.

**Interfaces:**

Consumes `filesystem::QualifiedScope`, `ownership::InstallationRoot`,
`ownership::validate_relative_path`, and `files::plan::{FileLocation, ObservedFile, FileValue}`.

Produces:

```rust
impl QualifiedScope {
    pub fn observe_destinations<'scope>(
        &'scope self,
        locations: &[FileLocation],
    ) -> anyhow::Result<DestinationSnapshot<'scope>>;
}

impl DestinationSnapshot<'_> {
    pub fn observations(&self) -> &[ObservedFile];
    pub fn verify_unchanged(&self) -> anyhow::Result<()>;
}
```

Private implementation may choose its exact structs, but must retain the
qualified scope borrow, selected additional root bindings, mount evidence,
directory handles/identities, present file handles/stamps or first-missing
component plus suffix, and exact observations. No private handle escapes the
filesystem module. A private `QualifiedScope::reopen_directory() -> Result<File>`
may return a freshly selected directory only after the existing original-spelling,
canonical-path and pinned-identity comparisons pass. Reuse those comparisons
rather than introducing weaker alternate scope validation.

- [x] **Step 1: Write acceptance tests that cannot compile before the API exists.**

Start `tests/filesystem_destinations.rs` with cfg Linux/macOS cases and these helpers:

```rust
use std::path::Path;
use upskill::filesystem::QualifiedScope;
use upskill::files::plan::FileLocation;
use upskill::ownership::{InstallationRoot, OwnershipScope};

fn root(path: &Path) -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: path.to_owned(),
    }
}

fn location(path: &Path, relative: &str) -> FileLocation {
    FileLocation { root: root(path), path: relative.to_owned() }
}

#[test]
fn d01_capture_exact_binary_bytes_and_mode() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("data"), b"exact\0\xff\n").unwrap();
    std::fs::set_permissions(tmp.path().join("data"),
        std::fs::Permissions::from_mode(0o640)).unwrap();
    let scope = QualifiedScope::open(&root(tmp.path())).unwrap();
    let snapshot = scope.observe_destinations(&[location(tmp.path(), "data")]).unwrap();
    let value = snapshot.observations()[0].value.as_ref().unwrap();
    assert_eq!(value.bytes, b"exact\0\xff\n");
    assert_eq!(value.mode, Some(0o640));
    snapshot.verify_unchanged().unwrap();
}
```

Extend D01 with the recursive bytes/entries/Unix-mode snapshot pattern from
`tests/filesystem_scope_lock.rs`. Do not compare access time. Add D02 absent leaf
and `missing/parent/file` observations, verifying that no entries are created;
test empty snapshots and unchanged scope writer-lock behavior independently.

- [x] **Step 2: Run acceptance RED and retain the exact failure.**

Run `cargo test --locked --test filesystem_destinations` with a temporary HOME
and the restricted PATH in the controller brief. Expected RED is the missing
`observe_destinations` method, before implementation. Capture the command/output
and distinguish compile RED from later assertion controls.

- [x] **Step 3: Pin portable validation, sorting and overlap with unit examples.**

The component relation has this exact behavior:

```rust
fn may_match(a: &str, b: &str) -> bool {
    !a.is_ascii() || !b.is_ascii() || a.eq_ignore_ascii_case(b)
}

fn may_overlap(a: &str, b: &str) -> bool {
    a.split('/').zip(b.split('/')).all(|(a, b)| may_match(a, b))
}

#[test]
fn potential_overlap_examples() {
    assert!(may_overlap("a/X", "a/x"));
    assert!(may_overlap("a/é", "a/e"));
    assert!(may_overlap("a/x", "a/x/y"));
    assert!(!may_overlap("a/é", "b/e"));
    assert!(!may_overlap("a/b", "ab/c"));
}
```

Validate paths first, then compare every pair including roots with different
labels/spellings. Task 1 first proves all roots physically match the borrowed
scope before reading descendants. Sort by `(root.scope, root.path.as_os_str(),
path)` using owned raw OsString keys where necessary. Reject duplicate inputs,
potential aliases and prefix pairs, and test permutations. Preserve exact root
and path data in the returned observations; never canonicalize ownership records.

- [x] **Step 4: Implement root and mount qualification with focused unit gates.**

Reopen and revalidate the borrowed scope, then qualify that descriptor. Linux
uses rustix `fstatfs` and `statx(fd, "", AtFlags::EMPTY_PATH, StatxFlags::MNT_ID)`;
require the returned mask to contain MNT_ID. Accept ext-family magic `0xef53`
and tmpfs magic `0x01021994`. macOS fstatfs must identify `apfs`; retain filesystem
ID and mount binding. Use returned metadata fields, not pathname prefix guesses.
Compare every descendant's mount evidence to that root. Retain additional
QualifiedScope bindings for location-root aliases, compare physical identities
and mount evidence, and fail different roots before descendant reads.

Factor only the small metadata validators needed to independently test unknown
profile rejection, missing mount-ID mask and same-device/different-mount-ID
rejection. These unit tests do not establish real privileged mount behavior.
Real D01/D02 must qualify the actual local/hosted filesystem, never silently skip
an unsupported profile. Other OS implementations return explicit unsupported
errors and keep all public capability types opaque.

- [x] **Step 5: Implement descriptor-relative capture and special-file rejection.**

Use rustix `Dir::read_from` for exact raw component enumeration and `statat` with
SYMLINK_NOFOLLOW for entry metadata. Open intermediate directories using
DIRECTORY, NOFOLLOW, CLOEXEC and RDONLY. Compare identities and mount evidence;
retain handles. Inspect the final entry's type before opening; only regular,
single-link files proceed to a NOFOLLOW, CLOEXEC, NONBLOCK, RDONLY open.
Compare metadata with the opened handle and retain it. Read exact binary bytes
and require stable identity/type/link-count/size/mode/mtime/ctime around capture.
Only ENOENT creates an absent proof, with the first missing component and all
remaining suffix components. EACCES, ENOTDIR, EIO and unsupported errors stay
contextual errors. Do not traverse an ambient joined descendant path.

Add D03 invalid path examples `""`, `"."`, `"../x"`, `"a/../x"`, `"/x"`,
`"a//x"`, `"a\\x"`, `"a:x"`, and a control-character component. Add a different
physical root, equivalent ancestor-alias root spellings and User/Project labels.
Add D04 inside/outside symlinks at final and intermediate components, file
ancestors, directory leaves, an outside hard link and a Unix socket. A Linux FIFO
fixture uses `rustix::fs::mkfifoat(CWD, absolute_path, Mode::RUSR | Mode::WUSR)`.
On macOS, rustix does not export that helper: use a bounded `/usr/bin/mkfifo`
subprocess inside the temporary fixture with temporary HOME/USERPROFILE, captured
output and deadline/kill/reap cleanup. This is test setup only, with no product
subprocess dependency. Bound any hang-sensitive observation in a child that is
killed/reaped on timeout.
No privileged device-node or mount creation is authorized. Pin nonregular type
classification separately if a device fixture cannot be made safely.

- [x] **Step 6: Implement exact revalidation and adversarial changes.**

Before returning and from `verify_unchanged`, revalidate scope/root mount
bindings, walk fresh no-follow descriptors and compare every retained directory
identity, exact stored spelling, mount and final identity/absence. Re-read
present file bytes through a fresh independent descriptor and compare exact
observations and retained file stamps. Read access-time changes are irrelevant;
directory timestamp changes from unrelated siblings are irrelevant. Never
replace stale expected evidence with new evidence.

Use a fresh fixture per D07 mutation:

```rust
let before = std::fs::metadata(&file).unwrap();
std::fs::write(&file, b"same-size-change").unwrap();
std::fs::File::options().write(true).open(&file).unwrap()
    .set_times(std::fs::FileTimes::new().set_modified(before.modified().unwrap()))
    .unwrap();
assert!(snapshot.verify_unchanged().is_err());
```

Choose equal-length initial/changed literals and assert their lengths match.
Also test chmod, atomic identical-byte replacement, parent rename/replacement,
missing-to-directory, missing-to-symlink, final missing-to-file, selected root
replacement and ancestor-root alias retargeting. A sibling edit must leave a
snapshot valid. Root and directory identity checks must each have an independent
test; one must not accidentally mask the other.

- [x] **Step 7: Add alias, failure and capability boundary tests.**

Add D05 exact duplicates, case-only ASCII names, NFC/NFD Unicode pairs, Unicode
versus ASCII uncertainty, every prefix direction, and absent-parent pairs across
root labels/spellings. Accept `a/x` with `b/y`, single exact Unicode paths and
distinct prefixes despite Unicode descendants. Where the filesystem resolves
alternate case/normalization spellings, D06 must reject the alias's nonexact
stored spelling; characterize case-sensitive refusal distinctly. Native root
names are conditional on filesystem support, as AD-0028 already records.

D08 checks error cleanup by dropping a failed result and reacquiring the existing
scope lock, while also pinning retained descriptor closure through focused
backend evidence tests. Lock reacquisition alone does not prove unrelated read
descriptor closure. Use a bounded local descriptor-count or weak-lifetime fixture
only where it checks an actual ownership path; do not introduce a public testing
API. The backend's returned-error classification must reject every tested
non-ENOENT error instead of producing absence. Permission-fixture limitations
under elevated execution must be reported, never counted as proven.

D09 uses separate compile-fail rustdoc examples for external literal
construction and `Clone`, and focused private descriptor flags checks for every
retained descriptor class. No lock is implicitly acquired by observation; test
observation under a held guard and unchanged same-process writer contention.

- [x] **Step 8: Run independent semantic controls and restore exactly.**

One mutation at a time, capture an assertion failure from: omission of directory
NOFOLLOW in a direct private opener test with a real directory symlink; omission of conservative pair rejection; omission of
exact-byte verification; treating an inserted first-missing component as still
absent; omission of profile/mount-ID/missing-mask rejection; omission of retained
descriptor CLOEXEC. Separate obligation checks must not mask one another. Use a
private acquisition phase hook only for a deterministic namespace-replacement
ordering case that existing public mutations cannot isolate. No broad mocked I/O
trait is authorized. Restore original source bytes in `finally`, and confirm
source hashes before the final green run. Pin the exact-byte comparison with
different expected bytes against an unchanged real file through the production
comparison helper; ordinary edit fixtures alone can be masked by ctime checks.
Likewise keep unknown profile, missing mount-ID mask and different mount ID as
separate unit inputs and separate controls. Every subprocess must have a deadline
and be killed/reaped; do not touch unrelated processes.

- [x] **Step 9: Complete docs, full gates and size evidence.**

Update specification §4.8, durable design and AD-0029 to describe exactly the
implemented rejection, sampled guarantees and pending gates. Keep the raw
working pair unchanged after it is archived by the controller's gardener.
Run `just fmt`, `just verify`, `cargo +1.85.0 check --locked --all-targets`, focused
destination/scope tests and warning-denying clippy. Retain exact output and
unexercised platform/privileged/failure paths. Existing CLI tests must stay green.

Measure release CLI at baseline 8298386 and candidate with one identical toolchain
and target. Also use matched standalone release probes that create the same
temporary input: control constructs its path/data only; live calls QualifiedScope
open, observe_destinations and verify_unchanged and consumes the captured bytes.
Record toolchain, exact bytes, code/data/linker interpretation and probe sources.
Do not claim CLI dead-code elimination measures the active snapshot cost.

- [x] **Step 10: Commit and report for independent task review.**

```bash
git add src/filesystem.rs src/filesystem/scope.rs src/filesystem/destinations.rs src/filesystem/destinations tests/filesystem_destinations.rs docs/specification/specification.md docs/design/qualified-destination-observation.md docs/adr/0029-capture-qualified-destination-evidence.md
git commit -m "feat: capture qualified destination observations"
```

Report exact commit SHAs, API and file responsibilities, acceptance RED and
GREEN, each assertion-failure control, MSRV/full gates, exact size evidence and
residual limits. Do not push or open a competing PR. Controller dispatches a
fresh task reviewer, corrects retained findings through the implementer, gardens
the owned working pair, runs a final whole-branch review, then opens the PR for
FULL numbered review and exact-head hosted CI before the delegated merge.
