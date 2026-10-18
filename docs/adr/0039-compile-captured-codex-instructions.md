# AD-0039 — Compile captured Codex instructions with a separate reader report

## Context

Project `AGENTS.md` can expose one managed contribution to several client
surfaces. Selecting Codex CLI alone cannot remove potential Copilot, OpenCode or
Codex IDE readers. Treating exposure as intentional ownership would create false
retention and compatibility claims. Existing captured lifecycle commands can
preserve separate instruction contributions, but complete source integration and
native qualification remain pending.

## Options

Alternatives not documented. The bounded source specification explicitly chooses
strict extension rejection and reserves admission of proven optional fields for
a later reviewed contract.

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
contract, and reports compatibility as not evaluated. Reject empty or
unrepresentable bodies and managed-marker injection before returning desired
output.

Derive a stable contribution ID as lowercase SHA-256 of the JSON tuple
`("upskill-codex-contribution-v1", installation_id)`. Body, alias and source
revision do not enter the ID separately. Bind the contract to the captured
manifest and canonical body digest. Distinct identities remain separate
contributions; the producer adds no sharing authority.

## Consequences

The narrow subset can reject otherwise harmless fields until their behavior is
reviewed. Structural equality does not qualify native behavior. Actual root and
native home qualification remain the caller's responsibility; the producer does
not inspect `CODEX_HOME` or grant external destination authority.

Historical generation clients, default selection, CLI routing and schema-1
persistence remain unchanged. Source integration, all-kind lifecycle adapters,
conflict aggregation, native user/IDE/Linux/OpenCode qualification and staging
accounting remain later gates. See the
[generation concern](../topics/0002-generation.md#captured-codex-instruction-boundary)
and [acceptance target](../../tests/codex_instruction_boundary.rs).

Satisfies:
[captured Codex instruction boundary](../topics/0002-generation.md#captured-codex-instruction-boundary).
Related: [AD-0013](0013-agent-customization-targets.md),
[AD-0024](0024-pure-captured-item-compilation.md),
[AD-0025](0025-reject-active-captured-overrides.md).
