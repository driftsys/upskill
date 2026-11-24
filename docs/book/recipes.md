# Recipes

## CI usage

```bash
# Install without prompts (auto-detects NO_COLOR, non-TTY)
upskill add owner/repo

# Lint in CI (fail on warnings)
upskill lint --strict
```

In a non-TTY environment, `upskill` skips interactive prompts and disables
coloured output when `NO_COLOR` is set.

## Private repositories

upskill never injects credentials into clone URLs — it clones the bare
`https://<host>/...` URL and relies entirely on git's own configuration. A
private source works whenever a manual `git clone <url>` would; set up whichever
git mechanism you already use:

```bash
# Credential helper (stores the token in your OS keychain)
git config --global credential.helper osxkeychain   # macOS

# Or rewrite HTTPS → SSH for a host you have keys for
git config --global url."git@github.com:".insteadOf "https://github.com/"
```

`gh auth setup-git` and `glab auth login` both configure a git credential
helper, which upskill then picks up transparently.

Any https git host works via the full URL form
(`https://git.mycompany.com/team/repo`), including projects nested under groups
to any depth (`https://git.mycompany.com/group/subgroup/team/repo`).

## Pin a source to a specific version

```bash
upskill add owner/repo@v1.2.0
```

The pinned ref is recorded in `.upskill-lock.json`. `upskill update` re-fetches
from the same ref unless you bump it explicitly.

## Install a curated bundle

```bash
upskill add owner/bundles:platform-baseline.bundle.yaml
```

A bundle is a YAML manifest (`*.bundle.yaml`) that names the items it includes
plus any other bundles it depends on. The dependency closure is resolved
transitively before any items are written.

The lockfile records the bundle entry alongside the items, so you can later
remove everything that came from it:

```bash
upskill remove --source github:owner/bundles:platform-baseline.bundle.yaml
```

## See whether a source has a newer version

```bash
upskill update --check              # both scopes
upskill update --check --global     # the global scope only
```

The check asks each git remote for the commit of the recorded ref with
`git ls-remote` and compares it with the commit in `.upskill-lock.json`. It
fetches nothing and changes no file. When it reports `update available`, run
`upskill update` (project) or `upskill update --global`. A source reported as
`no recorded commit` needs one `upskill update` first. An item of a git source
that `update` skipped because of a modified file keeps its older commit, so its
source stays `update available` until you resolve that item (a local path source
is compared by content and has no commit).

## Report available updates when a session starts

`upskill update --notice` prints one line for each scope that has updates,
prints a given set of updates once (with `--repeat`, at every run until
`upskill update` runs), contacts the sources at most once in 24 hours, and exits
with code 0 in every case after the arguments are parsed (also when it is
interrupted), so that it cannot make a hook fail. When it has nothing to report
it prints nothing.

The recommended way to call it from a session-start hook is the `upskill`
plugin, which the `driftsys/metapowers` marketplace publishes. Install it for
Claude Code:

```bash
claude plugin marketplace add driftsys/metapowers
claude plugin install upskill@metapowers
```

or for Copilot CLI:

```bash
copilot plugin marketplace add driftsys/metapowers
copilot plugin install upskill@metapowers
```

What you get depends on the client:

- Claude Code shows one line at session start. You see it, and the agent
  receives it.
