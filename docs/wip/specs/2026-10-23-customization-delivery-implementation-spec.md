# Customization delivery: implementation handoff specification

Date: 2026-10-23; reconciled on 2026-10-24 against the
[plan/design review ledger][ledger]. Status: slice-1 evidence work authorized;
architecture adoption remains pending. Basis: the
[customization delivery proposal](2026-10-23-customization-delivery-proposal.md)
is a proposal, not an accepted implementation contract for its direction
amendments. [AD-0040][direction], [Topic 0009][contract] and the
[roadmap][roadmap] remain controlling. This reconciliation changes no durable
record, code or production route ([design ledger][ledger]).

Decision required: the user adopts or rejects the three direction changes at the
step 7 PR review: subagents in MVP, packages as the default instead of the
shared-root experiment, and authorable bundles outside MVP. The OpenCode
explicit skill-source proposal also needs disposition against AD-0040's third
listed amendment. Route amendments consume native evidence separately; a PASS
does not adopt these direction changes
([AD-0040 proposed amendments][direction], [roadmap][roadmap], [Q5][Q5]).

Audience: the implementing agent that takes over this branch. It knows the
repository conventions in `AGENTS.md`. It did not take part in the review
conversation, so every decision it needs is written here.

## Intent

Make the proposal implementable by first closing the native qualification gaps
that its per-client tables depend on, then packaging content on the routes that
passed. The proposal's tables cite pass and fail controls per row; several rows
still rest on documentation alone. No product code that changes production
routing is written until the controls that gate that code have run.

## Goals, non-goals, anti-goals

Goals:

- Every route in the proposal's rules, skills and subagents tables has a pinned
  PASS, a pinned FAIL, or an explicit blocked/unsupported disposition before an
  adapter is written. Failed positives leave downstream controls not run, not
  negative passes ([CD-01 and CD-10][contract], [Q5][Q5]).
- Each control is retained the way the existing Gate 6 packets are retained, so
  the proposal's evidence section can cite it.
- The durable records listed in the proposal are amended once, after slice 1,
  from evidence rather than from documentation.

Non-goals for this handoff:

- The updater plugin, bundles as an authorable type, MCP management, hooks,
  tools and permissions as managed categories. They keep their roadmap place.
- Linux, IDE and hosted surfaces. Record them as unqualified; do not test them
  in slice 1.
- A new include or composition syntax for `RULE.md`, `SKILL.md`, `AGENT.md`.
- Lockfile schema 2 activation. Production routing stays schema 1 throughout.

Anti-goals, which make the work a failure even if tests pass:

- Any write to `CLAUDE.md`, `AGENTS.md` or a nested equivalent, including the
  shipped `@AGENTS.md` bridge creation.
- Any write into a client-owned plugin cache.
- A negative control made to pass by disabling or editing the user's or team's
  existing discovery settings.
- A test model identifier in any committed document.
- A documentation claim promoted to "qualified" without a retained pinned run.

## Constraints

- Single static Rust binary, no async runtime, no `git2`, dependency policy of
  ADR-0001. Client-side adapters may use the client's own supported runtime
  (shell for Claude, Codex and Copilot hooks; the OpenCode plugin API in
  JavaScript) and nothing else. No shared service or daemon.
- Installation is copy-only. No symlinks into or out of the managed root.
- The managed root must not be, or sit under, any documented client discovery
  path: `.claude/`, `.github/`, `.codex/`, `.opencode/`, `.agents/skills`,
  `.agents/plugins`, `AGENTS.md`, `CLAUDE.md` and their user-home equivalents.
  It must not match any `instructions`, `skills` or plugin glob already present
  in project or user client configuration.
- Each logical item has exactly one delivery route per intended client. A plan
  that selects both a native route and a package route for the same item on the
  same client is rejected.
