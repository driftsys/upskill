# T3: smoke test of `upskill pack` output with a marketplace install

Date: 2026-11-19. Binary: `upskill` built from `main` at `553590a`. Clients:
Claude Code 2.1.308 and GitHub Copilot CLI 1.0.112, on macOS. Each result is
from one run, except where a second prompt is named; model answers can vary
between runs.

## Setup

- Source: a copy of the test registry `tests/fixtures/pack/registry`, as a git
  repository. Bundle `demo` holds skill `alpha`, agent `helper` and rule
  `style`; through `requires: base` it also brings skill `beta` and rule
  `naming`.
- `upskill pack demo --source <registry> --out <market>/plugins` wrote
  `demo-claude/` and `demo-copilot/`, 7 files each.
- A local marketplace repository held both directories and both manifests:
  `.claude-plugin/marketplace.json` (source `./plugins/demo-claude`) and
  `.github/plugin/marketplace.json` (source `./plugins/demo-copilot`), in the
  format of `driftsys/metapowers`.
- Claude Code used scope `local` in a temporary project. Every run used a
  temporary `XDG_CACHE_HOME`.

## Results

| Check                                         | Claude Code                                                     | Copilot CLI                                                                       |
| --------------------------------------------- | --------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Install from the marketplace                  | pass (`claude plugin install demo@demomarket --scope local`)    | pass (`copilot plugin install demo@demomarket`, "Installed 2 skills")             |
| Skills visible                                | pass: `demo:alpha`, `demo:beta`                                 | pass on a second prompt: `alpha`, `beta` (no plugin prefix), and `/skills list`   |
| Agent visible                                 | pass: `demo:helper`                                             | pass: `demo:helper`                                                               |
| Init command visible                          | pass: `/demo:init`                                              | pass: skill `init`                                                                |
| Init ran and wrote only rules                 | pass: `.claude/rules/style.md`, `naming.md`; no skill or agent  | pass: `.github/instructions/*.instructions.md`; no skill or agent                 |
| Lockfile records the restriction              | pass: bundles `base` and `demo` with `"kinds": ["rule"]`        | pass: same                                                                        |
| Session notice after a change of rule `style` | pass: `upskill: 1 update available (project)` reached the model | pass (prompt mode, `copilot -p "Say OK."`): the line was printed before the reply |
| Uninstall and marketplace removal             | pass                                                            | pass                                                                              |

Copilot CLI read only `.github/plugin/marketplace.json` and installed
`plugins/demo-copilot`; it did not load the Claude directory of the same
repository. The reverse check on Claude Code was not run.

## Observations

- The Copilot CLI notice result is from prompt mode (`copilot -p`), where the
  hook line was printed before the reply. It does not change the interactive
  result of [T2](t2-session-notice-display.md), where Copilot CLI 1.0.112 did
  not show the notice in 0 of 3 interactive sessions.

- In a non-interactive Copilot CLI run, the init command needed
  `--allow-tool='shell(upskill:*)'` and also `--allow-all-paths`, because the
  source path was outside the project. With the first flag only, the shell call
  was denied.
- The init command also wrote the first-time ancillary files (`CLAUDE.md`,
  `opencode.json`, `.vscode/settings.json`), as any `upskill add` does.
- The notice also reported the global scope of the real home directory, because
  the hook checks both scopes.
- One listing prompt in Copilot CLI answered that no plugin skill was present; a
  second prompt and `/skills list` showed both skills. A model's answer about
  its own context is weak evidence.
- Claude Code uninstall left the plugin copy under
  `~/.claude/plugins/cache/demomarket/` marked as orphaned, and Copilot CLI left
  `~/.copilot/plugin-data/demomarket/demo`. User settings files
  (`~/.claude/settings.json`, `~/.copilot/config.json`) were byte-identical
  before and after.

## Not tested

A hosted git marketplace (the marketplace was a local directory), Windows and
Linux, and an update of an installed plugin through the client's marketplace.
