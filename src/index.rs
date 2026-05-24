//! Registry index: scan, cache, and HEAD-based freshness checking.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::config::RegistryEntry;
use crate::fetch::shallow_clone;
use crate::parse::frontmatter;
use crate::source::{InstallSource, home_dir, parse_install_source};

/// Schema version for the index file format.
const SCHEMA_VERSION: u32 = 1;

/// On-disk registry index.
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryIndex {
    pub schema: u32,
    pub registry: String,
    pub source: String,
    pub head: Option<String>,
    pub indexed_at: String,
    pub items: Vec<IndexedItem>,
}

/// A single item discovered in a registry.
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexedItem {
    pub name: String,
    pub kind: String,
    pub description: String,
    pub path: String,
}

/// Result from `ensure_fresh` that may include a staleness warning.
pub struct FreshResult {
    pub index: RegistryIndex,
    pub warning: Option<String>,
}

/// Returns the cache directory for index files.
pub fn cache_dir() -> PathBuf {
    if let Some(xdg) = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|v| !v.is_empty())
    {
        return PathBuf::from(xdg).join("upskill/index");
    }
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache/upskill/index")
}

/// Returns the cache file path for a given registry name.
pub fn cache_path(registry_name: &str) -> PathBuf {
    cache_dir().join(format!("{}.json", registry_name))
}

/// Read a cached index from disk. Returns `None` if the file does not exist.
pub fn read_index(registry_name: &str) -> Result<Option<RegistryIndex>> {
    let path = cache_path(registry_name);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read index cache at {}", path.display()))?;
    let index: RegistryIndex = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse index cache at {}", path.display()))?;
    Ok(Some(index))
}

/// Write an index to the cache directory.
pub fn write_index(index: &RegistryIndex) -> Result<()> {
    let path = cache_path(&index.registry);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create cache dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(index).context("failed to serialize index")?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write index cache at {}", path.display()))?;
    Ok(())
}

/// Fetch the HEAD sha from a remote git URL using `git ls-remote`.
pub fn fetch_remote_head(url: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["ls-remote", url, "HEAD"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .with_context(|| format!("failed to run git ls-remote for {}", url))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git ls-remote failed for {}: {}", url, stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ls_remote_output(&stdout))
}

/// Parse the output of `git ls-remote <url> HEAD` to extract the SHA.
fn parse_ls_remote_output(output: &str) -> Option<String> {
    let line = output.lines().next()?;
    let sha = line.split('\t').next()?;
    if sha.is_empty() {
        return None;
    }
    Some(sha.to_string())
}

/// Returns true if the cached index is stale (HEAD has changed).
pub fn is_stale(index: &RegistryIndex, current_head: &str) -> bool {
    match &index.head {
        Some(cached) => cached != current_head,
        None => true,
    }
}

/// Scan a registry root directory for items (max 2 levels deep).
pub fn scan_registry(root: &Path) -> Result<Vec<IndexedItem>> {
    let mut items = Vec::new();
    scan_dir(root, root, 0, &mut items)?;
    Ok(items)
}

fn scan_dir(root: &Path, dir: &Path, depth: u32, items: &mut Vec<IndexedItem>) -> Result<()> {
    if depth > 2 {
        return Ok(());
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }

        // Check if this directory is an item (contains SKILL.md, RULE.md, or AGENT.md)
        let marker_files = [
            ("SKILL.md", "skill"),
            ("RULE.md", "rule"),
            ("AGENT.md", "agent"),
        ];
        let mut found = false;
        for (file, kind) in &marker_files {
            let marker = path.join(file);
            if marker.exists() {
                let description = extract_description(&marker).unwrap_or_default();
                let rel_path = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                items.push(IndexedItem {
                    name: name.to_string_lossy().to_string(),
                    kind: kind.to_string(),
                    description,
                    path: rel_path,
                });
                found = true;
                break;
            }
        }

        if !found {
            scan_dir(root, &path, depth + 1, items)?;
        }
    }

    Ok(())
}

/// Extract the first paragraph from the body of an SSOT file.
fn extract_description(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let (_yaml, body) = frontmatter::split(&content)?;
    let paragraph = body
        .split("\n\n")
        .find(|p| !p.trim().is_empty())?
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph)
    }
}

