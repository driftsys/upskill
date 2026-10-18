# Captured Codex instruction boundary

Task: #322. This bounded slice implements part of M1-B from the approved
[foundation](2026-10-11-agent-customization-foundation-design.md) and
[owned lifecycle](2026-10-12-m1-owned-lifecycle-design.md) designs. Routine
decisions and delivery are delegated by the user.

Produce complete desired instruction contributions from validated captured rule
bytes for an explicitly requested Codex CLI contract. Return the existing
desired set and owner contracts together with a distinct reader report. Pure
values do not establish physical containment, source acquisition authority or
native qualification. This slice does not integrate source orchestration,
activate schema 2 or add a production CLI flag.

## Supported subset

Accept exactly one intentional surface, `codex.cli`, at contract revision 1. The
installation must be a rule whose source name, alias, manifest evidence and
destination binding validate. Supported destination candidates are `AGENTS.md`
under a declared project root and `.codex/AGENTS.md` under a declared global
home root. Other paths, scope mismatches and external native home destinations
are unsupported. The caller must establish the actual native home and physical
scope before later application; this pure producer never reads environment
variables or claims to resolve `CODEX_HOME`.

Rules must be unconditional and have no effective resources after the existing
ignore filter. Reject nonempty scope paths, client override entrypoints, native
extension maps and unknown top-level rule fields rather than silently dropping
potentially required behavior. Metadata, license declarations and dependency
declarations remain source information; this producer does not resolve
dependency graphs. Audience-excluded Codex output retains installation history
with no artifacts or owner contracts, as existing desired sets do.

An emitted canonical body must satisfy the existing contribution validator,
including its terminating newline and managed-marker restrictions. Reject empty
or otherwise unrepresentable bodies before returning a contribution; do not
leave that failure for later application.

The strict extension rejection is deliberate: this first compiler does not
represent native extension behavior. A later reviewed contract may admit proven
optional fields. The cost of this choice is rejecting some otherwise safe source
metadata until its semantics are established.

## Shared readers

Location contract revision 1 derives its full reader inventory; callers cannot
reduce it by selecting fewer intentional surfaces. Project readers are
`codex.cli`, `codex.ide`, `copilot.cli`, `copilot.vscode-local` and
`opencode.cli`. Default global Codex instructions have `codex.cli` and
`codex.ide` readers. IDE discovery is potential exposure, including the VS Code
Local `chat.useAgentsMdFile` condition, rather than a runtime observation.

For an emitted contribution, every declared reader must be eligible under the
source audience and receive identical canonical Markdown after family directive
processing. Audience exclusion is a conflict even when text happens to match.
Reject incompatible output before returning an applicable contribution. These
checks establish the restricted structural contract, not equal runtime behavior.
OpenCode, user, IDE and native Linux qualification remain separate gates.

Only requested Codex CLI enters artifact consumers and owner contract profiles.
Potential incidental readers never become owners, consumers or retention
references. The report records location, location-contract revision, reader
profiles/roles and whether compatibility was evaluated for emitted content. An
audience-excluded result must not claim successful compatibility checks.

## Identity and preservation

Derive a stable contribution ID from the installation identity using framed
SHA-256 input. Body or source revision updates retain that ID; distinct
installation identities remain distinct contributions. Bind the owner contract
to the original captured manifest and canonical contribution body digest.
Unsupported capabilities return a typed `CapabilityError` through the existing
error chain; malformed bindings remain contextual validation errors.

Dedicated acceptance tests must exercise generic project output, audience and
directive conflicts, Codex-only global output, stable identity, ignored versus
effective resources, unsupported profiles/destinations, and exact separation of
ownership from exposure. Captured command tests must demonstrate that two
contributions coexist with manual Markdown, and updating/removing one preserves
the other and manual bytes. Reader removal and incidental-owner mutations must
fail meaningful assertions.

Generation `Client`, historical layouts, the four existing default surfaces and
production command routing remain frozen. Family-aware audience/directive
support is additive; existing rendering remains byte-compatible. General
activation still needs complete all-kind lifecycle adapters, source integration,
conflict aggregation, native qualification and staging accounting in #316.
