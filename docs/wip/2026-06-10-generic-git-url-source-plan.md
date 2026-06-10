# Generic git-URL Source Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the GitLab-specific source machinery with a generic https git-URL source so any git host works, per the approved spec at `docs/wip/2026-06-10-generic-git-url-source-design.md`.

**Architecture:** `InstallSource::Gitlab(GitlabRepo)` becomes `InstallSource::Git(GitRepo)` where `GitRepo` stores the bare https remote URL plus optional ref/subfolder. The `gitlab:` / `gitlab+<host>:` shorthands are deleted with a dedicated parse error pointing at the full-URL form. GitHub keeps its typed variant. Clone behavior (`shallow_clone`, auth via git config) is untouched — only URL construction changes.

**Tech Stack:** Rust edition 2024, `thiserror` (source.rs typed errors), `assert_cmd` + `predicates` + `tempfile` (integration tests). Worktree: `.claude/worktrees/refactor-generic-git-url-source`. All paths below are relative to the worktree root.

**Verification baseline:** `just test` green before starting (branch tip `813173b`).

---

## Task 1: Core source-model swap (`GitRepo` replaces `GitlabRepo`)

This is one atomic refactor — the enum change breaks every consumer, so the crate only compiles again at the end of this task. Tests are written first (ATDD integration test + rewritten unit tests = RED as compile errors and failures), then the implementation and consumer arms.

**Files:**

- Modify: `tests/cli_exit_codes.rs` (ATDD test)
- Modify: `src/source.rs` (model, parsing, labels, unit tests)
- Modify: `src/pipeline/git.rs` (clone dispatch + unit tests)
- Modify: `src/index.rs:352-364` (`clone_url_for`)
- Modify: `src/main.rs:246-263` (`print_install_progress`)
- Modify: `src/pipeline/install.rs:363-369` (`source_git_ref`)
- Modify: `src/pipeline/mod.rs:206,265-269` (doc comment + `git_ref` match)

- [ ] **Step 1: Write the ATDD integration test (acceptance criterion 3)**

Append to `tests/cli_exit_codes.rs` (file already has `mod common;`; add `use predicates::prelude::*;` below the existing `use` lines):

```rust
#[test]
fn removed_gitlab_shorthand_exits_two_with_url_hint() {
    // The `gitlab:` shorthand was deleted in favor of full https URLs.
    // The error must name the replacement so users can self-serve.
    let cwd = tempdir().expect("must create temp dir");
    let home = cwd.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let mut cmd = common::upskill_cmd(&home);

    cmd.current_dir(cwd.path())
        .args(["add", "gitlab:team/skills", "--claude"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("https://gitlab.com"));
}
```

- [ ] **Step 2: Run the ATDD test to verify it fails**

Run: `cargo test --test cli_exit_codes removed_gitlab_shorthand -- --nocapture`
Expected: FAIL — exit code is currently 1 (the shorthand still parses, then the clone of `https://gitlab.com/team/skills.git` fails), and stderr does not contain the hint. If the sandbox has no network, the clone fails faster — the test still fails on the missing stderr hint.

- [ ] **Step 3: Rewrite the GitLab unit tests in `src/source.rs`**

Delete the test functions `parse_gitlab_prefix`, `parse_gitlab_prefix_with_ref`, `parse_gitlab_prefix_with_subfolder`, `parse_gitlab_url`, `parse_selfhosted_gitlab_url`, `parse_selfhosted_gitlab_with_port`, `parse_selfhosted_gitlab_subgroup`, `parse_selfhosted_gitlab_subgroup_with_ref_and_subfolder`, `parse_gitlab_prefix_subgroup`, `reject_gitlab_bare_project_without_namespace`, `label_roundtrip_gitlab_dot_com`, `label_roundtrip_gitlab_self_hosted`, `label_roundtrip_gitlab_self_hosted_subgroup`, `label_rejects_gitlab_plus_without_host` (currently `src/source.rs:504-633` and `674-717`). Keep `parse_github_url` and everything else. In their place add:

