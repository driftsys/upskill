# Design: MCP server support — config-write install + lazy provisioning

**Date**: 2026-06-03
**Status**: Approved (brainstorming)
**Relates to**: [ADR-0008](../../adr/0008-plugin-install-shellout.md) (plugin
shellout — the structural precedent), [ADR-0009](../../adr/0009-coupling-tiers-and-dependencies.md)
(coupling tiers / `requires` — the v2 dependency model, in flight)

## Problem

upskill distributes rules, skills, and agents from SSOT to per-client output,
and (since [ADR-0008](../../adr/0008-plugin-install-shellout.md)) installs
client-native **plugins** by shelling out to client CLIs. It has **no support
for MCP servers** — the Model Context Protocol servers that give an AI client
new tools (e.g. the draw.io MCP for diagram generation).

A skill is often only useful when its MCP server is present: a "draw.io
diagrams" skill that references `DrawIO:create_diagram` tools is dead weight if
the draw.io MCP was never configured in the client. Today a consumer must
install the skill via upskill and then _separately, manually_ wire up the MCP
server in each client's config. The two halves of a coherent capability are
delivered apart.

This design adds MCP support so a bundle (v1) or a skill dependency (v2) can
carry its MCP server alongside its content, configured into each targeted
client through that client's native mechanism.

### What an MCP server actually needs

MCP servers reach a client in one of two transport shapes:

- **Remote** — a hosted server the client connects to over `http` or `sse` at a
  `url`. Already running; auth is OAuth (client-driven, interactive) or a static
  token in a header.
- **Local (stdio)** — the client spawns a local `command` (with `args`/`env`)
  and speaks MCP over stdio. When the command is a self-fetching launcher
  (`npx -y <pkg>`, `uvx <pkg>`, `docker run …`) the launcher downloads and
  caches the server on first run — upskill installs **nothing**.

The only case needing a real install step is a **bare binary with no launcher**
(a compiled tool, a Homebrew formula) that must be detected → version-checked →
installed → set up. That heterogeneous case is the motivation for the v2 lazy
install agent (§ v2), and is explicitly _out of scope for v1_.

## Trust posture (non-negotiable invariant)

Today `upskill add` is **safe-by-construction**: it only (a) writes files into
known paths and (b) shells out to known client CLIs with declared args. Adding
an arbitrary fetched bundle cannot execute arbitrary code.

This design **preserves that invariant**. upskill itself never downloads,
builds, or runs server code. Specifically:

