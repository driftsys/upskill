# Proposed customization delivery architecture

Date: 2026-10-23; rules direction adopted by the user on 2026-10-24 after
reviewing Q1-Q9. Status: decided rules direction, pending durable-record
amendments and implementation. The separate step-7 adoption PR now reconciles
the
[durable inventory](../../topics/0009-customization-delivery.md#amendment-inventory).
The skills/subagent/package and release-scope amendments remain proposals; this
decision changes no code or production route
([user decision](#user-decision-2026-10-24), [AD-0040][direction],
[roadmap][roadmap]).

## Controlling records

This proposal records the new [rules decision](#user-decision-2026-10-24) and
identifies its required amendments to the direction accepted on 2026-10-19 in
[AD-0040](../../adr/0040-separate-customization-delivery-from-discovery.md)
("Separate customization delivery from discovery") and
[Topic 0009](../../topics/0009-customization-delivery.md), both on `main` since
PR #334 and extended on 2026-10-20 (PR #350). Where this proposal agrees with
them it says so. Where it diverges it says so and gives the reason, and the
pending direction amendments are recorded in [AD-0040][direction]; Topic 0009
links to that list rather than recording or adopting the amendments itself
([Topic 0009][contract]).

Agreements with AD-0040 and Topic 0009:

- One captured source with generated client-native outputs; a physically shared
  directory is optional, not the baseline.
- Rule delivery remains independent of team `AGENTS.md`; rewriting it is not a
  fallback. The new decision retains Codex hooks but allows explicit OpenCode
  instruction paths, requiring amendment of AD-0040's plugin-only wording
  ([user decision](#user-decision-2026-10-24), [AD-0040][direction]).
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
  wherever a native plugin root exists. Rules use native always-on routes first,
  with a package hook only where no native route exists or a fallback has been
  qualified ([user decision](#user-decision-2026-10-24)). AD-0040's extended
  section selects portable `.agents/skills` with one self-contained Claude
  adapter as the first experiment; this proposal makes the self-contained
  package the default and `.agents/skills` an optional qualified profile,
  because Claude direct `.agents/skills` discovery failed and the package route
  passed on Claude, Copilot and Codex, with OpenCode catalog evidence only.
  AD-0040 only requires that delivery and discovery stay separate; it does not
  select packages. Those successes are historical evidence at their recorded
  pins, not current route qualification ([Q6][Q6], [Q3][Q3], [Q5][Q5],
  [CD-10][contract]). The 2026-10-14 adapter roadmap's route mapping gives
  skills native discovery ("N") on all four clients and leaves subagents
  unsequenced; both change here.

## Records superseded or amended if this proposal is adopted

Adoption requires explicit amendments. The
[step-7 durable inventory](../../topics/0009-customization-delivery.md#amendment-inventory)
now records each disposition against merged evidence. Pending package/release
choices remain unadopted; shipped path tables are not silently replaced.

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

Additional required amendments from the user's 2026-10-24 rules decision are
listed here and reconciled in the separate
[step-7 record](../../topics/0009-customization-delivery.md#slice-1-evidence-and-adoption-2026-10-25),
with user adoption and live issue changes distinguished from native results
([user decision](#user-decision-2026-10-24)):

| Record                                                                                                                                                        | Required amendment                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [AD-0040][direction], [Topic 0009][contract] and [roadmap][roadmap] rules wording                                                                             | Replace mandatory qualified plugin delivery for Codex/OpenCode with delivery independent of team `AGENTS.md`; explicit OpenCode instruction file paths satisfy that intent when Q5 passes. Keep reader, ownership and activation obligations ([user decision](#user-decision-2026-10-24), [Q5][Q5])                                                 |
| [#379](https://github.com/driftsys/upskill/issues/379) and Codex rule status                                                                                  | Re-scope K0 blocker to K1 debt, retaining a resume-only attribution control. Record supported with declared limitation after that control is retained; do not relabel Q9's duplicate-body observation a zero-duplicate PASS ([user decision](#user-decision-2026-10-24), [Q9][Q9])                                                                  |
| [CD-04/CD-11][contract] and [roadmap dependencies][roadmap]                                                                                                   | Record the bounded Codex duplicate allowance and decided skills-before-rules order explicitly. Ownership deduplication and all other reader controls remain; the user records these amendments at step 7 ([user decision](#user-decision-2026-10-24), [Q9][Q9])                                                                                     |
| [ADR-0003](../../adr/0003-generation-pipeline.md), [format-spec §7](../../specification/format-spec.md) and [Topic 0001](../../topics/0001-content-format.md) | Record shared Claude/Copilot always-on native delivery, declared readers per item, explicit OpenCode file registration and the disposition of the old glob. Do not introduce a schema-2 writer; any declared-reader persistence gap in schema 1 needs its own implementation design ([user decision](#user-decision-2026-10-24), [CD-06][contract]) |

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
and any small client adapter. Always-on rules use native Claude/Copilot files
and explicit OpenCode instruction paths once Q5 passes; Codex uses the trusted
hook/index package. A broken native route uses only a separately qualified
fallback. Where supported, the package is a native plugin root
([decision][decision], [Q1][Q1], [Q5][Q5], [Q9][Q9]).

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
are not assumed inert or isolated. Q11 shows Claude 2.1.308 cataloging and
invoking a Copilot-shaped package when explicitly selected with `--plugin-dir`;
a client name or manifest shape is not an exclusion boundary ([Q11][Q11],
[summary][Q11 summary], [reader policy][contract]).

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

Separate marketplace entries can express intended installation choices, but they
do not enforce reader isolation. Q11's Claude explicit-selector candidate is
FAIL after a passed positive and clean baseline; Copilot's reverse direction is
blocked by its positive response contract. No unintended-reader PASS follows
from either result. Keep the retained PASS entry gate; the proposed
[Q11b registration-only control][Q11b] needs its own evidence and a separate
user decision before it could replace that gate ([Q11][Q11],
[summary][Q11 summary], [CD-03/CD-11][contract]).

Preflight must inventory incidental readers and manual-versus-package identity
collisions, including compatibility aliases. A separate listing or namespace
cannot make a requested audience exclusion safe when native activation remains
possible under the recorded discovery condition ([reader policy][contract],
[preflight design](2026-10-25-preflight-discovery-inventory-design.md)).

Update visibility is client-specific; a universal reinstall step is not
supported by the retained observations ([Q6][Q6], [Q3][Q3], [Q5][Q5]).

| Client and current pin | Update visibility                                                        | Evidence boundary                                                                                     |
| ---------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------- |
| Claude 2.1.308         | Fresh session sees a combined content-and-version edit without reinstall | PASS at project and user scopes; running-session visibility not run ([Q6][Q6], [summary][Q6 summary]) |
| Copilot 1.0.103        | Native installation reports live loading from managed source             | Q3 installation observation, not successful rule-update activation ([Q3][Q3], [summary][Q3 summary])  |
| Codex 0.164.0          | Source resource/version edit stays cached until native remove/re-add     | Q8 PASS at project activation; skill-body-only edits and other refresh verbs untested ([Q8][Q8])      |
| OpenCode 2.1.4         | Not run after failed positives                                           | Q5 provides no execution or update visibility result ([Q5][Q5], [summary][Q5 summary])                |

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

### User decision, 2026-10-24

The user adopted native-first always-on rule delivery after reviewing Q1-Q9.
Always-on rules are the MVP priority; scoped rules are secondary. Load rules
natively wherever the client has a native always-on route. Use a package hook
only where no native route exists, or as a separately qualified fallback when a
native route breaks. A uniform load mechanism across clients is not a goal.
Consistency belongs in the single authored source and upskill's existing
per-client generation ([ADR-0003](../../adr/0003-generation-pipeline.md),
[Q1][Q1], [Q3][Q3], [Q6][Q6], [Q9][Q9]).

The desired property for native routes is activation on clone, a rule file the
client itself lists, and no per-machine rule installation. Codex remains the
hook exception and therefore retains native package installation and trust
steps. Selecting this direction does not claim the still-pending OpenCode or
Copilot modular-file controls passed ([decision][decision], [Q5][Q5], [Q1][Q1],
[Q9][Q9]).

Reasons for the decision:

1. Evidence: the native rows have retained results at their named pins, while
   plugin lifecycle is the least-proven part of this campaign. Q3 fails the
   Copilot rules component, Q6 records native residue and a cache/source
   distinction, and Q9 records duplicate bodies. OpenCode's current-pin native
   positive failed, so its explicit-path retry remains required; a retained
   result does not mean PASS ([Q1][Q1], [Q3][Q3], [Q6][Q6], [Q9][Q9], [Q5][Q5],
   [historical native controls](../../technotes/m1-gate6-native-qualification.md)).
2. Onboarding: native files are available on clone. Claude and Codex plugins
   need per-machine, per-client installation; committed project settings do not
   install a collaborator's package. Codex hook definition changes require
   ordinary `/hooks` review; whether changing only the rule index triggers
   re-trust remains an open control ([Q6][Q6], [Q9][Q9],
   [design ledger][ledger]).
3. Debuggability: the user prefers a rule file listed by the client to text
   injected by a startup script. This is a design preference, not a measured
   performance result ([decision][decision], [Q6][Q6], [Q9][Q9]).

### Always-on rules

| Client      | Decided route                                                                                                                                                                                                                                           | Evidence and acceptance boundary                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code | Native `.claude/rules/<name>.md`, no frontmatter                                                                                                                                                                                                        | PASS at 2.1.292 in the [native controls](../../technotes/m1-gate6-native-qualification.md); an independent unscoped native positive also PASS at 2.1.308 ([Q4][Q4], [summary][Q4 summary]). Selected by the [decision][decision]                                                                                                                                                                                                                    |
| Copilot CLI | When Claude is also selected for the project, use that same `.claude/rules` file and record Copilot as a declared intended reader, without a `.github/instructions` copy. Otherwise use `.github/instructions/<name>.instructions.md` without `applyTo` | Q1 PASS: Copilot 1.0.103 reads unscoped `.claude/rules`; its native positive is `.github/copilot-instructions.md`, not a direct qualification of the modular no-`applyTo` output. The latter remains a selected route with its native validation obligation ([Q1][Q1], [summary][Q1 summary], [decision][decision])                                                                                                                                 |
| Codex       | Package-owned static rule index read by the trusted `SessionStart` hook                                                                                                                                                                                 | **Supported with declared limitation** is the adopted direction; the resume-only attribution is retained, with step-7 durable adoption still at user PR review ([follow-up][Q9 resume]). Q9 on 0.164.0: fresh positive and final responses PASS, but the former zero-duplicate contract FAILS after reopen plus first automatic compaction; the second compaction records one body per rule ([Q9][Q9], [summary][Q9 summary], [decision][decision]) |
| OpenCode    | Explicit `instructions` file paths, no glob. Fallback: qualified plugin context hook                                                                                                                                                                    | Explicit-path retry deferred at 2.1.4 after failed positives; downstream component controls not run ([latest retry][Q5 retry]). Hook execution passed historically on 2.0.32, not qualification at 2.1.4. See the [retained explicit-path observation](../README.md#opencode-explicit-path-observation) ([Q5][Q5], [summary][Q5 summary], [historical hook controls](../../technotes/gate6-opencode-user-rule-controls.md), [decision][decision])   |

Declared Codex limitation: the user accepts the recorded duplicate instead of
using #379 as an architecture blocker. The original combined run records two
bodies per rule after reopen plus the first automatic compaction and one after
the second. The 2026-10-25 resume-only follow-up is now retained: startup 1,
cumulative resumed history 2 and newly appended records 1 per rule, without
compaction. This attributes the extra recorded copy to resume.
Provider-effective input and duplicate lifetime remain unqualified; the accepted
transient allowance is not a new lifetime proof. Keep Q9's historical FAIL
unchanged, retain the HTTP 401 failure separately, and make the proposed #379 K1
re-scope at user adoption review ([Q9][Q9], [summary][Q9 summary],
[resume-only][Q9 resume], [decision][decision]).

### Planner and ownership rules

- One physical native delivery per rule per project. In a Claude+Copilot project
  write `.claude/rules/<name>.md` once, record Copilot as a declared intended
  reader per item in the lockfile, and refuse a second `.github/instructions`
  copy of that rule. This is a required planner behavior, not a claim of an
  implemented new lockfile field. Schema-1 production routing remains unchanged
  ([decision][decision], [Q1][Q1], [CD-04 and CD-06][contract]).
- Declared readers are per item. Other surfaces/readers, including Claude IDE
  sessions, Copilot cloud agent, code review and VS Code, remain unresolved
  under Topic 0009 rather than silently intended. Inventory their actual
  discovery conditions separately; CLI selection does not declare them consumers
  ([decision][decision], [CD-01 and CD-03][contract]).
- A Claude-only rule where Copilot runs must have an explicit compatible
  intended-reader declaration or be unsupported; a warning is not consent. The
  shared native route does not authorize Claude-only adaptations visible to
  Copilot ([decision][decision], [Q1][Q1], [AD-0040 reader policy][direction]).
- Rules leave managed packages on Claude and Copilot. Packages remain the home
  for skills, subagents, the Codex hook plus rule index, and the OpenCode
  fallback hook. Package ownership as the common abstraction means the lockfile
  owns native rule files and packages alike; it does not require identical
  loading mechanisms ([decision][decision], [CD-06][contract]).
- Hooks never call the upskill binary at runtime. They read a static index
  generated at install time; generation and native context loading remain
  separate operations ([decision][decision], [Q4 fixture][Q4], [Q9][Q9]).

### Scoped rules

Scoped rules are supported only where a native route enforces scope: Claude
`paths:`, or Copilot `applyTo:` when Claude is not selected. This is the decided
capability policy, subject to each native row's retained pin/scope evidence; the
unscoped Q1 positive alone does not qualify modular `applyTo` behavior. Every
other scoped request, including a shared Claude+Copilot project, is unsupported
and must never be widened to always-on. No scoped-rule work is scheduled in this
slice ([decision][decision], [AD-0040 capability policy][direction], [Q1][Q1]).

Q1b, `.claude/rules` with `paths:` plus matching-file access under Copilot, is
deferred and unscheduled. The Copilot plugin rules component is retired as a
candidate at 1.0.103 after Q3's failures at both scopes ([decision][decision],
[Q1][Q1], [Q3][Q3]).

### Rejected alternative retained as context

A package hook on all four clients reading rules from a managed root was
rejected as the default. It offers one mechanism; isolation depends on the
qualified discovery and registration conditions, not package construction. Q11's
explicit foreign selector crosses that boundary, so unintended-reader controls
remain required ([Q11][Q11], [CD-03][contract]). It makes rule activation depend
on plugin installation. Copilot session-start injection is untested: Q3 tested
its rules component, which failed, not a session-start hook. Q4 demonstrates
only the Claude unscoped hook ([decision][decision], [Q3][Q3], [Q4][Q4],
[CD-03][contract]).

Retain package hooks as the fallback category for a broken native route, usable
only after that client's fallback passes qualification. Deferred, unscheduled
Q3b tests a Copilot plugin session-start hook using Q4's fixture shape. Neither
Q3 nor Q4 qualifies that Copilot fallback today ([decision][decision], [Q3][Q3],
[Q4][Q4]).

### Performance comparison

No controlled performance measurement exists for either route. Criterion 7 of
the
[retained storage comparison](../README.md#opencode-explicit-path-observation)
stands: use the same rule bytes and task sequence, record cached and uncached
tokens, compare native versus hook on one client at a time, and measure
ordering, repeat/resume, updates and removal. Do not infer a caching advantage
from a path shape or from the user's explicit-path observation
([decision][decision], [Q5][Q5],
[delivery assessment](../../technotes/evidence/2026-10-23-delivery-choice/assessment.md)).

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
restricted-Claude exception is therefore unqualified; Q7 now supplies bounded
Codex explicit-registration, real-child and inherited read-only denial evidence,
not broader restriction or automatic plugin discovery qualification ([Q7][Q7]);
these Copilot packets supply none of that evidence ([Q2][Q2],
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

| Client          | User-wide activation                                                                        | Project activation                                                                                                                                       |
| --------------- | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude 2.1.308  | User plugin scope PASS in separate project ([Q6][Q6])                                       | Project plugin scope PASS; overlapping user/project activation not tested by Q6 ([summary][Q6 summary], [CD-04][contract])                               |
| Codex 0.164.0   | Q9 uses previously reviewed user registrations, not arbitrary-workspace delivery ([Q9][Q9]) | Fresh project rule registration not run; bounded Q8 project skill/resource and native refresh PASS ([Q8][Q8]) ([summary][Q9 summary], [CD-10][contract]) |
| Copilot 1.0.103 | User-enabled plugin candidate catalog present; plugin rules component FAIL ([Q3][Q3])       | Project-scope activation unverified (Q3): project-overlay candidate absent; user-enabled candidate present ([Q3][Q3], [summary][Q3 summary])             |
| OpenCode 2.1.4  | User activation not run in Q5 ([Q5][Q5])                                                    | Project configuration measured; component activation not run after failed positives ([Q5][Q5], [summary][Q5 summary])                                    |

Activation scope is not physical storage scope. A user cache can serve a
project-only installation. User and project installations must resolve one
intended effective representation per logical item in the current session;
checking uniqueness separately within each scope is insufficient.

Private project installations are an additional capability where supported, not
an assumed universal third scope. Test unrelated-project non-activation and
preserve native trust requirements. Arbitrary-workspace user delivery remains
unqualified: Q6 records separate projects, while the Codex rule controls guard
two declared project paths and Q8 observes one project activation. These are not
universal user-root delivery or cross-scope overlap qualification ([Q6][Q6],
[Q9 resume][Q9 resume], [Q8][Q8]).

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

| Client and pin  | Observed native residue and restoration boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude 2.1.308  | Native removal leaves empty `enabledPlugins` and `extraKnownMarketplaces`; measured settings are then restored exactly. The original removal receipt records `0.0.0`; a later read-only inventory also records a persisting `0.0.1` cache directory ([original cache receipt][Q6 cache], [review inventory](../../technotes/evidence/2026-10-24-claude-managed-root/retry-cache-review-inventory.json)). Registries match their baselines; session/log/cache effects are outside whole-profile equality ([Q6][Q6], [summary][Q6 summary]). Q4 also retains its initial onboarding restoration failure and subsequent owned cleanup ([Q4][Q4]) |
| Copilot 1.0.103 | Native uninstall leaves owned empty `installedPlugins`; measured configuration bytes/mode restored after unrelated-value checks. Session/log/cache/trust state is outside whole-profile equality ([Q3][Q3], [summary][Q3 summary])                                                                                                                                                                                                                                                                                                                                                                                                            |
| Codex 0.164.0   | Q8 restores its control baseline exactly; Q7 separately retains initial restoration failure and later notice-counter correction to its original hash/mode. Native logs/cache bookkeeping are outside whole-profile equality ([Q7][Q7], [Q8][Q8])                                                                                                                                                                                                                                                                                                                                                                                              |
| OpenCode 2.1.4  | Successful-route residue contract to be filled by Q5. Failed positives and rejected local installs leave measured configuration unchanged, not whole-profile equality ([Q5][Q5], [summary][Q5 summary])                                                                                                                                                                                                                                                                                                                                                                                                                                       |

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

The [rules decision][decision] supersedes the earlier gate options: Q3 retires
the Copilot rules-component candidate, Q4 remains an unscoped Claude fallback,
and Q9's duplicate observation becomes an accepted declared limitation under the
new policy, pending resume-only attribution and step 7 recording. Q5 retains
failed positives rather than execution qualification. Q7/Q8 still need their own
indexed, merged packet dispositions ([Q3][Q3], [Q4][Q4], [Q9][Q9], [Q5][Q5],
[design ledger][ledger]).

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
8. Does the superseded-records table separate the decided rules amendments from
   the other direction changes still awaiting step 7 user adoption
   ([decision][decision], [AD-0040][direction])?

The rules direction is adopted; implementation, other direction amendments and
production activation remain gated ([decision][decision], [CD-10][contract]).

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

The original fixed revisions below retain their historical snapshots. Q7/Q8 and
retry links now name merged packets. PASS and FAIL apply only to the recorded
macOS arm64 controls, not complete client qualification or adopted package
defaults. See the
[durable dispositions](../../topics/0009-customization-delivery.md#retained-control-dispositions)
([CD-10 and CD-11][contract]).

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
[decision]: #user-decision-2026-10-24
[Q7]: ../../technotes/gate6-codex-agent-registration.md
[Q8]: ../../technotes/gate6-codex-skill-updates.md
[Q9 resume]: ../../technotes/gate6-codex-resume-attribution.md
[Q5 retry]: ../../technotes/gate6-opencode-explicit-paths-retry.md
[Q11]: ../../technotes/gate6-cross-client-package-isolation.md
[Q11 summary]: ../../technotes/evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/summary.json
[Q11b]: 2026-10-25-q11-registration-isolation-follow-up.md
