//! User configuration loading and merging.
//!
//! Loads from `~/.config/upskill/config.yaml` (global) and
//! `.upskill/config.yaml` (project). Project entries override global
//! entries with the same name.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::select::{ClientSelection, SelectedClient};

/// Top-level configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub registries: Vec<RegistryEntry>,
    /// Persistent consumer-side client selection (ADR-0012). Empty means the
    /// built-in default: emit for all clients.
    pub clients: Vec<SelectedClient>,
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
/// global entries not shadowed are preserved. `clients:` does not merge
/// element-wise: a project `clients:` replaces the global one wholesale, so a
/// narrower project selection wins over a broader global one (ADR-0012).
pub fn merge_configs(global: Config, project: Config) -> Config {
    let mut registries = project.registries;
    let project_names: Vec<String> = registries.iter().map(|r| r.name.clone()).collect();
    for entry in global.registries {
        if !project_names.iter().any(|n| n == &entry.name) {
            registries.push(entry);
        }
    }
    let clients = if project.clients.is_empty() {
        global.clients
    } else {
        project.clients
    };
    Config {
        registries,
        clients,
    }
}

/// Resolve the effective client selection. Precedence, highest first:
/// per-invocation flags, then the merged config `clients:`, then the built-in
/// default (all clients). An empty `flag_clients` means no client flag was
/// passed on the command line.
pub fn resolve_client_selection(
    flag_clients: &[SelectedClient],
    config: &Config,
) -> ClientSelection {
    if !flag_clients.is_empty() {
        ClientSelection::restrict(flag_clients)
    } else {
        ClientSelection::restrict(&config.clients)
    }
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
            ..Default::default()
        };
        let project = Config {
            registries: vec![RegistryEntry {
                name: "a".into(),
                source: "project-a".into(),
            }],
            ..Default::default()
        };
        let merged = merge_configs(global, project);
        assert_eq!(merged.registries.len(), 2);
        assert_eq!(merged.registries[0].name, "a");
        assert_eq!(merged.registries[0].source, "project-a");
        assert_eq!(merged.registries[1].name, "b");
        assert_eq!(merged.registries[1].source, "global-b");
    }

    #[test]
    fn parse_config_with_clients() {
        let cfg = parse_config("clients: [claude, opencode]\n").unwrap();
        assert_eq!(
            cfg.clients,
            vec![SelectedClient::Claude, SelectedClient::OpenCode]
        );
    }

    #[test]
    fn parse_config_rejects_unknown_client() {
        assert!(parse_config("clients: [cursor]\n").is_err());
    }

    #[test]
    fn project_clients_replace_global_clients() {
        let global = Config {
            clients: vec![
                SelectedClient::Claude,
                SelectedClient::Copilot,
                SelectedClient::OpenCode,
            ],
            ..Default::default()
        };
        let project = Config {
            clients: vec![SelectedClient::Claude],
            ..Default::default()
        };
        let merged = merge_configs(global, project);
        assert_eq!(merged.clients, vec![SelectedClient::Claude]);
    }

    #[test]
    fn empty_project_clients_fall_back_to_global() {
        let global = Config {
            clients: vec![SelectedClient::OpenCode],
            ..Default::default()
        };
        let merged = merge_configs(global, Config::default());
        assert_eq!(merged.clients, vec![SelectedClient::OpenCode]);
    }

    #[test]
    fn flags_override_config_selection() {
        let config = Config {
            clients: vec![SelectedClient::Claude],
            ..Default::default()
        };
        // A flag selection wins over config.
        let sel = resolve_client_selection(&[SelectedClient::OpenCode], &config);
        assert!(sel.targets_generation(crate::generate::Client::OpenCode));
        assert!(!sel.targets_generation(crate::generate::Client::Claude));
    }

    #[test]
    fn no_flags_use_config_selection() {
        let config = Config {
            clients: vec![SelectedClient::Claude],
            ..Default::default()
        };
        let sel = resolve_client_selection(&[], &config);
        assert!(sel.targets_generation(crate::generate::Client::Claude));
        assert!(!sel.targets_generation(crate::generate::Client::Copilot));
    }

    #[test]
    fn no_flags_no_config_targets_all() {
        let sel = resolve_client_selection(&[], &Config::default());
        assert!(sel.is_all());
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
