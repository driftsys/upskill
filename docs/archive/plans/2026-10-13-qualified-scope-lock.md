# Qualified scope roots and writer locking implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pin selected scope directory identity and exclude competing
Linux/macOS writers without creating files.

**Architecture:** Introduce an opaque library module. It selects a trusted
ambient root, retains its OS identity and opens a fresh close-on-exec directory
description for each advisory exclusive guard. No CLI integrates the module.

**Tech Stack:** Rust edition 2024, Rust 1.85, synchronous rustix 1.1.4
filesystem calls.

**Spec:** `docs/design/qualified-scope-locking.md` (durable binding authority);
working pair `docs/wip/specs/2026-10-13-qualified-scope-lock-design.md`.

## Global Constraints

- Retain Rust 1.85, synchronous execution, a single binary, zero warnings and
  library-only presentation.
- Current commands remain on schema 1.
- Opening, validation, lock acquisition and release create no files,
  directories, backups, journals or schema changes and preserve existing bytes
  and modes.
- New APIs return an explicit unsupported error on other operating systems.
- Do not expose the locked handle or allow cloning it.
- No network-filesystem guarantee, descendant containment or protection against
  external editors.
- Existing worktrees and unrelated WIP files belong to other work; leave them
  unchanged.

## Review Focus

- Trailing slash/current-directory spellings must not hide a symlink root (Q02).
- Two independent guards within one process must contend rather than share an
  open description (Q04).
- A non-cooperating root replacement must invalidate binding even while an old
  handle survives (Q06).
- Forced holder termination and exec inheritance must not strand exclusion
  (Q05/Q08).
- Non-UTF-8 root paths must preserve native identity without lossy conversion
  (Q07).

---

### Task 1: Qualified root and writer guard

**Files:**

- Create: `src/filesystem.rs`, `src/filesystem/scope.rs`,
  `tests/filesystem_scope_lock.rs`.
- Modify: `src/lib.rs`, `Cargo.toml`, `Cargo.lock`.
- Update: `docs/design/qualified-scope-locking.md`,
  `docs/adr/0028-lock-qualified-scope-directories.md`, `docs/adr/README.md`,
  `docs/design/README.md`, `docs/specification/specification.md`, book summary
  file as required by existing documentation checker.

**Interfaces:**

- Consumes `ownership::{InstallationRoot, OwnershipScope}` without changing
  those types.
- Produces
  `filesystem::QualifiedScope::open(root: &InstallationRoot) -> anyhow::Result<Self>`,
  `verify_binding(&self) -> anyhow::Result<()>`,
  `try_lock_writer(&self) -> anyhow::Result<ScopeWriterGuard<'_>>` and guard
  `verify_binding(&self) -> anyhow::Result<()>`.
- QualifiedScope and ScopeWriterGuard remain opaque, non-Clone and
  non-serializable; safe Debug may omit raw descriptor and sensitive path
  details.
- No public raw descriptor or path-based write API. The held root may later
  supply crate-private handle-relative access; do not add unused executor
  abstractions now.

- [ ] **Step 1: Acceptance RED.** Add real-filesystem and subprocess examples
      Q01-Q08 from the durable design before code. Compile failure proves the
      missing API, then retain assertion-failure evidence using semantic
      controls after implementation. Include exact bytes, directory entries and
      Unix permission snapshots before/after operations. All waits have a
      deadline; a cleanup guard kills/reaps unfinished children. Child helpers
      in the integration test binary activate only under explicit test
      environment variables and use temporary HOME/USERPROFILE. No CLI helper
      command.

```rust
use upskill::filesystem::QualifiedScope;
use upskill::ownership::{InstallationRoot, OwnershipScope};
let tmp = tempfile::tempdir().unwrap();
let root = InstallationRoot { scope: OwnershipScope::Project, path: tmp.path().to_owned() };
let scope = QualifiedScope::open(&root).unwrap();
let guard = scope.try_lock_writer().unwrap();
assert!(scope.try_lock_writer().is_err());
guard.verify_binding().unwrap();
drop(guard);
assert!(scope.try_lock_writer().is_ok());
```

