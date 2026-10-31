# Four-client skills and resources delivery

Status: **approved implementation design**, 2026-10-31. This applies the
accepted
[AD-0041](../../adr/0041-separate-client-representations-as-design-baseline.md).
It does not activate schema 2 or qualify a native route. The approval record
below identifies the user-approved policies; detailed engineering mechanisms
remain subject to implementation review.

## Approval record

The user approved storage and packaging on 2026-10-31: one package per logical
skill and intended client, shared by references to that same installation;
self-contained resources; the neutral `.upskill/exports` layout; immutable
revisions; only the selected revision exposed; and safe retention of older
revisions.

The user also approved the client integration direction on 2026-10-31: native
plugins with separate manifests and local catalog registrations for Claude,
Copilot and Codex; explicit native skill-directory configuration for OpenCode;
project-local admission without transient user-wide activation; and
machine-local absolute OpenCode paths materialized separately by each
collaborator. Manual configuration remains preserved. These approvals choose
implementation direction, not successful native qualification.

The user approved the update/removal policy on 2026-10-31: stage and validate a
new revision before switching; permit temporary unavailability where needed to
prevent duplicate exposure; require native resource retention or session
clearance before refresh/detachment; preserve surviving direct/bundle
references; stop on local edits or conflicting configuration; observe uncertain
native outcomes before retrying; and provide no automatic fallback after
replacement removal. File rollback alone does not establish native recovery.

The user approved source selection and the initial metadata policy on
2026-10-31: canonical metadata and default body; existing three client body
overrides replace the default; canonical directives for Codex without new
override syntax; aliases bind native name and directory; exact resource
bytes/structure/modes; and an initial name/description/license/metadata
allowlist with target validation, rejecting nonempty native settings and unknown
fields until explicit mappings exist. Existing installation behavior remains
unchanged.

These approvals complete the design review. The staged implementation plan
remains pending approval. Native activation, schema-2 production admission and
qualification waivers are not authorized by design approval.

## Outcome and boundaries

Install one self-contained generated skill representation per logical item and
intended client, with exactly one effective exposure per client. Support Claude
Code, Copilot CLI, Codex CLI and OpenCode CLI. Keep canonical authoring sources,
relative resource references, source revisions and direct/bundle ownership.

Skills/resources come first. Existing rules and agents remain regression
obligations; new rules, owned subagents, general hooks, MCP and automatic
maintenance are outside this slice. Existing schema-1 commands stay unchanged.
No paid native experiment is part of preparing this design.

## Source selection

Capture the canonical `SKILL.md` and resource tree once before planning.
Audience exclusion removes a client before selecting its body. Canonical
frontmatter supplies identity and metadata. Existing `SKILL.claude.md`,
`SKILL.copilot.md` and `SKILL.opencode.md` replace only the body, verbatim,
without processing the canonical body's directives. Without an override, process
canonical directives for the selected target family.

Codex uses the canonical body and existing four-family directives. This slice
does not add `SKILL.codex.md`: reject that reserved-looking source filename with
an actionable error rather than copying it as a resource. Keep the historical
installer and captured compiler contracts unchanged; implement selection in a
new producer. Do not add Codex to the legacy three-client `Client::ALL` loop.

An alias changes the rendered native skill name and directory together. Protect
name and description from passthrough overrides. The initial metadata allowlist
is identical for all four targets: generated `name` and `description`, plus
canonical `license` and `metadata` when present. Emit optional fields only when
the target capability contract accepts them; otherwise reject the selection.
Reject every nonempty native passthrough block and every unknown flattened
frontmatter field in this initial producer. This excludes `allowed-tools`, model
selection and invocation controls until explicit mappings are reviewed. Keep
legacy rendering unchanged. Skill `requires` dependencies remain resolution
inputs rather than generated native frontmatter.

Copy selected supporting resources byte-for-byte, preserving relative structure
and supported executable modes. Apply subtractive ignore patterns; strip source
entrypoints and recognized overrides. Do not process directives in resources.
Reject links, special files, traversal and output collisions. Generated
entrypoints are ordinary non-executable files. Resource identity includes modes.

## Package and path choices

Use one package per logical item and client, independent of bundle membership. A
direct installation and two bundles referencing that same installation share one
package. Different selected sources/revisions or incompatible aliases must
resolve explicitly or produce a conflict; equal bytes do not establish
ownership.

The neutral root is `<scope>/.upskill/exports/<client>/<installation-id>/`.
`<scope>` is the qualified project root or user home. Reuse the validated
installation identifier; keep client and scope in the ownership key. Never use
symlinks or put the managed root under a client's automatic discovery tree.
Discovery inventory must also exclude configured imports/globs reaching it.

