# Shared customization roots and client adapters

Status: evaluation requested on 2026-10-20 in
[#349](https://github.com/driftsys/upskill/issues/349). The candidate is
`.agents/{rules,skills,agents}` with client-specific adapters. The subsequent
request also evaluates intentional `.claude` sharing and selective exclusion of
Claude compatibility sources in Copilot and OpenCode. This document changes no
installed content or production default. Adoption remains blocked until the
four-client coexistence controls below have native evidence.

## Assessment

A common content root can reduce duplicate managed copies, provided storage,
native discovery and activation remain separate. `.agents/skills` is an
intentionally shared discovery space. `.agents/rules` and `.agents/agents` are
candidate source catalogs; their names do not establish universal native formats
or prove that existing compatibility loaders leave them inert.

Qualify candidate A first: shared portable skills once, with a self-contained
Claude adapter if direct discovery is unavailable. This is an experiment order,
not adoption. Retain the baseline of one captured source with generated
client-native outputs; a physically shared directory is optional. Rules and
agent source should stay outside scanned catalogs unless separately qualified.
One candidate uses shared portable skills once, and compiles rules and agents
through qualified adapters. It keeps client-specific variants outside all shared
catalogs. Native plugin packages can distribute or maintain a shared item
without exporting a second native entrypoint for a client that already loads it.
A plugin may contain internal resources without advertising them as a second
skill. The exact package/import mechanism still needs qualification.

Independent Codex/OpenCode rules continue through qualified plugins beside
unchanged team `AGENTS.md`. This evaluation changes neither that architecture
nor the scope of the MVP. Bundles, plugins and MCP retain their existing
contracts; general hooks, tools, permissions and agent/MCP expansion remain
planned. Existing native restrictions and integrations remain regression
requirements.

## What current documentation establishes

The following official sources were inspected on 2026-10-20. They describe
candidate behavior, not conformance of the campaign's frozen executables.

| Client      | Documented skill sources relevant to the candidate                                                                                    | Adapter consequence to qualify                                                                                                 |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Codex       | Repository and user `.agents/skills`; nested/ancestor discovery; same-name skills may both appear                                     | Prefer one shared native entrypoint; do not also export it from the Codex plugin                                               |
| Copilot CLI | Project `.agents/skills`, `.github/skills`, `.claude/skills`; personal `.agents/skills` and `.copilot/skills`                         | Shared and compatibility copies require catalog/invocation counts, not a private-directory assumption                          |
| OpenCode v2 | Shared and Claude compatibility roots, native roots and additive configured sources; precedence by path-derived ID                    | Prefer one shared entrypoint; do not confuse a shadowed definition or hidden implicit invocation with removal from discovery   |
| Claude Code | Documented project/personal `.claude/skills` and plugin skills; the inspected page does not establish direct `.agents/skills` loading | Test direct shared discovery first; otherwise qualify an adapter such as a Claude-only plugin export with its native namespace |

Sources: [Codex skills](https://learn.chatgpt.com/docs/build-skills),
[Copilot skill sources](https://docs.github.com/en/copilot/concepts/agents/about-agent-skills),
[OpenCode v2 skills](https://dev.opencode.ai/v2/docs/skills/), and
[Claude skill locations and collisions](https://code.claude.com/docs/en/skills).

Absence from Claude's documented source list is not a native negative result. A
`.claude/skills` copy is not an isolated Claude adapter: Copilot and OpenCode
also document that compatibility source. A Claude plugin export is only a
candidate until other-client negatives and native namespace/invocation behavior
pass. Native names need not be identical across clients; one logical identity
maps to one qualified native identity per client. A second alias or namespace
must not hide a duplicate installation.

## Alternative roots and compatibility exclusions

No root is selected by this comparison. Sharing is declared per customization,
client and scope; choosing a parent directory does not make every format or
component below it portable.

| Candidate                                                           | Intended discovery                                                                                                 | Adapter and isolation costs                                                                                                                                                                           |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A: `.agents` intentionally shared                                   | Portable skills shared among clients with native discovery; Claude uses one qualified adapter if required          | Keeps a neutral catalog; Claude delivery still needs qualification. Rules and agents remain source candidates with native adapters, not universal formats                                             |
| B: `.claude` intentionally shared                                   | Claude native skills plus intentional Copilot/OpenCode compatibility consumption                                   | May reduce skill copies for those three clients. Codex needs a qualified route. Claude-specific agents, scoped rules and metadata cannot be assumed portable merely because another client scans them |
| C: `.agents` shared, selected Claude compatibility sources excluded | Shared portable skills plus one Claude adapter; Copilot/OpenCode exclude the relevant Claude compatibility sources | Requires effective, version-specific exclusion controls. Broad exclusions can remove unrelated manual content; missing or overly broad controls block this profile                                    |

Candidate B changes intended readership explicitly. It does not silently convert
existing private installations into shared content or authorize consumption of
all Claude configuration. A direct Claude skill must not also be exported by its
Claude plugin. Candidate A or C must likewise avoid a shared skill plus a second
plugin export for the same client. Compare package subsets and a standalone
Claude adapter rather than assuming every skill must be plugin-owned.

Claude plugin skill paths must resolve within the plugin root. Custom skill
directories supplement the default `skills/` directory; agents paths name files
and replace the default agent scan. There is no manifest `rules` location field.
An external repository catalog therefore cannot simply be registered with a
parent-relative manifest path. A packaged copy is a candidate generated output,
not another source of truth. These are current documented manifest constraints,
not frozen-client runtime qualification.
[Claude plugin manifest reference](https://code.claude.com/docs/en/plugins-reference).

### What can be excluded

| Client/surface                          | Documented control                                                                                                                  | Limits and evidence still needed                                                                                                                                                                                   |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Copilot CLI skills                      | User `settings.json` field `ignoredSkillsLocations` excludes directories and descendants, including `~` paths                       | Not listed among supported repository settings; do not assume a committed repository setting enforces it. Qualify the frozen CLI, effective settings location, path resolution and each project/ancestor/user root |
| Copilot CLI rules, agents and commands  | The inspected references describe Claude-compatible sources, but do not establish a selective exclusion covering all of them        | A skill exclusion is not an agent/rule/command exclusion. Keep each reader unresolved until its own control passes; do not disable all instructions or skills to simulate isolation                                |
| OpenCode Claude-compatible skills       | Rules documentation describes `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS=1`                                                               | Qualify the pinned v2 executable; prove `.claude` skills absent while `.agents` and native skills still work in both scopes. Do not infer current v2 behavior from an older compatibility flag                     |
| OpenCode Claude-compatible instructions | Rules documentation also describes broader `OPENCODE_DISABLE_CLAUDE_CODE=1` and prompt-only `OPENCODE_DISABLE_CLAUDE_CODE_PROMPT=1` | These have different advertised reach. Verify actual project/global fallback and explicit import behavior separately; no conclusion about every Claude component follows                                           |
| Copilot in VS Code                      | Current settings reference lists skill/agent location maps for the Local harness, marked deprecated                                 | These are separate from Copilot CLI and other harnesses. Qualify the applicable IDE regression surface; do not transfer a Local-harness result to CLI, cloud or another harness                                    |

Sources inspected on 2026-10-20:
[Copilot CLI skill and instruction sources](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference),
[Copilot user settings and repository allowlist](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference),
[OpenCode compatibility controls](https://opencode.ai/docs/rules/),
[OpenCode v2 additive skill sources](https://dev.opencode.ai/v2/docs/skills/),
and
[VS Code settings by harness](https://code.visualstudio.com/docs/agents/reference/ai-settings).

Directory exclusion concerns automatic customization discovery, not file access
control. Explicit imports, configured paths, plugin exports and ordinary file
reads are separate routes. A client may still read repository content through
its tools under existing permissions. Do not promise confidentiality from a
discovery exclusion or weaken permissions to complete a test.

Exclusions are planned profile configuration, not installation side effects
authorized by this evaluation. Prefer the narrowest owned subtree if a native
control permits it. A user-wide flag or directory exclusion that suppresses
unrelated manual skills is a conflict, not a safe default. Preserve existing
entries, configuration syntax and unrelated ownership; removal releases only the
owned contribution and retains other references. Record configurations that
require user provisioning as prerequisites, never silently write them to the
real user profile.

### Additional CD-11 controls for the comparison

For each candidate, retain a route table for rules, skills/resources, agents,
bundles, plugins and MCP, including unchanged or planned rows. Hooks, tools and
permissions remain visible but gain no new implementation scope. Test all four
clients under their declared project/user and platform profiles before adoption.

In isolated roots, first show a Claude-compatible fixture is actually
discovered. Then apply the candidate exclusion and show it absent from the
native catalog and unavailable through its explicit native invocation. In the
same effective configuration, demonstrate the selected shared/native/plugin
positive still works. Test ancestors, nested roots, user catalogs, configured
sources, imports and duplicate direct/plugin exports separately. Classify
unsupported flags, ignored settings and missing authentication as blocked or
failed controls, never as successful absence. Test removal/restoration against
manual configuration and unrelated content. Full qualification of these
exclusion and alternative-root cases remains planned. Bounded observed and
failed controls are retained below; they do not qualify a route or adopt a
layout. Experiments change only disposable client configuration, without
changing production defaults or personal installations.

## Rules and delegated subagents

Rules and subagents need capability-aware adapters; skill discovery cannot
establish their semantics. Keep client-specific variants outside shared
catalogs. The following are current documented candidates, not pinned-runtime
passes.

| Client      | Rule delivery to qualify                                                                                                       | Subagent delivery to qualify                                                                                                                              |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code | Native `.claude/rules` with native path/nested scope; no plugin manifest rule-directory field                                  | Project/user `.claude/agents`, or a plugin only for its supported capability subset; require actual delegation                                            |
| Copilot CLI | Native modular instructions with `applyTo`; canonical instructions and recursive imports are separate readers                  | Project `.github/agents` or personal `.copilot/agents`; project `.claude/agents` compatibility is a known reader, not private Claude storage              |
| Codex       | Independent trusted plugin rule payload beside unchanged team `AGENTS.md`; startup output alone does not qualify path matching | Native project/user `.codex/agents` TOML where supported by the frozen release; accepting plugin packaging does not establish Markdown agent registration |
| OpenCode v2 | Independent plugin context route beside team guidance; separately inventory native/configured instructions                     | Native `.opencode/agents` or agent configuration; require subagent mode, child permissions and parent delegation controls                                 |

Astra's review identified these constraints, independently checked against
source and official documentation on 2026-10-20:

- Claude plugin agents ignore `hooks`, `mcpServers` and `permissionMode`.
  Plugin-wide hooks/MCP and session-wide permission settings change scope; do
  not substitute them automatically. Reject unsupported mappings or retain a
  qualified standalone route. Delegated-agent spawn restrictions also need their
  own control rather than assuming main-agent restrictions carry over.
- Codex custom agents inherit the parent's execution boundary. A read-only
  instruction is not a sandbox restriction; reject a required restriction that
  the selected native mechanism cannot enforce.
- OpenCode v2 documents ordered `permissions` and merged agent definitions.
  Existing `src/generate/opencode.rs` emits legacy singular `permission`; this
  is a version-mapping gap to qualify, not evidence that v2 restrictions work.
  Do not silently relabel legacy output as a v2 adapter.
- `src/ancillary.rs` registers `.agents/rules/**/RULE.md`. Adding matching raw
  source can activate it through an existing OpenCode configuration. Treat
  `.agents/rules` as an occupied historical destination until a safe mapping and
  transition are demonstrated, not as inert source storage.

Sources:
[Claude subagent scope](https://code.claude.com/docs/en/sub-agents#choose-the-subagent-scope),
[Codex subagent execution boundaries](https://learn.chatgpt.com/docs/agent-configuration/subagents#approvals-and-sandbox-controls),
[OpenCode v2 agent permissions](https://dev.opencode.ai/v2/docs/agents/#permissions),
and
[Copilot agent discovery](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#custom-agent-locations).

No blanket Claude-rule/subagent exclusion has been established. Copilot's
instruction how-to and CLI command reference differ in their listed Claude rule
sources; the latter explicitly lists `.claude/rules/**/*.md`. Keep that reader
unresolved for the pinned executable rather than treating omission as absence.
Per-file instruction disabling, skill-directory exclusion and agent shadowing
are different mechanisms. OpenCode v2 currently documents no `CLAUDE.md`
fallback and does document configured instructions; v1 compatibility flags
cannot establish v2 rule/agent isolation. Preserve the dated earlier observation
in `customization-adapter-evidence.md` and record the rolling-documentation
change separately.
[Copilot instruction guide](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions),
[CLI instruction sources](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#custom-instructions-locations),
[OpenCode v2 instructions](https://dev.opencode.ai/v2/docs/instructions/).

Before claiming a subagent mapping, launch an actual child with a known native
identity and verify its selected model, allowed operation, denied operation,
delegation limits and intended instruction/MCP inheritance. A parent response,
registered prompt or hidden catalog entry cannot establish enforced execution.
For rules, test matching and nonmatching files as the session changes, including
actual child sessions where applicable. Conditional prose permanently injected
at startup does not qualify native file scoping. Bundles add owner references;
plugins and MCP retain their separate native registrations and scope. None of
these controls adds general agent/MCP management to M1.

## Catalog and adapter boundary

| Shared catalog   | Canonical content and intended consumers                                                                                                          | Native delivery/activation to qualify                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.agents/rules`  | Rule source and supporting resources; each rule declares its intended clients and scope                                                           | Claude native rules, Copilot modular instructions, and independent Codex/OpenCode plugin payloads; raw source or resources must not load incidentally through imports/globs |
| `.agents/skills` | Only consumption-ready portable skills/resources approved for shared discovery; compatible discovery is intentional for the declared installation | Native shared loading where supported; one adapter route for a client lacking that capability; no additional direct/plugin copy visible to the same client                  |
| `.agents/agents` | Agent source/resources with explicit client applicability and native tool/model/delegation requirements                                           | Client-native files or registration only where qualified; unsupported mappings reject without replacing restrictions with prose or adding new agent management to M1        |

Project and user catalogs remain distinct native scopes. Ancestor/nested roots,
imports, globs, administrator roots, account/cloud catalogs, compatibility
sources and explicit extra paths remain in every applicable reader inventory.
Known IDE/cloud/other-client readers also remain visible with explicit intent,
status and existing regression obligations; four-client CLI qualification grants
no new support claim for those surfaces.

Publishing to the shared skill catalog authorizes its declared compatible
readers; it cannot honestly promise a private single-client installation. A
requested audience exclusion must choose a separately qualified representation
outside that catalog or fail before writes. This does not retroactively broaden
existing installation intent or adopt existing files as owned.

Shared skills must be usable without client preprocessing. Do not publish raw
client directives, divergent metadata or a weaker common permission model merely
to use one directory. If a logical skill needs client-specific variants, the
safe candidate keeps that variant family outside shared native catalogs and
selects one adapter route per client. A simultaneously visible shared base and
client variant is blocked unless a qualified native selection mechanism proves
one effective entry without changing manual/global discovery. Exact variant
storage paths are adapter decisions; this evaluation creates no new on-disk
manifest format or runtime path guarantee. Do not put variants in client-named
subdirectories under a shared catalog and rely on names or ignored-file
folklore.

## One logical item per client

Logical identity follows captured source/content lineage and the declared item,
not matching names or bytes. Scope, alias, package, adapter and bundle
references are recorded delivery/ownership relationships; they do not turn one
item into several unrelated items to evade duplicate checks. Two independent
sources with equal content are not automatically the same item or shared owners.

Before effects, the complete selected-set plan enumerates every effective native
entrypoint for each logical item and client. Select the shared, direct,
configured or plugin-export route once. Counts cover catalog availability,
explicit invocation, and rule/delegation events as appropriate. One final marker
or native precedence alone does not establish one exposure. Preserve native
scope precedence where it yields the required behavior; if project/user or
ancestor copies remain separate native entries, reject the conflicting plan
until a qualified route works. Do not flatten scopes to satisfy a count.

Existing manual copies and third-party plugin exports are conflicts or
externally satisfied objects with explicit contracts. Never delete, disable,
rewrite or adopt them to manufacture a duplicate-free result. If an immutable
package exports the same item and its required exports cannot be selected
independently, block that combination rather than removing unrelated package
functionality. Bundle membership adds reference edges, not extra runtime
registrations.

## Preservation and transition

Reuse captured source evidence, owner contracts, complete preflight, qualified
roots/locks, exact backups, and guarded file/native-effect recovery. Preserve
manual team files, comments/configuration, native restrictions, trust and
credentials, resource modes/relative links, and unrelated direct/bundle owners.
Remove shared bytes or native registration only after the final owning reference
is released; an incidental reader never becomes an owner.

Current schema-1 `.agents/rules` and `.agents/skills` outputs are historical
installations, not proof of an approved shared source catalog or ownership.
Migration must distinguish owned output, captured source and manual content;
unknown rows remain inspectable and block unsafe mutation. Do not infer a new
canonical source from an existing destination. No move, rename, symlink, import
rewrite or schema-2 write is authorized by this evaluation. Existing copy-only
and root-safety contracts stay in force.

## Coexistence gate before adoption

CD-11 applies in stages, without replacing CD-01–CD-10 or reducing the four
clients, platforms, scopes or existing IDE obligations:

1. **Feasibility before selecting an adapter:** run isolated native discovery,
   scope, capability, resource and duplicate controls. A bounded fixture is
   sufficient to compare mechanisms; the product executor is not required yet.
2. **Gate 6 after implementing the selected M1 adapters:** qualify independent
   rule delivery and its integrated lifecycle, schema compatibility and
   preservation of existing installed skills/agents/MCP/configuration. Keep
   unadopted shared-layout candidates out of production effects.
3. **Shared-layout adoption:** require integrated install/update/removal,
   ownership and interruption/recovery controls for every newly adopted route.
   New owned-skill lifecycle implementation follows Gate 6 under the roadmap;
   its complete evidence blocks adopting that skill layout and the combined MVP
   release, not selection of an experimental adapter or completion of the
   preceding rule-lifecycle milestone. Existing-kind preservation and native
   regression evidence still block Gate 6 where applicable.

For every applicable frozen client/platform/scope profile, retain native
catalog, actual invocation/delegation and event receipts for:

1. One portable shared skill with supporting resources, discovered intentionally
   by all four clients through their selected routes, with one effective item
   and correct relative resource access per client.
2. Direct-plus-plugin, two bundles, aliases, same-name independent sources,
   project/user and ancestor/nested overlaps. Each logical item remains exposed
   once without changing native scoping or manual discovery.
3. Client-specific variants outside shared catalogs, with unintended-reader
   negatives and no duplicate shared base or alternate compatibility copy.
4. Rule source/resource globs and agent catalogs, including unsupported native
   capabilities, path matching, tool/model/delegation restrictions and native
   permission-denial controls. No unapproved agent/tool/hook/MCP feature is
   added simply to complete a matrix row.
5. Install, update, partial removal and final-reference removal with manual
   content/configuration, unrelated plugins/owners and native trust preserved;
   held locks, interrupted/reopened state and ambiguous native effects reject or
   recover under the existing contracts.

Each unintended-reader negative has a successful same-client positive. A failed
login, timeout or missing native catalog is blocked, not a negative pass.
Include configured compatibility/import/glob cases and existing IDE regressions.
macOS evidence does not qualify Linux; project results do not qualify user
roots. Generated-file tests and the earlier independent plugin-rule observations
are reusable background evidence, not shared-catalog coexistence qualification.
All corresponding cells remain unqualified until their controls pass. Review
feasibility before selecting an adapter; review integrated evidence before
adopting its route or enabling its production effects. Missing native evidence
never becomes a pass because a later milestone owns implementation. Production
schema-2 activation remains blocked by all applicable G6/CD requirements.

## Bounded native follow-up retained on 2026-10-21

The [shared-skill evidence note](gate6-shared-skill-controls.md) records Copilot
1.0.98 project/macOS invocation and relative-resource observations. A narrow
user exclusion, including native write/readback confirmation, did not prevent
explicit native activation of the selected Claude-compatible skill. That
candidate condition remains blocked. Surviving shared/manual positives do not
turn the failed isolation control into a pass or establish automatic selection.
All four-client/platform/scope requirements remain; no layout or route is
adopted and all campaign cells remain unqualified.

## Bounded Claude route follow-up (#353)

The [Claude evidence packet](gate6-claude-skill-controls.md) retains nine
completed macOS project sessions at 2.1.303. The desired direct `.agents/skills`
positive failed with a completed absent fallback beside a working native skill;
it is not a passing negative. A self-contained Claude plugin exposed the
namespaced skill. Development loading and normal project marketplace
installation are separate controls. Persisted disable/enable and uninstall
produced the expected absent/restored/absent results. Manual team/shared-source
bytes and modes survived; the manual settings value/mode survived, but native
removal left empty configuration objects. Do not claim byte restoration.

This advances candidate A feasibility without selecting it. No complete catalog
or duplicate/unintended-reader control, resource behavior, rule injection,
subagent restriction, user/Linux or integrated ownership/recovery qualification
follows. All alternatives, four-client obligations and production activation
blockers remain. Every campaign cell stays unqualified.

## Bounded coexistence and duplicate follow-up (#356)

The [next evidence packet](gate6-skill-coexistence-controls.md) records a common
Claude/Copilot project: one advertised Claude plugin export and one Copilot
shared entry, successful explicit positives, an unavailable Claude namespace in
Copilot, preserved manual compatibility consumption, Copilot survival under
Claude disable and a restored Claude positive. Failed bare-name and exact
fallback expectations remain failed; neither proves direct shared discovery.
Copilot shadowed a same-name compatible copy but exposed and invoked a renamed
alias alongside the shared item. Logical lineage and complete native readers
remain planning requirements. A private helper's post-capture fixture-mode error
and guarded restoration are explicit; no uninterrupted preservation or recovery
claim follows. All four-client/scope/platform/IDE and integrated gates remain,
all 160 cells stay unqualified and no layout or adapter is selected.
