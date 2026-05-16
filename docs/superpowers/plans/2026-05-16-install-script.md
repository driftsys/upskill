# install.sh + Release Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a `curl … | sh` install path for `upskill` backed by a GitHub Releases workflow that builds a 4-target prebuilt-binary matrix.

**Architecture:** A POSIX `install.sh` at the repo root detects OS/arch, downloads the matching `.tar.gz` from GitHub Releases (using the `releases/latest/download` redirect, or a pinned tag via `UPSKILL_VERSION`), verifies a sha256 sidecar, and installs the binary to `$HOME/.local/bin`. A tag-triggered `release.yml` builds `x86_64`/`aarch64` for Linux (musl, static) and macOS, archives each, writes checksums, and publishes them with `softprops/action-gh-release`. Windows is covered via WSL (Linux path) — no native Windows artifact.

**Tech Stack:** POSIX `sh`, `shellcheck`, GitHub Actions (`dtolnay/rust-toolchain`, `taiki-e/install-action` for `cross`, `softprops/action-gh-release`), Rust integration tests (`assert_cmd` + `predicates` + `tempfile`).

Spec: `docs/superpowers/specs/2026-05-16-install-script-design.md`

---

## File Structure

| File                                                         | Responsibility                                              | Action               |
| ------------------------------------------------------------ | ----------------------------------------------------------- | -------------------- |
| `install.sh`                                                 | POSIX installer: detect platform, download, verify, install | Create (repo root)   |
| `tests/cli_install.rs`                                       | ATDD: `--help` and unsupported-platform behavior            | Create               |
| `.github/workflows/release.yml`                              | Tag-triggered build matrix + release publish                | Create               |
| `justfile`                                                   | Add `shellcheck install.sh` to the `lint` recipe            | Modify               |
| `.github/workflows/ci.yml`                                   | Add `shellcheck` job; add it to the `ci` gate               | Modify               |
| `README.md`                                                  | Add the `curl` one-liner above `cargo install`              | Modify (lines 15-16) |
| `docs/getting-started.md`                                    | Add the `curl` one-liner + env-var note                     | Modify (lines 3-11)  |
| `docs/superpowers/specs/2026-05-16-install-script-design.md` | Flip `Status` to approved                                   | Modify (line 4)      |

---

## Task 0: Tooling — ensure `shellcheck` is available

`shellcheck` is not installed in the dev environment but is required by the
new `lint` step and CI. Install it once before doing script work.

**Files:** none.

- [ ] **Step 1: Install shellcheck**

Run:

```bash
sudo apt-get update && sudo apt-get install -y shellcheck
```

Expected: exit 0.

- [ ] **Step 2: Verify it runs**

Run:

```bash
shellcheck --version
```

Expected: prints `ShellCheck - shell script analysis tool` and a version line.

No commit (environment-only change).

---

## Task 1: ATDD tests for `install.sh`

Write the acceptance tests first. They will fail because `install.sh` does
not exist yet.

**Files:**

- Create: `tests/cli_install.rs`

- [ ] **Step 1: Write the failing test file**

Create `tests/cli_install.rs` with exactly this content:

```rust
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::str::contains;

fn install_sh() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("install.sh")
}

#[test]
fn help_flag_prints_usage_and_exits_zero() {
    Command::new("sh")
        .arg(install_sh())
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Usage"))
        .stdout(contains("UPSKILL_VERSION"));
}

#[test]
fn unsupported_platform_fails_with_cargo_hint() {
    let tmp = tempfile::tempdir().unwrap();
    let fake_uname = tmp.path().join("uname");
    fs::write(
        &fake_uname,
        "#!/bin/sh\ncase \"$1\" in\n  -s) echo Linux ;;\n  -m) echo riscv64 ;;\n  *) echo unknown ;;\nesac\n",
    )
    .unwrap();
    let mut perm = fs::metadata(&fake_uname).unwrap().permissions();
    perm.set_mode(0o755);
    fs::set_permissions(&fake_uname, perm).unwrap();

    let orig_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", tmp.path().display(), orig_path);

    Command::new("sh")
        .arg(install_sh())
        .env("PATH", new_path)
        .assert()
        .failure()
        .stderr(contains("cargo install upskill"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test --test cli_install
```