```rust
    // Generic git-URL source tests — any https host that is not github.com
    // is an opaque git remote (GitLab, Bitbucket, Gitea, corporate git).

    #[test]
    fn parse_git_url_gitlab_com() {
        let source = parse_install_source("https://gitlab.com/team/skills").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://gitlab.com/team/skills");
        assert_eq!(repo.git_ref, None);
        assert_eq!(repo.subfolder, None);
    }

    #[test]
    fn parse_git_url_bitbucket() {
        let source = parse_install_source("https://bitbucket.org/team/repo").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://bitbucket.org/team/repo");
    }

    #[test]
    fn parse_git_url_self_hosted_with_port() {
        let source =
            parse_install_source("https://git.company.com:8443/team/skills").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://git.company.com:8443/team/skills");
    }

    #[test]
    fn parse_git_url_subgroups_any_depth() {
        let source = parse_install_source(
            "https://gitlabee.dt.renault.com/partners/alliance-car/devex/process/seed",
        )
        .expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(
            repo.url,
            "https://gitlabee.dt.renault.com/partners/alliance-car/devex/process/seed"
        );
    }

    #[test]
    fn parse_git_url_with_ref_and_subfolder() {
        let source = parse_install_source(
            "https://gitlabee.dt.renault.com/partners/seed@v0.2.0:skills/seed.bundle.yaml",
        )
        .expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://gitlabee.dt.renault.com/partners/seed");
        assert_eq!(repo.git_ref.as_deref(), Some("v0.2.0"));
        assert_eq!(repo.subfolder.as_deref(), Some("skills/seed.bundle.yaml"));
    }

    #[test]
    fn parse_git_url_single_path_segment() {
        // Plain git servers can host a repo directly under the root —
        // unlike GitLab there is no namespace requirement.
        let source = parse_install_source("https://git.example.com/repo").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://git.example.com/repo");
    }

    #[test]
    fn parse_git_url_strips_trailing_dot_git() {
        let source =
            parse_install_source("https://gitlab.com/team/skills.git").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://gitlab.com/team/skills");
    }

    #[test]
    fn parse_git_url_strips_trailing_slash() {
        let source = parse_install_source("https://gitlab.com/team/skills/").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://gitlab.com/team/skills");
    }

    #[test]
    fn reject_git_url_without_path() {
        let err = parse_install_source("https://gitlab.com").expect_err("must fail");
        assert_eq!(err, SourceParseError::InvalidFormat);
    }

    #[test]
    fn reject_git_url_with_empty_path() {
        let err = parse_install_source("https://gitlab.com/").expect_err("must fail");
        assert_eq!(err, SourceParseError::EmptySegment);
    }

    #[test]
    fn reject_removed_gitlab_shorthand() {
        let err = parse_install_source("gitlab:team/skills").expect_err("must fail");
        assert_eq!(err, SourceParseError::GitlabShorthandRemoved);
    }

    #[test]
    fn reject_removed_gitlab_plus_shorthand() {
        let err =
            parse_install_source("gitlab+git.company.com:team/skills").expect_err("must fail");
        assert_eq!(err, SourceParseError::GitlabShorthandRemoved);
    }
```

And in the label round-trip section (after `label_roundtrip_github_full`):

```rust
    #[test]
    fn label_roundtrip_git_minimal() {
        assert_label_roundtrip(&InstallSource::Git(GitRepo {
            url: "https://gitlab.com/team/skills".into(),
            git_ref: None,
            subfolder: None,
        }));
    }

    #[test]
    fn label_roundtrip_git_full() {
        assert_label_roundtrip(&InstallSource::Git(GitRepo {
            url: "https://gitlabee.dt.renault.com/partners/alliance-car/devex/process/seed".into(),
            git_ref: Some("v0.2.0".into()),
            subfolder: Some("skills/seed.bundle.yaml".into()),
        }));
    }

    #[test]
    fn label_rejects_removed_gitlab_shorthand() {
        // Old lockfiles may still carry `gitlab:` labels — they hard-fail
        // with the same self-serve error as the CLI input form (pre-1.0,
        // no migration).
        let err = parse_install_source_label("gitlab:team/x@main").expect_err("must reject");
        assert_eq!(err, SourceParseError::GitlabShorthandRemoved);
        let err = parse_install_source_label("gitlab+h.com:team/x").expect_err("must reject");
        assert_eq!(err, SourceParseError::GitlabShorthandRemoved);
    }
```

