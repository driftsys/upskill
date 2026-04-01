# upskill — Architecture

> Implementation guide for the upskill CLI.

## 1. Stack

### 1.1 Language and edition

Rust 2024 edition. MSRV: 1.85 (first stable release to support edition 2024).

### 1.2 Dependencies

| Crate        | Version | Purpose                 | Justification                                     |
| ------------ | ------- | ----------------------- | ------------------------------------------------- |
| `clap`       | 4.x     | CLI parsing             | Derive API, subcommands, auto `--help`.           |
| `ctrlc`      | 3.x     | SIGINT handler          | Exit code 130 on Ctrl+C.                          |
| `serde`      | 1.x     | Serialization framework | Lockfile serialization.                           |
| `serde_json` | 1.x     | JSON serialization      | Lockfile read/write.                              |
| `sha2`       | 0.11.x  | SHA-256 hashing         | Lockfile content hash for modification detection. |
| `thiserror`  | 2.x     | Typed parse errors      | `SourceParseError` in `source.rs`.                |
| `anyhow`     | 1.x     | Error handling          | Ergonomic error chains with `.context()`.         |
| `ureq`       | 2.x     | HTTP client             | skills.sh API calls in `search.rs`.               |

**Not included:**

| Crate        | Why not                                                           |
| ------------ | ----------------------------------------------------------------- |
| `tokio`      | No async needed. All I/O is sequential: one fetch, one scan.      |
| `reqwest`    | Pulls in tokio. Fetch delegates to `git clone`.                   |
| `git2`       | Libgit2 binding is heavy (~5 MB). Shell out to `git` instead.     |
| `walkdir`    | Not needed — `std::fs::read_dir` is sufficient for current depth. |
| `dialoguer`  | Interactive prompts are simple enough with raw stdin.             |
| `serde_yaml` | No SKILL.md frontmatter parsing yet (v0.4+).                      |
| `toml`       | No `registries.toml` config yet (v0.4+).                          |

### 1.3 Build profile

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

Target: ~2-3 MB static binary. `panic = "abort"` saves ~200 KB by removing
unwind tables.

### 1.4 Crate layout

```toml
[lib]
name = "upskill"

[[bin]]
name = "upskill"
path = "src/main.rs"
```

Library crate (`lib.rs`) exposes all modules for integration testing. Binary
crate (`main.rs`) is the CLI entry point.

## 2. Module structure

```
src/
├── main.rs       CLI entry point, clap derive, command dispatch
├── lib.rs        Module declarations and re-exports
├── source.rs     Source URL parsing and classification
├── fetch.rs      Git clone, shallow clone, local path resolution
├── agent.rs      Agent detection, AGENT_DEFS, symlink/copy targets
├── install.rs    Canonical target, persist, skill selection
├── lockfile.rs   Lock file read/write, content hash
├── search.rs     skills.sh API search, registry URL resolution
├── ui.rs         Interactive prompts, TTY detection, colored output
└── auth.rs       Token resolution (env vars, gh/glab CLI fallback)
```

### 2.1 Dependency graph (modules)

```
main.rs
  ├── source.rs     (pure, no I/O)
  ├── auth.rs       (reads env, shells out to gh/glab)
  ├── fetch.rs      (uses: source, auth)
  ├── agent.rs      (filesystem checks, symlink operations)
  ├── install.rs    (uses: ui — canonical target, skill selection)
  ├── lockfile.rs   (uses: serde_json, sha2 — file I/O)
  ├── search.rs     (uses: ureq — HTTP GET to registry API)
  └── ui.rs         (stdin/stdout — TTY detection, prompts)
```

Rule: only `main.rs` and `ui.rs` write to stdout/stderr. All other modules
return data structures or `Result`; presentation is in `main.rs`.

## 3. Data structures

### 3.1 Source types (`source.rs`)

```rust
pub enum InstallSource {
    Github(GithubRepo),
    Gitlab(GitlabRepo),
    LocalPath(PathBuf),
}

pub struct GithubRepo {
    pub owner: String,
    pub name: String,
    pub git_ref: Option<String>,    // owner/repo@ref
    pub subfolder: Option<String>,  // owner/repo:path
}

pub struct GitlabRepo {
    pub host: String,               // "gitlab.com" or custom
    pub owner: String,
    pub name: String,
    pub git_ref: Option<String>,
    pub subfolder: Option<String>,
}

#[derive(Debug, Error)]
pub enum SourceParseError { ... }
```

### 3.2 Agent table (`agent.rs`)

