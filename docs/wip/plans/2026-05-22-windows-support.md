# Windows Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `upskill` work on native Windows (CMD / PowerShell) by fixing the `HOME`-only env-var assumption, adding Windows to the CI test matrix, and updating the install docs.

**Architecture:** A single `home_dir()` helper in `src/source.rs` (the module already responsible for path parsing and tilde expansion) checks `HOME` first, then falls back to `USERPROFILE`. Both call sites (`src/source.rs` tilde expansion and `src/main.rs` global-scope resolution) are updated to use it. The function is exported from `lib.rs` so `main.rs` can call it without duplication. CI gains a `windows-latest` matrix entry on the `test` job.

**Tech Stack:** Rust std (`std::env`, `std::path`), GitHub Actions matrix strategy.

---

## File Map

| File                        | Change                                                                            |
| --------------------------- | --------------------------------------------------------------------------------- |
| `src/source.rs`             | Add `pub fn home_dir()`, update tilde expansion to use it, add unit tests         |
| `src/lib.rs`                | Export `home_dir` alongside the existing `source` re-exports                      |
| `src/main.rs`               | Update `install_target()` to call `home_dir()`, improve error message             |
| `tests/cli_global_scope.rs` | Add two integration tests: USERPROFILE fallback + clear error when neither is set |
| `.github/workflows/ci.yml`  | Add `windows-latest` to `test` job matrix                                         |
| `docs/getting-started.md`   | Document native Windows install path (`cargo install upskill`)                    |

---

## Task 1: `home_dir()` helper in `src/source.rs`

**Files:**

- Modify: `src/source.rs`

### Step 1.1: Write the failing unit tests

Add the following test module at the bottom of `src/source.rs`, just before the final `}` that closes `mod tests`:

```rust
    #[cfg(test)]
    mod home_dir_tests {
        use super::*;

        /// Helper: run `f` with HOME and USERPROFILE set to the given values,
        /// then restore the originals regardless of panic.
        ///
        /// SAFETY: env mutation is inherently racy with parallel tests. These
        /// tests are in a dedicated sub-module so they don't share state with
        /// the tilde-expansion tests above, but the risk of cross-test
        /// interference still exists. Acceptable for this narrow case — the
        /// window is tiny and the alternative is worse (process-wide Mutex).
        fn with_home_env(home: Option<&str>, userprofile: Option<&str>, f: impl FnOnce()) {
            let prev_home = std::env::var_os("HOME");
            let prev_userprofile = std::env::var_os("USERPROFILE");
            unsafe {
                match home {
                    Some(v) => std::env::set_var("HOME", v),
                    None => std::env::remove_var("HOME"),
                }
                match userprofile {
                    Some(v) => std::env::set_var("USERPROFILE", v),
                    None => std::env::remove_var("USERPROFILE"),
                }
            }
            f();
            unsafe {
                match prev_home {
                    Some(v) => std::env::set_var("HOME", v),
                    None => std::env::remove_var("HOME"),
                }
                match prev_userprofile {
                    Some(v) => std::env::set_var("USERPROFILE", v),
                    None => std::env::remove_var("USERPROFILE"),
                }
            }
        }

        #[test]
        fn home_dir_reads_home_var() {
            with_home_env(Some("/home/alice"), None, || {
                assert_eq!(home_dir(), Some(PathBuf::from("/home/alice")));
            });
        }

        #[test]
        fn home_dir_falls_back_to_userprofile() {
            with_home_env(None, Some(r"C:\Users\alice"), || {
                assert_eq!(home_dir(), Some(PathBuf::from(r"C:\Users\alice")));
            });
        }

        #[test]
        fn home_dir_prefers_home_over_userprofile() {
            with_home_env(Some("/home/alice"), Some(r"C:\Users\alice"), || {
                assert_eq!(home_dir(), Some(PathBuf::from("/home/alice")));
            });
        }

        #[test]
        fn home_dir_returns_none_when_neither_set() {
            with_home_env(None, None, || {
                assert_eq!(home_dir(), None);
            });
        }
    }
```

- [ ] **Step 1.2: Run tests to verify they fail**

```bash
cargo test home_dir
```

Expected: compile error — `home_dir` is not yet defined.

- [ ] **Step 1.3: Implement `home_dir()`**

Add the following function to `src/source.rs`, right before `pub fn parse_install_source`:

```rust
/// Resolve the running user's home directory.
///
/// Checks `HOME` first (standard on Unix, Git Bash, and WSL), then falls
/// back to `USERPROFILE` (the canonical home-directory variable on native
/// Windows). Returns `None` when neither variable is set.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
```

- [ ] **Step 1.4: Run tests to verify they pass**

