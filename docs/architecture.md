# upskill — Architecture

> Implementation guide for the upskill CLI.

## 1. Stack

### 1.1 Language and edition

Rust 2021 edition. MSRV: 1.75 (for `async fn` in traits, though we don't use
async — keeps compat with stable distro toolchains).

### 1.2 Dependencies

| Crate        | Version | Purpose                        | Justification                                     |
| ------------ | ------- | ------------------------------ | ------------------------------------------------- |
| `clap`       | 4.x     | CLI parsing                    | Derive API, subcommands, auto `--help`.           |
| `serde`      | 1.x     | Serialization framework        | Frontmatter + lockfile deserialization.           |
| `serde_yaml` | 0.9.x   | YAML parser                    | SKILL.md frontmatter.                             |
| `serde_json` | 1.x     | JSON serialization             | Lockfile read/write.                              |
| `toml`       | 0.8.x   | TOML parser                    | `registries.toml` config.                         |
| `ureq`       | 2.x     | Blocking HTTP client           | Tarball download, GitHub API. No async.           |
| `flate2`     | 1.x     | Gzip decompression             | Tarball stream decompression.                     |
| `tar`        | 0.4.x   | Tarball extraction             | Tarball stream extraction.                        |
| `walkdir`    | 2.x     | Recursive directory traversal  | Skill discovery.                                  |
| `dialoguer`  | 0.11.x  | Interactive prompts            | Multi-select, confirm, fuzzy-select.              |
| `console`    | 0.15.x  | Terminal detection and styling | Colors, bold, `NO_COLOR`, TTY check.              |
| `indicatif`  | 0.17.x  | Progress indicators            | Spinners during fetch.                            |
| `dirs`       | 5.x     | Platform home directory        | `~/.agents/skills/`, `~/.upskill/`.               |
| `tempfile`   | 3.x     | Temporary directories          | Tarball extraction target.                        |
| `anyhow`     | 1.x     | Error handling                 | Ergonomic error chains with context.              |
| `sha2`       | 0.10.x  | SHA-256 hashing                | Lockfile content hash for modification detection. |

**Not included:**

| Crate      | Why not                                                       |
| ---------- | ------------------------------------------------------------- |
| `tokio`    | No async needed. All I/O is sequential: one fetch, one scan.  |
| `reqwest`  | Pulls in tokio. `ureq` is blocking and sufficient.            |
| `git2`     | Libgit2 binding is heavy (~5 MB). Shell out to `git` instead. |
| `octocrab` | GitHub API client is overkill. Raw `ureq` + JSON is enough.   |

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

### 1.4 Binary name

```toml
[[bin]]
name = "upskill"
path = "src/main.rs"
```

Single binary, no lib crate. The public API is the CLI — no Rust library
consumers.

## 2. Module structure

```
src/
├── main.rs          CLI entry point, clap derive, command dispatch
├── source.rs        Source URL parsing and classification
├── fetch.rs         Tarball download, git fallback, local path resolution
├── skill.rs         SKILL.md discovery, frontmatter parsing, validation
├── agent.rs         Agent detection, directory conventions, symlink targets
├── install.rs       Copy, symlink, remove, list operations
├── lockfile.rs      Lock file read/write/diff
├── search.rs        Registry search, caching, index queries
├── registry.rs      Registry config loading (built-in + registries.toml)
├── ui.rs            Interactive prompts, progress, colored output, TTY detection
└── auth.rs          Token resolution (env vars, gh/glab CLI fallback)
```

### 2.1 Dependency graph (modules)

```
main.rs
  ├── source.rs          (pure, no I/O)
  ├── auth.rs            (reads env, shells out to gh/glab)
  ├── fetch.rs           (uses: source, auth, ui)
  ├── skill.rs           (uses: walkdir, serde_yaml — pure scan)
  ├── agent.rs           (uses: dirs — filesystem checks)
  ├── install.rs         (uses: skill, agent, lockfile, ui)
  ├── search.rs          (uses: registry, fetch, skill, ui)
  ├── registry.rs        (uses: toml, dirs — config loading)
  ├── lockfile.rs        (uses: serde_json, sha2 — file I/O)
  └── ui.rs              (uses: dialoguer, console, indicatif)
```