```rust
struct AgentDef {
    name: &'static str,       // "claude"
    skill_link: &'static str, // ".claude/skills"
}

const AGENT_DEFS: [AgentDef; 7] = [
    AgentDef { name: "claude",    skill_link: ".claude/skills"    },
    AgentDef { name: "copilot",   skill_link: ".github/skills"    },
    AgentDef { name: "codex",     skill_link: ".codex/skills"     },
    AgentDef { name: "cursor",    skill_link: ".cursor/skills"    },
    AgentDef { name: "kiro",      skill_link: ".kiro/skills"      },
    AgentDef { name: "windsurf",  skill_link: ".windsurf/skills"  },
    AgentDef { name: "opencode",  skill_link: ".opencode/skills"  },
];
```

### 3.3 Lockfile types (`lockfile.rs`)

```rust
#[derive(Serialize, Deserialize)]
pub struct Lockfile {
    pub skills: Vec<LockedSkill>,
}

#[derive(Serialize, Deserialize)]
pub struct LockedSkill {
    pub name: String,
    pub source: String,             // "github:owner/repo@ref"
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    pub hash: Option<String>,       // SHA-256 of all files in skill dir
}
```

**File locations:**

| Scope   | Path                       |
| ------- | -------------------------- |
| Project | `{cwd}/.upskill-lock.json` |
| Global  | `~/.upskill-lock.json`     |

## 4. Algorithms

### 4.1 Source parsing (`source.rs`)

Input: raw string from CLI argument.

```
parse_install_source(input: &str) -> Result<InstallSource, SourceParseError>

1. if input starts with "./", "../", "/", "~"
     → InstallSource::LocalPath(PathBuf::from(input))

2. if input starts with "gitlab:" or is a URL with a gitlab host
     → parse owner/repo, optional @ref and :subfolder
     → InstallSource::Gitlab(GitlabRepo { host, owner, name, git_ref, subfolder })

3. if input starts with "https://github.com/"
     → extract owner/repo from URL
     → parse optional @ref and :subfolder
     → InstallSource::Github(GithubRepo { ... })

4. if input matches owner/repo pattern (with optional @ref and :subfolder)
     → InstallSource::Github(GithubRepo { ... })

5. else: SourceParseError
```

**Ref and subfolder extraction:**

```
input: "owner/repo@v1.0:skills/my-tool"
         ^^^^^^^^^^  ^^^^  ^^^^^^^^^^^^^^
         owner/repo   ref   subfolder

Split on "@" first (max 2 parts).
Split remainder on ":" (max 2 parts).
```

### 4.2 Authentication (`auth.rs`)

```
resolve_github_token() -> GitHubAuth

1. env::var("GITHUB_TOKEN") → GitHubAuth::Token(token)
2. env::var("GH_TOKEN")     → GitHubAuth::Token(token)
3. Command::new("gh").args(["auth", "token"])
   → if exit 0 and non-empty stdout → GitHubAuth::Token(stdout.trim())
4. GitHubAuth::None
```

Same pattern for GitLab with `GITLAB_TOKEN`, `GL_TOKEN`, `glab auth token`.

### 4.3 Fetch (`fetch.rs`)

```
clone_github_repo(repo: &GithubRepo, auth: &GitHubAuth) -> Result<TempDir>

1. url = "https://github.com/{owner}/{repo}.git"
   if token: inject credentials via GIT_ASKPASS or token URL

2. shallow_clone(url, git_ref, tmpdir)
   → git clone --depth 1 [--branch ref] url tmpdir/repo

3. if subfolder: resolve_subfolder(tmpdir/repo, subfolder)

4. return tmpdir (caller keeps alive)
```

Same structure for `clone_gitlab_repo`.

### 4.4 Agent resolution (`agent.rs`)

```
ensure_agent_targets(claude, copilot, all, copy, canonical_target) -> Result<()>

if all:
  → create_agent_target for every entry in AGENT_DEFS

else if claude || copilot:
  → create_agent_target only for flagged agents

else (auto-detect):
  → for each agent in AGENT_DEFS:
      parent = agent.skill_link parent directory (e.g. ".claude")
      if parent.exists():
        → create_agent_target(agent.skill_link, copy, canonical_target)
```

`create_agent_target` either creates a symlink (Unix) or copies the directory
(`--copy` mode). Non-Unix platforms require `--copy`.

### 4.5 Install (`install.rs`)

```
canonical_target(global) -> Result<PathBuf>
  → if global: $HOME/.agents/skills
  → else: .agents/skills (relative to CWD)

persist_installed_skills(canonical_target, skills, source_label) -> Result<()>
  → for each skill:
      mkdir -p canonical_target/skill
      write canonical_target/skill/.upskill-source = source_label

resolve_requested_skills(cli_skills, default_skill) -> Result<Vec<String>>
  → if --skill flags given: return those
  → else if TTY: prompt for comma-separated selection
  → else: return [default_skill]
```

### 4.6 Lockfile (`lockfile.rs`)

**Content hash** (for modification detection):

```
hash_skill_dir(dir: &Path) -> Option<String>

Walk all files in dir, sorted by relative path.
For each file: feed (relative_path + file_content) into SHA-256.
Return hex digest.
```

