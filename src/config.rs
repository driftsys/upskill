//! User configuration loading and merging.
//!
//! Loads from `~/.config/upskill/config.yaml` (global) and
//! `.upskill/config.yaml` (project). Project entries override global
//! entries with the same name.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Top-level configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub registries: Vec<RegistryEntry>,
}

/// A named registry source.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub source: String,
}

/// Parse a YAML string into a [`Config`]. Empty input yields the default.
pub fn parse_config(yaml: &str) -> Result<Config> {
    if yaml.trim().is_empty() {
        return Ok(Config::default());
    }
    serde_yaml_ng::from_str(yaml).context("failed to parse config YAML")
}

/// Merge two config layers. Project entries override global entries by name;
/// global entries not shadowed are preserved.
pub fn merge_configs(global: Config, project: Config) -> Config {
    let mut registries = project.registries;
    let project_names: Vec<String> = registries.iter().map(|r| r.name.clone()).collect();
    for entry in global.registries {
        if !project_names.iter().any(|n| n == &entry.name) {
            registries.push(entry);
        }
    }
    Config { registries }
}

/// Load configuration from standard paths, merging global and project layers.
///
/// - Global: `<global_root>/.config/upskill/config.yaml`
/// - Project: `<project_root>/.upskill/config.yaml`
///
/// Missing files are treated as empty config (no error).
pub fn load_config(global_root: Option<&Path>, project_root: Option<&Path>) -> Result<Config> {
    let global = match global_root {
        Some(root) => {
            let path = root.join(".config/upskill/config.yaml");
            read_config_file(&path)?
        }
        None => Config::default(),
    };
    let project = match project_root {
        Some(root) => {
            let path = root.join(".upskill/config.yaml");
            read_config_file(&path)?
        }
        None => Config::default(),
    };
    Ok(merge_configs(global, project))
}

fn read_config_file(path: &Path) -> Result<Config> {
    match std::fs::read_to_string(path) {
        Ok(contents) => parse_config(&contents),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_with_registries() {
        let yaml = "registries:\n  - name: foo\n    source: https://example.com/foo\n  - name: bar\n    source: https://example.com/bar\n";
        let cfg = parse_config(yaml).unwrap();
        assert_eq!(cfg.registries.len(), 2);
        assert_eq!(cfg.registries[0].name, "foo");
        assert_eq!(cfg.registries[0].source, "https://example.com/foo");
        assert_eq!(cfg.registries[1].name, "bar");
    }

    #[test]
    fn parse_empty_config() {
        let cfg = parse_config("").unwrap();
        assert!(cfg.registries.is_empty());
    }

    #[test]
    fn parse_config_no_registries_key() {
        let cfg = parse_config("other_key: value\n").unwrap();
        assert!(cfg.registries.is_empty());
    }

    #[test]
    fn merge_layers_project_overrides_global() {
        let global = Config {
            registries: vec![
                RegistryEntry {
                    name: "a".into(),
                    source: "global-a".into(),
                },
                RegistryEntry {
                    name: "b".into(),
                    source: "global-b".into(),
                },
            ],
        };
        let project = Config {
            registries: vec![RegistryEntry {
                name: "a".into(),
                source: "project-a".into(),
            }],
        };
        let merged = merge_configs(global, project);
        assert_eq!(merged.registries.len(), 2);
        assert_eq!(merged.registries[0].name, "a");
        assert_eq!(merged.registries[0].source, "project-a");
        assert_eq!(merged.registries[1].name, "b");
        assert_eq!(merged.registries[1].source, "global-b");
    }

    #[test]
    fn load_config_returns_default_when_no_files() {
        let cfg = load_config(
            Some(Path::new("/nonexistent/global")),
            Some(Path::new("/nonexistent/project")),
        )
        .unwrap();
        assert!(cfg.registries.is_empty());
    }
}
