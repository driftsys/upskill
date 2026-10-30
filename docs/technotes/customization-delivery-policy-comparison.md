# Customization delivery policies after Q11b

Date: 2026-10-31. Task [#442](https://github.com/driftsys/upskill/issues/442).
Status: decision material; neither layout nor replacement qualification gate is
adopted. This note completes a comparison from retained evidence. It schedules
no native experiment and changes no generated output, schema or runtime route.

## Decision to make

Compare **A, separate generated client representations**, with **B, shared
installed defaults and client-specific replacements**. My recommendation is to
use A as the next design baseline, with explicitly selected shared profiles as
exceptions. This is a design judgment about fewer selection obligations, not an
observed safety advantage. A still fails wherever its native exposure reaches an
excluded reader. Approving this comparison or merging its documentation does not
approve A, its paths, package defaults or implementation.

The immediate next deliverable is a concrete route/profile proposal for a chosen
worked example below, supported by a credible native selection mechanism. If no
changed mechanism is evidenced, retain the unsupported result and stop before
another paid probe. Do not repeat Q11b unchanged or turn a failed exclusion into
intentional sharing without the user's decision.

## Evidence that constrains the choice

| Retained control                       | Observed profile and outcome                                                                                                                                                                                                | Consequence for this comparison                                                                                                                                |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Q11][q11]                             | Claude 2.1.308, macOS arm64, explicit session `--plugin-dir`: Copilot-shaped package activates after calibrated positive and clean baseline. Reverse direction blocked by its positive.                                     | Package envelope and directory name do not exclude an explicitly selected foreign reader. This is not automatic-discovery evidence.                            |
| [Q11b][q11b]                           | Claude 2.1.308 project local marketplace registration; fresh Copilot 1.0.103 observer automatically catalogs and invokes the exact owner source. Clean selector-free baseline and post-removal; both native positives pass. | The tested Claude registration cannot serve Claude-only content with this unintended Copilot reader. No foreign selector or Copilot registration was supplied. |
| [Q11b preservation][q11b]              | Native uninstall and marketplace removal complete; strict whole-file witness returns `preservation_failed`. Formatting, empty owned settings containers and native bookkeeping differ.                                      | Native cleanup success and isolation FAIL remain separate from preservation. No snapshot restoration ran; this does not prove restoration impossible.          |
| [Earlier Copilot exclusion][exclusion] | Copilot 1.0.98, macOS project: narrow `ignoredSkillsLocations` write/readback succeeds, but explicit native invocation still activates the selected Claude-compatible skill.                                                | This condition is a failed isolation control. It is neither a new 1.0.103 result nor a proven solution to Q11b registration discovery.                         |
| [Four-client coexistence][coexistence] | Sequential macOS project observations: Claude local package; Copilot, Codex and OpenCode shared entry; bound resource reads and bounded disable/remove survival.                                                            | Useful intentional-sharing feasibility, without exhaustive aliases/catalogs, scope/platform or integrated lifecycle qualification.                             |

These dated results do not transfer to Linux, user scope, IDE/cloud surfaces or
changed pins. Q11b's continuation also probed Codex 0.166.0 and OpenCode 2.1.4
versions; neither ran a qualification control there. Q7/Q8 observations remain
on Codex 0.164.0. No four-client support claim follows from one pair's failure.

The [reader policy][contract] distinguishes delivery, discovery, activation and
ownership. Equal bytes, body compatibility, a namespace or a warning does not
authorize an incidental consumer. An intended reader is declared per logical
item and profile; incidental consumption creates no owner/reference edge.
Isolation here concerns customization discovery and activation, not content
confidentiality from ordinary permitted file reads.

## Compare the layouts

Both candidates retain the upstream/local install source as SSOT. Consumers get
copied, consumption-ready outputs with source identity, hashes and ownership; no
second visible authoring SSOT or symlink layout is proposed. Supporting
resources travel with their selected representation. A copied procedure resource
is not an additional skill entrypoint.

| Concern                                   | A: separate client representations                                                      | B: shared defaults plus replacements                                                                                                 |
| ----------------------------------------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Illustrative storage, not a path contract | `.agents/{claude,copilot,codex,opencode}`                                               | Shared `.agents/skills/<item>`; replacements outside every shared catalog                                                            |
| Native exposure                           | One qualified native file, registration or configuration route per intended item/client | Shared native reader, or one qualified adapter where needed; per-client replacement must suppress the shared default for that client |
| Additional selection burden               | Reader inventory, compatibility exclusion and duplicate controls still required         | Same obligations plus per-item shared/default selection, precedence, exclusion and replacement-removal policy                        |
| Storage/update tradeoff                   | More generated copies; per-client metadata/resources explicit                           | Potentially fewer identical copies; a shared update affects every declared consumer                                                  |
| Ownership                                 | Client outputs and native effects, direct/bundle references                             | Same, plus shared owners and final-reference retirement; readers do not become owners                                                |
| What Q11b rules out                       | Tested Claude-only project-marketplace route with unintended Copilot                    | Using that same registration as an isolating selector; a private variant path alone cannot hide a visible shared base                |
| Current disposition                       | Recommended design baseline only; native boundary unresolved                            | Useful candidate for deliberate sharing; replacements/exclusions unresolved                                                          |

