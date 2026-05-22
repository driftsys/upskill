# Plugin installation — extend Bundle, shell out to client CLIs

**Status**: Proposed (2026-05-22)

## Context

[ADR-0001](./0001-multi-kind-compiler-architecture.md) and
[ADR-0003](./0003-generation-pipeline.md) established the SSOT-to-files
contract: upskill reads SSOT items, generates per-client output files,
and never invokes the host client at runtime. That contract is the
right shape for rules, skills, and agents — they are content that
each client consumes by reading files from a known path.

Plugins do not fit that mold. Claude Code plugins (e.g.
`superpowers`), Copilot CLI plugins, VS Code extensions, and opencode
modules carry hooks, slash commands, marketplace registration, and
runtime state that the host client owns. Re-emitting that surface from
upskill SSOT duplicates the upstream plugin, can't capture hooks and
commands cleanly, and would have us shipping a second-class copy of
artifacts like `superpowers` that already exist as fully-formed plugins
maintained upstream.

Each of the four target clients exposes a CLI for plugin/extension
management:

- `claude plugin marketplace add <source>` +
  `claude plugin install <plugin>@<marketplace> --scope <s>`
- `copilot plugin marketplace add <source>` +
  `copilot plugin install <plugin>@<marketplace>`
- `code --install-extension <ext-id>`
- `opencode plugin <module>`

These CLIs already implement the lifecycle work (marketplace
registration, version pinning, scope semantics, idempotent updates)
that upskill would otherwise reinvent.

## Decision

### Bundle gains an optional `plugins:` map

[ADR-0007](./0007-bundle-yaml-format.md) defined the Bundle YAML
schema. This ADR extends it with one new top-level key, `plugins:`, a
map keyed by upskill-level plugin name:

```yaml
schema: 1
name: superpowers-baseline
description: Superpowers plugin and supporting rules
items:
  rules:
    - license-awareness
plugins:
  superpowers:
    claude:
      source: anthropics/claude-plugins
      plugin: superpowers
      install_url: https://github.com/obra/superpowers#install
    copilot:
      source: obra/superpowers-marketplace
      plugin: superpowers
      install_url: https://github.com/obra/superpowers#install
    vscode:
      extension: anthropic.superpowers
      install_url: https://marketplace.visualstudio.com/items?itemName=anthropic.superpowers
    opencode:
      module: superpowers-opencode
      install_url: https://opencode.ai/plugins/superpowers
```

- The map key (`superpowers`) is the upskill-level identity — used in
  the lockfile, CLI output, and `upskill remove plugin superpowers`.
- Per-client blocks (`claude`, `copilot`, `vscode`, `opencode`) are
  each optional and typed to that client's actual install primitives.
  A plugin that only exists for Claude Code carries only a `claude:`
  block.
- Each client block MAY carry `install_url:` — a URL surfaced in the
  warn-skip message when that client's CLI is not on PATH. The field
  is optional; if absent, the warning fires without the URL line.

### Install shells out to the native CLI

When `upskill add` resolves a bundle that declares plugins and the
user has the matching client targeted, upskill shells out:

| Client   | Commands                                                                                                                                                                  |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| claude   | `claude plugin marketplace add <source>` (idempotent — checked via `claude plugin marketplace list`), then `claude plugin install <plugin>@<marketplace> --scope <scope>` |
| copilot  | `copilot plugin marketplace add <source>` (idempotent), then `copilot plugin install <plugin>@<marketplace>`                                                              |
| vscode   | `code --install-extension <extension>` (naturally idempotent)                                                                                                             |
| opencode | `opencode plugin <module>`                                                                                                                                                |

Claude's `--scope` derives from upskill's existing project/global
flag:

| upskill flag       | lockfile location          | `claude --scope` |
| ------------------ | -------------------------- | ---------------- |
| `--global`         | `$HOME/.upskill-lock.json` | `user`           |
| (default, project) | `<cwd>/.upskill-lock.json` | `project`        |

Claude's `local` scope (machine-local, not committed) is not exposed
via upskill; users who want it can call `claude plugin install`
directly.

Copilot CLI does not support a `--scope` flag — plugins are installed
globally regardless of the upskill project/global context.

### CLI-missing policy: warn-skip

If a bundle declares a plugin for client X and X's CLI is not on
PATH, upskill prints a warning and continues with the rest of the
install:

```text
warn: claude CLI not found; skipped plugin 'superpowers' for claude.
      manual install: https://github.com/obra/superpowers#install
```

The rules/skills/agents portion of the bundle still installs.
Rationale: in mixed-client teams, the bundle's primary value
(SSOT-derived content) is independent of whether every plugin lands.
Failing the whole install for a missing tertiary tool is hostile.

### Lockfile records the resolved triple

Each plugin install records `(client, marketplace, plugin-name,
scope)` (or the client-equivalent identifier) in
`.upskill-lock.json` so `remove` / `update` / `doctor` can invoke the
inverse CLI command. The lockfile schema bump is documented in the
format spec.

### Implementation shape

New module `src/plugin.rs`, peer to `src/fetch.rs` and `src/auth.rs`.
Uses `std::process::Command` directly — no new dependencies, no
async, consistent with the project's "shell out to git" stance from
ADR-0001.

