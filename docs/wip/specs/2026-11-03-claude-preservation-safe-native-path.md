# Claude native delivery with exact settings preservation

Status: offline design hypothesis, 2026-11-03. No additional native execution,
provider request, production adapter or admission change is authorized.

## Decision to investigate

Keep the approved local-catalog native route and explicit project/user scopes.
Investigate predeclaring only the owned settings entries with the existing
span-preserving planner before invoking native registration and installation.
Require the native commands to preserve an already-declared document exactly.
This may avoid the serializer path that the original probe exercised. It is a
hypothesis about the pinned client, not an observed capability.

The initial target remains Claude 2.1.308. A successful project case cannot
substitute for user-scope evidence or for isolation and session-retention gates.

## Evidence that constrains the design

The
[retained probe](../../technotes/2026-11-01-claude-local-catalog-contract.md)
installed the package but reordered pre-existing settings keys. The before and
final documents have equal JSON values and unequal bytes. The
[integrated regressions](../../technotes/2026-11-03-claude-settings-preservation-regressions.md)
prove exact forward/inverse pure patches and coordinator refusal after byte-only
or mode-only native drift. The test source from `e796e35` is already integrated;
full verification passed independently at `e4c793d`. Those are application-side
controls, not a native preservation result.

The retained 2.1.308 help permits explicit project/user/local scopes for
[installation](../../../tests/fixtures/native-adapters/2026-11-01-help/claude-plugin-install-help.stdout)
and
[marketplace registration](../../../tests/fixtures/native-adapters/2026-11-01-help/claude-plugin-marketplace-add-help.stdout).
The reviewed help does not establish a settings-preserving materialization mode.
No new flag is invented by this proposal.

