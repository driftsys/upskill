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
  `upskill add` warn at install time that an MCP server needs a variable that
  is not set in the current environment, without ever reading the value.

This is the same posture upskill already takes for git auth (tokens come
from env / `gh` / `glab`, never persisted).

### Per-target install contract — CLI-first, config-write fallback

> **Amended 2026-07-01 (issue #237):** the contract below now covers **all
> four MCP targets** — Claude Code, GitHub Copilot, VS Code, and opencode.
> The original v1 shipped Claude-only; the _Deferred → Non-Claude clients_
> note is now delivered. VS Code is an MCP **target** but not a generation
> `Client`: it shares `.github/**` output with Copilot and forks only on MCP,
> so the MCP target set (`claude`, `copilot`, `vscode`, `opencode`) is modelled
> separately from `Client` (the `McpTarget` enum in `src/mcp.rs`).

Each bundle `mcps:` entry is configured for every target, CLI-first with a
config-write fallback per target. A target whose CLI is absent **and** has no
applicable config-write target produces a warn-skip — not an error.

| Target         | CLI-first                                                 | Config-write fallback file                | Root key     |
| -------------- | --------------------------------------------------------- | ----------------------------------------- | ------------ |
| Claude Code    | `claude mcp add …` (`--scope`, `-e`, `--transport`, `-H`) | `.mcp.json`                               | `mcpServers` |
| GitHub Copilot | `copilot mcp add …` (no `--scope`)                        | `~/.copilot/mcp-config.json` (user scope) | `mcpServers` |
| VS Code        | `code --add-mcp '<json>'`                                 | `.vscode/mcp.json`                        | `servers`    |
| opencode       | — (no `mcp add` verb)                                     | `opencode.json`                           | `mcp`        |

The Claude forms are unchanged:

- Local (stdio):
  `claude mcp add <name> --scope <scope> [-e KEY=${VAR} …] -- <command> [args…]`
- Remote (http/sse):
  `claude mcp add --transport <http|sse> <name> <url> --scope <scope> [-H "Header: ${VAR}" …]`

Per-target config shapes diverge:

- **Claude / Copilot** share `{ command, args?, env? }` (local) /
  `{ type, url, headers? }` (remote) under `mcpServers`.
- **VS Code** uses root key `servers`; local servers are typed `"stdio"`.
- **opencode** uses `type: "local"|"remote"`; local `command` is a single array
  `[command, args…]` and the env map is named `environment`.
- **Copilot** has no documented project-scope config file (`.github/mcp.json`
  is not a Copilot CLI source), so the fallback is always the user-scope
  `~/.copilot/mcp-config.json`; project scope is configured via the CLI or
  warn-skipped.

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
  `upskill remove`). For targets with a native remove verb (Claude
  `claude mcp remove <name> --scope <scope>`, Copilot `copilot mcp remove
  <name>`) it is CLI-first, falling back to deleting the entry from the
  target's config file when the CLI is absent; VS Code and opencode are
  config-file only. Drops every matching `LockedMcp` entry from the lockfile.
- **`upskill doctor`**: reconciles each lockfile MCP entry against its
  target. Claude is queried via `claude mcp list` (substring match); the
  config-write targets (Copilot, VS Code, opencode) are checked against their
  config file. A server present in the lockfile but absent from a present
  config file (or from the Claude client) makes doctor report `NotRegistered`
  and exit non-clean (exit 1). Claude also reports `CliNotFound` and
  `QueryFailed`; for a config-write target whose config file is **absent** the
  state is undetermined and the entry is skipped — no false positives. Doctor
  does **not** check `requires-env` variables and never auto-removes entries.
- **`upskill add` (install time)**: warns for each variable listed in
  `requires-env` that is not set in the current environment, immediately
  after configuring the server.

### Implementation shape

- `src/model/bundle.rs` — `McpEntry { transport: McpTransport, requires_env }`;
  `McpTransport::Remote(McpRemote)` / `McpTransport::Local(McpLocal)`;
  `Bundle::validate_mcps`.
- `src/mcp.rs` — the `McpTarget` enum (`Claude`, `Copilot`, `VsCode`,
  `OpenCode`) plus typed CLI-shellout functions per target with a CLI verb
  (`install_claude_*`, `install_copilot_*`, `install_vscode_*`,
  `uninstall_claude`, `uninstall_copilot`, `check_claude_installed`), reusing
  the spawn/CLI-not-found helpers from `src/plugin.rs`. Never writes to
  stdout/stderr.
- `src/ancillary.rs` — config-write fallbacks per target
  (`write_claude_mcp_*` → `.mcp.json`, `write_copilot_mcp_*` →
  `~/.copilot/mcp-config.json`, `write_vscode_mcp_*` → `.vscode/mcp.json`,
  `write_opencode_mcp_*` → `opencode.json`), the shared merge `upsert_mcp_server`
  (path + root key), removers (`remove_*_mcp`), and doctor state queries
  (`vscode_mcp_state` / `opencode_mcp_state` / `copilot_mcp_state`). All
  merge-preserving: sibling servers and other top-level keys are never clobbered.
- `src/pipeline/install.rs` — `install_mcps_from_bundles`: for each bundle
  `mcps:` entry, fans out over `McpTarget::ALL`, attempts CLI shellout, falls
  back to the target's config-write on `CliNotFound`, returns one structured
  `McpResult` per `(name, target)`.
- `src/lockfile.rs` — `LockedMcp`, `McpInstallStatus`, `upsert_mcp`
  (keyed by `(name, client)`), `remove_mcps_by_name`.
- `src/cli.rs` / `src/main.rs` — `Commands::RemoveMcp`; the `unconfigure_mcp`
  dispatch over targets; doctor reconciliation loop over `mcps` lockfile entries.

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

- A single bundle `mcps:` entry now fans out to four targets, so one server
  yields four `LockedMcp` rows and may write up to four config files. There is
  not yet a consumer-side switch to narrow the target set (tracked by #238); a
  per-target consumer selection should honour the same filtering when it lands.
- Determinism loss: install outcome depends on whether each target CLI is on
  PATH. Integration tests that exercise a CLI path must shim it on PATH; tests
  that exercise config-write must clear PATH or point to an absent binary.
- The warn-skip policy can mask a real misconfiguration. Mitigated by
  `doctor`, which surfaces every `NotRegistered` server, and by `upskill add`
  which warns at install time for every unset `requires-env` variable.
- Config-write fallbacks are project-scope files except Copilot, whose only
  documented config file is the user-scope `~/.copilot/mcp-config.json`. The
  Claude global-scope config location is client-version-specific and not yet
  handled by the fallback path. VS Code's `code --add-mcp` writes the user
  profile MCP store, which differs from the `.vscode/mcp.json` workspace file
  the fallback writes — so a CLI-installed VS Code server is not removed by the
  config-file remover.

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

**Non-Claude clients.** ~~Deferred.~~ **Delivered (2026-07-01, issue #237):**
opencode, VS Code (`code --add-mcp`), and GitHub Copilot (`copilot mcp add`)
now follow the same CLI-first / config-write-fallback pattern — see the
amended _Per-target install contract_ above.

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
