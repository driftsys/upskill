# Proposed customization delivery architecture

Date: 2026-10-23; evidence reconciliation: 2026-10-24 ([design ledger][ledger]).
Status: proposal for user review. Not an adopted ADR, implemented feature, or
production support claim.

This revision captures the requirements discussion and replaces the earlier
separate-output recommendation in this draft. The three direction amendments
remain unadopted under [AD-0040][direction] and the [roadmap][roadmap].

Decision required: the user accepts or rejects each direction change at the step
7 PR review, separately from evidence-driven route amendments
([design ledger][ledger]).

## Controlling records

This proposal refines, and must stay consistent with, the direction accepted on
2026-10-19 in
[AD-0040](../../adr/0040-separate-customization-delivery-from-discovery.md)
("Separate customization delivery from discovery") and
[Topic 0009](../../topics/0009-customization-delivery.md), both on `main` since
PR #334 and extended on 2026-10-20 (PR #350). Where this proposal agrees with
them it says so. Where it diverges it says so and gives the reason, and the
three direction amendments are recorded in [AD-0040][direction]; Topic 0009
links to that list rather than recording or adopting the amendments itself
([Topic 0009][contract]).

Agreements with AD-0040 and Topic 0009:

- One captured source with generated client-native outputs; a physically shared
  directory is optional, not the baseline.
- Independent Codex and OpenCode rules need a qualified plugin route. Team
  `AGENTS.md` is preserved. Rewriting it is not a fallback, and missing
  capabilities block the route instead of authorizing a rewrite.
- The merged managed-`AGENTS.md` contribution compiler is an experimental
  library mechanism, not production delivery.
- Whole-body overrides replace the canonical body and do not compose with it.
- Subagents keep native client formats. Claude plugin agents lose agent-local
  `hooks`, `mcpServers`, `permissionMode` and `initialPrompt`; these must not be
  replaced by broader plugin or session settings.

Divergences from AD-0040, Topic 0009 and the [roadmap](../../roadmap.md):

- Subagents join the MVP. The roadmap leaves subagent management unassigned and
  the 2026-10-11 roadmap placed them in M3 after skills.
- Bundles as an authorable content type leave the MVP table; native plugins stay
  as the delivery mechanism. The updater plugin is not addressed by this
  proposal and keeps its roadmap place. See "Requirements and scope" for what
  this does and does not change.
- Skills and subagents are delivered as upskill-managed packages per client
  wherever a native plugin root exists; rules use a package hook only where no
  qualified native route exists. AD-0040's extended section selects portable
  `.agents/skills` with one self-contained Claude adapter as the first
  experiment; this proposal makes the self-contained package the default and
  `.agents/skills` an optional qualified profile, because Claude direct
  `.agents/skills` discovery failed and the package route passed on Claude,
  Copilot and Codex, with OpenCode catalog evidence only. AD-0040 only requires
  that delivery and discovery stay separate; it does not select packages. Those
  successes are historical evidence at their recorded pins, not current route
  qualification ([Q6][Q6], [Q3][Q3], [Q5][Q5], [CD-10][contract]). The
  2026-10-14 adapter roadmap's route mapping gives skills native discovery ("N")
  on all four clients and leaves subagents unsequenced; both change here.

## Records superseded or amended if this proposal is adopted

Adoption requires explicit amendments. This proposal does not silently amend any
of them.

