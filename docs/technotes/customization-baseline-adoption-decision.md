# Customization architecture decision recap

Date: 2026-10-31. Task [#452](https://github.com/driftsys/upskill/issues/452).
Status: proposed; adoption pending. The full proposal is
[AD-0041](../adr/0041-separate-client-representations-as-design-baseline.md).

## Proposed direction

Use one authored source and generate a separate representation for each intended
client. Each item has one effective representation per client, with its
supporting resources. A client-specific body replaces its default rather than
loading alongside it; a default need not exist.

Use native delivery where it satisfies the required capabilities. Separate
outputs do not imply universal plugin loading. Sharing is an explicit, qualified
exception, including the existing one-copy unscoped Claude/Copilot rule policy.

## Why

Separate outputs require fewer shared-default suppression and fallback rules.
They cost more copies and native lifecycle work. This is an organizing choice,
not proof of runtime isolation. The
[A/B comparison](customization-delivery-policy-comparison.md) retains the
alternatives and their evidence.

## Lifecycle

Track source identity, hashes, ownership and direct/bundle references. Preserve
manual content, local edits, surviving owners and native state through install,
update, removal and recovery.

Removing a selected client replacement makes it unavailable. Loading a default
instead requires an explicit, qualified selection; there is no automatic
fallback. Existing schema-1 behavior is unchanged by this proposal.

## Boundaries and next step

Exact installed paths and packaging remain implementation decisions. Native
qualification, preservation requirements and the four-client release gates stay
in force. An unsupported route remains blocked after design adoption.

After adoption, reconcile the contract, roadmap and active plans, then design
concrete paths and packaging. Skills/resources remain first, followed by rules
and automatic maintenance. New owned-subagent MVP scope and other pending scope
changes require separate decisions.