- [ ] **Step 4: Implement the model and parsing in `src/source.rs`**

Replace `GitlabRepo` (lines 14-21) with:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepo {
    /// Bare https remote without a trailing `.git`, e.g.
    /// `https://gitlab.example.com/group/sub/repo`. The host and path are
    /// opaque — any git host works; auth is git's job (config/helpers).
    pub url: String,
    pub git_ref: Option<String>,
    pub subfolder: Option<String>,
}
```

Change the enum variant (line 26): `Gitlab(GitlabRepo)` → `Git(GitRepo)`.

Replace the `Display` doc comment lines 33-36 and the `Gitlab` arm (lines 49-62):

```rust
/// - `github:<owner>/<name>[@<ref>][:<subfolder>]`
/// - `<url>[@<ref>][:<subfolder>]` — generic git source; the bare https
///   URL is its own label
/// - `local:<path>` (absolute when known, otherwise as-given)
```

```rust
InstallSource::Git(r) => {
    write!(f, "{}", r.url)?;
    if let Some(g) = &r.git_ref {
        write!(f, "@{}", g)?;
    }
    if let Some(s) = &r.subfolder {
        write!(f, ":{}", s)?;
    }
    Ok(())
}
```

Add a `SourceParseError` variant:

```rust
#[error(
    "the `gitlab:` shorthand was removed; use the full https URL instead, \
     e.g. https://gitlab.com/owner/repo"
)]
GitlabShorthandRemoved,
```

In `parse_install_source`, replace the `gitlab:` branch (lines 119-122):

```rust
// The `gitlab:` / `gitlab+<host>:` shorthands were removed in favor of
// full https URLs. Catch them explicitly so the error names the
// replacement instead of falling through to a confusing
// owner/repo parse error.
if source.starts_with("gitlab:") || source.starts_with("gitlab+") {
    return Err(SourceParseError::GitlabShorthandRemoved);
}
```

In `parse_install_source_label`, replace the two gitlab branches (lines 169-180) and update the doc comment list (lines 157-161) to match the new Display forms:

```rust
if let Some(rest) = label.strip_prefix("https://") {
    return parse_url_source(rest);
}
if label.starts_with("gitlab:") || label.starts_with("gitlab+") {
    return Err(SourceParseError::GitlabShorthandRemoved);
}
```

Replace `parse_url_source`'s last line (line 198) and delete `parse_gitlab_source` (lines 201-244), adding in its place:

```rust
    // Any other https host is an opaque git remote.
    parse_git_url(host_part, path_part).map(InstallSource::Git)
}

