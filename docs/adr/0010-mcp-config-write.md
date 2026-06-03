# MCP server configuration — bundle `mcps:`, CLI-first config-write

**Status**: Accepted (2026-06-03)

## Context

[ADR-0001](./0001-multi-kind-compiler-architecture.md) and
[ADR-0003](./0003-generation-pipeline.md) established the SSOT-to-files
contract: upskill reads SSOT items, generates per-client output files, and
never invokes the host client at runtime.

[ADR-0008](./0008-plugin-install-shellout.md) extended that contract with
a `plugins:` map on Bundle that shells out to each client's plugin CLI at
install time. MCP servers need the same extension: a skill that relies on
an MCP server (e.g. a "draw.io diagrams" skill referencing
`DrawIO:create_diagram`) is inoperable if that server was never configured
in the client. Today the two halves of a coherent capability — the skill
content and its server — are delivered separately and manually wired together
by the consumer.

### What an MCP server actually needs from an install tool

MCP servers reach a client in one of two transport shapes:

- **Remote** — a hosted server the client connects to over `http` or `sse`
  at a URL. Already running; auth is OAuth (client-driven) or a static token
  in a header.
- **Local (stdio)** — the client spawns a local command (with args/env) and
  speaks MCP over stdio. When the command is a self-fetching launcher
  (`npx -y <pkg>`, `uvx <pkg>`, `docker run …`) the launcher downloads and
  caches the server on first run — upskill installs nothing.

The only case that genuinely needs a real install step is a bare binary with
no launcher (a compiled tool, a Homebrew formula) that must be
detected → version-checked → installed → set up. That heterogeneous case is
out of scope for v1 and is noted in the Deferred section below.

### Trust posture (non-negotiable invariant)

`upskill add` is safe-by-construction: it only writes files into known
paths and shells out to known client CLIs with declared args. Adding an
arbitrary fetched bundle cannot execute arbitrary code.

This ADR preserves that invariant. upskill itself never downloads, builds,
or runs server code. The v1 implementation writes config (via the client's
MCP CLI verb, or a config file) — the same class of action as the plugin
shellout in ADR-0008.

## Decision

### Bundle gains an optional `mcps:` map

This ADR adds one new top-level key, `mcps:`, to the Bundle schema defined
in [ADR-0007](./0007-bundle-yaml-format.md), as a direct parallel of the
`plugins:` map introduced by ADR-0008. The `mcps:` key is a sibling of
`plugins:` in the YAML schema.

```yaml
schema: 1
name: drawio-diagrams
description: Draw.io diagram skill and its MCP server
items:
  skills:
    - drawio-diagrams
mcps:
  drawio:
    remote:
      type: http # http | sse
      url: https://mcp.draw.io/mcp
      headers:
        Authorization: "Bearer ${DRAWIO_TOKEN}"
    requires-env: [DRAWIO_TOKEN]
```

Or with a local (stdio) launcher:

```yaml
mcps:
  drawio:
    local:
      command: npx
      args: ["-y", "drawio-mcp-server"]
      env:
        DRAWIO_TOKEN: "${DRAWIO_TOKEN}"
    requires-env: [DRAWIO_TOKEN]
```

- The map key (`drawio`) is the upskill-level identity — used in the
  lockfile, CLI output, and `upskill remove-mcp <name>`.
- Each entry carries **exactly one** of `remote:` or `local:`. The
  `validate_mcps` function (`src/model/bundle.rs`) enforces this at parse
  time; both or neither is an error.
- `remote.type` MUST be one of `http` or `sse`; `remote.url` MUST not be
  empty; `local.command` MUST not be empty.

### Secrets — `${VAR}` indirection, declare-and-check

upskill never owns a secret value. The rule:

- Config values that reference secrets MUST use `${VAR}` indirection only.
  upskill writes the reference verbatim; the **client** expands it at
  connect/launch time; the value lives in the user's environment or the
  client's OAuth store.
- upskill MUST NOT persist a literal secret — not into a client config
  file, not into the lockfile, not into SSOT.
- `requires-env: [VAR, …]` is a **declaration** (not a value). It lets
  `upskill doctor` warn that an MCP server needs a variable that is not set
  in the current environment, without ever reading the value.

This is the same posture upskill already takes for git auth (tokens come
from env / `gh` / `glab`, never persisted).

### Per-client install contract — CLI-first, config-write fallback

v1 targets **Claude Code only**. opencode, VS Code, and GitHub Copilot are
deferred to future releases (see Deferred section). An MCP entry in a
bundle that arrives on a machine where only non-Claude clients are targeted
produces a warn-skip — not an error.