The original local comparison draft is retained, uncommitted context in the main
worktree at
`docs/wip/specs/2026-10-23-customization-storage-delivery-candidates.md`. This
note restates the alternatives without moving or adopting that draft. The
[existing shared-root evaluation][roots] remains historical research. Its older
rules-first sequencing is superseded by the adopted skills-before-rules
amendment in [AD-0040][direction] and the [roadmap][roadmap].

## Four-client route and reader matrix

**Observed** denotes only the cited bounded native result. **Inferred blocked**
is a consequence of a failed control or unmet contract for the named use case.
**Unknown** means the complete proposed route lacks qualifying evidence; it is
not an absence result. Existing generated formats are not native qualification.
Every row retains project/user, macOS/Linux and existing IDE/cloud obligations;
only the profiles explicitly cited below have observations. The matrix compares
A/B obligations; it does not silently replace selected native-first rule routes.

Preservation classes are those in [Topic 0009][contract]: F protects files and
resources, R also protects team guidance and native scope, N covers observed
native registrations/trust and inverse effects, B covers reference composition.

| Client / kind             | Candidate representation and native exposure                                                                 | Readers and A/B selection obligations                                                                                                                            | Evidence and remaining blocker                                                                                                                                                                      | Preservation               |
| ------------------------- | ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| Claude rules              | Selected native `.claude/rules`; neither A nor B forces plugin packaging                                     | Copilot is intended only for explicitly shared unscoped rules; Claude-only with visible Copilot rejects. Codex/OpenCode/import/IDE readers remain inventoried.   | Observed Q4 Claude 2.1.308 and Q1 Copilot 1.0.103 native-rule cases; complete adapter/scoped coexistence unknown ([selected rules][rules]).                                                         | F + R                      |
| Copilot rules             | Selected same Claude file when both readers intended; otherwise native `.github/instructions` candidate      | No second Copilot copy for a shared rule. CLI, IDE/cloud and configured imports are separate consumers.                                                          | Observed bounded Q1 reading; modular/no-`applyTo` route and full preservation unknown. Q3 plugin-rule FAIL is not a skill-route result ([rules][rules]).                                            | F + R                      |
| Codex rules               | Trusted package hook reads a static generated index beside unchanged team guidance                           | A stores selected outputs; B needs the same one-source selection before injection. Shared root is not native rule support.                                       | Observed Q9 at 0.164.0; declared recorded-history duplication limitation, provider-effective input and complete lifecycle unknown ([rules][rules]).                                                 | F + R + N                  |
| OpenCode rules            | Selected explicit instruction paths, no glob; hook fallback separately qualified                             | Existing configured globs/imports must be inventoried and preserved; adding a hook cannot duplicate an existing reader.                                          | Q5 positive failed at 2.1.4; downstream unknown/deferred. Neither B nor A supplies a passing route ([Q5][q5]).                                                                                      | F + R + N where configured |
| Claude skills/resources   | A: self-contained native package; B: qualified adapter for a shared item, or selected private representation | Copilot project-registration reader is unintended for Claude-only; OpenCode compatibility/import readers unresolved. Do not add shared/direct/plugin duplicates. | Observed Q6 project/user skill/source visibility at 2.1.308; Q11b makes tested Claude-only coexistence inferred blocked. Integrated resources/lifecycle unknown ([Q6][q6], [Q11b][q11b]).           | F + N                      |
| Copilot skills/resources  | Native entry/package; B can intentionally expose a shared portable entry                                     | Claude-compatible, shared, plugin and alias readers all count; IDE/cloud are distinct consumers. Variant selection cannot leave shared base visible.             | Observed shared-resource controls at 1.0.98; native-name positive and unintended Q11b activation at 1.0.103. Full A/B selection and owned lifecycle unknown ([exclusion][exclusion], [Q11b][q11b]). | F + N if registered        |
| Codex skills/resources    | A: installed package/cache entry; B: one shared-native or adapter route                                      | Inventory package/extra-root/shared exports and aliases; compatibility readers remain unresolved for exclusions.                                                 | Observed Q8 0.164.0 project skill/resource and remove/re-add refresh; unrelated project, user/Linux and full reader isolation unknown ([Q8][q8]).                                                   | F + N if registered        |
| OpenCode skills/resources | Qualified native source or package directory; source selector remains a profile decision                     | Shared and Claude-compatible/configured roots may duplicate entries. A directory label supplies no exclusion.                                                    | Observed v2.0.32 shared resource controls; current campaign 2.1.4 Q5 positive failed, downstream deferred. No transfer across versions ([resources][opencode-resources], [Q5][q5]).                 | F + N where configured     |
| Claude agents             | Native agent definition or qualified package registration                                                    | Compatibility agent loaders/imports need independent negatives; skill exclusion does not exclude agents.                                                         | Native formats documented; complete proposed owned mapping unknown. Plugin-ignored restrictions cannot become wider session authority ([agent contract][agents]).                                   | F + N if registered        |
| Copilot agents            | Native `.github/agents` definition and actual delegation                                                     | Claude-compatible agents and IDE/cloud readers separate; shared body does not prove equivalent tool/model restrictions.                                          | Existing generation; proposed lifecycle/selection unknown. Actual child, allowed/denied operations and inheritance required ([agents][agents]).                                                     | F + N where configured     |
| Codex agents              | Explicit native configuration referencing package-owned role                                                 | Plugin-only registration is not assumed. Shared intent or A storage cannot establish inherited execution limits.                                                 | Observed Q7 0.164.0 real child/inherited read-only denial; plugin-only role absent; complete adapter and other profiles unknown ([Q7][q7]).                                                         | F + N                      |
| OpenCode agents           | Native v2 definitions/config with actual child execution                                                     | Config/file/plugin merging and compatibility aliases remain inventoried; reject unsupported permission/delegation mappings.                                      | Observed bounded v2.0.32 child/path-denial cases, not current-pin generated-adapter qualification; complete profile unknown ([children][opencode-children], [agents][agents]).                      | F + N where configured     |