- Reuse the existing library foundations without bypass: captured source
  evidence (AD-0019, AD-0020, AD-0030), complete selected-set preflight
  (AD-0027), qualified roots and locking (AD-0028), explicit file permissions
  (AD-0031), recovery checkpoints and journals (AD-0032, AD-0033), owner
  contracts (AD-0034). The 65 focused tests cited in the proposal's evidence
  assessment must keep passing.
- Repository workflow from `AGENTS.md`: own worktree per branch under
  `.claude/worktrees/<branch>`, acceptance tests before unit tests, Conventional
  Commits, `just fmt` then `just verify` before a PR, one PR per task, squash
  merge. Every task issue starts with `Epic: #283` and carries `task` plus
  `epic:<name>` labels.

## Slice 1: qualification of the gating routes

Each control below follows the method of the retained Gate 6 packets
(`docs/technotes/gate6-*.md`): disposable Git repositories with distinct root
commits, unpredictable positive, absent and fallback markers per control,
isolated client profiles with the user's normally provisioned authentication,
child environments stripped of inherited `GIT_*` and client override variables,
exact final-response matching, structured native event capture, and failed
positives retained as failures rather than counted as negative passes. Reuse
`scripts/qualify-native-instructions.py` and `scripts/qualify-plugin-rules.py`
patterns; extend them rather than writing a third harness where the fixture
shape allows. Offline fixture tests under `scripts/tests/` run in `just lint`
and never call a client.

Pins (my call): open a new campaign revision in
`docs/technotes/gate6-plugin-profile-inventory.md` that pins the currently
installed releases of all four clients before the first control runs. Every new
control runs on those pins. Retained cells from the 2026-10-19 campaign are not
re-run unless a step 7 decision depends on them. A table that mixes pins is
acceptable when every row names its pin; a row without a pin is not evidence.
Platform: macOS arm64 only in slice 1.

Harness scope (my call): slice 1 ships offline fixture and classification tests
per control under `scripts/tests/`, run by `just lint`. Native capture helpers
may stay private and unshipped, as PRs #347 to #380 did, provided the packet
retains receipts and hashes. A shipped automated native runner with timeout and
interruption acceptance tests is not part of slice 1.

Local installation verbs (my call): when a pinned CLI has no local-path install
verb, the control uses a local marketplace entry, as `src/plugin.rs` already
does for Claude and Copilot. For OpenCode the owned `plugins` array and the
explicit skill-source entry are the registration route; "no local install verb"
is recorded as a result, not as a failed control.

Sequencing: PR #381, including this document, merges to `main` before the first
Q issue opens. Until step 7, each technote names the proposal row it affects by
linking the proposal under `docs/wip/specs/`; step 7 repoints those links to the
amended durable records when it gardens the proposal.

