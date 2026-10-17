# Shared updater design

Date: **2026-10-14**. Status: **draft for review**. Scope:
[issue #284](https://github.com/driftsys/upskill/issues/284), which follows the
client startup probes in
[issue #285](https://github.com/driftsys/upskill/issues/285).

## Problem

Upskill already has a manual `upskill update` command for refreshing installed
content. It does not update the upskill executable or coordinate automatic
maintenance across client plugins. Calling `update` from each client startup
would fetch sources on every session, repeat work when several clients start,
and conflate content reconciliation with binary ownership and replacement.

## Goals

- Give thin client plugins one Rust entrypoint for scheduled maintenance.
- Refresh the upskill binary only when its installation is explicitly managed by
  the official installer.
- Refresh content only in the scope selected for that plugin invocation.
- Keep manual `upskill update` behavior content-only.
- Avoid repeated online checks, coordinate concurrent invocations, and preserve
  the last usable binary and installed content when updates fail or are
  interrupted.
- Return status that distinguishes updated, not due, disabled, unmanaged,
  offline/error, and another process currently maintaining the same scope.

## Non-goals

- Updating binaries owned by Cargo or another package manager.
- Native Windows installation or replacement. The existing release matrix is
  Linux and macOS; Windows users use WSL.
- Managing client plugins or marketplaces. Their native update path remains
  separate from upskill binary and content updates.
- Running maintenance on model turns or adding a background daemon.

## Proposed decisions

### Command boundary

Add a shared `upskill maintain` command for client plugins to invoke at a
qualified startup/lifecycle event. Keep `upskill update` as the explicit,
content-only command. `maintain` coordinates due checks, managed binary
replacement, and the ownership-aware content lifecycle; it does not implement a
second content installer or call the schema-1 update path for legacy content.

The command requires an explicit `--scope project|user`. A user-scoped plugin
updates the global lockfile and may update an explicitly managed user binary. A
project-scoped plugin updates only that project's lockfile and never changes the
user's global content or binary. It does not infer project scope from the
current working directory. The recommended default is to bind plugin
installation scope to this flag, so opening a project cannot silently expand a
user-scoped update. If the selected scope has no lockfile, report
`no content
installed` and skip content fetches.

Content maintenance requires ownership evidence sufficient to preserve local
edits. Schema-1 lockfiles contain source digests, not generated-output
baselines, so `maintain` skips schema-1 content without fetching, applying, or
migrating it and reports `legacy-unverified`. Binary maintenance remains
independent and may proceed for a managed executable. New content installations
become eligible only through the complete ownership-aware lifecycle and state
format; `maintain` must not add a parallel ownership sidecar or activate a
partial schema-2 writer. For mixed state, the ownership-aware lifecycle must
preserve legacy records and skip unresolved legacy content while applying only
explicitly owned, unchanged artifacts.

### Cadence and release selection

- Run no network work when the selected scope's last successful maintenance is
  less than 24 hours old.
- Use the latest non-prerelease GitHub release as the default binary channel;
  the current release workflow publishes stable releases only.
- Preserve an explicit `UPSKILL_VERSION` tag as a binary pin in installer
  metadata. A pinned binary stays on that tag until the user runs the installer
  again with the default `latest` selection.
- After a failed check, retain the installed state and retry after one hour.
  Report the failure and the last successful maintenance time.
- Honor `UPSKILL_AUTO_UPDATE=0` as a global opt-out. Return `disabled` without
  network or filesystem writes.
- Store maintenance timestamps in user state, outside the project checkout. Key
  project-scope state by a digest of its canonical root so paths are not written
  into shared state in plain text.

The 24-hour interval and one-hour retry delay are proposed defaults. An explicit
manual `upskill update` continues to run immediately and does not change the
automatic cadence.

### Binary ownership and replacement

Extend `install.sh` to write an adjacent managed-install record containing the
installed path, version, selected release tag/channel, and installation method.
`maintain` replaces the binary only when this record exists, points to the exact
executable being run, and the target is writable. A missing or invalid record
means the binary is unmanaged; report an actionable message and leave it
unchanged. In particular, Cargo and package-manager installations remain owned
by their installer. The documented route is to use that manager, or reinstall
with `install.sh` to opt into upskill-managed updates.

Keep the compressed archive and its SHA-256 sidecar for `install.sh`, and add
the raw target executable with its own SHA-256 sidecar to each official release.
The updater downloads the raw executable, avoiding archive extraction and a new
runtime dependency. Verify its checksum before replacement. Stage the verified
executable on the same filesystem and replace atomically, retaining a
recoverable previous binary until the new executable is in place. Never edit or
replace files in a package-manager prefix.

### Coordination and failure behavior

Use a process lock for each maintenance state key. Acquire it before the due
check and retain it through the content refresh and binary transition, so two
client startups do not apply the same scope concurrently. If another process
holds the lock, exit successfully with a `busy` status; the next client startup
can retry.

Run content reconciliation through the ownership-aware lifecycle under the
selected scope's qualified writer lock. Preserve source labels and pinned refs.
Complete acceptance requires interrupted-update recovery to use
ownership/checkpoint evidence, never overwrite a user's changed files, and
retain prior working outputs if fetching or application fails. Until the
ownership-aware lifecycle is available, legacy content is reported as
`legacy-unverified` and left unchanged; maintenance must not call the current
schema-1 update path for it.

Binary and content results are reported independently. A successful binary check
does not hide a content update failure, and a content success does not claim
that an unmanaged binary was updated. Disabled automatic maintenance performs no
network or write operation and reports `disabled`.

### Client integration boundary

Each client plugin supplies only its native manifest and startup entrypoint; it
invokes the same `upskill maintain --scope ...` command. Client-specific trust,
working directory, reload, and background-execution behavior remains owned by
the corresponding #285 adapter qualification. No adapter may invoke maintenance
on every model turn. Codex CLI remains unqualified by the current probe; Codex
desktop support must be separately established before the matrix claims it.

## Verification

- CLI acceptance tests use fake homes, a local HTTP release fixture, and local
  git sources. They cover due/not-due/disabled state, stable and pinned
  selection, package-manager/unmanaged binaries, checksum failure, offline
  behavior, concurrent processes, and retry after interruption.
- Observe a real binary transition by replacing a test-installed executable from
  a controlled release fixture. Verify the old executable remains usable after
  download, checksum, or replacement failure.
- Observe a real installed-skill transition from a local git source. Verify
  project and user scopes remain separate, pinned refs remain pinned, local
  modifications are preserved, and interrupted application recovers without
  losing the prior installation. Verify schema-1 content reports
  `legacy-unverified`, causes no fetch or write, and remains byte-identical.
- Qualify release replacement on Linux and macOS; verify package-manager-owned
  paths are reported and untouched.
- Verify release publishing includes exactly one raw executable and checksum for
  each existing Linux/macOS target, while `install.sh` continues to use the
  compressed archives.
- Client plugins invoke the same command and preserve native trust and startup
  behavior as qualified by #285. Measure startup overhead against the agreed
  baseline and budget before support is claimed.

## Open acceptance gates

- Treat completion of the M1 ownership-aware lifecycle and its activation gates
  as a prerequisite to starting #284 implementation. Do not deliver a
  binary-only slice of #284 while its automatic content-refresh contract is
  blocked on M1.
- Confirm the recommended project/user scope policy and the 24-hour/one-hour
  cadence.
- Complete Codex lifecycle qualification, including the applicable desktop
  surface and hook trust behavior.
- Agree on and measure the startup baseline and overhead budget.
- Confirm the updater can extend the current install script's ownership record
  without treating existing unmarked binaries as managed.

No production implementation should begin until this design is reviewed and
approved. The implementation plan follows that review.
