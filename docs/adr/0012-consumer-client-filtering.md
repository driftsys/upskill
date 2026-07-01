# Consumer-side client filtering — per-invocation flags and config

**Status**: Accepted (2026-07-01)

## Context

[ADR-0003](./0003-generation-pipeline.md) established the generation
contract: one SSOT item renders to per-client output for **every** client.
The only knob that narrows the target set is the **author-side** `audience:`
field in item frontmatter ([format-spec §5](../format-spec.md)). A
**consumer** has no say: every `upskill add` writes `.claude/**`,
`.github/**`, and `.agents/**` for every item, plus — since
[ADR-0010](./0010-mcp-config-write.md) — MCP config for all four MCP targets.

A consumer who only uses Claude Code still gets `.github/**` and `.agents/**`
trees they neither want nor track. The default (emit for all clients) is
correct and stays; what is missing is an opt-in **consumer-side**
restriction (issue #238).

### The two target spaces

[ADR-0010](./0010-mcp-config-write.md) deliberately split two enumerations
that this ADR must both honour:

- **`generate::Client`** (3): `claude`, `copilot`, `opencode` — the
  rules/skills/agents generation trees (`.claude/**`, `.github/**`,
  `.agents/**`).
- **`mcp::McpTarget`** (4): `claude`, `copilot`, **`vscode`**, `opencode` —
  MCP config-write targets. VS Code shares Copilot's `.github/**` generation
  output and forks **only** on MCP, so it is an MCP target but not a
  generation `Client`.

A consumer thinks in terms of _the client they use_, not these two internal
spaces. The selection surface must therefore be the **union** — the four
clients `claude`, `copilot`, `vscode`, `opencode` — and map each onto the
generation and MCP spaces internally.

## Decision

### Four boolean per-client flags on `add` and `update`

```text
upskill add <source> --claude --copilot --vscode --opencode
```

- No flag → **all clients** (default unchanged, regression-guarded).
- Any flag present → the selection is the **allowlist** of exactly the named
  clients.

There is no subtractive `--exclude-client` form: "Claude + Copilot only" is
`--claude --copilot`, and the boolean allowlist expresses every case a
consumer needs. `--exclude <NAME>` (item-name exclusion) is unrelated and
unchanged.

### Selection maps onto both target spaces

A selected client `c` contributes to:

| flag         | generation client | MCP target |
| ------------ | ----------------- | ---------- |
| `--claude`   | `claude`          | `claude`   |
| `--copilot`  | `copilot`         | `copilot`  |
| `--vscode`   | `copilot`         | `vscode`   |
| `--opencode` | `opencode`        | `opencode` |

`--vscode` maps to the **Copilot generation client** because VS Code reads
the shared `.github/**` rules/skills/agents tree; a VS-Code-only consumer
must still get those rules. It maps to the **VsCode MCP target** for its own
MCP config. Thus:

- **`--vscode` alone** writes `.github/**` (rules/skills/agents) **and** VS
  Code MCP config — a working VS Code setup.
- **`--copilot --vscode`** writes `.github/**` **once** (the generation
  clients dedupe to `{copilot}` via a set) and forks MCP into two writes:
  Copilot CLI MCP **and** VS Code MCP. This set-dedup on the generation side
  is an invariant, covered by a dedicated test.

### Effective generation set = author `audience:` ∩ consumer selection

The generation loop already filters each client by the item's `audience:`.
Consumer selection is a second, independent filter: an item is written for
generation client `g` iff `audience_targets(g)` **and**
`selection_targets(g)`. An item whose `audience:` excludes every selected
client is **warn-skipped**, not errored — consistent with the plugin/MCP
warn-skip ethos ([ADR-0008](./0008-plugin-install-shellout.md),
[ADR-0010](./0010-mcp-config-write.md)). The MCP fan-out iterates the
selected MCP targets instead of `McpTarget::ALL`.

### Persistent selection in config, with precedence

The existing user config ([config.rs](../../src/config.rs);
`~/.config/upskill/config.yaml` global, `.upskill/config.yaml` project) gains
a `clients:` list:

```yaml
clients: [claude, opencode]
```

Precedence, highest first:

1. **per-invocation flags** on `add` / `update`
2. **project** config `clients:`
3. **global** config `clients:`
4. **built-in default** — all clients

A per-invocation flag replaces the config selection for that run only;
`add --claude` never mutates config (no hidden persistence). Persisting a
selection means editing the config file. Config lives in `config.rs`, **not**
the lockfile — the lockfile is install _state_, not user _preference_.

### `doctor` respects what was actually written; `remove` already does

`doctor` reports "missing output" by checking each item's output path for
every generation client. With a selection, an unselected client's output was
never written, so an unfiltered `doctor` would report false drift. Fix: the
lockfile records the **effective generation clients** per item
(`LockedItem.clients`), and `doctor` iterates that recorded set. An empty
`clients` list means "all" — so pre-existing lockfiles (written before this
feature, for all clients) remain correct with no migration. This also fixes
the latent case of an `audience:`-restricted item showing false `doctor`
drift.

`remove` already deletes outputs with `fs::remove_file`, which is a no-op on
a never-written path — so removing an item installed under a narrow selection
needs no change.

## Alternatives considered

- **`--client <c>` / `--exclude-client <c>` value-parser flags** (the
  original issue sketch). Rejected in favour of boolean flags: `--claude`
  reads better, needs no value parser, and retroactively **validates** the
  `--claude --vscode` examples already shown in
  [ADR-0008](./0008-plugin-install-shellout.md) and
  [recipes.md](../recipes.md) — those become real instead of being deleted.
  The subtractive `--exclude-client` form is dropped as redundant.
- **`--vscode` as MCP-only** (no `.github/**`). Rejected: VS Code has no
  rules of its own; an MCP-only `--vscode` would leave a VS Code consumer
  without any rules/skills/agents.
- **Reading the config selection at `doctor` time** instead of recording it
  per item. Rejected: the selection can change between install and `doctor`
  (config edited, or a different scope), so `doctor` would validate against
  the wrong set. Recording what was actually written is robust.
- **Selection in the lockfile** rather than config. Rejected: the lockfile is
  per-install _state_; a persistent _preference_ belongs in config, which
  already has global+project layering.

## Consequences

- `add`/`update` gain four flags; `config.rs` gains a `clients:` key and a
  precedence resolver; `LockedItem` gains a `clients` field.
- A new selection type maps the four consumer clients onto the generation and
  MCP spaces; it is the single place the `--vscode → copilot generation`
  mapping lives.
- Docs that showed `--claude` / `--vscode` as aspirational
  ([ADR-0008](./0008-plugin-install-shellout.md),
  [recipes.md](../recipes.md)) are reconciled to describe the shipped flags.