Rule: `ui.rs` is the only module that writes to stdout/stderr. All other modules
return data structures; `main.rs` and `ui.rs` handle presentation.

## 3. Data structures

### 3.1 Core types

```rust
/// Parsed source specifier from CLI argument.
pub enum Source {
    GitHub {
        owner: String,
        repo: String,
        subpath: Option<String>,    // owner/repo:subpath
        git_ref: Option<String>,    // owner/repo@ref
    },
    GitLab {
        host: String,               // "gitlab.com" or custom
        owner: String,
        repo: String,
        subpath: Option<String>,
        git_ref: Option<String>,
    },
    Local {
        path: PathBuf,
    },
    /// Named registry from registries.toml
    Registry {
        name: String,
    },
}

/// A discovered skill — metadata + location on disk.
pub struct Skill {
    pub meta: SkillMeta,
    pub dir: PathBuf,           // directory containing SKILL.md
    pub rel_path: String,       // relative to source root (for display)
}

/// Parsed SKILL.md YAML frontmatter.
#[derive(Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub compatibility: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default, rename = "allowed-tools")]
    pub allowed_tools: Option<String>,
}

/// Known coding agent with directory conventions.
pub struct AgentDef {
    pub id: &'static str,          // "claude-code"
    pub flag_name: &'static str,   // "claude" (CLI flag)
    pub project_dir: &'static str, // ".claude/skills"
    pub global_dir: &'static str,  // ".claude/skills" (under $HOME)
}

/// An agent resolved to a concrete target path.
pub struct AgentTarget {
    pub def: &'static AgentDef,
    pub path: PathBuf,             // absolute path to skills dir
    pub scope: Scope,
}

pub enum Scope { Project, Global }

/// Lock file entry.
#[derive(Serialize, Deserialize)]
pub struct LockEntry {
    pub name: String,
    pub source: String,            // "microsoft/skills"
    pub subpath: Option<String>,
    pub git_ref: Option<String>,   // "main", "v1.0", or SHA
    pub commit: Option<String>,    // resolved commit SHA
    pub content_hash: String,      // SHA-256 of SKILL.md content
    pub installed_at: String,      // ISO 8601
    pub agents: Vec<String>,       // ["claude-code", "copilot"]
}

/// Lock file root.
#[derive(Serialize, Deserialize)]
pub struct LockFile {
    pub version: u32,              // 1
    pub skills: Vec<LockEntry>,
}

/// Registry entry from registries.toml or built-in list.
#[derive(Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub token_env: Option<String>,
}
```

### 3.2 Static agent table

```rust
pub const AGENTS: &[AgentDef] = &[
    AgentDef { id: "claude-code", flag_name: "claude",   project_dir: ".claude/skills",   global_dir: ".claude/skills" },
    AgentDef { id: "copilot",     flag_name: "copilot",  project_dir: ".github/skills",   global_dir: ".copilot/skills" },
    AgentDef { id: "codex",       flag_name: "codex",    project_dir: ".codex/skills",    global_dir: ".codex/skills" },
    AgentDef { id: "cursor",      flag_name: "cursor",   project_dir: ".cursor/skills",   global_dir: ".cursor/skills" },
    AgentDef { id: "kiro",        flag_name: "kiro",     project_dir: ".kiro/skills",     global_dir: ".kiro/skills" },
    AgentDef { id: "windsurf",    flag_name: "windsurf", project_dir: ".windsurf/skills", global_dir: ".codeium/windsurf/skills" },
    AgentDef { id: "opencode",    flag_name: "opencode", project_dir: ".opencode/skills", global_dir: ".config/opencode/skills" },
];
```

## 4. Algorithms

### 4.1 Source parsing (`source.rs`)

Input: raw string from CLI argument.

