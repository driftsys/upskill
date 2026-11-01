# Native adapter contract study

Status: informative research, 2026-11-01. No implementation, profile admission,
qualification amendment or runtime experiment is authorized by this report.

Study branch: `docs/native-adapter-study`; source commit:
`6fe2e2307ee91ca83c3958748cc11a12003be1d3`. The isolated worktree is
`.claude/worktrees/native-adapter-study`. This report is the only study output.
Bootstrap was attempted by the main lane and failed on GitHub DNS resolution,
including an escalated attempt. It was not repeated by the study worker. During
the original study, no native executable probes, provider calls, profile
changes, Rust checks, commits or PR operations ran. Later commits are recorded
below.

## Authority and evidence

The [approved design](2026-10-31-four-client-skills-resources-design.md),
[delivery plan](../plans/2026-10-31-four-client-skills-resources-delivery.md),
[AD-0044](../../adr/0044-coordinate-native-delivery-with-one-state-and-operation-intent.md)
and [preservation contract](2026-10-25-customization-preservation-contract.md)
remain authoritative. The
[qualification index](../../technotes/m1-gate6-native-qualification.md) supplies
historical discovery limits; the client packets below supply the more specific
plugin observations. None qualifies the complete approved route.

Evidence labels used below are **retained runtime**, **captured help**,
**current docs** and **proposal/inference**. All external pages linked below
were fetched on **2026-11-01**. They describe current interfaces, without
proving that those interfaces exist or behave identically at retained client
pins. Retained observations are macOS arm64 and their individual dated pins
only.

## Shared contract against Stage 5

Use the existing [adapter trait](../../../src/native_delivery/adapter.rs), not a
second coordinator. Its profile already binds ID/revision, surface, scope,
client version, platform, exposure order and command/file-only action. Its
`capture`, read-only `plan_inverse`, `apply`, independent `observe` and
`inverse` provide the seams below. Concrete selectors, serializers, native
command fixtures and retention witnesses still need qualification and profile
approval.

Every candidate profile must additionally pin the qualified project/home and
private-evidence roots, resolved executable identity and availability, effective
configuration sources and overrides, native package/catalog identities, source
and revision digests, exact invocation/environment, trust state, competing
readers and applicable session/cache policy. A missing command is unsupported,
not a successful no-op. Another OS, IDE, cloud surface or client release is a
different profile. Native defaults and configuration contributions can activate
content without an explicit install command; inventory those readers first.

`capture` records bindings and native inventories separately from protected file
presence/bytes/modes and exact owned selectors. `plan_inverse` identifies only
the owned delta and expected poststate, pre-existing versus operation-created
containers, surviving direct/bundle/manual references and shared catalogs.
Private evidence is not permission to persist credential-bearing whole-file
configuration: preserve D6's bounded in-memory observation and non-secret
receipt boundary. A writer needing secret-bearing snapshots remains blocked on a
separately approved capture/storage contract.

The durable recipe and Pending checkpoint precede each effect. Revalidate roots,
files, ownership and profile around every call. An attempt's `Confirmed`,
`Rejected` or `Unknown` is separate from an observation's
`Before`/`After`/`Diverged`/`Unknown` relation and final effective binding.
`Confirmed` needs an attributable intended outcome; an installed but unexposed
package may confirm an install effect while its binding remains absent.
`Rejected` needs an affirmative refusal plus observed unchanged before-state; a
nonzero exit after partial effects cannot establish rejection without those
observations. `Unknown` covers timeout, malformed/incomplete inventory,
unavailable observer and ambiguous partial effects.

After timeout, first establish native-worker quiescence under the qualified
profile. Reaping the direct child or finding its process group empty does not
prove that escaped workers have stopped. Without that witness, retain `Unknown`
and do not invoke inventory or cleanup commands. Once quiescence is established,
observe independently with the same operation/profile/target and exact
selection. An `After` observation can acknowledge that effect; final bindings
and files still need confirmation before state publication. `Before` requires
the coordinator's explicit recovery disposition and renewed preconditions.
Divergence or unknown retention stays `RecoveryRequired`, with files, ownership
and journal retained. Neither success exit nor file rollback permits blind
retry, inverse or completion. Failed inverse likewise preserves evidence and
manual bytes.

