# Captured rule desired sets

The opt-in `lifecycle::desired` library binds complete captured rule compilation
to an explicit installation root and explicit target consumers. It produces
supplied-value desired files, not ownership, execution authority or native
conformance evidence. Production commands and lockfiles remain on schema 1.

## Installation and source binding

`compile_rule_desired_set` accepts an `Installation`, a complete captured
`SourceManifest`, a source folder name and explicit `CompilerBinding` values. It
validates the installation through the combined-state validator and validates
the entire capture. The captured manifest digest must equal recorded source
evidence. Git and local revision constraints retain the combined-state contract;
these strings do not authenticate source origin.

Only rule installations are accepted. The existing captured-item compiler
resolves source identity and aliases, performs audience filtering, handles
resources and ignores, and renders exact per-family output. Its source name,
installed name and manifest identity must match the installation. No second
output-layout table is introduced, and source paths are never reread.

## Explicit consumers

| Compiler family | Allowed explicit consumer identities  |
| --------------- | ------------------------------------- |
| Claude          | `claude.cli`                          |
| Copilot         | `copilot.cli`, `copilot.vscode-local` |
| OpenCode        | `opencode.cli`                        |

Bindings require one entry per selected family, nonempty unique consumer lists
and exact accounting for requested surfaces. Copilot gains both consumers only
when both were explicitly requested. Codex CLI and IDE have no compiler binding
here and fail explicitly, even if author audience would exclude them. Family
identity matching is not a native discovery or scope qualification claim.

Audience-excluded families are projected onto only their explicitly bound
consumers. Their requested surfaces and complete installation record survive. An
empty desired set is valid after exclusion and is never converted into an
installation removal by this producer.

An active OpenCode output with nonempty SSOT `scope.paths` is rejected because
the historical renderer drops that restriction. Absent/empty scope is accepted;
an audience-excluded OpenCode family has no output whose scope could be widened.
Existing compiler and production rendering behavior remain unchanged.

## Complete desired files

Every compiled entrypoint and resource becomes a `DesiredArtifact::File` under
the supplied root, preserving its raw spelling and scope. Bytes, resource
filtering, alias link rewriting and optional captured modes are retained
exactly. Generated entrypoint modes remain unspecified. Physical permission
selection and qualification remain later application work.

Files are ordered by their portable relative path and consumers by surface name.
Repeated paths within an installation conflict. These lexical checks do not
resolve symlinks, aliases, case equivalence or physical containment.

## Remaining lifecycle boundary

This producer does not reconcile existing references, shared owners, legacy
selection, instruction contributions or existing destinations. The mixed
planner's affected multi-owner rejection remains intact. Combined lifecycle
planning, guarded backup/application/recovery, observational adapters and native
activation remain separate tasks.