```
parse_source(input: &str) -> Result<Source>

1. if input starts with "./", "../", "/", "~"
     → Source::Local { path: resolve(input) }

2. if input starts with "https://github.com/"
     → extract owner/repo from URL path
     → parse optional @ref and :subpath from remaining
     → Source::GitHub { ... }

3. if input starts with "https://gitlab.com/" or "https://<other-host>/"
     → if host contains "gitlab" or input starts with "gitlab:"
       → Source::GitLab { host, owner, repo, ... }
     → else error: "Unsupported host: {host}"

4. if input starts with "gitlab:"
     → strip prefix, parse owner/repo
     → Source::GitLab { host: "gitlab.com", ... }

5. if input matches pattern: {word}/{word} (with optional @ref and :subpath)
     → check if {word}/{word} is a known registry name from registries.toml
       → if yes: Source::Registry { name }
       → if no: Source::GitHub { owner, repo, ... }

6. if input is a single word
     → check if it matches a registry name
       → if yes: Source::Registry { name }
       → else error

7. else error: "Cannot parse source: {input}"
```

**Ref and subpath extraction** (step 2, 4, 5):

```
input: "owner/repo@v1.0:skills/my-tool"
         ^^^^^^^^^^  ^^^^  ^^^^^^^^^^^^^^
         owner/repo   ref   subpath

Split on "@" first (max 2 parts).
Split remainder on ":" first (max 2 parts).
```

### 4.2 Authentication (`auth.rs`)

```
resolve_github_token() -> Option<String>

1. env::var("GITHUB_TOKEN") → Some(token)
2. env::var("GH_TOKEN")     → Some(token)
3. Command::new("gh").args(["auth", "token"]).output()
   → if exit 0 and stdout non-empty → Some(stdout.trim())
4. None
```

Same pattern for GitLab with `GITLAB_TOKEN`, `GL_TOKEN`, `glab auth token`.

Cache the result per process — resolve once, reuse. Store in a lazy
`OnceCell<Option<String>>`.

### 4.3 Fetch (`fetch.rs`)

```
fetch_skills(source: &Source) -> Result<(Vec<Skill>, TempDir)>

match source:
  Local { path }      → skill::discover_skills(path)
  Registry { name }   → resolve name to Source via registry.rs, recurse
  GitHub { .. }       → fetch_github(owner, repo, ref, subpath)
  GitLab { .. }       → fetch_gitlab(host, owner, repo, ref, subpath)
```

**`fetch_github` algorithm:**

```
fetch_github(owner, repo, git_ref, subpath) -> Result<(Vec<Skill>, TempDir)>

1. token = auth::resolve_github_token()

2. Build tarball URL:
   if git_ref is Some(ref):
     if ref looks like a SHA (40 hex chars):
       url = "https://github.com/{owner}/{repo}/archive/{ref}.tar.gz"
     else:
       url = "https://github.com/{owner}/{repo}/archive/refs/heads/{ref}.tar.gz"
       (on 404, try refs/tags/{ref}.tar.gz)
   else:
     url = "https://github.com/{owner}/{repo}/archive/refs/heads/main.tar.gz"
     (on 404, try refs/heads/master.tar.gz)

3. request = ureq::get(url)
   if token is Some(t):
     request = request.set("Authorization", format!("Bearer {t}"))

4. response = request.call()
   match response:
     Ok(resp) → stream_extract(resp)
     Err(404) → try_git_fallback(owner, repo, git_ref, token)
     Err(401|403) → error with auth guidance
     Err(429) → error with retry-after
     Err(e) → try_git_fallback as last resort

5. stream_extract(resp):
   tmpdir = tempfile::tempdir()
   reader = resp.into_reader()
   decoder = flate2::read::GzDecoder::new(reader)
   archive = tar::Archive::new(decoder)
   archive.unpack(tmpdir.path())
   root = find single top-level directory in tmpdir
   search_root = if subpath: root.join(subpath) else root
   skills = skill::discover_skills(search_root)
   return (skills, tmpdir)   // tmpdir kept alive by caller
```

**Git fallback algorithm:**

