# Gate 6 OpenCode explicit-path retry: failed native positive

Task [#391](https://github.com/driftsys/upskill/issues/391) follows the
[adopted native-first direction](../wip/specs/2026-10-23-customization-delivery-proposal.md#rules)
and the
[slice-1 order](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md).
This 2026-10-25 campaign on macOS arm64 retains one failed OpenCode 2.1.4 native
positive. The provider returned HTTP 400 `model_not_supported`; the exported
session selected the requested private model. No downstream model control ran
([summary](evidence/2026-10-25-opencode-explicit-paths/summary.json),
[model audit](evidence/2026-10-25-opencode-explicit-paths/positive-model-audit.json)).

## Method and pins

All four installed releases were captured before the model request: Claude
2.1.308, Codex 0.164.0, Copilot 1.0.103 and OpenCode 2.1.4. The campaign date is
Europe/Paris; receipts retain UTC timestamps. Only OpenCode makes a model
request in this packet
([campaign pins](evidence/2026-10-25-opencode-explicit-paths/campaign-pins.json)).

The existing instruction harness prepares three distinct empty Git projects: a
single-file positive, a lifecycle candidate and an unrelated-project control.
Two independent rule files sit in a neutral managed directory. Their initial and
updated markers, the positive marker and fallback are independent. The positive
registers one explicit instruction-file path and denies all tools. The candidate
starts without instruction registration; no glob, guide file, symlink or direct
client-cache write is created
([fixture](evidence/2026-10-25-opencode-explicit-paths/fixture.json),
[validation](evidence/2026-10-25-opencode-explicit-paths/validation.json)).

The isolated, normally authenticated OpenCode profile is reused without copying
credentials. Project configuration declares the requested private model through
the native custom-model mapping; no provider endpoint or existing discovery
setting is changed. The native catalog contains that entry. Catalog presence is
not backend execution availability. A second version receipt precedes the
positive, and the exported native session confirms its model and session
identity. Raw configuration, transcripts and capture sources remain private
([preflight](evidence/2026-10-25-opencode-explicit-paths/model-preflight.json),
[receipt](evidence/2026-10-25-opencode-explicit-paths/positive-receipt.json),
[source index](evidence/2026-10-25-opencode-explicit-paths/private-source-index.json)).

## Results and limits

| Control                                                             | Pin            | Result                                                      |
| ------------------------------------------------------------------- | -------------- | ----------------------------------------------------------- |
| Explicit-file native positive                                       | OpenCode 2.1.4 | FAIL: HTTP 400 `model_not_supported`, no completed response |
| Unregistered/install/resume/update/remove/reinstall/outside-project | OpenCode 2.1.4 | NOT RUN after failed positive                               |
| Separate package skill/resource and fallback context hook           | OpenCode 2.1.4 | NOT RUN after failed positive                               |

The [summary](evidence/2026-10-25-opencode-explicit-paths/summary.json) records
`positive_control_failed`. This does not establish that OpenCode ignores
explicit instruction paths, nor that its skill or hook routes are unsupported.
The [earlier failed positives](gate6-opencode-current-routes.md) retain their
own receipts and are not replaced. No alternative test model is used.

Expected responses alone cannot establish a native file identity or one
delivered body per rule. The offline classifier requires separately observed
path and body-count fields; missing fields remain unobserved. This run supplies
neither. No caching or performance comparison, effective-input uniqueness,
Linux, or IDE/hosted qualification follows
([controls](evidence/2026-10-25-opencode-explicit-paths/controls.json),
[summary](evidence/2026-10-25-opencode-explicit-paths/summary.json)).

## Preservation and offline checks

Measured project and user configuration, rule files and manual neighbors retain
their exact bytes, presence and modes. Native session, cache, log and
authentication refresh effects remain outside whole-profile equality. No client
cache is directly cleaned or rewritten
([preservation](evidence/2026-10-25-opencode-explicit-paths/preservation.json)).

Fixture and classification tests preceded the harness extension. `just lint`
failed six new assertions before implementation, then passed 160 offline tests.
They enforce positive-first gating, exact marker identity and multiplicity,
per-control completion/pin/tool/workspace/session evidence, native path and body
identity, and neutral fixture roots. A separate failing regression rejected an
unnecessary native-path ordering requirement before it was removed. Guide
presence is mocked; no new guide-writing test is introduced
([validation](evidence/2026-10-25-opencode-explicit-paths/validation.json)).

Q5 remains open. The explicit-path retry has no PASS, so the future OpenCode
glob removal remains gated. The independent Codex attribution control can
continue; no production route, schema-1 state or durable record changes in this
packet
([handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
[summary](evidence/2026-10-25-opencode-explicit-paths/summary.json)).

Packet review adds a failed regression before rejecting contradictory native
paths or bodies in an absence control. Fixture tests now keep each rule marker
in its own file exactly once and reject response objects with extra keys. These
offline fixes do not rerun the failed native positive
([validation](evidence/2026-10-25-opencode-explicit-paths/validation.json)).