**Upsert on install:**

```
lockfile.upsert(LockedSkill { name, source, git_ref, hash })
  → remove existing entry with same name
  → push new entry
  → sort by name (deterministic output)
  → save to disk
```

### 4.7 Error handling

All non-parsing functions return `anyhow::Result<T>`. Errors are chained:

```rust
std::fs::read_to_string(&path)
    .with_context(|| format!("failed to read {}", path.display()))?;
```

`source.rs` uses `thiserror` for `SourceParseError` because callers match on
specific parse failures.

`main()` translates `Result` to exit codes:

| Outcome       | Exit code |
| ------------- | --------- |
| Success       | 0         |
| General error | 1         |
| Usage error   | 2         |
| SIGINT        | 130       |

## 5. CLI structure (`main.rs`)

```rust
#[derive(Parser)]
#[command(name = "upskill", about = "Upskill your coding agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        source: String,
        #[arg(long = "skill", short = 's')] skills: Vec<String>,
        #[arg(long)] claude: bool,
        #[arg(long)] copilot: bool,
        #[arg(long)] all: bool,
        #[arg(long)] copy: bool,
        #[arg(short = 'g', long = "global")] global: bool,
    },
    List   { #[arg(short = 'g', long = "global")] global: bool },
    Remove { skill: String, #[arg(long)] yes: bool, #[arg(short = 'g')] global: bool },
    Check  { #[arg(short = 'g', long = "global")] global: bool },
    Update { names: Vec<String>, #[arg(long = "dry-run")] dry_run: bool,
             #[arg(short = 'f')] force: bool, #[arg(short = 'g')] global: bool },
}
```

Agent flags are grouped into `AddContext` inside `run_add` to keep `finish_install`
under clippy's argument limit.

## 6. Command dispatch flow

### 6.1 `add` flow

```
run_add
  → parse_install_source(source_arg)
  → resolve_requested_skills(skills, default)
  → persist_installed_skills(canonical_target, skills, source_label)
  → ensure_agent_targets(flags, canonical_target)          [if !global]
  → lockfile.upsert(skill) for each skill
  → lockfile.save()
  → print_selected_skills(skills)
```

### 6.2 `list` flow

```
run_list
  → read_dir(canonical_target)
  → for each skill dir: read .upskill-source
  → detect_active_agents()
  → print: name  source=...  symlinks=...
```

### 6.3 `check` flow

```
run_check
  → lockfile.load()
  → for each skill: print name, source, pinned ref
```

### 6.4 `update` flow

```
run_update
  → lockfile.load()
  → filter to requested names (or all)
  → if dry_run: print preview, return
  → for each skill:
      if !force: compare hash_skill_dir vs lockfile.hash
        → if differs: warn and skip
      persist_installed_skills(canonical_target, [skill], source)
```

### 6.5 `remove` flow

```
run_remove
  → confirm_removal if TTY and !--yes
  → remove_dir_all(canonical_target/skill)
  → lockfile.remove(skill)
  → cleanup_agent_symlinks_if_empty(canonical_target)
```

## 7. Testing strategy

### 7.1 Unit tests

| Module        | What to test                                            |
| ------------- | ------------------------------------------------------- |
| `source.rs`   | Parsing all source formats, edge cases, error messages. |
| `agent.rs`    | Flag resolution, auto-detect logic.                     |
| `lockfile.rs` | Read/write/upsert, content hash computation.            |
| `auth.rs`     | Env var resolution order (mock env).                    |
| `fetch.rs`    | Clone with subfolder, copy, cleanup.                    |

`source.rs` is pure — no I/O, easy to test exhaustively.

### 7.2 Integration tests

CLI integration tests live in `tests/cli_*.rs` and use `assert_cmd` +
`tempfile`. Each test creates an isolated temp directory and runs the binary:

```rust
Command::cargo_bin("upskill")
    .unwrap()
    .current_dir(&tmp)
    .args(["add", "owner/repo", "--claude"])
    .assert()
    .success();
```

Test files by area: `cli_add`, `cli_list`, `cli_remove`, `cli_check`,
`cli_update`, `cli_lockfile`, `cli_moddetect`, `cli_dryrun`, `cli_global`,
`cli_gitlab`, `cli_ci_mode`, `cli_exit_codes`, `cli_search`.

## 8. Planned (v0.4+)

Features not yet implemented but tracked in the backlog:

- **Skill discovery** (`skill.rs`) — `SKILL.md` frontmatter scanning, validation, name/description extraction.
- **Custom registries** (`registry.rs`) — `.upskill/registries.toml` for project and global registry config.
- **Named registry install** — `upskill add <registry> --skill <name>`.
- **Local search cache** — 1-hour TTL cache per registry in platform cache dir (relevant once git-based registries are added).
