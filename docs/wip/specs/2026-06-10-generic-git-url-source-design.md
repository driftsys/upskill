# Generic git-URL source — design

Date: 2026-06-10
Status: approved (brainstorm), pending implementation plan
Branch: `refactor/generic-git-url-source` (based on
`refactor/clone-auth-git-config-only`)

## Problem

Since the clone-auth refactor (`e7c4120`), upskill clones with the bare
`https://<host>/...` URL and delegates all authentication to git's own
configuration. The GitLab-specific source machinery — the `gitlab:` /
`gitlab+<host>:` shorthands, host classification of full URLs, and the
typed `GitlabRepo` — no longer buys anything: `gitlab_clone_url()` merely
reassembles `https://{host}/{path}.git` from pieces the parser split
apart. Meanwhile, non-GitLab hosts (Bitbucket, Gitea, Codeberg, corporate
git servers) are excluded for no reason.

## Decision

Replace the GitLab-specific source with a generic git-URL source: any
`https://` URL is a clonable git remote. "GitLab support" stops being a
feature — it is just git.

### Decisions taken during brainstorm

1. **Shorthands deleted outright.** `gitlab:owner/repo` and
   `gitlab+<host>:...` input forms and lockfile labels are removed with no
   migration (pre-1.0 no-backcompat policy). Old lockfiles holding such
   labels hard-fail at parse with a clear error.
2. **GitHub variant kept.** `owner/repo[@ref][:path]` shorthand and
   `https://github.com/...` URLs continue to classify as
   `InstallSource::Github` with the compact `owner/repo` lockfile label.
   Only non-GitHub URLs go through the new generic source.
3. **`https://` only.** No `ssh://` or scp-style `git@host:path` input,
   and no `git+https://` scheme prefix. SSH users reach SSH via git's
   `url.<ssh>.insteadOf` rewrite, consistent with the auth-via-git-config
   design.

## Design

### Source model (`src/source.rs`)

Delete `GitlabRepo` and `InstallSource::Gitlab`. Add:

```rust
pub struct GitRepo {
    pub url: String,            // bare https remote, no trailing .git
    pub git_ref: Option<String>,
    pub subfolder: Option<String>,
}
```

`InstallSource` becomes `Github(GithubRepo) | Git(GitRepo) | Local(PathBuf)`.

### Parsing (`parse_install_source`)

- Local paths and GitHub shorthand `owner/repo[@ref][:path]` — unchanged.
- `https://github.com/...` — still classifies as `Github` (unchanged).
- `https://<any-other-host>/<path>[@<ref>][:<subfolder>]` — parses to
  `Git`. Host and path are opaque: subgroups at any depth, ports, and any
  git host work identically. A trailing `.git` on the path is stripped.
  At least one path segment is required after the host.
- `gitlab:` and `gitlab+<host>:` prefixes — parse error
  (`SourceParseError`), with a message pointing at the full-URL form.

### Labels (`Display` + `parse_install_source_label`)

The `Git` label round-trips as `url[@ref][:subfolder]` — i.e. the input
form is the label. GitHub and local labels are unchanged.

### Clone (`src/pipeline/git.rs`)

`install_from_gitlab` and `gitlab_clone_url` collapse into a generic path
cloning `{url}.git`. Shallow/sparse clone behavior and the
auth-via-git-config posture are untouched.

### Docs and tests

- Update the source-format lists in `docs/commands.md`, `AGENTS.md`,
  `README.md`, and `docs/format-spec.md` / `docs/specification.md` where
  `gitlab:` is mentioned.
- Rewrite `parse_gitlab_*` and GitLab label round-trip unit tests as
  generic git-URL tests, keeping the subgroup, port, and ref+subfolder
  cases and adding a non-GitLab host case (e.g. Bitbucket/Gitea).
- Update `pipeline_source` integration tests accordingly; add a test that
  `gitlab:` now fails with a usage error.

## Alternatives considered

- **Keep `gitlab:` as sugar over the generic source** — rejected: two
  spellings for one thing, and the shorthand saves almost nothing over
  pasting the URL.
- **Collapse GitHub into the generic source too** — rejected for now:
  the `owner/repo` shorthand and compact lockfile label are worth the
  second code path.
- **Accept ssh URLs directly** — rejected: `@ref` / `:subfolder` suffixes
  clash with `git@host:path` syntax; `insteadOf` rewrites cover SSH.
- **`git+https://` scheme prefix** — rejected: upskill has exactly one
  https fetch strategy (git clone), so the prefix would disambiguate
  against nothing and breaks browser-paste UX.

## Acceptance criteria

1. `upskill add https://gitlab.example.com/group/sub/repo@v1:skills/x`
   installs exactly as before the refactor.
2. `upskill add https://bitbucket.org/team/repo` clones and installs
   (host-agnostic).
3. `upskill add gitlab:team/repo` exits 2 with a parse error naming the
   full-URL form.
4. Lockfile written by the new code round-trips through
   `list` / `update` / `remove`.
5. `rg -i gitlab src/` only matches host-agnostic strings (tests/fixtures
   using gitlab.com as an example host are fine; no GitLab-specific code
   paths remain).
