# Updater plugin startup probes

Date: **2026-10-14**. Status: **partial installed-client evidence**. This note
records probes for [issue #285](https://github.com/driftsys/upskill/issues/285).
It does not declare a supported client matrix or production hook contract.

## Evidence limits

These dated observations do not establish support for current client releases.
Repeat the relevant probes during updater qualification, including trust,
disablement, reload behavior and startup timing. Preserving this note does not
reproduce or independently verify its historical runtime results.

## Environment

- macOS 15.6.1, Darwin arm64.
- Claude Code 2.1.287.
- GitHub Copilot CLI 1.0.97.
- Codex CLI 0.162.1.
- OpenCode v2.0.25.

The four versions are exact local releases tested as candidates. No minimum
version or compatibility range is established.

## Results

| Client      | Probe                                                                                                                                   | Result                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code | Strictly validate a local plugin with a `SessionStart` command hook, then launch it with `--plugin-dir` under a temporary home.         | Manifest validation passed after adding author metadata. The isolated home stopped at sign-in. With the existing signed-in profile and a one-session trust choice for the temporary project, `SessionStart` wrote a marker with the expected working directory. The temporary trust record was removed after the probe. Runtime invocation verified for 2.1.287 on this host.                                                                                                                                  |
| Copilot CLI | Install a local plugin in an isolated `COPILOT_HOME`; load the local plugin directory, then send one no-tools prompt.                   | The CLI plugin installer accepted the package, and `/plugin list` showed the ephemeral canary. The CLI reported `Loading: 1 plugin, 2 hooks`. A one-turn prompt returned `OK` and the `sessionStart` command wrote its marker. The marker's `cwd` was the plugin directory, not the temporary project. Runtime invocation is verified for 1.0.97 on this host; this cwd behavior needs to be reflected in the eventual updater path design.                                                                    |
| Codex CLI   | Register a local marketplace and install the canary through `codex plugin`; then present the project-local plugin to the installed CLI. | Isolated marketplace registration and installation succeeded. The temporary project trust prompt was accepted in the existing profile, and the CLI opened a normal session. The hook did not write a marker. A targeted scan found no trust entry for this project in `~/.codex/config.toml`, so there was no persisted project entry to remove. The official plugin guide currently limits bundled lifecycle hooks to manually installed Codex desktop plugins; CLI support is not established by that guide. |
| OpenCode    | Load a project-local plugin from `.opencode/plugins/` using the v2 API.                                                                 | OpenCode logged a load failure for the v1-style named export. The v2 `Plugin.define({ id, setup })` export loaded, and the setup function wrote a marker with the temporary project path. Startup entrypoint verified for v2.0.25 on this host.                                                                                                                                                                                                                                                                |

Canary packages, projects, and markers were under
`/private/tmp/upskill-283-canaries`. Claude and Copilot were launched with the
existing signed-in profile, but the canary was not installed into the normal
plugin configuration. Copilot received only the one-turn `OK` prompt described
above. Claude's temporary project trust record was removed after its successful
probe. Codex's temporary project trust prompt was accepted; the targeted scan
found no matching project entry in `~/.codex/config.toml` to remove.

## Startup timing sample

For OpenCode, three local server starts without a plugin and three with the
project plugin were timed from process launch until `GET /global/health`
returned successfully:

| Condition         | Samples (ms)        | Median (ms) |
| ----------------- | ------------------- | ----------: |
| No project plugin | 392.2, 150.4, 166.0 |       166.0 |
| Project plugin    | 171.7, 149.4, 136.3 |       149.4 |

This small local sample is noisy and does not isolate the plugin's incremental
cost. It measures server readiness, not interactive TUI readiness or a
maintenance hook. It is preliminary evidence only; an agreed startup budget and
repeatable measurement method are still needed.

## Open qualification work

- Confirm Copilot CLI 1.0.97's hook working-directory contract and whether it
  can be configured; design marker/state paths independently of the project cwd.
- Test Codex plugin hook execution in a disposable profile or use the manually
  installed desktop surface. Confirm whether MVP maintenance targets Codex
  desktop, Codex CLI, or both.
- Test explicit plugin disablement, Codex hook trust, and reload/new-session
  behavior on every selected profile.
- Select the supported OS/version matrix and OpenCode v1/v2 boundary.
- Agree on an interactive startup baseline and overhead budget, then repeat the
  measurement with the actual maintenance entrypoint.

## References

- [Claude Code plugin manifest reference](https://code.claude.com/docs/en/plugins-reference)
- [GitHub Copilot CLI plugin reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference)
- [Codex plugin packaging and lifecycle hooks](https://developers.openai.com/plugins/build/plugins)
- [OpenCode v2 plugin API](https://dev.opencode.ai/v2/docs/build/plugins/)
- [OpenCode server health endpoint](https://dev.opencode.ai/docs/server/)
