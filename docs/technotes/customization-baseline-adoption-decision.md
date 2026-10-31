# Bounded customization architecture decision

Date: 2026-10-31. Task [#452](https://github.com/driftsys/upskill/issues/452).
Status: proposed decision package; no user adoption or implementation authority.

## Decision requested

Can we adopt **A: separate generated client representations, with explicit
sharing exceptions**, as the organizing design baseline now, while every
unsupported native route remains blocked?

The concrete proposal is
[AD-0041](../adr/0041-separate-client-representations-as-design-baseline.md). It
settles source/selection, sharing and lifecycle direction; installed paths,
package defaults, production activation and other pending scope decisions stay
separate. Choosing this baseline need not wait for all native qualification.
Owned implementation still waits for the existing adoption and evidence gates.

## Why this proposal

Both A and B keep one authored source. A generates the selected client
representations independently. B puts portable installed defaults in a shared
catalog and must suppress them when a client replacement is selected.

A has fewer default-selection, suppression and fallback obligations. It costs
more generated copies and native lifecycle work. This is a design tradeoff, not
an observed safety advantage. B remains useful for explicit compatible sharing.
The [comparison](customization-delivery-policy-comparison.md) retains the
four-client/kind evidence matrix and three worked examples without selecting
either layout.

## Evidence that limits the decision

- [Q11](gate6-cross-client-package-isolation.md) and
  [Q11b](gate6-project-registration-isolation.md) retain their bounded results;
  changing a directory name does not qualify isolation.
- The tested Claude project registration exposes Claude-only content to the
  unintended Copilot reader under Q11b's profile. A cannot admit that route by
  organization alone. Other pins/scopes/readers remain separately classified.
- The [Copilot overlay proposal](copilot-project-plugin-overlay-profile.md)
  changes one native selection condition. Only its pure offline planning guards
  exist; no masked/unmasked/remasked native result is established.
- OpenCode's current campaign failed positive and the Codex recorded-history
  duplication limitation remain blockers for their affected claims. A does not
  fix them; the comparison identifies the exact bounded evidence.
- Existing native-first rules, explicit one-copy Claude/Copilot rule sharing,
  skills-before-rules sequencing, preservation classes and all-four-client
  release obligations remain in force.

## Concrete behavior to approve

1. One source, copied outputs and one effective representation per item/client.
   Client-specific content replaces its default; resources follow that content.
2. Native delivery selected per kind/client, with capabilities and intended
   readers preserved. Separate outputs do not imply universal plugin loading.
3. Sharing only for declared compatible readers with qualified exclusion and
   duplicate controls. Retain the already adopted shared-rule exception.
4. Reference-based ownership and complete install/update/remove/recovery
   preflight. Preserve manual edits, surviving references and native state;
   reject unsupported/ambiguous operations before effects.
5. Removing a selected client replacement makes it unavailable by default.
   Loading a default instead requires explicit, qualified reselection.
6. Every unresolved native route stays blocked; no original gate or release
   scope changes. No generated files, native registrations or settings change as
   part of this decision package.

## Review and adoption

Astra and actual Claude CLI Fable independently assessed the same frozen
[decision packet](evidence/2026-10-31-customization-baseline-decision/review-input.txt).
The
[review record](evidence/2026-10-31-customization-baseline-decision/review.json)
retains input/output hashes, actual models and correction dispositions.

- [Astra](evidence/2026-10-31-customization-baseline-decision/astra-review.txt),
  `gpt-6-astra`, xhigh: **YES**, no required corrections. Approval must cover
  the substantive selection and removal policies as well as A's organization.
- [Fable](evidence/2026-10-31-customization-baseline-decision/fable-review.txt),
  actual Claude CLI `claude-fable-5-1`, medium: **YES WITH REQUIRED
  CORRECTIONS**. Clarify defaults under A, the explicit removal commitment and
  existing copy-only contract, internal versus general hooks, Q11b's direction
  and separate preservation failure, and non-pre-emption of pending
  scope/routes. Those five clarifications are incorporated into the proposed
  ADR.

The original verdicts apply to the frozen packet. Fable was not called again
after the clarifications; their incorporation is an author disposition, not a
new unconditional Fable verdict. A fresh builder check of the clarified proposal
reported no important unanswered planning question.

Both support adoption while unsupported routes stay blocked. Their
reconsideration examples differ: Astra requires comparative evidence across the
required matrix to overturn the baseline; Fable also identifies a required-kind
profile where only compatible sharing works. Retain that distinction. A failed
profile blocks its effects and can trigger reconsideration; it cannot
automatically adopt B or turn unintended consumption into consent.

No native qualification probe ran for this task. Account balances and budget
receipts remain private; reviewer agreement supplies no native evidence.

The user then accepts, rejects or amends the concrete ADR. A document merge or
reviewer recommendation alone does not adopt it. Until that explicit decision,
AD-0040 and Topic 0009 remain authoritative and AD-0041 stays proposed.

After adoption, reconcile the authoritative contract, roadmap and active plans
for exactly the adopted changes. Select concrete paths/packaging in a bounded
implementation design, retain unsupported routes as blocked, and qualify the
required native profiles before owned implementation. Native probe spending is a
separate conditional step under the user's reserve policy.
