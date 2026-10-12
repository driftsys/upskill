# Captured item compilation implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compile one validated captured item into complete deterministic generation outputs without source rereads or destination effects.

**Architecture:** A pure producer reuses existing typed renderers, historical layouts, naming, resource classification, ignore filtering and link rewriting. Its client vocabulary is generation compatibility, not native target qualification. The current installer remains on its current path.

**Tech Stack:** Rust 1.85, synchronous standard library, existing `anyhow`, `sha2`, YAML parsing and Markdown generation dependencies.

**Spec:** [Captured item compilation design](../specs/2026-10-12-captured-item-compilation-design.md).

## Global Constraints

- The compiler never rereads a source path or writes output.
- Inputs are borrowed and unchanged; outputs own their values.
- Validate the complete manifest, including every digest, path, ordering and mode, before consuming any of its files.
- Require a nonempty, duplicate-free requested client list.
- Canonical result order is `Client::ALL`, independent of request order.
- An entirely excluded selection succeeds with empty outputs and explicit exclusions.
- This does not authorize removal of any previous installation.
- Exclusion from output never removes that file from the captured source identity.
- Render with the resolved source name, as the current installer does.
- An alias changes output paths and resource link namespaces; it does not change rendered frontmatter identity.
- Resource bytes and optional modes equal the captured values exactly.
- Generated entrypoints carry `mode: None`, because source permissions are not a generated destination permission policy.
- An error returns no successful partial compilation.
- Use Rust 1.85, synchronous existing dependencies and no library output to stdout or stderr.
- Do not route the current installer through the new compiler in this task.

## Review Focus

1. Ignored, excluded-client and non-entrypoint files still participate in source validation and identity; invalid or impossible public manifests cannot bypass validation.
2. Aliasing preserves original frontmatter and both legacy link passes, including pre-namespaced links, URLs, fragments and code examples.
3. Resource classification distinguishes reserved root names from nested equivalents and unrelated or unknown-client suffixes, preserving binary/empty bytes and exact modes.
4. No client output due to audience filtering is explicit and cannot become silent removal; active overrides cannot be hidden by ignore patterns.
5. Captured bytes remain sufficient after source deletion; exact nine-pair golden output, deterministic selection order and unchanged inputs prevent accidental filesystem or renderer-policy coupling.

## File structure

| File                                                 | Responsibility                                                           |
| ---------------------------------------------------- | ------------------------------------------------------------------------ |
| `src/compile/mod.rs`                                 | Export item compilation                                                  |
| `src/compile/item.rs`                                | Public DTOs, validation, captured parsing/rendering and complete outputs |
| `src/lib.rs`                                         | Export `compile`                                                         |
| `src/pipeline/mod.rs`                                | Crate-only re-exports of existing pure helpers                           |
| `src/pipeline/discovery.rs`                          | Narrow visibility changes for existing naming/classification helpers     |
| `src/pipeline/ignore.rs`                             | Narrow visibility change for existing filtering helper                   |
| `tests/compile_items.rs`                             | Public acceptance contract and semantic controls                         |
| `docs/specification/specification.md`                | As-built API/qualification boundary                                      |
| `docs/book/commands.md`                              | Current commands retain existing behavior                                |
| `docs/wip/plans/2026-10-11-instructions-rules-m1.md` | Record bounded prerequisite and remaining activation gates               |

---

### Task 1: Compile complete captured items

**Files:** All files listed above. Modify this plan's checkboxes as evidence is completed. Do not modify the parent lifecycle design, current installer behavior, model mapping, ownership/planner semantics or other WIP documents.

**Interfaces consumed:**