/// Build an index for a registry entry.
pub fn build_index(entry: &RegistryEntry) -> Result<RegistryIndex> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();

    if is_local_source(&entry.source) {
        let path = resolve_local_path(&entry.source)?;
        let items = scan_registry(&path)
            .with_context(|| format!("failed to scan registry at {}", path.display()))?;
        return Ok(RegistryIndex {
            schema: SCHEMA_VERSION,
            registry: entry.name.clone(),
            source: entry.source.clone(),
            head: None,
            indexed_at: timestamp,
            items,
        });
    }

    // Git source: clone to temp dir, scan, return index
    let source = parse_install_source(&entry.source)
        .with_context(|| format!("failed to parse source '{}'", entry.source))?;

    let clone_url = clone_url_for(&source)?;
    let tmp = tempfile::tempdir().context("failed to create tempdir for clone")?;

    shallow_clone(&clone_url, None, &entry.name, tmp.path(), None)
        .map_err(|e| anyhow::anyhow!(e))?;

    let clone_dir = tmp.path().join(&entry.name);
    let head = read_git_head(&clone_dir);
    let items = scan_registry(&clone_dir)
        .with_context(|| format!("failed to scan cloned registry {}", entry.name))?;

    Ok(RegistryIndex {
        schema: SCHEMA_VERSION,
        registry: entry.name.clone(),
        source: entry.source.clone(),
        head,
        indexed_at: timestamp,
        items,
    })
}

/// Orchestrator: ensure we have a fresh index for the given registry entry.
pub fn ensure_fresh(entry: &RegistryEntry) -> Result<FreshResult> {
    if is_local_source(&entry.source) {
        let index = build_index(entry)?;
        write_index(&index)?;
        return Ok(FreshResult {
            index,
            warning: None,
        });
    }

    // Git source: check HEAD freshness
    let source = parse_install_source(&entry.source)
        .with_context(|| format!("failed to parse source '{}'", entry.source))?;
    let clone_url = clone_url_for(&source)?;

    match fetch_remote_head(&clone_url) {
        Ok(Some(remote_head)) => {
            let use_cache = read_index(&entry.name)
                .ok()
                .flatten()
                .filter(|cached| !is_stale(cached, &remote_head));
            if let Some(cached) = use_cache {
                return Ok(FreshResult {
                    index: cached,
                    warning: None,
                });
            }
            // Rebuild
            let index = build_index(entry)?;
            write_index(&index)?;
            Ok(FreshResult {
                index,
                warning: None,
            })
        }
        Ok(None) => {
            // No HEAD returned — treat as network issue
            match read_index(&entry.name)? {
                Some(cached) => Ok(FreshResult {
                    index: cached,
                    warning: Some(format!(
                        "could not determine HEAD for '{}'; using cached index",
                        entry.name
                    )),
                }),
                None => bail!(
                    "cannot fetch HEAD for registry '{}' and no cached index exists",
                    entry.name
                ),
            }
        }
        Err(_) => {
            // Network failure
            match read_index(&entry.name)? {
                Some(cached) => Ok(FreshResult {
                    index: cached,
                    warning: Some(format!(
                        "network error fetching '{}'; using cached index",
                        entry.name
                    )),
                }),
                None => bail!(
                    "cannot reach registry '{}' and no cached index exists",
                    entry.name
                ),
            }
        }
    }
}

fn is_local_source(source: &str) -> bool {
    source.starts_with('/')
        || source.starts_with("./")
        || source.starts_with("../")
        || source.starts_with("~/")
        || looks_like_windows_path(source)
}