- **v1** writes config (via the client's MCP CLI verb, or a config file) — the
  same class of action as the existing plugin shellout.
- **v2's** "install agent" is **generated `agent` SSOT content**, executed by
  the _client_ (e.g. Claude Code) under the client's own per-command permission
  prompts — never by the upskill binary. The trust gate is the client's
  existing "allow this command?" prompt, not a new RCE surface in `upskill`.

Rejected for this reason: upskill running a bundled `install.sh`, or upskill
"triggering an AI prompt" itself. Both would hand a third-party bundle author a
shell on the consumer's machine (the npm `postinstall` supply-chain hole). See
_Alternatives considered_.

## Phasing

| Phase                | Scope                                                                                                                                                                            | Depends on                                                                                       |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| **v1**               | MCP as a **bundle-level descriptor** (`mcps:`, sibling of `plugins:`). Eager config-write at `upskill add`, CLI-first. Lockfile + `doctor` lifecycle. Declare-and-check secrets. | Nothing unfinished. Parallels ADR-0008.                                                          |
| **v2** (fast-follow) | **Skill-coupled lazy provisioning**: `requires.mcps` dependency edge + generated skill-body preflight + the **install agent** for bare-binary provisioning.                      | [ADR-0009](../../adr/0009-coupling-tiers-and-dependencies.md) coupling/`requires` model landing. |

v1 ships on its own. The genuinely novel part (lazy, agent-driven provisioning)
is parked behind the coupling work it actually needs.

---

## v1 design — bundle-level MCP descriptor, eager config-write

A near-exact parallel of the existing `plugins:` mechanism
([ADR-0008](../../adr/0008-plugin-install-shellout.md),
[`src/model/bundle.rs`](../../../src/model/bundle.rs)).

### Model

New `mcps:` map on `Bundle`, keyed by upskill-level MCP name (used in the
lockfile, CLI output, and `upskill remove mcp <name>`):

```yaml
schema: 1
name: drawio-diagrams
description: Draw.io diagram skill and its MCP server
items:
  skills:
    - drawio-diagrams
mcps:
  drawio:
    # exactly one of `remote` / `local`
    remote:
      type: http # http | sse
      url: https://mcp.draw.io/mcp
      headers: # optional; values are ${VAR} references only
        Authorization: "Bearer ${DRAWIO_TOKEN}"
    # — or —
    local:
      command: npx
      args: ["-y", "drawio-mcp-server"]
      env: # values are ${VAR} references only
        DRAWIO_TOKEN: "${DRAWIO_TOKEN}"
    requires-env: [DRAWIO_TOKEN] # declared so `doctor` can warn on unset vars
```

- A descriptor carries **exactly one** of `remote` or `local` (validated; both
  or neither is an error).
- The map key (`drawio`) is the upskill-level identity.
- Per-client targeting is implicit in v1: the descriptor is client-agnostic and
  upskill writes it into each _targeted_ client (the `--claude` / `--copilot` /
  etc. flags already on `add`). A client that does not support MCP → **warn-skip**
  (same policy as plugins).

### Secrets — indirection + declare-and-check, never custody

upskill never owns a secret value. The rule:

- Config values that reference secrets use **`${VAR}` indirection only**. upskill
  writes the reference; the **client** expands it at connect/launch time; the
  value lives in the user's environment / OS keychain / the client's OAuth store.
- upskill **never persists a literal secret** — not into a client config file,
  not into the lockfile, not into SSOT.
- `requires-env: [VAR, …]` is a **declaration** (not a value). It lets
  `upskill doctor` warn _"MCP `drawio` needs `DRAWIO_TOKEN`; it is not set in your
  environment"_ without ever reading the value.

This is the same posture upskill already takes for git auth (tokens come from
env / `gh` / `glab`, never persisted — see [AGENTS.md](../../../AGENTS.md)
"Authentication").

### Per-client install contract — CLI-first, config-write fallback

For each targeted client, prefer the client's native MCP CLI verb; fall back to
writing the client's config file **only** when the CLI is missing (the
warn-skip path) or cannot express the descriptor. Direct config patching as the
_primary_ path is rejected — same stance ADR-0008 took against patching
`settings.json`.

| Client      | Primary (CLI)                                                                                                                                  | Fallback (config-write)                                         |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Claude Code | `claude mcp add <name> --scope <s> [-e KEY=${VAR}] -- <command> [args…]` (stdio); `claude mcp add --transport http\|sse <name> <url>` (remote) | `.mcp.json` (project) / user config                             |
| opencode    | _(config-only)_                                                                                                                                | `mcp` key in `opencode.json` (ancillary already owns this file) |
| VS Code     | `code --add-mcp '<json>'`                                                                                                                      | `.vscode/mcp.json`                                              |
| Copilot CLI | per its MCP verb                                                                                                                               | per its config                                                  |

- Claude `--scope` derives from upskill's project/global flag exactly as plugins
  do (project→`project`, global→`user`; ADR-0008).
- Idempotent: re-adding an existing server is a no-op / overwrite, mirroring the
  `marketplace add` idempotency in [`src/plugin.rs`](../../../src/plugin.rs).
- CLI-missing → warn-skip with an optional `install_url`-style hint; the rest of
  the bundle still installs.

### Lifecycle — lockfile + doctor

Mirror `LockedPlugin` ([`src/lockfile.rs`](../../../src/lockfile.rs)):

- `LockedMcp { name, client, identifier, bundle, scope, status }` recorded per
  `(name, client)` on successful install (or warn-skip / instructions status).
  Status enum parallels `PluginInstallStatus`.
- `upsert_mcp` / `remove_mcps_by_name`, sorted, like the plugin equivalents.
- `upskill remove mcp <name>` runs the inverse CLI (`claude mcp remove <name>
  --scope <s>`, etc.).
- `doctor` reconciles: queries the client (`claude mcp list`, etc.) to detect a
  server present in the lockfile but absent from the client (parallel to
  `check_*_plugin_installed`), and warns on unset `requires-env` vars.
