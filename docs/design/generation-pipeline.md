# Generation and installation

This record describes the existing module boundaries. It does not extend
implemented client support. See [ADR-0001](../adr/0001-multi-kind-compiler-architecture.md)
and [ADR-0003](../adr/0003-generation-pipeline.md) for the architecture decisions.

The Rust crate provides both a library and CLI. `src/main.rs` and `src/cli.rs`
provide the CLI surface and orchestration. Domain work belongs in library
modules; presentation stays at the CLI boundary.

| Responsibility                              | Modules                                                  |
| ------------------------------------------- | -------------------------------------------------------- |
| Content model and parsing                   | `src/model/`, `src/parse/`                               |
| Source classification and retrieval         | `src/source.rs`, `src/fetch.rs`                          |
| Bundle composition                          | `src/bundle.rs`                                          |
| Per-client rendering                        | `src/generate/`                                          |
| Installation and installed-state operations | `src/pipeline.rs`, `src/lockfile.rs`, `src/ancillary.rs` |
| Native integrations                         | `src/plugin.rs`, `src/mcp.rs`                            |
| Author commands                             | `src/lint.rs`, `src/fmt.rs`, `src/scaffold.rs`           |

The central flow is source content to parsed items to per-client output.
Bundles compose items and native integrations. Generated content is copied to
its target location; the lockfile records installed state. The
[portable format](../specification/format-spec.md) owns field and layout details;
this record does not duplicate those contracts.

Current content generators are Claude Code, Copilot and OpenCode. Generation
and MCP target sets differ, as recorded in
[ADR-0012](../adr/0012-consumer-client-filtering.md).
[ADR-0013](../adr/0013-agent-customization-targets.md) adds design commitments for
Codex, Cursor and Gemini CLI; it does not establish implemented adapters.