```
try_git_fallback(owner, repo, git_ref, token) -> Result<(Vec<Skill>, TempDir)>

1. Check if "git" is on PATH:
   Command::new("git").arg("--version").output()
   → if fails: error "Tarball unavailable. Install git or set GITHUB_TOKEN."

2. tmpdir = tempfile::tempdir()
   url = format!("https://github.com/{owner}/{repo}.git")

3. cmd = Command::new("git")
     .args(["clone", "--depth", "1", "--filter=blob:none"])
   if git_ref is Some(ref):
     cmd.args(["--branch", ref])
   cmd.args([url, tmpdir.path().join("repo")])

4. ui::warn("Tarball not available, falling back to git clone")

5. Execute cmd
   → if fails: error with stderr

6. skills = skill::discover_skills(tmpdir.path().join("repo"))
   return (skills, tmpdir)
```

### 4.4 Skill discovery (`skill.rs`)

```
discover_skills(root: &Path) -> Result<Vec<Skill>>

1. Walk root with walkdir::WalkDir::new(root)
     .max_depth(4)
     .into_iter()
     .filter_entry(|e| !is_hidden_or_git(e))

2. For each entry where file_name == "SKILL.md":
   a. skill_dir = entry.path().parent()
   b. rel_path = skill_dir.strip_prefix(root)
   c. content = fs::read_to_string(entry.path())
   d. frontmatter = extract_frontmatter(&content)
      → find first "---\n", then next "\n---"
      → return content between them
   e. meta = serde_yaml::from_str::<SkillMeta>(frontmatter)
      → on error: ui::warn("Skipping {path}: {err}"), continue
   f. validate_meta(&meta, skill_dir):
      → name non-empty, <= 64 chars, lowercase + hyphens
      → description non-empty, <= 1024 chars
      → name matches directory name (warn if not)
      → on validation failure: warn, continue
   g. Push Skill { meta, dir, rel_path } to results

3. Sort results by meta.name

4. Return results
```

**Frontmatter extraction:**

```
extract_frontmatter(content: &str) -> Option<&str>

1. trimmed = content.trim_start()
2. if !trimmed.starts_with("---") → return None
3. after_first = &trimmed[3..]
4. Skip until newline (consume the "---\n" line)
5. end = after_first.find("\n---")
   → if None: return None
6. return Some(&after_first[..end])
```

This is intentionally simple. No Markdown parser, no edge case handling for
`---` inside code blocks. The Agent Skills spec guarantees frontmatter is at the
top of the file between the first two `---` delimiters.

### 4.5 Agent resolution (`agent.rs`)

```
resolve_agents(flags: &AgentFlags, global: bool) -> Result<Vec<AgentTarget>>

  Input: the boolean flags from clap (--claude, --copilot, --all, etc.)

  project_root = walk_up_to_git_root() or CWD
  home = dirs::home_dir()
  base = if global { home } else { project_root }

  if flags.all:
    → return all AGENTS mapped to base.join(agent.project_dir or global_dir)

  if any explicit flag is set (flags.claude, flags.copilot, etc.):
    → return only the flagged agents mapped to base.join(...)

  else (auto-detect):
    → for each agent in AGENTS:
        dir = base.join(agent.project_dir or global_dir)
        if dir.parent().exists():
          → include this agent
    → if none found: return empty vec (no symlinks, canonical only)
```

**Git root walk:**

```
find_project_root() -> Option<PathBuf>

  dir = env::current_dir()
  loop:
    if dir.join(".git").exists() → return Some(dir)
    if !dir.pop() → return None
```

### 4.6 Install (`install.rs`)

```
install_skills(
    skills: &[Skill],
    agents: &[AgentTarget],
    global: bool,
    copy_mode: bool,
) -> Result<()>

  base = if global { home } else { project_root }
  canonical_base = base.join(".agents/skills")
  fs::create_dir_all(&canonical_base)

  for skill in skills:
    // 1. Canonical copy (always)
    dest = canonical_base.join(&skill.meta.name)
    copy_dir_recursive(&skill.dir, &dest)
    ui::success(format!("{} → {}", skill.meta.name, dest.display()))

    // 2. Agent symlinks
    for agent in agents:
      agent_dest = agent.path.join(&skill.meta.name)

      // Create parent dir if agent config dir exists
      if agent.path.parent().exists() || agent.path.exists():
        fs::create_dir_all(&agent.path)
      else:
        continue   // don't create agent config dir from scratch

      if copy_mode:
        copy_dir_recursive(&skill.dir, &agent_dest)
        ui::success(format!("{} → {} (copy)", skill.meta.name, agent_dest.display()))
      else:
        // Remove existing target
        if agent_dest.exists() || agent_dest.is_symlink():
          remove(agent_dest)

        // Compute relative symlink path
        rel = relative_path_from(&agent.path, &dest)
        symlink(rel, agent_dest)
        ui::success(format!("{} → {} (symlink)", skill.meta.name, agent_dest.display()))

    // 3. Update lockfile
    lockfile::upsert(skill, source, agents)
```

