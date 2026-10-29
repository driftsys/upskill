# Q11b: Copilot native-name positive passes

The user-approved
[native-name calibration revision](../wip/specs/2026-10-30-q11b-native-name-profile.md)
passes on macOS arm64, 2026-10-30 Europe/Paris (2026-10-29 23:21 UTC), in one
fresh Copilot process. This is one positive, not a registration-isolation
qualification. Task [#428](https://github.com/driftsys/upskill/issues/428) stays
open. The original Q11 FAIL and slice-2 entry gate remain unchanged.

## Revised invocation and observed result

The original calibration required the model-facing tool argument to equal the
catalog namespace. The revised profile uses the native bare skill name and
requires the native load to prove its plugin identity/version, exact path and
instruction body against immutable fixture bytes and SHA-256. It rejects
colliding catalog names, wrong sources, extra calls, disconnected records,
pre-request loads, marker disclosure and response whitespace changes. The
historical namespace parser is unchanged and still rejects bare-name loads. The
original failed-positive outcomes are not reclassified.

The [fresh campaign](evidence/2026-10-30-q11b-native-name/campaign-pins.json)
retains Claude 2.1.308, Copilot 1.0.103, Codex 0.164.0 and OpenCode 2.1.4. The
isolated Copilot profile's active login was verified as `stasson` under its
exclusive lock. Haiku 5.5 was explicitly selected and the native session's
actual model was checked privately. Only Copilot executed a model request; the
other clients executed version probes only.

The [summary](evidence/2026-10-30-q11b-native-name/summary.json) records one
successful bare-name skill call, one correlated native load from the intended
plugin and file, exact instruction-body equality, the exact untrimmed marker
response, exit 0 and no timeout. The request contains no content marker. No
namespace attempt, automatic retry, model substitution or prompt change occurs
in this attempt. Claude calibration, owner registration, baseline, candidate and
post-removal controls were not run.

The capture runner initially passed in-memory Python path objects to a validator
that requires a captured JSON fixture. That correctly produced a blocked
[initial producer summary](evidence/2026-10-30-q11b-native-name/original-producer-summary.json).
Offline replay of the unchanged saved JSON fixture and raw stream then passed
all checks. Both records remain retained, and no native request was repeated to
correct the producer's input representation.

## Preservation and public replay

All declared protected-path receipts match before and after the model process.
Package envelopes and paired portable trees match their initial fingerprints.
The fixture's sentinel settings retain their content and mode. No owner
registration was created, so no fixture uninstall was required. Native
session/cache/log/usage bookkeeping remains native-managed, outside the
protected-file equality claim. No whole-profile equality is claimed. Before this
attempt, the user separately completed native login as `stasson` and explicitly
accepted the CLI's credential-file fallback; those authorized login changes are
not claimed as calibration preservation.

The public replay uses an explicit
[sanitized fixture projection](evidence/2026-10-30-q11b-native-name/replay-fixture.json),
[stdout selection](evidence/2026-10-30-q11b-native-name/stdout-selection.json),
[native history selection](evidence/2026-10-30-q11b-native-name/native-selection.json)
and [skill bytes](evidence/2026-10-30-q11b-native-name/replay-skill.txt).
Markers and temporary roots are replaced; irrelevant native data is omitted,
while record identities and parent links remain. The projected skill bytes have
their own hash, explicitly distinct from the private original in the
[source projection](evidence/2026-10-30-q11b-native-name/source-projection.json).
The
[replayed observation](evidence/2026-10-30-q11b-native-name/replay-observation.json)
passes using the same pure inspector. The private original fixture, bytes and
raw streams were separately replayed successfully; public projection does not
substitute its hash for the original native source hash.

The
[private-source index](evidence/2026-10-30-q11b-native-name/private-source-index.json)
and
[public receipt hashes](evidence/2026-10-30-q11b-native-name/receipt-hashes.json)
retain source integrity. Raw streams, auth-bearing configuration, model
identifiers and original marker values are not published. Authentication
configuration was inspected in memory and never exported. The independent
pre-native reviewer found request-ancestry and missing-data defects; both have
failing regressions and fixes. The
[offline validation](evidence/2026-10-30-q11b-native-name/offline-validation.json)
is synthetic guard evidence, separate from this native observation.

A passing Copilot calibration admits further work. It does not establish
selector-free isolation. The remaining native controls still need their concrete
capture review and bounded credit budget; a rejected namespace alone cannot
establish absence, and callable aliases and collisions remain required controls.
No further model requests ran in this attempt.
