# Captured item compilation

Status: proposed bounded implementation prerequisite. This does not activate the
[M1 lifecycle](2026-10-12-m1-owned-lifecycle-design.md).

## Intent and boundary

Produce complete, deterministic generation outputs from one validated captured
item directory. Parsing, rendering and resource selection consume the same
captured bytes. The compiler never rereads a source path or writes output.
This prepares source-derived inputs for the existing whole-file planner without
claiming source authenticity, filesystem qualification or ownership.

The user delegated routine design and delivery decisions for the customization
roadmap. This architectural slice retains a written design, implementation plan,
acceptance-first implementation and independent reviews. Its assumption is that
current rule, skill and agent rendering must remain available before general
lifecycle activation; current CLI behavior remains unchanged.

Three approaches were considered:

- Invoke the existing filesystem installer into a temporary directory. This
  would reread mutable sources, introduce effects and inherit recursive cleanup.
- Rewrite generation and resource handling as a new target adapter framework.
  This would combine compatibility work with native target qualification.
- Compile one captured item using existing pure renderers and historical layout
  helpers. This is selected: it establishes a small producer contract and keeps
  native adapters, registry discovery and execution separate.

## Public API

Add `compile::item`, exported through `compile` in the library:

```rust
pub struct ItemCompileRequest {
    pub kind: ItemKind,
    pub folder_name: String,
    pub alias: Option<String>,
    pub clients: Vec<Client>,
}

pub struct CompiledItemFile {
    pub path: String,
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}

pub struct CompiledClientItem {
    pub client: Client,
    pub files: Vec<CompiledItemFile>,
}

pub struct CompiledItem {
    pub kind: ItemKind,
    pub source_name: String,
    pub installed_name: String,
    pub source_manifest_sha256: String,
    pub outputs: Vec<CompiledClientItem>,
    pub audience_excluded: Vec<Client>,
}

pub fn compile_item(
    source: &SourceManifest,
    request: &ItemCompileRequest,
) -> anyhow::Result<CompiledItem>;
```

The four data structures derive `Debug`, `Clone`, `PartialEq` and `Eq`, without
serialization. Inputs are borrowed and unchanged; outputs own their values.
`Client` is the existing generation vocabulary, not `TargetSurface`. An output
does not prove native support, incidental-reader isolation or required semantics
for a particular surface. The future adapter supplies that contract.

## Validation and identity

Validate the complete manifest, including every digest, path, ordering and mode,
before consuming any of its files. Also reject strict file/descendant path pairs
anywhere in the manifest: a public manifest with both `a` and `a/b` cannot
represent an ordinary captured item directory. This includes ignored files.

Require a nonempty, duplicate-free requested client list. Canonical result order
is `Client::ALL`, independent of request order. Validate the folder, resolved
source name and optional alias as single portable relative components using the
ownership path predicate plus rejection of `/`. An invalid declared rule/agent
name is an error; a skill's declared name remains non-authoritative, matching
the existing installer. Host equivalence and reserved names remain unqualified.

The canonical top-level entrypoint for the requested kind is required. Decode it
strictly as UTF-8 and parse its existing typed frontmatter, including schema
rejection. Skills use the supplied folder name; rules and agents use a declared
name when present, otherwise the folder. `installed_name` is the alias when
present, otherwise this resolved source name.

Filter requested clients through the canonical `audience`. Report every excluded
client explicitly in `audience_excluded`; emit no output for it. An entirely
excluded selection succeeds with empty outputs and explicit exclusions. This
does not authorize removal of any previous installation.

## Resources and unsupported overrides

Classify resources using the existing top-level exclusion policy: exclude all
three canonical entrypoint names and recognized `<KIND>.<client>.md` override
names at the item root. Nested files with those names, unrelated names such as
`notes.claude.md`, hidden files, empty files and binary files remain resources.
Apply the requested kind's subtractive `ignore` patterns with the existing glob
semantics, preserving deterministic order. Exclusion from output never removes
that file from the captured source identity.