Current
[official settings reference](https://code.claude.com/docs/en/settings-reference)
documents `enabledPlugins` selectors and `extraKnownMarketplaces` declarations.
These identify the settings contributions to investigate; they do not promise
byte-preserving command behavior on 2.1.308.

Current [settings documentation](https://code.claude.com/docs/en/settings)
describes local-settings writes that can change global Git exclusions, and
worktree settings located at the main checkout root. Those are documented
counterexamples to treating a change to local scope as harmless isolation. They
were not observed in the pinned probe.

The [CLI reference](https://code.claude.com/docs/en/cli-reference) describes
`--settings` and `--setting-sources` as loading/override controls. It supplies
no write-confinement guarantee for the captured plugin mutation sequence. A
session flag or wrapper is therefore not adopted as an installation solution.

## Proposed transaction

1. Capture eligible configuration under D6 and bind exact roots, presence,
   bytes, modes and relevant precedence layers. This proposal grants no access
   to an ordinary credential-bearing profile. Reject ambiguous ownership,
   masking, changed roots, duplicate keys and an unowned matching selector,
   including an equal-value manual selector.
2. Stage and verify the immutable package and single selected catalog revision.
   Derive the full owner name, selector and scope from the existing compiler.
3. Plan the owned declarations through `plan_config_contributions`. The
   marketplace key is the native marketplace name; the enable key is the full
   plugin-at-marketplace selector. Keep their ownership/reference records
   distinct. Preserve unrelated lexical spans and existing ordinary mode.
4. Use the existing `BeforeNative` exposure order and durable operation intent.
   Apply the planned owned spans, then invoke the existing explicitly scoped
   native command vectors. Native registration, cache materialization and
   installed-plugin records remain native responsibilities. Do not write those
   registries or populate a native cache manually.
5. Establish qualified native-worker quiescence before stable after-state
   observations. Require every protected document to match its exact planned
   file value after each command. Confirm actual registration and effective
   binding separately, with the exact scope, catalog source and version. An
   already-declared result or nonzero exit is insufficient by itself. Ambiguous
   effects or unavailable worker-quiescence evidence retain recovery state.
6. Publish authoritative state only after all file/native observations satisfy
   the existing coordinator. Refresh and detach additionally need the approved
   resource-retention/session controls. Preserve surviving references; absence
   of an enable entry must not be mistaken for native detachment.

The mechanism needs a native no-op contract for declarations that already have
the desired value or are already absent. A native command that still reorders
keys, changes mode, recreates an absent settings file or removes another owner's
entry falsifies the candidate. Preserve that failure and stop. Whole-file
restoration after the command is not part of the transaction.

## Offline work before another experiment

Extend the real coordinator acceptance harness with a fake native client that
checks its declaration preconditions and leaves already-declared settings
unchanged. Verify the exact forward document before the native effect and
completion only after independent native observations. Keep the existing
unconditional serializer and mode-only rewrite as negative controls.

Cover project and user separately, initially present and absent files, manual
neighbors, shared references, a concurrent semantic-preserving reformat, update
and final detach. Pin declaration keys and native scope independently. Exercise
the native already-declared response without converting it directly into a
confirmed binding. Preserve legacy CLI behavior and the empty production
admission registry.

The pure planner and fake positive cannot prove that the actual client has the
required no-op behavior. No complete adapter claim follows from those tests.

## Future native falsification and alternatives

A later separately approved, fresh disposable packet could compare predeclared
owned settings with the retained empty-container failure. It would bind the
launcher/version, generated declarations, exact file samples and command budget
before execution. No command is scheduled or authorized by this design. A
passing diagnostic would still leave effective roots, worker quiescence,
isolation, duplicate exposure and session retention to their own controls.

If the native client rewrites predeclared settings, retain the current blocked
profile. Investigate an upstream selective/no-op writer or a documented native
primitive that prepares an inactive scoped registration without rewriting shared
settings. Such a primitive is a requirement to establish, not an available flag.
Any changed native version or distinct configuration surface needs its own
reviewed profile. Changing scope, weakening exact preservation, temporary
user-wide activation and direct registry repair do not resolve this design.

## Offline feasibility checkpoint, 2026-11-04

The coordinator acceptance test now exercises eight combinations: project/user
scope, initially present/absent settings, and available/unknown after-state
binding observations. The fake checks the exact planned declaration bytes and
mode before its effect. Known after-state permits completion; an unknown binding
retains recovery state and prevents authoritative lockfile publication. This
uses the real coordinator and supplied fake metadata, not a native executable.

A compiled mutation that moved `BeforeNative` configuration exposure after the
effect failed the exact-declaration assertion. Production source was restored
byte for byte. The complete focused file passed four tests, and targeted Clippy
passed with warnings denied. Evidence is retained in
`/private/tmp/claude-predeclaration-restored-tests.log`,
`/private/tmp/claude-predeclaration-clippy.log`, and
`/private/tmp/claude-predeclaration-order-mutation.json`.

This establishes only the proposed ordering and completion boundary under the
fake's no-rewrite behavior. The additional lifecycle controls below extend this
offline boundary. Actual native no-op behavior remains unestablished; the
adapter and production profile remain incomplete.

## Offline lifecycle controls, 2026-11-04

Additional coordinator acceptance cases exercise project and user scope with
initially present and absent settings. One installation has both a direct
reference and a bundle reference. Removing the direct reference must preserve
both owned declarations and the native selection without another native effect.
A payload refresh must preserve the exact settings document. Removing the final
bundle reference must expose the exact original settings value, including file
absence, before the native detach effect. The resulting state must relinquish
configuration ownership while retaining both immutable resource revisions.

Separate stale-capture controls reformat settings without changing JSON values
before the first operation and before an installed no-op. The latter omits an
explicit configuration capture to exercise the coordinator's implicit prior
configuration precondition. Both cases must preserve the edited bytes and mode,
leave the authoritative lockfile unchanged and perform no further native effect.

The supplied no-rewrite fake now leaves the settings file untouched, including
when it is absent. It observes exact declarations at the native boundary; it
does not simulate successful preservation by rewriting the expected document.
These are offline transaction controls, not native serializer,
worker-quiescence, isolation, shared ownership by distinct targets or
session-retention evidence.

All six focused preservation tests and targeted Clippy passed. Mutations that
omit implicit configuration capture or skip settings-file deletion failed the
new acceptance assertions. The
[regression record](../../technotes/2026-11-03-claude-settings-preservation-regressions.md#offline-follow-up-2026-11-04)
retains the receipts and the scoped review's deferred revision-metadata
assertion.