Bundles retain source/dependency identity, aliases and direct/bundle reference
edges (B); removing one edge does not remove a surviving representation. This
comparison does not decide authorable-bundle release scope. Native plugins and
marketplaces keep separate registrations, scope, trust, updates and shared
dependencies (F + N + B). MCP keeps existing native configuration and explicit
integration-removal behavior; it is not implicitly bundled into a layout (F +
N). General hooks, tools, permissions, model-profile resolution and new
owned-agent MVP scope remain separate planned/pending decisions. No matrix row
expands them.

## Three worked admission examples

These are proposed preflight outcomes, not current CLI behavior. An admissible
intent is not an implementation PASS. Unknown native boundaries reject effects
until their profile is qualified under [CD-01 through CD-11][gates].

### One intentionally shared portable skill

The user explicitly declares Claude and Copilot intended readers of one logical
skill with compatible body, metadata and relative resources. Codex/OpenCode are
unresolved until inventoried; they are not silently included or excluded. A
selects one client route each; B selects one shared entry or adapter per client.
The [coexistence packet][coexistence] gives bounded feasibility, not a complete
reader inventory for either plan.

Outcome: **unresolved for installation**; sharing intent for the two named
clients is admissible, but no complete profile is qualified here. Block on
Codex/OpenCode and applicable IDE/cloud reader disposition, resource identity,
alias/catalog counts and ownership lifecycle. Record owners from explicit
references, not incidental reads. An update must preserve compatibility for both
declared readers or reject; partial removal retains the surviving owner's bytes
and native dependency. Final removal observes native detachment and live
resource use before reclaiming owned files. Do not create a second Copilot copy.

### Claude-only skill with Copilot present

The requested reader set excludes Copilot. A copied Claude package under a
client-named root still uses the Q11b-tested project local marketplace route; B
cannot make that registration private by pointing at a different body.

Outcome: **reject the tested profile before effects**. The named blocker is
Copilot 1.0.103 automatic catalog/load of the Claude registration, not package
format or a missing positive. Source/resources reached by that activation stay
unintended. No observer-owned reference is created. A different isolation
mechanism requires its own profile; intentional sharing requires a new explicit
reader decision and full shared-route qualification. Neither permits deleting
manual compatibility content or changing global discovery to obtain a PASS.

### One logical skill with divergent client representations

Claude receives body/resource C; Copilot receives body/resource P with the same
logical source lineage. Both are intended consumers, but of different
representations. A needs one selected exposure per client, and must exclude C
from Copilot and P from Claude. B additionally needs to suppress the shared base
for the replacement reader; placing P outside shared catalogs is insufficient.

