# Gate 6 OpenCode user-configuration rule observations

Task [#375](https://github.com/driftsys/upskill/issues/375) continues
[#337](https://github.com/driftsys/upskill/issues/337), Task 2/CD-03 of the
[corrected plan](../wip/plans/2026-10-19-customization-delivery-reconciliation.md).
These are bounded OpenCode `2.0.32` macOS arm64 observations in a normally
provisioned isolated user profile. All 160 qualification cells remain
unqualified and production stays schema 1.

## Package CLI and configured local route

The first native `plugin list` fails because another process occupies the
default managed-service port. That process is untouched. After checking that the
isolated profile's service is stopped and its port setting unset, a temporary
unused port allows native inventory to complete. It reports no package plugins.
Native `plugin add` then explicitly rejects the new local directory: its
interface requires an npm registry package or Git package specifier. The
experiment stops its own service, confirms stopped status and restores the unset
port setting. No model request or local-package loading test occurs in this
phase.

The
[interface receipts](evidence/2026-10-23-gate6-opencode-user-rules/interface-observations.json)
retain both failures and cleanup. The initial default-port orchestration was an
inline snippet without a pre-frozen helper; its native arguments and outputs are
retained, not a qualified runner.

Rolling official [configuration](https://opencode.ai/v2/docs/config/) and
[plugin documentation](https://opencode.ai/v2/docs/plugins/) describe a separate
local-path route through `~/.config/opencode/opencode.json(c)`. They propose a
candidate; they are not pinned runtime proof. Two fresh native packets exercise
that route with explicit local package paths in the isolated profile's global
`opencode.json`. Native package CLI installation remains a separate boundary.

Each packet has new package/project/marker identities. Two client-specific
packages stay outside shared catalogs; each supplies an independent rule body
through a v2 context callback. The callback accepts only the two declared
project paths and the `build` agent. A trace record binds its native session,
actual module directory, project and rule. Thus this is user configuration with
fixture-limited injection, not arbitrary-workspace user delivery, project-scoped
registration or an adopted adapter. No shared/compatible directory, other
client, alias, import/glob or direct-plus-plugin duplicate result follows.

No authentication contents were inspected or copied, and no discovery
suppression or trust override was used. Native exports bind provider
`github-copilot`, model `gpt-6.1-sol` and variant `default`. Model tool calls
are absent. The project permission list denies all model tool actions, but
tool-free results do not qualify general permissions, tools, MCP or hook
management.

## Paired native controls

The first configured packet starts with an absent global file. The second
creates a representative manual global settings baseline: an unpredictable
`username` value, an empty plugin list and mode `0640`. Only fixture plugin
entries change during its controls. It restores that baseline exactly before
removing the experiment-created file to its original absence. This exercises a
manual field and file mode, not arbitrary existing JSONC or unrelated plugin
arrays. Both packets retain distinct manual team markers and unchanged project
`AGENTS.md`/`opencode.json` bytes and modes.

Each packet declares seven successful native turns:

| Condition                                 | Project | Expected rules | Bound context callbacks |
| ----------------------------------------- | ------- | -------------- | ----------------------- |
| Both global entries present               | A       | A and B        | One per rule            |
| Both global entries present               | B       | A and B        | One per rule            |
| A entry removed                           | A       | B              | One for B               |
| Both entries removed                      | B       | None           | None                    |
| Both entries restored                     | A       | A and B        | One per rule            |
| Both entries restored                     | B       | A and B        | One per rule            |
| File removed, or manual baseline restored | A       | None           | None                    |

All fourteen turns complete with the declared JSON object, allowing formatting
and rule-array order, and preserve the correct team marker. Eighteen callbacks
bind the actual configured local package directories. All controls remain
sequential; this is not simultaneous-session or exhaustive duplicate activation
qualification. The two configured packets are fresh contexts, not an
unchanged-policy or single-variable causal pair.

The [summary](evidence/2026-10-23-gate6-opencode-user-rules/summary.json)
records these outcomes separately from the failed service/package interface
attempts. A rejected local directory is not evidence that the configured local
route is unsupported. Entry removal tests fixture context delivery; it does not
test native package uninstall or a production upskill lifecycle.

## Native evidence and preservation limits

[Selected CLI receipts](evidence/2026-10-23-gate6-opencode-user-rules/receipt-hashes.json)
retain the exact response text and native session identity.
[Native export selections](evidence/2026-10-23-gate6-opencode-user-rules/native-session-selections.json)
retain the same session, model, assistant agent and successful idle outcome. The
native sanitizer redacts directory identity and user/assistant text. Export
placeholders remain placeholders: response equality is checked against CLI text,
and project identity binds the command cwd plus positive callback records, not
an unredacted export directory. Exports do not provide a system-prompt message
dump. Injection observations bind callback source, actual module path and
unpredictable response markers instead.

The
[fixture assessment](evidence/2026-10-23-gate6-opencode-user-rules/fixture-and-assessment.json)
retains declared inputs, source bodies, native commands, manual preservation and
cleanup. Both packets restore global-file absence and preserve the old project
activation trace. The manual-baseline packet also restores its intermediate
baseline bytes and mode exactly. Global configuration uses whole-file ownership
of a fixture-created file; no production in-place editor or arbitrary existing
configuration preservation is qualified. Dependency/cache files, parent
directories and complete native side-effect recovery are outside these checks.

The
[source index](evidence/2026-10-23-gate6-opencode-user-rules/private-source-index.json)
binds 154 original manifest entries, fourteen native session exports and the
frozen preparation/control helpers. Original and selected hashes were checked.
Opaque provider state, account identifiers, reasoning and accounting are omitted
from selected records. Private helpers are not qualified runners, and the
retained JSON/JSONL is informative evidence rather than product input.

Other clients/platforms/scopes/IDE, fresh/resume/compaction, complete default
and configured readers, aliases/imports/globs, unintended and duplicate
activation, subagent/MCP restrictions, timeout cleanup and integrated
ownership/update/ removal/recovery remain required. No root or adapter is
adopted. Task 2/CD-03 and schema-2 activation remain blocked.
