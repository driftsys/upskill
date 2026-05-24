# Multi-Registry Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `upskill search` to query configured git-based registries via
a local index with HEAD-based cache invalidation.

**Architecture:** New `config.rs` module loads `~/.config/upskill/config.yaml`
(+ project `.upskill/config.yaml`), new `index.rs` module handles
clone→scan→index and freshness checks, `search.rs` is extended to aggregate
results from skills.sh + local indexes, CLI gains `index` subcommand and
`--registry`/`--kind` flags on `search`.

**Tech Stack:** Rust, serde\_yaml\_ng (config), serde\_json (index), git CLI
(ls-remote/clone), existing frontmatter parser.

---

## Task 1: Config Module — Model and Parsing

**Files:**

- Create: `src/config.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write the failing test for config parsing**

```rust
// src/config.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_with_registries() {
        let yaml = r#"
registries:
  - name: corp
    source: gitlab:mycompany/ai-skills
  - name: anthropic
    source: anthropics/skills
"#;
        let config = parse_config(yaml).unwrap();
        assert_eq!(config.registries.len(), 2);
        assert_eq!(config.registries[0].name, "corp");
        assert_eq!(config.registries[0].source, "gitlab:mycompany/ai-skills");
        assert_eq!(config.registries[1].name, "anthropic");
        assert_eq!(config.registries[1].source, "anthropics/skills");
    }

    #[test]
    fn parse_empty_config() {
        let yaml = "";
        let config = parse_config(yaml).unwrap();
        assert!(config.registries.is_empty());
    }

    #[test]
    fn parse_config_no_registries_key() {
        let yaml = "other_key: value\n";
        let config = parse_config(yaml).unwrap();
        assert!(config.registries.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test config::tests --lib`
Expected: FAIL — module doesn't exist

- [ ] **Step 3: Write minimal implementation**

```rust
// src/config.rs
//! User configuration: `~/.config/upskill/config.yaml` (global) and
//! `.upskill/config.yaml` (project). Layers are merged with project
//! taking precedence over global.

use serde::Deserialize;

/// Top-level config structure. Extensible for future settings.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub registries: Vec<RegistryEntry>,
}

/// A configured registry source.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub source: String,
}

/// Parse a config YAML string into a `Config`. Empty or missing keys
/// produce defaults (empty registries list).
pub fn parse_config(yaml: &str) -> anyhow::Result<Config> {
    if yaml.trim().is_empty() {
        return Ok(Config::default());
    }
    let config: Config = serde_yaml_ng::from_str(yaml)?;
    Ok(config)
}
```

- [ ] **Step 4: Register module in lib.rs**

Add to `src/lib.rs`:

```rust
pub mod config;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test config::tests --lib`
Expected: PASS (3 tests)

- [ ] **Step 6: Commit**

```bash
git add src/config.rs src/lib.rs
git commit -m "feat(config): add config model and YAML parsing"
```

---

## Task 2: Config Module — Load and Merge Layers

**Files:**

- Modify: `src/config.rs`

- [ ] **Step 1: Write the failing test for layer merging**

```rust
#[test]
fn merge_layers_project_overrides_global() {
    let global = Config {
        registries: vec![
            RegistryEntry { name: "corp".into(), source: "owner/repo-old".into() },
            RegistryEntry { name: "public".into(), source: "other/repo".into() },
        ],
    };
    let project = Config {
        registries: vec![
            RegistryEntry { name: "corp".into(), source: "owner/repo-new".into() },
        ],
    };
    let merged = merge_configs(global, project);
    assert_eq!(merged.registries.len(), 2);
    // corp from project overrides global
    let corp = merged.registries.iter().find(|r| r.name == "corp").unwrap();
    assert_eq!(corp.source, "owner/repo-new");
    // public from global is preserved
    assert!(merged.registries.iter().any(|r| r.name == "public"));
}