```rust
source_manifest::validate_manifest(&SourceManifest) -> anyhow::Result<()>;
parse::frontmatter::parse::<Skill>(&str) -> anyhow::Result<(Skill, &str)>;
parse::frontmatter::parse::<Rule>(&str) -> anyhow::Result<(Rule, &str)>;
parse::frontmatter::parse::<Agent>(&str) -> anyhow::Result<(Agent, &str)>;
generate::render_skill(&Skill, &str, &str, Client) -> anyhow::Result<String>;
generate::render_rule(&Rule, &str, &str, Client) -> anyhow::Result<String>;
generate::render_agent(&Agent, &str, &str, Client) -> anyhow::Result<String>;
generate::link_rewrite::rewrite_resource_links(&str, &str, &HashSet<PathBuf>) -> anyhow::Result<String>;
generate::link_rewrite::reprefix_resource_links(&str, &str, &str, &HashSet<PathBuf>) -> anyhow::Result<String>;
```

Also consume the existing crate-private `legacy::layout::{output_path, resource_base_path, is_dir_backed}` using `client.into()`, and `ownership::validate_relative_path`.

**Interfaces produced:** The four exact public structures and `compile_item(&SourceManifest, &ItemCompileRequest) -> anyhow::Result<CompiledItem>` in the spec. These are the only new public API; do not add serialization, source fetching, ownership, planner calls or native outcomes.

- [ ] **Step 1: Write the creation acceptance test and capture compile RED.**

Create `tests/compile_items.rs`. Use `capture_source` to build valid normal fixtures before invoking the new API. This helper writes only into a test tempdir and returns captured values; keep the compiler call separate:

```rust
use std::fs;
use upskill::compile::item::{compile_item, ItemCompileRequest};
use upskill::generate::Client;
use upskill::pipeline::ItemKind;
use upskill::source_manifest::{capture_source, SourceManifest};

fn fixture(files: &[(&str, &[u8])]) -> SourceManifest {
    let directory = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let destination = directory.path().join(path);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(destination, bytes).unwrap();
    }
    capture_source(directory.path()).unwrap()
}

#[test]
fn captured_skill_emits_entrypoint_and_exact_binary_resource() {
    let source = fixture(&[
        ("SKILL.md", b"---\nschema: 1\ndescription: demo\n---\n\n# Demo\n"),
        ("assets/data.bin", &[0, 255]),
    ]);
    let request = ItemCompileRequest {
        kind: ItemKind::Skill,
        folder_name: "demo".into(),
        alias: None,
        clients: vec![Client::Claude],
    };
    let before = source.clone();
    let result = compile_item(&source, &request).expect("compile captured skill");
    assert_eq!(result.outputs.len(), 1);
    let files = &result.outputs[0].files;
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, ".claude/skills/demo/SKILL.md");
    assert_eq!(files[0].bytes, b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n");
    assert_eq!(files[0].mode, None);
    assert_eq!(files[1].path, ".claude/skills/demo/assets/data.bin");
    assert_eq!(files[1].bytes, [0, 255]);
    assert_eq!(files[1].mode, source.files[1].mode);
    assert_eq!(source, before);
}
```

Run `cargo test --test compile_items captured_skill_emits_entrypoint_and_exact_binary_resource -- --exact`. Save genuine missing-module/API compile output; no real compiler logic yet. The tempdir in `fixture` is dropped before compilation, proving that the API needs only the captured values.

- [ ] **Step 2: Add only DTOs and an empty-output stub; obtain genuine assertion RED.**

Copy the spec's exact DTO fields with `Debug, Clone, PartialEq, Eq`. Export `compile::item`. The temporary stub returns an owned `CompiledItem` with empty `outputs` and `audience_excluded`, the supplied kind/folder/alias and source digest. Run the exact creation test again: it must compile and fail its `outputs.len()` assertion. Save the output before real validation/rendering logic.

Add an input-validation test that corrupts `source.content_sha256` and asserts a contextual rejection:

```rust
let result = compile_item(&source, &request);
assert!(result.is_err(), "a tampered source manifest must be rejected");
```

