# Q11 follow-up: intended-client registration and unintended discovery

Date: 2026-10-25. Proposed control design under
[#421](https://github.com/driftsys/upskill/issues/421), after merged
[Q11 PR #420](https://github.com/driftsys/upskill/pull/420). This document
schedules no native run and adopts no route or weaker entry gate. Q11's existing
native owner retains responsibility for further execution. Production stays on
schema 1; Q5 remains deferred ([handoff][handoff], [Q11][Q11]).

## Result being preserved

Claude 2.1.308 on macOS arm64 passed its native package positive and a clean
selector-free baseline, then cataloged and invoked a Copilot-shaped package
through the explicit session `--plugin-dir` selector. The matching package bytes
and modes stayed unchanged. This is a bounded isolation FAIL, not a failed
positive or proof of automatic discovery. Copilot 1.0.103's reverse direction
remains blocked: its slash retry loaded content but failed the exact untrimmed
response contract. Its foreign candidate did not run. Neither result qualifies
marketplace scope, IDE surfaces or other clients ([Q11][Q11],
[Claude summary][claude-summary], [Copilot summary][copilot-summary]).

Q11's packet is merged and its technote indexed. That makes the packet done; it
does not satisfy slice 2's retained PASS criterion. Keep
[#410](https://github.com/driftsys/upskill/issues/410) open while its required
qualification is unresolved. Q10's separate bounded overlap PASS cannot supply
an unintended-reader negative ([packet completion][handoff],
[entry criteria][handoff], [CD-03/CD-04][contract]).

## Question for the narrower control

Proposed Q11b asks whether registration only in the intended client exposes the
same logical skill to an unintended client in that workspace through existing
discovery. Unlike Q11, it supplies no foreign package selector or registration
to the unintended client. These are different discovery conditions. An explicit
foreign selection must still be classified using Q11's failure, never presented
as excluded by package format, directory name or separate marketplace listings
([Q11][Q11], [reader policy][contract], [preflight design][preflight]).

My proposed first profile is Claude-owned marketplace registration observed by
Copilot CLI, because the handoff starts skills with Claude Q6. Its opposite
Copilot-owned profile needs its own positive and is independent. Project and
user registrations are separate cells; neither inherits session-selector or Q10
overlap results. Pins are captured again before execution. Installed versions
that differ from Q11 create a new campaign revision ([handoff][handoff],
[Q6][Q6], [Q11][Q11]).

## Acceptance and sequence for each cell

These are planned controls, not observed results. Extend the existing
`scripts/qualify-plugin-rules.py` and offline tests first. `just lint` must fail
for the new classification before the harness change and pass afterward; retain
native receipts, summary, hashes and private-source index in a separate packet.
Raw source and test model identifiers remain outside the repository
([handoff][handoff], [Q11][Q11]).

1. Record the intended owner, unintended observer, OS, both client pins,
   registration verb, scope, effective roots and native package identity.
   Inventory existing project/user imports, globs, ancestor/nested roots,
   compatibility readers and known IDE/cloud surfaces. Unknown relevant readers
   block the applicable claim. Do not change discovery preferences or infer
   isolation from the absence of a registration ([reader policy][contract],
   [preflight design][preflight]).
2. Calibrate both clients independently using their own supported route and the
   same skill/resource bytes for that cell. Obtain a native catalog identity,
   exact namespaced invocation, correlated successful load and exact response. A
   failed positive blocks that cell. Retain Copilot's earlier failed trials; do
   not trim their output or change the response rule after observing a run
   ([Q11][Q11], [CD-03][contract]).
3. Remove calibration registrations through native cleanup and record the
   resulting state. In the same observer profile and workspace, run a baseline
   with the candidate absent, no foreign selector and no candidate registration.
   Require catalog absence, explicit namespaced not-found evidence and no load
   of that logical skill. A marker-free unrelated conversation is insufficient
   ([Q11][Q11], [reader policy][contract]).
4. Install the candidate only through the owner's supported native marketplace
   route, at the cell's stated scope. Verify its intended native identity and
   invocation. Run the observer in the same workspace with no foreign selector
   or candidate registration, while leaving all existing discovery preferences
   unchanged. Capture catalog plus explicit namespaced invocation and native
   load records. If a compatibility alias exists, exercise it too; identity is
   the logical item and its bytes, not only one command spelling ([Q6][Q6],
   [Q11][Q11], [CD-03/CD-04][contract]).
5. Remove the owner's registration through native cleanup, repeat the observer
   check and verify measured file presence, bytes, modes and registration state.
   Retain native cache/log effects and approved empty residue separately. Never
   write directly into a client cache or create team guidance files. Concrete
   preservation profiles still need approval; policy-level D1-D6 approval does
   not supply an unmeasured adapter contract
   ([preservation contract][preservation], [Q11][Q11]).

Each baseline, installed-candidate and post-removal observation starts a fresh
observer process and native session, with distinct recorded session IDs. The
baseline starts after calibration cleanup completes; the candidate starts after
the owner's native install completes; the final observation starts after native
removal completes. Retain correlated operation completion and process/session
startup receipts. Do not reuse a running or resumed baseline session to observe
a registration change: a startup-cached catalog could hide the candidate.
Missing freshness or ordering evidence blocks a clean absence result. An already
calibrated, correlated candidate activation remains FAIL even when subsequent
cleanup or post-removal evidence is missing or invalid ([Q11][Q11],
[Claude summary][claude-summary], [Q6][Q6]).

For the observer, PASS requires a successful calibration, clean baseline, known
applicable reader conditions, no candidate catalog exposure, explicit not-found
outcomes for its relevant identities, and no correlated activation. Catalog
exposure without a successful invocation is not a clean isolation PASS; retain
it as exposure and block the claim until resolved. Any unintended activation is
FAIL. A failed positive, ambiguous invocation, incomplete inventory or unknown
candidate prerequisite blocks that observation. Once a valid candidate
activation establishes FAIL, later unknown cleanup or post-removal outcomes do
not change it. Preservation is classified separately; failed preservation
prevents accepting the profile even if activation is absent
([reader policy][contract], [CD-01/CD-03/CD-10][contract], [Q11][Q11]).

## What a future PASS could establish

A retained Q11b PASS would establish only unintended discovery absence for its
recorded owner registration, observer, effective configuration, scope, versions
and OS. It would not prove that the observer rejects a package explicitly
selected later, that another marketplace entry is inaccessible, or that all
IDE/hosted readers are absent. Configured broad imports or explicit foreign
selectors must be included in preflight; known activation is unsupported for an
excluded audience. An unresolved possible reader blocks admission, without
rewriting team settings or declaring it intended on the user's behalf
([Q11][Q11], [reader policy][contract], [preflight design][preflight]).

Decision required: the user must decide whether a successful Q11b profile can
satisfy a revised slice-2 entry gate limited to intended-client registration and
its inventoried readers. Until an explicit decision and amendments to CD-03,
CD-11, AD-0040 and this handoff where necessary, the existing Q11 retained PASS
gate remains. The proposed restriction cannot silently replace the explicit
foreign-package challenge. The implementing maintainer prepares that separate
adoption PR after the new evidence, with the user as adoption owner
([handoff][handoff], [CD-03/CD-11][contract], [AD-0040][direction]).

## Work after this design

The Q11 native owner reviews the concrete profile and preservation inventory
before any run, writes the failing offline tests, then retains one packet per
bounded profile. The first Copilot observer needs a fresh successful native
positive before its baseline and candidate. The reverse direction and each
additional scope stay separate, not implied by one success. No Q5 retry,
production route change or package-default adoption follows from this document
([handoff][handoff], [Q11][Q11], [preservation contract][preservation]).

[handoff]: 2026-10-23-customization-delivery-implementation-spec.md
[contract]: ../../topics/0009-customization-delivery.md
[direction]: ../../adr/0040-separate-customization-delivery-from-discovery.md
[Q11]: ../../technotes/gate6-cross-client-package-isolation.md
[claude-summary]: ../../technotes/evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry/summary.json
[copilot-summary]: ../../technotes/evidence/2026-10-25-cross-client-package-isolation/copilot-slash-retry/summary.json
[Q6]: ../../technotes/gate6-claude-managed-root.md
[preflight]: 2026-10-25-preflight-discovery-inventory-design.md
[preservation]: 2026-10-25-customization-preservation-contract.md
