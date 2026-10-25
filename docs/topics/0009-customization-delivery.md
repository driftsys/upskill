# Customization delivery, discovery and activation

Status: intended architecture and acceptance contract, reconciled on 2026-10-19
and extended with shared-catalog evaluation requirements on 2026-10-20. The
2026-10-24 native-first rules decision and implementation-order amendment are
reconciled with retained slice-1 evidence on 2026-10-25 for user adoption review
under
[AD-0040](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md).
This contract does not describe newly implemented adapters or qualify a client.
The [roadmap](https://github.com/driftsys/upskill/blob/main/docs/roadmap.md)
owns release order;
[AD-0040](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md)
resolves conflicts with earlier experimental instruction plans. A proposal dated
2026-10-23 suggests managed per-client packages for skills and subagents, and
places subagents in the MVP; its amendments are listed in
[AD-0040's proposed amendments](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md#proposed-amendments-2026-10-23)
and are not adopted here.

## Architecture and authority

Delivery places files or registers a package. Discovery makes content available
to a runtime. Activation loads instructions, invokes a skill or agent, starts a
server, registers a tool or executes a hook. These are separate outcomes.
Ownership authorizes upskill changes; it does not authorize an incidental
reader.

Always-on rules use native loading where available and a qualified trusted hook
where no native route exists. Codex and OpenCode delivery must be independent of
team `AGENTS.md`; explicit OpenCode paths are a candidate, not a mandatory
plugin route. See the [selected routes](#rules) and
[retained limits](#slice-1-evidence-and-adoption-2026-10-25). Install, update
and removal must leave team instruction bytes and modes unchanged. File
placement or packaging alone is insufficient: the selected route must deliver
guidance at the required lifecycle points with scope, precedence, exclusion and
activation evidence. CD-04 records the bounded Codex history-duplication
allowance; it does not waive planner deduplication. A skill inside a package is
not automatically an always-on rule. The exact event/API and package storage
remain adapter qualification decisions. Missing capabilities block that route;
they never authorize an `AGENTS.md` rewrite fallback.

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

## Shared content root evaluation

Evaluate `.agents/{rules,skills,agents}` with client-specific adapters under
[#349](https://github.com/driftsys/upskill/issues/349). This is a candidate
content layout, not an adopted production default. Compare intentional `.claude`
sharing and selective Copilot/OpenCode Claude-compatibility exclusions as
alternative profiles before selecting a root. A skill-directory exclusion does
not establish exclusion of rules, agents, commands or explicit imports. The
repository research note `docs/technotes/shared-content-root-evaluation.md`
records current source evidence and route candidates. CD-11 below defines the
coexistence gate.

Treat `.agents/skills` as intentionally shared discovery space for a declared
shared-catalog installation. Prefer one shared native entrypoint per logical
skill and client; a client lacking qualified direct discovery needs one adapter
route, not an additional private-looking compatibility copy visible to other
clients. Client-specific variants stay outside all shared catalogs. A variant
and shared base may not both remain exposed to the same client. Native source
namespaces, aliases and package exports do not redefine logical identity to hide
duplicates.

`.agents/rules` and `.agents/agents` are source-catalog candidates, not
universal native formats or proven inert storage. Each adapter retains qualified
native scoping and restrictions. Codex/OpenCode rules still require independent
delivery beside unchanged team `AGENTS.md`; the native-first amendment permits
explicit OpenCode instruction paths after Q5 PASS. Shared skills must be
directly consumption-ready; raw client directives or silently weakened
permission semantics cannot be placed there. An audience exclusion chooses
another qualified representation or fails preflight rather than silently
publishing to shared discovery.

Shared authoring does not require shared native discovery. A common procedure
can remain in the canonical body with generation-time client directives for
small orchestration differences, or in copied resources explicitly read by
client-specific wrappers for larger differences. Whole-body overrides replace
the canonical body and do not compose with it. Their specified semantics are not
implemented by the historical installer or captured compiler; see
[format §2.3](https://github.com/driftsys/upskill/blob/main/docs/topics/0001-content-format.md#23-per-client-override-files)
and AD-0025. Resource copies do not create additional skill entrypoints. Qualify
the native read, resource path and permission behavior separately; references do
not register agents or grant tools/MCP access. An embedded composition/include
feature and override execution are separate work, outside this Gate 6 change.

This intent does not adopt existing schema-1 outputs or manual content, add
owners from compatibility readers, or authorize moving installed files. Select
one effective native route per logical item/client before effects and preserve
manual/third-party entries and surviving direct/bundle references. Require
per-client retained PASS before implementing a candidate skill adapter and
integrated qualification before adopting its route. The user's
skills-before-rules implementation order supersedes the former
all-four-feasibility-first and rules-before-new-owned-skills implementation
dependency only. Gate 6 still requires selected M1 rules and preservation of
existing kinds; the new skill layout and combined four-client MVP release remain
blocked until qualified
([AD-0040 amendment](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md#native-first-rules-and-implementation-order-2026-10-24),
[CD-11](#acceptance-and-evidence-gates)). An unadopted layout has no production
effects. All applicable platform/scope and existing regression obligations still
apply to schema-2 activation. Generated files and earlier independent-rule
probes do not satisfy shared-layout qualification.

## Reader and activation policy

Every adapter/profile must declare the client/runtime/surface, exact version,
OS, project or user scope, effective roots and configuration. Its record
includes all entrypoints, resources, package payloads, native registrations,
imports, recursive discovery, compatibility directories, configured globs,
ancestor and nested roots, shared paths and relevant external-home overrides.
Inventory all four clients, plus known supported IDE/compatibility surfaces.
Unknown behavior is a blocker for an isolation claim, not an empty reader set.

For each possible reader record `intended`, `unintended` or `unresolved`, with
its discovery condition. Declared shared-catalog consumption under the policy
above is intentional; other cross-client consumption defaults to unintended
unless explicitly selected and accepted for that installation/profile.
Audience/body compatibility, equal bytes, a directory name or a warning alone
never supplies that consent. A requested audience exclusion must prevent
activation, not just omit one generated copy. Incidental readers gain neither
owners nor references.

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
separate events. CD-04 separately records the selected Codex recorded-history
limitation without claiming provider-effective duplication or waiving planner
deduplication. Client-specific precedence or deduplication is evidence to
verify, not a portable assumption ([resume-only][D-resume], [AD-0040][D-AD]).

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

The following are intended routes, not a change to schema-1 production output.
Each evidence link qualifies only its recorded case. The user selected these
always-on routes on 2026-10-24; CD-01 through CD-11 still gate implementation
([AD-0040](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md#native-first-rules-and-implementation-order-2026-10-24)).

| Client          | Intended always-on delivery                                                                                         | Readers and scope                                                                                                               | Preservation and current status                                                                                                                                                                                                                      |
| --------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code     | Native `.claude/rules/<name>.md`, no frontmatter                                                                    | With Copilot selected, both are declared per item and there is one physical file; other CLI/IDE/cloud readers remain unresolved | R; historical generation I. Default native case Q at 2.1.292; independent native positive PASS at 2.1.308 in Q4; complete adapter P ([native control][D-native], [Q4][D-Q4], [Q1][D-Q1])                                                             |
| Copilot CLI     | Same Claude file when Claude is selected; otherwise `.github/instructions/<name>.instructions.md` without `applyTo` | Never also write the Copilot modular copy for the shared rule. IDE/cloud/review readers need their own declaration              | R; Q1 at 1.0.103 reads unscoped Claude rules. Its positive uses the canonical Copilot file, not modular no-`applyTo` output; modular-route qualification P. Plugin rules component FAIL at both scopes ([Q1][D-Q1], [Q3][D-Q3])                      |
| Codex CLI       | Trusted package `SessionStart` hook reads a generated static rule index; no runtime upskill invocation              | Preserve native project/user guard and trust. Other package/readers remain unintended unless declared                           | R + N; selected policy supported with declared limitation. O at 0.164.0: startup 1, resumed cumulative history 2, appended 1 per rule, no compaction; provider input and full adapter P ([Q9][D-Q9], [resume-only][D-resume])                        |
| OpenCode v2 CLI | Explicit `instructions` file paths, no glob; separately qualified context-hook fallback                             | All configured imports/globs and compatibility readers inventoried before effects                                               | R + N; legacy glob output I. Q5 positive FAIL at 2.1.4, downstream NOT RUN; explicit paths and current-pin fallback P. Q5 deferred, not evidence of absence ([Q5 retries][D-Q5-retry], [issue #391](https://github.com/driftsys/upskill/issues/391)) |

One physical delivery per rule/project and per-item intended-reader ownership
are required planner behavior, not a new implemented lockfile field. Existing
schema-1 output does not yet implement that ownership record. Claude-only rules
visible to Copilot require a compatible intended-reader declaration or are
unsupported for that project; warnings do not supply consent. Claude IDE,
Copilot cloud agent, code review and VS Code are unresolved readers, not
intended consumers inferred from CLI selection ([Q1][D-Q1], [AD-0040][D-AD]).

Scoped rules are a secondary native capability policy: Claude `paths:`, and
Copilot `applyTo` only where Claude is not selected. Matching/nonmatching native
controls remain required; Q1 did not access a matching file. Shared scoped
Claude+Copilot rules and other unenforced scopes are reported unsupported, never
widened to always-on. Q1b matching-file access and Q3b Copilot startup-hook
injection are deferred, unscheduled. Q4 is unscoped-only and does not prove
fresh resume attachment or provider-effective uniqueness ([Q1][D-Q1],
[Q3][D-Q3], [Q4][D-Q4], [handoff][D-spec]).

### Skills and supporting resources

| Client          | Intended delivery, discovery and activation                                                                            | Potential other readers and sharing intent                                                                                                                                                                                      | Preservation                                                                 | Current status                                                                                                                                 |
| --------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code     | Native skill entrypoint and adjacent resources, standalone or plugin-packaged; qualified explicit/automatic invocation | `.claude/skills` is a Copilot/OpenCode compatibility source; shared-catalog consumers are intentional, but a Claude adapter must not add a second visible copy; other imports/readers retain explicit intent                    | F + N for packaging; resources never auto-executed on install                | I `.claude/skills`; O Q6 at 2.1.308, separate project/user scopes and fresh-session combined edit visibility; owned/coexistence P ([Q6][D-Q6]) |
| Copilot CLI     | Native skill discovery/invocation with resources; package location chosen by scope/profile                             | `.github/skills` can serve Copilot IDE/cloud surfaces; `.agents/skills` is intentionally shared for the evaluated catalog; `.claude/skills` and direct/plugin copies can duplicate one logical item; qualify one selected route | F; preserve other catalogs and manual skills                                 | I `.github/skills`; user-root mapping, owned lifecycle and native matrix P                                                                     |
| Codex CLI       | Qualified native skill discovery from profile-declared roots or plugin catalog, with resources                         | `.agents/skills` is intentionally shared for the evaluated catalog; plugin or extra-root exports must not expose the same logical item again; other readers retain explicit intent                                              | F + N if plugin; preserve resource paths/modes and independent manual config | O Q8 at 0.164.0: namespaced cache skill/resource reads and remove/re-add refresh; integrated adapter P ([Q8][D-Q8])                            |
| OpenCode v2 CLI | Qualified native skill source or plugin-provided catalog, resources loaded only when needed                            | `.agents/skills` is intentionally shared for the evaluated catalog; `.claude/skills`, configured sources and plugins can add duplicates; other-reader intent remains explicit                                                   | F + N if plugin; no claim that current shared location is private            | I legacy `.agents/skills`; client-specific candidate `.opencode/skills` and exact v2/root behavior require qualification                       |

Every resource inherits its parent installation's consumer classification. Test
relative links after relocation/aliasing, executable modes, ignored files,
resource-name collisions and resources reachable through another client's
instruction glob. Native packaging must not cause direct and packaged copies of
the same skill/resources to activate twice. A valid entrypoint cannot qualify
missing or differently interpreted supporting files.

### Agents

Require real delegated execution with model, tool, permission, instruction and
MCP inheritance controls. A registered prompt or parent response is
insufficient. Claude plugin agents ignore `hooks`, `mcpServers` and
`permissionMode`; do not replace these with broader plugin/session settings.
Reject unsupported mappings. Codex instructions do not establish a child sandbox
override. OpenCode v2's ordered permission/agent-merging contract needs a
qualified adapter; existing legacy generated output is not proof. Copilot
project `.claude/agents` is a known compatibility reader with no established
selective directory exclusion. Keep client variants outside shared catalogs and
preserve manual registrations. These are capability/regression requirements, not
new agent management in M1.

| Client          | Intended delivery, discovery and activation                                                    | Potential other readers and sharing intent                                                                                                 | Preservation                                                        | Current status                                                                                                                                       |
| --------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code     | Native `.claude/agents` or qualified plugin agent registration; explicit/automatic delegation  | Other clients' compatibility loaders, imports and package agent catalogs remain an inventory obligation; no cross-client intent by default | F + N if registered; preserve manual agents and tool restrictions   | I Markdown generation; owned/native compatibility P, lifecycle expansion unsequenced                                                                 |
| Copilot CLI     | Native `.github/agents/*.agent.md`, qualified CLI agent/subagent behavior                      | Copilot IDE/cloud and any compatibility loader are separate consumers; none implied by CLI selection                                       | F; preserve model/tool mappings and delegation restrictions         | I generation; CLI/IDE behavior requires separate evidence                                                                                            |
| Codex CLI       | Native agent configuration/registration for the pinned release, or explicit unsupported result | Shared config roots and any plugin compatibility agent catalogs require inspection; unintended consumers blocked                           | F + N; do not convert agent tool/permission restrictions into prose | O Q7 at 0.164.0: explicit package TOML registration, real child and inherited read-only denial; plugin-only role absent; full adapter P ([Q7][D-Q7]) |
| OpenCode v2 CLI | Native agent files/config and qualified invocation/delegation for v2                           | Current `.opencode/agents` files may coexist with config/plugin-defined agents; other-client imports and duplicate IDs require controls    | F + N where configured; preserve existing agents and restrictions   | I legacy generation; v2 semantic/native qualification P                                                                                              |

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

| Client          | Hooks: intended activation and exposure                                                                                              | Tools: intended mapping and exposure                                                               | Permissions: intended enforcement and exposure                                                   | Preservation/status                                                    |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| Claude Code     | Native events via qualified plugin; inspect compatibility manifests and nested hook exports; other-runtime execution unintended      | Explicit native tool IDs and MCP exports; duplicate names/exports require controls                 | Native approval/sandbox/settings authority; other-runtime config import is not a grant           | N; preserve hooks, grants and deny rules; expanded management P        |
| Copilot CLI     | Native events via qualified plugin; CLI/IDE and imported hook definitions treated separately                                         | CLI-native identifiers and MCP/plugin exports; no CLI-to-IDE equivalence assumed                   | CLI controls retained; no prose substitution or widening                                         | N; preserve manual config/registration; expanded management P          |
| Codex CLI       | Trusted static-index internal rule hook; Q9 observes bounded continuity at 0.164.0, complete lifecycle/re-trust still P ([Q9][D-Q9]) | Native tool identifiers and explicit capability refusal; package/shared-config exports inventoried | Native execution/approval policy and root authority; plugin cannot override sandbox restrictions | N; preserve user policy; general management P                          |
| OpenCode v2 CLI | Native plugin events/API; guard reload/duplicate registration and imported config                                                    | Native tool/plugin/MCP registration; same-name tools and duplicate exports controlled              | V2 native permissions and deny rules; runtime-specific translation only when equivalent          | N; preserve ordered policy and unrelated config; expanded management P |

## Slice-1 evidence and adoption (2026-10-25)

This is the separate step-7 reconciliation for
[task #412](https://github.com/driftsys/upskill/issues/412). The user owns
adoption at its PR review. Native-first always-on rules and skills-before-rules
implementation order were already decided on 2026-10-24; the additional
direction choices below are not implied by a PASS. Merged packets and indexed
technotes complete a control administratively, including FAIL results, without
completing Gate 6 or adopting a production adapter ([handoff][D-spec],
[AD-0040][D-AD]).

### Retained control dispositions

All rows below are macOS arm64 observations. Pins are per row, not an implied
current four-client support range. Retained `qualified: false` stays false;
Linux, IDE/hosted, arbitrary user-root and integrated ownership/recovery
evidence are not supplied by these packets
([CD-09 and CD-10](#acceptance-and-evidence-gates)).

| Control | Pin             | Retained result and limit                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1      | Copilot 1.0.103 | PASS: reads an unscoped `.claude/rules` file. The `paths:` variant returned fallback without matching-file access; no scope/exclusion conclusion. Other three pre-run client pin receipts are missing in this early packet ([note][D-Q1], [summary][D-Q1-summary])                                                                                                                                                                                |
| Q2      | Copilot 1.0.103 | PASS for Claude-format agent discovery, real delegation and resource read; tool catalog, `permissionMode` and equivalent restriction enforcement remain unqualified ([note][D-Q2], [summary][D-Q2-summary])                                                                                                                                                                                                                                       |
| Q3      | Copilot 1.0.103 | FAIL: plugin rule positives at both scopes. Bounded user-child delegation observed, exact response contract fails; project-overlay activation unverified. Rules component retired as a candidate at this pin; not a proof that all plugin components are absent ([note][D-Q3], [summary][D-Q3-summary])                                                                                                                                           |
| Q4      | Claude 2.1.308  | PASS: unscoped startup/post-compaction observations and absence controls. Resume callback/response do not prove a fresh attachment; scope and effective-input uniqueness remain unqualified ([note][D-Q4], [summary][D-Q4-summary])                                                                                                                                                                                                               |
| Q5      | OpenCode 2.1.4  | FAIL: native positives; component controls NOT RUN. Explicit-path retries retain backend-unavailable and provider-limit failures. User deferred further retries; no absence or route-failure inference ([initial][D-Q5], [latest retry][D-Q5-retry], [summary][D-Q5-summary])                                                                                                                                                                     |
| Q6      | Claude 2.1.308  | PASS: local marketplace, namespaced skill and combined content/version edit visibility in fresh sessions at separate project/user scopes. Earlier auth failures and settings/cache residue retained; overlap and running-session edits untested ([note][D-Q6], [summary][D-Q6-summary])                                                                                                                                                           |
| Q7      | Codex 0.164.0   | PASS: explicit package-owned TOML registration, real child/resource read, inherited read-only native denial and calibrated plugin-only catalog absence. Initial restoration failure and final exact correction both retained; full agent adapter unqualified ([note][D-Q7], [summary][D-Q7-summary])                                                                                                                                              |
| Q8      | Codex 0.164.0   | PASS: namespaced skill/resource activation and native remove/re-add refresh. Resource+version edits stay cached before refresh; skill-body-only updates and unrelated-project isolation untested ([note][D-Q8], [summary][D-Q8-summary])                                                                                                                                                                                                          |
| Q9      | Codex 0.164.0   | Original zero-duplicate result FAIL: counts 1/2/1 across startup and combined reopen/compaction windows. Resume-only attribution PASS: startup 1, resumed cumulative history 2, appended records 1 per rule, zero compaction. Earlier HTTP 401 positive failure remains retained. Provider input and duplicate lifetime are unqualified ([combined][D-Q9], [failed retry][D-resume-failed], [attribution][D-resume], [summary][D-resume-summary]) |

The Codex selected support policy is supported with declared limitation. The
resume-only result attributes the extra recorded copy before compaction; it does
not establish whether the provider receives both copies or when that duplicate
ceases to affect input. The older combined run's second compaction records one
body per rule in that separate window. Do not merge those observations into a
new provider-lifetime claim, rewrite the historical FAIL, remove native history
or implement a transcript-dedup workaround ([Q9][D-Q9],
[resume-only][D-resume]).

### Update visibility and preservation review

These observations bound the implementation contract; they do not grant
permission to leave unrelated configuration changed. Exact restoration of
measured files and declared native effects are separate from whole-profile
equality. Native cache directories are never directly edited or reclaimed by
upskill ([Q6][D-Q6], [Q8][D-Q8], [CD-07](#acceptance-and-evidence-gates)).

| Client and pin  | Update visibility                                                                                                          | Measured restoration and native residue                                                                                                                                                                                                                              |
| --------------- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude 2.1.308  | Fresh sessions see combined content/version edits without reinstall; running-session/content-only behavior untested        | Native removal leaves empty `enabledPlugins`/`extraKnownMarketplaces`, then measured settings restored exactly. Native cache remains, including later-observed `0.0.1`; registry files match. Session/log/cache state is outside whole-profile equality ([Q6][D-Q6]) |
| Copilot 1.0.103 | Managed-source catalog/load paths observed; do not infer a complete passing content-update loop from failed rule positives | Empty owned `installedPlugins` residue after native uninstall; measured configuration restored. Session/log/cache/trust effects excluded ([Q3][D-Q3])                                                                                                                |
| Codex 0.164.0   | Q8 resource+version source edit stays invisible until native remove/re-add; other refresh verbs not qualified              | Q8 restores its own config baseline; Q7 separately corrects a notice counter to restore its earlier pre-UI hash. Both initial failure and correction retained. Native logs/cache bookkeeping excluded ([Q7][D-Q7], [Q8][D-Q8])                                       |
| OpenCode 2.1.4  | Unobserved after failed positives; Q5 deferred                                                                             | Measured config/manual/rule bytes and modes unchanged in retry. No successful install/update/remove lifecycle or whole-profile claim ([Q5 retry][D-Q5-retry])                                                                                                        |

Decision required: the user approves each client's owned-key/byte preservation
allowances before implementation. Recovery must use the journal plus observed
native registration state; no successful file operation proves deactivation
([CD-07](#acceptance-and-evidence-gates), [handoff][D-spec]).

### Amendment inventory

This inventory reconciles every record named by the proposal. Shipped output is
kept distinct from intended route changes; no path table below is a declaration
that code has changed ([proposal inventory][D-inventory]).

| Record                                                                                                                                                                                                                                                                                                                          | Disposition in this adoption review                                                                                                                                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [AD-0040][D-AD] and this topic                                                                                                                                                                                                                                                                                                  | Record the decided native-first rules, shared readers, Codex limitation and implementation order. Package/MVP alternatives remain explicit pending decisions                                                          |
| [Roadmap](https://github.com/driftsys/upskill/blob/main/docs/roadmap.md)                                                                                                                                                                                                                                                        | Skills-before-rules implementation order recorded; updater remains after both lifecycles. Four-client release scope and qualification remain. Bundles/subagent release changes are not adopted                        |
| [ADR-0003](https://github.com/driftsys/upskill/blob/main/docs/adr/0003-generation-pipeline.md)                                                                                                                                                                                                                                  | Keep shipped paths/ancillary behavior, add intended native rule/package ownership transition and step-8 gate. Managed package root and declared-reader persistence still need implementation design                   |
| [Specification section 3.4](https://github.com/driftsys/upskill/blob/main/docs/specification/specification.md#34-ancillary-file-handling)                                                                                                                                                                                       | Keep current bridge/glob writes documented. Removal and future doctor reporting wait for step 8; existing files are never automatically deleted                                                                       |
| [AD-0016](https://github.com/driftsys/upskill/blob/main/docs/adr/0016-pure-instruction-planning.md), [AD-0017](https://github.com/driftsys/upskill/blob/main/docs/adr/0017-shared-instruction-owner-evidence.md), [AD-0018](https://github.com/driftsys/upskill/blob/main/docs/adr/0018-retain-empty-instruction-aggregates.md) | Explicitly bound aggregate contributions to the experimental library; planner/owner algorithms remain reusable                                                                                                        |
| [AD-0039](https://github.com/driftsys/upskill/blob/main/docs/adr/0039-compile-captured-codex-instructions.md)                                                                                                                                                                                                                   | Existing experimental-only clarification already applies; no algorithm amendment or new production destination                                                                                                        |
| [Topic 0001 section 7](https://github.com/driftsys/upskill/blob/main/docs/topics/0001-content-format.md#7-generation-client-specific-output) and [format-spec section 7](https://github.com/driftsys/upskill/blob/main/docs/specification/format-spec.md#7-generation-client-specific-output)                                   | Update the canonical concern reference's current-versus-intended clarification; the compatibility page already forwards there. Preserve shipped output tables until corresponding code changes                        |
| [ADR-0012](https://github.com/driftsys/upskill/blob/main/docs/adr/0012-consumer-client-filtering.md) and VS Code ancillary output                                                                                                                                                                                               | Preserve current selection/defaults and write. CLI reader consent does not qualify VS Code. Its profile/write disposition is deferred, not silently removed                                                           |
| [2026-10-14 route mapping](https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-10-14-customization-adapter-roadmap.md#initial-route-mapping) and [adapter evidence](https://github.com/driftsys/upskill/blob/main/docs/technotes/customization-adapter-evidence.md)                                               | Reconcile the mechanism labels and add a dated native-evidence supplement; preserve historical research and later-client scope                                                                                        |
| [ADR-0014](https://github.com/driftsys/upskill/blob/main/docs/adr/0014-versioned-agent-model-mappings.md)                                                                                                                                                                                                                       | No change: accepted client-specific offline mappings remain; no new runtime resolver or committed test model identifier                                                                                               |
| [Storage candidates registration](https://github.com/driftsys/upskill/blob/main/docs/wip/README.md#retained-context-not-active-proposals)                                                                                                                                                                                       | Retained local comparison context only. User-reported OpenCode cache performance is unqualified; no source bytes copied from the original dirty worktree                                                              |
| [#379](https://github.com/driftsys/upskill/issues/379)                                                                                                                                                                                                                                                                          | Resume-only attribution merged in #411. Re-scope to K1 debt on user adoption; do not close it as a zero-duplicate PASS or silently change current labels before review                                                |
| Proposal/implementation WIP and technote backlinks                                                                                                                                                                                                                                                                              | Keep active while the following direction choices and later implementation remain unresolved. Backlinks still name a live proposal; archive/repoint only when that proposal is fully adopted, preserving packet bytes |

### Decisions and entry controls still required

Decision required: the user separately accepts or rejects each direction change
at this PR review. Until then, baseline scope remains in force
([AD-0040 proposed amendments][D-pending],
[roadmap](https://github.com/driftsys/upskill/blob/main/docs/roadmap.md#mvp--next-release)):

- Decision required: make self-contained per-client skill/subagent packages the
  default instead of the shared-root-first experiment. Q6/Q8 establish bounded
  feasibility, not a default selection or integrated ownership design.
- Decision required: add new owned-subagent management to the MVP. Q2/Q7 do not
  adopt release scope.
- Move authorable bundles out of the MVP, preserving existing bundle
  composition.
- Decision required: select explicit OpenCode package skill sources instead of
  the native-directory candidate. Q5 has not run that control, so this is a
  design decision only.

Decision required: select the Codex hook-bearing package distribution channel.
The local marketplace fixtures establish no public-directory distribution or
automatic trust. Ordinary native hook review is mandatory
([design ledger](https://github.com/driftsys/upskill/pull/385#issuecomment-6384387650),
[Q9][D-Q9]).

The Codex **index-only trust control** remains unscheduled: start with a trusted
hook/index positive, change only the static index while retaining exact hook
bytes, compare native trust state, complete any ordinary user re-review, then
verify activation. Capture version, exact changed bytes and native trust/outcome
receipts. Failed positive blocks the result; no trust bypass or credential copy.
Its result determines update UX, not a presumed exemption from hook review
([handoff][D-spec], [Q9][D-Q9]).

[Q10 #409](https://github.com/driftsys/upskill/issues/409) must pass
same-package user/project overlap with one catalog entry and one observable
activation; [Q11 #410](https://github.com/driftsys/upskill/issues/410) must pass
calibrated unintended-reader controls for each applicable package profile.
Direct-plus-package coexistence remains CD-04 work even without another numbered
control. Preflight must inventory `.claude/rules`, `.github/instructions`,
compatibility roots and imports/globs; onboarding, preservation and update
visibility need per-client controls before owned-skill implementation
([handoff][D-spec], [CD-03 through CD-08](#acceptance-and-evidence-gates)).

Q1b and Q3b remain deferred and unscheduled. Step 8 stays blocked while Q5 lacks
explicit-path PASS: no bridge/glob removal, automatic deletion or production
route activation follows from this documentation PR. Generic package-hook
loading on all four clients remains rejected context; Copilot hook injection is
untested. No native-versus-hook performance comparison has run. Criterion 7
still requires identical bytes/tasks and cached/uncached token counts, one
client at a time ([user decision][D-decision], [Q5 retry][D-Q5-retry]).

## Acceptance and evidence gates

| ID    | Required executable example and evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| CD-01 | For each applicable matrix cell, record delivery, discovery, activation, complete known reader conditions, intentional consumers, F/R/N/B preservation, status and exact evidence/version/OS/scope. Missing/unresolved entries block that claim.                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| CD-02 | With manual team `AGENTS.md` present, deliver two independent Codex/OpenCode rules through the selected native or hook route; update/remove one and observe the other. Preserve team bytes/modes and scope/trust. Test fresh/resumed sessions and compaction where applicable, then disable/remove the selected registration or file as a negative. Native files are not forced into a plugin. Q9 continuity observations do not qualify fresh registration or complete lifecycle; CD-04 records the Codex limitation ([AD-0040][D-AD], [Q9][D-Q9], [resume-only][D-resume]).                                                                                                                                |
| CD-03 | For each intended client run an unpredictable positive control; for every unintended reader run a successful native session without that activation marker. Exercise compatibility dirs, imports, broad globs, nested roots and configured overrides separately. Each negative requires a successful same-client positive; skills/agents need native catalog and explicit-invocation controls. Failed positives/auth/timeouts are blocked, never negative passes.                                                                                                                                                                                                                                            |
| CD-04 | Install standalone plus package, duplicate aliases, same-name project/user content and two bundles. Observe one logical exposure/activation and correct surviving references; retain native catalog/registration/event counts. Planner duplicate delivery remains rejected. The selected Codex 0.164.0 rule policy allows the declared recorded-history duplicate after resume; provider-effective input and duplicate lifetime are unqualified. No other client, content kind or lifecycle inherits that exception ([resume-only][D-resume], [Q10](https://github.com/driftsys/upskill/issues/409)).                                                                                                        |
| CD-05 | Resource link/mode and agent tool/model/delegation restrictions survive install/update/remove; an excluded client cannot activate them through a compatible catalog, imported resource or alternate agent registration.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| CD-06 | Mixed rule/skill/resource/agent/bundle/plugin/MCP state remains lossless through schema-1 backup/migration and every current installed-state command, including `remove-mcp`. Manual edits, unselected owners, unknown legacy rows and unresolved native effects reject safely or remain inspectable.                                                                                                                                                                                                                                                                                                                                                                                                        |
| CD-07 | Whole selected-set preflight, lock contention, source/destination drift and process interruption preserve recoverability. Native effects have observed pre/post state and safe replay/compensation evidence; file-only receipts cannot substitute.                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| CD-08 | Package registration, trust, enablement, authentication, discovery and actual use have separate outcomes; test disabled/denied/offline cases. Hook/tool/permission negative controls prove no unintended execution, exposure or grant without expanding the implementation scope.                                                                                                                                                                                                                                                                                                                                                                                                                            |
| CD-09 | All four CLI clients pass applicable pinned macOS arm64 and Linux x86_64 project/user cells. Existing binary platforms and supported IDE behavior retain regression obligations; CLI tests cannot qualify an IDE. No automatic single-client/project-only fallback.                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| CD-10 | Generated-file, offline harness, experimental-library, native-observed and native-qualified evidence stay separately labeled. Link sanitized raw/summary receipts and their hashes; reproducible commands alone do not substantiate historical passes. Publish unsupported cells and reload requirements.                                                                                                                                                                                                                                                                                                                                                                                                    |
| CD-11 | Implement skills/resources before always-on rule work under the user's explicit sequencing amendment. Require per-client retained PASS, step-7 user adoption and approved preservation contracts; start Claude with Q6, then Codex Q8, OpenCode only when Q5 skills/resources pass. Q10/Q11 are additional entry controls. This changes implementation order, not the all-four-client combined release gate or production activation. Keep one effective entrypoint per item/client, variants outside shared catalogs, integrated scope/permission/ownership/lifecycle evidence and existing-kind regressions. No unadopted route may activate ([AD-0040][D-AD], [Q6][D-Q6], [Q8][D-Q8], [handoff][D-spec]). |

Qualification starts with a complete matrix and upstream contract refresh for
pinned releases. The existing canonical-file discovery harness qualifies only
its exact fixtures; it does not test installed independent rule delivery, all
customization kinds, unintended-reader isolation or duplicate activation.
Existing observations retain their provenance but cannot pass CD-02 through
CD-09 or CD-11 by implication. Production activation remains blocked until
applicable G6 and CD examples have evidence and exact-head review/hosted checks
pass.

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

[D-native]: https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate6-native-qualification.md
[D-AD]: https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md
[D-pending]: https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md#proposed-amendments-2026-10-23
[D-spec]: https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-10-23-customization-delivery-implementation-spec.md
[D-inventory]: https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-10-23-customization-delivery-proposal.md#records-superseded-or-amended-if-this-proposal-is-adopted
[D-decision]: https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24
[D-Q1]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-copilot-claude-rules-reader.md
[D-Q1-summary]: https://github.com/driftsys/upskill/blob/main/docs/technotes/evidence/2026-10-24-copilot-claude-rules-reader/summary.json
[D-Q2]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-copilot-claude-agents-reader.md
[D-Q2-summary]: https://github.com/driftsys/upskill/blob/main/docs/technotes/evidence/2026-10-24-copilot-claude-agents-reader/summary.json
[D-Q3]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-copilot-plugin-components.md
[D-Q3-summary]: https://github.com/driftsys/upskill/blob/main/docs/technotes/evidence/2026-10-24-copilot-plugin-components/summary.json
[D-Q4]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-claude-rule-hooks.md
[D-Q4-summary]: https://github.com/driftsys/upskill/blob/main/docs/technotes/evidence/2026-10-24-claude-rule-hooks/summary.json
[D-Q5]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-opencode-current-routes.md
[D-Q5-summary]: https://github.com/driftsys/upskill/blob/main/docs/technotes/evidence/2026-10-25-opencode-explicit-paths-retry/summary.json
[D-Q6]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-claude-managed-root.md
[D-Q6-summary]: https://github.com/driftsys/upskill/blob/main/docs/technotes/evidence/2026-10-24-claude-managed-root/summary.json
[D-Q7]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-codex-agent-registration.md
[D-Q7-summary]: https://github.com/driftsys/upskill/blob/main/docs/technotes/evidence/2026-10-24-codex-agent-registration/summary.json
[D-Q8]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-codex-skill-updates.md
[D-Q8-summary]: https://github.com/driftsys/upskill/blob/main/docs/technotes/evidence/2026-10-24-codex-skill-updates/summary.json
[D-Q9]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-codex-rule-lifecycle.md
[D-Q9-summary]: https://github.com/driftsys/upskill/blob/main/docs/technotes/evidence/2026-10-24-codex-rule-lifecycle/summary.json
[D-resume]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-codex-resume-attribution.md
[D-resume-summary]: https://github.com/driftsys/upskill/blob/main/docs/technotes/evidence/2026-10-25-codex-resume-attribution/summary.json
[D-Q5-retry]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-opencode-explicit-paths-retry.md
[D-resume-failed]: https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-codex-resume-only.md
