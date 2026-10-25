# Gate 6 cross-client package isolation: Claude activation observed

Task [#410](https://github.com/driftsys/upskill/issues/410), Q11 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains a bounded isolation FAIL on macOS arm64, 2026-10-25: Claude 2.1.308
activates the Copilot-shaped package through the explicit session selector after
its native positive and clean absence baseline pass. Copilot's opposite
direction remains blocked by its exact-response positive. Earlier failed
attempts remain retained unchanged. Production routing and durable architecture
records are unchanged.

## Scope and method

A fresh
[campaign](evidence/2026-10-25-cross-client-package-isolation/campaign-pins.json)
pins Claude Code 2.1.308, Copilot 1.0.103, Codex 0.164.0 and OpenCode 2.1.4
before native controls. Only Claude and Copilot execute model requests. Q5 stays
deferred. No Codex, OpenCode, Linux, IDE or hosted qualification is inferred.

Both installed CLIs document `--plugin-dir`. This attempt selects that explicit,
session-only local-package route for both native positive and foreign candidate.
This is narrower than persisted project/user marketplace installation. The
initial
[selector inventory](evidence/2026-10-25-cross-client-package-isolation/selectors.json)
records other selectors as unrun. Documentation describes candidate behavior; it
does not replace native evidence
([Claude plugins](https://code.claude.com/docs/en/plugins),
[Copilot plugin reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference),
[Copilot plugin directories](https://docs.github.com/en/copilot/how-tos/copilot-sdk/features/plugin-directories)).

The existing `scripts/qualify-plugin-rules.py` builds a neutral root outside
native discovery roots and the repository. Each target has independent package
and skill identities and two disposable Git projects with distinct root commits.
Within a target, native and foreign envelopes share the plugin/skill names and
identical skill and resource files, including file modes. Claude envelopes use
`.claude-plugin/plugin.json`; Copilot envelopes use Agent Plugins 1.0 root
`plugin.json`. Fingerprints cover every skill-tree file before and after native
execution. No team guidance, symlink or client cache is authored
([fixture](evidence/2026-10-25-cross-client-package-isolation/fixture.json)).

Previously provisioned isolated profiles use their normal authentication. The
capture helper strips inherited Git/client override variables, disables
automatic updates, selects the model privately and restricts model tools to the
native skill tool. It does not disable existing discovery preferences. Existing
Claude synced plugins remain enabled. Copilot's existing disabled Q3
registrations remain untouched. The discovery audits retain source-key names and
hashes rather than auth-bearing configuration values
([Claude audit](evidence/2026-10-25-cross-client-package-isolation/claude-discovery-audit.json),
[Copilot audit](evidence/2026-10-25-cross-client-package-isolation/copilot-discovery-audit.json)).

Each invocation requests the exact namespaced skill; its expected marker occurs
only in fixture content, not the prompt. The intended sequence is native
positive, selector-free catalog/invocation baseline after the positive process
exits, then foreign selector with catalog and invocation. A failed positive
stops the latter phases. Missing catalog alone could never establish isolation.

## Earlier failed-positive observations

| Target  | Pin     | Positive observation                                                                                                                                                                                | Disposition        |
| ------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| Claude  | 2.1.308 | Native init catalogs the exact namespace and inline source path; revoked OAuth prevents a successful turn. Exit 1 and `is_error: true`, despite subtype `success`. No skill call occurs.            | `blocked_positive` |
| Copilot | 1.0.103 | Runtime catalog advertises the exact command namespace. The native skill tool rejects that namespace, then loads the bare skill name. The final response fails the exact untrimmed marker contract. | `blocked_positive` |

Claude's
[selected records](evidence/2026-10-25-cross-client-package-isolation/claude-positive-native-selection.json)
and
[receipt](evidence/2026-10-25-cross-client-package-isolation/claude-positive-receipt.json)
retain completion flags and session identity. Raw provider error text stays
private; selected records label its authentication category. This is consistent
with the failure retained separately by Q10
[PR #414](https://github.com/driftsys/upskill/pull/414). No credential renewal
or alternative profile is attempted.

Copilot's standalone `skill list --json` displays a bare `name`. Its runtime
`session.skills_loaded` separately provides the namespaced `commandName` and the
exact skill path. Those fields are not interchangeable. The
[selected stream](evidence/2026-10-25-cross-client-package-isolation/copilot-positive-native-selection.json)
retains the namespaced failed call, correlated failure, bare-name retry,
correlated success and untrimmed final response. A bare-name fallback is not
promoted to a namespaced positive. The
[receipt](evidence/2026-10-25-cross-client-package-isolation/copilot-positive-receipt.json)
records exit 0; successful process completion alone does not meet the control.
No agent delegation or other model tool occurs.

Both initial
[classifications](evidence/2026-10-25-cross-client-package-isolation/summary.json)
are independently replayable from selected native records. Baselines and foreign
candidates are **not run**, rather than isolation passes or failures. In
particular, those failed-positive attempts say nothing about whether Copilot
activates a Claude package through its documented legacy manifest reader. Every
unexercised selector and compatibility reader remains outside this observation.

## Copilot explicit slash retry

A separate
[fresh four-client capture](evidence/2026-10-25-cross-client-package-isolation/copilot-slash-retry/campaign-pins.json)
precedes a new fixture and a request consisting exactly of `/<plugin>:<skill>`.
This tests the client slash route separately from the initial model-selected
tool argument. The native catalog exposes the command; one bare-name skill load
succeeds without a preceding namespaced-tool failure. The
[untrimmed response](evidence/2026-10-25-cross-client-package-isolation/copilot-slash-retry/native-selection.json)
is two newline characters followed by the exact fixture marker. It fails the
existing exact-response contract. This attempt therefore also stops before its
baseline and foreign candidate
([summary](evidence/2026-10-25-cross-client-package-isolation/copilot-slash-retry/summary.json)).
Configuration hashes and modes remain unchanged, and complete package envelope
fingerprints now match before and after. The initial failed attempt remains
unchanged. This control does not establish that the slash route is unsupported;
it shows content loading with a failed response contract. Any further trial must
state its response policy before execution rather than trimming this result into
a PASS.

## Preservation and parallel work

Native targets run serially. Each target holds a private OS advisory lock for
its isolated profile. Such a lock coordinates participating capture processes;
it cannot exclude an unrelated process that ignores the lock. Q10's retained
cleanup precedes these Q11 calls, and Q11 creates no persistent registrations.
Q10 and Q11 retain separate results, fixtures and campaign revisions.

The originally measured profile `settings.json`, `config.json`,
`plugins/known_marketplaces.json` and `plugins/installed_plugins.json` retain
matching presence, hashes and modes across each target's calls. Copilot's
`config.json` is auth-bearing. Claude does not use that path as its global
configuration: neither possible `.claude.json` location nor managed settings was
measured before this attempt. Their equality and restoration are unqualified; an
absent `config.json` does not establish preservation of Claude's global
configuration. No restoration write is made to the originally measured files.
The classifier computes that limited preservation contract separately from
isolation
([Claude](evidence/2026-10-25-cross-client-package-isolation/claude-preservation.json),
[Copilot](evidence/2026-10-25-cross-client-package-isolation/copilot-preservation.json)).

No native registration cleanup is required for `--plugin-dir`; its effect ends
with the process. After-only inventories retain the observed Claude global
configuration locations and Copilot session files, without their contents
([Claude residue](evidence/2026-10-25-cross-client-package-isolation/claude-residue-after.json),
[Copilot residue](evidence/2026-10-25-cross-client-package-isolation/copilot-residue-after.json)).
There is no before inventory for those paths, so creation or change attribution
is unavailable. Neutral fixtures remain private for replay. Native
authentication, session/log files, built-in caches and synced plugin effects are
outside measured equality. Whole-profile restoration is not claimed. No direct
cache cleanup or writes occur.

The Copilot audit hashes individual discovery-setting values. Its empty
`extraKnownMarketplaces` value and Claude's empty marketplace registry have the
same hash because both hash the constant `{}`; this does not establish a copied
file or shared profile
([verification](evidence/2026-10-25-cross-client-package-isolation/audit-hash-explanation.json)).
The Claude runtime catalogs other enabled synced plugins, but this attempt does
not identify their enablement configuration source. That reader inventory
remains necessary before a foreign-package control.

## Offline validation and continuation

Fixture/classifier tests first fail in `just lint` on missing Q11 interfaces,
then all 208 offline tests pass. Four native-inspector tests fail before the
inspector exists. Further tests reject missing tool IDs, stream completion
without invocation, content leakage, selector conversion and manifest mutation.
At the initial implementation review, the focused file contained 20 tests and
the complete offline suite contained 217. Quick-review corrections extend these
to 27 focused and 224 total tests. Tests replay the retained failed positives
and the explicit slash retry without any native call
([validation](evidence/2026-10-25-cross-client-package-isolation/validation.json)).

Classifier tests require metadata and tree integrity, calibrated positive and
clean baseline, then check activation before catalog completeness. Catalog
exposure is a separate isolation failure, not proof of executed content.
Unavailable selectors and documented selector rejection have different results.
A contaminated baseline and an incomplete baseline have their own blocked
results. A negative requires an exact namespaced attempt with a correlated
native not-found resolution, a successful catalog request, the exact absence
response, and no content activation. A response containing the private fixture
marker, a successful bare-name load or a selector install/conversion is recorded
as activation even when the response is not an exact marker match.

The reviewed builder fingerprints package envelopes as well as portable skill
trees and records each package's intended client and manifest path. Envelope
hashes were not captured before the initial failed-positive attempt. Its failed
positives remain retained blockers for that attempt; the authenticated retry
below uses a fresh before/after envelope capture.

Q11 stays open. Claude's authenticated direction now has a retained activation
FAIL, rather than an isolation PASS. Copilot's direct slash request loads skill
content but still fails the exact response contract; a new positive must meet
the declared response policy before its foreign-package candidate can be
interpreted. This packet qualifies neither a complete slice-2 package profile
nor Q10 overlap. Earlier results are not relabeled by the authenticated retry.

The
[private-source index](evidence/2026-10-25-cross-client-package-isolation/private-source-index.json)
separates capture-time and later replay source epochs. Raw streams, helpers and
model selection stay private. Public selected records omit model identifiers,
reasoning, usage and unrelated catalog entries. The
[receipt manifest](evidence/2026-10-25-cross-client-package-isolation/receipt-hashes.json)
hashes retained artifacts; raw-output hashes remain separate from sanitized-file
hashes.

## Quick-review coverage corrections

A subsequent quick review found independent guards that the original tests did
not pin. Six source mutations survived all 20 original focused tests;
strengthened assertions reject all six while the unchanged implementation passes
([before/after evidence](evidence/2026-10-25-cross-client-package-isolation/quick-review-regressions.json)).
The new coverage isolates Claude's error flag at process exit zero, checks every
phase's metadata, rejects initially unequal content pairs, validates equal
present-file receipts, requires independent project roots, and correlates Claude
successful tool results. Other tests pin invalid campaign inputs, package-entry
guards, the untrimmed slash-retry response and complete public receipt hashes.
The plan's result enumeration now includes both baseline-blocking outcomes.
These changes do not rerun a native control or alter an earlier native verdict.

## Authenticated Claude retry

The native positive below confirms working authentication after the user's
normal isolated login. The harness preserves OS `USER` while stripping token,
API-key, Git and client-root overrides. Two offline regressions pin those
allowlist properties without contacting an authentication service.

A separately indexed
[later diagnostic repeat](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/auth-diagnostic-repeat.json)
runs `claude auth status` with `USER`, without it, then with it again. The CLI
reports logged in, not logged in, then logged in, with identical profile roots
and all other environment values held constant. The diagnostic has its own
post-control timestamps and
[source index](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/auth-diagnostic-source-index.json);
it is not a reconstruction of the original unretained preflight. Only status
fields and raw-output digests are retained, excluding account metadata. No model
request, credential copy or configuration-root fallback is used by this
diagnostic.

A fresh
[four-client campaign](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/campaign-pins.json)
precedes the new Claude fixture. The pins remain Claude 2.1.308, Copilot
1.0.103, Codex 0.164.0 and OpenCode 2.1.4. Only Claude runs model controls in
this retry. The
[fixture](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/fixture.json)
records Claude as the native intended reader and Copilot as the foreign
package's intended reader, both manifest paths, identical skill/resource bytes
and modes, and full package-envelope fingerprints.

| Phase             | Selector and native evidence                                                                                                                                          | Result         |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| Native positive   | Claude-shaped package through `--plugin-dir`; catalog, one exact namespaced `Skill` call, correlated success, exact content marker                                    | PASS           |
| Absence baseline  | No selector in the candidate project; absent catalog entry, explicit exact namespaced call returns native not-found, exact absence marker, zero activations           | PASS           |
| Foreign candidate | Copilot-shaped root `plugin.json` package through `--plugin-dir`; catalog, one exact namespaced activation, exact content marker, runtime path in the foreign package | Isolation FAIL |

The three fresh processes have distinct native session IDs. The content marker
is absent from every prompt. The baseline runs after the positive process exits;
its successful absence calibration precedes the candidate. The selected records
and command/version receipts retain those separate boundaries
([positive](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/positive-native-selection.json),
[baseline](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/baseline-native-selection.json),
[candidate](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/candidate-native-selection.json)).
Phase replay binds the existing positive inspector to each phase's declared cwd
and source package, checks the actual receipt cwd, and then classifies against
the original paired fixture. Envelopes, skills and modes remain unchanged; no
on-disk conversion or persistent installation is observed.

This establishes an activation counterexample for one pinned client, package
shape and session selector. It does not establish all compatibility readers,
marketplace scopes, duplicate lifetimes, provider-effective input uniqueness,
Q10 overlap or other clients. Existing synced plugins and discovery preferences
remain enabled. Copilot's own failed-positive direction is not rerun or
inferred.

### Measured preservation

The retry measures both possible `.claude.json` locations, profile settings,
local settings, installation/marketplace registries and the system managed
settings file. The live capture retains original configuration bytes only in
memory. Native inline calls change the numeric feature-cache timestamp and only
the fixture's `pluginUsage` and `skillUsage` entries. They remain present
through the baseline and candidate, so the absence control is not manufactured
by clearing usage state.

A tested pure guard permits only those observed numeric bookkeeping shapes and
exact owned keys. Unrelated account, discovery, usage or permission changes stop
restoration. After all native processes exit, the helper records residue,
rechecks the observed bytes, restores only the measured bookkeeping file, and
verifies every measured file's original presence, hash and mode. The profile
lock is released after that check
([preservation](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/preservation.json),
[residue](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/native-bookkeeping-residue.json)).

A
[filtered witness](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/bookkeeping-witness.json)
retains the fixture's usage values and hashes of all unrelated semantic fields
and entries. Its replay verifies the selected bookkeeping shape; full original
auth-bearing configuration is not published. Native authentication, logs, caches
and synced effects remain outside whole-profile equality. Neutral fixtures stay
private for replay. No direct native cache writes or cleanup are performed.

The environment regression fails before the allowlist fix. Three bookkeeping
regressions fail before that guard exists; a mode-type/range regression also
fails before stricter validation. All 33 focused Q11 tests then pass, including
replay of these three native phases and the filtered restoration witness. Full
verification and source epochs are retained with the packet
([source index](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/private-source-index.json),
[summary](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/summary.json)).

### Follow-up coverage

Follow-up assertions reject zero usage counts, every negative permitted usage
value, noninteger modes in either operand, inconsistent fixture names, malformed
configurations and nonmapping usage data. Positive cases separately exercise
cache-only changes, independent usage additions and initially absent containers.
Receipt replay now requires a positive duration for each invocation and strict
positive-finish → baseline-start → baseline-finish → candidate-start ordering,
with all four campaign probes preceding the positive. All 40 focused Q11 tests
pass and reject the six previously surviving guard and timing mutations
([before/after evidence](evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/coverage-followup-mutations.json)).
The original native records and isolation verdict are unchanged.

The private restoration helper's write/recheck sequence is documented by its
source and retained receipts; these offline tests do not constitute an automated
native-runner test suite.
