# Proposed Copilot-local plugin exclusion profile

Date: 2026-10-31. Task [#446](https://github.com/driftsys/upskill/issues/446).
Status: source-backed profile; native execution is not implemented by this
layer. No model, registration or configuration command ran while preparing it.
Only version/help commands were captured. [PR #444][comparison-pr] merged the
comparison; it did not adopt A, a new audience or a narrower entry gate.

## Question and changed condition

Can an explicitly provisioned, per-plugin Copilot-local override prevent Copilot
from activating a Claude project registration, while Claude still uses that
package and an unrelated Copilot positive still works?

[Q11b][q11b] failed without that override. This proposal adds one
operation-owned key in a disposable project's
`.github/copilot/settings.local.json`. Its `enabledPlugins` map contains
`<owner-plugin>@<owner-marketplace>: false`. The key targets a fresh fixture
plugin, not a manual or pre-existing plugin. This is a different configured
profile, not a correction or rerun of Q11b. No directory name, manifest shape or
recorded failure is treated as isolation.

## Source and local inspection

GitHub's [installation guide][install-guide] describes disabling a repository
default through a `false` entry in the Copilot-local settings file and starting
a new session. It says not to commit that local file.

The [configuration reference][config-reference] documents `.claude` settings as
a cross-tool source, per-key plugin maps and managed-policy precedence. These
facts make the override a credible lead; they do not prove the pinned
executable's cross-file merge or activation behavior.

The [plugin reference][plugin-reference] distinguishes repository overlays from
global enable/disable commands. An activation controlled by a repository overlay
may reject a global toggle. Therefore this proposal does not substitute
`copilot plugin disable` for the stated local key.

Those primary pages were inspected on 2026-10-31. Rolling documentation is not
native qualification. Installed Copilot 1.0.103 help confirms `config --local`
and JSON plugin/skill inventory options. The [help receipts][receipts] retain
command identities, exits and stdout/stderr hashes. They establish syntax only;
no effective setting, discovery or permission result was measured.

## Scope and unresolved policy

Target profile: macOS arm64, project scope, Claude 2.1.308 owner and Copilot
1.0.103 observer. These are proposed execution pins based on Q11b; only the
Copilot help/version pin was checked for this note. Capture all four client
versions again before a future run and open a new campaign revision on drift.
Codex/OpenCode, Linux, user scope and IDE/cloud behavior remain unqualified.

This proposes an opt-in provisioned profile. It does not authorize upskill to
write real local settings or suppress existing discovery. Approval must cover
the new owned key, fixture file, measured boundary and credit ceiling. Any
managed requirement, pre-existing key, manual registration or ambiguous reader
blocks effects. No existing preference is overwritten or silently adopted.

A future configured-profile PASS would not satisfy Q11's retained explicit
foreign-selector gate. That gate, production schema 1 and the four-client
release requirements remain unchanged. Any prospective gate amendment is a
separate user decision, even if this experiment later passes.

## Fixture and selectors

Prepare one neutral disposable Git project outside existing discovery roots. Use
separate self-contained owner and observer-calibration packages, with fresh
identities and the same portable skill/resource bytes. Use a common bare skill
name only while one calibration/candidate route is active at a time. The Copilot
calibration package has a distinct plugin identity so the owner key cannot
disable its positive. Record that identity difference explicitly.

Declare these values before effects: project, managed source paths, owner
plugin/marketplace key, calibration identity, namespaced/bare skill identities,
source/resource bytes and modes, and unpredictable private response markers.
Native load paths must bind to the declared source or a byte-matching native
cache. A cache match also requires native origin receipts binding its plugin
key/version and captured source; equal bytes alone cannot select the right
package. Marker text is absent from requests. No team instruction files,
symlinks, hooks, MCP declarations or native-cache writes are created.

The proposed local contribution is:

```json
{
  "enabledPlugins": {
    "OWNER-PLUGIN@OWNER-MARKETPLACE": false
  }
}
```

This illustrates the owned contribution, not a replacement for an existing file.
The pre-existing fixture local settings file contains a supported
`companyAnnouncements` sentinel and an empty `enabledPlugins` map at mode
`0600`. The empty map is manual baseline and is never removed. Add/remove only
the fresh owned entry through a guarded fixture edit, preserving unrelated bytes
and metadata. After release, the file must match its initial bytes/mode exactly.
Real user settings and authentication stores are not copied or exported. An
absent-file profile is separate work. The Claude project settings fixture uses a
nonempty sentinel and existing empty plugin/marketplace maps in the native JSON
formatting, mode `0640`. Pin its exact initial bytes before execution. This
deliberately does not qualify arbitrary compact/manual formatting: Q11b's failed
compact-file preservation remains failed, and no reformatting of an existing
real file is authorized.

The diagnostic command shapes below are supported by the captured help. They are
a proposed future sequence; none was executed here beyond `--help`:

```bash
copilot --no-auto-update config --local --list --json
copilot --no-auto-update plugin list --json
copilot --no-auto-update skill list --json
```

Config readback can establish the local file's declared value, not effective
precedence by itself. Plugin inventory may retain a disabled plugin row; that is
not an available skill and is not an absence PASS. Skill catalog, actual
invocation and native load receipts decide activation.

For owner effects, reuse the recorded [Q11b native verbs][owner-receipts]:

```bash
claude plugin marketplace add "$OWNER_MARKETPLACE_DIR" --scope project --json
claude plugin install "$OWNER_PLUGIN@$OWNER_MARKETPLACE" --scope project --json
```

Use native project uninstall and marketplace remove for inverse effects.
Observer candidate phases receive no `--plugin-dir` or observer-owned
registration. The observer's own native package selector is allowed only in its
independent calibration, and is gone before the baseline. No installed package,
source or reader changes during the masked/unmasked/remasked trio.

## Preflight before any model process

1. Validate pins, source manifests, source/resource hashes, neutral project and
   complete known reader inventory. Hold both isolated-profile locks. Require
   normal authentication/trust; never copy credentials or bypass trust.
2. Check the fresh owned key has no pre-existing owner or managed override.
   Inventory `.claude` and Copilot project/local settings, ancestors, imports,
   compatibility roots and aliases. Missing effective-policy evidence blocks a
   clean absence claim. Existing manual conflicts stop the run.
3. Record exact protected files and declared native effects under approved
   [D1-D6][preservation]. Prepare the owned fixture-only local contribution,
   capture its readback and guard every later edit against unexpected drift.
4. Validate supported catalog commands and retained event shapes offline before
   spending credits. A diagnostic mismatch stops; it is not an isolation PASS or
   permission to retry a model. Native registration/inventory can itself have
   effects, so these steps also require the approved execution profile.
5. Pin private model selections and per-process credit/spend caps, and approve
   the complete ceiling below. No inherited default model or automatic process
   retry. If the remaining allowance is insufficient, do not start a partial
   paid sequence.

The capture runner does not yet implement this profile. Before any execution,
add failing offline examples for local-map conflicts, drift, event correlation,
phase ordering, the reversal and preservation. Extend the existing qualification
runner rather than introducing a third native harness.

Model processes expose only native skill invocation and the narrowly scoped read
operation required for the declared fixture resource. Record effective
permissions; shell, writes, child delegation and built-in MCP tools are denied.
Do not grant temporary-directory-wide reads or run hooks from unreviewed
profiles. If the pinned client cannot enforce the required read/tool boundary,
stop as unsupported rather than replacing it with prompt prose.

## First offline planning layer

Task [#448](https://github.com/driftsys/upskill/issues/448) adds a pure planning
mode to the existing qualification script:

```bash
python3 scripts/qualify-plugin-rules.py --plan-copilot-overlay PACKET.json
```

The packet is captured, non-secret fixture data. The CLI reads it and prints a
JSON plan; it does not write a settings file or invoke any client. Invalid
packets exit 2. All outputs contain `native_run_authorized: false`, including a
complete eight-step planning prefix. These are not native observations or
isolation verdicts.

**Edit packets** contain exactly `kind: "edit"`, `baseline_utf8`,
`current_utf8`, `plugin_key`, `action` and integer `mode: 384` (0600). Baseline
is canonical two-space UTF-8 JSON with a final newline, exactly a nonempty
`companyAnnouncements` string list and an empty `enabledPlugins` map.
Unsupported formatting/content, duplicate keys, nonfinite values, pre-existing
plugin entries, invalid identities, stale current bytes and changed mode reject.
The plugin key has exactly two components separated by `@`; each is 1–64
characters, starts with a lower-case letter or digit and otherwise contains only
lower-case letters, digits or hyphens. `mask`/`remask` require baseline current
bytes; `unmask`/`release` require the exact planned masked bytes. The
`fixture_edit_plan` output supplies `updated_utf8` and its `sha256`; only the
declared map content changes. No generic settings writer or actual
filesystem-mode/ownership check is implemented.

**Phase packets** contain exactly `kind: "phase"`, `prefix`, `attempts` and
`budget`. Prefix is a planning input, not evidence that a phase succeeded. It
must follow the eight named phases below. Attempts must be the same integer as
prefix length; retries, reordered/skipped steps and a ninth attempt reject.
Budget must contain exactly integer `process_limit: 8`, `copilot_max_ai_credits`
and `claude_max_budget_usd`; both per-client caps are explicit positive decimal
strings, never defaults. The output selects the next client's declared cap and
expected overlay/owner condition. Its `phase_plan` includes `planning_complete`
and `next_phase`; after all eight steps it returns completion with no next phase
or declared cap. It cannot verify real approval, remaining account quota, native
cap enforcement or billed usage.

The pinned CLI's
[limits help receipt](evidence/2026-10-31-copilot-overlay-offline-planning/limits-help-receipt.json)
reports a minimum session limit of 30 AI credits and post-response soft-cap
accounting. A positive planner amount is not necessarily a usable native limit.
Small amounts in tests are synthetic schema/round-trip inputs, not an approved
execution allowance. Before native admission, validate the actual client's
minimum, check live account usage/reset times, leave the agreed reserve, and
account for response overshoot. A CLI session cap alone cannot protect the
account reserve; unknown or insufficient headroom blocks execution.

The pure APIs are
`plan_copilot_overlay_edit(baseline, current, plugin_key,
action, mode) -> bytes`,
`plan_copilot_overlay_phase(prefix, attempts, budget)
-> dict` and
`replay_copilot_overlay_plan(packet) -> dict`. File application, qualified
roots/locks, native event/resource correlation, actual phase completion,
managed-policy checks, persistent attempt accounting and native-effect recovery
remain separate unimplemented boundaries. Passing these guards never admits a
paid native process or substitutes for the required native replay.

The [implementation](../../scripts/qualify-plugin-rules.py) and
[offline acceptance tests](../../scripts/tests/test_copilot_overlay_planning.py)
are the executable references for this layer. The archived
[spec](../archive/specs/2026-10-31-copilot-overlay-offline-guards.md) and
[plan](../archive/plans/2026-10-31-copilot-overlay-offline-guards.md) retain its
working scope and execution ceremony. Existing Q11/Q11b classifiers are
unchanged.

The
[offline validation record](evidence/2026-10-31-copilot-overlay-offline-planning/validation.json)
retains historical RED-log hashes/summaries, current source hashes, suite
results and reproducible mutation definitions/results. Intermediate RED source
snapshots were not retained; the record does not invent them or establish native
results.

## Native sequence and process ceiling

At most **eight fresh model processes**, with no automatic process retries. This
is a process ceiling, not a promise of eight billable API requests; tool round
trips may use more than one request. Native client limits and actual usage
receipts must enforce the separately approved credit/spend allowance. Each step
starts only after preceding native operations and observations finish.

1. **Copilot positive, mask active.** Load only its native calibration package.
   Require a correlated skill invocation, correct source identity and relative
   resource read, exact response and a complete native catalog. The owner's
   `false` entry is already present; an unrelated native route must work under
   this effective configuration. End the calibration session/selector.
2. **Claude positive, mask active.** Register/invoke the owner package through
   the project marketplace and verify namespaced identity, source/resource reads
   and exact response. Native uninstall/remove completes before baseline.
3. **Copilot baseline, mask active, owner absent.** Fresh selector-free process:
   target skill catalog absent, genuine native not-found, no load/resource read
   and exact absence response. Contamination blocks the remaining sequence.
4. **Claude candidate, mask active.** Register the same owner package, retain
   native scope/inventory and verify fresh intended invocation/resource access.
5. **Copilot masked candidate.** Same observer configuration as baseline, owner
   active. Require target catalog absence, actual not-found, no target content
   or resource activation, exact absence response and no unexpected tools.
6. **Copilot unmasked reference.** Temporarily remove only the newly owned local
   map entry, exposing the inherited Claude default; preserve the pre-existing
   map, owner registration and all other fixture bytes/settings. Do not add a
   Copilot-local `true` auto-install declaration. A fresh selector-free process
   must catalog and invoke the exact owner source and read its resource. If it
   cannot, the proposed selector's effect is unproven; retain a blocked result
   rather than a clean isolation PASS.
7. **Copilot remasked candidate.** Reinsert only that owned entry as `false`. A
   new observer session must again meet the masked absence contract. Unknown
   configuration/startup state blocks the result; catalog reuse is invalid.
8. **Copilot post-removal, mask active.** After native owner uninstall/remove
   completes, require fresh absence. Then release the owned local entry and
   verify exact fixture restoration plus separately declared native outcomes.

Every phase needs distinct process/session IDs and ordered timestamps. Link
call, completion, load, response and resource-read IDs; bind catalogs and
startup configuration to that native session. Do not trim final responses or
infer actual settings from a controller summary. Read protected paths again at
the end; stale before-receipts cannot stand in for an unobserved final witness.

Stop before the next paid process on failed/ambiguous positive, contaminated
baseline, exposed masked skill catalog, configuration drift, source mismatch,
missing tool/event evidence or exhausted allowance. Retain bounded activation as
FAIL when it is established. An earlier partial masked absence cannot become
PASS if the unmasked positive or remasking is missing. Native cleanup runs on
all exits; cleanup failure blocks later absence claims and leaves the operation
incomplete. Observe whether unmasked discovery created a Copilot-side binding or
cache: do not infer that no observer command means no native side effect. Any
new attributable binding needs a safe native inverse; ambiguous ownership blocks
cleanup and reclamation. Never edit a cache or registry directly to manufacture
cleanup.

## Results and preservation

**PASS for this proposed profile** requires both calibrated positives, valid
baseline, masked absence, source-bound unmasked activation, remasked absence,
post-removal and the approved preservation outcome. A future result qualifies
only its configured pin/scope/surface and cannot imply unconfigured isolation.

**FAIL** retains a calibrated, correlated masked activation or failed approved
preservation. Catalog exposure alone rejects the clean isolation claim, even
without invocation. **Blocked/incomplete** covers failed positives, missing
reversal, unresolved managed/local precedence or incomplete native outcomes.
Keep isolation, preservation, cleanup and evidence completeness separate.

D2's narrow Claude empty-container allowance does not authorize whole-file
rewrites; D1's native bookkeeping boundary is not whole-profile equality. The
new Copilot-local contribution has no residue allowance: preserve the unrelated
sentinel, siblings, file presence and `0600` mode, then return exact initial
bytes. Use native operations for registries/caches; observe their remaining
state, manual dependencies and unknown effects separately. D5 retains managed
bytes while resumed/live resource use is unresolved. This profile does not
qualify interruption/recovery or authorize a settings writer in the product.

## Decision after review

The user approved offline preparation. Native execution requires explicit
authorization for the concrete fixture/profile and model allowance, followed by
the requested live allowance/reset checks that preserve headroom. The reserve
instruction alone does not admit a native run; all remaining capture/enforcement
guards must also be implemented and satisfied. Account figures and concrete
reserve allocations remain private; no billing setting or additional-usage
purchase is authorized. The native capture/approval/quota-enforcement path is
not implemented here, so planning output cannot admit a run. This instruction
adopts no package layout, public default, audience or replacement qualification
gate. Original Q11/Q11b results, preservation outcomes and [#410][q11-issue]
remain unchanged.

[comparison-pr]: https://github.com/driftsys/upskill/pull/444
[q11b]: gate6-project-registration-isolation.md
[receipts]: evidence/2026-10-31-copilot-plugin-overlay-reference/help-receipts.json
[owner-receipts]: evidence/2026-10-30-q11b-registration-isolation/owner-install-receipts.json
[preservation]: ../wip/specs/2026-10-25-customization-preservation-contract.md
[q11-issue]: https://github.com/driftsys/upskill/issues/410
[install-guide]: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/plugins-finding-installing#turn-off-a-repository-default
[config-reference]: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference#configuration-file-settings
[plugin-reference]: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference#cli-commands
