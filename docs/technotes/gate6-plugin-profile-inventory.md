# Gate 6 pinned profiles and reader inventory

Task [#336](https://github.com/driftsys/upskill/issues/336) executes Task 1 of
the
[corrected plan](../wip/plans/2026-10-19-customization-delivery-reconciliation.md)
after architecture PR #334 merged as `44a5e2f`. This is qualification input, not
an implementation or native support claim. Production installed state remains
schema 1.

## Slice 1 campaign revision: 2026-10-24

Task [#382](https://github.com/driftsys/upskill/issues/382) opens a new campaign
revision for the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md).
Installed CLI version checks on macOS arm64 pin the following releases before
any new native control. Every new control row must name its pin. Historical
2026-10-19 evidence keeps its original pins and is not implicitly rerun.

| Client          | Installed release | Surface         |
| --------------- | ----------------- | --------------- |
| Claude Code     | 2.1.308           | macOS arm64 CLI |
| Codex CLI       | 0.164.0           | macOS arm64 CLI |
| Copilot CLI     | 1.0.103           | macOS arm64 CLI |
| OpenCode v2 CLI | 2.1.4             | macOS arm64 CLI |

These are installed release observations, not native qualification. Linux, IDE
and hosted surfaces remain unqualified. Production routing remains schema 1.

## Frozen campaign

[Campaign metadata](agent-customization/gate6-campaign-2026-10-19.json) pins:

| Client          | Version | macOS arm64 and Linux x86_64 availability | Execution in this task |
| --------------- | ------- | ----------------------------------------- | ---------------------- |
| Codex CLI       | 0.163.0 | Official non-prerelease GitHub artifacts  | Not run                |
| Claude Code     | 2.1.303 | Official non-prerelease GitHub artifacts  | Not run                |
| Copilot CLI     | 1.0.98  | Official non-prerelease GitHub artifacts  | Not run                |
| OpenCode v2 CLI | 2.0.32  | Exact npm CLI/platform packages           | Not run                |

The metadata retains URLs, reported digests/integrity and asset identities.
OpenCode v2 is distributed as `@opencode/cli`, not the default GitHub v1 release
line or the legacy `@opencode-ai/cli` beta channel. The v2 package version and
both platform packages were resolved directly. Artifact availability is not
proof of installation, successful execution, authenticity beyond the stated
metadata checks or native conformance. This task installs nothing globally and
changes no native configuration, authentication or team instructions.

These versions are frozen for this campaign; changing one requires a new
campaign evidence revision and rerunning affected cells. The installed older
Claude/OpenCode versions recorded by #334 do not qualify these pins.

## Matrix and interpretation

The
[reader/profile inventory](agent-customization/customization-delivery-matrix.json)
is informative documentation, not a new product manifest or runtime input. It
contains 16 profiles (four clients, two operating systems, two scopes), 40
client/customization routes and 160 explicit profile/customization cells. There
are seven current categories (rules, skills, resources, agents, bundles, plugins
and MCP) and three visible deferred categories (hooks, tools, permissions). No
cell is qualified. Deferred cells are not scheduled, not silently supported. A
bundle has no independent native activation; its children retain all controls.

Each route records delivery, discovery, activation, project/user location
candidates, potential other readers and conditions, explicit consumption intent,
preservation contracts, implementation references, status and unresolved facts.
Resources include skill files and resources of rules/agents/plugins where those
routes actually support them; installation never authorizes arbitrary resource
execution. Current historical paths remain listed separately from intended
native paths. Compatibility readers retain their scope conditions; a project
source is not automatically a personal source. Cursor/Gemini remain possible
reader/design obligations without new implementation commitments.

Root variables denote declared qualified roots, not instructions to write into
personal homes. Native cache and registration roots remain unresolved until
observed. External-home authority cannot be inferred from an accessible path.
Every unresolved reader/root remains a blocker for the affected isolation or
write-authority claim. Unknowns are explicit, not empty reader sets. Each route
separately records ancestor roots, nested roots and configured-source
boundaries. OpenCode skills include documented ancestor traversal, nested groups
and additional local/HTTP sources; exact pinned-runtime behavior remains
unqualified. Other unresolved traversal boundaries require their own controls
before isolation can pass.

The matrix shares the F/R/N/B preservation contracts from the canonical
[delivery contract](../topics/0009-customization-delivery.md): exact owned
files/resources, unchanged team guidance, separately observed native effects,
and surviving direct/bundle references. Existing schema-1 renderers and
shellouts are implementation facts only; they do not establish compatible
schema-2 state handling or native-effect recovery.

## Pinned independent rule candidates

### Codex 0.163.0

Use an explicitly installed local marketplace plugin with the supported Codex
compatibility manifest, a declared hook source and separately owned rule
payload. This candidate does not use managed team `AGENTS.md` contributions.
Normal hook trust must be reviewed and bound to the current definition; changed
hooks need renewed trust. The dangerous one-off trust-bypass flag cannot qualify
ordinary install/update trust behavior.

The candidate event is `SessionStart`, returning hook-specific
`additionalContext`. Startup, resume, clear, compact and fork are distinct
sources in the released source. Package enablement, hook trust, hook execution,
context arrival and rule behavior require separate native evidence. Do not infer
all of them from marketplace listing or a zero exit code.

Project and user marketplace/config roots are distinct from native-managed
installation caches. Scope, native registration ownership and safe cleanup must
be observed before model probes; no user-root fixture has been provisioned by
this task. Preserve normal native authentication without copying secrets.

The released parser accepts compatibility marketplace layouts including
`.claude-plugin` and `.cursor-plugin`. Consequently, a vendor directory name
alone cannot prove package isolation. Other clients' registrations and exported
skill catalogs remain potential readers and need negative/duplicate controls.

Version-specific source locations and hashes are in the matrix. Relevant
[manifest parser](https://github.com/openai/codex/blob/rust-v0.163.0/codex-rs/core-plugins/src/manifest.rs),
[marketplace parser](https://github.com/openai/codex/blob/rust-v0.163.0/codex-rs/core-plugins/src/marketplace.rs),
[hook discovery](https://github.com/openai/codex/blob/rust-v0.163.0/codex-rs/hooks/src/engine/discovery.rs)
and
[session-start event](https://github.com/openai/codex/blob/rust-v0.163.0/codex-rs/hooks/src/events/session_start.rs)
were read without executing vendor source. Rolling
[packaging](https://developers.openai.com/plugins/build/plugins) and
[hook documentation](https://learn.chatgpt.com/docs/hooks) remain secondary
candidate guidance, not a replacement for the pinned runtime controls.

### OpenCode 2.0.32

Use the v2 default export from `Plugin.define` in `@opencode/plugin`, whose
setup registers `ctx.session.hook("context", handler)` and supplies independent
rule text through `event.system`. Preserve team `AGENTS.md`. Do not port the v1
`experimental.chat.system.transform` name into the v2 adapter.

The exact `@opencode/plugin@2.0.32` archive was fetched, its SHA-512 integrity
verified against the package metadata, and its Promise plugin/session type
declarations inspected. The SDK declares plugin `id`/`setup`, context
`location`, context hooks and mutable system parts. This proves declaration
availability, not that an installed CLI loads the plugin or a provider consumes
the text. The archive was not installed or executed. Its metadata/hash and local
receipt are retained in the campaign/matrix.

The hook must filter the actual session directory and relevant agent/context.
`ctx.location` is the plugin instance's location, not universal authority over
all sessions the API can access. Compaction, resume, disposal/reload and
exactly-once registration need separate controls. Project config/plugin roots
and user XDG/config overrides must be bound independently. Existing v1-style
plugin verbs and `mcp.<name>` config are not evidence for v2's explicit add verb
and `mcp.servers` shape.

References: [v2 plugin API](https://dev.opencode.ai/v2/docs/build/plugins/),
[v1 migration](https://dev.opencode.ai/v2/docs/build/plugins/migrate-v1),
[plugin discovery](https://dev.opencode.ai/v2/docs/plugins/) and
[configuration roots](https://dev.opencode.ai/v2/docs/config/). These describe
mechanisms; all native cells remain unqualified.

## Prerequisites for Task 2

Proceed with offline harness examples and disposable installed-plugin probes
against these pins. Before each applying/native probe, establish its exact
registration/cache/root authority, ordinary hook trust, authentication isolation
and provider availability. Missing facts stop that probe, not the four-client
release gate. Linux and authenticated isolated user-root execution remain
unprovisioned here. Generated files, SDK declarations and this inventory cannot
satisfy CD-02 through CD-09 or authorize production schema-2 activation.

Validation is JSON parsing, matrix identity/reference/coverage checks,
formatting, documentation/book checks and independent review. The plan
explicitly requires schema/fixture unit tests only if the matrix becomes
executable input; no such consumer or runtime test is introduced here.