Each root contains immutable `revisions/<digest>/` payloads and, where needed,
one stable `catalog/` pointing to the selected revision. Each payload contains
`skills/<installed-name>/SKILL.md` and its resources. Native envelopes live at
the revision root. The catalog advertises exactly one revision, never the
revision parent directory. Inactive retained revisions are not native sources.

Compute a domain-separated SHA-256 payload identity from target surface, scope
identity, installation identity, captured source revision, selected-body origin,
rendered entrypoint and ordered resource paths/bytes/modes. Encode
length-delimited fields. Derive native version `0.0.0-r<full-payload-hash>`;
then hash the complete rendered package separately for verification. This avoids
hashing a manifest's own version into its input. Native acceptance of that
version is a qualification condition; do not truncate identities or rely on
version ordering for refresh.

Define the stable owner key as the length-framed tuple of qualified scope
identity, installation ID and target surface. Scope identity includes canonical
qualified scope path and scope kind. Exclude source revision, content digest,
alias and bundle membership. The installation ID identifies one logical
installation across updates; replacing it is a remove/add operation. Native
package and catalog names use `upskill-<full-SHA-256-of-owner-key>`. The skill's
user-facing name remains the installed name. A client name-length restriction
blocks that profile until a reviewed collision-checked encoding is specified;
silently shortening names is forbidden.

## Concrete client routes

These are candidate native profiles, not claims of supported production paths.
Every profile needs project and user controls independently.

### Claude Code

Emit `.claude-plugin/plugin.json` at the revision root and use its default
`skills/` component directory. Do not additionally declare the same directory:
custom component paths supplement native defaults. The stable catalog uses
`.claude-plugin/marketplace.json` with a local source for the selected revision.
Register/install through the native marketplace/plugin commands in the intended
scope. Capture registration and enabled-state contributions separately.

Q6 establishes a bounded skill/resource positive and fresh-session update for
its tested profile. It does not qualify running-session updates. Q11b records
Copilot activation of a Claude project registration and separate preservation
failure. Consequently that project profile remains blocked.

### Copilot CLI

Emit the legacy `.plugin/plugin.json` envelope, default `skills/` directory and
`.plugin/marketplace.json` catalog. Use native plugin registration,
installation, refresh and removal rather than manual cache edits. This
deliberately avoids a portable root manifest that might broaden compatibility
discovery; the legacy envelope still does not prove isolation.

Require an owned enabled-state contribution in the intended scope. Do not assume
CLI installation has project scope: install-then-disable user activation is not
an approved project admission sequence. Block project activation until a native
primitive or predeclared disabled registration proves there is no unintended
user exposure. Q3 does not establish the required skill update lifecycle.
Retained Q11 and Q11b failures remain release blockers.

### Codex CLI

Emit `.codex-plugin/plugin.json` with `skills: "./skills/"`, and use a catalog
at `.agents/plugins/marketplace.json` under the neutral package root. Use native
catalog/plugin operations and normal trust handling. Do not create automatic
`.agents/skills` discovery exposure or invent an `enable --scope project`
command.

Q8 requires native removal/reinstallation before updated resource bytes become
visible in its tested profile. Plan this as an explicit refresh, preserving
native state and surviving references. A user-scoped registration followed by a
project activation does not establish safe project-only admission. Project
activation stays blocked until that transition and unrelated-project absence are
qualified. User scope needs its own positive and isolation controls.

### OpenCode CLI

Use the native explicit `skills` configuration source, pointing at the selected
revision's absolute `skills/` directory. Do not introduce a JavaScript plugin
solely to rediscover generated skills. Explicit skill paths are interpreted
relative to the active working directory, so relative paths cannot safely cover
nested invocations. Do not use missing-file or missing-environment interpolation
as a substitute for an absolute qualified path.

For project scope, contribute to a local `.opencode/opencode.json` overlay;
require that overlay to be untracked and ignored, since the absolute pointer is
machine-local. Preserve tracked root configuration. If that overlay already
exists as tracked content, or another config masks/duplicates its source, reject
with a diagnostic rather than rewrite the user's configuration. For user scope,
use the effective native user config, subject to qualified-root support. An XDG
config outside the qualified home root is blocked until the coordinator can
safely qualify that additional root. Preserve unrelated config bytes/comments.

A shared project retains portable installation intent; collaborators explicitly
materialize their local native bindings. A clone is not an activation promise.
Q5's provider failure leaves skill/resource activation and updates unqualified.
The proposed source must pass nested-CWD and effective-config controls.

## Ownership, state and transactions

Reuse `ArtifactOwner`, owner consumers, reference reachability and qualified
file planning. Do not represent native bindings as ordinary files or use
`plugin.rs` warn-and-skip helpers as successful owned installation.

Introduce three distinct records: immutable package revisions; the selected
revision per owner/client; and machine-local native binding receipts. Receipts
include exact client version/surface, scope, native identity, captured prestate,
planned inverse, observed outcome and operation identity. They are not a second
authoritative installation lock. Portable desired intent and machine-local
receipts must be reconciled with the state-compatibility work before CLI wiring;
copying an absolute-root `CombinedState` into another clone is not onboarding.

