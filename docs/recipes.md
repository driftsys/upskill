# Recipes

## CI usage

```bash
# Install without prompts (auto-detects NO_COLOR, non-TTY)
upskill add owner/repo

# Lint in CI (fail on warnings)
upskill lint --strict
```

In a non-TTY environment, `upskill` skips interactive prompts and
disables coloured output when `NO_COLOR` is set.

## Private repositories

upskill never injects credentials into clone URLs — it clones the bare
`https://<host>/...` URL and relies entirely on git's own configuration.
A private source works whenever a manual `git clone <url>` would; set up
whichever git mechanism you already use:

```bash
# Credential helper (stores the token in your OS keychain)
git config --global credential.helper osxkeychain   # macOS

# Or rewrite HTTPS → SSH for a host you have keys for
git config --global url."git@github.com:".insteadOf "https://github.com/"
```

`gh auth setup-git` and `glab auth login` both configure a git credential
helper, which upskill then picks up transparently.

Any https git host works via the full URL form
(`https://git.mycompany.com/team/repo`), including projects nested under
groups to any depth
(`https://git.mycompany.com/group/subgroup/team/repo`).

## Pin a source to a specific version

```bash
upskill add owner/repo@v1.2.0
```

The pinned ref is recorded in `.upskill-lock.json`. `upskill update`
re-fetches from the same ref unless you bump it explicitly.

## Install a curated bundle

```bash
upskill add owner/bundles:platform-baseline.bundle.yaml
```

A bundle is a YAML manifest (`*.bundle.yaml`) that names the items it
includes plus any other bundles it depends on. The dependency closure
is resolved transitively before any items are written.

The lockfile records the bundle entry alongside the items, so you can
later remove everything that came from it:

```bash
upskill remove --source github:owner/bundles:platform-baseline.bundle.yaml
```

## Bisect drift

If a teammate's `.upskill-lock.json` shows different content than yours
after pulling main:

```bash
upskill doctor              # see which bucket the drift is in
upskill update --dry-run    # see which sources would change
upskill update              # apply
```

`update` is always-fetch and idempotent; running it twice produces no
diffs the second time.

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

`upskill fmt` is idempotent — files already in canonical form aren't
rewritten, so you can run it on every commit hook without churn.

## Shipping an MCP that needs a local install

A bundle's `mcps:` map configures an MCP server into the consumer's client
(see [format-spec §3.7](format-spec.md) and
[ADR-0010](adr/0010-mcp-config-write.md)). For most servers that is all you
need. This recipe covers the one case that needs more: a **local server that
is a bare binary with no self-fetching launcher**, so the binary must be
installed on the consumer machine.

**First — do you even need an installer?**

- Remote server (`remote: { type, url }`) → no installer; it is already
  running.
- Local server launched by `npx -y <pkg>`, `uvx <pkg>`, or `docker run …` →
  no installer; the launcher fetches and caches the server on first run.
- Local server that is a **bare binary** (a compiled tool, a Homebrew
  formula) → continue below.

**The pattern.** upskill has no way to install the binary itself — it is a
CLI, not an agent. Instead, ship the install knowledge as content and let the
consumer's AI agent run it:

1. **Author a per-MCP installer agent.** Scaffold one with
   `upskill new agent <name>-mcp-installer`. Its body is the install recipe
   for that one server — detect the OS, check the version, install via the
   right channel (`brew install …`, download + `chmod`, `cargo install …`),
   run any one-time setup. Give it the `Bash` tool, and state in its body
   that it must get the user's approval before running install commands.

2. **Add a trigger to the skill that uses the MCP.** Near the top of the
   skill body, add one line of plain prose:

   > If the `<name>` MCP server is not responding (a tool call fails because
   > the binary is missing), dispatch the `<name>-mcp-installer` agent to
   > install it, then retry.

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
[client selection](./commands.md#upskill-add-source-items)); drop it to target
every client. This installs the skill, installs the `<name>-mcp-installer`
agent into the client's agents directory, and configures the MCP — in one
step.

**At runtime:** the consumer uses the skill, the model reads the trigger
line, and if the MCP tool is missing it dispatches `<name>-mcp-installer` —
which installs the binary under the client's own "allow this command?"
prompts — then retries. upskill is never in this loop after `add`: it
configures the server and ships the installer agent as content, but the
client's agent does the installing, gated by the client's permission prompts.