/// Detect Windows absolute paths like `C:\...` or `C:/...`.
fn looks_like_windows_path(source: &str) -> bool {
    let bytes = source.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn resolve_local_path(source: &str) -> Result<PathBuf> {
    if let Some(rest) = source.strip_prefix("~/") {
        let home = home_dir().context("could not determine HOME directory")?;
        Ok(home.join(rest))
    } else {
        Ok(PathBuf::from(source))
    }
}

fn clone_url_for(source: &InstallSource) -> Result<String> {
    match source {
        InstallSource::Github(repo) => Ok(format!(
            "https://github.com/{}/{}.git",
            repo.owner, repo.name
        )),
        InstallSource::Gitlab(repo) => Ok(format!(
            "https://{}/{}/{}.git",
            repo.host, repo.owner, repo.name
        )),
        InstallSource::LocalPath(_) => bail!("local paths do not have clone URLs"),
    }
}

fn read_git_head(clone_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(clone_dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn round_trip_index() {
        let index = RegistryIndex {
            schema: 1,
            registry: "test-reg".to_string(),
            source: "owner/repo".to_string(),
            head: Some("abc123".to_string()),
            indexed_at: "1700000000".to_string(),
            items: vec![IndexedItem {
                name: "my-skill".to_string(),
                kind: "skill".to_string(),
                description: "A test skill".to_string(),
                path: "my-skill".to_string(),
            }],
        };
        let json = serde_json::to_string(&index).unwrap();
        let parsed: RegistryIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.registry, "test-reg");
        assert_eq!(parsed.head, Some("abc123".to_string()));
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].name, "my-skill");
    }

    #[test]
    fn index_cache_path() {
        let path = cache_path("my-registry");
        assert!(path.to_string_lossy().contains("upskill"));
        assert!(path.to_string_lossy().ends_with(".json"));
    }

    #[test]
    fn ls_remote_head_parses_output() {
        let output = "abc123\tHEAD\n";
        assert_eq!(parse_ls_remote_output(output), Some("abc123".to_string()));
    }

    #[test]
    fn ls_remote_head_empty_output() {
        assert_eq!(parse_ls_remote_output(""), None);
    }

    #[test]
    fn is_stale_when_head_differs() {
        let index = RegistryIndex {
            schema: 1,
            registry: "r".to_string(),
            source: "s".to_string(),
            head: Some("aaa".to_string()),
            indexed_at: "0".to_string(),
            items: vec![],
        };
        assert!(is_stale(&index, "bbb"));
        assert!(!is_stale(&index, "aaa"));
    }

    #[test]
    fn is_local_source_detects_windows_paths() {
        assert!(is_local_source("C:\\Users\\foo\\registry"));
        assert!(is_local_source("D:/repos/my-reg"));
        assert!(is_local_source("/unix/path"));
        assert!(is_local_source("./relative"));
        assert!(is_local_source("../parent"));
        assert!(is_local_source("~/home"));
        assert!(!is_local_source("owner/repo"));
        assert!(!is_local_source("https://github.com/o/r"));
    }

    #[test]
    fn scan_registry_finds_items() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create a skill item
        let skill_dir = root.join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\ndescription: ignored\n---\n\nThis is the description.\n\nMore text.",
        )
        .unwrap();

        // Create a rule item in a category subdir
        let rule_dir = root.join("category/my-rule");
        fs::create_dir_all(&rule_dir).unwrap();
        fs::write(
            rule_dir.join("RULE.md"),
            "---\ntitle: Rule\n---\n\nA rule description.",
        )
        .unwrap();

        let items = scan_registry(root).unwrap();
        assert_eq!(items.len(), 2);

        let skill = items.iter().find(|i| i.name == "my-skill").unwrap();
        assert_eq!(skill.kind, "skill");
        assert_eq!(skill.description, "This is the description.");

        let rule = items.iter().find(|i| i.name == "my-rule").unwrap();
        assert_eq!(rule.kind, "rule");
        assert_eq!(rule.description, "A rule description.");
    }

    #[test]
    fn build_index_for_local_path() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let skill_dir = root.join("test-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\ntitle: Test\n---\n\nA test skill.",
        )
        .unwrap();

        let entry = RegistryEntry {
            name: "local-reg".to_string(),
            source: root.to_string_lossy().to_string(),
        };

        let index = build_index(&entry).unwrap();
        assert_eq!(index.registry, "local-reg");
        assert!(index.head.is_none());
        assert_eq!(index.items.len(), 1);
        assert_eq!(index.items[0].name, "test-skill");
    }
}
