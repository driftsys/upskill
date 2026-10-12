# Captured item compilation

This record describes the implemented pure producer in
[`compile::item`](../../src/compile/item.rs). [Specification §4.5](../specification/specification.md#45-captured-item-compilation)
owns its requirements. The producer consumes one supplied captured manifest and
returns complete current-generation outputs; the current installer does not call
it. [AD-0024](../adr/0024-pure-captured-item-compilation.md) records the bounded
compatibility choice and its alternatives.

## Public boundary

`compile_item` borrows a `SourceManifest` and `ItemCompileRequest`, returning an
owned `CompiledItem` through `anyhow::Result`. The four public structures are
`ItemCompileRequest`, `CompiledItemFile`, `CompiledClientItem` and `CompiledItem`.
They derive `Debug`, `Clone`, `PartialEq` and `Eq` without serialization. The
request supplies the existing `ItemKind`, folder name, optional installed alias
and explicit generation clients. Each client result contains files with portable
relative paths, exact bytes and optional modes; the outer result retains kind,
resolved source identity, installed name, supplied manifest digest and explicit
audience exclusions. The [source module](../../src/source_manifest.rs) owns
capture and manifest validation. No source path is passed to the compiler.

The `Client` vocabulary covers Claude, Copilot and OpenCode generation. It does
not establish `TargetSurface` conformance, native support, required target
semantics or incidental-reader isolation. Results contain no roots, owners,
destination observations, reference claims or application instructions.

## Validation and identity

Compilation starts with complete manifest validation, including files later
ignored or reserved. An additional topology check rejects duplicate paths and
strict slash-component file/descendant pairs throughout the supplied source.
The folder and optional alias are validated as single portable components using
the ownership path predicate and slash rejection. Requested clients must be
nonempty and unique.

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

## Resources and unsupported overrides

The producer shares the existing crate-private resource classifier and
subtractive ignore filter with the installer through narrow re-exports in
[`pipeline`](../../src/pipeline/mod.rs). Their bodies and historical call paths
are unchanged. Only recognized canonical entrypoints and known override names
at the root are reserved. Nested equivalents, hidden files, unrelated names,
unknown-client suffixes and empty or binary files remain resources unless
ignored. Selection retains original captured values in manifest order.

For each requested client, canonical audience exclusion is recorded explicitly.
An entirely excluded request returns empty outputs and all requested exclusions;
it grants no removal authority. Before rendering an included client, compilation
rejects a matching root override for the requested kind, independently of ignore
patterns. Other-kind and non-emitted-client overrides remain reserved source
inputs. [AD-0025](../adr/0025-reject-active-captured-overrides.md) records the
unsupported boundary; body override precedence remains unimplemented.

## Rendering and output

Existing typed renderers retain directives, passthrough, client frontmatter and
Markdown formatting. Frozen historical [layout helpers](../../src/legacy/layout.rs)
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
carry `mode: None`; [AD-0026](../adr/0026-separate-generated-permission-policy.md)
records why destination permission policy requires separate qualification.

## Evidence and downstream integration

[`tests/compile_items.rs`](../../tests/compile_items.rs) pins all nine kind/client
pairs with independent expected rendering and paths, complete resource vectors,
nonuniform resource modes, explicit audiences for all clients, both flat alias
link passes, source identity despite reserved or ignored output, invalid inputs,
impossible topology and source deletion after capture. The tests establish
supplied-value compilation and compatibility, without filesystem execution or
native-target conformance.

The producer prepares complete source-derived sets for future lifecycle
coordination. The implemented [mixed coordinator](mixed-owned-artifact-planning.md)
composes explicit complete file/contribution sets, without deriving ownership
inputs from compiled outputs. Connecting them to ownership and planners still
requires explicit roots, consumers and destination evidence. Registry/bundle resolution, qualified
source and destination observation, writer locking, destination permission policy,
durable state, apply/recovery and native Codex activation remain pending. Returning an OpenCode rule file does not install
its separate instruction registration. This prerequisite changes no commands,
installer behavior or schema-1 persistence and completes neither M1-A2 nor M1-A3.
