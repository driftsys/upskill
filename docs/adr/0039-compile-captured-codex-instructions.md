# AD-0039 — Compile captured Codex instructions with a separate reader report

## Context

Project `AGENTS.md` can expose one managed contribution to several client
surfaces. Selecting Codex CLI alone cannot remove potential Copilot, OpenCode or
Codex IDE readers. Treating exposure as intentional ownership would create false
retention and compatibility claims. Existing captured lifecycle commands can
preserve separate instruction contributions. Selected rule source integration
now uses that boundary; native qualification remains pending.

## Options

Keep strict extension rejection and reserve proven optional fields for a later
reviewed contract. For source orchestration, choose a separate explicit Codex
entry point sharing the existing private engine. A unified public route would
alter historical compiler authority; accepting instructions through that route
would omit the separate reader contract. Later unification remains possible.

## Decision

Add a pure library producer for exactly one intentional `codex.cli` profile at
contract revision 1. Accept unconditional rules with no effective resources,
using validated captured source bytes and existing installation bindings. Reject
nonempty scope, override entrypoints, native extension maps and unknown rule
fields instead of omitting potentially required semantics.

Version the location reader contract separately. Revision 1 accepts project
`AGENTS.md` with five readers (`codex.cli`, `codex.ide`, `copilot.cli`,
`copilot.vscode-local`, `opencode.cli`), and default global `.codex/AGENTS.md`
with two (`codex.cli`, `codex.ide`). VS Code Local discovery includes the
`chat.useAgentsMdFile` condition. These are potential exposures, not runtime
observations. Require audience eligibility and identical canonical
family-filtered Markdown for every declared reader before returning a
contribution.

Return a reader report separately from complete desired artifacts and owner
contracts. Only requested Codex CLI enters consumers and owner profiles.
Excluded Codex output retains installation history, emits no artifact or
contract, and reports compatibility as `NotEvaluated`. For emitted
contributions, reject empty or unrepresentable bodies and managed-marker
injection before returning desired output. These body and marker checks do not
apply to excluded output.

Derive a stable contribution ID as lowercase SHA-256 of the JSON tuple
`("upskill-codex-contribution-v1", installation_id)`. Body, alias and source
revision do not enter the ID separately. Bind the contract to the captured
manifest and canonical body digest. Distinct identities remain separate
contributions; the producer adds no sharing authority.

Add `CapturedRuleSource::compile_codex` with a source binding derived from the
retained resolver. Add `with_prepared_codex` with explicit target/profile
options, sharing recovery, complete preflight, resolution and scoped application
with the historical route. Recovery and Remove precede compiler validation;
Remove needs no source or current compiler. Unsupported profile/selection
failures retain typed Instructions capability errors.

Preflight all retained selected owners before any source resolution, including
same-alias Add refreshes. Require exact Codex CLI rule intent and the declared
session root in both scope and raw path spelling, including artifact-free
installations. Derive canonical instruction destinations from that declared
root. Accept zero artifacts or one canonical producer-ID contribution with sole
ownership, Codex CLI per-owner consumers and a matching revision-1 contract.
Reject shared/arbitrary/multiple contributions, whole-file conversion and other
adapters. The historical route also rejects artifact-free Codex installations on
Add/Update.

Expose complete reports sorted by installation identity through the borrowed
`SourcePrepared::instruction_readers()` slice, including exclusions. Reports do
not enter persisted ownership, references or receipts. Historical and Remove
callbacks expose no newly compiled reports. Keep retained source and actual Git
revision checks immediately before application.

## Consequences

The narrow subset can reject otherwise harmless fields until their behavior is
reviewed. Structural equality does not qualify native behavior. Actual root and
native home qualification remain the caller's responsibility; the producer does
not inspect `CODEX_HOME` or grant external destination authority. Qualified
access to a declared user root does not identify real native HOME/CODEX_HOME.

Historical generation clients, default selection, CLI routing and schema-1
persistence remain unchanged. Selected Codex rule source integration is
library-only; all-kind lifecycle adapters, conflict aggregation, native
user/IDE/Linux/OpenCode qualification and staging accounting remain later gates.
See the
[generation concern](../topics/0002-generation.md#captured-codex-instruction-boundary)
and [acceptance target](../../tests/codex_instruction_boundary.rs).

Satisfies:
[captured Codex instruction boundary](../topics/0002-generation.md#captured-codex-instruction-boundary),
[selected lifecycle rule sources](../topics/0003-sources.md#selected-lifecycle-rule-sources),
and
[source-backed lifecycle orchestration](../topics/0005-lifecycle-planning.md#source-backed-lifecycle-orchestration).
Related: [AD-0013](0013-agent-customization-targets.md),
[AD-0024](0024-pure-captured-item-compilation.md),
[AD-0025](0025-reject-active-captured-overrides.md).