Expected: FAIL — both tests error because `sh` cannot open `install.sh`
(`No such file or directory`), so the commands do not succeed / do not emit
the expected output.

- [ ] **Step 3: Commit**

```bash
git add tests/cli_install.rs
git commit -m "test: add ATDD coverage for install.sh"
```

---

## Task 2: Implement `install.sh`

Make Task 1's tests pass and keep the script `shellcheck`-clean.

**Files:**

- Create: `install.sh`

- [ ] **Step 1: Write the script**

Create `install.sh` at the repo root with exactly this content:

```sh
#!/bin/sh
# upskill installer — downloads a prebuilt release binary.
#
#   curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh
#
# Windows users: run this inside WSL.
set -eu

REPO="driftsys/upskill"
BIN="upskill"
INSTALL_DIR="${UPSKILL_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${UPSKILL_VERSION:-latest}"

usage() {
    cat <<EOF
upskill installer

Usage:
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh

Environment:
  UPSKILL_VERSION       Release tag to install (default: latest)
  UPSKILL_INSTALL_DIR   Install directory (default: \$HOME/.local/bin)

Options:
  --help, -h            Show this help and exit
EOF
}

err() {
    echo "install.sh: $*" >&2
    exit 1
}

download() {
    # $1 = url, $2 = destination path
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    else
        wget -q "$1" -O "$2"
    fi
}

verify() {
    # $1 = checksum file (sibling archive must be in the cwd)
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -c "$1"
    else
        shasum -a 256 -c "$1"
    fi
}

for arg in "$@"; do
    case "$arg" in
    --help | -h)
        usage
        exit 0
        ;;
    *)
        err "unknown argument: $arg (try --help)"
        ;;
    esac
done

os="$(uname -s)"
arch="$(uname -m)"
case "${os}/${arch}" in
Linux/x86_64) target="x86_64-unknown-linux-musl" ;;
Linux/aarch64 | Linux/arm64) target="aarch64-unknown-linux-musl" ;;
Darwin/x86_64) target="x86_64-apple-darwin" ;;
Darwin/arm64) target="aarch64-apple-darwin" ;;
*) err "no prebuilt binary for ${os}/${arch}; install with: cargo install upskill" ;;
esac

if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
    err "need curl or wget to download releases"
fi
if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
    err "need sha256sum or shasum to verify the download"
fi

asset="${BIN}-${target}.tar.gz"
if [ "$VERSION" = "latest" ]; then
    base="https://github.com/${REPO}/releases/latest/download"
else
    base="https://github.com/${REPO}/releases/download/${VERSION}"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

echo "Downloading ${asset} (${VERSION})..." >&2
download "${base}/${asset}" "${tmp}/${asset}" ||
    err "download failed: ${base}/${asset}"
download "${base}/${asset}.sha256" "${tmp}/${asset}.sha256" ||
    err "checksum download failed: ${base}/${asset}.sha256"

echo "Verifying checksum..." >&2
(cd "$tmp" && verify "${asset}.sha256") >/dev/null 2>&1 ||
    err "checksum verification failed for ${asset}"

tar -xzf "${tmp}/${asset}" -C "$tmp"
mkdir -p "$INSTALL_DIR"
mv "${tmp}/${BIN}" "${INSTALL_DIR}/${BIN}"
chmod +x "${INSTALL_DIR}/${BIN}"

echo "Installed ${BIN} to ${INSTALL_DIR}/${BIN}" >&2
case ":${PATH}:" in
*":${INSTALL_DIR}:"*) ;;
*)
    echo "Note: ${INSTALL_DIR} is not on your PATH. Add it with:" >&2
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\"" >&2
    ;;
esac

"${INSTALL_DIR}/${BIN}" --version
```

