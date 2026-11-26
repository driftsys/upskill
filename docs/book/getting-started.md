# Getting started

## Set up the upskill plugin

The `upskill` plugin is the first setup path for Claude Code and Copilot CLI.
The `driftsys/metapowers` marketplace publishes it. Install it for each client
that you use:

```bash
claude plugin marketplace add driftsys/metapowers
claude plugin install upskill@metapowers

copilot plugin marketplace add driftsys/metapowers
copilot plugin install upskill@metapowers
```

The plugin gives you the upskill skills and a session-start hook. When the
`upskill` CLI is missing, the hook prints the command that installs it, at most
once a day. When the CLI is present, the hook runs `upskill update --notice`, so
you see updates of installed content and of the CLI release.

For opencode, or when you want no plugin, install the CLI (next section) and run
`upskill add driftsys/upskill --global --opencode`.

If you installed the upskill skills with `upskill add driftsys/upskill`, the
plugin now provides them, so each skill would exist twice, and the old install
keeps the old names (`upskill-cli` is now `upskill-managing-content`, and the
bundle `prompt-engineering` is now `upskill`). Run `upskill remove` on those
skills before your next `upskill update`, and add `--global` for a global
install:

```bash
upskill remove --global upskill-cli upskill-prompt-design upskill-prompt-distilling upskill-writing-rules upskill-writing-subagents upskill-evaluating-prompts upskill-writing-bundles
```

List the installed names with `upskill list`. Remove the old upskill skills
before the next `upskill update`. A lockfile that records the bundle
`prompt-engineering` makes `update` stop until its items are removed. For an
install by item name, `update` removes the old `upskill-cli` and keeps the other
six old skills beside the plugin's copies.

## Install

**Linux / macOS:**

```bash
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh
```

`UPSKILL_VERSION` pins a release tag (default: latest) and `UPSKILL_INSTALL_DIR`
overrides the install location (default `$HOME/.local/bin`).

**Windows (native — CMD / PowerShell):**

```powershell
cargo install upskill
```

Requires a [Rust toolchain](https://rustup.rs). The binary has no other runtime
dependencies. `HOME` and `USERPROFILE` are both recognised as the global install
root, so the standard Windows `USERPROFILE` variable works without any extra
setup.

**Windows (WSL):** use the Linux install command above inside your WSL terminal.

**Cross-platform alternative:**

```bash
cargo install upskill
```

Or download a pre-built binary directly from the [releases page][releases].

`upskill` is a single static binary with no runtime dependencies.

## Install your first source (consumer)

Inside a project where you want AI clients to pick up rules / skills / agents:

```bash
# Install everything from a source repo
upskill add driftsys/skills

# Or install a curated bundle (by name — resolves the .bundle.yaml automatically)
upskill add driftsys/bundles platform-baseline

# Pin to a tag, branch, or commit
upskill add driftsys/skills@v1.2.0
```

Generated files land in `.claude/`, `.github/`, and `.agents/`. The
`.upskill-lock.json` file in your repo records what was installed and at which
ref — **commit it** so other developers and CI regenerate the same content.

Verify state any time:

```bash
upskill list      # what's installed
upskill doctor    # any drift between lockfile and on-disk output?
upskill update    # re-fetch sources and regenerate
```

## Configuring registries

To search content from corporate or custom registries, add them to your config:

```yaml
# ~/.config/upskill/config.yaml
registries:
  - name: corp
    source: https://gitlab.com/mycompany/ai-skills
  - name: team
    source: myorg/team-skills
```

Then `upskill search` will include results from all configured registries. You
can also search a specific registry:

```bash
upskill search auth --registry corp
```

## Scaffold your first item (author)

Inside a source-registry repo (where SSOT items live), the recommended layout
puts all items and bundles under a top-level `skills/` directory (see
[Conventions](conventions.md)):

```bash
mkdir -p skills && cd skills
upskill new skill code-review
```

This writes `skills/code-review/SKILL.md` with the minimum frontmatter the
format spec requires (item directories live at one level under the
source-registry root per format-spec §2.1). Open it, replace the `TODO`
description and body, then run:

```bash
upskill lint --strict   # validate against the format spec
upskill fmt             # canonicalise YAML frontmatter
```

You can also scaffold rules (`upskill new rule <name>`) and agents
(`upskill new agent <name>`). Bundles are plain `*.bundle.yaml` files that sit
alongside the items they reference, also under `skills/`.

## Man pages

`upskill` ships a man-page generator. Run it from the source tree:

```bash
just man             # or: cargo run --example mangen --release
```

That writes one `.1` per command into `target/man/`:

```text
target/man/upskill.1
target/man/upskill-add.1
target/man/upskill-doctor.1
…
```

Install system-wide:

```bash
sudo cp target/man/*.1 /usr/local/share/man/man1/
man upskill          # or: man upskill-add
```

## Where to next

- **[Commands](commands.md)** — every verb explained, with flags.
- **[Recipes](recipes.md)** — CI, private repos, pinning, cleanup.
- **[Portable format](format-spec.md)** — the on-disk SSOT contract the lint
  validates against.

[releases]: https://github.com/driftsys/upskill/releases