**Relative symlink computation:**

```
relative_path_from(from_dir: &Path, to: &Path) -> PathBuf

  Example:
    from_dir = "/project/.claude/skills"
    to       = "/project/.agents/skills/markspec"
    result   = "../../.agents/skills/markspec"

  Algorithm:
    1. Canonicalize both paths
    2. Find common ancestor
    3. Count ".." steps from from_dir to ancestor
    4. Append remaining path from ancestor to target
```

Relative symlinks are critical — they survive project moves and work inside git
repos.

**Remove algorithm:**

```
remove_skills(names: &[String], global: bool) -> Result<()>

  base = if global { home } else { project_root }

  for name in names:
    // 1. Remove canonical copy
    canonical = base.join(".agents/skills").join(name)
    if canonical.exists():
      fs::remove_dir_all(canonical)

    // 2. Remove all agent symlinks pointing to it
    for agent in AGENTS:
      agent_path = base.join(agent.project_dir or global_dir).join(name)
      if agent_path.is_symlink():
        fs::remove_file(agent_path)   // remove symlink, not target
      else if agent_path.is_dir():
        fs::remove_dir_all(agent_path) // was a --copy install

    // 3. Remove from lockfile
    lockfile::remove(name)

    ui::success(format!("Removed {}", name))
```

### 4.7 Lockfile (`lockfile.rs`)

**File location:**

| Scope   | Path                                |
| ------- | ----------------------------------- |
| Project | `{project_root}/.upskill-lock.json` |
| Global  | `~/.upskill-lock.json`              |

**Read:**

```
load(scope: Scope) -> Result<LockFile>

  path = lockfile_path(scope)
  if !path.exists() → return LockFile { version: 1, skills: vec![] }
  content = fs::read_to_string(path)
  serde_json::from_str(&content)
```

**Write:**

```
save(lockfile: &LockFile, scope: Scope) -> Result<()>

  path = lockfile_path(scope)
  content = serde_json::to_string_pretty(lockfile)
  fs::write(path, content)
```

**Upsert (on install):**

```
upsert(skill: &Skill, source: &Source, agents: &[AgentTarget])

  1. load lockfile
  2. Find existing entry by name → update, or push new
  3. Set fields: source, subpath, git_ref, commit (from fetch),
     content_hash (sha256 of SKILL.md), installed_at (now), agents
  4. save lockfile
```

**Content hash** (for modification detection):

```
content_hash(skill_dir: &Path) -> String

  Walk all files in skill_dir, sorted by relative path.
  For each file: feed relative_path + file_content into SHA-256 hasher.
  Return hex digest.
```

This detects any file change inside the skill dir, not just SKILL.md.

### 4.8 Search (`search.rs`)

```
search(query: &str, local_only: bool) -> Result<Vec<SearchResult>>

  results = vec![]

  // 1. Local search (always)
  for skill in install::list_installed():
    if matches(query, &skill.meta):
      results.push(SearchResult { source: "local", skill })

  if local_only:
    return results

  // 2. Registry search
  registries = registry::load_all()   // built-in + custom
  for reg in registries:
    index = fetch_or_cache_index(&reg)
    for skill_meta in index:
      if matches(query, &skill_meta):
        results.push(SearchResult { source: reg.name, skill_meta })

  // 3. Deduplicate by skill name (first match wins)
  dedup(&mut results)

  return results
```

**Index caching:**