| Record                                                                                                                                                                                   | Conflict with this proposal                                                                                                                                           | Required change                                                                                                      |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| [ADR-0003](../../adr/0003-generation-pipeline.md) output table and ancillary files                                                                                                       | Shipped layout writes `.claude/rules`, `.github/instructions`, `.agents/rules` plus an `instructions[]` glob in `opencode.json`, and `.opencode/agents`               | Record the managed-package layout and the disposition of each shipped path (see "Existing generated output")         |
| [Specification §3.4 ancillary file handling](../../specification/specification.md), with §2.3 `remove` leaving them untouched                                                            | `upskill` creates `CLAUDE.md` with `@AGENTS.md` when absent ([ancillary.rs](../../../src/ancillary.rs))                                                               | Remove `CLAUDE.md` creation; record what happens to an existing generated bridge file                                |
| [AD-0016](../../adr/0016-pure-instruction-planning.md), [AD-0017](../../adr/0017-shared-instruction-owner-evidence.md), [AD-0018](../../adr/0018-retain-empty-instruction-aggregates.md) | Owned contributions are written as marked regions inside shared instruction aggregates such as `AGENTS.md`                                                            | Mark as experimental library mechanisms with no production destination, as AD-0040 already does                      |
| [AD-0039](../../adr/0039-compile-captured-codex-instructions.md)                                                                                                                         | Compiles captured Codex rules into project `AGENTS.md` or global `.codex/AGENTS.md`                                                                                   | Same as above; the reader-report contract may be reused for exposure reporting                                       |
| [Roadmap](../../roadmap.md) MVP table and implementation dependencies                                                                                                                    | MVP lists automatic updates, rules, skills, bundles and plugins; subagents are unassigned                                                                             | Re-sequence explicitly; keep the three ordered dependencies or replace them                                          |
| [Topic 0001 §7](../../topics/0001-content-format.md) and [format-spec §7](../../specification/format-spec.md)                                                                            | Define the per-client output paths this proposal replaces                                                                                                             | Amend together with ADR-0003                                                                                         |
| [ADR-0012](../../adr/0012-consumer-client-filtering.md) and the `.vscode/settings.json` ancillary write                                                                                  | `chat.instructionsFilesLocations` is written for the VS Code Copilot surface                                                                                          | Record a disposition like `opencode.json` and `CLAUDE.md`; VS Code is deferred, so the write may have no replacement |
| [AD-0040](../../adr/0040-separate-customization-delivery-from-discovery.md) and [Topic 0009](../../topics/0009-customization-delivery.md)                                                | Subagents in MVP; packages as default for skills and subagents instead of `.agents/skills` first; OpenCode package skills instead of the `.opencode/skills` candidate | AD-0040 lists proposed amendments; Topic 0009 links to them without adoption; the user decides at step 7             |
| [2026-10-14 adapter roadmap](2026-10-14-customization-adapter-roadmap.md) and [adapter evidence](../../technotes/customization-adapter-evidence.md)                                      | Route mapping gives skills "N" everywhere and Claude rules "N"; adapter evidence separately lists hooks                                                               | Update the route mapping to this table                                                                               |
| [ADR-0014](../../adr/0014-versioned-agent-model-mappings.md)                                                                                                                             | None. Accepted offline S/A/B/C model mappings exist for Claude, Codex and Copilot                                                                                     | Reference it from the subagent section; this proposal adds no new resolver                                           |
| Storage candidates draft `docs/wip/specs/2026-10-23-customization-storage-delivery-candidates.md`                                                                                        | The local uncommitted draft compares candidates and records a user-reported OpenCode explicit-path/cache observation; it is retained context, not an active proposal  | Register retained context in docs/wip/README.md; Q5 does not test or validate its cache claim                        |

Decision required: at step 7 the user must separately approve (1) adding
subagents to the MVP, (2) making self-contained packages the default instead of
the shared-root experiment, and (3) moving authorable bundles out of the MVP.
The OpenCode explicit skill-source change also needs the user's disposition of
AD-0040's third listed amendment; Q5 has not executed it. None follows from a
native PASS ([AD-0040][direction], [roadmap][roadmap], [Q5][Q5]).

## Requirements and scope

| Phase | Content                                                     |
| ----- | ----------------------------------------------------------- |
| MVP   | Rules, skills and subagents, including supporting resources |
| Next  | Commands, MCP integrations and plugin bundles               |
| Later | Workflows                                                   |

"Plugin bundles" in the Next row means bundles as an authorable content type.
Native plugins as the MVP delivery mechanism are a different thing and are in
the MVP row implicitly.

The initial client surfaces are Claude Code, Codex CLI, Copilot CLI and
OpenCode. IDE and hosted surfaces require separate qualification. Native plugins
used as an internal delivery mechanism belong to the MVP; plugin bundles as an
authorable content type belong to the next phase.

Scope boundaries this table does not change:

- Bundles, plugin installation through client CLIs and MCP configuration are
  shipped behavior (ADR-0007, ADR-0008, ADR-0010). Moving them out of the MVP
  table means no new capability for them in the MVP. It does not remove, freeze
  or deprecate them; they remain part of the regression contract. A decision to
  remove any shipped behavior needs its own record.
- The updater plugin is not addressed by this proposal. The roadmap's third
  implementation dependency (shared automatic maintenance) stays where it is
  until the roadmap is re-sequenced.

The design must:

- Maintain one canonical authored source and compose shared content with client
  differences during generation. Whole-body replacement is not composition.
- Avoid duplicate authoring and duplicate activation. Generated physical copies
  are acceptable where native delivery requires them; minimize unnecessary
  copies.
- Preserve required scope, tools, permissions and execution behavior. Reject
  unsupported mappings instead of silently weakening them.
- Keep `CLAUDE.md` and `AGENTS.md`, including nested equivalents, entirely
  team-owned. Do not create, edit, append, insert imports, or add managed
  sections. Preserve their bytes and modes through lifecycle operations. This
  forbids both the shipped `CLAUDE.md` bridge and the merged managed-region
  contribution route (see the superseded-records table).
- Support user-wide and project-scoped activation, with explicit handling of
  overlap and unintended cross-client readers.
- Manage install, update, disable, removal and recovery of owned artifacts and
  external registrations together, preserving unrelated user configuration.
- Keep test model identifiers out of repository documentation.

## Recommended architecture

Use one canonical source, generation-time composition and managed packages per
client. A package is the default home for generated skills, subagents, resources
and any small client adapter. Rules use the qualified native rule route where
one exists (Claude, Copilot) and a package hook where none does (Codex,
OpenCode); see "Rules". Where supported, the package is a native plugin root.
Prefer native components over custom loading code.