## Claude Code

**Surfaces and provenance.** Approved packaging uses a revision-local
`.claude-plugin/plugin.json`, default `skills/` and an item-root
`.claude-plugin/marketplace.json`. **Current docs:**
[manifest reference](https://code.claude.com/docs/en/plugins-reference) confirms
the compatibility manifest and that explicit skill paths supplement the default
directory; declaring the default twice risks duplicate readers. **Retained
runtime:** [Q6](../../technotes/gate6-claude-managed-root.md), 2026-10-24,
Claude 2.1.308, records project/user install and fresh-session body/version edit
visibility, with native inventory cache paths distinct from runtime source
paths. [Q10](../../technotes/gate6-claude-scope-overlap.md), 2026-10-25, records
one visible activation during same-package user/project overlap and after either
scope is removed. It does not establish general precedence for different
packages, manual collisions or provider-effective uniqueness.

**Contract sketch.** Capture user `$CLAUDE_CONFIG_DIR/settings.json`, project
`.claude/settings.json` and any effective local/managed contributions; own only
the exact `enabledPlugins` key and `extraKnownMarketplaces` identity. Observe
native `known_marketplaces.json`, `installed_plugins.json` and reported cache
paths without writing them. Apply native marketplace/plugin operations in the
intended scope after inactive exports are ready. Observe source-bound native
inventory, enablement, scope, namespace, fresh catalog and correlated
skill/resource activation separately. Inverse releases only owned entries and
uses native uninstall/catalog removal after checking surviving dependencies. D2
permits only its narrowly declared empty containers in pre-existing settings;
lexical rewrites, changed mode or an unexpected new file remain failures.

**Blocker and unblock evidence.**
[Q11b](../../technotes/gate6-project-registration-isolation.md), 2026-10-30,
proves Copilot 1.0.103 automatically activates a Claude project registration; it
separately records native settings rewriting as preservation failure. Thus
project isolation and exact preservation remain blocked despite Q6/Q10
positives. The proposed
[Copilot local mask](../../technotes/copilot-project-plugin-overlay-profile.md)
is a hypothesis pending its calibrated unmasked/masked/remasked controls and
exact restoration, or an upstream reader-exclusion change with equivalent
evidence. User scope needs its own unintended-reader controls. Q6's persistent
caches and fresh sessions do not prove resource retention for live/resumable
sessions: before refresh/detach require observed native old-resource retention
or session clearance; otherwise keep managed revisions and operation pending.

## Copilot CLI

**Surfaces and provenance.** Approved envelopes stay `.plugin/plugin.json` and
`.plugin/marketplace.json`, with default `skills/`. **Current docs:**
[plugin reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference)
describes install/update/uninstall and enable/disable commands, native
`~/.copilot/installed-plugins` storage, platform marketplace caches,
compatibility manifest discovery and first-found skill-name deduplication. It
caps both marketplace and plugin-entry names at 64 characters. The approved
`upskill-<full SHA-256>` name has 72 characters: documented incompatibility, not
a newly executed rejection. This blocks admission pending pinned acceptance
evidence resolving the discrepancy or a reviewed collision-checked encoding; do
not truncate names.

**Current docs:** the
[configuration reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)
identifies user `settings.json`, repository `.github/copilot/settings.json` and
local `settings.local.json`; it also reads shared `.claude/settings*` plugin
keys. Repository-only `enabledPlugins` is documented to install with global
activation disabled. Repository values override user entries; managed entries
can prevent local changes. Repository declarations also reach Copilot cloud.
`config.json` contains internal/authentication state. These facts suggest a
project admission candidate without install-then-disable exposure, but do not
qualify the retained pin or authorize a new writer.

**Contract sketch.** Capture the resolved `COPILOT_HOME`, effective settings
layers, native installed-plugin representation and exact owned plugin/catalog
keys, without journaling credentials. A project contribution is itself an
activation effect and needs correct exposure ordering; reject conflicts with
managed policy or an existing manual registration. Apply native lifecycle
commands and only qualified configuration patches. Observe native install,
effective enabled scope and source-bound skill/resource reads independently;
first-found masking can hide a wrong source behind an apparently correct name.
Inverse preserves manual registrations and shared marketplaces, releases only
owned scope references and never edits native caches. D3's empty
`installedPlugins` residue allowance needs the exact qualified profile and
pre-existing-file conditions; it does not cover settings overlays or auth data.

**Blockers and unblock evidence.**
[Q3](../../technotes/gate6-copilot-plugin-components.md) at 1.0.103 is not a
skill/resource update lifecycle PASS. Q11 and Q11b retain distinct
foreign-selector and automatic-reader failures. Admit no project route until a
pinned native project-only declaration has positive activation and
unrelated-project absence before, during and after installation, plus manual
preservation and cross-client duplicate controls. Capture exact current help
against that pin before implementation; the retained
[overlay help analysis](../../technotes/copilot-project-plugin-overlay-profile.md)
is useful captured-help provenance, not proof of effective precedence. Refresh,
disable and uninstall need live/resumable cache-retention or clearance evidence;
the documented future-session enablement semantics alone are insufficient.

## Codex CLI

**Surfaces and provenance.** Approved packages retain
`.codex-plugin/plugin.json` with `skills: "./skills/"` and an item-root catalog
at `.agents/plugins/marketplace.json`; do not expose `.agents/skills`. **Current
docs:**
[official packaging guide](https://developers.openai.com/plugins/build/plugins)
supports the compatibility manifest, marketplace-root-relative local sources and
project `.codex/config.toml` plugin enablement. Trusted project settings
override user/cloud/system defaults subject to requirements; disabled plugins
may still have files refreshed. Current guidance uses the desktop app for local
installation/testing and restarting after edits, while exposing CLI marketplace
management. This differs from retained Q8's native CLI remove/re-add procedure;
do not silently substitute desktop behavior or invent `enable --scope project`.

**Retained runtime:** [Q8](../../technotes/gate6-codex-skill-updates.md),
2026-10-24, 0.164.0, observes a project binding after disabling its newly
created user binding. Resource/version edits remain stale until native
remove/re-add. It proves neither absence of transient user-wide activation nor
unrelated-project isolation.
[Q7](../../technotes/gate6-codex-agent-registration.md) retains a separate
preservation failure and fixture-specific correction; that is no production
trust/notice repair policy.

**Contract sketch.** Capture effective `$CODEX_HOME/config.toml`, project
`.codex/config.toml`, exact marketplace/plugin selectors, project trust and
inventory source/cache identities. Do not own whole plugin/trust/notice tables.
Apply native catalog/plugin commands only under a qualified exact command
profile; no user activation followed by disable as project admission. Observe
install effect separately from effective binding, intended namespace/version,
cached resource hash and scoped fresh-session expansion. Refresh may require
detach-before-attach with explicit temporary unavailability; preserve surviving
references and check all intermediate scopes. Inverse uses the recorded owned
native delta and baseline selectors. D4 allows no configuration residue; drift
in unrelated notice/trust values stops reconciliation. TOML span-preserving
writer/parser admission remains separate.

**Blockers and unblock evidence.** Require pinned current help and native
inventory proving a CLI route compatible with the approved packaging; exact full
name/hash-version acceptance; a disabled-before-registration or equivalent
project-only sequence with unrelated-project witnesses throughout; source-bound
refresh and inverse preservation. Ordinary trust refusal must be recoverable
without bypass. User scope requires independent qualification. Neither retained
exports nor Q8 cache paths prove live/resumed resource survival: obtain native
retention or session-clearance observations before refresh/removal, retaining
files and incomplete state otherwise.

## OpenCode CLI

**Surfaces and provenance.** Approved delivery uses an absolute selected
revision `skills/` source in a machine-local, ignored and untracked
`.opencode/opencode.json` overlay, or qualified effective user config. No
runtime JavaScript plugin is needed. **Current docs:**
[skills](https://opencode.ai/v2/docs/skills) says explicit sources merge across
configuration files; relative paths use active CWD; duplicate IDs select the
later registered source. Compatibility roots include `.claude/skills` and
`.agents/skills`, while explicit sources load later. Resources have a
skill-local base, and loading obeys skill permissions. The
[configuration reference](https://opencode.ai/v2/docs/config) documents global,
project and nested config layering, with system/managed overrides. Absolute
paths solve CWD dependence but do not prove that the proposed overlay is read.

**Contract sketch.** Capture every effective config layer, environment override,
explicit skills array, duplicate ID and reader reaching exports. Own exactly one
absolute source entry, preserving comments/order and other entries; reject a
tracked overlay or an ambiguous duplicate. External XDG roots remain blocked
until qualified as additional roots. `apply` is a file-only exposure under the
coordinator's file checks; configuration publication can activate a source.
`observe` independently establishes effective source selection, scope, unique
ID, fresh invocation and relative resource read, including nested CWD. `inverse`
removes only the exact owned array contribution with poststate checks and
restores original file absence/mode when applicable. No approved residue
allowance or JSONC serializer follows from this study.

**Blockers and unblock evidence.**
[Q5 retry](../../technotes/gate6-opencode-explicit-paths-retry.md), 2026-10-25,
2.1.4, stops on HTTP 429 `provider.free-limit`; downstream lifecycle and
skill/resource controls are NOT RUN. A catalog listing does not repair the
failed positive. A separately authorized future run needs verified allowance and
an available permitted model/provider, then a successful explicit-source
positive before negatives. Also require effective overlay loading and ordering,
nested-CWD and unrelated-project controls, exact preservation and live/resumable
resource retention or clearance. HTTP catalog version/cache rules in current
docs are a different route and do not qualify local-directory refresh. No failed
profile was retried here.

## Offline acceptance outline

These are proposed scenario obligations for the main lane, not new tests or
results. Start acceptance tests with a fake adapter and real durable store;
assert unchanged manual bytes/modes/presence and retained evidence at each stop.

1. Missing executable, unsupported version or command, malformed inventory and
   trust refusal reject preparation or yield an explicit unresolved outcome;
   none becomes successful installation. No trust/authentication mutation is
   inferred from permission to install.
2. Project admission never enables user scope at an intermediate step; another
   project observes absence throughout. A configured unintended reader, manual
   same-name skill or duplicate manifest/source rejects before activation.
3. Nested CWD resolves the same qualified project and absolute OpenCode source;
   tracked overlay, external unqualified XDG root and duplicate JSON/TOML keys
   reject. Existing user/project/manual layers survive scope overlap/removal.
4. Edited owned resources, equal-byte unowned files, concurrent settings edits,
   changed modes and native rewrites stop without adopting or overwriting them.
   Check D2/D3 residue separately from exact restoration and D4's no allowance.
5. Interrupt before invocation, after effect, before observation persistence and
   after acknowledgement. Timeout-after-effect requires proven native-worker
   quiescence before observing `After` and avoids replay; unknown liveness stays
   unresolved. Observed `Before` waits for recovery disposition; malformed,
   divergent or unavailable observation remains Pending. Wrong operation,
   profile revision, scope or selection cannot acknowledge a step.
6. Failed inverse and partially completed remove/re-add retain journal, files
   and ownership. Shared catalog/direct/bundle references remain available;
   removing a replacement never restores an old binding automatically.
7. Fresh sessions see only the selected revision; live and resumable sessions
   with unknown cached resource use prevent reclamation and completed removal.
   Registration absence plus intact exports cannot establish native retention.

## Bounded qualification outline

Qualification remains separately authorized. Select one exact client/scope/OS
profile at a time; serialize native profile mutation or prove isolation. Before
any model call, recheck executable/version/help, ordinary authentication/trust,
live account allowance/reset and permitted model, and allocate a finite call
budget with reserve thresholds verified then. This report sets no price, reserve
amount or authorization to spend. A failed positive, contaminated baseline,
missing attribution, config drift or exhausted allowance stops the cell; changed
provider preconditions require a new explicit execution decision.

Use neutral immutable self-contained fixtures, independent Git project roots,
manual settings/registration sentinels and distinct markers absent from prompts.
First prove intended native skill and resource activation. Then retain this
receipt order with distinct observer process/session IDs and timestamps:
calibrated observer positive and cleanup; selector-free baseline absence; owner
registration completion; owner positive; unintended-reader and unrelated-project
candidate observations; update/source-version and duplicate controls; native
removal completion; fresh post-removal absence; final preservation witness.
Require correlated catalog, load path, expansion, tool/result and resource hash
evidence, not only marker responses or counts. Failed positives never qualify
absence. Keep user/project overlap and surviving references as separate cells.

Only where the positive passes, add the minimum refresh/detachment cases for
fresh, running and resumed sessions and capture old/new resource paths. Prove
native retention or clearance before destructive native transitions, not merely
before deleting exports. Cleanup uses native commands, never registry/cache
patches; failed cleanup blocks later absence and completion. Report exact
protected preservation, allowed residue, native cache/log/session effects and
unknown effects separately. Retain packages and recovery evidence until every
required cleanup/retention condition is established. Public evidence contains
non-secret selections/digests; D6 continues to exclude credentials.

## Handoff

**Ready after the coordinator interface settles:** implement captured-help
command fixtures, profile rejection checks, exact selector capture/inverse
planning and the offline scenarios above against the existing trait. Bind each
observation to both native effect relation and final effective selection. Stage
5 durable recovery/completion tests remain main-lane work.

**Main-lane decisions:** select concrete profile revisions and exposure order
for configuration-triggered activation; approve exact non-secret selector and
span-preserving writer contracts; resolve the Copilot 72/64 name discrepancy
without changing names silently; choose whether to pursue the existing local
mask study and documented repository-only Copilot route; reconcile current Codex
surface guidance with a pinned CLI profile. Preserve existing design authority
and profile-approval requirements.

**Blocked on upstream behavior or runtime evidence:** Claude/Copilot
registration isolation needs reader exclusion or a qualified configured mask;
native rewrites need passing exact preservation within approved residue bounds;
Codex project admission needs no transient user exposure; OpenCode needs an
available authorized positive plus overlay/source controls. All four need
full-name/hash-version acceptance, scope/platform-specific qualification and
native live/resumable retention or clearance. Each missing witness blocks only
its profile; current docs and workarounds cannot replace it. No Stage 6
admission or PASS follows from completion of this study.

## Main-lane reconciliation on 2026-11-01

The report was integrated into `feat/native-delivery` as `77c1d9b`, following
implementation head `997ffce`. This is a later integration snapshot, not an
extension of the original study's runtime evidence. Stage 5 coordination is
implemented in `cf2f877`; its review and release boundaries remain those in the
delivery plan. Stage 6 already includes the closed admission registry, bounded
process runner, Claude metadata decoding/reading and command planning,
Copilot/Codex user command planning, and OpenCode source planning. These are
partial infrastructure, not complete adapters or qualified profiles. The
existing trait retains the study's operation seams and now explicitly requires
non-secret payloads and native-worker quiescence before stable observations.

The main lane has unrelated uncommitted test changes. Preserve their ownership
and finish their verification/review there rather than starting a second
implementation of those pieces. The next evidence-gathering task is the existing
[Claude local-catalog contract probe](../plans/2026-11-01-claude-local-catalog-contract-probe.md),
which tests the generated full name/version, native local-catalog record shape,
project scope and exact preservation. It remains prepared pending explicit
native-execution approval; this integration does not execute it or authorize
provider spending. Its bounded result can inform subsequent capture/inverse
fixtures and effect-observation work, while runtime isolation, retention,
complete adapter acceptance and production admission remain separate gates.