- [ ] **Step 2: Lint the script**

Run:

```bash
shellcheck install.sh
```

Expected: no output, exit 0.

- [ ] **Step 3: Run the ATDD tests to verify they pass**

Run:

```bash
cargo test --test cli_install
```

Expected: PASS — `help_flag_prints_usage_and_exits_zero` and
`unsupported_platform_fails_with_cargo_hint` both pass (2 passed).

- [ ] **Step 4: Make the script executable and commit**

```bash
chmod +x install.sh
git add install.sh
git commit -m "feat: add install.sh release-binary installer"
```

---

## Task 3: Wire `shellcheck` into `lint` and CI

**Files:**

- Modify: `justfile` (the `lint` recipe)
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add shellcheck to the `lint` recipe**

In `justfile`, replace this block:

```text
# Lint and format check
lint:
    cargo clippy -- -D warnings
    cargo fmt -- --check
    dprint check
    npx markdownlint-cli '**/*.md' --ignore node_modules
```

with:

```text
# Lint and format check
lint:
    cargo clippy -- -D warnings
    cargo fmt -- --check
    dprint check
    npx markdownlint-cli '**/*.md' --ignore node_modules
    shellcheck install.sh
```

- [ ] **Step 2: Add a `shellcheck` job to CI and add it to the gate**

In `.github/workflows/ci.yml`, add this job immediately after the `convco`
job and before the `ci` job (2-space job-key indentation, matching the
other jobs):

```yaml
shellcheck:
  name: ShellCheck
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v6
    - run: shellcheck install.sh
```

Then change the `ci` job's `needs` line from:

```yaml
needs: [fmt, clippy, test, convco]
```

to:

```yaml
needs: [fmt, clippy, test, convco, shellcheck]
```

- [ ] **Step 3: Verify the lint recipe passes**

Run:

```bash
just lint
```

Expected: exit 0 (clippy, fmt, dprint, markdownlint, and `shellcheck install.sh` all pass).

- [ ] **Step 4: Validate the CI YAML parses**

Run:

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); print('ok')"
```

Expected: prints `ok`.

- [ ] **Step 5: Commit**

```bash
git add justfile .github/workflows/ci.yml
git commit -m "chore: enforce shellcheck on install.sh in lint and CI"
```

---

## Task 4: Release workflow

**Files:**

- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/release.yml` with exactly this content:

```yaml
name: Release

on:
  push:
    tags: ["v*"]
  workflow_dispatch:

permissions:
  contents: write

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            cross: false
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
            cross: true
          - target: x86_64-apple-darwin
            os: macos-latest
            cross: false
          - target: aarch64-apple-darwin
            os: macos-latest
            cross: false
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Install musl tools
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: sudo apt-get update && sudo apt-get install -y musl-tools
      - name: Install cross
        if: matrix.cross
        uses: taiki-e/install-action@v2
        with:
          tool: cross
      - name: Build (cargo)
        if: ${{ !matrix.cross }}
        run: cargo build --release --target ${{ matrix.target }}
      - name: Build (cross)
        if: matrix.cross
        run: cross build --release --target ${{ matrix.target }}
      - name: Package
        shell: bash
        run: |
          set -euo pipefail
          mkdir -p dist
          archive="upskill-${{ matrix.target }}.tar.gz"
          tar czf "dist/${archive}" -C "target/${{ matrix.target }}/release" upskill
          ( cd dist && shasum -a 256 "${archive}" > "${archive}.sha256" )
      - uses: actions/upload-artifact@v4
        with:
          name: upskill-${{ matrix.target }}
          path: dist/*
          if-no-files-found: error

  release:
    name: Publish release
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true
      - uses: softprops/action-gh-release@v2
        with:
          files: dist/*
          fail_on_unmatched_files: true
          draft: false
          prerelease: false
```

- [ ] **Step 2: Validate the YAML parses**