```
fetch_or_cache_index(registry: &RegistryEntry) -> Result<Vec<SkillMeta>>

  cache_dir = dirs::cache_dir().join("upskill/registries")
  cache_file = cache_dir.join(format!("{}.json", registry.name))

  if cache_file.exists() && age(cache_file) < Duration::from_secs(3600):
    return load from cache

  // Fetch fresh
  source = source::parse(&registry.source)
  (skills, _tmpdir) = fetch::fetch_skills(&source)
  index = skills.iter().map(|s| s.meta.clone()).collect()

  // Write cache
  fs::create_dir_all(cache_dir)
  fs::write(cache_file, serde_json::to_string(&index))

  return index
```

Cache TTL: 1 hour. Cache is per-registry, stored as JSON array of `SkillMeta`.
On `upskill search`, stale caches are refreshed transparently.

**Match function:**

```
matches(query: &str, meta: &SkillMeta) -> bool

  let q = query.to_lowercase()
  meta.name.to_lowercase().contains(&q)
    || meta.description.to_lowercase().contains(&q)
```

Simple substring match. No fuzzy scoring, no stemming. Good enough for a CLI
tool.

### 4.9 Registry config (`registry.rs`)

```
load_all() -> Vec<RegistryEntry>

  // 1. Built-in well-known registries
  let mut regs = vec![
    RegistryEntry { name: "anthropic",       source: "anthropics/skills",      .. },
    RegistryEntry { name: "microsoft",       source: "microsoft/skills",       .. },
    RegistryEntry { name: "awesome-copilot", source: "github/awesome-copilot", .. },
  ];

  // 2. Global config
  if let Some(home) = dirs::home_dir():
    load_toml(home.join(".upskill/registries.toml"), &mut regs)

  // 3. Project config (overrides global by name)
  load_toml(project_root.join(".upskill/registries.toml"), &mut regs)

  return regs
```

**Override semantics:** if a custom registry has the same `name` as a built-in,
the custom one replaces it. This lets teams override `anthropic` with an
internal mirror.

## 5. CLI structure (`main.rs`)

```rust
#[derive(Parser)]
#[command(name = "upskill", version, about = "Upskill your coding agents.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Install skills from a GitHub/GitLab repo or local path
    #[command(alias = "i")]
    Add {
        source: String,

        #[arg(short, long)]
        skill: Vec<String>,

        #[arg(long)]
        all: bool,

        #[arg(short, long)]
        list: bool,

        #[arg(short, long)]
        global: bool,

        #[arg(short, long)]
        yes: bool,

        #[arg(long)]
        copy: bool,

        // Agent flags
        #[arg(long)]
        claude: bool,
        #[arg(long)]
        copilot: bool,
        #[arg(long)]
        codex: bool,
        #[arg(long)]
        cursor: bool,
        #[arg(long)]
        kiro: bool,
        #[arg(long)]
        windsurf: bool,
        #[arg(long)]
        opencode: bool,
        #[arg(long, name = "all-agents")]
        all_agents: bool,
    },

    /// List installed skills
    #[command(alias = "ls")]
    List {
        #[arg(short, long)]
        global: bool,
    },

    /// Search for skills locally and across registries
    Search {
        query: Option<String>,
        #[arg(long)]
        local: bool,
        #[arg(long)]
        registry: Option<String>,
    },

    /// Remove installed skills
    #[command(alias = "rm")]
    Remove {
        skill: Vec<String>,
        #[arg(long)]
        all: bool,
        #[arg(short, long)]
        global: bool,
    },

    /// Check for skill updates
    Check {
        #[arg(short, long)]
        global: bool,
    },

    /// Update installed skills
    Update {
        skill: Vec<String>,
        #[arg(short, long)]
        global: bool,
        #[arg(short, long)]
        yes: bool,
        #[arg(long)]
        dry_run: bool,
    },
}
```

**Agent flags struct** (extracted in `agent.rs`):

```rust
pub struct AgentFlags {
    pub claude: bool,
    pub copilot: bool,
    pub codex: bool,
    pub cursor: bool,
    pub kiro: bool,
    pub windsurf: bool,
    pub opencode: bool,
    pub all: bool,
}
```

## 6. Command dispatch flow

### 6.1 `add` flow