The transition is: capture complete prestate; validate all readers and owners;
prepare inactive immutable files; persist native-operation intent; apply native
binding/refresh; observe effective state; commit selection and ownership. Only
then retire detached revisions when resource-retention controls permit it. Never
expose two selected bodies during a switch. If a native API cannot switch
without overlap, require a qualified detach-before-attach sequence with explicit
temporary unavailability and a recoverable inverse.

Use deterministic root lock ordering for project files and user native state.
The current executor accepts a single scope root: extend coordination before a
cross-root native profile is executable. Do not claim cross-root atomicity.
Durable receipts precede external effects. Unknown native outcomes stop as
`RecoveryRequired`; recovery observes before retrying and does not overwrite
manual changes. File-only rollback cannot close a native transaction.

Configuration writes own exact selectors, not entire manual files. Use parsed
spans and byte patches with expected-prestate checks; preserve unrelated bytes,
comments and keys. Reject duplicate/ambiguous keys and concurrent edits. JSONC
and TOML parser/dependency admission is a separate reviewed prerequisite: this
design adds no dependency and authorizes no handwritten partial TOML parser.

Before refresh or detach, prove retention of native cached resource paths for
live/resumable sessions, or prove session clearance. Export revision retention
alone is insufficient when a client reads from its own cache. Without either
proof, block the transition and retain the current binding; never remove and
reinstall it speculatively.

Updates stage a new immutable revision and explicitly refresh the native
binding. A no-op revalidates owned bytes and native selection. Local edits block
update, removal and no-op reconciliation. Final reference removal detaches the
owned native contribution; shared dependencies survive. Removing a selected
override makes the item unavailable, with no automatic default fallback.
Explicit default selection is a separate preflighted operation. Do not delete a
revision while live/resumable sessions or manual references still require it;
absent reliable clearance, retain it and report why. Native residue is allowed
only by the approved D1-D6 profile, never by broad whole-profile equality
exemptions.

## Acceptance and admission

Offline acceptance must demonstrate selection, aliases, metadata protection,
resource bytes/modes, deterministic hashes, no extra native component roots,
reference sharing, complete preflight, manual-content preservation and recovery
at every file/native/state checkpoint. Tests must include unexpected outcomes,
concurrent edits, masked configuration, relocated clones and nested working
directories. Existing legacy goldens and CLI behavior remain unchanged.

Native admission separately requires fresh attributed baseline, intended-client
positive including a path-bound resource read, unintended-client and unrelated
project negatives, discovery/selector inventory, direct/plugin duplicate
controls, update visibility, final detach, preservation and post-removal
absence. Record exact client/version/OS/scope and distinct observer sessions.
Keep Q10, Q11, CD-01 through CD-11 and the four-client release gate unchanged. A
configured negative control cannot replace the retained foreign-selector Q11
requirement. No profile is activated merely because its package compiles or CI
is green.

## Evidence and alternatives

Retained observations:
[Q6 Claude](../../technotes/gate6-claude-managed-root.md),
[Q3 Copilot](../../technotes/gate6-copilot-plugin-components.md),
[Q8 Codex](../../technotes/gate6-codex-skill-updates.md),
[Q5 OpenCode retry](../../technotes/gate6-opencode-explicit-paths-retry.md),
[Q11](../../technotes/gate6-cross-client-package-isolation.md) and
[Q11b](../../technotes/gate6-project-registration-isolation.md). Apply the
approved
[preservation contract](2026-10-25-customization-preservation-contract.md).

The existing compiler rejects active overrides; directive family support already
includes Codex. `OwnedArtifact` and combined reference reconciliation are
reusable; the file journal has no native effect type. These are implementation
seams, not evidence of completed delivery.

Shared automatic skill roots would reduce copies but leave exclusion and
fallback obligations unresolved. Mutable in-place packages would reduce retained
files but could change resources read by existing sessions. Per-bundle packages
would duplicate shared items. Choose separate immutable item packages instead.
Native explicit OpenCode sources avoid an unnecessary runtime plugin dependency.

Primary interface references:
[Claude plugin components](https://code.claude.com/docs/en/plugins-reference),
[Copilot plugin reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference),
[Codex plugins](https://developers.openai.com/plugins/build/plugins),
[OpenCode skills](https://opencode.ai/v2/docs/skills) and
[OpenCode configuration](https://opencode.ai/v2/docs/config). These interfaces
are version-sensitive; retained repository qualification packets define the
bounded observations above. Recheck documentation and pinned native help before
adapter implementation, without consuming model credits.

See the
[staged implementation plan](../plans/2026-10-31-four-client-skills-resources-delivery.md).