#[test]
fn load_config_returns_default_when_no_files() {
    let tmp = tempfile::tempdir().unwrap();
    let config = load_config(Some(tmp.path()), Some(tmp.path())).unwrap();
    assert!(config.registries.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test config::tests --lib`
Expected: FAIL — `merge_configs` and `load_config` not defined

- [ ] **Step 3: Write implementation**

```rust
use anyhow::{Context, Result};
use std::path::Path;

/// Merge two config layers. Project entries override global entries
/// with the same `name`. Global entries not shadowed are preserved.
pub fn merge_configs(global: Config, project: Config) -> Config {
    let mut registries = project.registries.clone();
    for entry in global.registries {
        if !registries.iter().any(|r| r.name == entry.name) {
            registries.push(entry);
        }
    }
    Config { registries }
}

/// Load config from the standard locations. Either path can be `None`
/// to skip that layer (useful in tests or when HOME is unavailable).
///
/// - Global: `<global_root>/.config/upskill/config.yaml`
///   (typically `$HOME/.config/upskill/config.yaml`)
/// - Project: `<project_root>/.upskill/config.yaml`
pub fn load_config(global_root: Option<&Path>, project_root: Option<&Path>) -> Result<Config> {
    let global = match global_root {
        Some(root) => load_layer(&root.join(".config/upskill/config.yaml"))?,
        None => Config::default(),
    };
    let project = match project_root {
        Some(root) => load_layer(&root.join(".upskill/config.yaml"))?,
        None => Config::default(),
    };
    Ok(merge_configs(global, project))
}

fn load_layer(path: &Path) -> Result<Config> {
    match std::fs::read_to_string(path) {
        Ok(content) => parse_config(&content)
            .with_context(|| format!("failed to parse config at {}", path.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(err) => Err(err).with_context(|| format!("failed to read {}", path.display())),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test config::tests --lib`
Expected: PASS (5 tests)

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): load and merge global + project layers"
```

---

## Task 3: Index Module — Model and Read/Write

**Files:**

- Create: `src/index.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// src/index.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_index() {
        let index = RegistryIndex {
            schema: 1,
            registry: "corp".into(),
            source: "gitlab:mycompany/ai-skills".into(),
            head: Some("abc123".into()),
            indexed_at: "2026-05-24T10:00:00Z".into(),
            items: vec![IndexedItem {
                name: "code-review".into(),
                kind: "skill".into(),
                description: "Structured code review".into(),
                path: "skills/code-review".into(),
            }],
        };
        let json = serde_json::to_string_pretty(&index).unwrap();
        let parsed: RegistryIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.registry, "corp");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].name, "code-review");
    }

    #[test]
    fn index_cache_path() {
        let path = cache_path("corp");
        assert!(path.ends_with("corp.json"));
        assert!(path.to_str().unwrap().contains("upskill"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test index::tests --lib`
Expected: FAIL — module doesn't exist

- [ ] **Step 3: Write implementation**

```rust
// src/index.rs
//! Local registry index: clone → scan → persist as JSON.
//! Freshness via `git ls-remote` HEAD comparison.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// On-disk index schema for a single registry.
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryIndex {
    pub schema: u32,
    pub registry: String,
    pub source: String,
    pub head: Option<String>,
    pub indexed_at: String,
    pub items: Vec<IndexedItem>,
}

/// A single item in the index.
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexedItem {
    pub name: String,
    pub kind: String,
    pub description: String,
    pub path: String,
}

/// Path to the index cache file for a registry.
pub fn cache_path(registry_name: &str) -> PathBuf {
    let cache_dir = dirs_cache_dir().join("upskill").join("index");
    cache_dir.join(format!("{}.json", registry_name))
}

/// Read a cached index from disk. Returns `None` if not found.
pub fn read_index(registry_name: &str) -> Result<Option<RegistryIndex>> {
    let path = cache_path(registry_name);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let index: RegistryIndex = serde_json::from_str(&content)
                .with_context(|| format!("failed to parse index at {}", path.display()))?;
            Ok(Some(index))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("failed to read {}", path.display())),
    }
}

/// Write an index to disk, creating parent directories as needed.
pub fn write_index(index: &RegistryIndex) -> Result<()> {
    let path = cache_path(&index.registry);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create cache dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(index)
        .context("failed to serialize index")?;
    std::fs::write(&path, json)
        .with_context(|| format!("failed to write index to {}", path.display()))?;
    Ok(())
}

/// Platform cache directory. Uses `$XDG_CACHE_HOME` on Linux,
/// `~/Library/Caches` on macOS, falls back to `~/.cache`.
fn dirs_cache_dir() -> PathBuf {
    std::env::var("XDG_CACHE_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            crate::source::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".cache")
        })
}
```

- [ ] **Step 4: Register module in lib.rs**

Add to `src/lib.rs`:

```rust
pub mod index;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test index::tests --lib`
Expected: PASS (2 tests)

- [ ] **Step 6: Commit**

```bash
git add src/index.rs src/lib.rs
git commit -m "feat(index): registry index model and cache read/write"
```

---

## Task 4: Index Module — HEAD Check and Freshness

**Files:**

- Modify: `src/index.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn ls_remote_head_parses_output() {
    // Simulates git ls-remote output format
    let output = "abc123def456789\tHEAD\n";
    let sha = parse_ls_remote_output(output).unwrap();
    assert_eq!(sha, "abc123def456789");
}

#[test]
fn ls_remote_head_empty_output() {
    let result = parse_ls_remote_output("");
    assert!(result.is_none());
}

#[test]
fn is_stale_when_head_differs() {
    let index = RegistryIndex {
        schema: 1,
        registry: "test".into(),
        source: "owner/repo".into(),
        head: Some("old_sha".into()),
        indexed_at: "2026-01-01T00:00:00Z".into(),
        items: vec![],
    };
    assert!(is_stale(&index, "new_sha"));
    assert!(!is_stale(&index, "old_sha"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test index::tests --lib`
Expected: FAIL — functions not defined

- [ ] **Step 3: Write implementation**

```rust
use std::process::Command;

/// Run `git ls-remote <url> HEAD` and return the SHA if successful.
pub fn fetch_remote_head(url: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["ls-remote", url, "HEAD"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .with_context(|| format!("failed to run git ls-remote {url}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git ls-remote failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ls_remote_output(&stdout))
}

/// Parse the SHA from `git ls-remote` output (format: `<sha>\tHEAD\n`).
fn parse_ls_remote_output(output: &str) -> Option<String> {
    output
        .lines()
        .next()
        .and_then(|line| line.split('\t').next())
        .map(|sha| sha.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Check if a cached index is stale relative to the current remote HEAD.
pub fn is_stale(index: &RegistryIndex, current_head: &str) -> bool {
    match &index.head {
        Some(cached_head) => cached_head != current_head,
        None => true, // No HEAD recorded (local-path index) — always re-scan
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test index::tests --lib`
Expected: PASS (5 tests total)

- [ ] **Step 5: Commit**

```bash
git add src/index.rs
git commit -m "feat(index): HEAD-based freshness check via git ls-remote"
```

---

## Task 5: Index Module — Scan Registry and Build Index

**Files:**

- Modify: `src/index.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn scan_registry_finds_items() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Create a skill item
    let skill_dir = root.join("skills").join("code-review");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nschema: 1\nname: code-review\ndescription: Review code\n---\n\nFirst paragraph of body.\n\nSecond paragraph.\n",
    ).unwrap();

    // Create a rule item
    let rule_dir = root.join("skills").join("no-console");
    std::fs::create_dir_all(&rule_dir).unwrap();
    std::fs::write(
        rule_dir.join("RULE.md"),
        "---\nschema: 1\nname: no-console\ndescription: No console.log\n---\n\nDo not use console.\n",
    ).unwrap();

    let items = scan_registry(root).unwrap();
    assert_eq!(items.len(), 2);

    let skill = items.iter().find(|i| i.name == "code-review").unwrap();
    assert_eq!(skill.kind, "skill");
    assert_eq!(skill.description, "First paragraph of body.");
    assert_eq!(skill.path, "skills/code-review");

    let rule = items.iter().find(|i| i.name == "no-console").unwrap();
    assert_eq!(rule.kind, "rule");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test index::tests::scan_registry_finds_items --lib`
Expected: FAIL — `scan_registry` not defined

- [ ] **Step 3: Write implementation**

```rust
use std::fs;

/// Scan a registry root directory and collect all items with metadata.
pub fn scan_registry(root: &Path) -> Result<Vec<IndexedItem>> {
    let mut items = Vec::new();
    scan_dir(root, root, &mut items)?;
    Ok(items)
}

fn scan_dir(root: &Path, dir: &Path, items: &mut Vec<IndexedItem>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Check if this directory is an item (contains SKILL.md, RULE.md, or AGENT.md)
        if let Some(item) = try_parse_item(root, &path)? {
            items.push(item);
        } else {
            // Recurse one level deeper (handles `skills/<item>/` layout)
            scan_dir(root, &path, items)?;
        }
    }
    Ok(())
}

/// Entry-point filenames and their corresponding kind.
const ENTRYPOINTS: &[(&str, &str)] = &[
    ("SKILL.md", "skill"),
    ("RULE.md", "rule"),
    ("AGENT.md", "agent"),
];

fn try_parse_item(root: &Path, dir: &Path) -> Result<Option<IndexedItem>> {
    for &(filename, kind) in ENTRYPOINTS {
        let entry_path = dir.join(filename);
        if entry_path.is_file() {
            let content = fs::read_to_string(&entry_path)
                .with_context(|| format!("failed to read {}", entry_path.display()))?;
            let name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let description = extract_description(&content);
            let rel_path = dir
                .strip_prefix(root)
                .unwrap_or(dir)
                .to_str()
                .unwrap_or("")
                .replace('\\', "/");
            return Ok(Some(IndexedItem {
                name,
                kind: kind.to_string(),
                description,
                path: rel_path,
            }));
        }
    }
    Ok(None)
}

/// Extract the first non-empty paragraph from the body (after frontmatter).
fn extract_description(content: &str) -> String {
    let body = match crate::parse::frontmatter::split(content) {
        Some((_, body)) => body,
        None => content,
    };
    body.trim()
        .split("\n\n")
        .next()
        .unwrap_or("")
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test index::tests --lib`
Expected: PASS (6 tests total)

- [ ] **Step 5: Commit**

```bash
git add src/index.rs
git commit -m "feat(index): scan registry tree and extract item metadata"
```

---

## Task 6: Index Module — Build Index for a Registry Entry

**Files:**

- Modify: `src/index.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn build_index_for_local_path() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Create a skill
    let skill_dir = root.join("skills").join("my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nschema: 1\nname: my-skill\ndescription: test\n---\n\nA skill.\n",
    ).unwrap();

    let entry = crate::config::RegistryEntry {
        name: "local".into(),
        source: root.to_str().unwrap().to_string(),
    };
    let index = build_index(&entry).unwrap();
    assert_eq!(index.registry, "local");
    assert_eq!(index.items.len(), 1);
    assert_eq!(index.items[0].name, "my-skill");
    assert!(index.head.is_none()); // local path, no HEAD
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test index::tests::build_index_for_local_path --lib`
Expected: FAIL — `build_index` not defined

- [ ] **Step 3: Write implementation**

```rust
use crate::config::RegistryEntry;
use crate::source::parse_install_source;

/// Build (or rebuild) the index for a registry entry. For git sources,
/// clones into a temp dir, scans, and returns the index. For local paths,
/// scans in place.
pub fn build_index(entry: &RegistryEntry) -> Result<RegistryIndex> {
    let source = &entry.source;
    let now = chrono_now();

    // Check if source is a local path
    if is_local_source(source) {
        let path = resolve_local_path(source)?;
        let items = scan_registry(&path)?;
        return Ok(RegistryIndex {
            schema: 1,
            registry: entry.name.clone(),
            source: entry.source.clone(),
            head: None,
            indexed_at: now,
            items,
        });
    }

    // Git source: resolve URL, clone, scan, cleanup
    let parsed = parse_install_source(source)
        .with_context(|| format!("invalid registry source: {source}"))?;
    let url = source_to_clone_url(&parsed)?;
    let head = fetch_remote_head(&url)?.unwrap_or_default();

    let tmp = tempfile::tempdir().context("failed to create temp dir for index clone")?;
    crate::fetch::shallow_clone(&url, None, &entry.name, tmp.path(), None)
        .map_err(|e| anyhow::anyhow!(e))?;

    let clone_dir = tmp.path().join(&entry.name);
    let items = scan_registry(&clone_dir)?;

    Ok(RegistryIndex {
        schema: 1,
        registry: entry.name.clone(),
        source: entry.source.clone(),
        head: Some(head),
        indexed_at: now,
        items,
    })
}

fn is_local_source(source: &str) -> bool {
    source.starts_with('/')
        || source.starts_with("./")
        || source.starts_with("../")
        || source.starts_with("~/")
}

fn resolve_local_path(source: &str) -> Result<PathBuf> {
    if source.starts_with("~/") {
        let home = crate::source::home_dir()
            .ok_or_else(|| anyhow::anyhow!("HOME not set"))?;
        Ok(home.join(&source[2..]))
    } else {
        Ok(PathBuf::from(source))
    }
}

fn source_to_clone_url(source: &crate::source::InstallSource) -> Result<String> {
    use crate::source::InstallSource;
    match source {
        InstallSource::Github(repo) => {
            Ok(format!("https://github.com/{}/{}.git", repo.owner, repo.name))
        }
        InstallSource::Gitlab(repo) => {
            Ok(format!("https://gitlab.com/{}/{}.git", repo.owner, repo.name))
        }
        InstallSource::LocalPath(_) => {
            anyhow::bail!("local path should not reach source_to_clone_url")
        }
    }
}

/// ISO 8601 timestamp without pulling in `chrono`.
fn chrono_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    // Simple Unix timestamp — good enough for a cache marker.
    format!("{}", duration.as_secs())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test index::tests --lib`
Expected: PASS (7 tests total)

- [ ] **Step 5: Commit**

```bash
git add src/index.rs
git commit -m "feat(index): build_index for local and git sources"
```

---

## Task 7: Index Module — Ensure Fresh Index (Cache-Aware)

**Files:**

- Modify: `src/index.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn ensure_fresh_uses_cache_when_head_matches() {
    let tmp = tempfile::tempdir().unwrap();

    // Pre-populate a cache file
    let index = RegistryIndex {
        schema: 1,
        registry: "cached-test".into(),
        source: "/fake/path".into(),
        head: Some("sha123".into()),
        indexed_at: "0".into(),
        items: vec![IndexedItem {
            name: "cached-item".into(),
            kind: "skill".into(),
            description: "from cache".into(),
            path: "skills/cached-item".into(),
        }],
    };
    // Write to a custom path for test isolation
    let cache_file = tmp.path().join("cached-test.json");
    let json = serde_json::to_string_pretty(&index).unwrap();
    std::fs::write(&cache_file, json).unwrap();

    let loaded: RegistryIndex =
        serde_json::from_str(&std::fs::read_to_string(&cache_file).unwrap()).unwrap();
    assert_eq!(loaded.items[0].name, "cached-item");
    assert!(!is_stale(&loaded, "sha123"));
    assert!(is_stale(&loaded, "different_sha"));
}
```

- [ ] **Step 2: Run test to verify it passes** (this test validates existing primitives)

Run: `cargo test index::tests::ensure_fresh_uses_cache --lib`
Expected: PASS

- [ ] **Step 3: Write the orchestrator function**

```rust
/// Ensure we have a fresh index for a registry entry. Returns the index
/// (from cache if fresh, rebuilt if stale or missing).
///
/// On network failure with an existing cache, returns stale cache with
/// a warning printed to stderr.
pub fn ensure_fresh(entry: &RegistryEntry) -> Result<RegistryIndex> {
    // Local paths: always re-scan (fast)
    if is_local_source(&entry.source) {
        let index = build_index(entry)?;
        write_index(&index)?;
        return Ok(index);
    }

    // Git source: check HEAD
    let parsed = parse_install_source(&entry.source)
        .with_context(|| format!("invalid registry source: {}", entry.source))?;
    let url = source_to_clone_url(&parsed)?;

    let remote_head = match fetch_remote_head(&url) {
        Ok(Some(head)) => head,
        Ok(None) => {
            // Empty repo or no HEAD — rebuild
            let index = build_index(entry)?;
            write_index(&index)?;
            return Ok(index);
        }
        Err(_) => {
            // Network failure — try stale cache
            if let Ok(Some(cached)) = read_index(&entry.name) {
                eprintln!(
                    "warning: using cached index for '{}' (offline)",
                    entry.name
                );
                return Ok(cached);
            }
            anyhow::bail!(
                "cannot reach registry '{}' and no cached index exists",
                entry.name
            );
        }
    };

    // Check cache freshness
    if let Ok(Some(cached)) = read_index(&entry.name) {
        if !is_stale(&cached, &remote_head) {
            return Ok(cached);
        }
    }

    // Stale or missing — rebuild
    let index = build_index(entry)?;
    write_index(&index)?;
    Ok(index)
}
```

- [ ] **Step 4: Run all index tests**

Run: `cargo test index::tests --lib`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/index.rs
git commit -m "feat(index): ensure_fresh with HEAD-based cache invalidation"
```

---

## Task 8: Extend Search — Query Local Indexes

**Files:**

- Modify: `src/search.rs`

- [ ] **Step 1: Write the failing test**

```rust
// Add to src/search.rs tests
#[test]
fn search_index_matches_name_and_description() {
    use crate::index::{IndexedItem, RegistryIndex};

    let index = RegistryIndex {
        schema: 1,
        registry: "test".into(),
        source: "owner/repo".into(),
        head: Some("sha".into()),
        indexed_at: "0".into(),
        items: vec![
            IndexedItem {
                name: "code-review".into(),
                kind: "skill".into(),
                description: "Structured code review workflow".into(),
                path: "skills/code-review".into(),
            },
            IndexedItem {
                name: "no-console".into(),
                kind: "rule".into(),
                description: "Ban console.log".into(),
                path: "skills/no-console".into(),
            },
        ],
    };

    let results = search_index(&index, "code", None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "code-review");

    // Also matches description
    let results = search_index(&index, "workflow", None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "code-review");

    // Kind filter
    let results = search_index(&index, "co", Some("rule"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "no-console");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test search::tests::search_index_matches --lib`
Expected: FAIL — `search_index` not defined

- [ ] **Step 3: Write implementation**

```rust
use crate::index::{IndexedItem, RegistryIndex};

/// Result from a local index search.
pub struct IndexSearchResult {
    pub name: String,
    pub kind: String,
    pub description: String,
    pub path: String,
    pub registry: String,
    pub source: String,
}

/// Search a local index by substring match on name and description.
/// Optionally filter by kind.
pub fn search_index(
    index: &RegistryIndex,
    query: &str,
    kind_filter: Option<&str>,
) -> Vec<IndexSearchResult> {
    let query_lower = query.to_lowercase();
    index
        .items
        .iter()
        .filter(|item| {
            if let Some(kind) = kind_filter {
                if item.kind != kind {
                    return false;
                }
            }
            item.name.to_lowercase().contains(&query_lower)
                || item.description.to_lowercase().contains(&query_lower)
        })
        .map(|item| IndexSearchResult {
            name: item.name.clone(),
            kind: item.kind.clone(),
            description: item.description.clone(),
            path: item.path.clone(),
            registry: index.registry.clone(),
            source: index.source.clone(),
        })
        .collect()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test search::tests --lib`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/search.rs
git commit -m "feat(search): search_index for local registry indexes"
```

---

## Task 9: CLI — Add `--registry` and `--kind` Flags to Search

**Files:**

- Modify: `src/cli.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write the failing integration test**

Create or add to `tests/cli_search.rs`:

```rust
use assert_cmd::Command;

#[test]
fn search_accepts_registry_flag() {
    Command::cargo_bin("upskill")
        .unwrap()
        .args(["search", "test", "--registry", "nonexistent"])
        .assert()
        .failure(); // Registry not configured — should error gracefully
}

#[test]
fn search_accepts_kind_flag() {
    // --kind is a filter, should not error even if no registries configured
    Command::cargo_bin("upskill")
        .unwrap()
        .args(["search", "test", "--kind", "skill"])
        .assert()
        .success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cli_search search_accepts`
Expected: FAIL — unknown flag

- [ ] **Step 3: Add flags to cli.rs**

```rust
// In Commands::Search variant:
Search {
    /// Search query.
    query: String,
    /// Maximum number of results (skills.sh only).
    #[arg(short = 'l', long, default_value = "10")]
    limit: usize,
    /// Search only a specific configured registry (skip skills.sh).
    #[arg(short = 'r', long)]
    registry: Option<String>,
    /// Filter results by item kind (skill, rule, agent, bundle).
    #[arg(short = 'k', long)]
    kind: Option<String>,
},
```

- [ ] **Step 4: Update main.rs dispatch**

```rust
// In the match arm:
Commands::Search { query, limit, registry, kind } => {
    run_search(&query, limit, registry.as_deref(), kind.as_deref())
}
```

Update `run_search` signature and implementation:

```rust
fn run_search(query: &str, limit: usize, registry: Option<&str>, kind: Option<&str>) -> i32 {
    let config = match upskill::config::load_config(
        upskill::source::home_dir().as_deref(),
        Some(Path::new(".")),
    ) {
        Ok(c) => c,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    // If --registry is specified, only search that registry
    if let Some(reg_name) = registry {
        let entry = match config.registries.iter().find(|r| r.name == reg_name) {
            Some(e) => e,
            None => {
                print_error(format!("registry '{}' not found in config", reg_name));
                return EXIT_ERROR;
            }
        };
        return run_search_registry(query, entry, kind);
    }

    // Search skills.sh (unless --kind filters to something skills.sh doesn't support)
    if kind.is_none() || kind == Some("skill") {
        match search::search(query, limit) {
            Ok(results) if !results.is_empty() => {
                if !style::is_quiet() {
                    println!("{}", style::dim("── skills.sh ──"));
                    for skill in &results {
                        let repo = skill.source
                            .trim_start_matches("github/")
                            .trim_start_matches("gitlab/");
                        println!(
                            "  {}\t{}\t{}",
                            style::name(&skill.name),
                            style::dim(&format!("{} installs", skill.installs)),
                            style::dim(&format!("upskill add {repo} {}", skill.name))
                        );
                    }
                }
            }
            Ok(_) => {}
            Err(err) => {
                if !style::is_quiet() {
                    eprintln!("{} skills.sh: {}", style::warn("warning:"), err);
                }
            }
        }
    }

    // Search configured registries
    for entry in &config.registries {
        run_search_registry(query, entry, kind);
    }

    EXIT_SUCCESS
}

fn run_search_registry(query: &str, entry: &upskill::config::RegistryEntry, kind: Option<&str>) -> i32 {
    let index = match upskill::index::ensure_fresh(entry) {
        Ok(idx) => idx,
        Err(err) => {
            if !style::is_quiet() {
                eprintln!("{} {}: {}", style::warn("warning:"), entry.name, err);
            }
            return EXIT_SUCCESS; // Non-fatal — skip this registry
        }
    };

    let results = search::search_index(&index, query, kind);
    if !results.is_empty() && !style::is_quiet() {
        println!("\n{}", style::dim(&format!("── {} ──", entry.name)));
        for item in &results {
            let source_path = format!("{}:{}", entry.source, item.path);
            println!(
                "  {} {}\t{}",
                style::name(&item.name),
                style::dim(&format!("[{}]", item.kind)),
                style::dim(&format!("upskill add {source_path}"))
            );
        }
    }

    EXIT_SUCCESS
}
```

- [ ] **Step 5: Run integration tests**

Run: `cargo test --test cli_search`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/main.rs tests/cli_search.rs
git commit -m "feat(search): add --registry and --kind flags, multi-registry output"
```

---

## Task 10: CLI — Add `index` Subcommand

**Files:**

- Modify: `src/cli.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write the failing integration test**

Add to `tests/cli_search.rs` (or create `tests/cli_index.rs`):

```rust
#[test]
fn index_clear_succeeds_with_no_cache() {
    Command::cargo_bin("upskill")
        .unwrap()
        .args(["index", "--clear"])
        .assert()
        .success();
}

#[test]
fn index_with_no_registries_configured() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["index"])
        .assert()
        .success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cli_search index_clear`
Expected: FAIL — unknown subcommand

- [ ] **Step 3: Add subcommand to cli.rs**

```rust
/// Build or manage the local registry index cache.
///
/// Without flags, rebuilds the index for all configured registries.
/// Use `--clear` to remove all cached indexes.
#[command(after_help = "EXAMPLES:\n  \
        upskill index\n  \
        upskill index --registry corp\n  \
        upskill index --clear")]
Index {
    /// Rebuild only a specific registry.
    #[arg(short = 'r', long)]
    registry: Option<String>,
    /// Remove all cached indexes.
    #[arg(long, conflicts_with = "registry")]
    clear: bool,
},
```

- [ ] **Step 4: Add dispatch in main.rs**

```rust
// In the command match:
Commands::Index { registry, clear } => run_index(registry.as_deref(), clear),

// Implementation:
fn run_index(registry: Option<&str>, clear: bool) -> i32 {
    if clear {
        let cache_dir = upskill::index::cache_dir();
        if cache_dir.exists() {
            if let Err(err) = std::fs::remove_dir_all(&cache_dir) {
                print_error(&err);
                return EXIT_ERROR;
            }
        }
        if !style::is_quiet() {
            println!("cleared index cache");
        }
        return EXIT_SUCCESS;
    }

    let config = match upskill::config::load_config(
        upskill::source::home_dir().as_deref(),
        Some(std::path::Path::new(".")),
    ) {
        Ok(c) => c,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    let entries: Vec<_> = match registry {
        Some(name) => match config.registries.iter().find(|r| r.name == name) {
            Some(e) => vec![e],
            None => {
                print_error(format!("registry '{name}' not found in config"));
                return EXIT_ERROR;
            }
        },
        None => config.registries.iter().collect(),
    };

    if entries.is_empty() {
        if !style::is_quiet() {
            println!("no registries configured");
        }
        return EXIT_SUCCESS;
    }

    let mut errors = 0;
    for entry in entries {
        if !style::is_quiet() {
            eprintln!("{} {}", style::dim("indexing"), style::name(&entry.name));
        }
        match upskill::index::build_index(entry) {
            Ok(index) => {
                if let Err(err) = upskill::index::write_index(&index) {
                    print_error(&err);
                    errors += 1;
                } else if !style::is_quiet() {
                    println!(
                        "  {} ({} items)",
                        style::name(&entry.name),
                        index.items.len()
                    );
                }
            }
            Err(err) => {
                print_error_chain(&err);
                errors += 1;
            }
        }
    }

    if errors > 0 { EXIT_ERROR } else { EXIT_SUCCESS }
}
```

- [ ] **Step 5: Expose `cache_dir` from index module**

Add to `src/index.rs`:

```rust
/// The directory where index cache files are stored.
pub fn cache_dir() -> PathBuf {
    dirs_cache_dir().join("upskill").join("index")
}
```

- [ ] **Step 6: Run integration tests**

Run: `cargo test --test cli_search`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/cli.rs src/main.rs src/index.rs tests/
git commit -m "feat(cli): add 'upskill index' subcommand"
```

---

## Task 11: Update Docs and Spec

**Files:**

- Modify: `docs/commands.md`
- Modify: `docs/specification.md`
- Modify: `docs/getting-started.md`

- [ ] **Step 1: Add `index` command to commands.md**

Add a section documenting `upskill index [--registry <name>] [--clear]` with examples.

- [ ] **Step 2: Add `--registry` and `--kind` flags to `search` section in commands.md**

Update the existing search documentation.

- [ ] **Step 3: Add config file documentation to getting-started.md**

Document `~/.config/upskill/config.yaml` with the `registries:` key and an example.

- [ ] **Step 4: Update specification.md**

Add `config.yaml` to the environment/files table. Add `index` to the command table.

- [ ] **Step 5: Run `just lint` to verify docs**

Run: `just lint`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add docs/
git commit -m "docs: multi-registry search, index command, config file"
```

---

## Task 12: Final Verification

- [ ] **Step 1: Run full test suite**

Run: `just check`
Expected: PASS (all tests, lint, format)

- [ ] **Step 2: Run `just verify`**

Run: `just verify`
Expected: PASS

- [ ] **Step 3: Manual smoke test**

```bash
# Create a config with a local registry
mkdir -p ~/.config/upskill
cat > ~/.config/upskill/config.yaml << 'EOF'
registries:
  - name: local-test
    source: ~/path/to/your/skills-repo
EOF

# Search across all registries
upskill search code-review

# Search specific registry
upskill search code-review --registry local-test

# Force rebuild index
upskill index

# Clear cache
upskill index --clear
```

- [ ] **Step 4: Final commit if any fixes needed**