Run `cargo test --locked --test filesystem_scope_lock`. Save missing-API output
before implementation. Run focus cases individually during iteration.

- [ ] **Step 2: Implement root selection.** Validate absolute root, reject
      ParentDir components and normalize remaining components before symlink
      metadata. Require directory and no final symlink. Select canonical ambient
      path, open it using rustix O_DIRECTORY/O_NOFOLLOW/O_CLOEXEC, compare
      descriptor identity against selected nonsymlink metadata, retain
      normalized original path, resolved path and device/inode identity.
      Reopen/revalidate selected root and canonical path for verify_binding. If
      the filesystem refuses a required operation, return contextual error. Do
      not claim that this pins ancestor namespace or descendants.

```rust
// Target-gated Linux/macOS backend sketch, not a mandate to duplicate logic.
use rustix::fs::{open, Mode, OFlags};
let fd = open(&resolved, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC, Mode::empty())?;
let file = std::fs::File::from(fd);
// Compare device/inode from File::metadata with the selected root metadata.
```

Use target-specific `rustix = { version = "=1.1.4", features = ["fs"] }`; use
the existing lock resolution and no new async dependencies. Other OS backend
must return explicit unsupported errors and compile warning-free without rustix
imports.

- [ ] **Step 3: Implement guard.** Validate binding, open `.` relative to the
      pinned root with a fresh open description and O_CLOEXEC, compare identity,
      call nonblocking exclusive flock, then verify root binding again. Return
      an opaque borrowing guard that owns only that fresh descriptor;
      close-on-drop releases the lock. Context distinguishes contention from
      unsupported/permission errors. Do not File::try_clone or explicitly unlock
      a shared description.

```rust
use rustix::fs::{flock, openat, FlockOperation, Mode, OFlags};
let lock_fd = openat(&root_file, ".", OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC, Mode::empty())?;
flock(&lock_fd, FlockOperation::NonBlockingLockExclusive)?;
// Retain lock_fd privately in the guard; no explicit unlock is needed.
```

- [ ] **Step 4: GREEN and semantic RED.** Run all focus cases. Mutate one
      responsibility at a time: omit flock (same-process and child contention
      assertions fail); replace fresh openat with try_clone
      (independent-description assertion fails); omit root binding checks
      (replacement assertion fails); omit CLOEXEC (live child-exec inheritance
      assertion fails). Restore code after each control and retain failure
      output and final green output. Where a control needs a stronger fixture,
      strengthen that assertion rather than claiming coverage from a compile
      error.

- [ ] **Step 5: Evidence and documentation.** Run complete Rust tests once, Rust
      1.85 all-target check and clippy. Measure same-toolchain release CLI
      baseline and candidate bytes. Build two isolated temporary release probe
      packages with the repository release profile: matched argument/path/result
      scaffolding, one calls root pin/lock/verify and one performs only basic
      path checking. Measure both and state the delta includes root/lock/error
      machinery. No probe source enters the shipped CLI. Record exact compiler,
      target and bytes in durable design. Clearly mark hosted Linux and
      unsupported Windows evidence pending until actual CI. Add scope-lock
      section in canonical specification with limits and AD-0028/design
      navigation links. No user command documentation claims activation.

- [ ] **Step 6: Commit and report.** Use Conventional Commits. Report full
      RED/GREEN commands, mutations, files, measurements, self-review,
      limitations and exact commits in the SDD report. Root controller then
      dispatches fresh task reviewer, final branch reviewer, gardening of only
      this working pair, `just fmt` and `just verify`, numbered PR reviews and
      hosted CI before authorized merge. Preserve scratch evidence under
      delegated existing retention preference.