Package ownership is the common abstraction; identical loading mechanisms are
not required. External registrations or native files are allowed where necessary
to preserve capabilities. The user explicitly accepted this exception for
subagents. Do not force everything into a plugin at the cost of functionality.
The same exception applies to rules where a qualified native rule route
preserves scoping that a package hook cannot (see "Rules").

`RULE.md`, `SKILL.md` and `AGENT.md` are authored inputs. Client differences
contain metadata and instructions specific to that client. Compose the effective
body before delivery where possible, instead of requiring the model to retrieve
an external common procedure. Package-local reference resources remain useful
for progressive disclosure, but are not an enforcement boundary.

This is a design contract, not a claim that a new include syntax, override
execution or model-profile resolver already exists. Exact authoring syntax needs
review against the existing compiler and format specification. Existing override
suffixes are `claude`, `copilot` and `opencode` only; a Codex suffix and any
embedded composition feature are separate, unscheduled work.

### Packaging changes identity

Moving content into a client plugin changes behavior even when bodies are
identical. Claude plugin skills and agents are namespaced by plugin, the
manifest `agents` list replaces default agent discovery, and plugin-root
`CLAUDE.md` does not load as project context. Codex plugin skills appear under
their package identity. "One active representation per logical item" must
therefore be stated in terms of the native identity the client exposes, and
user-facing invocation names (for example `/plugin:skill`) must be documented
per client. Retained evidence shows a bare-name Claude request resolving to the
namespaced plugin skill rather than a direct copy; this is a precedence
observation, not proof of exclusion.

## Storage and activation

Distinguish three locations:

1. The canonical authoring source.
2. The upskill-managed generated package, which may serve as the plugin root.
3. The client's installation/cache location, if its native installer copies the
   package rather than referencing it directly.

Do not write directly into client-owned plugin caches. Register packages through
supported mechanisms and record the resulting native identity and state. Native
configuration records can live outside packages. Arbitrary package directories
are not automatically discovered, and a client name in a path is not isolation.

Known installation mechanics that shape the lifecycle:

- Claude `--plugin-dir` loads a package for one session only; persistent
  installation goes through a marketplace entry and `claude plugin install` with
  a user, project or local scope. A committed project entry does not download
  the plugin for collaborators.
- Codex installs plugins from a marketplace into `~/.codex/plugins/cache/`, so
  the active bytes are a native copy. The retained project controls note that
  fixture user registration and cache writes "do not qualify project-only write
  authority".
- OpenCode 2.0.32 `plugin add` rejected a local directory; the route that worked
  was an explicit local package path in the owned `plugins` array.

One marketplace listing is cross-installable: Codex's parser accepts
`.claude-plugin` layouts and VS Code Copilot accepts Copilot, Claude and
OpenPlugin formats. Use one marketplace entry per client package, or prove with
a negative control that a client ignores another client's package. A same-name
manual project skill also shadows a plugin skill on Copilot (`.github/skills`
variant wins, assessment 1.0.103); preflight must detect manual-versus-package
name collisions, not only shared-versus-compatibility ones.

Update visibility is client-specific; a universal reinstall step is not
supported by the retained observations ([Q6][Q6], [Q3][Q3], [Q5][Q5]).

| Client and current pin | Update visibility                                            | Evidence boundary                                                                                      |
| ---------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| Claude 2.1.308         | Fresh session sees edited managed source without reinstall   | PASS at project and user scopes; running-session visibility not run ([Q6][Q6], [summary][Q6 summary])  |
| Copilot 1.0.103        | Native installation reports live loading from managed source | Q3 installation observation, not successful rule-update activation ([Q3][Q3], [summary][Q3 summary])   |
| Codex 0.164.0          | Unknown until Q8                                             | Native cache-copy lifecycle still needs the Q8 control ([AD-0040][direction], [design ledger][ledger]) |
| OpenCode 2.1.4         | Not run after failed positives                               | Q5 provides no execution or update visibility result ([Q5][Q5], [summary][Q5 summary])                 |

Collaborator onboarding at project scope needs an explicit task: committed
registration does not prove native installation, trust or invocation for another
collaborator ([Q6][Q6], [CD-08][contract]).

The managed root is an open design detail with one fixed constraint: it must not
be, or sit under, any documented discovery path of any supported client, and it
must not match any `instructions`, `skills` or plugin glob already present in
the project's or user's client configuration. Documented discovery paths today
include `.claude/`, `.github/`, `.codex/`, `.opencode/`, `.agents/skills`,
`.agents/plugins`, `AGENTS.md` and `CLAUDE.md`, plus their user-home
equivalents. Preflight must inventory configured globs before selecting a root,
because `.agents/rules/**/RULE.md` is already registered in shipped
`opencode.json` files. Package granularity is likewise open. Packages may
represent an item or a resolved bundle; the planner must deduplicate shared
logical items before exporting them. Equal bytes alone do not establish equal
identity, capability, resource dependencies or intended consumers. Installation
remains copy-only, consistent with the repository constraint.

