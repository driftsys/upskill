# Getting started

## Install

```bash
cargo install upskill
```

Or download a pre-built binary from the [releases page][releases].

`upskill` is a single static binary with no runtime dependencies.

## Install your first source (consumer)

Inside a project where you want AI clients to pick up rules / skills /
agents:

```bash
# Install everything from a source repo
upskill add driftsys/skills

# Or install a curated bundle
upskill add driftsys/bundles:platform-baseline.bundle.md

# Pin to a tag, branch, or commit
upskill add driftsys/skills@v1.2.0
```

Generated files land in `.claude/`, `.github/`, and `.agents/`. The
`.upskill-lock.json` file in your repo records what was installed and
at which ref — **commit it** so other developers and CI regenerate the
same content.

Verify state any time:

```bash
upskill list      # what's installed
upskill doctor    # any drift between lockfile and on-disk output?
upskill update    # re-fetch sources and regenerate
```

## Scaffold your first item (author)

Inside a source-registry repo (where SSOT items live):

```bash
upskill new skill code-review
```

This writes `skills/code-review/SKILL.md` with the minimum frontmatter
the format spec requires. Open it, replace the `TODO` description and
body, then run:

```bash
upskill lint --strict   # validate against the format spec
upskill fmt             # canonicalise YAML frontmatter
```

You can also scaffold rules (`upskill new rule <name>`) and agents
(`upskill new agent <name>`).

## Where to next

- **[Commands](./commands.md)** — every verb explained, with flags.
- **[Recipes](./recipes.md)** — CI, private repos, pinning, cleanup.
- **[Portable format](./format-spec.md)** — the on-disk SSOT contract
  the lint validates against.

[releases]: https://github.com/driftsys/upskill/releases
