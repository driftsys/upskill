# Gate 6 cross-client package isolation: blocked positives

Task [#410](https://github.com/driftsys/upskill/issues/410), Q11 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains two failed native positives on macOS arm64, 2026-10-25. Neither target
reaches its absence baseline or foreign-package candidate. This packet does not
establish isolation or unintended cross-client activation. Production routing
and durable architecture records are unchanged.

## Scope and method

A fresh
[campaign](evidence/2026-10-25-cross-client-package-isolation/campaign-pins.json)
pins Claude Code 2.1.308, Copilot 1.0.103, Codex 0.164.0 and OpenCode 2.1.4
before native controls. Only Claude and Copilot execute model requests. Q5 stays
deferred. No Codex, OpenCode, Linux, IDE or hosted qualification is inferred.

Both installed CLIs document `--plugin-dir`. This attempt selects that explicit,
session-only local-package route for both native positive and foreign candidate.
This is narrower than persisted project/user marketplace installation. The
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

## Native observations

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

Both
[classifications](evidence/2026-10-25-cross-client-package-isolation/summary.json)
are independently replayable from selected native records. Baselines and foreign
candidates are **not run**, rather than isolation passes or failures. In
particular, this packet says nothing about whether Copilot activates a Claude
package through its documented legacy manifest reader. Every unexercised
selector and compatibility reader remains outside this observation.

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
The final focused file contains 20 tests; the complete offline suite contains
217. Tests replay both retained failed positives without any native call
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
hashes were not captured before this native attempt. Its failed positives remain
valid blockers, but its envelopes cannot support later candidate qualification
without a fresh before/after capture.

Q11 stays open. Claude needs normal isolated authentication renewal before a new
pinned positive. Copilot's direct slash request loads skill content but still
fails the exact response contract; a new positive must meet the declared
response policy before a foreign-package candidate can be interpreted. This
packet qualifies neither a complete slice-2 package profile nor Q10 overlap.
Retained results must not be relabeled after a later retry.

The
[private-source index](evidence/2026-10-25-cross-client-package-isolation/private-source-index.json)
separates capture-time and later replay source epochs. Raw streams, helpers and
model selection stay private. Public selected records omit model identifiers,
reasoning, usage and unrelated catalog entries. The
[receipt manifest](evidence/2026-10-25-cross-client-package-isolation/receipt-hashes.json)
hashes retained artifacts; raw-output hashes remain separate from sanitized-file
hashes.
