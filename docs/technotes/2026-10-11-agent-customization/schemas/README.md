# Collected schema snapshots

These files are research inputs. They are not wired into upskill's parser or
generation pipeline, and no client was configured or run while collecting them.

[manifest.json](manifest.json) records each schema's source URL, retrieval time,
SHA-256 digest, size, publisher, and evidence classification. Git-hosted
artifacts also record the source commit and blob identity. Original bytes are
retained. The manifest is a provenance inventory, not an upskill schema.

## Inventory

| Group                  | Files                                     | Status                                                                                |
| ---------------------- | ----------------------------------------- | ------------------------------------------------------------------------------------- |
| Agent Plugins 1.0      | `agent-plugins-1.0.0-*.schema.json`       | Published standard: manifest and plugin MCP configuration                             |
| Agent Plugins 1.1      | `agent-plugins-1.1.0-draft-*.schema.json` | Working draft from pinned upstream source; not the published baseline                 |
| Codex configuration    | `codex-config.schema.json`                | OpenAI's rolling public configuration schema                                          |
| Codex hook events      | `codex-hooks/*.schema.json`               | All JSON schemas found in the vendor-generated event directory at the recorded commit |
| Claude settings        | `claude-code-settings.schema.json`        | SchemaStore schema referenced by official Claude documentation                        |
| Claude plugin manifest | `claude-code-plugin-manifest.schema.json` | SchemaStore community schema; vendor authorship not established                       |
| Gemini CLI settings    | `gemini-cli-settings.schema.json`         | Google source snapshot pinned to a commit                                             |
| OpenCode configuration | `opencode-config.schema.json`             | Rolling public schema; v1/v2 contract differences require separate verification       |

The Codex hook event directory contains command-handler input/output contracts.
It is not a complete schema for every hook handler or the standalone hooks
configuration file. Similarly, portable plugin MCP schemas do not validate every
client's native MCP configuration envelope.

## Reuse and attribution

- Agent Plugins schemas: Apache-2.0;
  [licensing notice](licenses/agent-plugins-LICENSE.txt) and
  [license text](licenses/agent-plugins-Apache-2.0.txt).
- Codex source schemas:
  [upstream Apache-2.0 license](licenses/codex-LICENSE.txt).
- SchemaStore schemas:
  [upstream Apache-2.0 license](licenses/schemastore-LICENSE.txt).
- Gemini CLI source schema: [upstream license](licenses/gemini-cli-LICENSE.txt).
- OpenCode source project:
  [upstream MIT license](licenses/opencode-LICENSE.txt). The rolling schema's
  canonical source URL is retained in the manifest.

These snapshots do not relicense upstream material. No third-party schema is
presented as authored or endorsed by upskill.

## Interpretation limits

JSON syntax, schema structure, and reference integrity can be checked locally.
That does not establish that a particular client release accepts every field,
that undocumented settings are supported, or that the runtime enforces a policy.
Dynamic documentation and source `main` may lead released clients.

Refresh snapshots deliberately and compare hashes and semantic differences. Do
not fetch a rolling schema at install time and silently change generated
behavior. Native validation, discovery, and runtime conformance checks belong in
the subsequent implementation design.

Raw schema and license snapshots are excluded from Prim formatting through
`.primignore` to preserve upstream bytes. `just docs-check` verifies the schema
inventory, JSON parsing, byte counts and SHA-256 hashes. The manifest and this
index remain subject to repository formatting checks.
