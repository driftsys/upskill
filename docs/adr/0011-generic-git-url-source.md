# Generic git-URL source — replace GitLab-specific source with host-agnostic https

**Status**: Accepted

## Context

The auth-via-git-config refactor (ADR — see commit `e7c4120`, PR #232) changed
upskill to clone bare `https://` URLs and delegate all authentication to git's
own configuration (credential helpers, `url.<base>.insteadOf` rewrites, SSH).
After that change the GitLab-specific source machinery — the `gitlab:` and
`gitlab+<host>:` shorthands, host classification of full URLs, and the typed
`GitlabRepo` struct — no longer provided meaningful value: `gitlab_clone_url()`
simply reassembled `https://{host}/{path}.git` from fields the parser had just
split apart. Meanwhile non-GitLab hosts (Bitbucket, Gitea, Codeberg, corporate
git servers) were excluded for no structural reason.

upskill is pre-1.0 and carries no backward-compatibility obligation for source
labels or lockfile shapes.

## Decision

Replace `InstallSource::Gitlab(GitlabRepo)` with `InstallSource::Git(GitRepo)`.
Any `https://` URL pointing to a host other than `github.com` is treated as an
opaque git remote and stored as a bare URL without host-specific logic.

```
pub struct GitRepo {
    pub url: String,           // bare https remote, no trailing .git
    pub git_ref: Option<String>,
    pub subfolder: Option<String>,
}
```

`InstallSource` is `Github(GithubRepo) | Git(GitRepo) | LocalPath(PathBuf)`.

The `git:` label form is not used. The `Display` for `Git` is the URL itself
(`<url>[@<ref>][:<subfolder>]`), which is already globally unique and
round-trips through `parse_install_source_label` via the `https://` prefix.

The clone URL appended for git operations is `{url}.git`. Shallow/sparse clone
behavior and the auth-via-git-config posture are untouched.

### What the `gitlab:` removal means in practice

The `gitlab:` and `gitlab+<host>:` shorthands emit
`SourceParseError::GitlabShorthandRemoved` (exit code 2) with a message pointing
at the full-URL replacement. The original decision also rejected old lockfile
labels. The dated amendment below supersedes that persisted-state policy only.

### URL normalization

A trailing `/` or `.git` on the input URL is stripped so browser-pasted URLs and
bare-clone URLs normalize to one canonical label in the lockfile.

## Options considered

**Keep `gitlab:` as sugar over the generic source.** Two spellings for one
thing; the shorthand saves almost nothing over pasting the URL. Rejected.

**Collapse GitHub into the generic source too.** The `owner/repo` compact
shorthand and lockfile label are worth keeping a second, typed code path.
`github.com` remains `InstallSource::Github`. Rejected for now.

**Accept ssh URLs directly.** The `@ref` and `:subfolder` suffix syntax clashes
with the scp-style `git@host:path` syntax (the `:` separator is ambiguous). SSH
reachability is already covered by git's `url.<ssh>.insteadOf` rewrites,
consistent with the auth-via-git-config design. Rejected.

**`git+https://` scheme prefix.** upskill has exactly one https fetch strategy
(git clone), so the prefix would disambiguate against nothing and would break
browser-paste UX. Rejected.

## Consequences

Any https git host works: GitLab (including self-hosted instances and subgroups
at any depth), Bitbucket, Gitea, Codeberg, and plain git servers. The only input
forms `add` accepts are:

- `owner/repo[@ref][:path]` — GitHub shorthand (unchanged)
- `https://github.com/owner/repo[...]` — GitHub full URL (unchanged, funnels to
  `Github`)
- `https://<any-other-host>/<path>[@<ref>][:<subfolder>]` — generic git remote
  (new)
- `./path`, `/abs`, `~/path` — local paths (unchanged)

At least one path segment after the host is required. Labels round-trip cleanly
through the lockfile.

## Satisfies

- Acceptance criteria from
  `docs/wip/specs/2026-06-10-generic-git-url-source-design.md`
- Implemented in: `src/source.rs` (`GitRepo`, `parse_git_url`,
  `GitlabShorthandRemoved`), `src/pipeline/git.rs` (`git_clone_url`,
  `install_from_git`)
- Tested by:
  `tests/cli_exit_codes.rs::removed_gitlab_shorthand_exits_two_with_url_hint`,
  `src/source.rs::parse_git_url_*`, `src/pipeline/git.rs::git_clone_url_*`

## Amendment: persisted GitLab source labels (2026-10-12)

Foundation design section 5 and issue #236 require existing schema-1 state to
remain usable before the ownership migration. `parse_install_source_label`
therefore decodes these historical labels into generic HTTPS git sources:

- `gitlab:<namespace>/<repository>[@ref][:subfolder]` uses `gitlab.com`.
- `gitlab+<hostname>:<namespace>/<repository>[@ref][:subfolder]` uses the
  recorded hostname, including nested namespaces.
- `gitlab+<hostname>:<port>:<namespace>/<repository>[@ref][:subfolder]` uses the
  recorded numeric HTTPS port. Both colons before the namespace are required.

A hostname consists of non-empty dot-separated ASCII labels, each starting and
ending with a letter or digit and containing only letters, digits and hyphens.
Single-label self-hosted names and bracketed, validated IPv6 addresses are
supported, including `[2001:db8::1]:org/repo` and `[::1]:8443:org/repo`.
Credentials, embedded schemes, whitespace, control characters, malformed IPv6
brackets and unbracketed IPv6 authorities are rejected. A port contains only
decimal digits and has a value from 1 through 65535. A second colon after the
hostname or closing IPv6 bracket and before the first path slash identifies a
port. Without that second delimiter, a numeric namespace remains a namespace:
`gitlab+host:123/repo` means `https://host/123/repo`, while
`gitlab+host:8443:org/repo` means `https://host:8443/org/repo`. Malformed
authority or port delimiters require reconciliation; no remote is guessed. Every
namespace and repository component, and any supplied ref or subfolder, must be
non-empty.

Conversion must round-trip through the existing modern label parser without
changing the generic source, host, repository, ref or subfolder. Historical
repository spellings ending in `.git` or `/`, and authorities that the modern
parser specializes as GitHub sources, require explicit reconciliation instead of
silent normalization or loss of a port. Unsupported or ambiguous state fails
before fetch with its lockfile label and guidance to reconcile a verified HTTPS
URL, ref and subfolder. It remains inspectable with `list` and `doctor`.

A successful applying `update` writes canonical HTTPS source labels for the
items and bundles it reinstalls, including unchanged pinned content.
`update
--dry-run`, `list` and `doctor` preserve lockfile and generated-output
bytes. Alias, pinned ref, subfolder, bundle membership and unrelated
item/plugin/MCP records retain their existing lifecycle behavior. Authentication
still uses only git configuration, credential helpers and URL rewrites.

New `add` input is unchanged: `gitlab:` and `gitlab+` shorthands still exit 2
with the full-HTTPS replacement hint. Modern HTTPS, GitHub and local parsing is
unchanged. This amendment keeps lockfile schema 1; it neither infers artifact
ownership nor adopts files, writes schema 2 or reconciles native integrations.
Legacy projection, ownership verification, migration backup and lifecycle
integration remain separate M1-A2 work.
