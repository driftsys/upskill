# MCP servers in `pack` output: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

Date: 2026-11-24. Story: [#633](https://github.com/driftsys/upskill/issues/633).

**Goal:** `upskill pack` writes the MCP servers of the bundle closure to
`.mcp.json` at the root of each plugin directory, and the `init` command no
longer lists MCP servers.

**Architecture:** `src/pack.rs` builds one more `PackFile` from `output.bundles`
(the closure, required bundles first, entry bundle last). The server objects
come from the two existing value functions of `src/ancillary.rs`, which become
`pub(crate)`. No new module, no new dependency, no change to the bundle schema.

**Tech stack:** Rust 2024, `serde_json`, `anyhow`; integration tests with
`assert_cmd` through `common::upskill_cmd`.

**Spec:** [plugin MVP design](../../wip/specs/2026-11-24-plugin-mvp-design.md),
sections 3 and 4; [AD-0046](../../adr/0046-plugin-mvp-components.md), Decisions
2, 9 and 11.

## Global constraints

- The file is `.mcp.json` at the plugin root in both the Claude Code output and
  the Copilot CLI output. The top-level key is `mcpServers`.
- A local server is written as `command`, `args`, `env`. A remote server is
  written as `type`, `url`, `headers`. Empty `args`, `env` and `headers` are
  omitted, as `src/ancillary.rs` does today.
- `requires-env` is not written.
- An output without MCP servers has no `.mcp.json`.
- Only `main.rs` writes to stdout and stderr. `src/pack.rs` returns
  `anyhow::Result`.
- Zero warnings: `just verify` and `just sanity` pass.
- Do not change `upskill add`, `update`, `remove-mcp` or `doctor`.
- Comments, documentation and messages use plain, literal English.

## Review focus

1. The same server name in two bundles of the closure with equal content: one
   entry, no error (test in Task 1).
2. The same server name with content that differs only in `requires-env`: no
   error, because `requires-env` is not written (test in Task 1).
3. A bundle without rules and with MCP servers: `.mcp.json` exists, and no
   `init` command and no hook file exist (test in Task 1).
4. Byte stability: two runs of `pack` write equal `.mcp.json` bytes; server
   names are in sorted order (the golden fixture covers this).
5. A failed `pack` because of a conflict writes nothing below `--out` (test in
   Task 1).

---

### Task 1: `.mcp.json` in the plugin directory

**Files:**

- Modify: `src/ancillary.rs` (visibility of `mcp_servers_local_value` and
  `mcp_servers_remote_value`: `pub(crate)`)
- Modify: `src/pack.rs` (`init_text`, `build`, new private `mcp_json`, unit
  tests)
- Modify: `tests/cli_pack.rs`
- Create: `tests/fixtures/pack/expected/demo-claude/.mcp.json`,
  `tests/fixtures/pack/expected/demo-copilot/.mcp.json`
- Modify: `tests/fixtures/pack/expected/demo-claude/commands/init.md`,
  `tests/fixtures/pack/expected/demo-copilot/commands/init.md` (the two
  `- MCP server` lines are removed)
- Modify: `docs/book/commands.md` (section `upskill pack`),
  `docs/specification/specification.md` (section 2.8)

**Interfaces:**

- Produces, private to `src/pack.rs`:
  `fn mcp_json(bundles: &[Bundle]) -> Result<Option<Vec<u8>>>`. It returns
  `None` when no bundle declares a server. Otherwise it returns pretty-printed
  JSON with one trailing newline: `{"mcpServers": {"<name>": {...}}}`.
- Conflict rule: two bundles declare the same name and the written JSON objects
  differ. The error text is exactly
  `` MCP server `<name>` is declared with different content by the bundles `<first>` and `<second>` ``,
  where `<first>` is the bundle that comes first in `bundles`. `main.rs` already
  maps an error of `build` to exit code 1.
- `build` pushes `PackFile { rel: ".mcp.json".into(), bytes, mode: None }` when
  `mcp_json(&output.bundles)?` is `Some`, for both clients, with or without
  rules.

The fixture registry already declares a local server (`base-server` in
`base.bundle.yaml`, command `base-command`) and a remote server (`srv` in
`demo.bundle.yaml`, type `http`, URL `https://example.com/mcp`).

- [ ] **Step 1: Write the failing integration tests in `tests/cli_pack.rs`**

  - Rename
    `pack_init_lists_plugins_and_mcps_from_required_bundles_for_both_clients` to
    `pack_init_lists_plugins_and_no_mcp_server_for_both_clients`. Replace the
    MCP assertion by `assert!(!text.contains("MCP server"));`.
  - `pack_writes_mcp_json_with_the_servers_of_the_closure_for_both_clients`: run
    `f.command_for_clients("demo")`; for `claude` and `copilot`, parse
    `demo-<client>/.mcp.json` and assert that it equals

    ```json
    {
      "mcpServers": {
        "base-server": { "command": "base-command" },
        "srv": { "type": "http", "url": "https://example.com/mcp" }
      }
    }
    ```

  - `pack_of_a_bundle_without_mcp_servers_writes_no_mcp_json`: pack `norules`;
    assert that `norules-claude/.mcp.json` does not exist.
  - `pack_of_a_bundle_with_mcp_servers_and_no_rules_writes_mcp_json_only`:
    append an `mcps` map with one local server to `norules.bundle.yaml` in the
    temporary registry; pack `norules`; assert that `.mcp.json` exists and that
    `commands/init.md` and `hooks/hooks.json` do not exist.
  - `pack_refuses_one_server_name_with_different_content_in_the_closure`: append
    to `demo.bundle.yaml` in the temporary registry a server `base-server` with
    `local: { command: other-command }`; assert exit code 1, stderr contains
    `` MCP server `base-server` is declared with different content by the bundles `base` and `demo` ``,
    and `f.out` does not exist.
  - `pack_accepts_one_server_name_with_equal_content_in_the_closure`: append to
    `demo.bundle.yaml` a server `base-server` with
    `local: { command: base-command }` and `requires-env: [TOKEN]`; assert
    success and that `mcpServers` has exactly the keys `base-server` and `srv`.

- [ ] **Step 2: Run `cargo test --test cli_pack` and confirm that the new tests
      fail**

- [ ] **Step 3: Implement**

  - `src/ancillary.rs`: make the two value functions `pub(crate)`. Change
    nothing else in that file.
  - `src/pack.rs`: add `mcp_json` as specified under Interfaces; call it in
    `build`; remove the MCP list from `init_text` (the `mcps` set, the
    `|| !mcps.is_empty()` condition and the `- MCP server` loop); remove the
    imports that become unused.
  - Update the unit tests of `src/pack.rs` that assert the MCP line of
    `init_text`: they assert that the text does not contain `MCP server`.

- [ ] **Step 4: Update the golden fixtures**

  Write the two `.mcp.json` files with the bytes that `pack` writes, and remove
  the `- MCP server` lines from the two `commands/init.md` files. When no plugin
  line remains for a client, the whole "does not install the following"
  paragraph is absent.

- [ ] **Step 5: Update the documentation**

  In `docs/book/commands.md` (section `upskill pack`) and in section 2.8 of
  `docs/specification/specification.md`, which hold the same text:

  - State that `pack` writes `.mcp.json` at the root of each client directory,
    with the MCP servers of the bundle closure under `mcpServers`; that an
    output without MCP servers has no such file; that `requires-env` is not
    written; that the client adds and removes the servers with the plugin; and
    that one server name with different content in two bundles of the closure
    fails with exit code 1.
  - Remove "and MCP servers from the closure" from the sentence about what
    `init` lists.
  - In "Known limits", change "plugin and MCP packaging" so that it no longer
    names MCP packaging as outside the scope.
  - Change any other sentence of these two sections that this task makes false.

- [ ] **Step 6: Run `cargo test` and confirm that all tests pass**

  The implementer does not commit. The orchestrator runs `just fmt`,
  `just verify` and `just sanity`, and commits with
  `feat(pack): write the MCP servers of a bundle into its plugin (#633)`.