/// Parse the path portion of a generic git URL:
/// `<path>[@<ref>][:<subfolder>]`. The host is opaque (and may carry a
/// port); the path may nest arbitrarily deep (GitLab subgroups). A
/// trailing `/` or `.git` on the repo path is stripped so browser-pasted
/// and clone URLs normalise to one canonical label.
fn parse_git_url(host: &str, path: &str) -> Result<GitRepo, SourceParseError> {
    // Split off :subfolder first. The port is on the host part, already
    // split away, so a ':' here is always a subfolder separator.
    let (before_subfolder, subfolder) = if let Some((before, sub)) = path.split_once(':') {
        if sub.trim().is_empty() {
            return Err(SourceParseError::EmptySubfolder);
        }
        (before, Some(sub.to_string()))
    } else {
        (path, None)
    };

    // Split off @ref
    let (repo_path, git_ref) = if let Some((before, r)) = before_subfolder.split_once('@') {
        if r.trim().is_empty() {
            return Err(SourceParseError::EmptyRef);
        }
        (before, Some(r.to_string()))
    } else {
        (before_subfolder, None)
    };

    let repo_path = repo_path.trim_end_matches('/');
    let repo_path = repo_path.strip_suffix(".git").unwrap_or(repo_path);
    if repo_path.trim().is_empty() {
        return Err(SourceParseError::EmptySegment);
    }

    Ok(GitRepo {
        url: format!("https://{host}/{repo_path}"),
        git_ref,
        subfolder,
    })
}
```

- [ ] **Step 5: Verify RED — crate fails to compile only at the known consumer sites**

Run: `cargo test --lib 2>&1 | grep -E "^error" | head -20`
Expected: compile errors mentioning `Gitlab` in `src/pipeline/git.rs`, `src/index.rs`, `src/main.rs` (binary compiles separately — lib errors first), `src/pipeline/install.rs`, `src/pipeline/mod.rs`. No errors in other files.

- [ ] **Step 6: Update `src/pipeline/git.rs`**

Replace the import (line 16): `use crate::source::{GitRepo, GithubRepo, InstallSource};`

Update the `install_from_source` doc comment (lines 25-27):

```rust
/// - `Github` — `https://github.com/<owner>/<repo>.git`.
/// - `Git` — `<url>.git`; the URL is stored verbatim on [`GitRepo`], so
///   any https git host (GitLab incl. self-hosted/subgroups, Bitbucket,
///   Gitea, …) works identically.
```

Replace the `Gitlab` dispatch arm (line 40), `install_from_gitlab` (lines 60-74), and `gitlab_clone_url` (lines 80-82):

```rust
InstallSource::Git(repo) => install_from_git(repo, target, filter),
```

```rust
fn install_from_git(
    repo: &GitRepo,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    let (owner, name) = repo_display_parts(&repo.url);
    install_from_git_url(
        &git_clone_url(repo),
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        owner,
        name,
        target,
        filter,
    )
}
```

```rust
fn git_clone_url(repo: &GitRepo) -> String {
    format!("{}.git", repo.url)
}

/// Split a bare remote URL into (prefix, last segment) for the
/// human-facing error messages in `fetch::resolve_subfolder`, which
/// formats them back as `{owner}/{name}` — reproducing the full URL.
fn repo_display_parts(url: &str) -> (&str, &str) {
    url.rsplit_once('/').unwrap_or(("", url))
}
```

Replace the `fetch_ssot` `Gitlab` arm (lines 106-113):

```rust
InstallSource::Git(repo) => {
    let (owner, name) = repo_display_parts(&repo.url);
    clone_to_tempdir(
        &git_clone_url(repo),
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        owner,
        name,
    )
}
```

Replace the two GitLab tests (`gitlab_clone_url_uses_repo_host`, `gitlab_clone_url_with_subgroups`, lines 174-218):

```rust
    #[test]
    fn git_clone_url_appends_dot_git() {
        let repo = GitRepo {
            url: "https://gitlab.example.com/team/rules".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            git_clone_url(&repo),
            "https://gitlab.example.com/team/rules.git"
        );
    }

    #[test]
    fn git_clone_url_preserves_deep_paths_and_ports() {
        let repo = GitRepo {
            url: "https://git.company.com:8443/partners/devex/process/seed".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            git_clone_url(&repo),
            "https://git.company.com:8443/partners/devex/process/seed.git"
        );
    }

    #[test]
    fn repo_display_parts_splits_last_segment() {
        assert_eq!(
            repo_display_parts("https://gitlab.com/team/skills"),
            ("https://gitlab.com/team", "skills")
        );
    }