| Id | Client and scope                         | Control                                                                                                                                                                                                                                                             | Passes when                                                                                                                                                                                      | Decision it informs in the proposal                                                                                                                                           |
| -- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1 | Copilot CLI, project                     | Positive: marker in `.github/copilot-instructions.md`. Candidate: a different marker only in `.claude/rules/probe.md`, no `.github/instructions`. Also run with a `paths:` frontmatter block present.                                                               | Positive returns its marker. Candidate result is recorded either way; "reads" or "does not read" is the output.                                                                                  | Establishes whether the duplicate vector exists; Q1 reads unscoped files, but cannot select (a)/(b)/(c) ([Q1][Q1], [summary][Q1 summary]).                                    |
| Q2 | Copilot CLI, project                     | Claude-format agent at `.claude/agents/probe.md` with a restricted `tools` list and a `permissionMode`. Check the Copilot agent catalog, invoke it, and record which tools the child could use.                                                                     | Catalog and invocation outcomes are recorded; a tool outside the Claude `tools` list being used is a FAIL for the "exception route is safe" claim.                                               | Subagents section: whether the native-file exception for restricted Claude subagents is usable in projects that also use Copilot.                                             |
| Q3 | Copilot CLI, project and user            | Local plugin with `com.github.copilot/rules/<name>.md` (unscoped and `applyTo` scoped) and `com.github.copilot/agents/<name>.agent.md`. Install from a local path, run positive, disable, uninstall, reinstall; matching and nonmatching file access for the scope. | Rule marker present after install, absent after disable and uninstall; scoped rule activates on matching access and not on nonmatching; agent is delegable.                                      | Rules candidate FAIL at both scopes; project agent activation unverified; bounded user child does not qualify restrictions ([Q3][Q3], [summary][Q3 summary]).                 |
| Q4 | Claude Code, project                     | Local marketplace plugin whose `SessionStart` hook injects one unscoped rule from a package-owned rule index. Positive, disable, uninstall, resume, one compaction.                                                                                                 | One rule event per fresh session and per resume; none after disable or uninstall; rule present after compaction.                                                                                 | Rules table: PASS, unscoped only; no scoped or effective-input uniqueness claim ([Q4][Q4], [summary][Q4 summary]).                                                            |
| Q5 | OpenCode, current 2.1.x release, project | Re-run the 2.0.32 plugin context-hook control, the package skill directory registered as an explicit skill source with a resource read, and `plugin add` with a local directory.                                                                                    | Context hook: one event per rule, resume included. Skill: native invocation reads the package resource. `plugin add` behavior recorded either way.                                               | Native positives FAIL at 2.1.4; component controls not run. Retry requires a successful positive ([Q5][Q5], [summary][Q5 summary]).                                           |
| Q6 | Claude Code, project and user            | Managed root outside all discovery paths, exposed through a local marketplace. Install at project scope, then user scope. Record the actual load path, the namespaced skill name, update behavior after editing the package, and settings bytes after uninstall.    | Load path, namespace, update visibility and measured settings/registry residue recorded. Q6 PASS includes persistent native cache, not whole-profile equality ([Q6][Q6], [summary][Q6 summary]). | Storage section: installation mechanics and the residue contract; Packaging section: namespacing.                                                                             |
| Q8 | Codex CLI, project                       | Local marketplace plugin with one skill and one resource. Install, invoke, edit the package, invoke again without reinstall, then `plugin upgrade` or remove and re-add, then invoke.                                                                               | Namespaced identity recorded; whether edits are visible without reinstall is recorded; reinstall restores the edited bytes.                                                                      | Storage section: update loop for cache-copying clients.                                                                                                                       |
| Q7 | Codex CLI, trusted project               | Package-owned agent TOML referenced from `.codex/config.toml` as `agents.<name>.config_file`. Invoke a real delegated child; attempt an operation the parent sandbox forbids. Separately place the same TOML inside an installed plugin and check discovery.        | Child runs, inherits the parent boundary (forbidden operation denied), plugin-contained TOML is not discovered unless registered.                                                                | Subagents table, Codex row.                                                                                                                                                   |
| Q9 | Codex CLI, project                       | Resolve [#379](https://github.com/driftsys/upskill/issues/379): resume plus two automatic compactions in one session with two plugin rules; count developer rule bodies, not callbacks.                                                                             | Exactly one developer rule body per logical rule after resume and after each compaction. Two bodies for one rule is a FAIL, whatever the callback count.                                         | FAIL: duplicate bodies after reopen + first automatic compaction. Codex rules blocked while #379 is open; user owns the step 7 disposition ([Q9][Q9], [summary][Q9 summary]). |

Additional controls close reader, overlap and trust gaps identified by the
[design ledger][ledger] under [CD-03, CD-04 and CD-08][contract]. These are
planned controls, not retained results.

| Id  | Client and scope                                      | Control and explicit outcome                                                                                                                                                                                                                                                                                                                                                                                                                         | Decision it informs                                                                                                                        |
| --- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Q1b | Copilot 1.0.103, project                              | Successful native positive first; `.claude/rules` with `paths:` and matching-file access, then separate nonmatching access. PASS for scoped consumption only if the exact rule activates once on matching access and not on nonmatching access, with native read/count evidence. FAIL for that scoped contract otherwise; record reads/does not read separately at each access. Failed positive blocks both candidates ([Q1][Q1], [CD-03][contract]) | Tests option (a)'s unproven premise; supplies evidence for the user's shared-rule decision, without selecting it ([design ledger][ledger]) |
| Q10 | One client at a time, same project plus user scope    | Install the same package at both scopes in one workspace. PASS: one catalog entry and one observed activation per logical item. FAIL: duplicate entry/activation; failed positive blocks the result. Repeat for each candidate client's pinned profile ([CD-04][contract])                                                                                                                                                                           | Cross-scope deduplication; Q6 used separate projects and does not satisfy this ([Q6][Q6])                                                  |
| Q11 | Another client pointed at a Claude or Copilot package | Require a successful positive on each other client before accepting its negative; test that client's relevant catalog and explicit invocation. PASS: no unintended activation; FAIL: unintended activation. Record imports/globs and compatibility package readers without editing existing discovery settings ([CD-03][contract])                                                                                                                   | Package isolation, including Claude-only content in shared projects ([Q1][Q1], [AD-0040][direction])                                       |
| Q12 | Each candidate client, direct plus package            | Put the same logical skill in direct `.claude/skills` discovery and a package. PASS: planner rejects before effects or native catalog/counts prove one entry and one activation. FAIL: two effective exposures/activations; a final marker alone is insufficient ([CD-04][contract])                                                                                                                                                                 | Direct/compatibility plus package duplicate rejection ([design ledger][ledger])                                                            |
| Q13 | Codex 0.164.0, ordinary reviewed hook installation    | Record ordinary `/hooks` trust, change only the package rule index, observe native trust status before invocation, complete any required user review, then invoke. PASS: trust transition and exact once-only delivery recorded with no bypass. FAIL: silent unreviewed changed definition where review is required or duplicate/missing delivery; unavailable safe observation leaves blocked ([Q9][Q9], [CD-08][contract])                         | Whether a rule-index update requires re-trust; Q9 used unchanged already reviewed definitions ([Q9][Q9])                                   |

Execution history starts Q1, Q2, Q6, then Q9 and Q3/Q4/Q5, as their packets
record. Remaining evidence work prioritizes Q1b, Q8 and the Q5 retry ahead of
Q7; Q10-Q13 follow each applicable successful route. Q8 and the Q5 retry gate
skills feasibility; Q7 does not unlock it ([Q1][Q1], [Q2][Q2], [Q6][Q6],
[Q9][Q9], [Q3][Q3], [Q4][Q4], [Q5][Q5], [design ledger][ledger]).

Current campaign pins are Claude 2.1.308, Copilot 1.0.103, Codex 0.164.0 and
OpenCode 2.1.4 on macOS arm64. Every follow-up names its actual pin and opens a
new campaign revision if that pin changes. Q7/Q8 are pending evidence review in
this document; earlier-pin successes do not replace their controls ([Q6][Q6],
[Q5][Q5], [CD-10][contract]).

Steps per control:

1. Open a task issue (`Epic: #283`, `task`, severity and effort labels) with the
   control's fixture, markers policy, pin and pass criteria from the table.
2. Write the offline fixture and classification tests first; `just lint` must
   fail before the harness change and pass after.
3. Run the native control. Retain the packet under
   `docs/technotes/evidence/<YYYY-MM-DD>-<slug>/` with `summary.json`
   (`qualified`, controls, counts, limits), `receipt-hashes.json` and
   `private-source-index.json`, following the existing packets. Raw transcripts
   and private source stay outside the repository.
4. Write `docs/technotes/gate6-<slug>.md`: method, pins, each control's PASS or
   FAIL, limits, and the exact proposal row it affects. Add it to
   `docs/technotes/README.md`.
5. Open one PR per packet ("docs: retain ..."), as PRs #347 to #380 did.
6. Restore the client profile to its pre-control configuration and verify the
   bytes; record any residue in the technote.
7. After Q1-Q9 have retained results and the applicable Q1b/Q10-Q13 gaps have
   explicit dispositions, open a separate durable-record PR covering the
   proposal's superseded-records table. The user reviews direction adoption
   separately from evidence-driven route amendments. Route status must retain
   each pin, PASS/FAIL/not run and its limits; a failed positive is not proof of
   absence ([design ledger][ledger], [CD-10][contract]).

   While Q9/#379 remains open, the Codex rules row is **blocked**, not
   unsupported and never supported. The implementing maintainer owns evidence
   collection on #379; the user owns any architecture disposition at the step 7
   PR review. The default is to retain AD-0040's intended plugin route with its
   blocker. Decision required: any change of that direction needs explicit user
   adoption and a durable amendment, not a status inferred from a failed run
   ([Q9][Q9], [summary][Q9 summary], [AD-0040][direction], [roadmap][roadmap]).

   Decision required: the user selects the `.claude/rules` policy from the
   proposal's three alternatives, consuming Q1b, Q3 and Q4 and classifying
   Claude-only rules where Copilot runs. Neither failed option (c) nor the
   untested option (a) premise closes this decision ([Q1][Q1], [Q3][Q3],
   [Q4][Q4], [AD-0040][direction]).

8. Decision required: the user confirms timing of the separate bridge/glob
   removal code PR at step 7 review. Keep step 8 after step 7 **and a retained
   Q5 PASS**; do not remove the shipped OpenCode registration on the current
   failed positives. Q5's HTTP 400 failures do not qualify a replacement
   ([Q5][Q5], [summary][Q5 summary], [design ledger][ledger],
   [shipped ancillary contract](../../specification/specification.md)).

   If the user instead approves earlier removal, the amendment must explicitly
   record that deleting the `opencode.json` instructions glob removes shipped
   OpenCode rule registration, new OpenCode rule installs are unsupported in the
   interim, and `doctor` must report that capability gap. This is a future code
   acceptance requirement, not an existing doctor claim. The code PR would
   delete `CLAUDE.md` bridge creation in `src/ancillary.rs` and the
   `instructions: [".agents/rules/**/RULE.md"]` write, with no fallback or
   migration, and update tests/specification/usage. Existing generated bridges
   and globs are reported, never automatically deleted; the VS Code ancillary
   write remains until its separately deferred profile decision
   ([design ledger][ledger], [ADR-0003](../../adr/0003-generation-pipeline.md),
   [AD-0040][direction]).

Slice 1 records evidence outcomes even when they FAIL or stay blocked. Its
implementation completion additionally requires the applicable follow-up
controls/dispositions, the user's step 7 decisions and merged durable-record PR,
and the authorized step 8 PR. No result in this WIP activates production routes
([CD-10 and CD-11][contract], [Q9][Q9], [Q5][Q5]).

## Stack and merge order

The packet stack has two dependency chains, rather than independent PRs against
main ([design ledger][ledger]):

- [385](https://github.com/driftsys/upskill/pull/385) <-
  [386](https://github.com/driftsys/upskill/pull/386) <-
  [394](https://github.com/driftsys/upskill/pull/394): merge Q1, then Q2, then
  Q3.
- [385](https://github.com/driftsys/upskill/pull/385) <-
  [387](https://github.com/driftsys/upskill/pull/387) <-
  [389](https://github.com/driftsys/upskill/pull/389),
  [392](https://github.com/driftsys/upskill/pull/392): merge Q1, then Q6, then
  Q9 and Q4. Q5 [396](https://github.com/driftsys/upskill/pull/396) follows Q4
  ([Q5][Q5]).

After each squash merge, update dependent branches/base targets and rerun the
required checks before their merges. A change request on #385 can affect both
chains. A Q packet is **done** only when its packet is merged to main and its
technote is indexed, regardless of PASS or FAIL. This administrative done status
is separate from native route qualification, completion flags inside historical
packets and closure of #379 or a retry issue ([design ledger][ledger],
[CD-10][contract]).

## Slice ordering decision

Decision required: the user must approve or reject the proposed skills-before-
rules order (slice 2 skills, slice 3 rules). It reverses the roadmap's Gate 6
rules-before-new-owned-skills dependency, AD-0040 and CD-11. The reason to
consider it is Claude Q6's passing package feasibility while Codex Q9 is blocked
and OpenCode Q5 has failed positives. Those observations justify continued
harmless probes, not implementation re-sequencing. Until adopted at step 7,
retain the controlling rules-first dependency; do not start owned-skill
implementation merely because the outline below exists ([roadmap][roadmap],
[AD-0040][direction], [CD-11][contract], [Q6][Q6], [Q9][Q9], [Q5][Q5]).

## Slice 2 outline: skills packaging on passed routes

Entry criteria: **retained PASS** for each client's applicable package,
invocation and resource controls, plus step 7 adoption and the governing
rules-first dependency unless the user explicitly amends it. Retained failures
do not satisfy entry ([design ledger][ledger], [CD-10 and CD-11][contract]).
Client order: Claude first on Q6 PASS with the bounded residue contract
approved; Codex when Q8 passes; OpenCode when Q5 passes. Copilot follows only
when its applicable route passes; Q3's failed rules and unverified project
activation do not qualify a complete route ([Q6][Q6], [Q5][Q5], [Q3][Q3]).

Scope remains a pure package generator for skills/resources, one managed root,
local native registration and add/update/remove/recovery over the inactive
owned-artifact planning foundations. Production remains schema 1; no WIP outline
adopts a production route ([AD-0040][direction], [CD-11][contract]).

Required slice-2 tasks cover the proposal's previously unassigned musts:

| Task                             | Required result and acceptance evidence                                                                                                                                                                                                                                                                                                    |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Preflight glob/import inventory  | Inventory project/user imports, globs, compatibility and nested roots, explicitly covering `.claude/rules` and `.github/instructions` as well as the managed root. Classify intended/unintended/unresolved readers and reject unsafe profiles before effects; preserve existing discovery settings ([Q1][Q1], [CD-01 and CD-03][contract]) |
| Collaborator project onboarding  | From a committed project registration, a fresh collaborator performs ordinary install/trust and invokes the intended item once. Record offline/denied behavior, native catalog/registration and unrelated-project negative; committed registration alone is insufficient ([Q6][Q6], [CD-08][contract])                                     |
| Per-client preservation contract | Specify exact bytes/modes/presence and owned keys, native pre/post registrations, allowed empty residue, persisting cache effects and recovery boundaries per client; reject unrelated changes. Obtain the user's explicit allowance before implementing it ([Q6][Q6], [Q3][Q3], [Q5][Q5], [CD-07][contract])                              |
| Per-client update visibility     | Test source edit, fresh session, running/resumed session and native refresh separately; document what loads when. Preserve Claude's no-reinstall observation and Copilot's live-source report without extrapolating to Codex/Q8 or OpenCode/Q5 ([Q6][Q6], [Q3][Q3], [Q5][Q5], [CD-10][contract])                                           |

Acceptance for slice 2 (to be refined in its own plan):

- Generating a package twice from the same captured source yields identical
  bytes.
- Installing a package exposes exactly one native skill per logical item per
  client, verified by native catalog listing, not by file presence.
- Remove disables registration before reclaiming files and preserves unrelated
  bytes/modes/keys under the approved per-client contract, recording declared
  native residue separately ([Q6][Q6], [Q3][Q3], [CD-07][contract]).
- An interrupted install or remove is recoverable from the journal plus observed
  native registration state. File-only success cannot prove native deactivation;
  unknown outcomes block replay ([CD-07][contract]).

Slices 3 (rules) and 4 (subagents) follow the same shape and are planned after
slice 1 decides their routes.

## Settled constraints and pending direction

- Preserve team guidance and schema-1 production routing; no managed `AGENTS.md`
  fallback or direct client-cache writes ([AD-0040][direction]).
- Keep one delivery per intended client/item and reject duplicates before
  effects; sharing needs explicit consent and native controls
  ([CD-03 and CD-04][contract]).
- Codex hook installation preserves ordinary user trust review. Q9's reused
  reviewed packages do not qualify fresh trust or changed-index behavior
  ([Q9][Q9]).
- Decision required: package defaults, subagents in MVP, authorable bundles,
  OpenCode skill sources, slice order and step 8 timing remain user decisions at
  step 7, separate from evidence-driven route status ([AD-0040][direction],
  [roadmap][roadmap], [design ledger][ledger]).
- Native Claude/Copilot rules remain the intended defaults where their consumer
  and scope contracts pass. Q4 is a bounded unscoped exception; Q3 fails its
  plugin rule candidate and Q9 blocks Codex rules ([AD-0040][direction],
  [Q4][Q4], [Q3][Q3], [Q9][Q9]).

## Left to the builder

- Exact managed root path, subject to the exclusion list above.
- Package granularity (per item or per resolved bundle), as long as the planner
  deduplicates logical items before export.
- The deterministic rule index format inside a package.
- Harness extension versus new script, per control.
- Issue slicing inside slice 1 beyond one issue per control.

## Open questions for the user

These decisions belong to the user at step 7 PR review, not the implementing
agent ([design ledger][ledger], [AD-0040][direction]):

- Decision required: adopt/reject the direction amendments and any
  skills-before-rules re-sequencing, independently of retained route results
  ([AD-0040][direction], [roadmap][roadmap], [CD-11][contract]).
- Decision required: choose among unscoped-only shared projects, per-item
  intended Copilot consumption of `.claude/rules`, or scoped shared rules
  unsupported. Also decide the declared/unsupported branch for Claude-only rules
  where Copilot runs ([Q1][Q1], [Q3][Q3], [Q4][Q4]).
- Decision required: retain Codex rules blocked while #379 is open, or propose
  an explicit direction amendment at step 7. No automatic unsupported status
  substitutes for that decision ([Q9][Q9], [AD-0040][direction]).
- Decision required: confirm step 8 after Q5 PASS, or approve explicit interim
  OpenCode unsupported installs and doctor reporting in the removal record
  ([Q5][Q5], [design ledger][ledger]).
- Decision required: select the Codex distribution channel for hook-bearing
  packages, which cannot use the public directory under the reviewed contract; a
  metapowers entry gives no trust. Decide the update/review policy after Q13
  establishes whether a rule-index change triggers Codex re-trust
  ([design ledger][ledger], [Q9][Q9],
  [proposal trust boundary](2026-10-23-customization-delivery-proposal.md#rules)).
- Decision required: approve per-client preservation allowances; choose explicit
  removal or report-only handling for experimental managed `AGENTS.md` regions,
  and decide updater sequencing in the roadmap amendment
  ([CD-06 and CD-07][contract], [AD-0040][direction], [roadmap][roadmap]).

## References

- [Proposal](2026-10-23-customization-delivery-proposal.md)
- [AD-0040](../../adr/0040-separate-customization-delivery-from-discovery.md)
  and [Topic 0009](../../topics/0009-customization-delivery.md)
- [Gate 6 pinned profiles](../../technotes/gate6-plugin-profile-inventory.md)
- [Gate 6 native qualification method](../../technotes/m1-gate6-native-qualification.md)
- [Evidence assessment](../../technotes/evidence/2026-10-23-delivery-choice/assessment.md)
- Issues: [epic #283](https://github.com/driftsys/upskill/issues/283),
  [task #337](https://github.com/driftsys/upskill/issues/337) (plugin rule
  delivery and isolation, still open; Q3 to Q5 extend it)

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
