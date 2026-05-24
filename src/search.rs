use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;

use crate::index::RegistryIndex;

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
            if let Some(kind) = kind_filter
                && item.kind != kind
            {
                return false;
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

const DEFAULT_REGISTRY_URL: &str = "https://skills.sh";

/// Hard cap on the registry round-trip. Defends against a hung server or
/// slow proxy — without this, `ureq`'s default is unbounded for the body
/// read phase, which clig.dev §"Responsiveness" calls a UX bug.
const REGISTRY_TIMEOUT: Duration = Duration::from_secs(15);

pub fn registry_url() -> String {
    std::env::var("UPSKILL_REGISTRY_URL").unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_string())
}

/// Read the HTTPS proxy from the environment, mirroring what `git`, `gh`,
/// and `glab` honor implicitly. Falls back to lowercase form, which some
/// shells set instead. No `NO_PROXY` host-bypass is applied — corporate
/// users behind a proxy that excludes specific hosts should configure
/// that at the system level.
fn proxy_from_env() -> Option<String> {
    std::env::var("HTTPS_PROXY")
        .ok()
        .or_else(|| std::env::var("https_proxy").ok())
        .filter(|s| !s.trim().is_empty())
}

fn build_agent() -> Result<ureq::Agent> {
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(REGISTRY_TIMEOUT)
        .timeout_read(REGISTRY_TIMEOUT)
        .timeout_write(REGISTRY_TIMEOUT);
    if let Some(proxy_url) = proxy_from_env() {
        let proxy = ureq::Proxy::new(&proxy_url)
            .with_context(|| format!("invalid HTTPS_PROXY value: {proxy_url}"))?;
        builder = builder.proxy(proxy);
    }
    Ok(builder.build())
}

#[derive(Deserialize)]
pub struct SkillResult {
    pub name: String,
    pub installs: u64,
    pub source: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    skills: Vec<SkillResult>,
}

pub fn search(query: &str, limit: usize) -> Result<Vec<SkillResult>> {
    let base = registry_url();
    let url = format!("{}/api/search?q={}&limit={}", base, query, limit);
    let agent = build_agent()?;
    let response: SearchResponse = agent
        .get(&url)
        .call()
        .context("failed to reach skills registry")?
        .into_json()
        .context("failed to parse registry response")?;
    Ok(response.skills)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce() -> R, R>(vars: &[(&str, Option<&str>)], f: F) -> R {
        let _lock = ENV_LOCK.lock().unwrap();
        let originals: Vec<_> = vars
            .iter()
            .map(|(k, _)| (*k, std::env::var(k).ok()))
            .collect();
        for (k, v) in vars {
            // SAFETY: tests are serialised by ENV_LOCK so no concurrent mutation.
            unsafe {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
        let result = f();
        for (k, original) in &originals {
            // SAFETY: tests are serialised by ENV_LOCK so no concurrent mutation.
            unsafe {
                match original {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
        result
    }

    #[test]
    fn proxy_from_env_reads_https_proxy() {
        // On Windows env vars are case-insensitive — removing `https_proxy`
        // also removes `HTTPS_PROXY`. Only set the uppercase form there.
        #[cfg(not(windows))]
        let vars: &[(&str, Option<&str>)] = &[
            ("HTTPS_PROXY", Some("http://proxy.corp:8080")),
            ("https_proxy", None),
        ];
        #[cfg(windows)]
        let vars: &[(&str, Option<&str>)] = &[("HTTPS_PROXY", Some("http://proxy.corp:8080"))];

        with_env(vars, || {
            assert_eq!(proxy_from_env(), Some("http://proxy.corp:8080".to_string()));
        });
    }

    #[test]
    fn proxy_from_env_falls_back_to_lowercase() {
        // On Windows env vars are case-insensitive — `https_proxy` and
        // `HTTPS_PROXY` are the same variable. The "fallback" path is
        // unreachable, so this test only verifies on Unix.
        #[cfg(not(windows))]
        let vars: &[(&str, Option<&str>)] = &[
            ("HTTPS_PROXY", None),
            ("https_proxy", Some("http://proxy.corp:8080")),
        ];
        #[cfg(windows)]
        let vars: &[(&str, Option<&str>)] = &[("https_proxy", Some("http://proxy.corp:8080"))];

        with_env(vars, || {
            assert_eq!(proxy_from_env(), Some("http://proxy.corp:8080".to_string()));
        });
    }

    #[test]
    fn proxy_from_env_returns_none_when_unset() {
        with_env(&[("HTTPS_PROXY", None), ("https_proxy", None)], || {
            assert_eq!(proxy_from_env(), None);
        });
    }

    #[test]
    fn proxy_from_env_ignores_empty_value() {
        with_env(&[("HTTPS_PROXY", Some("")), ("https_proxy", None)], || {
            assert_eq!(proxy_from_env(), None);
        });
    }

    #[test]
    fn build_agent_succeeds_with_no_proxy() {
        with_env(&[("HTTPS_PROXY", None), ("https_proxy", None)], || {
            build_agent().expect("agent without proxy");
        });
    }

    #[test]
    fn build_agent_succeeds_with_valid_proxy() {
        with_env(
            &[
                ("HTTPS_PROXY", Some("http://proxy.corp:8080")),
                ("https_proxy", None),
            ],
            || {
                build_agent().expect("agent with proxy");
            },
        );
    }

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

        let results = search_index(&index, "workflow", None);
        assert_eq!(results.len(), 1);

        let results = search_index(&index, "co", Some("rule"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "no-console");
    }
}
