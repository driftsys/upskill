# Source-backed Codex instruction lifecycle

Task: #325. This bounded Gate 6 slice connects the captured Codex CLI producer
from AD-0039 to selected source resolution and the existing qualified lifecycle
session. The user delegated routine design, implementation, review and delivery
decisions. Production CLI routing, schema 1, generation Client, target defaults
and native qualification claims remain unchanged.

## Intent and approach

A caller should resolve a selected local or Git rule, inspect the complete known
reader report, and apply an ownership-aware add/update/remove operation without
constructing an Installation or detaching captured source evidence by hand.

Add an explicit Codex source compiler and orchestration entry point beside the
historical route. Share internal session/preflight/source lifetime machinery; do
not duplicate the orchestration algorithm. The historical public route keeps its
existing contract and rejects Codex installations on Add/Update, including
audience-excluded ones without artifacts. Both routes may Remove recorded
ownership independently of adapter availability. The Codex route rejects
cross-adapter conversions rather than guessing whether a historical installation
can be migrated.

These are controller design choices. A unified public route would reduce entry
points but alter existing caller authority; removing the historical rejection
would accept instruction producers without their reader contract. The separate
opt-in route makes that boundary explicit while leaving later unification open.

## Captured source binding

Expose CapturedRuleSource::compile_codex with a CodexSourceBinding containing
installation_id, expected_source_name, installed_name, root, requested, profile
and location. It reuses the existing Installation construction, verifies
retained source/revision identity before compilation, and returns
CompiledCodexInstructions. Source label, actual Git commit and manifest digest
come only from the resolver. There is no caller-supplied source evidence or
desired result substitution.

## Orchestration contract

Expose with_prepared_codex with the existing SourceRequest and callback
structure, and CodexSourceOptions containing target selection, add defaults, one
explicit TargetProfile and ResourceExecutionPolicy. Only CodexCli contract
revision 1 and an effective target selection of exactly [CodexCli] are
supported. Validate these before resolving any source. Unsupported
profile/selection failures retain typed CapabilityError values; malformed
retained bindings remain contextual validation errors. No missing target
defaults are invented.

Canonical destination candidates are derived from the declared session root:
project AGENTS.md and user .codex/AGENTS.md. They must pass the existing
producer binding checks. The qualified session establishes physical access to
this declared root, but this library route does not discover actual
HOME/CODEX_HOME or certify native loading. External native home selection and
production activation remain separate preflight requirements.

Preflight the complete selected set before any resolution. Add may create a new
installation or refresh an identical supported Codex binding. Update may select
only retained rule installations requested exactly for CodexCli; any owned
artifact must be a Markdown contribution at the canonical root/location with
CodexCli-only consumers and its matching revision-1 owner contract. This
retained representation validation also applies before same-alias Add refresh.
Unsupported instruction destinations, profiles, historical whole-file owners,
mixed selections and cross-adapter retargeting fail without a source fetch or
callback invocation. Retained installations may have zero artifacts (excluded
output) or exactly one contribution with the producer-derived ID for that
installation. Reject arbitrary IDs, multiple contributions and
cross-installation shared ownership before source resolution. Audience-excluded
supported Codex installations remain updateable. This conservative
representation rule may require an explicit later migration for valid
hand-constructed instruction state outside the producer contract.

Use the existing framed installation identity and original captured manifest.
Updates keep installation and contribution identities while refreshing source
revision, canonical body digest, contracts and reader compatibility. Existing
source mutation/revision checks run before application. Multi-source compilation
must complete before the callback or any journal/publication, so a later source
conflict cannot partially apply earlier sources.

## Reader exposure and application

SourcePrepared exposes instruction_readers() returning a borrowed slice of
SourceInstructionReaders records, each containing installation_id and the
complete InstructionReaderReport. The slice is deterministically ordered by
installation identity. Historical preparations and remove operations have no
newly compiled reader reports; an empty slice does not claim that retained
artifacts have no readers. Remove never fetches source or consults
compiler/target availability.

Reports include audience-excluded results with NotEvaluated compatibility. Only
sets and owner contracts enter CapturedUpsert. Reports never enter
CombinedState, artifact consumers, references or execution receipts. Callers can
clone reports before apply if they need to retain presentation data. The opaque
apply closure keeps source captures and qualified session lifetime checks
intact.

Removal uses recorded ownership, preserves unrelated contributions/manual bytes,
and requires no source tree. A caller may discard preparation after inspecting
reports without publishing files or state.

## Acceptance and remaining gates

Dedicated source and orchestration examples cover local and moving Git sources,
project and user roots, generic and Codex-only audiences, exclusions, two-rule
coexistence, manual text/mode preservation, update/remove, source mutation after
preparation, and discarded preparation. Literal report inventories and exact
ownership/contracts are independent assertions. Negative examples prove
no-effect failure for unsupported retained bindings, mixed selections, invalid
profiles, reader conflicts and a later invalid source in a multi-source request.
Explicitly cover artifact-free historical-route rejection, excluded-to-included
refresh, deterministic multi-installation report ordering, and
unavailable-option removal.

Regression controls must detect dropped reports (including excluded content),
incidental-reader ownership, global destination corruption, lost final source
revalidation and bypassed complete preflight. Existing historical source and
orchestration tests remain unchanged unless an additive rejection diagnostic
requires a precise update.

This slice does not add preview fetching, persist exposure metadata, migrate
historical adapters, activate schema 2, add CLI flags, resolve bundles or
qualify native clients. Complete all-kind adapters, conflict aggregation, native
user/IDE/Linux/OpenCode evidence and staging accounting in #316 remain gates.