```bash
cargo test home_dir
```

Expected: 4 tests pass, 0 fail.

- [ ] **Step 1.5: Update tilde expansion in `parse_install_source` to use `home_dir()`**

In `src/source.rs`, replace the block at lines 88–100 (the `~/` expansion):

Old:

```rust
if source == "~" || source.starts_with("~/") {
    if let Some(home) = std::env::var_os("HOME") {
        let mut path = PathBuf::from(home);
        if let Some(rest) = source.strip_prefix("~/") {
            path.push(rest);
        }
        return Ok(InstallSource::LocalPath(path));
    }
    // `HOME` unset: fall through and let the path be parsed verbatim.
    // Downstream `LocalPath` handling will fail with a useful filesystem
    // error rather than a confusing "this looks like owner/repo" parse.
    return Ok(InstallSource::LocalPath(PathBuf::from(source)));
}
```

New:

```rust
if source == "~" || source.starts_with("~/") {
    if let Some(mut path) = home_dir() {
        if let Some(rest) = source.strip_prefix("~/") {
            path.push(rest);
        }
        return Ok(InstallSource::LocalPath(path));
    }
    // Neither HOME nor USERPROFILE set: fall through and let the path be
    // parsed verbatim. Downstream `LocalPath` handling will fail with a
    // useful filesystem error rather than a confusing parse error.
    return Ok(InstallSource::LocalPath(PathBuf::from(source)));
}
```

- [ ] **Step 1.6: Run all tests**

```bash
cargo test
```

Expected: all tests pass (the existing `parse_local_path_tilde_expands_home` tests still set `HOME` explicitly, so they continue to work).

- [ ] **Step 1.7: Commit**

```bash
git add src/source.rs
git commit -m "feat(source): add home_dir() helper with USERPROFILE fallback for Windows"
```

---

## Task 2: Export `home_dir` from `lib.rs`

**Files:**

- Modify: `src/lib.rs`

- [ ] **Step 2.1: Add `home_dir` to the re-export line**

In `src/lib.rs`, change line 27:

Old:

```rust
pub use source::{InstallSource, parse_install_source};
```

New:

```rust
pub use source::{InstallSource, home_dir, parse_install_source};
```

- [ ] **Step 2.2: Run tests to verify no regressions**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 2.3: Commit**

```bash
git add src/lib.rs
git commit -m "refactor(lib): export home_dir so main.rs can use it"
```

---

## Task 3: Fix `install_target()` in `src/main.rs`

**Files:**

- Modify: `src/main.rs`

- [ ] **Step 3.1: Update the import line**

In `src/main.rs`, change:

Old:

```rust
use upskill::source::{InstallSource, parse_install_source};
```

New:

```rust
use upskill::source::{InstallSource, home_dir, parse_install_source};
```

- [ ] **Step 3.2: Update `install_target()` to use `home_dir()`**

In `src/main.rs`, find `install_target` (around line 228) and replace the `Scope::Global` arm:

Old:

```rust
Scope::Global => std::env::var_os("HOME")
    .map(PathBuf::from)
    .ok_or_else(|| anyhow::anyhow!("HOME is not set")),
```

New:

```rust
Scope::Global => home_dir()
    .ok_or_else(|| anyhow::anyhow!("HOME (or USERPROFILE on Windows) is not set")),
```

- [ ] **Step 3.3: Run tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 3.4: Commit**

```bash
git add src/main.rs
git commit -m "fix(main): use home_dir() for global scope so USERPROFILE works on Windows"
```

---

## Task 4: Integration tests for `USERPROFILE` fallback and missing-home error

**Files:**

- Modify: `tests/cli_global_scope.rs`

- [ ] **Step 4.1: Write the failing integration tests**

Append to `tests/cli_global_scope.rs`:

```rust
#[test]
fn add_global_uses_userprofile_when_home_unset() {
    // Windows compatibility: when HOME is absent, USERPROFILE should be
    // used as the global install target.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let home = tmp.path().join("fakehome");
    let cwd = tmp.path().join("cwd");
    stage_source(&source);
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(cwd.join(".git")).unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env_remove("HOME")
        .env("USERPROFILE", &home)
        .args(["add", "--global", source.to_str().unwrap()])
        .assert()
        .success();

    assert!(
        home.join(".upskill-lock.json").exists(),
        "global lockfile written under USERPROFILE"
    );
    assert!(
        home.join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists(),
        "global skill output under USERPROFILE"
    );
}

#[test]
fn add_global_errors_clearly_when_neither_home_nor_userprofile_set() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path().join("cwd");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(cwd.join(".git")).unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&cwd)
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .args(["add", "--global", "./nonexistent"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicates::str::contains("USERPROFILE"));
}
```