Run:

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('ok')"
```

Expected: prints `ok`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add tag-triggered release workflow (4-target matrix)"
```

---

## Task 5: Documentation

**Files:**

- Modify: `README.md` (the `## Install (consumer)` code block, lines 15-16)
- Modify: `docs/getting-started.md` (the `## Install` section, lines 3-11)

- [ ] **Step 1: Update README install block**

`README.md` line 16 is `cargo install upskill` inside a bash fence that
continues with more commands. This is a pure **insertion** of three lines
directly above line 16 — do not touch any line below it.

Find this exact line (the first line inside the bash fence under
`## Install (consumer)`):

```text
cargo install upskill
```

and replace just that one line with these four lines:

```text
# Linux / macOS (Windows: run inside WSL)
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh

# Or, with a Rust toolchain:
cargo install upskill
```

The blank line and the `# Install everything from a source repo …` content
that already follow `cargo install upskill` stay exactly as they are.

- [ ] **Step 2: Update getting-started install section**

In `docs/getting-started.md`, replace this exact block (it is the entire
current `## Install` section, lines 3-11):

````text
## Install

```bash
cargo install upskill
```

Or download a pre-built binary from the [releases page][releases].

`upskill` is a single static binary with no runtime dependencies.
````

with this block:

````text
## Install

```bash
# Linux / macOS (Windows: run inside WSL)
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh
```

`UPSKILL_VERSION` pins a release tag (default: latest) and
`UPSKILL_INSTALL_DIR` overrides the install location (default
`$HOME/.local/bin`). Windows users run the same command inside WSL.

With a Rust toolchain instead:

```bash
cargo install upskill
```

Or download a pre-built binary directly from the [releases page][releases].

`upskill` is a single static binary with no runtime dependencies.
````

The `[releases]:` link reference at the bottom of the file stays as-is.

- [ ] **Step 3: Format and lint the docs**

Run:

```bash
dprint fmt && npx markdownlint-cli '**/*.md' --ignore node_modules
```

Expected: exit 0, no markdownlint errors.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/getting-started.md
git commit -m "docs: document curl|sh installer and env vars"
```

---

## Task 6: Finalize

**Files:**

- Modify: `docs/superpowers/specs/2026-05-16-install-script-design.md` (line 4)

- [ ] **Step 1: Flip the spec status**

In `docs/superpowers/specs/2026-05-16-install-script-design.md`, change:

```text
Status: Approved (pending spec review)
```

to:

```text
Status: Approved
```

- [ ] **Step 2: Run the full local verification**

Run:

```bash
cargo test && just lint
```

Expected: all tests pass; `just lint` exits 0 (clippy, fmt, dprint,
markdownlint, shellcheck).

- [ ] **Step 3: Commit and push**

```bash
git add docs/superpowers/specs/2026-05-16-install-script-design.md
git commit -m "docs(spec): mark install-script design approved"
git push -u origin claude/add-install-script-eZubT
```

Expected: push succeeds. PR #135 already exists for this branch — do **not**
open a new PR. The active PR-activity subscription will report CI status;
investigate and fix any failure on PR #135.

---

## Notes for the implementer

- The release workflow only runs on `v*` tag pushes / manual dispatch; it
  will **not** run on the PR. Its correctness is verified by the first real
  `just release` (out of scope for this PR) — within this PR, the YAML-parse
  check in Task 4 Step 2 is the gate.
- `install.sh`'s download/verify/install path needs a published release to
  exercise end-to-end; the ATDD tests deliberately cover only the offline
  paths (`--help`, unsupported platform) so they need no network.
- `cross` (Task 4) runs the aarch64-musl build in a Docker container;
  `ubuntu-latest` runners provide Docker. `shasum -a 256` exists on both
  `ubuntu-latest` and `macos-latest`, so the packaging step is uniform.
- Keep commits Conventional (`feat`, `test`, `chore`, `ci`, `docs`) — the
  `convco` CI job checks the PR commit range.