The module exposes typed functions per client operation
(`claude_marketplace_add`, `claude_plugin_install`,
`copilot_marketplace_add`, `copilot_plugin_install`,
`vscode_extension_install`, `opencode_plugin_install`, plus the
inverse uninstall calls). All return `anyhow::Result<T>` and never
write to stdout/stderr — presentation lives in `main.rs` per
existing project conventions.

### Platform scope

Per PR #135 the supported Windows path is `install.sh` under
WSL — `upskill` runs as a Linux ELF binary regardless of host
OS. Inside WSL all three target CLIs are present on PATH as
ordinary Linux executables (Claude Code, opencode, and VS Code
each ship Linux builds; VS Code's WSL-remote workflow
additionally injects a `code` shim that proxies to Windows-side
`code.exe` via interop). Shellout is indistinguishable from
native Linux — no `cfg!(windows)` branches, no `PATHEXT`
handling, no `.cmd` argument-escaping concerns.

Presence detection for the warn-skip policy uses
`Command::spawn()` and matches `ErrorKind::NotFound`, which is
portable across every platform Rust targets — no `which` crate
dependency, consistent with ADR-0001 §3.

Users who side-step the stance via `cargo install upskill` on
native Windows are unsupported. Native-Windows shellout (`.cmd`
shims, `PATHEXT`, Rust 1.77 batch-argument escaping, VS Code
Insiders discovery) would need its own ADR if that ever moves
in-scope.

## Consequences

**Positive.**

- Native lifecycle (marketplace updates, version pinning, scope
  semantics, idempotent upgrades) is delegated to each client, not
  reinvented in upskill.
- Consistent with the "shell out to git instead of `git2`" stance
  from [ADR-0001](./0001-multi-kind-compiler-architecture.md) §3 — a
  proven pattern in the codebase.
- Bundle authors get a single SSOT entry that fans out to whichever
  clients the consumer has installed.
- Warn-skip keeps mixed-client teams unblocked: a Copilot-only user
  installing a bundle that includes a `claude:` plugin block sees a
  warning and gets the rest of the bundle.

**Negative.**

- Determinism loss: install outcome depends on which CLIs are
  present, which marketplaces they have configured, and which
  upstream plugin versions resolve at install time. Integration tests
  must shim the client CLIs on PATH (the pattern already used by
  `fetch.rs::tests` for `git`).
- Bundle schema growth: `plugins:` is a non-trivial sub-shape with
  three client-specific variants. Future clients add more variants.
- Warn-skip can mask a real misconfiguration (user expected the
  plugin to install but the CLI was missing). Mitigated by `doctor`,
  which surfaces every skipped plugin during reconciliation.

## Alternatives considered

**(a) Patch `~/.claude/settings.json` directly.** Rejected:
depends on undocumented internal schema, breaks across Claude Code
versions, duplicates registration logic the CLI already implements,
and doesn't generalise to VS Code or opencode (each has its own
state file).

**(b) Re-emit plugin contents from SSOT.** Rejected: duplicates
upstream plugins like `superpowers`, can't represent hooks or slash
commands in the current item model, and forces upskill to track
upstream plugin updates instead of letting each client's CLI do it.

**(c) New top-level `Plugin` kind alongside rules/skills/agents.**
Rejected: plugins compose with rules/skills/agents in practice — a
bundle like `superpowers-baseline` wants to ship both the plugin and
its supporting rules in one unit. Keeping plugins on Bundle aligns
with how authors actually distribute them. A standalone Plugin kind
would force users to install two artifacts to get a coherent setup.

**(d) Hard-fail when the target client's CLI is missing.** Rejected:
too hostile to mixed-client teams. A user who runs `upskill add
some-bundle --claude --vscode` and is missing `code` would lose the
entire install. Warn-skip lets the parts that can install proceed,
and `doctor` reports the gap so it's not silently lost.

**(e) Per-plugin `install_url` (single URL, applies to all
clients).** Rejected: install instructions are usually
client-specific — "how to install superpowers for Claude Code"
differs from "how to install the VS Code extension." Per-client
`install_url:` is the natural home.

**(f) Make `install_url:` required.** Rejected: forces every bundle
author to track three URLs even when the warn-skip path is
unlikely. Optional, with a graceful degradation in the warning
message when absent.

## Migration

None — additive. `plugins:` is optional on Bundle. Existing bundles
without plugin entries are unaffected. v0.6 introduces the schema
extension; v0.5 bundles parse unchanged.

## References

- Predecessor: [ADR-0001](./0001-multi-kind-compiler-architecture.md)
  (SSOT-to-files contract, dependency philosophy).
- Predecessor: [ADR-0003](./0003-generation-pipeline.md)
  (generation pipeline this ADR explicitly _does not_ extend).
- Predecessor: [ADR-0007](./0007-bundle-yaml-format.md)
  (Bundle YAML schema this ADR extends with `plugins:`).
- Authoritative spec: [`docs/format-spec.md`](../format-spec.md) §3.7
  — to be updated with the `plugins:` sub-shape.
- Claude Code plugin CLI: `claude plugin --help`,
  `claude plugin marketplace --help`.
- GitHub Copilot CLI plugin: `copilot plugin --help`,
  `copilot plugin marketplace --help`.
