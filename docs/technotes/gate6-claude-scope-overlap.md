# Gate 6 Claude package scope overlap

Task [#409](https://github.com/driftsys/upskill/issues/409), Q10 of the
[delivery handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains a successful user-first retry on 2026-10-25. Claude Code 2.1.308 showed
one visible target catalog entry and one activation with both scopes registered
in the same workspace, then after either scope was removed. The
[summary](evidence/2026-10-25-claude-scope-overlap/summary.json) records this
bounded PASS. Production remains schema 1; broader qualification and Q11 remain
separate gates.

The earlier failed positive and its project-first protocol deviation remain in
the
[initial summary](evidence/2026-10-25-claude-scope-overlap/initial-attempt-summary.json).
The retry does not change that attempt's result or reorder its receipts.

## Pins and authentication

The [retry campaign revision](gate6-plugin-profile-inventory.md) freshly pins
Claude 2.1.308, Copilot 1.0.103, Codex 0.164.0 and OpenCode 2.1.4 on macOS
arm64. Only Claude ran a control. The other commands queried versions;
[Q5 remains deferred](https://github.com/driftsys/upskill/issues/391).

The
[authentication record](evidence/2026-10-25-claude-scope-overlap/auth-recovery.json)
retains the recovery prerequisites. The old token was revoked. Initial renewed
login could not be saved because the isolated home had no default macOS Keychain
reference. Native `security default-keychain` selected the existing login
Keychain for that home; the normal home's default and search list were
unchanged. Ordinary native login then succeeded. The Claude helper also supplies
`USER` and `LOGNAME` from the native OS account: the sanitized environment
without those identity keys could not find the saved login. A fresh API request
then returned `OK`. No credentials were read or copied, no Keychain ACL was
changed and no client discovery setting was changed. This authentication probe
does not itself qualify package behavior.

## Fixture and sequence

The existing `scripts/qualify-plugin-rules.py` reuses the
[Q6 neutral marketplace fixture](gate6-claude-managed-root.md). One package owns
a namespaced skill and a relative resource. The resource marker appears only in
the resource; a separate activation receipt appears in the skill body. Neither
marker appears in the request. The managed source is outside repository and
client discovery roots. No team guidance files or symlinks are created. The
reused builder's second project remains unused and supplies no Q10 observation.

The
[native steps](evidence/2026-10-25-claude-scope-overlap/retry-native-steps.json)
and
[sequence assessment](evidence/2026-10-25-claude-scope-overlap/retry-sequence-assessment.json)
retain this order, using the same package identity and workspace:

1. Install at user scope and capture the first user-only positive before any
   project installation. Uninstall it and remove its user marketplace.
2. Register and install at project scope for a separate project-only positive.
   Uninstall it and remove the project marketplace.
3. Reinstall at user scope and capture a refreshed user-only positive.
4. Add the same package at project scope and capture overlap.
5. Remove the project registration and capture the surviving user activation.
6. Reinstall the project registration, remove the user registration and capture
   the surviving project activation. Perform native cleanup afterward.

Each request starts a fresh native session with only `Skill` and `Read`, empty
strict MCP configuration, ordinary permissions and no session persistence. The
first failed positive would stop subsequent phases. During overlap one
user-scoped marketplace supplies both package registrations; simultaneous
user/project marketplace declarations and collaborator onboarding are not
tested.

## Native observations

The
[selected observations](evidence/2026-10-25-claude-scope-overlap/retry-observations.json)
retain both scope bindings, exact namespace, native session/tool identities,
correlated results, full visible skill expansion and resource-only response.
Each row has one target entry in each of the `skills` and `slash_commands`
catalogs, one `Skill` call, one visible body expansion and one `Read` call:

| Control                              | Pin            | Result                                       |
| ------------------------------------ | -------------- | -------------------------------------------- |
| First user-only positive             | Claude 2.1.308 | PASS: one entry and activation               |
| Separate project-only positive       | Claude 2.1.308 | PASS: one entry and activation               |
| Refreshed user-only positive         | Claude 2.1.308 | PASS: one entry and activation               |
| Both package scopes in one workspace | Claude 2.1.308 | PASS: two bindings, one entry and activation |
| User surviving project removal       | Claude 2.1.308 | PASS: one user binding and activation        |
| Project surviving user removal       | Claude 2.1.308 | PASS: one project binding and activation     |

All six native session identities are distinct. Every invocation exits 0 with
`is_error: false`, no permission denial and the exact resource marker. No
duplicate target exposure or visible activation was observed. The
[capture-time replay](evidence/2026-10-25-claude-scope-overlap/retry-offline-replay.json)
and
[separate raw-count audit](evidence/2026-10-25-claude-scope-overlap/retry-independent-audit.json)
check the six phases and user-first order. The central five-phase classifier
also returns `observed_single_entry_and_activation`; the extra first-user
calibration is checked through its phase inspector and the sequence audit.

This is pinned visible evidence for one fixture and package version `0.0.0`. It
does not establish provider-effective input uniqueness, arbitrary package
behavior, other clients or platforms. The packet must merge with its technote
indexed before Q10 is done. Q11 and the other slice-2 entry gates remain
separate; no production route or durable record changes with this packet.

## Preservation and residue

The
[retry preservation record](evidence/2026-10-25-claude-scope-overlap/retry-preservation.json)
compares settings presence, bytes and modes, both native registration metadata
files and package source files. User and project settings each retained only
added empty `enabledPlugins` and `extraKnownMarketplaces` objects after native
removal. After recording those objects, the helper restored both settings files
to their original bytes, modes and presence. The unrelated manual value
survived. Both registration metadata files and all measured package source files
matched their original hashes and modes.

Two redundant final cleanup attempts exited 1: the
[user uninstall](evidence/2026-10-25-claude-scope-overlap/retry-35-final-uninstall-user.stderr)
found the already removed registration, and
[project marketplace removal](evidence/2026-10-25-claude-scope-overlap/retry-36-final-market-remove-project.stderr)
found the declaration already removed after calibration. Required earlier
removals succeeded, and the measured final metadata equality confirms
restoration. These command failures remain visible in the receipts.

The
[retry cache observation](evidence/2026-10-25-claude-scope-overlap/retry-cache-after.json)
records that native `0.0.0` cache storage remains a regular directory after
uninstall. No direct cache writes or cleanup were performed. Measured
preservation passed; whole-profile equality is not claimed. Authentication, the
isolated home's Keychain reference, native app bookkeeping, logs and cache
effects remain outside that equality claim.

## Initial failed attempt

The
[original observation](evidence/2026-10-25-claude-scope-overlap/project-positive-observation.json)
and [original steps](evidence/2026-10-25-claude-scope-overlap/native-steps.json)
retain the first project-only calibration. Native installation and its catalog
succeeded, but the invocation exited 1 with HTTP 401 OAuth token rejection and
`is_error: true`, despite subtype `success`. No skill call, body expansion or
resource read occurred. User calibration, overlap and surviving-scope controls
were not run in that attempt. A catalog alone did not establish activation.

Starting with project calibration deviated from
[task #409](https://github.com/driftsys/upskill/issues/409), which requires a
successful user-only positive before project installation. The fresh retry
corrected the sequence; the original failed positive and deviation remain
retained. Q6's separate-project results also remain distinct from Q10 overlap.

## Offline checks and provenance

Offline fixture and classification tests preceded the harness extension, with
`just lint` failing on missing helpers before passing after implementation. Full
review later found a missing stream-completeness guard; 20 regression examples
failed before that fix. The inspector now requires explicit complete stream
evidence. Missing, malformed and non-Boolean completeness evidence cannot
establish activation. The separate
[review verification](evidence/2026-10-25-claude-scope-overlap/review-verification.json)
records 218 suite tests, 21 focused tests and 16 rejected author mutations. The
[earlier verification](evidence/2026-10-25-claude-scope-overlap/verification.json)
describes its own source epoch. The
[retry pre-native lint](evidence/2026-10-25-claude-scope-overlap/retry-q10-retry-pre-native-lint.log)
again passed all 218 tests before the native retry. No public harness or test
code changed for this retry.

The
[private source index](evidence/2026-10-25-claude-scope-overlap/private-source-index.json)
locates raw transcripts, fixture sources, capture-time harness/tests and helper
hashes outside the repository. Earlier epochs remain unchanged. The
[retry manifest](evidence/2026-10-25-claude-scope-overlap/retry-private-manifest.json)
indexes the new source epoch, excluding generated Python bytecode and the
manifest itself. The
[receipt hashes](evidence/2026-10-25-claude-scope-overlap/receipt-hashes.json)
cover retained files. Public paths use `$PACKET_D` and `$PROFILE` for the retry;
earlier epochs retain their aliases. Selected events omit model identifiers,
usage, thinking, unrelated catalogs and assistant prose. Complete raw output
hashes differ from selected retained output hashes.