Also add `use predicates;` at the top of the file if not already present. Check the existing imports first — if `predicates` is not imported, add:

```rust
use predicates::str::ContainsPredicate;
```

Actually `assert_cmd` re-exports `predicates`, so the import is:

```rust
use assert_cmd::prelude::*;  // if not already present
```

Check the file — `assert_cmd::Command` is already imported. Use inline `predicates::str::contains(...)` which works without a `use` statement.

- [ ] **Step 4.2: Run tests to verify they fail**

```bash
cargo test add_global_uses_userprofile
cargo test add_global_errors_clearly
```

Expected: `add_global_uses_userprofile_when_home_unset` FAILS (installs under nothing / exits non-zero before Task 3 is merged, but if running after Task 3 it should already pass). `add_global_errors_clearly_when_neither_home_nor_userprofile_set` passes once Task 3 is done.

> **Note:** Both tests should pass after Tasks 1–3 are complete. If running in order, run `cargo test` here and confirm both new tests pass.

- [ ] **Step 4.3: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 4.4: Commit**

```bash
git add tests/cli_global_scope.rs
git commit -m "test(global-scope): USERPROFILE fallback and missing-home error message"
```

---

## Task 5: Add `windows-latest` to CI test matrix

**Files:**

- Modify: `.github/workflows/ci.yml`

- [ ] **Step 5.1: Update the `test` job to a matrix strategy**

In `.github/workflows/ci.yml`, replace the `test` job:

Old:

```yaml
test:
  name: Test
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v6
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
    - run: cargo test
```

New:

```yaml
test:
  name: Test (${{ matrix.os }})
  strategy:
    matrix:
      os: [ubuntu-latest, windows-latest]
    fail-fast: false
  runs-on: ${{ matrix.os }}
  steps:
    - uses: actions/checkout@v6
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
    - run: cargo test
```

(`fail-fast: false` lets both OS jobs complete so you see all failures at once rather than cancelling Windows when Ubuntu fails, or vice versa.)

- [ ] **Step 5.2: Verify the YAML is valid**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "YAML OK"
```

Expected output: `YAML OK`

- [ ] **Step 5.3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add windows-latest to test matrix"
```

---

## Task 6: Update install docs for native Windows

**Files:**

- Modify: `docs/getting-started.md`

- [ ] **Step 6.1: Replace the Install section with Windows-aware content**

In `docs/getting-started.md`, replace lines 1–22 (the `## Install` section):

Old:

````markdown
# Getting started

## Install

```bash
# Linux / macOS (Windows: run inside WSL)
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh
```
````

`UPSKILL_VERSION` pins a release tag (default: latest) and
`UPSKILL_INSTALL_DIR` overrides the install location (default
`$HOME/.local/bin`). Windows users run the same command inside WSL.

With a Rust toolchain instead:

```bash
cargo install upskill
```

Or download a pre-built binary directly from the releases page.

`upskill` is a single static binary with no runtime dependencies.

````
New:
```markdown
# Getting started

## Install

**Linux / macOS:**

```bash
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh
````

`UPSKILL_VERSION` pins a release tag (default: latest) and
`UPSKILL_INSTALL_DIR` overrides the install location (default
`$HOME/.local/bin`).

**Windows (native — CMD / PowerShell):**

```powershell
cargo install upskill
```

Requires a [Rust toolchain](https://rustup.rs). The binary has no other
runtime dependencies. `HOME` and `USERPROFILE` are both recognised as the
global install root, so the standard Windows `USERPROFILE` variable works
without any extra setup.

**Windows (WSL):** use the Linux install command above inside your WSL
terminal.

Or download a pre-built binary directly from the releases page.

`upskill` is a single static binary with no runtime dependencies.

````
- [ ] **Step 6.2: Verify docs build**

```bash
just book
````

Expected: book builds without errors.

- [ ] **Step 6.3: Commit**

```bash
git add docs/getting-started.md
git commit -m "docs: document native Windows install path via cargo install"
```

---

## Final verification

- [ ] **Run full check suite**

```bash
just check
```

Expected: all tests pass, no lint warnings, no clippy issues.

- [ ] **Confirm commit history looks right**

```bash
git log --oneline main..HEAD
```

Expected (6 commits):

```
docs: document native Windows install path via cargo install
ci: add windows-latest to test matrix
test(global-scope): USERPROFILE fallback and missing-home error message
fix(main): use home_dir() for global scope so USERPROFILE works on Windows
refactor(lib): export home_dir so main.rs can use it
feat(source): add home_dir() helper with USERPROFILE fallback for Windows
```