The format defines body overrides, but the current installer does not apply
them. This slice must not silently claim that semantics is implemented: reject
a top-level override for the requested kind and an audience-included requested
client, with a contextual unsupported-override error. Ignore patterns cannot
suppress that error. Overrides for other kinds or non-emitted clients remain
classified source inputs and are excluded from resources. Unknown client suffixes
remain resources under the existing classification policy. A later task can
implement the specified override precedence with its own compatibility tests.

## Rendering and output

Reuse the existing typed renderers, including directives, client frontmatter,
passthrough and Markdown formatting. Render with the resolved source name, as
the current installer does. An alias changes output paths and resource link
namespaces; it does not change rendered frontmatter identity.

Compute paths with the existing frozen historical layout helpers for the current
three generation clients and three item kinds. For flat kinds, first rewrite
copied-resource links using the source name, then apply existing alias
re-prefixing when the alias differs. Preserve both passes: a pre-namespaced
source link can be affected only by the second. Directory-backed output keeps
entrypoint and resource links together and requires neither pass.

Each included client output contains exactly one rendered entrypoint and every
retained resource, with portable relative paths sorted in ascending order.
Resource bytes and optional modes equal the captured values exactly. Generated
entrypoints carry `mode: None`, because source permissions are not a generated
destination permission policy. A future qualified writer must establish that
policy and persist observed mode evidence; `None` does not authorize changing
permissions. Never infer a digest for rendered content from the source digest.

Validate output paths and reject duplicate or strict ancestor/descendant output
locations before returning. Results contain no roots, owners, destination
observations, reference claims or application instructions. The recorded source
manifest digest identifies the validated supplied bytes, not provenance or an
immutable Git revision. An error returns no successful partial compilation.

## Integration and compatibility

Use Rust 1.85, synchronous existing dependencies and no library output to stdout
or stderr. Expose only the narrow existing pure helpers needed for shared naming,
resource classification and ignore filtering within the crate; preserve their
behavior and tests. Reuse existing historical layout and link-rewrite functions.
Do not route the current installer through the new compiler in this task.

Registry/bundle discovery, dependency closure, aliases across installations,
typed target surfaces, new adapter semantics, native registrations and ancillary
configuration are not this API's responsibilities. OpenCode rule registration
remains a separate effect; returning its current generated file does not claim
that configuration was installed.

## Acceptance and evidence

- Literal expected entrypoint bytes and output paths cover all nine kind/client
  pairs, with resources alongside directory-backed and flat outputs.
- Exact empty/binary/Unicode resource bytes, modes and ordering survive; ignored
  and reserved top-level files are omitted while nested equivalents remain.
- Source/frontmatter/folder/alias identity differences and both alias link passes
  are pinned by independent expected output, including already-namespaced links.
- Audience exclusions and reordered explicit selections are deterministic and
  never imply deletion. Invalid/duplicate/empty selections fail.
- Tampered digest, invalid manifest order/path/mode, impossible source topology,
  missing entrypoint, invalid UTF-8/frontmatter/schema/name and active overrides
  fail contextually with unchanged inputs and no partial result.
- A captured manifest compiles after its original directory is changed or
  removed. Creating plans from its output still requires explicit roots,
  ownership, consumers and destination observations.
- Real compile/assertion RED precedes implementation. Semantic controls must
  catch ignored manifest validation, omitted resources, changed resource bytes
  or modes, bypassed ignore filtering and skipped alias re-prefixing. Restore
  production exactly, then rerun focused tests.

Run ordered `just fmt`, `just verify`, and Rust 1.85 all-targets checking. Obtain
task and branch reviews, full PR review and hosted Linux/Windows CI before merge.
This prerequisite completes neither M1-A2 nor M1-A3. Mixed instruction/file
composition, qualified filesystem observation, locking, apply/recovery, combined
state, all lifecycle adapters and native Codex activation remain pending.
