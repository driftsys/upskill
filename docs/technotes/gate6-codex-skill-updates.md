# Gate 6 Codex plugin skill update visibility

Task [#395](https://github.com/driftsys/upskill/issues/395), Q8 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains three passing native turns on Codex CLI 0.164.0, macOS arm64,
2026-10-24. A namespaced installed skill expands from a native cache and reads
its supporting resource. Editing the managed resource and package version is
invisible before refresh. Native remove/re-add exposes the edited resource from
the new cache version. Measured configuration bytes and mode are restored
exactly. Production remains schema 1.

## Fixture and registration

The [dated campaign](gate6-plugin-profile-inventory.md) pins Codex 0.164.0. Each
request retains a pre-request version receipt, and each native session's
metadata reports the same CLI version. Claude 2.1.308, Copilot 1.0.103 and
OpenCode 2.1.4 run no controls in this packet.

The existing `scripts/qualify-plugin-rules.py` prepares one package under a
neutral managed root with `.codex-plugin/plugin.json`, one native skill and one
resource. The compatibility manifest declares `skills: "./skills/"`. The entry
instructs a relative resource read; initial and edited response markers exist
only in their respective resource bytes. No `CLAUDE.md`, `AGENTS.md`, nested
equivalent, symlink or hook definition is authored. Two disposable projects have
distinct empty Git roots; the unrelated project is prepared but not used as an
isolation control.

Native local marketplace add and plugin add create the owned registration. The
helper disables only its newly created user binding and creates one new project
activation entry. Normal native folder trust is accepted for that reviewed
fixture. It submits no UI model prompt and adds no hook trust. Existing
discovery entries remain unchanged.

The installed catalog contains one exact `plugin:skill` identity. Its aliased
skill path resolves to the client's native cache, and the cached entry initially
matches the managed source bytes. The
[catalog selection](evidence/2026-10-24-codex-skill-updates/native-catalog-selection.json)
and
[registration receipt](evidence/2026-10-24-codex-skill-updates/registration.json)
retain identity, load path and scope evidence. These observations do not qualify
arbitrary user delivery or unrelated-project non-activation.

## Native controls

All model turns use explicit read-only sandboxing, approvals set to `never`, and
disabled delegation. The normally authenticated isolated profile and stripped
child environment are reused without copying credentials. Explicit native
`$plugin:skill` invocation expands the skill body as a user context item. Each
turn reads its declared cached resource through a correlated native tool call.
Exact final responses are required.

| Control                   | Pin           | Managed source version | Native load version | Response                                           |
| ------------------------- | ------------- | ---------------------- | ------------------- | -------------------------------------------------- |
| Installed positive        | Codex 0.164.0 | 0.0.0                  | 0.0.0               | PASS: exact initial marker                         |
| Source edited, no refresh | Codex 0.164.0 | 0.0.1                  | 0.0.0               | PASS: exact initial marker; edit remains invisible |
| Native remove/re-add      | Codex 0.164.0 | 0.0.1                  | 0.0.1               | PASS: exact edited marker                          |

The source edit changes the supporting resource and package version. It leaves
the skill body unchanged. Only the managed source is edited; the helper never
writes the native cache. The refresh uses the native remove/re-add alternative
permitted by Q8, and disables the newly recreated user binding again so the
owned project activation remains the intended route.

The [summary](evidence/2026-10-24-codex-skill-updates/summary.json) classifies
this result as `edits_require_refresh`. Native expansion records retain the
exact namespaced identity and cache path. Tool call IDs bind each resource read
to its returned procedure text. Three expansions and three resource reads are
observed; callback counts or responses do not establish provider-effective input
uniqueness. This resource edit does not qualify every form of package update.

The
[official plugin packaging reference](https://developers.openai.com/plugins/build/plugins)
informs the fixture. The pinned run establishes the observed compatibility
manifest, namespace and update behavior.

## Preservation and provenance

The controller holds original user configuration bytes only in memory across
registration, native UI admission, model controls and cleanup. Native plugin and
marketplace removal run before it checks that unrelated parsed values still
match. It then restores the exact original bytes and mode and removes the new
project config, restoring its original absence. Credentials and original
configuration contents are never exported to the packet.

The
[restoration receipt](evidence/2026-10-24-codex-skill-updates/profile-restoration.json)
records exact return to this control's starting baseline. The later Q7 notice
counter correction separately restores the earlier pre-Q7 configuration; it is
not presented as part of Q8's own cleanup. Native session logs, cache parent
directories and other bookkeeping remain outside whole-profile equality. Native
cache removal is performed by the client, with any remaining owned files
recorded; the helper does not reclaim cache files directly.

Raw transcripts, native prompt inputs, private test model selection and source
stay outside the repository. The
[source index](evidence/2026-10-24-codex-skill-updates/private-source-index.json)
and receipt hashes bind them. Public event selections retain skill expansions,
resource call/results, version metadata and exact responses while omitting
unrelated catalog content and test model identifiers.

## Offline validation and consequence

Tests are written first: `just lint` fails seven missing-helper tests, then
passes 66 offline tests. A missing-response assertion fails before its
classifier fix and then passes 67 tests. Native-user-root exclusions also fail
before their guard correction and pass afterward. The tests pin package
identity, one skill, resource secrecy, independent roots, strict
completion/pins, correlated identity, load-path evidence and recording source
visibility either way. They never invoke a client or qualify native execution.

The affected proposal rows are
[Storage](../wip/specs/2026-10-23-customization-delivery-proposal.md#storage)
and
[skills](../wip/specs/2026-10-23-customization-delivery-proposal.md#skills-and-resources).
This bounded current-pin cache-copy update loop is observed. Durable adoption
waits for step 7. Linux, IDE/hosted surfaces, arbitrary user delivery,
unrelated-project isolation and integrated ownership/recovery remain
unqualified. Codex rules adoption and the proposed #379 severity amendment
remain at the user's step 7 review after resume-only attribution, as the adopted
handoff requires. This skill packet does not decide either amendment.

Current-main integration uses the shared neutral-root guard. A new failing
regression precedes the configured-home, case-alias and ancestor-guidance
rejection checks; guide presence is mocked. Git identity checks also use the
stripped fixture environment. Other main harness nodes are unchanged. These
offline changes do not repeat the native control, and original capture source
remains separate from integration source
([integration audit](evidence/2026-10-24-codex-skill-updates/main-integration-audit.json),
[validation](evidence/2026-10-24-codex-skill-updates/validation.json)).