```

- [ ] **Step 7: Update the remaining consumer match arms**

`src/index.rs:358-361` — replace the `Gitlab` arm of `clone_url_for`:

```rust
InstallSource::Git(repo) => Ok(format!("{}.git", repo.url)),
```

`src/main.rs:255-262` — replace the `Gitlab` arm of `print_install_progress`:

```rust
InstallSource::Git(repo) => {
    eprintln!("{} {}", style::dim("Cloning"), style::name(&repo.url));
}
```

`src/pipeline/install.rs:366` — replace the `Gitlab` arm of `source_git_ref`:

```rust
InstallSource::Git(r) => r.git_ref.clone(),
```

`src/pipeline/mod.rs:267` — replace the `Gitlab` arm in the `git_ref` match:

```rust
InstallSource::Git(r) => r.git_ref.as_deref(),
```

`src/pipeline/mod.rs:206` — in the doc comment, change "Github/Gitlab" to "Github/Git" so it reads: is pinned (Github/Git `git_ref`); local-path sources record `None`.

- [ ] **Step 8: Run the full test suite to verify GREEN**

Run: `just test`
Expected: PASS — all unit tests including the new `parse_git_url_*` / `label_roundtrip_git_*` tests, and the Step 1 ATDD test `removed_gitlab_shorthand_exits_two_with_url_hint` now passes (parse fails fast with exit 2 and the URL hint, no clone attempted).

- [ ] **Step 9: Commit**

```bash
git add src/source.rs src/pipeline/git.rs src/index.rs src/main.rs src/pipeline/install.rs src/pipeline/mod.rs tests/cli_exit_codes.rs
git commit -m "refactor(source)!: replace GitLab-specific source with generic git URL

Any https git host now works (GitLab incl. self-hosted/subgroups,
Bitbucket, Gitea, corporate git). The gitlab:/gitlab+host: shorthands
are removed; the parse error names the full-URL replacement. Pre-1.0:
old gitlab: lockfile labels hard-fail, no migration.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 2: Documentation sweep

No code. Update every user-facing mention of the `gitlab:` shorthand and GitLab-specific URL support to the generic git-URL story. Keep ADRs untouched (historical record).

**Files:**

- Modify: `AGENTS.md` (Source format section)
- Modify: `docs/commands.md:44-46`
- Modify: `docs/specification.md:78-79,271-273`
- Modify: `docs/recipes.md:35-37`
- Modify: `docs/format-spec.md:503`
- Modify: `docs/getting-started.md:77`
- Modify: `tests/pipeline_source.rs:71-74,230-232` (comments only)

- [ ] **Step 1: Update `AGENTS.md` "Source format" list**

Replace the line:

```markdown
- `gitlab:owner/repo[...]` or `https://gitlab.com/...` — GitLab
```

with:

```markdown
- `https://<host>/<path>[...]` — any https git host (GitLab incl.
  self-hosted and subgroups, Bitbucket, Gitea, …)
