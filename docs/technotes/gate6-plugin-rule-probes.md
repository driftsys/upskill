# Gate 6 independent plugin fixture preparation

Task [#339](https://github.com/driftsys/upskill/issues/339) delivers the offline
fixture prerequisite for native qualification
[#337](https://github.com/driftsys/upskill/issues/337). It completes neither
Task 2 of the
[corrected plan](../wip/plans/2026-10-19-customization-delivery-reconciliation.md)
nor a production activation gate. The
[frozen inventory](gate6-plugin-profile-inventory.md) still requires all four
clients, both platforms and project/user scopes.

## Implemented mechanics

`scripts/qualify-plugin-rules.py --prepare --client codex --root /tmp/new-probe`
creates a new disposable project, a separate home, two plugin packages and a
fixture descriptor. Substitute `opencode` for its v2 candidate. The directory
must not already exist and must be outside the repository. Nothing is installed
by this command; no model is called. It neither reads nor copies credentials.

Each rule has its own payload and unpredictable marker. Team `AGENTS.md` has a
separate marker and mode `0640`. No managed contributions are inserted into that
file. `fixture_environment` constructs disposable native/config/cache roots
without inherited credentials, Git selectors or client overrides. Authentication
must be provisioned normally in those roots; environment isolation alone does
not prove operating-system keychain isolation or native-root authority.

Offline examples exercise actual fixture creation and the Codex hook, plus
paired-observation classification. Failed/incomplete/tool-using turns, wrong
versions, changed team bytes/modes, missing observations and wrong activation
counts cannot produce `observed`. Duplicate checks require independent native
event counts; equal model responses cannot prove one activation. `observed`
describes one pair only and never means campaign qualification. These tests use
synthetic observations and do not establish native discovery.

The Codex hook filters its declared fixture directory and SessionStart event,
records its rule/session/source and emits only its own independent context. The
OpenCode package is an unqualified v2 SDK candidate using a session-directory
and agent filter, a context hook and disposable registration. Native
compilation, hook execution, provider delivery, compaction and reload/disposal
remain unproven. No Node/npm dependency is added to the Rust project; the SDK
declaration belongs only to the disposable native package.

## Native observations and blockers

Sanitized
[Codex installation receipts](evidence/2026-10-19-gate6-plugin-fixtures/codex-installation.json)
record macOS arm64 project observations at 0.163.0. With only the disposable
environment active, `plugin marketplace add`, both `plugin add` commands and
`plugin list` succeeded. The installed copies were reported beneath
`$FIXTURE/home/.codex/plugins/cache/upskill-qualification/<plugin>/0.0.0`. Team
bytes and mode matched the preparation snapshot after installation. This
establishes package installation, not hook trust or rule delivery.

`codex login status` in that root reported an authentication failure. Ordinary
hook trust has not been reviewed/provisioned for these definitions. No Codex
model pair was run. The trust-bypass flag is not used to manufacture a pass.

The pinned Claude 2.1.303 and OpenCode 2.0.32 binaries were fetched into
temporary storage and verified against the campaign's digest/integrity values.
All four CLI pins reported their expected versions in separate credential-free
temporary homes. This is executable availability only. Existing personal
binaries and configuration were not replaced.

[OpenCode prerequisite receipts](evidence/2026-10-19-gate6-plugin-fixtures/opencode-prerequisites.json)
record two unsuccessful 2.0.32 observations. Managed `plugin list` could not
start because its default service port was occupied. The existing listener was
left untouched. A private `run --standalone --format json` with deny-all fixture
permissions emitted an error with status 500, with no successful completed
positive turn. Its cause remains unresolved; it does not establish an
authentication diagnosis or plugin injection. Neither failure qualifies a
negative. Native probes stopped at these blockers.

The external receipt roots are
`/private/tmp/upskill-gate6-codex-installed-20261019`,
`/private/tmp/upskill-gate6-opencode-prepared-20261019` and
`/private/tmp/upskill-gate6-interface-20261019`. The retained JSON includes
exact sanitized commands, outcomes and hashes. Retained stdout receipts match
their recorded hashes; the OpenCode managed-port diagnostic is a separately
labeled rerun. Raw arbitrary client output is not treated as model evidence.
Temporary invocations used the existing process-group cleanup helper; the new
preparation command starts no subprocess. A future automated model runner still
needs its own timeout/interruption acceptance cases.

## Remaining activation gates

| Gate  | Current evidence                                              | Remaining requirement                                                                  |
| ----- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| G6-01 | Existing selection/library tests                              | Qualified production command dispatch and unchanged defaults                           |
| G6-02 | Two Codex packages install without changing team instructions | Native update/remove and other-rule preservation                                       |
| G6-03 | Merged complete-preflight foundation                          | Reader-intent and qualified package/native-effect integration                          |
| G6-04 | Existing qualified writer, backup and recovery foundation     | Production dispatch and explicit native-effect recovery                                |
| G6-05 | Schema-1 production remains in use                            | Lossless mixed-kind migration and every installed-state command                        |
| G6-06 | Version/interface and installation observations               | Successful fresh/resume/compaction plugin controls; authentication/trust prerequisites |
| G6-07 | Explicit reader inventory and offline classifiers             | Native unintended-reader, matching and duplicate controls                              |
| G6-08 | Disposable project/home roots prepared                        | Authenticated user-root and external-home authority evidence                           |
| G6-09 | All-kind/four-client inventory                                | Applicable mixed-kind native compatibility and IDE regression evidence                 |
| G6-10 | Corrected architecture/contracts merged                       | Qualified support/migration guidance and exact-head activation review/CI               |

None of these rows is an activation pass. A network-disabled Debian 12 x86_64
container on the Linux arm64 Docker host ran all four pinned binaries with
`--version`;
[receipts](evidence/2026-10-19-gate6-plugin-fixtures/linux-pinned-interfaces.json)
record the exact image digest, emulation, artifact hashes, output and container
removal. These are executable checks under emulation only. The authenticated
Linux campaign and isolated authenticated roots remain prerequisites, with no
credentials copied from real homes. The preparation CLI has no model runner or
native receipt parser yet. After the roots, provider and ordinary trust are
available, finish the native runner, paired lifecycle controls and remaining
reader cases in #337 before selecting adapters. Production stays schema 1; no
plugin is published.