Run it against the same stub and capture genuine assertion RED. Fixture/schema/type errors are not behavioral RED; correct those first. Write the initial report checkpoint before drafting real logic.

- [ ] **Step 3: Extend acceptance coverage before implementing the compiler.**

Use fixed expected bytes and paths, including existing checked-in golden sources and outputs for `create-api-endpoint`, `api-conventions` and `security-reviewer` under `tests/fixtures/items` and `tests/fixtures/expected`. Do not generate expected output by calling the renderer/compiler being tested. Pin all nine kind/client combinations. Capture valid sources from temporary copied fixture directories rather than mutating repository fixtures.

Add cases for every Review Focus line:

| Input                                                                                           | Required assertion                                                                                                            |
| ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Multiple binary/empty/Unicode/hidden resources                                                  | Exact complete sorted file vectors, content, captured modes and `mode: None` for generated entrypoint                         |
| Root `RULE.md`, `AGENT.md`, every recognized override plus nested equivalents                   | Root reserved names excluded; nested equivalents and `notes.claude.md` retained                                               |
| Ignore `*.log`, `scripts/**`, `a/*.txt`, `?.md`, bare directory name                            | Same existing subtractive semantics; ignored resources never emitted or link-rewritten                                        |
| Rule/agent declared name differs from folder; skill declared name differs                       | Effective source name follows existing policy; exact header and output names                                                  |
| Alias differs from source name, flat vs directory-backed output                                 | Header retains source identity; output/resource paths use alias; both ordinary and pre-namespaced links reach alias namespace |
| Relative link, image, reference definition, URL, fragment, missing resource and code span/fence | Rewrite only existing copied resource destinations; exact unaffected examples retained                                        |
| Reordered requested clients                                                                     | Exact identical outputs/exclusions ordered by `Client::ALL`                                                                   |
| Empty/duplicate requested client list                                                           | Contextual error, unchanged inputs                                                                                            |
| Audience subset and empty audience                                                              | Explicit exclusions for each requested omitted client; no inferred deletion, successful empty result when all excluded        |
| Active matching override plus ignore pattern for that filename                                  | Unsupported-override error regardless of ignore                                                                               |
| Other-kind/non-emitted-client overrides; unknown-client suffix                                  | Classified exclusions vs ordinary resource behavior as specified                                                              |
| Invalid path, order, file digest, aggregate digest or mode, including an ignored file           | Manifest rejection before compilation; no partial result                                                                      |
| Canonical entrypoint missing, binary, malformed YAML, unsupported schema                        | Specific contextual failure                                                                                                   |
| Invalid folder/alias/effective rule or agent name                                               | Reject slash, empty, dot/dot-dot, backslash, colon and control characters; preserve valid Unicode                             |
| Valid synthetic manifest with `a` and `a/b`                                                     | Reject impossible source topology including ignored paths                                                                     |
| Original source changed/deleted after capture                                                   | Exact captured output and source identity; no source reread                                                                   |

For impossible topology and independent mode values, build synthetic manifests in test code by the documented framed protocol, rather than adding a production manifest-builder API. First assert `validate_manifest(&source)` succeeds for the synthetic fixture, so the compiler's extra topology assertion is actually exercised. Input hashing helpers may use SHA-256; expected generated bytes and critical resource outputs remain independent literals.

- [ ] **Step 4: Expose narrow pure helpers without changing their behavior.**

Change `effective_name` and `is_entrypoint_or_override` in `pipeline/discovery.rs` to `pub(crate)`. Change `filter_ignored` in `pipeline/ignore.rs` to `pub(crate)`. Re-export only those names from the otherwise private modules in `pipeline/mod.rs`:

```rust
pub(crate) use discovery::{effective_name, is_entrypoint_or_override};
pub(crate) use ignore::filter_ignored;
```

Keep the existing pipeline call paths, helper bodies, discovery error policy and tests unchanged. This is crate-only access for shared policy, not a new public discovery API.