```
main
  → source::parse(source_arg)
  → fetch::fetch_skills(source)
  → if --list: ui::print_skill_table(skills); return
  → select skills (--all, --skill, or ui::interactive_select)
  → agent::resolve_agents(flags, global)
  → if !--yes: ui::confirm_install(skills, agents)
  → install::install_skills(skills, agents, global, copy)
  → ui::print_summary(count, agents)
```

### 6.2 `list` flow

```
main
  → install::list_installed(global)
  → ui::print_installed_table(skills)
```

### 6.3 `search` flow

```
main
  → if query is None and TTY: ui::interactive_fuzzy_select
  → search::search(query, local_only)
  → ui::print_search_results(results)
```

### 6.4 `remove` flow

```
main
  → if names empty and TTY: ui::interactive_select from installed
  → install::remove_skills(names, global)
```

### 6.5 `check` flow

```
main
  → lockfile::load(scope)
  → for each entry: fetch latest commit SHA from source
  → compare with lockfile commit
  → ui::print_check_results(results)
```

### 6.6 `update` flow

```
main
  → run check internally
  → filter to outdated skills (or named skills)
  → for each: detect local modifications via content_hash
  → if modified and !--yes: ui::confirm_overwrite
  → if --dry-run: ui::print_dry_run; return
  → for each: re-fetch, replace canonical, update lockfile
  → ui::print_update_summary
```

## 7. Error handling

### 7.1 Strategy

All functions return `anyhow::Result<T>`. Errors are chained with `.context()`:

```rust
fs::read_to_string(&path)
    .context(format!("Failed to read {}", path.display()))?;
```

`main()` catches the top-level `Result` and:

- On `Ok`: exit 0
- On `Err`: print error chain to stderr, exit 1

Usage errors from `clap` exit with code 2 automatically.

### 7.2 Non-fatal vs fatal

| Situation                         | Severity | Action                             |
| --------------------------------- | -------- | ---------------------------------- |
| Invalid SKILL.md in one subdir    | Warning  | Skip, continue scanning others     |
| Tarball 404                       | Retry    | Try next branch, then git fallback |
| Git not on PATH (during fallback) | Fatal    | Exit with guidance message         |
| Network timeout                   | Fatal    | Exit with proxy suggestion         |
| Symlink creation fails            | Warning  | Print error, continue with next    |
| Lockfile parse error              | Warning  | Treat as empty, continue           |
| Registry fetch fails              | Warning  | Skip registry, continue search     |

Rule: a failure in one skill or one agent should not abort the entire operation.

### 7.3 SIGINT handling

`ctrlc` crate or raw signal handler: on SIGINT, clean up tempdir and exit 130.
The `tempfile` crate handles cleanup automatically on drop, so this is mostly
about clean output:

```
^C
  Interrupted. Partial install may remain in .agents/skills/.
```

## 8. Testing strategy

### 8.1 Unit tests

| Module        | What to test                                            |
| ------------- | ------------------------------------------------------- |
| `source.rs`   | Parsing all source formats, edge cases, error messages. |
| `skill.rs`    | Frontmatter extraction, validation, edge cases.         |
| `agent.rs`    | Flag resolution, auto-detect logic.                     |
| `lockfile.rs` | Read/write/upsert, content hash computation.            |
| `registry.rs` | TOML parsing, override semantics, built-in list.        |
| `auth.rs`     | Env var resolution order (mock env).                    |

`source.rs` and `skill.rs` are pure — no I/O, easy to test.

### 8.2 Integration tests

Use `tempfile` to create fake project structures with `.claude/`, `.github/`,
etc. and verify:

- `install_skills` creates canonical copy + correct symlinks.
- `remove_skills` cleans up everything.
- Symlinks are relative and survive directory moves.
- `--copy` creates independent copies.
- Auto-detect finds the right agents.

### 8.3 End-to-end tests

Against real repos (gated behind `--ignored` or env var):

```rust
#[test]
#[ignore]
fn e2e_install_from_anthropics_skills() {
    // upskill add anthropics/skills --skill pdf --yes
    // assert .agents/skills/pdf/SKILL.md exists
    // assert frontmatter is valid
}
```
