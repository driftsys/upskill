# Gate 6 Claude managed-root controls

Task [#384](https://github.com/driftsys/upskill/issues/384), Q6 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains project- and user-scope marketplace controls on 2026-10-24. After normal
isolated login renewal, all six fresh invocations passed. Q6 records source edit
visibility before and after native update at both scopes. The two earlier
authentication failures remain retained. Production remains schema 1.

## Method and pin

Claude Code 2.1.308 on macOS arm64 uses the
[2026-10-24 campaign revision](gate6-plugin-profile-inventory.md). The other
campaign pins are Codex 0.164.0, Copilot 1.0.103 and OpenCode 2.1.4; none ran a
control in this packet. Model selection remains private.

The existing `scripts/qualify-plugin-rules.py` builds a new neutral fixture root
outside the repository and documented discovery directories. Its `delivery/`
managed root contains a local marketplace manifest and one self-contained
package under `packages/`. Each project has a distinct empty Git root commit and
an unrelated manual environment setting in `.claude/settings.json`, mode `0640`.
No team guidance files or symlinks are created. The builder rejects existing
roots, including dangling symlinks, before writing.

The private native helper uses the previously provisioned isolated Claude
profile, strips inherited Git/client overrides through an allowlisted
environment and disables automatic updates. A source-key audit of the isolated
profile and project settings found no explicit instructions, skills or plugin
source selectors. The managed root is outside all declared discovery roots; this
audit does not qualify arbitrary real-user globs or other clients.

Project scope runs first, then user scope in a separate Git project. Each phase
validates the marketplace, adds it through `claude plugin marketplace add`,
installs through `claude plugin install`, captures native inventory and starts a
fresh skill request. Native uninstall and marketplace removal run afterward,
including on failed positives. The helper does not write native plugin caches.

## Observations

| Control                            | Pin            | Project scope                                     | User scope                      |
| ---------------------------------- | -------------- | ------------------------------------------------- | ------------------------------- |
| Marketplace validation/addition    | Claude 2.1.308 | PASS: native exit 0                               | PASS: native exit 0             |
| Plugin install and inventory       | Claude 2.1.308 | PASS: enabled project registration                | PASS: enabled user registration |
| Runtime init load path             | Claude 2.1.308 | Managed source under `delivery/packages/`         | Same managed source             |
| Inventory `installPath`            | Claude 2.1.308 | Distinct native cache path                        | Distinct native cache path      |
| Initial invocation                 | Claude 2.1.308 | PASS: namespaced Skill call, exact initial marker | PASS: same                      |
| Source edit without update         | Claude 2.1.308 | PASS: fresh session returns exact edited marker   | PASS: same                      |
| Native update and fresh invocation | Claude 2.1.308 | PASS: update exit 0; exact edited marker          | PASS: same                      |
| Uninstall and marketplace removal  | Claude 2.1.308 | PASS: native exit 0                               | PASS: native exit 0             |

The [summary](evidence/2026-10-24-claude-managed-root/summary.json) retains all
three controls at each scope. Each request has one native `Skill` call with the
exact `<plugin>:<skill>` identity, a correlated successful tool result, an exact
expected marker, exit 0, `is_error: false` and no permission denial. The marker
was absent from the request. Separate sessions observed the initial source and
then the edited source; native update also succeeded. No reinstall was required
for this observed fresh-session edit loop. This does not establish visibility
inside an already running session.

Runtime init reports the managed source path while native inventory reports a
cache `installPath`. These are distinct observations. The runtime source version
changes from `0.0.0` to `0.0.1` after editing the package, before native update.
The packet does not infer cache synchronization or recovery from this result.

The
[initial attempt](evidence/2026-10-24-claude-managed-root/initial-attempt-summary.json)
retains both failed positives: expired OAuth, exit 1 and `is_error: true`, even
though the result subtype says `success`. Edit/update candidates were not run in
that attempt. The successful retry uses a new fixture and normal renewed login;
no credential store was copied or exported.

## Preservation and residue

The initial
[preservation record](evidence/2026-10-24-claude-managed-root/preservation.json)
and the
[retry preservation record](evidence/2026-10-24-claude-managed-root/retry-preservation.json)
retain settings-file hashes and modes before installation, after native removal
and after exact restoration. At project scope the project settings retain added
empty `enabledPlugins` and `extraKnownMarketplaces` objects; the user settings
remain unchanged. At user scope the project settings remain unchanged and the
user settings retain those empty objects. No other semantic settings changes or
mode changes were observed.

This is declared native empty-object residue, not original-byte equality after
uninstall. After recording it, the helper restores the measured settings to
exact pre-control bytes, modes and presence. The unrelated manual value
survives. Both native registration metadata files, `known_marketplaces.json` and
`installed_plugins.json`, have unchanged bytes and modes after removal in each
phase. Unexpected settings changes would have stopped automatic restoration and
been retained as preservation failures.

A
[post-removal cache observation](evidence/2026-10-24-claude-managed-root/post-removal-cache.json)
and the
[retry cache observation](evidence/2026-10-24-claude-managed-root/retry-post-removal-cache.json)
record that the reported cache entry remains a regular directory without symlink
descendants after removal. Its persistence is a native-managed effect, not exact
whole-profile restoration. Native session/log and authentication state are not
whole-profile restoration evidence. Native-managed effects remain separate from
the measured settings contract. No direct cache writes or cleanup were
performed.

## Consequence for the proposal

The affected rows are the proposal's
[Storage and activation](../wip/specs/2026-10-23-customization-delivery-proposal.md#storage-and-activation)
and
[Packaging changes identity](../wip/specs/2026-10-23-customization-delivery-proposal.md#packaging-changes-identity)
sections. Local marketplace registration and namespaced catalog identity are
observed at both scopes, with successful namespaced calls and edited-source
visibility in fresh sessions before and after native update. Empty settings
residue matches the stated allowance. This is bounded Q6 evidence; complete
profile qualification, production adoption and durable-record amendments remain
gated. Durable-record amendments wait for step 7.

## Provenance and offline checks

Raw transcripts, fixture sources and the capture helper remain outside the
repository. The
[source index](evidence/2026-10-24-claude-managed-root/private-source-index.json)
and receipt manifests retain their hashes. Selected init/result events omit
model identifiers, thinking, usage and unrelated profile information. Private
paths are normalized; complete raw-output hashes differ from retained hashes.

Tests preceded implementation: `just lint` failed in three tests referencing two
missing Q6 helpers, then passed all 45 offline tests. A further dangling-symlink
regression failed before its guard and passed afterward. The subsequent lint run
passed all 46 offline tests. Tests cover the package layout, copy-only fixtures,
excluded discovery roots, preserved existing roots, distinct project identities
and exact settings-residue classification. Only added empty native registration
objects are allowed; unrelated JSON changes, mode changes and formatting-only
byte changes fail preservation classification.

Review regressions also cover nonempty marketplace residue, existing native
registrations, repository-relative roots and JSON boolean/number type changes.
The type regression failed before type-sensitive comparison and passed after it.
All three reviewed mutations are rejected; the subsequent full verification
passes all 48 offline tests. A further positive regression preserves existing
native registrations and unchanged boolean/numeric values when object keys are
reordered and an allowed empty sibling registration is added. The latest full
verification passes all 49 offline tests.

Offline checks never call a client. Linux, IDE/hosted surfaces, running-session
edit visibility, native cache recovery, interrupted operations and integrated
upskill ownership remain unqualified. This packet promotes no production adapter
or complete Gate 6 profile.
