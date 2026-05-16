# Design: `install.sh` + release workflow

Date: 2026-05-16
Status: Approved (pending spec review)

## Problem

`upskill` ships as a single static binary, but the only documented install
paths are `cargo install upskill` (requires a Rust toolchain) and an
unpopulated GitHub Releases page. There is no `curl | sh` one-liner, no
GitHub Releases, and no release CI. Users on a fresh machine cannot get the
binary without Rust.

Goal: a best-practice `curl | sh` install experience backed by real release
artifacts, documented in the README and getting-started guide.

## Scope

Native Windows is intentionally **out of scope**: Windows users run
`upskill` under WSL, which is Linux and is fully covered by `install.sh` +
the Linux musl binary.

In scope:

- `install.sh` (POSIX `sh`) at repo root — Linux (incl. WSL) + macOS.
- `.github/workflows/release.yml` building a 4-target matrix and publishing
  GitHub Releases.
- README + `docs/getting-started.md` updated with the curl one-liner.
- Lint + ATDD coverage for the script.

Out of scope:

- Native Windows installer / `*-pc-windows-msvc` targets — WSL covers
  Windows via the Linux path. Revisit only if a non-WSL need appears.
- Bundling man pages in release archives (`just man` already documents the
  generator; revisit later).
- Homebrew tap / scoop manifest / distro packaging (future work).
- Signing / notarization (future work).

## Approaches considered

**Release discovery in `install.sh`:**

- **A (chosen):** GitHub `releases/latest/download/<asset>` redirect — zero
  API calls, no rate limit, no JSON parsing. Pinning via
  `UPSKILL_VERSION` switches to `releases/download/<tag>/<asset>`.
- B: `api.github.com/.../releases/latest` + JSON parse — needs `jq` or
  fragile `grep`, 60 req/hr unauthenticated rate limit. Rejected.
- C: build from source via `cargo` — slow, needs a toolchain, defeats the
  static-binary value proposition. Rejected.

**Release workflow shape:** a build matrix across runners (macOS needs its
own runner). `cross` is used only where native compilation is unavailable
(Linux aarch64).

## Release target matrix — 4 artifacts

| OS    | Arch   | Rust target                  | Archive   | Binary    |
| ----- | ------ | ---------------------------- | --------- | --------- |
| Linux | x86_64 | `x86_64-unknown-linux-musl`  | `.tar.gz` | `upskill` |
| Linux | arm64  | `aarch64-unknown-linux-musl` | `.tar.gz` | `upskill` |
| macOS | x86_64 | `x86_64-apple-darwin`        | `.tar.gz` | `upskill` |
| macOS | arm64  | `aarch64-apple-darwin`       | `.tar.gz` | `upskill` |

Linux uses **musl** for a fully static binary that runs on any distro
(Alpine, old glibc) and under WSL. Asset naming is fixed and shared by the
installer and the workflow:

- Archive: `upskill-<target>.tar.gz`
- Checksum sidecar: `<archive-name>.sha256` (one line: `<sha256>  <archive>`)

## `install.sh` (POSIX `sh`, repo root)

Constraints: no bashisms (no arrays, `[[ ]]`, `(( ))`), `set -eu`, runs
under `dash`/`sh`/`bash`. Passes `shellcheck`.

Behavior:

1. **Config:** env `UPSKILL_VERSION` (default: latest),
   `UPSKILL_INSTALL_DIR` (default `$HOME/.local/bin`); `--help` prints usage
   and exits 0.
2. **Platform detection:** `uname -s` + `uname -m` →
   - `Linux`/`x86_64` → `x86_64-unknown-linux-musl`
   - `Linux`/`aarch64`|`arm64` → `aarch64-unknown-linux-musl`
   - `Darwin`/`x86_64` → `x86_64-apple-darwin`
   - `Darwin`/`arm64` → `aarch64-apple-darwin`
   - anything else → error to stderr, exit 1, message:
     `no prebuilt binary for <os>/<arch>; install with: cargo install upskill`
