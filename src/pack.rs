//! Build client plugin directories from a source bundle in memory.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde::Serialize;

use crate::generate::Client;
use crate::model::Bundle;
use crate::model::bundle::ClaudePluginDescriptor;
use crate::pipeline::plan_bundle_output;

/// A client whose plugin directory can be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackClient {
    Claude,
    Copilot,
}

/// One file relative to the plugin directory.
pub struct PackFile {
    pub rel: PathBuf,
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}

/// A plugin directory and its planned files.
pub struct PluginDir {
    pub dir_name: String,
    pub files: Vec<PackFile>,
}

/// Validate a source for use as an argument in the init command.
pub fn validate_source(source: &str) -> Result<()> {
    if source.starts_with('-') {
        bail!("source must not start with a hyphen");
    }
    crate::source::parse_install_source(source)?;
    if !source
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"/@:._-~+".contains(&b))
    {
        bail!("source must contain only ASCII letters, digits and / @ : . _ - ~ +");
    }
    Ok(())
}

/// Validate a bundle name for use in the init command and directory name.
pub fn validate_bundle_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.starts_with('-')
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        bail!(
            "bundle name must be nonempty, must not start with a hyphen, and must contain only ASCII letters, digits and - _ ."
        );
    }
    Ok(())
}

#[derive(Serialize)]
struct Manifest<'a> {
    name: &'a str,
    description: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<&'a str>,
}

fn manifest(bundle: &Bundle) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(&Manifest {
        name: &bundle.name,
        description: &bundle.description,
        version: bundle.metadata.as_ref().and_then(|m| m.version.as_deref()),
        license: bundle.license.as_ref().map(|l| l.0.as_str()),
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn init_text(bundle: &str, source: &str, bundles: &[Bundle], client: PackClient) -> String {
    let flag = match client {
        PackClient::Claude => "--claude",
        PackClient::Copilot => "--copilot",
    };
    let mut text = format!(
        "---\ndescription: Install the rules of the {bundle} bundle into this repository\n---\n\nRun this command in the root directory of the repository. Then report its\noutput to the user.\n\n```text\nupskill add {source} {bundle} --kind rule --project {flag}\n```\n\nIf the command fails, report the error text. Do not run another command in its\nplace.\n"
    );
    let mut plugins = BTreeMap::new();
    let mut mcps = BTreeSet::new();
    for bundle in bundles {
        for (name, entry) in &bundle.plugins {
            match client {
                PackClient::Claude => {
                    if let Some(descriptor) = &entry.claude {
                        let url = match descriptor {
                            ClaudePluginDescriptor::Install { install_url, .. } => {
                                install_url.as_deref()
                            }
                            ClaudePluginDescriptor::Instructions { .. } => None,
                        };
                        plugins.insert(name, url);
                    }
                }
                PackClient::Copilot => {
                    if entry.copilot.is_some() {
                        plugins.insert(name, None);
                    }
                }
            }
        }
        mcps.extend(bundle.mcps.keys());
    }
    if !plugins.is_empty() || !mcps.is_empty() {
        text.push_str("\nThe command does not install the following. After the install, give this list\nto the user:\n\n");
        for (name, url) in plugins {
            text.push_str(&format!("- plugin `{name}`"));
            if let Some(url) = url {
                text.push_str(&format!(": <{url}>"));
            }
            text.push('\n');
        }
        for name in mcps {
            text.push_str(&format!("- MCP server `{name}`\n"));
        }
    }
    text
}

/// Build a plugin directory; the caller validates the source and bundle name first.
pub fn build(
    registry_root: &Path,
    bundle: &str,
    source: &str,
    client: PackClient,
) -> Result<PluginDir> {
    let (generator, suffix, manifest_path, hook_path, hook) = match client {
        PackClient::Claude => (
            Client::Claude,
            "claude",
            ".claude-plugin/plugin.json",
            "hooks/hooks.json",
            include_str!("../plugins/upskill-claude/hooks/hooks.json"),
        ),
        PackClient::Copilot => (
            Client::Copilot,
            "copilot",
            "plugin.json",
            "hooks.json",
            include_str!("../plugins/upskill-copilot/hooks.json"),
        ),
    };
    let output = plan_bundle_output(registry_root, bundle, generator)?;
    let entry = output
        .bundles
        .last()
        .expect("bundle closure includes its entry");
    let mut files: Vec<PackFile> = output
        .files
        .into_iter()
        .map(|file| PackFile {
            rel: file.rel,
            bytes: file.bytes,
            mode: file.mode,
        })
        .collect();
    files.push(PackFile {
        rel: manifest_path.into(),
        bytes: manifest(entry)?,
        mode: None,
    });
    if !output.rules.is_empty() {
        files.push(PackFile {
            rel: "commands/init.md".into(),
            bytes: init_text(bundle, source, &output.bundles, client).into_bytes(),
            mode: None,
        });
        files.push(PackFile {
            rel: hook_path.into(),
            bytes: hook.as_bytes().to_vec(),
            mode: None,
        });
    }
    Ok(PluginDir {
        dir_name: format!("{bundle}-{suffix}"),
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> Bundle {
        serde_yaml_ng::from_str(include_str!(
            "../tests/fixtures/pack/registry/demo.bundle.yaml"
        ))
        .unwrap()
    }

    #[test]
    fn init_with_list_matches_golden() {
        let text = init_text("demo", "driftsys/demo@v1", &[demo()], PackClient::Claude);
        assert!(text.contains("- plugin `tool`: <https://example.com/tool>\n"));
        assert_eq!(
            text,
            include_str!("../tests/fixtures/pack/expected/demo-claude/commands/init.md")
        );
    }

    #[test]
    fn init_without_list_ends_after_failure_instruction() {
        let expected = include_str!("../tests/fixtures/pack/expected/demo-claude/commands/init.md")
            .split("\nThe command does not install")
            .next()
            .unwrap();
        assert_eq!(
            init_text("demo", "driftsys/demo@v1", &[], PackClient::Claude),
            expected
        );
    }

    #[test]
    fn manifest_with_optional_fields_matches_golden() {
        assert_eq!(
            manifest(&demo()).unwrap(),
            include_bytes!(
                "../tests/fixtures/pack/expected/demo-claude/.claude-plugin/plugin.json"
            )
        );
    }

    #[test]
    fn manifest_without_optional_fields_omits_keys() {
        let mut bundle = demo();
        bundle.metadata = None;
        bundle.license = None;
        assert_eq!(
            manifest(&bundle).unwrap(),
            b"{\n  \"name\": \"demo\",\n  \"description\": \"Demo bundle\"\n}\n"
        );
    }
}
