# Gate 6 Claude package scope overlap

Task [#409](https://github.com/driftsys/upskill/issues/409), Q10 of the
[delivery handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains a failed first native positive on 2026-10-25. Claude Code 2.1.308
returned HTTP 401 from the isolated profile. User-only calibration, overlap and
both surviving-scope controls were not run. This packet does not qualify overlap
or advance the slice-2 entry gate. Production remains schema 1.

## Method and pins

The [campaign revision](gate6-plugin-profile-inventory.md) pins Claude 2.1.308,
Copilot 1.0.103, Codex 0.164.0 and OpenCode 2.1.4 on macOS arm64. Only Claude
ran a native control. The other commands queried versions, including OpenCode;
[Q5 remains deferred](https://github.com/driftsys/upskill/issues/391).

The existing `scripts/qualify-plugin-rules.py` extends the
[Q6 neutral marketplace fixture](gate6-claude-managed-root.md). One package owns
a namespaced skill and a relative resource. A resource marker appears only in
the resource, while a distinct activation receipt appears in the skill body.
Neither marker appears in the request. The fixture lives outside the repository
and client discovery roots; no team guidance files or symlinks are created. The
reused Q6 builder also creates an unused second project. That project supplies
no Q10 observation.

The planned sequence was project-only calibration, user-only calibration in the
same workspace, both scopes, user surviving project removal, then project
surviving user removal. Each request would start a fresh native session with
only `Skill` and `Read`, empty strict MCP configuration and ordinary
permissions. The first failed positive stops later native requests. Native
cleanup still runs. Native plugin management uses an existing isolated
authenticated profile; the helper does not copy credentials, modify discovery
settings or write client plugin caches.

Starting with project-only calibration deviated from
[task #409](https://github.com/driftsys/upskill/issues/409), which requires a
successful user-only positive before project installation. This attempt stopped
at the initial failed positive. A future retry must start with user-only
calibration.

## Retained result

The [summary](evidence/2026-10-25-claude-scope-overlap/summary.json) and
[selected observation](evidence/2026-10-25-claude-scope-overlap/project-positive-observation.json)
retain these outcomes at Claude 2.1.308:

| Control                                                              | Pin            | Result                                                         |
| -------------------------------------------------------------------- | -------------- | -------------------------------------------------------------- |
| Native marketplace validation, project registration and installation | Claude 2.1.308 | PASS: each command exited 0                                    |
| Project-only catalog                                                 | Claude 2.1.308 | One target skill and one target slash-command entry            |
| Project-only invocation positive                                     | Claude 2.1.308 | FAIL: exit 1, HTTP 401 OAuth token rejection, `is_error: true` |
| User-only positive                                                   | Claude 2.1.308 | Not run                                                        |
| Both scopes in one workspace                                         | Claude 2.1.308 | Not run                                                        |
| User surviving project removal                                       | Claude 2.1.308 | Not run                                                        |
| Project surviving user removal                                       | Claude 2.1.308 | Not run                                                        |
| Native project uninstall and marketplace removal                     | Claude 2.1.308 | PASS: each command exited 0                                    |

The failed result has subtype `success`, but `is_error: true` and exit 1 make it
a failed positive. No `Skill` tool call, visible skill-body expansion or
resource read occurred. A catalog entry alone cannot establish activation or
uniqueness. The
[command receipts](evidence/2026-10-25-claude-scope-overlap/native-steps.json)
retain raw output hashes; provider error text, model identifiers, usage and
unrelated catalog entries remain private.

Q6's separate-project passes remain distinct. They do not answer same-workspace
overlap. Q10 needs a new successful positive after ordinary authentication
renewal before any overlap inference is possible. No authentication retry was
attempted in this packet.

## Preservation and residue

The
[preservation record](evidence/2026-10-25-claude-scope-overlap/preservation.json)
compares settings presence, bytes and modes plus both native registration
metadata files and package source files. Native removal left only added empty
`enabledPlugins` and `extraKnownMarketplaces` objects in project settings. User
settings were unchanged. After recording those empty objects, the helper
restored the project settings to the original bytes and mode, preserving the
unrelated manual value. Both registration metadata files and all measured
package source files matched their original hashes and modes.

The
[cache observation](evidence/2026-10-25-claude-scope-overlap/cache-after.json)
records that the native `0.0.0` cache path remains a regular directory after
uninstall. No cache cleanup was performed. Measured preservation passed;
whole-profile equality is not claimed. Authentication, native app bookkeeping,
logs and cache effects remain outside that equality claim.

## Offline classification and provenance

The new fixture and classification tests preceded the harness extension.
`just lint` first failed on missing Q10 helpers, then passed 206 tests. A public
phase-inspection regression failed before that helper was exposed. A subsequent
test incorrectly treated the valid `project` scope as invalid; its input was
corrected to an unknown scope. A further regression showed that a duplicate
already observed in overlap must remain a failure when later probes are absent.
It failed before the classifier ordering change. The final pre-native
[lint receipt](evidence/2026-10-25-claude-scope-overlap/q10-duplicate-green-lint.log)
passed all 208 tests.

The quick review found missing offline examples. Author-added cases now cover
both native installation resource paths, fixture resource-reading instructions,
partial additional body receipts, wrong workspace and namespace identities,
wrong binding versions and two reads after one skill call. All 15 focused tests
pass, and all eight reviewed mutations are rejected. These test-only additions
are separate from the unchanged capture-time source and native result; no
additional quick review of those fixes was run.

Full review confirmed a missing stream-completeness guard. A new regression
failed before the fix: malformed, missing or non-Boolean completeness evidence
could otherwise be counted. The inspector now requires explicit complete-stream
evidence; a malformed positive fails calibration, and malformed candidate
evidence is unobserved. Additional cases cover calibration cardinality, empty or
unrelated catalogs, reused surviving-scope sessions, tool errors, result
subtype, exit-status type and duplicate classification after removal. These
offline corrections do not rerun or alter the retained native attempt. The
separate
[review verification](evidence/2026-10-25-claude-scope-overlap/review-verification.json)
records the revised source; the earlier verification describes its own source
epoch.

The classifier requires two successful single-scope positives, exact native
scope bindings, fresh session identities, catalog visibility, correlated tool
results, full visible skill-body expansions and a resource-only marker. It
rejects duplicate exposure or activation, mismatched registrations, incomplete
controls and preservation failures. It counts visible native evidence, not
provider-effective input. The
[offline replay](evidence/2026-10-25-claude-scope-overlap/offline-replay.json)
uses the capture-time harness and preserves `positive_control_failed` for this
attempt.

The
[private source index](evidence/2026-10-25-claude-scope-overlap/private-source-index.json)
locates raw transcripts, fixture sources, capture-time harness/tests and helper
hashes outside the repository. The
[private manifest](evidence/2026-10-25-claude-scope-overlap/private-manifest.json)
indexes that source epoch, excluding generated Python bytecode and the manifest
itself. The
[receipt hashes](evidence/2026-10-25-claude-scope-overlap/receipt-hashes.json)
cover retained files. Public paths use `$PACKET_A` and `$PROFILE`; complete raw
output hashes differ from selected retained output hashes. No documentation
support claim, durable record or production route changes with this packet.
