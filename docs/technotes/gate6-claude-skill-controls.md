# Gate 6 Claude skill-route observations

Task [#353](https://github.com/driftsys/upskill/issues/353) retains a bounded
native slice of [#337](https://github.com/driftsys/upskill/issues/337),
following the [shared-root evaluation](shared-content-root-evaluation.md).
Retained on 2026-10-22; original native events were captured on 2026-10-21 UTC.
Production stays schema 1. All 160 campaign cells remain unqualified.

## Context and completion

The user renewed the normally provisioned isolated Claude login after its
previous OAuth token was revoked. A tool-free unpredictable marker request
completed before these controls. No credential files were copied or included in
the packet. These requests use Claude Code 2.1.303 on macOS arm64, fresh project
sessions, and resolved model `claude-sonnet-5-5`. The retained events do not
independently identify the underlying account provider route; this is not a
provider-range claim.

The [receipt summary](evidence/2026-10-22-gate6-claude-skills/summary.json)
contains nine distinct completed sessions and five successful native `Skill`
invocations. Every session exited 0 and emitted a native success result with
`is_error: false` and no permission denials. Positive calls have matching native
tool-use/result IDs, successful results and exact fixture markers. Three
intended negative controls returned the exact unpredictable absent fallback with
no skill call. The direct shared-route candidate instead missed its desired
positive marker; retain that failed candidate positive separately, even though
its native request completed with the absent fallback. Requests exposed only
`Skill`, allowed that tool explicitly, used project settings, disabled session
persistence and supplied an empty strict MCP config. These constraints describe
the experiment, not a general permission adapter.

## Route and lifecycle controls

| Control                                                       | Native observation                           | Interpretation                                                                            |
| ------------------------------------------------------------- | -------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Native `.claude/skills` fixture                               | One successful `Skill` call and exact marker | Native positive calibrates the project context                                            |
| Shared `.agents/skills` fixture in the same project           | Exact absent fallback; no skill call         | Failed desired shared-route positive; bounded absence observation, not a passing negative |
| Self-contained plugin through `--plugin-dir`                  | One successful namespaced skill call         | Development-loading feasibility only                                                      |
| Same namespaced request without `--plugin-dir`                | Exact absent fallback; no skill call         | Paired development-route negative                                                         |
| Development plugin loaded again                               | One successful namespaced skill call         | Restored development positive                                                             |
| Local marketplace added and plugin installed at project scope | One successful namespaced skill call         | Ordinary installed-plugin explicit invocation observed                                    |
| Installed plugin disabled at project scope                    | Exact absent fallback; no skill call         | Persisted disabled negative                                                               |
| Installed plugin re-enabled                                   | One successful namespaced skill call         | Re-enabled positive                                                                       |
| Plugin uninstalled at project scope                           | Exact absent fallback; no skill call         | Removed negative; marketplace subsequently removed                                        |

Development controls used a self-contained copy of the portable skill under the
plugin's `skills/` directory. Installed controls used a separate disposable
local marketplace and project, a new portable shared source and a byte-identical
packaged skill. Native validation, marketplace addition, install, disable,
enable, uninstall and marketplace removal all exited 0. Commands and native
outputs are retained; no marketplace was published. Normal project installation
also used the isolated profile's native cache; this does not qualify a
user-scope installation or cache recovery.

No simultaneous direct-plus-plugin invocation or catalog-count test ran. One
successful call does not establish a unique available entrypoint. The failed
direct-route positive and working plugin namespace support continuing candidate
A's Claude adapter experiments, not adopting that route. Candidate B, selective
exclusions and the baseline of generated native outputs remain alternatives.
These skill results establish no independent rule injection or subagent support.

## Preservation and native residue

The first three sessions have helper-reported team-file equality only. The later
development pair retains before/after fixture contents, modes and hashes. The
installed sequence retains before/after snapshots and each native command's
project settings. The
[preservation record](evidence/2026-10-22-gate6-claude-skills/preservation.json)
shows unchanged manual `CLAUDE.md` and shared skill source bytes/modes, and an
unchanged unrelated manual environment setting and settings-file mode `0640`
through native operations. The package manifest and skill are also captured.

After native uninstall and marketplace removal, project settings retain empty
`enabledPlugins` and `extraKnownMarketplaces` objects. The original manual value
survives, but settings bytes differ. This is native configuration residue, not
exact original-state restoration. No unrelated plugin registration was created
in this fixture, so preservation of other registered plugins remains untested.
No user authentication/configuration store was snapshotted or exported.

Private capture helpers are not shipped or tested product runners. The packet
keeps selected original init, assistant, native skill-result, synthetic loaded
skill and final-result events. Full requests, thinking, usage and unrelated
metadata are omitted. Paths are normalized to `$PROFILE`, `$PROJECT_A`,
`$PROJECT_B` and packet aliases. Complete private stdout/stderr hashes differ
from hashes of sanitized retained files. The
[hash manifest](evidence/2026-10-22-gate6-claude-skills/receipt-hashes.json)
covers 18 receipt/metadata files. Nine distinct native session IDs are retained.

Private source packets:

- `/private/tmp/upskill-gate6-claude-shared-route-lnwzy9bs`
- `/private/tmp/upskill-gate6-claude-route-controls-plw2_tys`
- `/private/tmp/upskill-gate6-claude-installed-skill-6io0vj0e`

## Remaining qualification

These are bounded project observations. Resource access/absence/relocation,
automatic selection, unintended readers, direct/plugin and scope/alias duplicate
controls, parent/ancestor configuration, native rule scoping, actual delegated
restrictions and integrated upskill ownership remain pending. No timeout,
interruption, descendant-cleanup, lock, backup, update or crash-recovery control
ran. Model results do not establish automatic rule delivery.

Retain all four clients, project/user scopes, macOS arm64/Linux x86_64 and
existing IDE obligations. New owned-skill lifecycle remains after Gate 6 and
requires integrated qualification before adopting a new skill layout or
releasing the combined MVP. This packet selects no production adapter, changes
no runtime route and does not unblock schema-2 activation.
