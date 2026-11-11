# Q11b Copilot calibration by native skill name

Task: [#428](https://github.com/driftsys/upskill/issues/428). The user approved
this small revision in chat on 2026-10-30: use Copilot's native invocation
identity while proving the loaded plugin, path and content; reject collisions
and wrong-source loads. Offline checks and independent review precede one fresh
Haiku 5.5 request. Earlier results keep their original interpretation.

The [retained attempt](../../technotes/gate6-copilot-native-calibration.md)
passed this revised positive in one fresh process. The
[completed follow-up cell](../../technotes/gate6-project-registration-isolation.md)
records automatic Copilot activation after Claude project registration. The
result is an isolation FAIL; no production qualification is granted.

This supplements the [project profile](2026-10-27-q11b-project-profile.md), with
a separately versioned `copilot-native-name-v1` positive. Its preservation
inventory, neutral fixtures, profile locks, tools, package equality, exact
response, fresh-session ordering and failed-positive stop continue to apply.
Q11, production routes and the slice-2 entry gate do not change. This is a
calibration revision, not a client qualification result or proof of an upstream
Copilot bug.

## Accepted positive

The new `inspect_copilot_native_positive` pure replay function takes stdout
records, complete persisted native session history, the captured fixture and
immutable skill-file bytes. It performs no native or credential operations. Only
a row with `calibration_passed: true`, `provenance_verified: true` and
`session_bound: true` may supply the revised observer positive. Preserve the
logical catalog namespace in the row; record the actual tool argument separately
as `invocation_name`.

Require all of the following:

- One enabled catalog entry with the expected bare skill name, logical namespace
  and exact skill-file path. Reject duplicate names, alternate bare command
  aliases and ambiguous entries rather than applying guessed precedence.
- Exactly one native `skill` call with the bare skill name and one successful
  completion. A namespace failure followed by a bare-name fallback does not pass
  this revision. Extra tools or loads block it.
- Exactly one `skill.invoked` record directly linked to that completion,
  matching the skill name, plugin identity/version, plugin source, absolute path
  and exact instruction body. The body excludes the fixture's YAML frontmatter,
  without trimming its remaining bytes. Verify the complete fixture bytes
  against their independently captured SHA-256 first.
- A single native startup and shutdown, unique event identities and an ordered
  parent chain. Bind startup session ID, CLI pin and cwd to the terminal stdout
  receipt. Bind catalog startup parent and every stdout user, assistant,
  tool-call and completion record to the corresponding native records. The
  completion may add native detail, while identity, outcome and call ID agree.
- The final exact, untrimmed private-marker response after the native load.
  Reject marker disclosure in the request or an assistant response before load,
  failed processes, timeout, malformed streams and native session errors.

The positive prompt is:

`Invoke the skill named <skill-name> and follow its instruction. Do not use any
other tools. Return only the required response.`

It differs from the original only by using the bare native name. The private
marker stays exclusively in the fixture content. A new unique fixture and native
session are required. The earlier Haiku namespace-to-bare run remains a failed
positive under its original profile, even though its native load provenance and
exact response are useful diagnostic observations.

## Account and credit boundary

Use the existing isolated Copilot root. While holding its exclusive lock, verify
that the native profile's active GitHub login is `stasson` before making any
model request. Keep auth configuration in memory only; never export or copy
token values. The previous quota failure used a different account and is not
evidence about `stasson` quota. The user separately completed native device
login as `stasson` and explicitly accepted the client's private-file credential
fallback after keychain storage was unavailable. No further login or credential
repair occurs as part of calibration.

Select Haiku 5.5 explicitly and confirm the selected native session model
privately. Run at most one fresh calibration process, without automatic retries
or model fallback. Capture new four-client pins and preservation receipts. Do
not change discovery preferences. Preserve the original source packages and all
prior packets.

## Subsequent isolation controls

A passing positive only admits the remaining controls; it is not isolation
success. Claude's positive, native calibration cleanup, selector-free baseline,
owner registration/candidate checks and post-removal observer checks remain
required under the project profile.

For Copilot absence, the native bare name must produce the exact native
not-found result and exact absence marker, with no activation. Retain catalog
absence and checks for every applicable callable alias. A rejected catalog
namespace alone cannot establish absence. Catalog exposure, ambiguity or an
unresolved alias blocks PASS; a demonstrated unintended activation remains FAIL.
Fresh session and operation-order receipts and separate preservation reporting
remain mandatory. This change provides the positive replay inspector; it does
not invent negative receipts or treat the existing phase classifier as a native
capture producer. Review the concrete negative capture before executing it.

## Offline evidence and implementation

The tests in `scripts/tests/test_qualify_copilot_native_name.py` exercise
synthetic captures. They demonstrate guards and require no model credits.
Neither their success nor replaying diagnostic history qualifies a new native
experiment. The [implementation plan](../plans/2026-10-30-q11b-native-name.md)
records the bounded code and test work. Historical
`inspect_cross_client_positive` and `classify_registration_isolation` behavior
stays unchanged; producers forward a new positive only after this profile's
affirmative provenance checks pass.