Outcome: **unresolved; reject effects until a selector is qualified**. Copilot's
Q11b compatibility route is a named blocker to claiming C private. Alternative
aliases or a package prefix do not create a new logical item or hide duplicates.
Retain distinct resource paths/permissions and actual read receipts. Update must
not re-expose the alternate/shared body. Before replacement removal, choose
explicitly between unavailable and a compatible shared fallback; no fallback is
assumed. Keep surviving direct/bundle ownership, manual copies and unrelated
registrations. Missing per-item suppression leaves B unsupported for this case;
A remains unsupported if its separate native route is also exposed.

## Mechanism register and stop conditions

This register uses retained sources, not a fresh audit of rolling vendor docs.
References and command shapes describe their recorded pins; they are not
instructions to run against a user's profile. No command was executed for this
comparison. Existing command receipts are replay evidence, not a newly qualified
replacement mechanism.

| Candidate mechanism / retained source                                                                                   | Runnable detail retained                                                                                                                     | What changes before another experiment could be useful                                                                                                                                                                                 |
| ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude project marketplace, [Q6][q6] and [Q11b receipts][q11b-receipts]                                                 | Native marketplace add, plugin install/uninstall and marketplace remove with project scope; exact recorded arguments in receipts             | Already tested and fails Claude-only Q11b. A different folder alone is not an evidenced isolation change. No repeat scheduled.                                                                                                         |
| Copilot narrow skill-directory exclusion, [2026-10-21 receipts][exclusion-receipts] and [dated reference survey][roots] | Recorded `copilot config ignoredSkillsLocations <owned-skill-directory>` and readback `copilot config ignoredSkillsLocations`                | Explicit invocation still activated at 1.0.98. Need evidence of a materially different supported selector/version condition and its applicable registration reader; do not assume this field handles Q11b plugins or repository scope. |
| OpenCode compatibility flags, [2026-10-20 reference survey][roots]                                                      | Survey records `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS=1` and distinct broader/prompt-only flags; no qualified current-pin command/profile here | They do not solve Copilot's Q11b reader. Need exact v2/version/scope semantics, preservation of unrelated readers and positive/negative parity; Q5 is still deferred.                                                                  |
| B per-item shared-default replacement/exclusion                                                                         | **None evidenced** in the assembled packet with a runnable qualified selector and lifecycle                                                  | Need source-backed exact selector/configuration, effective scope, alias/precedence semantics, shared-base suppression and defined removal outcome. No native run is scheduled.                                                         |
| A private Claude exposure with Copilot present                                                                          | **None evidenced** as a qualified replacement for Q11b project registration                                                                  | Need source-backed changed native exposure and a complete reader/preservation profile. An unscanned-looking storage root is insufficient.                                                                                              |

A newly documented selector would be a lead, not runtime proof. First identify
its source, frozen executable/configuration and exact command sequence offline.
Then prepare a separately reviewable profile: same-client positives, clean
selector-free baseline, intended-owner install and invocation, source-bound
observer catalog/attempt/load receipts, distinct fresh sessions, native cleanup,
post-removal and measured preservation. Declare the one changed discovery
condition and all reader/duplicate controls before effects. Require resource
reads and replacement removal/fallback checks where relevant. A failed positive
blocks clean absence; a bound unintended activation stays FAIL despite later
gaps. No probe follows until its profile and authority are explicit.

A could lose its provisional design preference if a sourced B selector passes
shared-base suppression, exclusion, one-effective-representation and lifecycle
controls with fewer ownership/configuration effects than the competing A route.
This is a falsifiable comparison criterion, not a ready-to-run experiment; no
such selector is established in this package.

## Preservation: retain outcomes and approved policy

[D1-D6][preservation] were already approved on 2026-10-25. Do not reopen them as
if approval were missing. Concrete adapter profiles still require approval and
qualification; D2/D3 allow only specified operation-created empty containers,
not arbitrary settings rewrites or changed modes/presence. D1 reports native
cache, trust, auth, logs and session effects separately; it does not authorize
direct mutation of native registries/caches or ignoring protected changes.

