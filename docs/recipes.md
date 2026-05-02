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

Token resolution per host:

```bash
# GitHub
export GITHUB_TOKEN=ghp_...      # or GH_TOKEN, or rely on `gh auth token`

# GitLab
export GITLAB_TOKEN=glpat_...    # or GL_TOKEN, or rely on `glab auth token`
```

Resolution order:

| Host   | Order                                                             |
| ------ | ----------------------------------------------------------------- |
| GitHub | `GITHUB_TOKEN` → `GH_TOKEN` → `gh auth token` → unauthenticated   |
| GitLab | `GITLAB_TOKEN` → `GL_TOKEN` → `glab auth token` → unauthenticated |

Self-hosted GitLab is supported via the full URL form
(`https://gitlab.mycompany.com/team/repo`).

## Pin a source to a specific version

```bash
upskill add owner/repo@v1.2.0
```

The pinned ref is recorded in `.upskill-lock.json`. `upskill update`
re-fetches from the same ref unless you bump it explicitly.

## Install a curated bundle

```bash
upskill add owner/bundles:platform-baseline.bundle.md
```

A bundle is a manifest (`*.bundle.md`) that names the items it
includes plus any other bundles it depends on. The dependency closure
is resolved transitively before any items are written.

The lockfile records the bundle entry alongside the items, so you can
later remove everything that came from it:

```bash
upskill remove --source github:owner/bundles:platform-baseline.bundle.md
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
# Inside a source-registry repo
upskill new skill my-skill
$EDITOR skills/my-skill/SKILL.md

# Validate as you go
upskill lint
upskill fmt

# Pre-publish guard rail
upskill lint --strict
```

`upskill fmt` is idempotent — files already in canonical form aren't
rewritten, so you can run it on every commit hook without churn.

## Migrating from v0.1

v0.1 users on `.upskill-lock.json` get an automatic in-place migration
on the first run of any v0.2 command — the file is rewritten with the
new `schema: 2` entry shape and no manual step is required.

The legacy verbs `install` and `uninstall` are gone in v0.2; use
`add` and `remove` instead. `check` is replaced by `doctor`.