```

- [ ] **Step 2: Update `docs/commands.md` add examples (lines 44-46)**

Replace:

```bash
upskill add gitlab:owner/repo                       # GitLab.com
upskill add https://gitlab.example.com/owner/repo   # self-hosted GitLab
upskill add https://gitlab.example.com/group/subgroup/repo  # GitLab subgroups (any depth)
```

with:

```bash
upskill add https://gitlab.com/owner/repo           # any https git host
upskill add https://git.example.com/owner/repo      # self-hosted (GitLab, Gitea, …)
upskill add https://gitlab.com/group/subgroup/repo  # nested groups (any depth)
```

- [ ] **Step 3: Update `docs/specification.md`**

Lines 78-79, replace:

```markdown
- `gitlab:owner/repo[...]` or `https://gitlab.com/[...]` — GitLab
- `gitlab:group/subgroup[/…]/project[...]` — GitLab subgroups (any nesting
```

with:

```markdown
- `https://<host>/<path>[...]` — any other https git host: GitLab
  (including self-hosted instances and subgroups at any nesting depth),
  Bitbucket, Gitea, or a plain git server
```

Lines 270-273, replace:

```markdown
Self-hosted GitLab is supported via full URL form
(`https://gitlab.mycompany.com/team/repo`), including projects nested under
subgroups to any depth
(`https://gitlab.mycompany.com/group/subgroup/team/repo`).
```

with:

```markdown
Any https host is treated as a git remote and cloned through git's own
configuration (`https://git.mycompany.com/team/repo`), including projects
nested under groups to any depth
(`https://git.mycompany.com/group/subgroup/team/repo`).
```

- [ ] **Step 4: Update `docs/recipes.md:34-37`**

Replace:

```markdown
Self-hosted GitLab is supported via the full URL form
(`https://gitlab.mycompany.com/team/repo`), including projects nested under
subgroups to any depth
(`https://gitlab.mycompany.com/group/subgroup/team/repo`).
```

with:

```markdown
Any https git host works via the full URL form
(`https://git.mycompany.com/team/repo`), including projects nested under
groups to any depth
(`https://git.mycompany.com/group/subgroup/team/repo`).
```

- [ ] **Step 5: Update `docs/format-spec.md:503`**

In the `requires` source-DSL parenthetical, replace:

```markdown
`upskill add` source DSL — `owner/repo@ref`, https, `gitlab:`, local). Absent `source` resolves
```

with:

```markdown
`upskill add` source DSL — `owner/repo@ref`, https URLs, local paths). Absent `source` resolves
```

- [ ] **Step 6: Update `docs/getting-started.md:77`**

Replace the registry config example source:

```yaml
source: gitlab:mycompany/ai-skills
```

with:

```yaml
source: https://gitlab.com/mycompany/ai-skills
```

- [ ] **Step 7: Update stale comments in `tests/pipeline_source.rs`**

Lines 71-74: replace the comment naming `gitlab_clone_url_uses_repo_host` with the new test name (`git_clone_url_appends_dot_git`) and reword "GitLab dispatch" to "generic git-URL dispatch".

Lines 230-232: replace `` `github:` / `gitlab:` / `gitlab+host:` / `local:` prefixes `` with `` `github:` / `local:` prefixes and bare https URLs `` (the file:// caveat still holds — `file://` is not an accepted source form).

- [ ] **Step 8: Verify docs build and grep for leftovers**

Run: `rg -in "gitlab" src/ tests/ AGENTS.md docs/commands.md docs/specification.md docs/recipes.md docs/format-spec.md docs/getting-started.md`
Expected: matches only in (a) test/doc example hostnames like `gitlab.com`/`gitlabee.dt.renault.com` used as opaque hosts, (b) the `GitlabShorthandRemoved` error variant, its message, and its tests. No remaining `gitlab:` shorthand documentation.

Run: `just book`
Expected: mdBook builds with no errors.

- [ ] **Step 9: Commit**

```bash
git add AGENTS.md docs/ tests/pipeline_source.rs
git commit -m "docs: describe generic git-url sources, drop gitlab shorthand

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 3: Final verification against the spec

**Files:** none (verification only)

- [ ] **Step 1: Format and full verify**

Run: `just fmt && just verify`
Expected: both exit 0, zero warnings (CI enforces `-D warnings`).

- [ ] **Step 2: Check the spec's acceptance criteria**

From `docs/wip/2026-06-10-generic-git-url-source-design.md`:

1. Deep-URL install — covered by `parse_git_url_with_ref_and_subfolder` + the unchanged `install_from_git_url` file:// integration tests in `tests/pipeline_source.rs`.
2. Host-agnostic — covered by `parse_git_url_bitbucket` + `git_clone_url_appends_dot_git`.
3. `gitlab:` exits 2 with hint — covered by `removed_gitlab_shorthand_exits_two_with_url_hint`.
4. Lockfile round-trip — covered by `label_roundtrip_git_minimal` / `label_roundtrip_git_full`.
5. No GitLab-specific code paths — covered by the Task 2 Step 8 grep.

Confirm each maps to a passing test or check; if any gap, add the missing test before proceeding.

- [ ] **Step 3: Commit any formatting fallout**

```bash
git status --short
# if dirty:
git add -A && git commit -m "chore: formatting

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

After this task, the branch is ready for the finishing flow (sdd-gardening of `docs/wip/`, then PR per `superpowers:finishing-a-development-branch`).