3. **Downloader:** prefer `curl -fsSL`, else `wget -qO-`; neither → error.
4. **URL:** latest →
   `https://github.com/driftsys/upskill/releases/latest/download/<asset>`;
   pinned → `.../releases/download/$UPSKILL_VERSION/<asset>`.
5. **Fetch:** download archive + `.sha256` into `mktemp -d`; `trap`
   removes the temp dir on EXIT/INT/TERM.
6. **Verify:** `sha256sum -c` or `shasum -a 256 -c`; mismatch → error,
   exit 1.
7. **Install:** extract, `mkdir -p "$UPSKILL_INSTALL_DIR"`, move `upskill`
   into place (overwrite ok), `chmod +x`.
8. **PATH check:** if install dir not found in `:$PATH:`, print a
   shell-agnostic hint (`export PATH="<dir>:$PATH"`).
9. **Finish:** run `"$UPSKILL_INSTALL_DIR/upskill" --version` and print the
   resolved install path.

Failure modes are explicit: unsupported platform, no downloader, download
404 (release/asset missing), checksum mismatch — each prints a distinct
stderr message and exits non-zero.

Windows users run this under WSL; no PowerShell installer is provided.

## Release workflow (`.github/workflows/release.yml`)

- **Trigger:** push of tags matching `v*` (matches `just release` →
  `git std bump`), plus `workflow_dispatch`.
- **Permissions:** `contents: write` (create release / upload assets).
- **Build matrix:**
  - `ubuntu-latest`: `x86_64-unknown-linux-musl` — `rustup target add` +
    `sudo apt-get install -y musl-tools`, native `cargo build --release
    --target`.
  - `ubuntu-latest`: `aarch64-unknown-linux-musl` — via `cross`
    (`cargo install cross` or the prebuilt action) for cross-compilation.
  - `macos-latest` (arm64 runner): `aarch64-apple-darwin` native +
    `x86_64-apple-darwin` via `rustup target add` (Apple toolchain
    cross-compiles both).
- **Per leg:** build with the existing release profile (`opt-level=z`,
  `lto`, `strip`, `panic=abort`) → package `.tar.gz` containing just the
  binary → write `<archive>.sha256` → upload as a workflow artifact.
- **Publish job:** downloads all artifacts, runs
  `softprops/action-gh-release` on the tag with all 8 files (4 archives +
  4 checksums) attached. Body autogenerated from the tag; release marked
  non-draft, non-prerelease.

## Documentation

`README.md` "Install (consumer)" and `docs/getting-started.md` "Install"
gain one primary one-liner, placed **above** `cargo install upskill`:

```bash
# Linux / macOS (Windows: run inside WSL)
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh
```

Followed by a short note: Windows users run the same command under WSL;
`UPSKILL_VERSION` pins a release, `UPSKILL_INSTALL_DIR` overrides the
location, and `cargo install upskill` remains the fallback for unsupported
platforms. The existing `docs/getting-started.md` releases-page link is
kept.

## Testing

- **Lint:** add `shellcheck install.sh` to the `lint` recipe in `justfile`
  and a `shellcheck` step in CI (zero-warnings policy).
- **ATDD (`tests/cli_install.rs`):**
  - `sh install.sh --help` → exit 0, usage text on stdout.
  - `install.sh` with a forced unsupported `uname` (via a stub on `PATH`)
    → non-zero exit, stderr contains `cargo install upskill`.
- **Docs:** existing `dprint` + `markdownlint` cover the README and
  getting-started edits.

## Acceptance criteria

1. `install.sh` exists at repo root and passes `shellcheck`.
2. On a supported platform with a published release, the one-liner
   downloads, checksum-verifies, and installs a runnable `upskill`.
3. Unsupported platform/arch fails fast with the cargo hint.
4. Pushing a `v*` tag produces a GitHub Release with all 4 archives + 4
   checksums.
5. README and getting-started document the one-liner (incl. the WSL note
   for Windows) and the env knobs.
6. `just verify` passes (tests, clippy, fmt, dprint, markdownlint,
   shellcheck).
