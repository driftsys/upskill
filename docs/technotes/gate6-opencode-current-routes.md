# Gate 6 current OpenCode routes: failed execution positives

Task [#391](https://github.com/driftsys/upskill/issues/391), Q5 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains two failed native positives on OpenCode 2.1.4, macOS arm64, 2026-10-24.
The provider rejects the requested private backend with HTTP 400 and
`model_not_supported`. Context-hook, resume and skill/resource model controls
are not run after either failed positive. Those routes remain unqualified.
Production remains schema 1.

## Fixtures and pin

The [dated campaign](gate6-plugin-profile-inventory.md) pins OpenCode 2.1.4.
Each attempted request checks that version. Claude 2.1.308, Copilot 1.0.103 and
Codex 0.164.0 run no controls in this packet.

The existing `scripts/qualify-plugin-rules.py` now prepares Q5 without creating
`CLAUDE.md`, `AGENTS.md` or nested equivalents. Independent empty Git roots
separate a native instruction positive from a candidate package project.
Unpredictable positive, rule, resource and fallback markers are distinct.
Managed packages sit outside documented discovery paths; no symlinks or direct
client cache writes are used.

The positive project registers its own `native-positive.md` through an explicit
`instructions` entry and denies all tools. The candidate project registers two
local context plugins, using the existing context-hook template and the pinned
SDK dependency. A package-owned skill source is explicitly registered through
`skills`; its response exists only in a supporting resource. Candidate
permissions deny all operations except the declared skill and resource read. No
existing user or team discovery setting is altered.

## Model preflight and native results

The first disposable project adds a documented model alias through
`providers.<provider>.models.<alias>.modelID`. The native catalog lists that
entry. Its positive request nevertheless fails with `model_not_supported`.

A second fresh fixture uses the requested backend identifier as both its catalog
key and configured `modelID`, removing alias translation as a variable. Its
positive also fails with the same provider error. Both requests explicitly
select the configured route. Native session exports record the selected catalog
entry; retained Boolean audits compare it with the declared private selection.
No alternate model is substituted. A later normal-profile catalog lookup also
omits the exact requested reference and matching model-name suffix. That
[later preflight](evidence/2026-10-24-opencode-current-routes/later-normal-catalog.json)
makes no model request; its version is observed afterward. It does not establish
availability across every provider configuration, and normal-profile byte
equality is not measured. A catalog entry and selected session metadata do not
prove backend inference succeeded.

| Control                             | Pin            | Result                                                                          |
| ----------------------------------- | -------------- | ------------------------------------------------------------------------------- |
| First native positive               | OpenCode 2.1.4 | FAIL: HTTP 400, requested model unsupported, no final response                  |
| Second native positive              | OpenCode 2.1.4 | FAIL: same, using direct catalog/backend identifier                             |
| Context hook and resume             | OpenCode 2.1.4 | NOT RUN after failed positives                                                  |
| Explicit package skill and resource | OpenCode 2.1.4 | NOT RUN after failed positives                                                  |
| Local-directory `plugin add`        | OpenCode 2.1.4 | Rejected in both fixtures: target must be npm registry or Git package specifier |

The [summary](evidence/2026-10-24-opencode-current-routes/summary.json) and
selected native error events retain both failures. The local install verb's
rejection is independent of backend execution. It does not establish whether
local directories registered through the `plugins` array load successfully.
Failed positives never become negative reader results or unsupported component
claims.

The [model configuration reference](https://opencode.ai/v2/docs/models/),
[plugin reference](https://opencode.ai/v2/docs/plugins/) and
[skill-source reference](https://opencode.ai/v2/docs/skills/) inform the
fixture. They are rolling documentation, not retained runtime qualification.
Custom-model context/output limit fallbacks and performance are not qualified.

## Preservation and offline validation

Later root assertions cover resolved native user homes and unchanged filesystems
after rejection. Final full verification passes 67 offline tests; the retained
native failures remain unchanged.

Project configuration and the isolated profile's two user configuration
candidates retain their exact bytes, presence and modes. Native local-directory
install rejection leaves those measured files unchanged. Session logs,
client-managed caches and authentication refresh effects are outside this
measurement; whole-profile restoration is not claimed. Credentials are neither
copied nor exported.

Raw transcripts, private model declarations and capture source remain outside
the repository. The
[source index](evidence/2026-10-24-opencode-current-routes/private-source-index.json)
and manifests bind both attempts. Public receipts redact the test identifier and
retain native errors, completion outcomes and configuration hashes.

Offline tests are written first: `just lint` fails eight missing-helper tests,
then passes all 66 offline tests after the harness extension. They pin neutral
roots, independent marker roles, package/skill registration, native-positive
failure handling, individual rule callback counts, resume identity, real
resource evidence and recording the local install verb either way. They never
invoke a client and cannot qualify execution.

The affected proposal rows are
[OpenCode rules](../wip/specs/2026-10-23-customization-delivery-proposal.md#rules),
[skills](../wip/specs/2026-10-23-customization-delivery-proposal.md#skills-and-resources)
and the current-pin evidence assessment. Q5 stays open until a successful pinned
positive permits the component controls. No older 2.0.32 execution result is
promoted to 2.1.4 support. Durable adoption waits for step 7.
