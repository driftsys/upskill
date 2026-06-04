# Design: MCP local-install convention (v2 — docs only)

**Date**: 2026-06-03
**Status**: Approved (brainstorming)
**Builds on**: v1 MCP support ([spec](2026-06-03-mcp-support-design.md), [ADR-0010](../../adr/0010-mcp-config-write.md), PR #218)

## TL;DR for a cold-start executor

**This is a documentation-only deliverable. No Rust. No new upskill capability.**
v2 is an _authoring convention_ that already works on v1 + existing
rule/skill/agent/bundle primitives. Ship exactly two content edits:

1. A recipe in `docs/recipes.md` — _"Shipping an MCP that needs a local install."_
2. A short subsection in `skills/prompt-distilling/SKILL.md` — _"Provisioning an
   MCP server"_ — naming the placement decision and linking to the recipe.

Follow the `writing-skills` discipline for edit #2 (it is SSOT skill content in
this repo). Edit #1 is ordinary user-guide prose.

## Why this is docs-only (the reasoning, so it isn't re-litigated)

The v1 design sketched a larger "v2": a `requires: { mcps }` dependency edge, an
auto-injected skill-body "preflight," and a generic install agent. We
pressure-tested all of it and dropped it:

- **upskill cannot trigger/dispatch a subagent.** It is a ~3 MB CLI with no LLM
  and no agent runtime. Only the _client's_ model (e.g. Claude Code) can dispatch
  a subagent. upskill's only lever is producing content the model reads. So any
  "auto-install" mechanism reduces to _content the model acts on_ — which authors
  can write directly.
- **No auto-injection needed.** The trigger line ("if the MCP is missing, run its
  installer agent") can be authored in the skill body. upskill auto-injecting it
  would be optional sugar, not a requirement — so it is out of scope.
- **Per-MCP installer agent, not a generic one.** Bare-binary install is too
  heterogeneous for a declarative matrix or a generic agent guessing from hints.
  A per-MCP `<name>-mcp-installer` _agent_ lets the author encode the exact recipe
  in prose. That is just `agent` SSOT content — already a first-class kind.
- **MCP stays a v1 bundle descriptor.** No MCP-as-a-kind, no `requires.mcps`, no
  migration. Keeping v1 as-is is a deliberate decision.

Everything the pattern needs already ships in v1: a bundle can carry
`items.skills`, `items.agents`, and an `mcps:` config simultaneously, and one
`upskill add <bundle>` installs all three.

## The convention (what the recipe documents)

Goal: ship an MCP server whose local binary has no self-fetching launcher, so it
must be installed/verified on the consumer machine.

1. **Configure the MCP** with v1's bundle `mcps:` descriptor.
   - Remote server → `remote: { type, url }`. No installer needed.
   - Local server launched by `npx -y <pkg>` / `uvx <pkg>` / `docker run …` →
     `local: { command, args }`. **No installer needed** — the launcher
     self-fetches on first run. State this prominently: most local servers need
     nothing.
   - Local server that is a **bare binary** with no launcher → `local: { command:
     <binary>, … }`, and proceed to step 2.
2. **Author a per-MCP installer agent** `<name>-mcp-installer` (an `agent` item).
   Its system prompt is the install recipe for that one server: detect the OS,
   check the version (`<binary> --version` ≥ X), install via the right channel
   (`brew install …` / download+chmod / `cargo install …`), and run any one-time
   setup. Scaffold a stub with `upskill new agent <name>-mcp-installer`.
   - Give it a narrow tool surface (it needs `Bash`).
   - Its body should be explicit that it must obtain user approval before running
     install commands (the client's permission prompts are the real gate).
3. **Add the trigger to the consuming skill.** In the skill that teaches using the
   MCP, add one authored line near the top, e.g.:
   > If the `<name>` MCP server is not responding (a tool call fails because the
   > server/binary is missing), dispatch the `<name>-mcp-installer` agent to
   > install it, then retry.
   > This is plain authored prose — upskill injects nothing.
4. **Bundle the three together** so they install as a unit:

   ```yaml
   # <name>-mcp.bundle.yaml
   schema: 1
   name: <name>-mcp
   description: <name> MCP server, its skill, and its local-install agent
   items:
     skills: [<name>-diagrams] # the skill that uses + triggers
     agents: [<name>-mcp-installer] # the per-MCP installer
   mcps:
     <name>:
       local: { command: <binary>, args: [...] }
       requires-env: [...] # if any
   ```

   `upskill add <source>` then installs the skill, installs the installer agent
   into `.claude/agents/`, and configures the MCP — all at once.

**Runtime flow:** the consumer uses the skill → the model reads the trigger line
→ if the MCP tool is missing, the model dispatches `<name>-mcp-installer` → the
agent installs the binary under the client's permission prompts → the model
retries. upskill is never in this loop after `add`.

**Trust framing to state in the recipe:** upskill never downloads, builds, or
runs server/binary code. It configures the MCP (CLI/config-write) and installs
the installer agent as content. The _client's_ agent runs the install, gated by
the client's own "allow this command?" prompts. No new RCE surface.

## Deliverable 1 — `docs/recipes.md` recipe

Add a recipe section _"Shipping an MCP that needs a local install."_ Structure:

- One-paragraph problem statement (bare binary, no launcher).
- "First, do you even need this?" — remote and `npx -y`/`uvx`/`docker` servers
  need no installer; only a launcher-less bare binary does.
- The 4 steps above, with the bundle YAML example.
- The runtime-flow + trust paragraphs.
- A pointer to [ADR-0010](../../adr/0010-mcp-config-write.md) and the v1 `mcps:`
  format-spec section.

Match the existing recipe style in `docs/recipes.md` (read a sibling recipe for
heading level, voice, and code-fence conventions). If `docs/recipes.md` is in the
mdBook `SUMMARY.md`, no new entry is needed (it is an in-page section); confirm.

## Deliverable 2 — `skills/prompt-distilling/SKILL.md` subsection

Add a short subsection — _"Provisioning an MCP server"_ — that frames the
**placement decision**:

- The _install logic_ is heterogeneous reasoning → it belongs in a **subagent**
  (a per-MCP `<name>-mcp-installer`), not a rule, a skill, or a declarative
  config.
- The _trigger_ belongs in the **skill** that uses the MCP (one authored line).
- The _MCP config_ belongs in the **bundle** (`mcps:`), which also bundles the
  skill + installer agent.
- Link to the `docs/recipes.md` recipe for the full how-to.

Keep it brief — prompt-distilling is about _where behavior lives_, so this is a
placement rule plus a pointer, not a tutorial. Follow `writing-skills`: this is
SSOT content; after editing, run `upskill lint` (and `just fmt`) on the skill.
An optional one-line cross-reference may be added to `skills/prompt-design/SKILL.md`
(the framework entry point) pointing at the distilling subsection — author's
discretion.

## Out of scope (explicitly)

- Any Rust/binary change. MCP config stays exactly as v1 ships it.
- `requires: { mcps }`, MCP-as-a-kind, auto-injected preflights, a generic
  installer agent, add-time dispatch hints. All considered and rejected above.

## Testing / acceptance

- `just fmt` clean; `upskill lint` clean on the edited skill; mdBook builds
  (`just book`) if the recipe touches anything the book renders.
- Manual read-through: a third-party author could follow the recipe end-to-end
  and produce a working bundle without reading any source code.

## Branching / sequencing note

These edits reference v1's `mcps:`, which ships in **PR #218** (still in CI at
authoring time). Execute **after #218 merges**, on a fresh branch off updated
`main` (avoids a stacked PR). A new session can pick this up cold from this
document.