- Copilot CLI gives the notice to the agent. It has no documented way to show
  the output of a session-start hook to you. The plugin therefore asks the agent
  to tell you at the start of its first reply. **In Copilot CLI you see the
  notice only when the agent follows the request.** In 5 runs in prompt mode
  with Copilot CLI 1.0.112 and the plugin loaded from a directory, the agent did
  so each time. In 3 runs in the interactive terminal of the same version, the
  agent did not do so in any run. For this reason the Copilot plugin calls
  `upskill update --notice --repeat`: the notice is given to the agent at every
  session start until `upskill update` runs. The repeat ends for an item when an
  applied `update` reports the item as updated, up to date, completed after an
  interruption, or removed; a dry run, an `update` that stops with an error, and
  a modified item that `update` skips do not end it. Issue
  [#530](https://github.com/driftsys/upskill/issues/530) records the trials.

In Claude Code a given set of updates is announced once.

The plugin needs an `upskill` binary on `PATH` that has `update --notice`.
Release 0.9.0 is the first release that has it. The Copilot plugin needs a
binary that has `update --notice --repeat`, and release 0.9.0 does not have it:
with release 0.9.0, the Copilot hook prints nothing. When the binary is missing
or older, the hook prints nothing.

A hand-written hook that only runs `upskill update --notice` does not show the
notice to you in either client. Claude Code gives a plain line to the agent and
does not display it. Copilot CLI ignores a plain line: it reads only a JSON
object with `additionalContext`, and gives that text to the agent. To write your
own hook, start from the plugin sources in `plugins/` of the upskill repository,
which wrap the line in the JSON that each client reads.

See [`update --notice`](commands.md#update---notice) for the output, the cache
file and the exit codes. A session without network access records the attempt
and tries again after 24 hours.

## Bisect drift

If a teammate's `.upskill-lock.json` shows different content than yours after
pulling main:

```bash
upskill doctor              # see which bucket the drift is in
upskill update --dry-run    # see which sources would change
upskill update              # apply
```

`update` is always-fetch and idempotent; running it twice produces no diffs the
second time.

## Author workflow

```bash
# Inside a source-registry repo, under the recommended skills/ root
# (see docs/conventions.md — items and *.bundle.yaml both live here).
cd skills/
upskill new skill my-skill
$EDITOR my-skill/SKILL.md

# Validate as you go (run from the registry root)
cd ..
upskill lint
upskill fmt

# Pre-publish guard rail
upskill lint --strict
```

`upskill fmt` is idempotent — files already in canonical form aren't rewritten,
so you can run it on every commit hook without churn.

## Shipping an MCP that needs a local install

A bundle's `mcps:` map configures an MCP server into the consumer's client (see
[format-spec §3.7](format-spec.md) and
[ADR-0010](https://github.com/driftsys/upskill/blob/main/docs/adr/0010-mcp-config-write.md)).
For most servers that is all you need. This recipe covers the one case that
needs more: a **local server that is a bare binary with no self-fetching
launcher**, so the binary must be installed on the consumer machine.

**First — do you even need an installer?**

- Remote server (`remote: { type, url }`) → no installer; it is already running.
- Local server launched by `npx -y <pkg>`, `uvx <pkg>`, or `docker run …` → no
  installer; the launcher fetches and caches the server on first run.
- Local server that is a **bare binary** (a compiled tool, a Homebrew formula) →
  continue below.

**The pattern.** upskill has no way to install the binary itself — it is a CLI,
not an agent. Instead, ship the install knowledge as content and let the
consumer's AI agent run it:

1. **Author a per-MCP installer agent.** Scaffold one with
   `upskill new agent <name>-mcp-installer`. Its body is the install recipe for
   that one server — detect the OS, check the version, install via the right
   channel (`brew install …`, download + `chmod`, `cargo install …`), run any
   one-time setup. Give it the `Bash` tool, and state in its body that it must
   get the user's approval before running install commands.

2. **Add a trigger to the skill that uses the MCP.** Near the top of the skill
   body, add one line of plain prose:

   > If the `<name>` MCP server is not responding (a tool call fails because the
   > binary is missing), dispatch the `<name>-mcp-installer` agent to install
   > it, then retry.

3. **Bundle the three together** so one install brings them all:

   ```yaml
   # <name>-mcp.bundle.yaml
   schema: 1
   name: <name>-mcp
   description: <name> MCP server, its skill, and its local-install agent
   items:
     skills: [<name>-diagrams]
     agents: [<name>-mcp-installer]
   mcps:
     <name>:
       local: { command: <binary>, args: [...] }
   ```

```bash
upskill add owner/registry:<name>-mcp.bundle.yaml --claude
```

The `--claude` flag restricts this install to Claude Code (see
[client selection](commands.md#upskill-add-source-items)); drop it to target
every client. This installs the skill, installs the `<name>-mcp-installer` agent
into the client's agents directory, and configures the MCP — in one step.

**At runtime:** the consumer uses the skill, the model reads the trigger line,
and if the MCP tool is missing it dispatches `<name>-mcp-installer` — which
installs the binary under the client's own "allow this command?" prompts — then
retries. upskill is never in this loop after `add`: it configures the server and
ships the installer agent as content, but the client's agent does the
installing, gated by the client's permission prompts.

## Publish a bundle as client plugins

From the source registry, build both client directories:

```bash
upskill pack my-bundle --source owner/repo@v1 --out dist
```

Copy `dist/my-bundle-claude` and `dist/my-bundle-copilot` into your marketplace
repository and configure publication there. upskill does not install, register
or publish these plugins. Add README and LICENSE files yourself if needed;
`pack --force` replaces the selected output directories, so files that you add
there are deleted.

Install with the client's marketplace command. When the output includes rules,
call `/<plugin>:init` in an interactive session in each project to install only
rules through `add --kind rule --project` for that client. Init reports
third-party plugins for separate setup. The plugin holds the command items of
the bundle closure as user-only skills at `skills/<name>/SKILL.md`, with their
resources beside them. Call them as `/<plugin>:<name>`. It also holds the MCP
servers in `.mcp.json` and its LSP servers in `.lsp.json` for Claude Code or
`lsp.json` for Copilot CLI. Declare LSP servers in the bundle's
[`lsps` map](https://github.com/driftsys/upskill/blob/main/docs/topics/0001-content-format.md#lsps-map).
The plugin does not install the server binary. `upskill add` and
`upskill update` do not configure LSP servers. They report their count in the
same stderr skip line as command items, unless `--quiet` or a kind restriction
applies. Run `upskill update` in the project to update the rules that init
installed; the client marketplace manages plugin content.

Plugin output is experimental until a smoke test with a hosted git marketplace
passes in both clients (the local-marketplace smoke test of 2026-11-19 passed).
Check for duplicate skills from an existing `add` install and check that Copilot
does not also load the Claude directory. See
[pack](commands.md#upskill-pack-bundle---source-source---out-dir) for the output
layout and limits.
