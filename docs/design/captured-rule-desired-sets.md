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

## Complete batch preflight

`compile_rule_desired_sets` compiles every independent request and returns
either `Ready` with all sets in input order or `Blocked` with contextual
diagnostics. There is no successful subset in a blocked outcome. An empty batch
is ready with no sets. Compilation reports one error per failed request, without
claiming all errors within that rule.

Duplicate installation IDs conflict even at disjoint roots. Successful requests
also conflict when selected file paths are equal or ancestors of one another
under the same scope and raw root spelling. Equal bytes do not establish a
shared-owner contract. Different raw roots or scopes remain separate at this
lexical boundary; physical aliases and case equivalence need later
qualification. Diagnostics contain both request indices and installation IDs and
sort by indices, then message. This boundary does not reconcile existing state
or references.

## Interfaces and acceptance evidence

The [single-rule producer](../../src/lifecycle/desired/mod.rs) defines
`CompilerBinding`, `RuleDesiredSetRequest` and `DesiredInstallationSet`. The
[batch boundary](../../src/lifecycle/desired/batch.rs) exposes
`DesiredSetDiagnostic`, `DesiredSetsOutcome` and `compile_rule_desired_sets`.
These supplied-value interfaces neither serialize plans nor authorize effects.

The [public acceptance tests](../../tests/lifecycle_desired.rs) compare literal
paths, binary resource bytes, modes and checked-in renderer goldens. They cover
source identity and fallback names, exact requested and excluded consumers,
OpenCode scope refusal, raw root retention, complete ordered batch payloads,
component-boundary topology, duplicate IDs and contextual diagnostic ordering.
The [validation note](../technotes/2026-10-15-m1-gate5-desired-validation.md)
records local checks and the limits of the independent review.

## Remaining lifecycle boundary

This producer validates each installation in a minimal direct-reference graph;
it does not reconcile the authoritative graph across the batch or establish
cross-request installation-identity consistency beyond duplicate ID rejection.
Separate [reference reconciliation](lifecycle-reference-reconciliation.md) and
[removal planning](lifecycle-reference-removal.md) now establish graph retention
and shared-owner retirement with authoritative per-owner consumers. Pure
[selection preflight](lifecycle-selection.md) now aggregates legacy/native
conflicts for all/literal-name requests. The pure
[single-owner upsert coordinator](lifecycle-upsert.md) now joins complete sets,
selected graph changes and supplied destination observations. Source/scoped
selectors and CLI precedence, instruction compiler adapters, shared add/update
contracts and complete lifecycle integration remain task 2b. The mixed planner's
affected multi-owner rejection remains intact. Guarded
backup/application/recovery, observational adapters and native activation remain
separate tasks.