| Q11b observation                                             | Existing boundary and prospective profile requirement                                                                                                                                                                                   |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Original compact settings reformatted                        | Historical strict witness remains FAIL. D2 does not authorize a whole settings rewrite; qualify preservation of unrelated manual bytes/configuration for the selected profile.                                                          |
| Empty owned `enabledPlugins` / `extraKnownMarketplaces`      | D2 is a bounded allowance only after operation-created ownership, released references, manual content, mode/presence and remaining-reader conditions pass. It cannot relabel Q11b whole-file equality.                                  |
| Native registry/cache changes                                | Report native operations and observed remaining state separately. Establish preserved manual entries, ownership, safe inverse/recovery outcomes and unknown-state handling; never restore an entire registry or patch a cache directly. |
| Usage/plugin-sync/session bookkeeping                        | Report under the explicit D1 measured boundary; no whole-profile equality claim. Do not infer inability to recover or permit unrelated protected changes from the existence of bookkeeping residue.                                     |
| Native uninstall/remove succeeds and observer becomes absent | This proves the recorded cleanup/absence, not byte equality, interrupted recovery, resumed-session detachment or safe resource reclamation. D5 applies while live use is unresolved.                                                    |

A new exception, expanded measured boundary or settings writer needs its own
explicit policy/secret-safe capture decision. Implement only approved profiles,
with exact F/R/N/B owner contracts, before/after observations and native-effect
recovery. This comparison neither edits real settings nor adopts a recovery
mechanism.

## Gate and adoption decisions

Under **A**, CD-03 and Q11 still require calibrated unintended-reader negatives
for every applicable selected native exposure, including explicit foreign
selection where the retained gate calls for it. Separate storage cannot satisfy
that gate. CD-04 additionally requires one effective item through scopes,
aliases, direct/package and bundle overlap.

Under **B**, the same gates apply, plus evidence that the intended replacement
is the only effective body for that reader and that excluded readers cannot
activate the shared or alternate body. Intentional-sharing positives do not
substitute for exclusions. CD-05 through CD-11 retain resource, permission,
configuration, ownership and integrated recovery obligations for both layouts.
Q10's bounded overlap PASS supplies neither layout isolation nor all profiles.

The user still decides:

1. Whether to adopt A as the design baseline, retain B for explicitly shared
   profiles, or compare them without that preference. Exact storage and package
   defaults remain unadopted either way.
2. Which consumers are intended for each item/profile. This note chooses no new
   shared audience and grants no consent for incidental readers.
3. Replacement-removal behavior: unavailable, or an explicitly compatible
   fallback qualified under the same restrictions. No silent weakening.
4. Each concrete preservation/profile boundary under approved D1-D6, and any
   prospective exception beyond them. Historical failures remain immutable.
5. Whether to retain the current Q11 entry requirement or request a separate
   prospective gate proposal. This package retains it; Q11b's FAIL cannot
   justify a waiver or itself satisfy a narrower isolation gate.

Adoption would require explicit amendments to [AD-0040][direction],
[CD-03/CD-11][gates] and the [implementation handoff][handoff], as applicable.
Recording completed evidence in the handoff now does not adopt a layout or gate.
The inherited package-default, owned-agent MVP, authorable-bundle and OpenCode
skill-source decisions remain separate. Schema 1 stays production. Schema-2
activation and the combined four-client release remain blocked by applicable
native, platform/scope, permission, preservation and integrated lifecycle gates.

## Completion boundary

This decision package is complete when the comparison, twelve client/kind rows,
three worked examples, mechanism gaps, approved-policy boundaries and adoption
questions are reviewable and linked from the active handoff/indexes. It does not
complete [#410](https://github.com/driftsys/upskill/issues/410), choose a
runtime mechanism or create native qualification. Unrelated active WIP stays in
place.

[q11]: gate6-cross-client-package-isolation.md
[q11b]: gate6-project-registration-isolation.md
[q11b-receipts]: evidence/2026-10-30-q11b-registration-isolation/owner-install-receipts.json
[exclusion]: gate6-shared-skill-controls.md
[exclusion-receipts]: evidence/2026-10-21-gate6-shared-skill/summary.json
[coexistence]: gate6-four-client-skill-coexistence.md
[roots]: shared-content-root-evaluation.md
[contract]: ../topics/0009-customization-delivery.md
[rules]: ../topics/0009-customization-delivery.md#rules
[agents]: ../topics/0009-customization-delivery.md#agents
[gates]: ../topics/0009-customization-delivery.md#acceptance-and-evidence-gates
[direction]: ../adr/0040-separate-customization-delivery-from-discovery.md
[roadmap]: ../roadmap.md
[handoff]: ../wip/specs/2026-10-23-customization-delivery-implementation-spec.md
[preservation]: ../wip/specs/2026-10-25-customization-preservation-contract.md
[q5]: gate6-opencode-explicit-paths-retry.md
[q6]: gate6-claude-managed-root.md
[q7]: gate6-codex-agent-registration.md
[q8]: gate6-codex-skill-updates.md
[opencode-resources]: gate6-opencode-shared-resources.md
[opencode-children]: gate6-opencode-child-controls.md
