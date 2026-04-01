use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_REGISTRY_URL: &str = "https://skills.sh";

pub fn registry_url() -> String {
    std::env::var("UPSKILL_REGISTRY_URL").unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_string())
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
    let response: SearchResponse = ureq::get(&url)
        .call()
        .context("failed to reach skills registry")?
        .into_json()
        .context("failed to parse registry response")?;
    Ok(response.skills)
}