## Rules

| Client              | Default route                                                                        | Package or exception route                                                       | Qualification boundary at current pin                                                                                                                                                                                                                                                                                                                        |
| ------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Claude Code 2.1.308 | Native `.claude/rules/<name>.md` with `paths:` where qualified                       | Package-owned unscoped rule through `SessionStart`                               | Native unscoped positive PASS; scoped matching/nonmatching control not run at this pin. Hook: **PASS, unscoped only (Q4)**, project startup/resume/manual compaction and disabled/uninstalled controls; effective-input uniqueness unqualified ([Q4][Q4], [summary][Q4 summary])                                                                             |
| Copilot CLI 1.0.103 | Native `.github/instructions/<name>.instructions.md` with `applyTo:` where qualified | `com.github.copilot/` rules component                                            | Native `.github/copilot-instructions.md` positive PASS; modular `applyTo` not qualified by it. Unscoped `.claude/rules` reader PASS (Q1); `paths:` matching access not run. Plugin component: **FAIL at both scopes (Q3)**; empty disabled/uninstalled arrays are not qualified negatives ([Q1][Q1], [summary][Q1 summary], [Q3][Q3], [summary][Q3 summary]) |
| Codex 0.164.0       | Package-owned rules through trusted plugin `SessionStart`                            | No team `AGENTS.md` rewrite                                                      | **FAIL: duplicate developer bodies after reopen + first automatic compaction (#379)**. Fresh positive PASS; second compaction records one body, effective input unobserved; fresh project registration not run. Route remains blocked ([Q9][Q9], [summary][Q9 summary])                                                                                      |
| OpenCode 2.1.4      | Independent executable plugin context adapter                                        | Explicit instruction paths are an unqualified alternative, not the adopted route | Native positives FAIL twice, HTTP 400; context/resume and skill controls **not run**. Local-directory `plugin add` rejected; explicit local `plugins` loading remains a separate unexecuted candidate ([Q5][Q5], [summary][Q5 summary], [AD-0040][direction])                                                                                                |

Historical successes at earlier pins do not qualify these current cells. Native
scoping and hook activation are separate capabilities; Q4 cannot supply `paths:`
semantics. CLI results also do not qualify the IDE, hosted or configured readers
of `.claude/rules` and `.github/instructions`; classify them per item rather
than treating generated paths as private ([Q4][Q4],
[CD-01 and CD-10][contract]).

### Exactly one delivery per client, and the `.claude/rules` gate

Each logical rule has exactly one delivery route per intended client. A client
never receives the same rule through its native route and through a package
hook. The native defaults above are therefore exclusive of the exception routes
for the same item, and the planner must reject a plan that selects both.

Copilot 1.0.103 reads unscoped project `.claude/rules` (Q1). The `paths:`
variant returned the fallback in a tool-free request; matching-file access was
not run, so it establishes neither ignored frontmatter nor selective exclusion
([Q1][Q1], [summary][Q1 summary]). Q1 establishes the duplicate vector; it does
not select a safe route for every rule ([design ledger][ledger]).

The earlier gate options now have these boundaries:

- (a) One `.claude/rules` copy consumed by both clients rests on the untested
  premise about Copilot's handling of `paths:`. Q1b must test matching and
  nonmatching access before any scoped sharing claim ([Q1][Q1]).
- (b) Copilot native plus Claude package hook is unscoped-only. Q4 passes that
  bounded Claude hook control, not path-scoped delivery ([Q4][Q4]).
- (c) Copilot plugin rules plus Claude native failed at both Copilot scopes. It
  is not an available qualified alternative ([Q3][Q3]).

No route currently demonstrates delivery of a scoped rule to both Claude and
Copilot in one project without duplication ([Q1][Q1], [Q3][Q3], [Q4][Q4]).
Decision required: the user selects the shared-project policy at step 7:

| Possible decision                                                | Consequence                                                                                                                                                                                                                |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Allow only unscoped shared rules                                 | Claude's Q4 hook and one Copilot native delivery can be evaluated together; scoped shared requests fail preflight. Integrated duplicate/isolation controls remain required ([Q4][Q4], [CD-03 and CD-04][contract])         |
| Declare Copilot an intended consumer of `.claude/rules` per item | Emit one compatible representation and no second Copilot copy; obtain explicit per-item consent and Q1b scope evidence. This does not justify Claude-only adaptations or ignored `paths:` ([Q1][Q1], [AD-0040][direction]) |
| Report scoped shared rules unsupported                           | Block that requested capability until a retained scoped route or selective exclusion passes; do not emit duplicate files ([Q3][Q3], [CD-01][contract])                                                                     |

Claude-only rules in a project where Copilot runs need their own branch. A
`.claude/rules` file has an unresolved Copilot reader unless that consumption is
explicitly declared and compatible; a Claude-only audience request must use a
qualified isolated representation or be unsupported. Q4 is only a candidate
unscoped alternative until unintended-reader controls pass. A warning alone is
not consent, and existing discovery settings stay unchanged ([Q1][Q1], [Q4][Q4],
[AD-0040][direction], [CD-03][contract]).

Codex and OpenCode have no documented default reader of `.claude/rules` or
`.github/instructions`, and Claude has no documented reader of
`.github/instructions`, so the remaining pairs carry no known default-discovery
duplicate vector. Configured readers are a separate matter: an OpenCode
`instructions` glob, a Copilot explicit import or any user-configured client
glob can select the native rule destinations as well as the managed root. The
glob and import inventory in preflight therefore covers `.claude/rules` and
`.github/instructions`, not only the managed root. Skills and plugin-delivered
agents avoid the native project-tree discovery paths because they live in the
managed package or a client cache, not in `.claude/`, `.github/` or
`.agents/skills`. Note that Claude loaded a project-scope plugin from its local
marketplace package source, not from a copied cache
([four-client coexistence controls](../../technotes/gate6-four-client-skill-coexistence.md)),
so for Claude the managed root's location is what provides isolation. The
native-file subagent exception for Claude does not have this property and has
the same Copilot reader problem; see "Subagents".

For Codex and OpenCode, a package's `rules/` directory is an upskill convention,
not automatic native rule discovery. The hook/adapter supplies the text.
Generate an explicit, deterministic rule index rather than scanning source globs
at runtime.

Always-on rules and scoped rules are separate capabilities. Startup injection
with conditional prose does not preserve native path-scoped activation. A route
must preserve the requested scope or report it unsupported. Resume, compaction,
subagent inheritance and running sessions require explicit lifecycle tests.
Removal cannot retract context already retained by a running session.

Codex hooks require ordinary `/hooks` trust review; Q9 reused unchanged,
previously reviewed definitions and did not test rule-index changes ([Q9][Q9]).

Decision required: select a distribution channel for hook-bearing Codex packages
and determine through Q13 whether index changes trigger re-trust ([Q9][Q9],
[design ledger][ledger]).

Hook-bearing plugins cannot currently be submitted to the public Codex
directory; local or manual distribution is the documented path. A `metapowers`
marketplace entry therefore does not by itself give a trusted Codex rule hook.

Cross-client compatibility readers: Copilot documents project `.claude/skills`,
`.claude/agents` and `.claude/commands` as sources. Q1 proves the pinned Copilot
CLI reads unscoped `.claude/rules`; scoped matching-file behavior remains
untested ([Q1][Q1]). A retained control also showed Copilot
`ignoredSkillsLocations` failing to block explicit activation of a
`.claude/skills` compatibility skill. Treat every `.claude/` path as potentially
visible to Copilot until a pinned test says otherwise. Never put Claude-only
adaptations there without proven reader controls. Existing team rules must not
be rewritten to manufacture isolation.

## Skills and resources

| Client      | Proposed delivery                                                                    |
| ----------- | ------------------------------------------------------------------------------------ |
| Claude Code | Native managed plugin `skills/<name>/SKILL.md`                                       |
| Codex       | Native installed and enabled plugin `skills/<name>/SKILL.md`                         |
| Copilot CLI | Native managed plugin `skills/<name>/SKILL.md`                                       |
| OpenCode    | Self-contained package skill directories registered as explicit native skill sources |

Keep references, scripts and assets beside the skill and validate dependencies
before publishing the entrypoint. A directory inside an OpenCode plugin is not
assumed to become a skill source automatically.

Prefer one complete generated package over mandatory runtime references to an
external shared core. Authors still maintain the common body once. Physical
sharing through `.agents/skills` is an optional optimization for qualified
consumer sets, not a prerequisite. Retained evidence: Codex, Copilot and
OpenCode read a shared `.agents/skills` entry and its resource; Claude direct
`.agents/skills` discovery FAILED, so any shared-root profile excludes Claude or
adds a Claude package. Do not expose a shared skill and a plugin variant
simultaneously unless tested native selection leaves one active item. A
same-name shared plus `.claude/skills` pair showed one Copilot catalog entry;
that is shadowing, not proof of a single installation.

## Subagents

| Client      | Preferred route                                                                            | Necessary exception or uncertainty                                                                                                                                                                                                                               |
| ----------- | ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code | Native plugin `agents/<name>.md`                                                           | Plugin agents ignore agent-local `hooks`, `mcpServers`, `permissionMode` and `initialPrompt`; use native standalone files if those capabilities are required                                                                                                     |
| Copilot CLI | Agent Plugins 1.0 `com.github.copilot/agents/<name>.agent.md` (candidate)                  | Only the `com.github.copilot/` namespace is documented in retained technotes, same caveat as the rules component; qualify delegation, identity, restrictions, collisions; project `.claude/agents` is a known compatibility reader without a selective exclusion |
| Codex       | Package-owned agent TOML referenced by `agents.<name>.config_file` in native configuration | Explicit registration, not automatic plugin discovery; the primary documented route is `.codex/agents/*.toml`; qualify against the installed agent schema                                                                                                        |
| OpenCode    | Package-owned definitions if a supported registration route is verified                    | Documented native routes are `.opencode/agents`, user agent directories or the `agents` config object; plugin-contained loading remains unresolved                                                                                                               |

The Claude exception route has a cross-client cost. A standalone
`.claude/agents/<name>.md` is also a documented Copilot CLI agent source, and VS
Code Copilot additionally lists `~/.claude/agents`. Copilot documents its own
agent fields and treats an omitted `tools` list as all tools; Claude-only fields
such as `permissionMode` are not documented as interpreted. A restricted Claude
subagent written as a native file can therefore become an unrestricted or
differently restricted Copilot agent in the same project. Q2 now proves
compatibility discovery and delegation, but not equivalent restriction
enforcement. Use the native-file exception only in projects without Copilot,
unless an explicitly intended compatible route passes its restriction controls;
otherwise report that restricted exception unsupported for the project. Do not
assume a selective exclusion ([Q2][Q2], [summary][Q2 summary],
[AD-0040][direction]). The same reasoning forbids `.claude/skills` and
`.claude/commands` as exception routes.

At Copilot 1.0.103, Q2 records a real child and resource read from a
Claude-format agent, without independently proving its tool catalog or
`permissionMode` enforcement. Q3 records a real namespaced user child, but its
exact-marker response fails and project-overlay activation is unverified. A safe
restricted-Claude exception is therefore unqualified; Q7 remains the Codex
registration control, not evidence supplied by these packets ([Q2][Q2],
[summary][Q2 summary], [Q3][Q3], [design ledger][ledger]). OpenCode 2.0.32
executed configured and file-backed children as real child sessions and produced
a native `permission.rejected` for a nonmatching-path edit under one unchanged
policy; MCP inheritance, delegation limits and rule scoping inside children were
not tested.

Prefer package-owned subagents with minimal external registration. Permit owned
native files when necessary. External agents must not remain active after the
corresponding upskill installation is disabled or removed. Direct native plugin
toggles that bypass upskill need reconciliation or an explicitly documented
limitation; do not assume they also toggle external registrations.

Share task instructions and return contracts at authoring time; generate native
metadata separately. Preserve actual restrictions, not just equivalent-looking
prose. Codex inherited execution boundaries, Copilot tool-policy limits and
OpenCode ordered permission semantics need capability-specific tests.

Models and effort remain client-specific settings resolved through the accepted
offline tier mappings in ADR-0014. This proposal adds no runtime fallback
resolver and does not record test model choices.

## User and project scope

| Client          | User-wide activation                                                                        | Project activation                                                                                                                           |
| --------------- | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude 2.1.308  | User plugin scope PASS in separate project ([Q6][Q6])                                       | Project plugin scope PASS; overlapping user/project activation not tested by Q6 ([summary][Q6 summary], [CD-04][contract])                   |
| Codex 0.164.0   | Q9 uses previously reviewed user registrations, not arbitrary-workspace delivery ([Q9][Q9]) | Fresh project rule registration not run; Q8 package lifecycle pending ([summary][Q9 summary], [CD-10][contract])                             |
| Copilot 1.0.103 | User-enabled candidate catalog present; complete rule route FAIL ([Q3][Q3])                 | Project-scope activation unverified (Q3): project-overlay candidate absent; user-enabled candidate present ([Q3][Q3], [summary][Q3 summary]) |
| OpenCode 2.1.4  | User activation not run in Q5 ([Q5][Q5])                                                    | Project configuration measured; component activation not run after failed positives ([Q5][Q5], [summary][Q5 summary])                        |

Activation scope is not physical storage scope. A user cache can serve a
project-only installation. User and project installations must resolve one
intended effective representation per logical item in the current session;
checking uniqueness separately within each scope is insufficient.

Private project installations are an additional capability where supported, not
an assumed universal third scope. Test unrelated-project non-activation and
preserve native trust requirements. All user-root delivery remains unqualified:
the retained Codex user-profile run limited callbacks to two declared project
paths and is not arbitrary-workspace user delivery.

## Existing generated output

Shipped upskill installations wrote `.claude/rules`, `.claude/skills`,
`.claude/agents`, `.github/instructions`, `.github/skills`, `.github/agents`,
`.agents/rules`, `.agents/skills`, `.opencode/agents`, an
`instructions: [".agents/rules/**/RULE.md"]` entry in `opencode.json`, and a
`CLAUDE.md` bridge. Each needs a recorded disposition before the new layout has
production effects: retained as-is under schema 1, migrated into a package, or
removed by an explicit owned operation. `.agents/rules` is a legacy OpenCode
destination registered through that glob; it is not inert storage and must not
become a source catalog without first handling the registration. Schema-1
installations are historical facts, not evidence of an approved shared catalog.

## Lifecycle and implementation constraints

Reuse captured-source evidence, complete selected-set planning, typed capability
errors, qualified roots, locking, local-edit detection and recovery. Stage
complete packages outside active discovery routes, then register them through
the native client. Capture previous configuration and observe the resulting
activation.

Disable registrations before reclaiming files. Account for native copies, active
or resumable sessions and shared owners. Do not delete client caches based only
on a missing registration. Cross-client operations are recoverable, not a
claimed distributed atomic transaction. Native residue needs a separate
preservation contract for each client; measured restoration is not whole-profile
equality ([CD-07][contract], [Q6][Q6], [Q3][Q3]).

| Client and pin  | Observed native residue and restoration boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude 2.1.308  | Native removal leaves empty `enabledPlugins` and `extraKnownMarketplaces`; measured settings are then restored exactly. The native `0.0.0` cache directory persists; `0.0.1` is the runtime source version, not an observed refreshed cache ([cache receipt][Q6 cache]). Registries match their baselines; session/log/cache effects are outside whole-profile equality ([Q6][Q6], [summary][Q6 summary]). Q4 also retains its initial onboarding restoration failure and subsequent owned cleanup ([Q4][Q4]) |
| Copilot 1.0.103 | Native uninstall leaves owned empty `installedPlugins`; measured configuration bytes/mode restored after unrelated-value checks. Session/log/cache/trust state is outside whole-profile equality ([Q3][Q3], [summary][Q3 summary])                                                                                                                                                                                                                                                                            |
| Codex 0.164.0   | Registration-removal residue contract to be filled by Q8. Q9 changes no registration and only proves unchanged measured configuration/package definitions ([Q9][Q9], [design ledger][ledger])                                                                                                                                                                                                                                                                                                                 |
| OpenCode 2.1.4  | Successful-route residue contract to be filled by Q5. Failed positives and rejected local installs leave measured configuration unchanged, not whole-profile equality ([Q5][Q5], [summary][Q5 summary])                                                                                                                                                                                                                                                                                                       |

Decision required: approve each per-client byte/key preservation allowance at
step 7; a retained native residue observation alone does not authorize the
implementation to preserve it as an accepted change
([CD-01 and CD-07][contract]).

Keep the single static Rust binary and existing dependency policy. Thin native
adapters may run in the client's own supported runtime; this does not authorize
a new shared JavaScript service or daemon. No automatic trust approval,
credential copying or broader permission grant follows from installation.

## Evidence and review questions

The retained
[evidence assessment](../../technotes/evidence/2026-10-23-delivery-choice/assessment.md)
records bounded skill/resource tests, historical independent rule context tests,
shared-path precedence controls and 65 existing lifecycle tests. The retained
Gate 6 technotes on `main`
([authenticated project controls](../../technotes/gate6-authenticated-project-controls.md),
[Codex user rules](../../technotes/gate6-codex-user-rule-controls.md),
[OpenCode user rules](../../technotes/gate6-opencode-user-rule-controls.md),
[Claude skills](../../technotes/gate6-claude-skill-controls.md),
[shared skills](../../technotes/gate6-shared-skill-controls.md),
[skill coexistence](../../technotes/gate6-skill-coexistence-controls.md),
[four-client coexistence](../../technotes/gate6-four-client-skill-coexistence.md),
[OpenCode children](../../technotes/gate6-opencode-child-controls.md)) add the
Codex and OpenCode plugin rule controls, the Claude and Copilot skill controls
and the OpenCode child-session controls cited above; their pins are Codex
0.163.0, Claude 2.1.292/2.1.303, Copilot CLI 1.0.98 and OpenCode 2.0.32. Those
checks do not qualify this complete design. Current OpenCode 2.1.4 execution
tests were blocked; its catalog checks do not prove invocation. No cache or
latency superiority was measured. Failed controls listed in this document are
retained as failures, never as negative passes.

The current rules boundaries above supersede the draft's documentation-only
labels: Q3 fails the Copilot plugin rule candidate, Q4 passes a bounded Claude
unscoped hook, and Q9 fails Codex uniqueness. Q5 retains failed positives rather
than execution qualification. Q7/Q8 still need their own indexed, merged packet
dispositions ([Q3][Q3], [Q4][Q4], [Q9][Q9], [Q5][Q5], [design ledger][ledger]).

Fable should review:

1. Does this proposal satisfy the agreed MVP and preserve one canonical shared
   core without duplicate activation?
2. Are the package/native-file exceptions sufficient, especially for Claude
   restricted subagents and OpenCode agent registration?
3. Which rule scopes can each adapter actually preserve?
4. Can user/project overlap and cross-client compatibility readers cause
   unintended activation, including through shared settings and manifests?
5. Can external registrations, native plugin toggles, updates and recovery
   remain consistent without modifying unrelated settings?
6. What package granularity, stable identities and managed roots should be
   chosen?
7. Which version/platform/scope tests must pass before each route is supported?
8. Is the superseded-records table complete, and in which order should those
   amendments land relative to AD-0040 merging?

Implementation and production activation remain unapproved.

## Primary references

These are rolling documentation sources checked during the discussion on
2026-10-23; they are not pinned runtime evidence.

- [Claude plugin manifest](https://code.claude.com/docs/en/plugins-reference)
- [Claude subagents and plugin restrictions](https://code.claude.com/docs/en/sub-agents)
- [Claude plugin installation scopes](https://code.claude.com/docs/en/discover-plugins)
- [Copilot plugin components](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference)
- [Copilot instruction readers](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#custom-instructions-locations)
- [Copilot configuration and scope](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)
- [Codex plugins and hook trust](https://developers.openai.com/plugins/build/plugins)
- [Codex configuration and agent registration](https://learn.chatgpt.com/docs/config-file/config-reference)
- [Codex subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents)
- [OpenCode plugins](https://opencode.ai/v2/docs/plugins/)
- [OpenCode agents](https://opencode.ai/v2/docs/agents/)

Repository records: [AD-0003](../../adr/0003-generation-pipeline.md),
[AD-0013](../../adr/0013-agent-customization-targets.md),
[AD-0014](../../adr/0014-versioned-agent-model-mappings.md),
[AD-0016](../../adr/0016-pure-instruction-planning.md) to
[AD-0018](../../adr/0018-retain-empty-instruction-aggregates.md),
[AD-0039](../../adr/0039-compile-captured-codex-instructions.md),
[adapter roadmap](2026-10-14-customization-adapter-roadmap.md),
[adapter evidence](../../technotes/customization-adapter-evidence.md),
[Gate 6 native qualification](../../technotes/m1-gate6-native-qualification.md).

## Retained campaign references

These fixed revisions cite the packet branches before merge. PASS and FAIL apply
only to the recorded macOS arm64 controls, not complete client qualification
([CD-10][contract]). Q7 and Q8 remain pending in this reconciliation; no local
unmerged result is promoted here ([CD-11][contract]).

[Q1]: https://github.com/driftsys/upskill/blob/a6166cf60d306f21c73b36688afdc49b57dbf5fd/docs/technotes/gate6-copilot-claude-rules-reader.md
[Q1 summary]: https://github.com/driftsys/upskill/blob/a6166cf60d306f21c73b36688afdc49b57dbf5fd/docs/technotes/evidence/2026-10-24-copilot-claude-rules-reader/summary.json
[Q2]: https://github.com/driftsys/upskill/blob/897876a1ce72bdb6faf8288b883e3dd793470ed6/docs/technotes/gate6-copilot-claude-agents-reader.md
[Q2 summary]: https://github.com/driftsys/upskill/blob/897876a1ce72bdb6faf8288b883e3dd793470ed6/docs/technotes/evidence/2026-10-24-copilot-claude-agents-reader/summary.json
[Q6]: https://github.com/driftsys/upskill/blob/0985dfb43c637996a8030c51bbd0e91fad2fe7eb/docs/technotes/gate6-claude-managed-root.md
[Q6 summary]: https://github.com/driftsys/upskill/blob/0985dfb43c637996a8030c51bbd0e91fad2fe7eb/docs/technotes/evidence/2026-10-24-claude-managed-root/summary.json
[Q9]: https://github.com/driftsys/upskill/blob/8ffde75620ca0b0993490b136f7015c869f7cc10/docs/technotes/gate6-codex-rule-lifecycle.md
[Q9 summary]: https://github.com/driftsys/upskill/blob/8ffde75620ca0b0993490b136f7015c869f7cc10/docs/technotes/evidence/2026-10-24-codex-rule-lifecycle/summary.json
[Q4]: https://github.com/driftsys/upskill/blob/43f7c26c3d64ad55aa5c1f8302ecb1a9573fba81/docs/technotes/gate6-claude-rule-hooks.md
[Q4 summary]: https://github.com/driftsys/upskill/blob/43f7c26c3d64ad55aa5c1f8302ecb1a9573fba81/docs/technotes/evidence/2026-10-24-claude-rule-hooks/summary.json
[Q3]: https://github.com/driftsys/upskill/blob/cd8c605674f34fea13c20233d648dcf16dd29cac/docs/technotes/gate6-copilot-plugin-components.md
[Q3 summary]: https://github.com/driftsys/upskill/blob/cd8c605674f34fea13c20233d648dcf16dd29cac/docs/technotes/evidence/2026-10-24-copilot-plugin-components/summary.json
[Q5]: https://github.com/driftsys/upskill/blob/6ec187f5422d72e9329c4c7f9cb706468e2ebb6b/docs/technotes/gate6-opencode-current-routes.md
[Q5 summary]: https://github.com/driftsys/upskill/blob/6ec187f5422d72e9329c4c7f9cb706468e2ebb6b/docs/technotes/evidence/2026-10-24-opencode-current-routes/summary.json
[contract]: ../../topics/0009-customization-delivery.md
[direction]: ../../adr/0040-separate-customization-delivery-from-discovery.md
[roadmap]: ../../roadmap.md
[ledger]: https://github.com/driftsys/upskill/pull/385#issuecomment-6384387650
[Q6 cache]: https://github.com/driftsys/upskill/blob/0985dfb43c637996a8030c51bbd0e91fad2fe7eb/docs/technotes/evidence/2026-10-24-claude-managed-root/retry-post-removal-cache.json