For Claude Code, the preference order is:

1. **CLI-first**: shell out to `claude mcp add`. The specific form depends
   on transport:
   - Local (stdio):
     `claude mcp add <name> --scope <scope> [-e KEY=${VAR} …] -- <command> [args…]`
   - Remote (http/sse):
     `claude mcp add --transport <http|sse> <name> <url> --scope <scope> [-H "Header: ${VAR}" …]`
2. **Config-write fallback**: when `claude` is not on PATH
   (`ErrorKind::NotFound`), fall back to writing `.mcp.json` in the project
   root (for project scope) or the user config location (for global scope).
   The `.mcp.json` shape is `{ "mcpServers": { "<name>": { … } } }`.

Config-write fallback is never the primary path — the same stance
ADR-0008 took against patching `settings.json` as the primary plugin
install mechanism.

Claude's `--scope` derives from upskill's project/global context exactly
as plugins do:

| upskill scope | `claude --scope` |
| ------------- | ---------------- |
| project       | `project`        |
| global        | `user`           |

### CLI-missing policy: warn-skip

If `claude` is not on PATH and the config-write fallback also fails,
upskill prints a warning and continues installing the rest of the bundle.
The rules, skills, and agents portion of the bundle MUST install regardless
of MCP configuration success.

### Lockfile lifecycle

`LockedMcp` is recorded per `(name, client)` pair (parallel to
`LockedPlugin` from ADR-0008):

```json
{
  "name": "drawio",
  "client": "claude",
  "scope": "project",
  "bundle": "drawio-diagrams",
  "status": "installed"
}
```

Status values:

| `status`      | Meaning                                                             |
| ------------- | ------------------------------------------------------------------- |
| `"installed"` | Configured successfully via CLI shellout or config-write.           |
| `"skipped"`   | CLI was not on PATH and no config-write target applied (warn-skip). |

`upsert_mcp` / `remove_mcps_by_name` mirror the plugin equivalents. The
lockfile `schema` field stays at **1** — the `mcps` array is additive and
backward-compatible; old upskill versions reading a lockfile with `mcps`
entries silently ignore the unknown array (serde `default`).

### Removal and reconciliation

- **`upskill remove-mcp <name>`**: a dedicated top-level subcommand (not
  `upskill remove`). It shells out to `claude mcp remove <name> --scope
  <scope>` (CLI-first), falling back to removing the entry from `.mcp.json`
  when the CLI is absent. Drops the `LockedMcp` entry from the lockfile.
- **`upskill doctor`**: checks each `claude`-client MCP entry in the
  lockfile via `claude mcp list` (substring match on the server name). A
  server present in the lockfile but absent from the client makes doctor
  report `NotRegistered` and exit non-clean (exit 1). Doctor also warns on
  unset `requires-env` variables. Reconciliation never auto-removes entries
  — consistent with the project's "never auto-delete" ethos.

### Implementation shape

- `src/model/bundle.rs` — `McpEntry { transport: McpTransport, requires_env }`;
  `McpTransport::Remote(McpRemote)` / `McpTransport::Local(McpLocal)`;
  `Bundle::validate_mcps`.
- `src/mcp.rs` — typed CLI-shellout functions (`install_claude_local`,
  `install_claude_remote`, `uninstall_claude`, `check_claude_installed`),
  reusing the spawn/CLI-not-found helpers from `src/plugin.rs`. Never
  writes to stdout/stderr.
- `src/ancillary.rs` — `write_claude_mcp_local` / `write_claude_mcp_remote`
  (config-write fallback into `.mcp.json`); `remove_claude_mcp` (removal
  fallback). Merge-preserving: existing servers in `mcpServers` are not
  clobbered.
- `src/pipeline/install.rs` — `install_mcps_from_bundles`: iterates
  resolved bundles, attempts CLI shellout, falls back to config-write on
  `CliNotFound`, returns structured `McpResult` per server.
- `src/lockfile.rs` — `LockedMcp`, `McpInstallStatus`, `upsert_mcp`,
  `remove_mcps_by_name`.
- `src/cli.rs` / `src/main.rs` — `Commands::RemoveMcp`; doctor
  reconciliation loop over `mcps` lockfile entries.

## Consequences

**Positive.**

- Native lifecycle (scope semantics, CLI-driven add/remove) is delegated to
  the `claude` CLI, not reinvented in upskill — consistent with ADR-0008's
  stance on plugins.
- Consistent with the "shell out to git instead of `git2`" pattern from
  [ADR-0001](./0001-multi-kind-compiler-architecture.md) §3.
