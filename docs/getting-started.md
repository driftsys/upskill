# Getting started

## Install

```bash
# Linux / macOS (Windows: run inside WSL)
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh
```

`UPSKILL_VERSION` pins a release tag (default: latest) and
`UPSKILL_INSTALL_DIR` overrides the install location (default
`$HOME/.local/bin`). Windows users run the same command inside WSL.

With a Rust toolchain instead:

```bash
cargo install upskill
```

Or download a pre-built binary directly from the [releases page][releases].

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

This writes `code-review/SKILL.md` with the minimum frontmatter the
format spec requires (item directories live at one level under the
source-registry root per format-spec §2.1). Open it, replace the `TODO` description and
body, then run:

```bash
upskill lint --strict   # validate against the format spec
upskill fmt             # canonicalise YAML frontmatter
```

You can also scaffold rules (`upskill new rule <name>`) and agents
(`upskill new agent <name>`).

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

- **[Commands](./commands.md)** — every verb explained, with flags.
- **[Recipes](./recipes.md)** — CI, private repos, pinning, cleanup.
- **[Portable format](./format-spec.md)** — the on-disk SSOT contract
  the lint validates against.

[releases]: https://github.com/driftsys/upskill/releases
