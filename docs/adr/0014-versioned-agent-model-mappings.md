# Versioned model mappings for custom agents

**Status**: Accepted (2026-10-11)

## Context

An agent author needs a stable way to request a capability tier without binding
the source definition to one vendor's changing model catalog. Claude Code,
Codex, and Copilot expose different model and reasoning controls. Copilot CLI
supports an ordered `models` list, `modelPolicy`, and one `reasoningEffort` for
an agent. A model list can fall back to the session model when none is
accessible; `auto` does not guarantee an authored tier.

Model availability and native identifiers can change independently of an
upskill release. Fetching live documentation during installation would make
the same source resolve differently over time and could change an existing
installation without a clear user action.

## Decision

upskill ships **offline, versioned model mappings** for the S, A, B, and C
tiers for Claude Code, Codex, and Copilot. Installation and explicit
regeneration resolve the selected mapping version at that time and record the
resolved native values. A mapping update alone does not regenerate installed
agents.

An explicit mapping-update command may fetch a maintained mapping from the
upskill GitHub repository. It validates the mapping version and format before
installing it locally. The command does not scrape vendor documentation or
rewrite installed agents. Global overrides in `~/.upskill/upskill.toml` take
precedence over shipped or fetched mappings. Mapping updates preserve that
file and its overrides.

Agent source may use `model:effort` as upskill shorthand. The resolver keeps
the tier and effort separate until target generation. Initial tier guidance is
S `high`, A `high`, B native/default effort, and C `low`; an explicit author
effort or global override takes precedence. Unsupported model/effort
combinations fail validation rather than silently changing the requested tier.

For Copilot CLI, generate an ordered `models` preference list in this order:
latest Claude, latest GPT, previous Claude, previous GPT. Use
`modelPolicy: preferred`. If no listed model is accessible, Copilot's session
model is the fallback; upskill reports that the authored tier was not
enforced. `auto` may be a session fallback but is not a tier mapping.
Copilot's native `reasoningEffort` applies to the entire agent, so every model
in one generated fallback list must accept one effective effort. If the list
cannot satisfy that contract, generation reports a capability conflict rather
than giving different implied efforts to individual entries.

The initial Copilot candidates are **draft labels, not native CLI IDs**:

| Tier | Ordered candidates                                | Initial effort |
| ---- | ------------------------------------------------- | -------------- |
| S    | Fable 5.1 → GPT-6 Astra → Fable 5                 | high           |
| A    | Opus 5.5 → GPT-6.1 Sol → Opus 5 → GPT-6 Sol       | high           |
| B    | Sonnet 5.5 → GPT-6.1 Sol → Sonnet 5 → GPT-6 Sol   | default        |
| C    | Haiku 5.5 → GPT-6 Luna → Haiku 4.5 → GPT-5.6 Luna | low            |

The S draft has no previous-GPT candidate; the resolver must not invent one.
Before implementation, verify every candidate's exact Copilot CLI model ID,
availability, and supported reasoning efforts against the native CLI and
current official documentation. The verified, versioned mapping, rather than
these display labels, becomes executable input.

## Consequences

- Mapping provenance and version must be visible in preview and recorded with
  generated agent state, so regeneration can explain a model change.
- Native projection remains target-specific. An ordered Copilot fallback list
  does not imply that Claude Code or Codex offer the same fallback semantics.
- User overrides are explicit local policy; an update of maintained defaults
  never edits them. A missing or invalid override is reported before writing
  generated agent files.
- Mapping updates and agent regeneration are separate operations. Existing
  installations change only through an explicit install or regeneration action.

## Alternatives considered

**Resolve the latest models online during each install.** Rejected because
availability, documentation, and account access vary, making output
non-reproducible.

**Use Copilot `auto` for each tier.** Rejected because it does not enforce a
specific authored tier.

**Lock Copilot agents to one required model.** Rejected as the default because
plan-dependent access can make a reusable agent unavailable. Authors needing
strict enforcement require a separate explicit policy decision.

## References

- [Agent customization roadmap](../wip/specs/2026-10-11-agent-customization-roadmap.md)
- [GitHub Copilot CLI command reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference)
- [GitHub Copilot CLI programmatic reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-programmatic-reference)