- Removal never cascades (the #196 "never auto-delete" ethos).

### Implementation shape

- **model**: `mcps: BTreeMap<String, McpEntry>` on `Bundle`; `McpEntry` with
  `remote` / `local` variants, `requires_env`. In
  [`src/model/bundle.rs`](../../../src/model/bundle.rs), beside `PluginEntry`.
- **parse**: accept + validate the shape in
  [`src/parse/bundle.rs`](../../../src/parse/bundle.rs) (exactly-one-of
  remote/local; `${VAR}`-only secret values).
- **install**: new `src/mcp.rs`, peer to [`src/plugin.rs`](../../../src/plugin.rs)
  — typed CLI-shellout + config-write functions per client, returning structured
  outcomes, never writing to stdout/stderr. Reuses the spawn / CLI-not-found
  helpers' pattern.
- **ancillary**: extend [`src/ancillary.rs`](../../../src/ancillary.rs) to merge
  MCP entries into `opencode.json` / `.vscode/mcp.json` / `.mcp.json` without
  clobbering user-authored entries (richer than the current single-URI append in
  `write_opencode_plugin_uri`).
- **pipeline**: `McpResult` + `install_mcps_from_bundles`, mirroring
  `install_plugins_from_bundles` in [`src/pipeline.rs`](../../../src/pipeline.rs).
- **lockfile**: `LockedMcp` + upsert/remove + schema note.
- **main**: reporting, `upskill remove mcp <name>`, `doctor` lines. (Presentation
  stays in `main.rs` per project convention.)
- **lint/fmt**: validate/canonicalise the `mcps:` shape.

### Testing

- **Unit**: descriptor parse (exactly-one-of, `${VAR}` enforcement),
  `requires-env` presence-check, lockfile read/write, scope mapping.
- **Integration** (`tests/`): shim `claude` / `code` on PATH (the pattern
  `fetch.rs` tests use for `git`) to assert the right CLI args; assert config-write
  fallback shape when the shim is absent; assert warn-skip when no CLI; assert
  `doctor` reconciliation. New `cli_mcp.rs` / `pipeline_mcp.rs`, golden config
  fixtures in `tests/fixtures/`.

### Docs

- New **ADR-0010** (MCP config-write + secrets posture).
- `docs/format-spec.md` — `mcps:` sub-shape + lockfile schema bump.
- Book: commands / recipes / writing-skill-bundles update.

---

## v2 design (fast-follow, sketch — finalized in its own brainstorm)

Once [ADR-0009](../../adr/0009-coupling-tiers-and-dependencies.md) lands:

- **Dependency edge**: add `mcps` to ADR-0009's `requires:` vocabulary
  (`requires: { rules, skills, agents, mcps }`), resolved by `(kind, name)`, hard
  - acyclic, with `required_by` provenance. A skill declares
    `requires: { mcps: [drawio] }`; installing the skill resolves and config-writes
    its MCP (the v1 machinery). **Open question for the v2 ADR**: whether MCP
    becomes a first-class _kind_ (resolved by `(kind, name)` like rules/skills) or
    stays a bundle descriptor referenced by name.
- **Skill-body preflight** (the runtime trigger): the dependency _generates_ a
  leading step into the rendered skill — _"Before using draw.io, verify the
  `drawio` MCP responds; if a local binary is required and missing, dispatch the
  `mcp-installer` agent."_ Pure, portable content.
- **Install agent**: an `agent` kind shipped as SSOT content whose job is detect →
  version-check → install → set up a bare-binary server, reasoning through the
  heterogeneity a schema cannot. Runs in the client under the client's permission
  prompts. upskill stays execution-free.

## Alternatives considered (rejected)

- **upskill runs a bundled `install.sh`** — turns `upskill add` into arbitrary
  remote code execution (npm `postinstall` hole); breaks the safe-by-construction
  invariant. The install agent (v2) achieves the same provisioning with the
  client's permission system as the gate.
- **upskill "triggers an AI prompt" itself** — upskill has no LLM; this means
  injecting fetched-bundle instructions into the consumer's agent (prompt
  injection). Same RCE risk wearing a different hat.
- **upskill as secret manager / storing tokens** — custody risk and redundant;
  clients already resolve `${VAR}` / run OAuth. upskill stays indirection-only.
- **Patch client config files as the primary path** — brittle across client
  versions; ADR-0008 already rejected this for plugins. CLI-first, config-write
  fallback only.
- **New top-level `Mcp` kind for v1** — ADR-0008 rejected a parallel `Plugin`
  kind for the same reason: MCP servers compose with skills and travel in
  bundles. (v2 may revisit kind-hood as part of the `requires` model.)
- **Modeling bare-binary install declaratively** (per-platform install command
  matrix in the descriptor) — too heterogeneous (version pins, build steps, OS
  branches); the v2 install agent reasons through it instead.

## Open questions

- **v2 only**: MCP as a first-class kind vs. bundle descriptor referenced by
  `requires.mcps` (defer to the v2 ADR).
- Exact Copilot CLI MCP verb and VS Code `--add-mcp` payload shape — confirm
  against current client CLIs during implementation.
