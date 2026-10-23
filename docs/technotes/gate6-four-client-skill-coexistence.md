# Gate 6 four-client skill coexistence observations

Task [#372](https://github.com/driftsys/upskill/issues/372) continues
[#337](https://github.com/driftsys/upskill/issues/337), Task 2/CD-11 of the
[corrected plan](../wip/plans/2026-10-19-customization-delivery-reconciliation.md).
These are sequential macOS arm64 project observations, not adoption of a root, a
production adapter or full four-client qualification. All 160 cells remain
unqualified and production stays schema 1.

## Candidate and native boundaries

One portable skill and its `references/review-procedure.md` live in a fresh Git
project's `.agents/skills`. The same bodies are copied into a local Claude
package outside shared catalogs. Ordinary native project marketplace/install
commands register that skill-only package; no development `--plugin-dir`, hook
change or trust bypass is used. The other three clients use intentional native
shared discovery. There are no client directives, variants or embedded includes.
Client invocation constraints remain outside the shared catalog.

All four normally provisioned isolated profiles read the same project. The
executables are frozen Codex `0.163.0`, Copilot CLI `1.0.98`, OpenCode `2.0.32`
and Claude Code `2.1.303`. Native model evidence identifies `gpt-6.1-sol`
through `openai` for Codex and `github-copilot` for OpenCode, and
`claude-sonnet-5.5` through `copilot` for Copilot CLI. Claude's native init
identifies model `claude-sonnet-5-5`; the selected init does not report its
provider identifier.

The
[version receipt](evidence/2026-10-22-gate6-four-client-coexistence/native-version-verification.json)
retains independent post-run `--version` output. Its binary fingerprints match
both capture boundaries in both packets. Original capture asserted version
output without retaining that stdout; the later receipt is labeled separately.
No authentication contents were inspected or copied. Profile use does not
qualify user-scope delivery.

Model tool surfaces are intentionally limited to skill/resource operations:
Claude `Skill`/`Read`, Copilot `skill`/`view`, Codex's native read-only sandbox
and OpenCode's default-deny permission list with exact skill/read grants.
Copilot's built-in MCPs are disabled for these requests. This is not general
permission management, MCP coexistence or scope/permission-denial qualification.
No global skill-discovery exclusion is added. Wider profile/configured readers
remain unresolved.

## Preliminary failed calibration remains failed

Packet A contains eleven completed turns and two skipped OpenCode controls:

- OpenCode loads the skill but its resource read receives native
  `Permission denied: read`. The fixture declared an absolute read resource; the
  earlier working probe used project-relative identity. The expected resource
  response fails and both OpenCode absence/survival controls are skipped. This
  is not a passed negative or a provider outage.
- Codex later reads the correct shared resource but includes the period beside
  the marker in the procedure text. Its exact response expectation fails;
  punctuation is not stripped to manufacture a pass.

Packet B is fresh and uses the intended exact project-relative grant, no
explicit OpenCode `plugins` key, and an unambiguous next-line output
instruction. It also has new project/package/marker identities. These are
different experimental contexts, not an unchanged-policy pair, a single-variable
causal proof or retroactive passes for packet A.

## Corrected common-project observations

Every declared final response in packet B matches exactly. `R` means the
resource marker, and `A` means the declared unavailable-skill fallback.

| Native condition                              | Claude | Copilot | Codex | OpenCode |
| --------------------------------------------- | ------ | ------- | ----- | -------- |
| Shared entry and Claude plugin enabled        | R      | R       | R     | R        |
| Whole shared entry temporarily removed        | R      | A       | A     | A        |
| Shared entry restored; Claude plugin disabled | A      | R       | R     | R        |
| Claude plugin re-enabled                      | R      | —       | —     | —        |

The [summary](evidence/2026-10-22-gate6-four-client-coexistence/summary.json)
retains all 24 completed turns across both packets: 22 exact responses and the
two preliminary failures. There are 33 native tool calls, including 17 resource
read attempts: 16 successful reads and the preliminary permission denial. Four
Codex native skill-body expansions are counted separately from tool calls.

The corrected context has thirteen completed turns, nine successful resource
reads and four unavailable controls. Each positive has exactly one fixture skill
activation or native Codex expansion and one resource read. Claude's actual
`Read` path is its installed local marketplace package source, not the shared
directory; these observations do not establish a copied cache layout. The other
clients' read paths bind the shared directory. Equal markers alone would not
establish these source identities.

Copilot and OpenCode absence turns contain unsuccessful native skill calls.
Claude's disabled turn has no advertised fixture skill and no tool call; Codex's
absent turn has no matching metadata/expansion or read call. The latter two are
catalog/context absence plus successful exact fallbacks, not failed file-load or
permission-denial events. Namespace aliases, every possible source and
direct-plus-plugin duplicates are not exhaustively tested. One fixture
activation per positive turn is not a global duplicate-activation proof.

## Preservation and receipts

The
[packet A assessment](evidence/2026-10-22-gate6-four-client-coexistence/a-fixture-and-assessment.json)
and
[packet B assessment](evidence/2026-10-22-gate6-four-client-coexistence/b-fixture-and-assessment.json)
retain exact before/after manual team/configuration and source/package
bytes/modes. The whole shared entry is restored in guarded cleanup. Selected
profile configuration candidates also agree; absent candidate paths are shown as
absent. Those candidates are not a complete configuration inventory.

Native Claude install/disable/enable/uninstall/marketplace removal targets only
the new fixture identifiers. Manual project settings environment content and
mode survive. Empty `enabledPlugins` and `extraKnownMarketplaces` objects remain
after removal, so initial settings bytes are not restored. Complete unrelated
native-registration preservation and cache/cloud side-effect recovery are not
established. Fixtures and private packets remain preserved.

Selected per-turn CLI receipts and
[native session projections](evidence/2026-10-22-gate6-four-client-coexistence/native-session-projections.json)
bind successful completion, model/session identities, skill expansions and
actual tool/read paths. The
[source index](evidence/2026-10-22-gate6-four-client-coexistence/private-source-index.json)
fingerprints both original manifests, each frozen executed helper, six owned
Codex sessions, the version check and the retention helper. All original hashes
and the
[selected manifest](evidence/2026-10-22-gate6-four-client-coexistence/receipt-hashes.json)
were checked. Provider state, account identifiers, unrelated metadata,
accounting and world state are omitted from selected receipts. Negative Copilot
error-result text is omitted with its digest while native failure status and
input identity remain visible. Private helpers are not qualified runners; these
records are informative data, not product input.

User/Linux/IDE, simultaneous sessions, implicit activation, exhaustive default/
configured/compatibility readers, imports/globs, namespace aliases, duplicates,
scoped rules, actual subagent restrictions, MCP, timeout cleanup and integrated
ownership/update/removal/recovery remain required. No override/composition
execution, new owned-skill lifecycle or route adoption follows. Task 2/CD-11 and
schema-2 activation remain blocked.
