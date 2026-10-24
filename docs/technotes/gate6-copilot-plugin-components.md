# Gate 6 Copilot plugin components: failed positives

Task [#388](https://github.com/driftsys/upskill/issues/388), Q3 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains failed component controls on Copilot CLI 1.0.103, macOS arm64,
2026-10-24. Native instruction positives pass. The installed plugin does not
produce the required rule responses, and its agent is absent from a calibrated
native catalog. No rule lifecycle or real custom-child route is qualified. The
task remains open; production remains schema 1.

## Fixture and campaign

The pin is Copilot 1.0.103 in the
[2026-10-24 revision](gate6-plugin-profile-inventory.md). Claude 2.1.308, Codex
0.164.0 and OpenCode 2.1.4 do not run controls in this packet. Every one of 14
parent native session-start records identifies 1.0.103 and the requested private
model selection. Automatic client updates are disabled. Initial requests lack a
separate pre-request version receipt; their native session-start records
establish the pin. Later invocations also check the version first.

The existing `scripts/qualify-native-instructions.py` prepares a neutral managed
source with Agent Plugins 1.0 `plugin.json`, unscoped and `applyTo`-scoped files
under `com.github.copilot/rules/`, and a restricted agent under
`com.github.copilot/agents/`. Separate project and user test projects have
distinct empty Git roots, unpredictable native markers and ordinary resources.
Their native `.github/copilot-instructions.md` positive asks for a JSON native
marker and the sorted loaded-rule marker array. No team-guide file or symlink is
created. The later catalog fixture additionally has a separate native agent at
`.github/agents/catalog-positive.agent.md`; it is a different logical agent.

The namespace layout is the documented
[Copilot component layout](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference).
Documentation does not establish support at this pin; this packet retains the
observed failures rather than promoting the route.

The normally authenticated isolated profile and allowlisted child environment
are reused. Existing profile configuration has no instruction/skill glob keys;
the managed source is outside documented discovery directories. The generated
project registration is a new owned entry, not a modification of an existing
user or team discovery setting.

## Installation and scope mechanics

A direct local directory install succeeds. Native inventory reports its bare
plugin identity, and native uninstall succeeds. A separate local marketplace
registration provides a namespaced identity for the lifecycle attempts. Native
installation reports that the package loads live from its managed directory; it
does not report a copied source. This is client-native behavior, distinct from
upskill's copy-only generated managed source. The helper never writes a client
cache directly.

For project activation, the helper disables its new user binding through the
native CLI and writes a new project-owned `enabledPlugins` entry. Disable
toggles only that owned project entry; uninstall removes it and invokes the
native uninstall verb. For user activation, native install/disable/uninstall
manage the new user registration. No pre-existing plugin or discovery entry is
toggled. Project-overlay activation is not established by these failed component
probes.

## Observations

| Control                           | Pin             | Project attempt                                | User attempt                                        |
| --------------------------------- | --------------- | ---------------------------------------------- | --------------------------------------------------- |
| Native instruction positive       | Copilot 1.0.103 | PASS: exact native marker, empty rule array    | PASS: same                                          |
| Local/marketplace install         | Copilot 1.0.103 | Native install succeeds                        | Native install succeeds                             |
| Installed component rule positive | Copilot 1.0.103 | FAIL: refuses the required fixture response    | FAIL: additional prose, no expected rule marker     |
| Matching/nonmatching access       | Copilot 1.0.103 | Expected component markers not established     | Expected component markers not established          |
| Expected custom child             | Copilot 1.0.103 | FAIL: generic fallback, timeout                | NOT RUN: delegation blocked                         |
| Disabled/uninstalled response     | Copilot 1.0.103 | Empty arrays observed; not qualified negatives | Disabled empty array; uninstall request interrupted |
| Reinstalled response              | Copilot 1.0.103 | FAIL: empty rules array                        | NOT RUN: model requests paused                      |
| Fresh calibrated catalog          | Copilot 1.0.103 | Native agent present, plugin candidate absent  | Native agent present, plugin candidate absent       |

The [summary](evidence/2026-10-24-copilot-plugin-components/summary.json)
retains per-scope failure classifications. Empty arrays after disable/uninstall
cannot qualify absence because the installed component positive failed. The
model's statement that rules are absent is not a verified reader result.
Markdown fences or extra prose also fail the fixture's exact JSON response
contract.

The second fresh fixture's two ACP catalog checks make no model requests. Native
initialization and session creation succeed, and the distinct native agent is
present in each catalog. The plugin-contained candidate is absent in each. This
supports the bounded missing-catalog observation; it does not establish actual
custom-agent delegation, all plugin formats or every CLI surface.

## Model-policy failure and ruling

The project agent attempt selects a generic agent instead of the fixture custom
agent, then selects further built-in delegates. Four delegate-start events
select models other than the user's requested parent route. Those starts do not
prove successful inference on each selected model, but they violate the intended
test selection boundary. The attempt reaches its timeout and is terminated. It
is retained as a failure, never as successful custom delegation.

Further delegation is blocked; the later model sequence is paused and its active
isolated-profile process terminated. The user-scope reinstall request is not
run. The empty `--available-tools` value also did not establish a reliable
tool-free boundary: the user installed request inspected files. It must not be
reused as a hard enforcement claim. Recorded shell commands list files and Git
history; no model file-write tool is observed. No claim of full sandbox
containment is made for these controls.

The
[decision record](evidence/2026-10-24-copilot-plugin-components/decisions.json)
requires a calibrated custom-agent identity and a bounded child-model policy
before another delegation attempt. Catalog absence is checked without inference.
No further Copilot model requests are made for this packet. These failures do
not justify switching to an unrequested test model or hiding the first attempts.

## Preservation and provenance

The
[preservation record](evidence/2026-10-24-copilot-plugin-components/preservation.json)
retains configuration hashes and modes. Initial local-install calibration,
lifecycle cleanup and the later catalog control each restore the measured
configuration exactly. Native uninstall leaves an added empty `installedPlugins`
key. The calibration removes that owned empty key only when the resulting bytes
match the pre-control hash. The later controllers retain original configuration
bytes in memory and restore them in place only after checking that unrelated
values are unchanged. The auth-bearing configuration is never copied to the
packet or exported; only hashes, modes and key names are retained.

New project registration files are removed. Native profile session, log and
registry/cache effects remain separate; whole-profile byte restoration is not
qualified. No credentials, team guides or client cache files are directly
copied, rewritten or reclaimed by the harness. The failed delegation did not
create a team-guide file in either fixture.

Raw events, sources and helpers remain outside the repository. The
[source index](evidence/2026-10-24-copilot-plugin-components/private-source-index.json)
and receipt manifests bind them. Retained event selections omit thinking, model
identifiers, auth-bearing values and unrelated profile data. They preserve
parent results, tool identities and the failed generic-agent selection. Four
nonrequested model-selection events remain visible as Boolean audit results.

## Offline checks and consequence

Tests run first: `just lint` fails five missing-helper tests, then passes all 54
offline tests. A further catalog-positive fixture assertion fails before its
implementation and passes afterward. Strict per-control pin checks similarly
fail before implementation; the subsequent suite passes 55 tests. Offline tests
never call a client.

The affected proposal rows are
[Rules, Copilot](../wip/specs/2026-10-23-customization-delivery-proposal.md#rules)
and
[Subagents, Copilot](../wip/specs/2026-10-23-customization-delivery-proposal.md#subagents).
The tested component route remains unsupported pending a successful pinned
control. This is not a universal statement that Copilot cannot read a file or
that another package format cannot work. A bounded delegation guard, successful
component positives and the remaining lifecycle controls are still needed.
Linux, IDE/hosted surfaces, complete preservation and integrated ownership
remain unqualified. Q4 continues independently; durable adoption waits for
step 7.
