# Customization delivery, discovery and activation

Status: intended architecture and acceptance contract, reconciled on 2026-10-19
under explicit user direction. This contract does not describe newly implemented
adapters or qualify a client. The
[roadmap](https://github.com/driftsys/upskill/blob/main/docs/roadmap.md) owns
release order;
[AD-0040](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md)
resolves conflicts with earlier experimental instruction plans.

## Architecture and authority

Delivery places files or registers a package. Discovery makes content available
to a runtime. Activation loads instructions, invokes a skill or agent, starts a
server, registers a tool or executes a hook. These are separate outcomes.
Ownership authorizes upskill changes; it does not authorize an incidental
reader.

Independent Codex and OpenCode rules use a qualified client plugin route beside
team `AGENTS.md`. Install, update and removal must leave team instruction bytes
and modes unchanged. Plugin packaging alone is insufficient: the installed
integration must deliver the intended guidance at the required lifecycle points,
with scope, precedence, exclusion and exactly-once activation evidence. A skill
inside a package is not automatically an always-on rule. The exact event/API and
package storage remain adapter qualification decisions. Missing capabilities
block that route; they never authorize an `AGENTS.md` rewrite fallback.

The merged Codex contribution compiler and `with_prepared_codex` are
**experimental library mechanisms**. Their explicit reader reports and guarded
editing tests remain useful foundation evidence. They are not the approved
production rule architecture, and identical Markdown for potential readers does
not establish intentional sharing. No existing experimental installation is
silently adopted or converted to a plugin installation. Recorded removal and
recovery retain their root/ownership safety contracts.

Claude and Copilot rules use native rule loading where qualification establishes
the requested behavior. Skills/resources use qualified native discovery where
possible, even when packaged in a plugin. Agents retain native client formats;
new owned-agent management is not added to M1. Bundles compose installations and
references; native plugins remain separate packages. Existing plugin/MCP
functionality must remain compatible without advancing general hooks, tools,
permissions or expanded MCP management into this implementation.

## Reader and activation policy

Every adapter/profile must declare the client/runtime/surface, exact version,
OS, project or user scope, effective roots and configuration. Its record
includes all entrypoints, resources, package payloads, native registrations,
imports, recursive discovery, compatibility directories, configured globs,
ancestor and nested roots, shared paths and relevant external-home overrides.
Inventory all four clients, plus known supported IDE/compatibility surfaces.
Unknown behavior is a blocker for an isolation claim, not an empty reader set.

For each possible reader record `intended`, `unintended` or `unresolved`, with
its discovery condition. Cross-client consumption defaults to unintended unless
explicitly selected and accepted for that installation/profile. Audience/body
compatibility, equal bytes, a directory name or a warning alone never supplies
that consent. A requested audience exclusion must prevent activation, not just
omit one generated copy. Incidental readers gain neither owners nor references.

A shared artifact is permissible only for explicitly intended consumers with
qualified equivalent behavior, compatible resources and recorded owner
contracts. Otherwise select an independently qualified route or fail preflight.
Do not disable existing user/team discovery or rewrite their settings to make a
negative control pass. Existing configurations with imports/globs that prevent
isolation are explicitly unsupported until an approved representation works.

Native catalogs and prompts must distinguish availability from activation. Each
unintended-reader negative needs a successful positive control in that same
client and an invocation relevant to the customization. For skills and agents,
inspect native catalog/registration and explicit invocation as well as behavior;
no marker in an unrelated prompt does not prove absence from discovery. Use
registration/event counters or equivalent native receipts to distinguish
repeated activation: a single final marker cannot prove exactly-once loading.
Test same-name and alias collisions, project/user overlap, compatibility copies
and direct-plus-plugin installation. One logical installation activates at most
once per intended event/invocation. Intentional repeated user invocations are
separate events. Client-specific precedence or deduplication is evidence to
verify, not a portable assumption.

## Preservation contracts

Every matrix row below inherits these installation, update and removal rules:

- **Files (F):** capture exact entrypoint/resource bytes and modes, enumerate
  owned resources individually, reject unowned collisions even when bytes match,
  and preserve manual files, unrelated owners and references. Update/remove
  rejects edited owned artifacts; no recursive deletion of a mixed directory.
  Rule resources must not become instructions through a broad Markdown glob.
- **Rules (R):** F plus no team `AGENTS.md`, override-file or
  imported-manual-text changes for independent plugin rules. Native
  Claude/Copilot rule files remain separately owned; existing bridge/settings
  entries need their own ownership evidence. Scope mappings must preserve
  matching and nonmatching behavior.
- **Native registrations (N):** F for package/config files plus observe existing
  registration, scope, enablement, trust and inverse actions. Add only owned
  entries; update/remove preserve other keys, comments, order where meaningful,
  credentials and manual registrations. Unknown/pre-existing registrations are
  conflicts or externally satisfied objects, not automatically owned. File
  success is not native effect success. Recovery observes native outcomes before
  retrying, and blocks if it cannot safely determine the result.
- **Composition (B):** preserve dependency identity, source/ref, aliases and
  direct/bundle reference edges. Removing one bundle releases only its edges;
  surviving direct/other-bundle references retain content, resources and native
  registrations. No new retention is inferred from incidental readers. Existing
  explicit integration-removal policy remains; narrowed selection cannot
  silently uninstall a plugin or MCP server.

Reuse complete selected-set preflight, captured source/destination evidence,
qualified roots and writer locking, exact schema-1 backup, owner contracts and
journaled recovery. Extend native-effect planning where needed; the file journal
alone is not proof of native registration recovery. Keep schema-1 production
routing until every applicable activation requirement passes. Never expose a
schema-2 writer alongside legacy recursive removal or save only the legacy
projection over an outer owned state.

## Per-client delivery matrix

The paths here distinguish current output from intended routes. They are not new
path guarantees. Project/user roots, package locations and compatibility
settings must be pinned for each released version before qualification.

Status vocabulary: **I** means code exists; **X** means experimental library;
**O** means a bounded native observation exists; **Q** means the exact recorded
native case passed; **P** means intended work remains. I and generated-file
tests do not imply O/Q. Historical Q applies only to its recorded
method/version, never automatically to this corrected delivery route.

### Rules

| Client          | Intended delivery, discovery and activation                                                                                   | Potential other readers and sharing intent                                                                                                                                                            | Preservation                                                            | Current status                                                                                                                          |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code     | Dedicated `.claude/rules` files; native rule discovery and qualified path matching                                            | Copilot explicit imports, unresolved version-specific compatibility or any configured client glob may read them; unintended unless explicitly accepted; verify both entrypoints and resource Markdown | R; preserve `CLAUDE.md`, manual rules and settings                      | I historical generation; O/Q bounded canonical discovery only; complete isolation/precedence P                                          |
| Copilot CLI     | Dedicated `.github/instructions/*.instructions.md`; native `applyTo` matching; CLI profile distinct from VS Code              | Copilot IDE surfaces and explicitly importing/globbing clients may read them; intended IDE sharing requires a separate profile, otherwise unintended                                                  | R; preserve repository/personal instruction files and existing settings | I historical generation; canonical `.github/copilot-instructions.md` observations do not qualify modular `applyTo` output; full route P |
| Codex CLI       | Client plugin with independent rule payload; qualified event/API adds guidance beside unchanged team `AGENTS.md`              | Other clients must not discover payload through `.agents`, `.claude`, imports, package compatibility or shared paths; cross-client activation unintended by default                                   | R + N; preserve team file and native plugin/config ownership            | X managed `AGENTS.md` contributions; O/Q canonical-file probes only; independent plugin delivery P and unqualified                      |
| OpenCode v2 CLI | Client plugin API supplies independent rules beside unchanged team `AGENTS.md`; qualify initial, resume and scoped activation | Broad `instructions` globs/imports, shared payload paths or other package loaders can expose it; other-client activation unintended                                                                   | R + N; preserve manual `instructions` and other plugin entries          | I legacy `.agents/rules` plus config registration; O incomplete canonical probes; independent plugin delivery P and unqualified         |

### Skills and supporting resources

| Client          | Intended delivery, discovery and activation                                                                            | Potential other readers and sharing intent                                                                                                                                  | Preservation                                                                 | Current status                                                                                                           |
| --------------- | ---------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Claude Code     | Native skill entrypoint and adjacent resources, standalone or plugin-packaged; qualified explicit/automatic invocation | `.claude/skills` is a Copilot/OpenCode compatibility source; those consumers are unintended unless explicitly selected; imported/shared paths need their own inventory      | F + N for packaging; resources never auto-executed on install                | I `.claude/skills`; owned lifecycle and cross-client/duplicate qualification P                                           |
| Copilot CLI     | Native skill discovery/invocation with resources; package location chosen by scope/profile                             | `.github/skills` can serve Copilot IDE/cloud surfaces; `.claude/skills` and `.agents/skills` compatibility copies may duplicate another installation; explicit sharing only | F; preserve other catalogs and manual skills                                 | I `.github/skills`; user-root mapping, owned lifecycle and native matrix P                                               |
| Codex CLI       | Qualified native skill discovery from profile-declared roots or plugin catalog, with resources                         | `.agents/skills` is also a Copilot/OpenCode source; plugin or extra-root exposure must be measured; other consumers unintended by default                                   | F + N if plugin; preserve resource paths/modes and independent manual config | P Codex adapter; portable skill format support is not installed upskill support                                          |
| OpenCode v2 CLI | Qualified native skill source or plugin-provided catalog, resources loaded only when needed                            | Current `.agents/skills` is shared with Codex/Copilot; `.claude/skills` compatibility can add a second copy; unintended exposure and duplicates block the profile           | F + N if plugin; no claim that current shared location is private            | I legacy `.agents/skills`; client-specific candidate `.opencode/skills` and exact v2/root behavior require qualification |

Every resource inherits its parent installation's consumer classification. Test
relative links after relocation/aliasing, executable modes, ignored files,
resource-name collisions and resources reachable through another client's
instruction glob. Native packaging must not cause direct and packaged copies of
the same skill/resources to activate twice. A valid entrypoint cannot qualify
missing or differently interpreted supporting files.

### Agents

| Client          | Intended delivery, discovery and activation                                                    | Potential other readers and sharing intent                                                                                                 | Preservation                                                        | Current status                                                                       |
| --------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Claude Code     | Native `.claude/agents` or qualified plugin agent registration; explicit/automatic delegation  | Other clients' compatibility loaders, imports and package agent catalogs remain an inventory obligation; no cross-client intent by default | F + N if registered; preserve manual agents and tool restrictions   | I Markdown generation; owned/native compatibility P, lifecycle expansion unsequenced |
| Copilot CLI     | Native `.github/agents/*.agent.md`, qualified CLI agent/subagent behavior                      | Copilot IDE/cloud and any compatibility loader are separate consumers; none implied by CLI selection                                       | F; preserve model/tool mappings and delegation restrictions         | I generation; CLI/IDE behavior requires separate evidence                            |
| Codex CLI       | Native agent configuration/registration for the pinned release, or explicit unsupported result | Shared config roots and any plugin compatibility agent catalogs require inspection; unintended consumers blocked                           | F + N; do not convert agent tool/permission restrictions into prose | P adapter; agent management not advanced into M1                                     |
| OpenCode v2 CLI | Native agent files/config and qualified invocation/delegation for v2                           | Current `.opencode/agents` files may coexist with config/plugin-defined agents; other-client imports and duplicate IDs require controls    | F + N where configured; preserve existing agents and restrictions   | I legacy generation; v2 semantic/native qualification P                              |

### Bundles

| Client          | Intended delivery, discovery and activation                                           | Potential other readers and sharing intent                                                          | Preservation                                                  | Current status                                           |
| --------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------- |
| Claude Code     | Upskill resolves bundle graph; each child activates through its declared Claude route | Every child's reader inventory applies; bundle YAML is not native runtime registration              | B plus child F/R/N; no extra activation for bundle membership | I composition; owned mixed-state/native combination P    |
| Copilot CLI     | Upskill graph and per-child Copilot delivery; no native bundle loader assumed         | Child compatibility/IDE exposure remains explicit, not made intentional by common bundle membership | B plus child contracts                                        | I composition; corrected-route evidence P                |
| Codex CLI       | Upskill graph with Codex plugin/native child routes                                   | Child `.agents`/package/shared-path readers; do not reinterpret bundle as a Codex plugin            | B plus child contracts                                        | Generic graph foundations I; Codex integrated delivery P |
| OpenCode v2 CLI | Upskill graph with OpenCode plugin/native child routes                                | Shared skills/config imports and child package compatibility; explicit child-level sharing only     | B plus child contracts                                        | I legacy composition; v2 corrected-route evidence P      |

### Plugins

| Client          | Intended delivery, discovery and activation                                                                          | Potential other readers and sharing intent                                                                                           | Preservation                                                | Current status                                                                     |
| --------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Claude Code     | Native package/marketplace installation, enablement and trust; qualify each exported skill/agent/hook/MCP separately | Other clients may accept compatible manifests or scan exported content; cross-client package activation unintended without selection | N + B; preserve manual marketplace/plugin entries           | I install/remove shellouts; thin upskill integration and exact native lifecycle P  |
| Copilot CLI     | Native plugin package/marketplace lifecycle with qualified exports                                                   | Compatibility packages, shared catalogs and IDE runtimes are separate potential readers; do not infer interoperability               | N + B; preserve existing plugins and trust                  | I shellouts; thin integration qualification P                                      |
| Codex CLI       | Pinned native plugin installation/enablement plus independently qualified rule integration                           | Package compatibility and exported `.agents`/other discovery paths can expose payload elsewhere; no shared activation by default     | N + B; no personal-config replacement or auth copying       | P upskill integration; installed client plugin support alone does not implement it |
| OpenCode v2 CLI | Qualified native plugin loader/API and lifecycle; package registration distinct from rule injection                  | Shared config imports, auto-loaded plugin directories and exported skill/agent catalogs can duplicate activation                     | N + B; preserve plugin array order/entries and manual setup | I legacy shellouts; v2 invocation, registration and upskill integration P          |

Plugin update transport, upskill CLI binary updates and installed-content
reconciliation remain three separate lifecycles. Namespace or manifest names
alone cannot prove activation isolation. A package that exports MCP/tools has
the corresponding additional activation and permission checks.

### MCP servers

| Client          | Intended delivery, discovery and activation                                                                    | Potential other readers and sharing intent                                                                                                          | Preservation                                                                           | Current status                                                           |
| --------------- | -------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Claude Code     | Existing native CLI/config registration; distinguish configured, enabled, trusted, authenticated and connected | Shared/imported config or plugin exports may register the same server in another runtime; explicit server-sharing intent required                   | N + B; retain manual server keys, env/secret references and integration-removal policy | I CLI/config fallback; owned native-effect recovery/qualification P      |
| Copilot CLI     | Existing CLI/config registration and qualified connection/tool exposure                                        | Copilot IDE configuration is a separate surface; plugin exports or shared config can duplicate server/tool registration                             | N + B; preserve manual entries and native approvals                                    | I CLI/config integration; expanded management P                          |
| Codex CLI       | Planned native MCP adapter/config registration for pinned CLI                                                  | Config roots, plugin MCP exports and imported definitions can expose shared endpoint/tools; another client is never inferred from endpoint equality | N + B; no credential migration or grants inferred from tool prose                      | P upskill adapter; no existing Codex MCP support implied                 |
| OpenCode v2 CLI | Existing config route adapted and qualified for v2; verify actual connection and tool activation               | Config imports and plugins can duplicate registrations; other-client config sharing must be explicit                                                | N + B; preserve native restrictions and unrelated JSON entries                         | I legacy config writes; v2/native qualification and broader management P |

An intentionally shared remote endpoint does not imply shared credentials,
consent, ownership or connection lifecycle. A denied/offline server must not be
reported as connected; a config parser passing is not runtime evidence.

### Planned hooks, tools and permissions

These rows are requirements inventory, not new implementation work. Internal
rule-delivery/maintenance hooks needed by the MVP are qualified as plugin
internals; general hook management, tool mapping and permission management
retain the roadmap's later scope. Existing restrictions remain regression
obligations.

| Client          | Hooks: intended activation and exposure                                                                                         | Tools: intended mapping and exposure                                                               | Permissions: intended enforcement and exposure                                                   | Preservation/status                                                    |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| Claude Code     | Native events via qualified plugin; inspect compatibility manifests and nested hook exports; other-runtime execution unintended | Explicit native tool IDs and MCP exports; duplicate names/exports require controls                 | Native approval/sandbox/settings authority; other-runtime config import is not a grant           | N; preserve hooks, grants and deny rules; expanded management P        |
| Copilot CLI     | Native events via qualified plugin; CLI/IDE and imported hook definitions treated separately                                    | CLI-native identifiers and MCP/plugin exports; no CLI-to-IDE equivalence assumed                   | CLI controls retained; no prose substitution or widening                                         | N; preserve manual config/registration; expanded management P          |
| Codex CLI       | Pinned native event/API for internal rule route; exact lifecycle/trust unqualified until probed                                 | Native tool identifiers and explicit capability refusal; package/shared-config exports inventoried | Native execution/approval policy and root authority; plugin cannot override sandbox restrictions | N; preserve user policy; general management P                          |
| OpenCode v2 CLI | Native plugin events/API; guard reload/duplicate registration and imported config                                               | Native tool/plugin/MCP registration; same-name tools and duplicate exports controlled              | V2 native permissions and deny rules; runtime-specific translation only when equivalent          | N; preserve ordered policy and unrelated config; expanded management P |

## Acceptance and evidence gates

| ID    | Required executable example and evidence                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ----- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CD-01 | For each applicable matrix cell, record delivery, discovery, activation, complete known reader conditions, intentional consumers, F/R/N/B preservation, status and exact evidence/version/OS/scope. Missing/unresolved entries block that claim.                                                                                                                                                                                                                  |
| CD-02 | With manual team `AGENTS.md` present, install two independent Codex/OpenCode plugin rules, update/remove one and observe the other. Team bytes/modes remain identical; rule behavior loads through the installed plugin, including new/resumed sessions and compaction where required. Disable/remove the plugin as a negative: team guidance remains, managed rule guidance does not activate.                                                                   |
| CD-03 | For each intended client run an unpredictable positive control; for every unintended reader run a successful native session without that activation marker. Exercise compatibility dirs, imports, broad globs, nested roots and configured overrides separately. Each negative requires a successful same-client positive; skills/agents need native catalog and explicit-invocation controls. Failed positives/auth/timeouts are blocked, never negative passes. |
| CD-04 | Install standalone plus plugin, duplicate aliases, same-name project/user content and two bundles. Observe one logical activation, retain correct references on partial removal, and preserve manual catalogs/configuration. Use native registration/catalog/event counts, not final-response equality alone, to establish no duplication. Explicit intended sharing has matching behavior controls.                                                              |
| CD-05 | Resource link/mode and agent tool/model/delegation restrictions survive install/update/remove; an excluded client cannot activate them through a compatible catalog, imported resource or alternate agent registration.                                                                                                                                                                                                                                           |
| CD-06 | Mixed rule/skill/resource/agent/bundle/plugin/MCP state remains lossless through schema-1 backup/migration and every current installed-state command, including `remove-mcp`. Manual edits, unselected owners, unknown legacy rows and unresolved native effects reject safely or remain inspectable.                                                                                                                                                             |
| CD-07 | Whole selected-set preflight, lock contention, source/destination drift and process interruption preserve recoverability. Native effects have observed pre/post state and safe replay/compensation evidence; file-only receipts cannot substitute.                                                                                                                                                                                                                |
| CD-08 | Package registration, trust, enablement, authentication, discovery and actual use have separate outcomes; test disabled/denied/offline cases. Hook/tool/permission negative controls prove no unintended execution, exposure or grant without expanding the implementation scope.                                                                                                                                                                                 |
| CD-09 | All four CLI clients pass applicable pinned macOS arm64 and Linux x86_64 project/user cells. Existing binary platforms and supported IDE behavior retain regression obligations; CLI tests cannot qualify an IDE. No automatic single-client/project-only fallback.                                                                                                                                                                                               |
| CD-10 | Generated-file, offline harness, experimental-library, native-observed and native-qualified evidence stay separately labeled. Link sanitized raw/summary receipts and their hashes; reproducible commands alone do not substantiate historical passes. Publish unsupported cells and reload requirements.                                                                                                                                                         |

Qualification starts with a complete matrix and upstream contract refresh for
pinned releases. The existing canonical-file discovery harness qualifies only
its exact fixtures; it does not test installed independent plugin rule delivery,
all customization kinds, unintended-reader isolation or duplicate activation.
Existing observations retain their provenance but cannot pass CD-02 through
CD-09 by implication. Production activation remains blocked until applicable G6
and CD examples have evidence and exact-head review/hosted checks pass.

## Documentation sources and limits

The following official documentation was refreshed on 2026-10-19 to identify
reader risks, not to declare installed-version conformance:

- [OpenCode v2 skills](https://dev.opencode.ai/v2/docs/skills/) lists native and
  Claude/shared compatibility sources.
  [V2 instructions](https://dev.opencode.ai/v2/docs/instructions/) describes
  nested `AGENTS.md` discovery and configured instruction loading.
- [Copilot skill sources](https://docs.github.com/en/copilot/concepts/agents/about-agent-skills)
  and
  [CLI instructions/imports](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions)
  expose compatibility and explicit-reference paths across client conventions.
- [Claude skills](https://code.claude.com/docs/en/skills) and
  [plugin manifest](https://code.claude.com/docs/en/plugins-reference) describe
  standalone/package discovery and exported customization types.
- [OpenAI skill documentation](https://learn.chatgpt.com/docs/build-skills) and
  [plugin documentation](https://learn.chatgpt.com/docs/plugins) distinguish
  skill payloads from package installation. Exact Codex CLI discovery and rule
  injection capabilities still require the pinned CLI contract and native probe;
  generic plugin documentation is not proof of a supported rule event.

Compatibility paths not established by these sources remain explicit questions
for the adapter inventory, never assertions that other clients cannot read them.