- [ ] **Step 5: Implement captured validation, typed rendering and complete outputs.**

Start with full `validate_manifest(source)` and contextual errors. Check all source file/descendant pairs. Validate folder/alias and explicit selection before parsing. Decode only the requested canonical entrypoint with `std::str::from_utf8`; use typed `frontmatter::parse` and shared `effective_name`, then validate the resolved name. A single-component check is:

```rust
fn validate_name(name: &str) -> anyhow::Result<()> {
    crate::ownership::validate_relative_path(name)?;
    if name.contains('/') {
        anyhow::bail!("item name must be a single portable component");
    }
    Ok(())
}
```

Dispatch by `ItemKind` to parse the existing model and collect its audience/ignore and per-client rendered strings. Iterate `Client::ALL`, including only explicitly requested clients. Record audience exclusions; check active override presence before rendering and independently of ignore. Use contextual `render_skill`, `render_rule` or `render_agent` calls with the source name.

Classify source resources by root-only shared exclusion, convert their valid portable strings to `PathBuf` for the shared ignore helper and link-rewrite set, and retain the corresponding original captured values. Never read files or convert arbitrary paths lossily. Compute existing output/resource paths through `legacy::layout` and the installed alias. Convert generated paths to portable strings by collecting UTF-8 normal components and joining them with `/`; reject non-normal/non-UTF-8 components rather than using display strings or lossy conversion. This preserves portable output on Windows where `PathBuf::join` can introduce native separators. Validate each output string and preserve the original source manifest digest.

For flat outputs with copied resources, call `rewrite_resource_links` with source name, then `reprefix_resource_links` with source and installed names when different. Emit the generated entrypoint with no permission evidence and resources with exact cloned bytes/modes. Sort each output's files by portable path; reject duplicates and strict ancestor conflicts. Assemble complete client outputs and exclusions in declaration order. No output is returned on error.

- [ ] **Step 6: Run focused tests and genuine semantic controls.**

Run `cargo test --test compile_items`; fix only implementation/fixture defects consistent with the spec. Independently exercise these controls: bypass manifest validation, omit a retained resource, change resource bytes, erase a captured mode, bypass ignore filtering and skip alias re-prefixing. Each must fail an intended behavioral assertion, not fail compilation or fixture setup. Use `try/finally` restoration of exact production bytes, compare restored files to saved bytes, and rerun focused GREEN. Save outputs and report actual results; do not claim unexecuted mutations.

Run existing regression coverage with `cargo test --test cli_resources`, `cargo test --test generate_skills`, `cargo test --test generate_rules`, `cargo test --test generate_agents` and `cargo test --test source_manifests`. Do not rerun baseline unnecessarily.

- [ ] **Step 7: Update canonical docs and run final gates.**

Add an as-built specification section with the actual API, validation, compatibility, override rejection, mode boundary and deferred activation. Add a brief command-guide note that the library producer does not activate lifecycle changes. Update only the M1 plan's bounded prerequisite context and incoming link to this design; retain pending coordinator/qualification/state/executor/target gates.

Run ordered `just fmt`, `just verify`, `cargo +1.85.0 check --all-targets --locked`, then `git diff --check`. Save full output and a detailed report in the plan's ignored SDD workspace: compile/assertion RED chronology, implemented fields/files, focused results, each semantic control/restoration, gate results, deviations, self-review and limits. Bootstrap and baseline are controller-owned and already recorded.

- [ ] **Step 8: Commit the task and hand off for review.**

Stage only assigned code, tests, canonical docs and plan checkboxes. Commit with `feat: compile captured item outputs`; return exact HEAD and clean status. No push or PR by the implementer. The controller owns independent task review, exact-pair gardening, broad whole-branch review, final verification, PR, full review, CI and squash merge. Any explicitly triggered automatic QUICK is advisory only and does not replace those gates; no review cycle over its fixes.
