# LSP servers in `pack` output: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

Date: 2026-11-24. Story: [#634](https://github.com/driftsys/upskill/issues/634).

**Goal:** A bundle declares LSP servers under the new key `lsps`, and
`upskill pack` writes them into each plugin directory in the format of the
client.

**Architecture:** `src/model/bundle.rs` gets the field `Bundle::lsps` and the
type `LspEntry`, with a validation function that the bundle parser and `lint`
call next to `validate_mcps`. `src/pack.rs` builds one more `PackFile` from the
closure, in the same way as `mcp_json`. The report of skipped LSP servers in
`add` and `update` is implemented in story #632 (see Task 2).

**Tech stack:** Rust 2024, `serde`, `serde_json`, `serde_yaml_ng`, `anyhow`;
integration tests with `assert_cmd` through `common::upskill_cmd`.

**Spec:** [plugin MVP design](../../wip/specs/2026-11-24-plugin-mvp-design.md),
section 5; [AD-0046](../../adr/0046-plugin-mvp-components.md), Decisions 10
and 11.

## Global constraints

- Bundle key: `lsps`, a map from a server name to `command` (required), `args`
  (optional list), `env` (optional map) and `extensions` (required, not empty; a
  map from a file extension that starts with a dot to a language identifier).
- Claude Code output: `.lsp.json` at the plugin root, one top-level key for each
  server, with `command`, `args`, `env` and `extensionToLanguage`.
- Copilot CLI output: `lsp.json` at the plugin root, the servers under the key
  `lspServers`, each with `command`, `args`, `env` and `fileExtensions`.
- Plan decision: empty `args` and `env` are omitted, as `.mcp.json` does.
- An output without LSP servers has no LSP file.
- The plugin does not install the server binary. `add`, `update`, `doctor` and
  the lockfile do not change in Task 1.
- Only `main.rs` writes to stdout and stderr. Zero warnings: `just verify` and
  `just sanity` pass.
- Comments, documentation and messages use plain, literal English.

## Review focus

1. An extension without a leading dot (`rs: rust`): the parser and `lint` report
   an error that names the server and the extension (Task 1).
2. An empty `extensions` map or an empty `command`: error (Task 1).
3. The same server name in two bundles of the closure with equal content: one
   entry; with different content: error, nothing written (Task 1).
4. A bundle with `lsps` and no rules: the LSP file exists and no `init` and no
   hook file exist (Task 1).
5. An existing bundle without `lsps` parses and serializes as before: the key is
   absent from the serialized form (Task 1).

---

### Task 1: bundle key `lsps` and the LSP file of each client

**Files:**

- Modify: `src/model/bundle.rs` (`Bundle::lsps`, `LspEntry`,
  `Bundle::validate_lsps`, unit tests)
- Modify: `src/parse/bundle.rs` (call `validate_lsps` at both places that call
  `validate_mcps`)
- Modify: `src/lint.rs` (call `validate_lsps` next to `validate_mcps`; one unit
  test)
- Modify: every place that builds a `Bundle` literal (`src/bundle.rs`,
  `src/conflict.rs` and tests): add `lsps: Default::default()`
- Modify: `src/pack.rs` (`lsp_json`, `build`, unit tests)
- Modify: `tests/cli_pack.rs`, `tests/parse_bundles.rs`
- Modify: `tests/fixtures/pack/registry/base.bundle.yaml` (add one LSP server);
  create `tests/fixtures/pack/expected/demo-claude/.lsp.json` and
  `tests/fixtures/pack/expected/demo-copilot/lsp.json`
- Modify: `docs/topics/0001-content-format.md` (bundle section, next to the
  `mcps` map, with an anchor `lsps-map`), `docs/specification/format-spec.md`
  (the link list that names `mcps-map`), `docs/book/commands.md` (section
  `upskill pack`), `docs/specification/specification.md` (section 2.8),
  `docs/book/recipes.md` and `skills/upskill-cli/SKILL.md` (the sentences that
  say what the plugin holds), `skills/upskill-writing-bundles/SKILL.md` (the
  key), `docs/roadmap.md` (stage 2: mark the LSP item as implemented in #634 for
  `pack`, as the MCP item is marked)

**Interfaces:**

- `src/model/bundle.rs`:

  ```rust
  /// One entry in the bundle `lsps:` map.
  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct LspEntry {
      pub command: String,
      #[serde(default, skip_serializing_if = "Vec::is_empty")]
      pub args: Vec<String>,
      #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
      pub env: BTreeMap<String, String>,
      pub extensions: BTreeMap<String, String>,
  }
  ```

  `Bundle` gets
  `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")] pub lsps: BTreeMap<String, LspEntry>`
  after `mcps`.
  `impl Bundle { pub fn validate_lsps(&self) -> Result<(), String> }` returns
  the first of these errors:
  - ``lsp `<name>`: command must not be empty``
  - ``lsp `<name>`: extensions must not be empty``
  - ``lsp `<name>`: extension `<ext>` must start with a dot``
  - ``lsp `<name>`: the language of extension `<ext>` must not be empty``

  Plan decision: `deny_unknown_fields` on `LspEntry`, so that a misspelled key
  (`extension:`) is an error and not silently ignored. Check first whether
  `McpLocal` rejects unknown keys; if the model types of this file accept them
  everywhere, keep `deny_unknown_fields` here and state it in the format
  document.

- `src/pack.rs`, private:
  `fn lsp_json(bundles: &[Bundle], client: PackClient) -> Result<Option<Vec<u8>>>`.
  `None` when no bundle declares a server. Pretty-printed JSON with one trailing
  newline. Claude:
  `{"<name>": {"command", "args"?, "env"?,
  "extensionToLanguage"}}`. Copilot:
  `{"lspServers": {"<name>": {"command",
  "args"?, "env"?, "fileExtensions"}}}`.
  Two declarations of one name conflict when the `LspEntry` values differ. Error
  text, exactly:
  `` LSP server `<name>` is declared with different content by the bundles `<first>` and `<second>` ``.
- `build` pushes the file with `rel` `.lsp.json` (Claude) or `lsp.json`
  (Copilot), `mode: None`, with or without rules.

- [ ] **Step 1: Write the failing tests**

  `tests/parse_bundles.rs` (or the unit tests of `src/model/bundle.rs`, where
  the `mcps` tests are):

  - `bundle_parses_lsps`: the YAML of section 5 of the design gives one entry
    with command `rust-analyzer` and `extensions` `{".rs": "rust"}`.
  - `bundle_without_lsps_serializes_without_the_key`.
  - `validate_lsps_reports_each_invalid_entry`: one case for each of the four
    error texts above, compared with `assert_eq!`.
  - `lsp_entry_refuses_an_unknown_key`.

  `src/lint.rs`: `lint_flags_an_lsp_extension_without_a_dot_in_bundle`, in the
  form of `lint_flags_invalid_mcps_transport_in_bundle`.

  `src/pack.rs` unit tests:

  - `lsp_json_is_none_without_servers`.
  - `lsp_json_writes_the_format_of_each_client`: a bundle with the server `ra`
    (`command: rust-analyzer`, `args: [--stdio]`, `env: { RA_LOG: info }`,
    `extensions: { .rs: rust }`). Assert the exact string for Claude:

    ```json
    {
      "ra": {
        "args": [
          "--stdio"
        ],
        "command": "rust-analyzer",
        "env": {
          "RA_LOG": "info"
        },
        "extensionToLanguage": {
          ".rs": "rust"
        }
      }
    }
    ```

    and for Copilot the same server object below `lspServers`, with
    `fileExtensions` in place of `extensionToLanguage`.
  - `lsp_json_refuses_one_name_with_different_content`: one case each for a
    difference in `command`, `args`, `env` and `extensions`; the error text is
    compared with `assert_eq!`. Equal content in two bundles gives one entry.

  `tests/cli_pack.rs`:

  - Add to `base.bundle.yaml`:

    ```yaml
    lsps:
      base-lsp:
        command: base-lsp-command
        extensions:
          .base: base
    ```

  - `pack_writes_the_lsp_file_of_each_client`: pack `demo` for both clients;
    `demo-claude/.lsp.json` equals
    `{"base-lsp": {"command": "base-lsp-command", "extensionToLanguage": {".base": "base"}}}`;
    `demo-copilot/lsp.json` equals
    `{"lspServers": {"base-lsp": {"command": "base-lsp-command", "fileExtensions": {".base": "base"}}}}`;
    `demo-claude/lsp.json` and `demo-copilot/.lsp.json` do not exist.
  - `pack_of_a_bundle_without_lsp_servers_writes_no_lsp_file`: pack `norules`
    for both clients; neither file exists in either directory.
  - `pack_of_a_bundle_with_lsp_servers_and_no_rules_writes_no_init`: append an
    `lsps` map to `norules.bundle.yaml` in the temporary registry; the LSP file
    exists for both clients; no `init` file and no hook file exist.
  - `pack_refuses_one_lsp_server_name_with_different_content_in_the_closure`:
    append `lsps` with `base-lsp` and another command to `demo.bundle.yaml`;
    exit code 1, stderr contains the error text with `base` and `demo`, `f.out`
    does not exist.
  - `pack_refuses_a_bundle_with_an_invalid_lsp_entry`: an extension without a
    dot; exit code 1; stderr contains ``extension `rs` must start with a dot``;
    `f.out` does not exist.
  - Update the file count that an existing test asserts on stdout.

- [ ] **Step 2: Run `cargo test --test cli_pack --test parse_bundles` and
      `cargo test --lib` and confirm that the new tests fail**

- [ ] **Step 3: Implement the model, the validation calls, `lsp_json` and the
      call in `build`**

- [ ] **Step 4: Write the two golden fixture files with the bytes that `pack`
      writes**

  Run `git check-ignore -v` on both paths and report the result.

- [ ] **Step 5: Update the documentation listed under Files**

  State: the key and its fields; the two output files and their keys; that empty
  `args` and `env` are omitted; that an output without LSP servers has no LSP
  file; that the plugin does not install the server binary; the conflict error
  with exit code 1; that `upskill add` and `upskill update` do not configure LSP
  servers. Change every sentence in these files that this task makes false. Do
  not state that `add` reports skipped servers: Task 2 adds that.

- [ ] **Step 6: Run `cargo test` and
      `cargo clippy --all-targets -- -D
  warnings`**

  The implementer does not commit. The orchestrator runs `just fmt`,
  `just verify` and `just sanity`, and commits with
  `feat(pack): add the bundle key lsps and write LSP servers into the plugin (#634)`.

### Task 2: moved to story #632

Decision of 2026-11-24: the report of skipped LSP servers in `add` and `update`
needs the skip mechanism that story
[#632](https://github.com/driftsys/upskill/issues/632) adds in its Task 2. It is
implemented there, as one more value of the enum of skipped kinds, after this
branch is merged. This branch delivers Task 1 only, and the pull request refers
to #634 without closing it.
