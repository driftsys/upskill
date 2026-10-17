# Generation and compilation

This concern owns the requirements and implemented architecture below. Moving
these records does not change their implementation or approval status.
Library-only capabilities remain separate from production CLI activation; each
section records its own limits. Accepted decisions remain in
[ADRs](https://github.com/driftsys/upskill/blob/main/docs/adr/README.md).

## Requirements

<a id="45-captured-item-compilation"></a>

### Captured item compilation

The opt-in `compile::item` library API compiles one captured item into complete
per-client file sets without rereading source paths, writing destinations or
mutating inputs:

`compile_item` borrows a `SourceManifest` and `ItemCompileRequest` and returns
`anyhow::Result<CompiledItem>`. The exact public declarations are in
[`compile::item`](https://github.com/driftsys/upskill/blob/main/src/compile/item.rs).
`ItemCompileRequest` contains `kind: ItemKind`, `folder_name: String`,
`alias: Option<String>` and `clients: Vec<Client>`. `CompiledItemFile` contains
`path: String`, `bytes: Vec<u8>` and `mode: Option<u32>`. `CompiledClientItem`
contains `client: Client` and `files: Vec<CompiledItemFile>`. `CompiledItem`
contains `kind: ItemKind`, `source_name: String`, `installed_name: String`,
`source_manifest_sha256: String`, `outputs: Vec<CompiledClientItem>` and
`audience_excluded: Vec<Client>`.

All four structures derive `Debug`, `Clone`, `PartialEq` and `Eq`, without
serialization. `ItemKind` is the existing pipeline kind and `Client` is the
current Claude, Copilot and OpenCode generation vocabulary. The result owns its
values and records the exact validated supplied manifest digest as source
identity; that digest does not establish provenance or identify generated
content.

Compilation first validates every manifest path, ordering, digest and mode,
including files later excluded or ignored. File/descendant pairs such as `a` and
`a/b` fail even when ignored. Requested clients must be explicit, nonempty and
unique. The canonical root entrypoint for the requested kind must exist, contain
UTF-8 and parse as its existing typed frontmatter with a supported schema. The
folder, resolved source name and optional alias must each be one portable
relative component. Skills use the folder name; rules and agents use their
optional declared name, otherwise the folder. An invalid declared rule or agent
name fails. An alias controls installed paths and resource namespaces while
rendered frontmatter retains the source identity under the existing renderers.

Outputs and `audience_excluded` follow `Client::ALL` order regardless of request
order. Each requested client omitted by canonical `audience` is explicitly
excluded; a wholly excluded selection succeeds with empty outputs. This does not
imply removal of existing files.

Resource selection reuses the existing root-only canonical entrypoint and known
`<KIND>.<client>.md` override exclusions and the requested kind's subtractive
`ignore` semantics. Nested equivalents, unrelated client-suffixed filenames,
unknown-client suffixes, hidden files, empty files and binary files remain
resources unless ignored. A root override for the requested kind and an emitted
client fails with an unsupported-override error even if ignored. Overrides for
other kinds or non-emitted clients remain excluded source inputs. Body override
precedence is deferred.

Each included output contains exactly one rendered entrypoint and every retained
resource, sorted by portable UTF-8 relative path. Compilation reuses existing
renderers, directives, passthrough, formatting and frozen historical layout. For
flat outputs it rewrites copied-resource links using the source name, then
re-prefixes source namespaces to the alias when different, including previously
namespaced links. Directory-backed outputs retain relative links. Duplicate or
file/descendant output paths fail; errors return no successful partial result.

Resource bytes and optional permission modes equal captured evidence exactly.
Generated entrypoints carry `mode: None`; this does not authorize permission
changes. A qualified writer must establish destination permission policy and
persist observed mode evidence separately.

Current commands, installer behavior and lockfile persistence remain unchanged.
Returning an OpenCode rule file does not install its instruction registration.
The API supplies no installation roots, ownership, destination observations,
reference claims or application instructions and makes no native support claim.
Connecting these outputs to lifecycle planning, registry/bundle resolution,
requested-selection reconciliation, qualified source/filesystem evidence, writer
locking, durable state, execution/recovery and Codex activation remain pending.
The separate mixed coordinator in
[mixed owned-artifact operation plans](0006-artifact-planning.md#46-mixed-owned-artifact-operation-plans)
accepts explicit desired ownership and observations; compilation does not
produce those lifecycle inputs. This prerequisite completes neither M1-A2 nor
M1-A3.

See the
[compiler design](https://github.com/driftsys/upskill/blob/main/docs/topics/0002-generation.md#architecture-captured-item-compilation),
[AD-0024](https://github.com/driftsys/upskill/blob/main/docs/adr/0024-pure-captured-item-compilation.md),
[AD-0025](https://github.com/driftsys/upskill/blob/main/docs/adr/0025-reject-active-captured-overrides.md)
and
[AD-0026](https://github.com/driftsys/upskill/blob/main/docs/adr/0026-separate-generated-permission-policy.md).

<a id="414-captured-rule-desired-sets"></a>

### Captured rule desired sets

The opt-in `lifecycle::desired::compile_rule_desired_set` API validates a
complete captured rule source and its explicit installation/source/consumer
binding, then projects every existing compiler output into a complete desired
file set. Exact bytes, resource modes, aliases, raw roots and explicit consumers
survive. Requested surfaces remain recorded after audience exclusion, including
when the desired set is empty. Empty output is not an installation removal.

Consumers must exactly account for requested surfaces and match the explicitly
selected Claude, Copilot or OpenCode compiler family. No Codex binding or native
conformance claim is introduced. Active scoped OpenCode output is rejected
rather than silently losing nonempty SSOT `scope.paths`.

These are pure supplied-value results with no I/O, ownership adoption, lockfile
rewrite or production command activation. Physical qualification, permission
selection, reference/shared-owner reconciliation and application remain
separate. See
[captured rule desired sets](https://github.com/driftsys/upskill/blob/main/docs/topics/0002-generation.md#architecture-captured-rule-desired-sets).

`compile_rule_desired_sets` returns all complete sets in request order only when
all requests compile and pass preflight. Otherwise `Blocked` contains contextual
request-index/installation-ID diagnostics and no successful subset. Duplicate
installation IDs and equal or ancestor/descendant selected paths under the same
scope and raw root spelling conflict, including equal bytes. Diagnostics sort by
request indices then message. Empty input returns `Ready([])`. Different raw
roots and scopes remain separate; physical aliases and case equivalence require
later qualification. Each failed compilation contributes one contextual error
rather than exhaustive diagnostics within that rule.

## Architecture

<a id="architecture-generation-pipeline"></a>

<a id="architecture-generation-pipeline-generation-and-installation"></a>

### Generation and installation

This record describes the existing module boundaries. It does not extend
implemented client support. See
[ADR-0001](https://github.com/driftsys/upskill/blob/main/docs/adr/0001-multi-kind-compiler-architecture.md)
and
[ADR-0003](https://github.com/driftsys/upskill/blob/main/docs/adr/0003-generation-pipeline.md)
for the architecture decisions.

The Rust crate provides both a library and CLI. `src/main.rs` and `src/cli.rs`
provide the CLI surface and orchestration. Domain work belongs in library
modules; presentation stays at the CLI boundary.

| Responsibility                              | Modules                                                |
| ------------------------------------------- | ------------------------------------------------------ |
| Content model and parsing                   | `src/model/`, `src/parse/`                             |
| Source classification and retrieval         | `src/source.rs`, `src/fetch.rs`                        |
| Bundle composition                          | `src/bundle.rs`                                        |
| Per-client rendering                        | `src/generate/`                                        |
| Installation and installed-state operations | `src/pipeline/`, `src/lockfile.rs`, `src/ancillary.rs` |
| Pure legacy-state interpretation            | `src/legacy/`                                          |
| Native integrations                         | `src/plugin.rs`, `src/mcp.rs`                          |
| Author commands                             | `src/lint.rs`, `src/fmt.rs`, `src/scaffold.rs`         |

The central flow is source content to parsed items to per-client output. Bundles
compose items and native integrations. Generated content is copied to its target
location; the lockfile records installed state. The
[portable format](../format-spec.md) owns field and layout details; this record
does not duplicate those contracts.

Current content generators are Claude Code, Copilot and OpenCode. Generation and
MCP target sets differ, as recorded in
[ADR-0012](https://github.com/driftsys/upskill/blob/main/docs/adr/0012-consumer-client-filtering.md).
[ADR-0013](https://github.com/driftsys/upskill/blob/main/docs/adr/0013-agent-customization-targets.md)
adds design commitments for Codex, Cursor and Gemini CLI; it does not establish
implemented adapters.

<a id="architecture-captured-item-compilation"></a>

<a id="architecture-captured-item-compilation-captured-item-compilation"></a>

### Captured item compilation

This record describes the implemented pure producer in
[`compile::item`](https://github.com/driftsys/upskill/blob/main/src/compile/item.rs).
[captured item compilation](#45-captured-item-compilation) owns its
requirements. The producer consumes one supplied captured manifest and returns
complete current-generation outputs; the current installer does not call it.
[AD-0024](https://github.com/driftsys/upskill/blob/main/docs/adr/0024-pure-captured-item-compilation.md)
records the bounded compatibility choice and its alternatives.

<a id="architecture-captured-item-compilation-public-boundary"></a>

#### Public boundary

`compile_item` borrows a `SourceManifest` and `ItemCompileRequest`, returning an
owned `CompiledItem` through `anyhow::Result`. The four public structures are
`ItemCompileRequest`, `CompiledItemFile`, `CompiledClientItem` and
`CompiledItem`. They derive `Debug`, `Clone`, `PartialEq` and `Eq` without
serialization. The request supplies the existing `ItemKind`, folder name,
optional installed alias and explicit generation clients. Each client result
contains files with portable relative paths, exact bytes and optional modes; the
outer result retains kind, resolved source identity, installed name, supplied
manifest digest and explicit audience exclusions. The
[source module](https://github.com/driftsys/upskill/blob/main/src/source_manifest.rs)
owns capture and manifest validation. No source path is passed to the compiler.

The `Client` vocabulary covers Claude, Copilot and OpenCode generation. It does
not establish `TargetSurface` conformance, native support, required target
semantics or incidental-reader isolation. Results contain no roots, owners,
destination observations, reference claims or application instructions.

<a id="architecture-captured-item-compilation-validation-and-identity"></a>

#### Validation and identity

Compilation starts with complete manifest validation, including files later
ignored or reserved. An additional topology check rejects duplicate paths and
strict slash-component file/descendant pairs throughout the supplied source. The
folder and optional alias are validated as single portable components using the
ownership path predicate and slash rejection. Requested clients must be nonempty
and unique.

The compiler finds the requested kind's canonical root entrypoint, decodes it as
strict UTF-8 and parses its existing typed frontmatter. A private `ParsedItem`
enum dispatches typed naming, audience, ignore and rendering policy. Shared
`effective_name` uses the folder for skills and a declared name, if present, for
rules and agents; the resolved name is validated before use. The installed name
is the alias when present. Existing renderers receive the resolved source name,
so aliasing changes paths and resource namespaces while retaining rendered
frontmatter identity.

`source_manifest_sha256` copies the validated manifest identity, covering all
captured files regardless of output selection. It establishes neither source
provenance nor rendered-content identity. Inputs remain unchanged, and any error
returns no successful partial compilation.

<a id="architecture-captured-item-compilation-resources-and-unsupported-overrides"></a>

#### Resources and unsupported overrides

The producer shares the existing crate-private resource classifier and
subtractive ignore filter with the installer through narrow re-exports in
[`pipeline`](https://github.com/driftsys/upskill/blob/main/src/pipeline/mod.rs).
Their bodies and historical call paths are unchanged. Only recognized canonical
entrypoints and known override names at the root are reserved. Nested
equivalents, hidden files, unrelated names, unknown-client suffixes and empty or
binary files remain resources unless ignored. Selection retains original
captured values in manifest order.

For each requested client, canonical audience exclusion is recorded explicitly.
An entirely excluded request returns empty outputs and all requested exclusions;
it grants no removal authority. Before rendering an included client, compilation
rejects a matching root override for the requested kind, independently of ignore
patterns. Other-kind and non-emitted-client overrides remain reserved source
inputs.
[AD-0025](https://github.com/driftsys/upskill/blob/main/docs/adr/0025-reject-active-captured-overrides.md)
records the unsupported boundary; body override precedence remains
unimplemented.

<a id="architecture-captured-item-compilation-rendering-and-output"></a>

#### Rendering and output

Existing typed renderers retain directives, passthrough, client frontmatter and
Markdown formatting. Frozen historical
[layout helpers](https://github.com/driftsys/upskill/blob/main/src/legacy/layout.rs)
compute entrypoint and resource locations. Flat output first rewrites copied
resource links under the source name and then re-prefixes that namespace to a
different alias. Both existing link passes are preserved, including the second
pass for links already namespaced with the source name. Directory-backed output
keeps relative links with its entrypoint and resources.

Generated native paths are converted by accepting only normal UTF-8 components
and joining them with `/`, followed by portable path validation. Each included
client contains one rendered entrypoint and all retained resources, sorted by
portable path. Duplicate and strict file/descendant output locations fail.
Clients and exclusions follow `Client::ALL`, independently of request order.

Resources retain exact captured bytes and optional modes. Generated entrypoints
carry `mode: None`;
[AD-0026](https://github.com/driftsys/upskill/blob/main/docs/adr/0026-separate-generated-permission-policy.md)
records why destination permission policy requires separate qualification.

<a id="architecture-captured-item-compilation-evidence-and-downstream-integration"></a>

#### Evidence and downstream integration

[`tests/compile_items.rs`](https://github.com/driftsys/upskill/blob/main/tests/compile_items.rs)
pins all nine kind/client pairs with independent expected rendering and paths,
complete resource vectors, nonuniform resource modes, explicit audiences for all
clients, both flat alias link passes, source identity despite reserved or
ignored output, invalid inputs, impossible topology and source deletion after
capture. The tests establish supplied-value compilation and compatibility,
without filesystem execution or native-target conformance.

The producer prepares complete source-derived sets for future lifecycle
coordination. The implemented
[mixed coordinator](0006-artifact-planning.md#architecture-mixed-owned-artifact-planning)
composes explicit complete file/contribution sets, without deriving ownership
inputs from compiled outputs. Connecting them to ownership and planners still
requires explicit roots, consumers and destination evidence. Qualified scope
roots, writer locking,
[destination observation](0007-filesystem-access.md#architecture-qualified-destination-observation)
and
[qualified source observation](0003-sources.md#architecture-qualified-source-observation)
are implemented as separate inactive capabilities. Final-head hosted source
qualification remains required. Registry/bundle resolution, integrating
qualified source evidence into the lifecycle, combined durable state and native
Codex activation remain pending. Pure
[permission selection](0007-filesystem-access.md#architecture-file-permission-policy)
and opt-in
[qualified file execution and recovery](0008-execution-and-recovery.md#architecture-qualified-file-execution)
are implemented as separate library capabilities. Returning an OpenCode rule
file does not install its separate instruction registration. This prerequisite
changes no commands, installer behavior or schema-1 persistence and completes
neither M1-A2 nor M1-A3.

<a id="architecture-captured-rule-desired-sets"></a>

<a id="architecture-captured-rule-desired-sets-captured-rule-desired-sets"></a>

### Captured rule desired sets

The opt-in `lifecycle::desired` library binds complete captured rule compilation
to an explicit installation root and explicit target consumers. It produces
supplied-value desired files, not ownership, execution authority or native
conformance evidence. Production commands and lockfiles remain on schema 1.

<a id="architecture-captured-rule-desired-sets-installation-and-source-binding"></a>

#### Installation and source binding

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

<a id="architecture-captured-rule-desired-sets-explicit-consumers"></a>

#### Explicit consumers

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

<a id="architecture-captured-rule-desired-sets-complete-desired-files"></a>

#### Complete desired files

Every compiled entrypoint and resource becomes a `DesiredArtifact::File` under
the supplied root, preserving its raw spelling and scope. Bytes, resource
filtering, alias link rewriting and optional captured modes are retained
exactly. Generated entrypoint modes remain unspecified. Physical permission
selection and qualification remain later application work.

Files are ordered by their portable relative path and consumers by surface name.
Repeated paths within an installation conflict. These lexical checks do not
resolve symlinks, aliases, case equivalence or physical containment.

<a id="architecture-captured-rule-desired-sets-explicit-contract-production"></a>

#### Explicit contract production

`compile_rule_desired_set_with_contracts` accepts the same captured compilation
request plus explicit `TargetProfile` values and returns `ContractedDesiredSet`
containing the unchanged desired set and per-artifact owner contracts. Profiles
must exactly cover requested surfaces, be unique, match the surface's family and
carry nonzero revisions. The producer never obtains revisions from defaults.

Compilation reuses `compile_rule_desired_set`, including source identity,
alias/resource handling and audience filtering. Each generated file contract
binds the exact root/path/installation, captured source-manifest digest,
rendered byte hash, optional mode and only that output's consumers with supplied
revisions. Consumer profiles sort by surface name. Audience exclusions create no
artifact contracts, including an empty desired set. Instruction compilation
remains separate work. The original single and batch producers keep their
existing behavior and shared-path rejections.

The
[contract producer](https://github.com/driftsys/upskill/blob/main/src/lifecycle/desired/contracts.rs)
and
[acceptance examples](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_contract_producer.rs)
cover explicit profile validation and exact captured bindings. These are
recorded supplied contracts, not native qualification.

<a id="architecture-captured-rule-desired-sets-complete-batch-preflight"></a>

#### Complete batch preflight

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

<a id="architecture-captured-rule-desired-sets-interfaces-and-acceptance-evidence"></a>

#### Interfaces and acceptance evidence

The
[single-rule producer](https://github.com/driftsys/upskill/blob/main/src/lifecycle/desired/mod.rs)
defines `CompilerBinding`, `RuleDesiredSetRequest` and `DesiredInstallationSet`.
The
[batch boundary](https://github.com/driftsys/upskill/blob/main/src/lifecycle/desired/batch.rs)
exposes `DesiredSetDiagnostic`, `DesiredSetsOutcome` and
`compile_rule_desired_sets`. These supplied-value interfaces neither serialize
plans nor authorize effects.

The
[public acceptance tests](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_desired.rs)
compare literal paths, binary resource bytes, modes and checked-in renderer
goldens. They cover source identity and fallback names, exact requested and
excluded consumers, OpenCode scope refusal, raw root retention, complete ordered
batch payloads, component-boundary topology, duplicate IDs and contextual
diagnostic ordering. The
[validation note](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate5-desired-validation.md)
records local checks and the limits of the independent review.

<a id="architecture-captured-rule-desired-sets-remaining-lifecycle-boundary"></a>

#### Remaining lifecycle boundary

This producer validates each installation in a minimal direct-reference graph;
it does not reconcile the authoritative graph across the batch or establish
cross-request installation-identity consistency beyond duplicate ID rejection.
Separate
[reference reconciliation](0005-lifecycle-planning.md#architecture-lifecycle-reference-reconciliation)
and
[removal planning](0005-lifecycle-planning.md#architecture-lifecycle-reference-removal)
now establish graph retention and shared-owner retirement with authoritative
per-owner consumers. Pure
[selection preflight](0005-lifecycle-planning.md#architecture-lifecycle-selection)
now aggregates legacy/native conflicts for all/literal-name requests. The pure
[single-owner upsert coordinator](0005-lifecycle-planning.md#architecture-lifecycle-upsert)
now joins complete sets, selected graph changes and supplied destination
observations. Source/scoped selectors and CLI precedence, instruction compiler
adapters and complete lifecycle integration remain task 2b.
[Explicit shared upsert](0005-lifecycle-planning.md#architecture-lifecycle-upsert-explicit-shared-upsert)
now consumes recorded agreement through a separate inactive path. The mixed
planner's affected multi-owner rejection remains intact. Guarded
backup/application/recovery, observational adapters and native activation remain
separate tasks.