- Bundle authors get a single SSOT entry that declares both transport
  descriptors and required env vars. Consumers get server configuration as
  part of `upskill add` rather than a separate manual step.
- Secrets stay with the user: upskill never reads, stores, or transmits
  actual token values.
- Config-write fallback ensures the server is configured even when the
  `claude` CLI is not on PATH (CI or first-time machines).

**Negative / limits.**

- v1 is Claude-only. Teams using opencode or VS Code in parallel do not get
  MCP configuration from the same bundle install — they must configure MCP
  manually until those clients are supported.
- Determinism loss: install outcome depends on whether `claude` is on PATH.
  Integration tests that exercise the CLI path must shim `claude` on PATH;
  tests that exercise config-write must clear PATH or point to an absent
  binary.
- The warn-skip policy can mask a real misconfiguration. Mitigated by
  `doctor`, which surfaces every `NotRegistered` server and every unset
  `requires-env` variable.
- Config-write fallback (`.mcp.json`) is a project-scope file; it does not
  cover the global scope as cleanly as the CLI. The global config location
  is client-version-specific and not yet handled by the fallback path.

## Alternatives considered

**(a) upskill runs a bundled `install.sh`.** Rejected: gives any bundle
author a shell on the consumer's machine (the npm `postinstall`
supply-chain hole). Breaks the safe-by-construction invariant. The v2
install agent achieves the same provisioning with the client's permission
system as the gate, not upskill's trust boundary.

**(b) upskill "triggers an AI prompt" itself.** Rejected: upskill has no
LLM runtime. Injecting fetched-bundle instructions into the consumer's
agent session is prompt injection with the same RCE risk wearing a
different hat.

**(c) upskill as secret manager — storing tokens in the lockfile or SSOT.**
Rejected: custody risk and redundant. Clients already resolve `${VAR}` and
run OAuth. upskill stays indirection-only; secrets remain with the user.

**(d) Patch client config files as the primary path.** Rejected: brittle
across client versions, depends on undocumented internal schema, and breaks
when the schema evolves. ADR-0008 rejected the same approach for plugins.
CLI-first; config-write is the fallback-only path.

**(e) New top-level `Mcp` kind for v1.** Rejected for the same reason
ADR-0008 rejected a `Plugin` kind: MCP servers compose with skills and
travel in bundles. A consumer bundle like `drawio-diagrams` wants to ship
both the skill content and its server in one unit. A standalone `Mcp` kind
would require two separate installs for a coherent setup. (v2 may revisit
kind-hood as part of the `requires` coupling model — see Deferred.)

**(f) Modeling bare-binary install declaratively** (per-platform install
command matrix in the descriptor). Rejected: too heterogeneous. Version
pins, build steps, and OS branches can't be captured in a static schema. The
v2 install agent reasons through this at runtime instead.

## Deferred (noted, not decided)

**v2: skill dependency edge.** Once
[ADR-0009](./0009-coupling-tiers-and-dependencies.md) lands, the
`requires` vocabulary will be extended with `mcps` so a skill can declare
`requires: { mcps: [drawio] }`. Resolving the dependency would re-use the
v1 config-write machinery. v2 also includes a generated skill-body preflight
(verify the MCP responds before the skill body runs) and an **install agent**
— an `agent` SSOT item that reasons through bare-binary provisioning at
runtime under the client's permission prompts. upskill stays
execution-free.

**Non-Claude clients.** opencode, VS Code (`code --add-mcp`), and GitHub
Copilot MCP configuration are deferred. When added, they will follow the
same CLI-first / config-write-fallback pattern used here for Claude.

## Migration

None — additive. `mcps:` is optional on Bundle. Existing bundles without
`mcps:` are unaffected. Old lockfiles without an `mcps` array load fine
(serde `#[serde(default)]`). Lockfile `schema` stays at `1`.

## References

- Structural precedent: [ADR-0008](./0008-plugin-install-shellout.md) —
  plugin install via client CLI shellout.
- Dependency coupling model (v2 prerequisite):
  [ADR-0009](./0009-coupling-tiers-and-dependencies.md).
- Design spec (background, rationale, alternatives):
  [`docs/superpowers/specs/2026-06-03-mcp-support-design.md`](../superpowers/specs/2026-06-03-mcp-support-design.md).
- Authoritative format spec: [`docs/format-spec.md`](../format-spec.md)
  §3.7 — `mcps:` sub-shape.
- `src/model/bundle.rs`, `src/mcp.rs`, `src/ancillary.rs`,
  `src/pipeline/install.rs`, `src/lockfile.rs` — as-built implementation.
