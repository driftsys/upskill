# Upskill roadmap

Updated: **2026-11-11**. Status: **planned delivery scope**. This roadmap
describes upcoming work, not implemented support. Release numbers and dates will
be assigned separately.

## Direction

Upskill manages agent customization from one source, with a client adapter
choosing native conventions/configuration or active plugin behavior for each
feature. Decisions prioritize **reliability and performance**, then
**maintainability as clients evolve**.

Claude Code and Copilot CLI come first, then Codex, then OpenCode. Each client
can receive a thin upskill plugin integration backed by shared Rust CLI logic.
Client-specific packages can share one source repository.
**`driftsys/metapowers` is the marketplace repository.**

Plugin packaging and runtime loading are separate decisions: a plugin can
distribute or update content that the client loads through native conventions.
Hooks are used where they provide required behavior, rather than for every
customization.

The organizing baseline is
[separate generated client representations](https://github.com/driftsys/upskill/blob/main/docs/adr/0041-separate-client-representations-as-design-baseline.md),
with explicitly qualified sharing exceptions. Resources follow the selected
representation, ownership follows references, and removal never silently loads a
default. This does not select exact installed paths or package defaults. Each
client receives a support level instead of passing release gates.

## MVP — Next release

Tracking: [MVP epic #283](https://github.com/driftsys/upskill/issues/283).

The
[re-steer decision](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md)
of 2026-11-11 replaces the earlier gate sequence. The MVP makes install, update
and removal reliable for Claude Code and Copilot first. Codex and OpenCode come
next. The
[re-steer design](https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-11-11-customization-resteer-design.md)
holds the full requirements.

### Implementation dependencies

Each step is one pull request, is releasable, and starts with a failing CLI
acceptance test.

| Step | Content                                                                            |
| ---- | ---------------------------------------------------------------------------------- |
| 0    | Park unused code (section 7). Record this re-steer as one decision record.         |
| 1    | Fix [#465](https://github.com/driftsys/upskill/issues/465). Run trial T1.          |
| 2    | Ownership record, schema 2, adoption step; `remove` by recorded files (section 1). |
| 3    | `update` with modified and unowned file protection (section 1).                    |
| 4    | Repeatable operations with pending records and interruption tests (section 2).     |
| 5    | Scope reporting, both-scope `list` and `doctor`, global output paths (section 3).  |
| 6    | Recorded commits and `update --check` (section 4).                                 |
| 7    | Smoke test and support levels for Claude Code and Copilot CLI (section 6).         |
| 8    | Notice mode; Claude Code plugin; Copilot CLI plugin (section 4). Release.          |
| Next | Codex, then OpenCode, each with its own short design.                              |
| Next | Plugin and catalog generation from a bundle, with its own design.                  |

Section numbers refer to the re-steer design. Bundles and plugins need no
separate step. Bundle items are ordinary items and receive the ownership
behaviour. Plugin behaviour is unchanged.

## Post-MVP — Cleanup and refactoring

Tracking: [Job 1 epic #291](https://github.com/driftsys/upskill/issues/291).

Consolidate the implementation after the MVP release and before adding the extra
customization capabilities.

- Remove dead code and obsolete adapter paths proven unused by supported
  installations.
- Replace duplicated generation, configuration and lifecycle logic with shared
  code where the client contracts actually match.
- Reconcile legacy configuration handling and migration paths. Retain required
  compatibility; explicitly deprecate supported behavior before removing it.
- Simplify module boundaries using the working adapters as evidence, rather than
  designing abstractions ahead of their use.
- Update tests and documentation to describe the resulting architecture.

The exit gate is unchanged behavior for the supported MVP matrix, verified by
its regression and migration tests. Cleanup does not expand product scope.

## Extra capabilities — Following release

Tracking: [Job 1 epic #291](https://github.com/driftsys/upskill/issues/291).

Extend the client adapters with these managed capabilities:

| Capability     | Scope                                                                                     |
| -------------- | ----------------------------------------------------------------------------------------- |
| Tools          | Map tool identifiers and capabilities; expose unsupported mappings                        |
| Permissions    | Translate native approval and execution controls without silently broadening access       |
| Hooks          | Manage client events, payloads, results, trust and installation lifecycle                 |
| MCP management | Manage server configuration, transport/authentication references, discovery and lifecycle |

Record a support level for each capability on each client. Test combined
configurations as well as individual features. This stage adds first-class
management; it does not remove MCP functionality already shipped or postpone
respecting native permissions in the MVP.

## Later expansion

- **Cursor and Gemini CLI:** include their requirements in the model now;
  schedule adapter implementation after the Claude Code, Copilot, Codex and
  OpenCode adapters.
- **Subagents:** retain in the customization inventory; management expansion is
  not yet assigned to a release.
- **VS Code:** qualify profiles for the actual Copilot, Claude, Codex or Local
  runtime. Reuse client integrations where supported; add editor-specific
  integration only where needed. Selecting a model does not select its vendor's
  agent runtime.
- **Remote and cloud surfaces:** establish configuration discovery, executable
  deployment and trust separately before claiming support.

## References

- [Client support commitment](https://github.com/driftsys/upskill/blob/main/docs/adr/0013-agent-customization-targets.md)
- [Six-client capability evidence and plugin examples](https://github.com/driftsys/upskill/blob/main/docs/technotes/customization-adapter-evidence.md)
- [Original customization research](https://github.com/driftsys/upskill/blob/main/docs/technotes/agent-customization/README.md)
