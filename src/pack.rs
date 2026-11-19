//! Build and write Claude Code and GitHub Copilot CLI plugin directories from a source bundle.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::generate::Client;
use crate::model::Bundle;
use crate::model::bundle::{ClaudePluginDescriptor, CopilotPluginDescriptor};
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
                            ClaudePluginDescriptor::Instructions {
                                instructions_url, ..
                            } => Some(instructions_url.as_str()),
                        };
                        plugins.insert(name, url);
                    }
                }
                PackClient::Copilot => {
                    if let Some(descriptor) = &entry.copilot {
                        let url = match descriptor {
                            CopilotPluginDescriptor::Install { install_url, .. } => {
                                install_url.as_deref()
                            }
                            CopilotPluginDescriptor::Instructions {
                                instructions_url, ..
                            } => Some(instructions_url.as_str()),
                        };
                        plugins.insert(name, url);
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

/// Build a Claude Code or GitHub Copilot CLI plugin directory.
/// The caller validates the source and bundle name first.
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

/// Check all target directories, then write each plugin directory.
/// Return the path and file count of each written directory.
///
/// Each directory is first written in full to a staging directory next to
/// its target (`.<name>.upskill-stage`). A failed write removes every
/// staging directory and leaves every target unchanged. When all directories
/// are written, each staging directory replaces its target by a rename.
pub fn write_plugin_dirs(
    out: &Path,
    dirs: &[PluginDir],
    force: bool,
) -> Result<Vec<(PathBuf, usize)>> {
    for directory in dirs {
        let target = out.join(&directory.dir_name);
        match std::fs::symlink_metadata(&target) {
            Ok(metadata) => {
                anyhow::ensure!(metadata.is_dir(), "{} is not a directory", target.display());
                if !force && std::fs::read_dir(&target)?.next().transpose()?.is_some() {
                    anyhow::bail!(
                        "{} is not empty; use --force to replace it",
                        target.display()
                    );
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let stages: Vec<PathBuf> = dirs
        .iter()
        .map(|directory| out.join(format!(".{}.upskill-stage", directory.dir_name)))
        .collect();
    let staged = dirs
        .iter()
        .zip(&stages)
        .try_for_each(|(directory, stage)| write_stage(stage, directory));
    if let Err(error) = staged {
        for stage in &stages {
            let _ = std::fs::remove_dir_all(stage);
        }
        return Err(error);
    }
    let mut written = Vec::new();
    for (directory, stage) in dirs.iter().zip(&stages) {
        let target = out.join(&directory.dir_name);
        if std::fs::symlink_metadata(&target).is_ok() {
            std::fs::remove_dir_all(&target)
                .with_context(|| format!("removing {}", target.display()))?;
        }
        std::fs::rename(stage, &target)
            .with_context(|| format!("renaming {} to {}", stage.display(), target.display()))?;
        written.push((target, directory.files.len()));
    }
    Ok(written)
}

/// Write the files of `directory` under `stage`, after the removal of a
/// staging directory that an earlier run left.
fn write_stage(stage: &Path, directory: &PluginDir) -> Result<()> {
    if std::fs::symlink_metadata(stage).is_ok() {
        std::fs::remove_dir_all(stage).with_context(|| format!("removing {}", stage.display()))?;
    }
    std::fs::create_dir_all(stage).with_context(|| format!("creating {}", stage.display()))?;
    for file in &directory.files {
        let path = stage.join(&file.rel);
        std::fs::create_dir_all(path.parent().expect("output file has a parent"))
            .with_context(|| format!("creating the directory of {}", path.display()))?;
        std::fs::write(&path, &file.bytes)
            .with_context(|| format!("writing {}", path.display()))?;
        #[cfg(unix)]
        if let Some(mode) = file.mode {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))?;
        }
    }
    Ok(())
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

    fn plugin_dir(name: &str, files: &[(&str, &str)]) -> PluginDir {
        PluginDir {
            dir_name: name.into(),
            files: files
                .iter()
                .map(|(rel, text)| PackFile {
                    rel: PathBuf::from(rel),
                    bytes: text.as_bytes().to_vec(),
                    mode: None,
                })
                .collect(),
        }
    }

    /// The names in `out`, sorted, and the content of `<dir>/old.txt`.
    fn state(out: &Path) -> (Vec<String>, Vec<String>) {
        let mut names: Vec<String> = std::fs::read_dir(out)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        let old = names
            .iter()
            .map(|n| std::fs::read_to_string(out.join(n).join("old.txt")).unwrap_or_default())
            .collect();
        (names, old)
    }

    #[test]
    fn force_failure_on_the_second_client_leaves_both_directories_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path();
        for name in ["b-claude", "b-copilot"] {
            std::fs::create_dir_all(out.join(name)).unwrap();
            std::fs::write(out.join(name).join("old.txt"), name).unwrap();
        }
        let before = state(out);
        // `x/y` needs a directory `x`, but `x` is a file: the write fails.
        let dirs = [
            plugin_dir("b-claude", &[("new.txt", "new")]),
            plugin_dir("b-copilot", &[("x", "file"), ("x/y", "fails")]),
        ];
        assert!(write_plugin_dirs(out, &dirs, true).is_err());
        assert_eq!(state(out), before);
    }

    #[test]
    fn force_replaces_each_directory_and_leaves_no_staging_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path();
        std::fs::create_dir_all(out.join("b-claude")).unwrap();
        std::fs::write(out.join("b-claude").join("old.txt"), "old").unwrap();
        // A staging directory that an earlier run left.
        std::fs::create_dir_all(out.join(".b-claude.upskill-stage")).unwrap();
        std::fs::write(out.join(".b-claude.upskill-stage").join("stale.txt"), "x").unwrap();
        let dirs = [
            plugin_dir("b-claude", &[("a/new.txt", "new")]),
            plugin_dir("b-copilot", &[("new.txt", "new")]),
        ];
        let written = write_plugin_dirs(out, &dirs, true).unwrap();
        assert_eq!(written.len(), 2);
        assert_eq!(
            state(out).0,
            vec!["b-claude".to_string(), "b-copilot".to_string()]
        );
        assert!(!out.join("b-claude").join("old.txt").exists());
        assert!(!out.join("b-claude").join("stale.txt").exists());
        assert_eq!(
            std::fs::read_to_string(out.join("b-claude").join("a/new.txt")).unwrap(),
            "new"
        );
    }

    #[test]
    fn init_with_list_matches_golden() {
        let base: Bundle = serde_yaml_ng::from_str(include_str!(
            "../tests/fixtures/pack/registry/base.bundle.yaml"
        ))
        .unwrap();
        let text = init_text(
            "demo",
            "driftsys/demo@v1",
            &[base, demo()],
            PackClient::Claude,
        );
        assert!(text.contains("- plugin `tool`: <https://example.com/tool>\n"));
        assert_eq!(
            text,
            include_str!("../tests/fixtures/pack/expected/demo-claude/commands/init.md")
        );
    }

    #[test]
    fn init_lists_all_descriptor_forms_for_each_client_and_required_bundle() {
        let base: Bundle = serde_yaml_ng::from_str(include_str!(
            "../tests/fixtures/pack/registry/base.bundle.yaml"
        ))
        .unwrap();
        for (client, name) in [
            (PackClient::Claude, "claude"),
            (PackClient::Copilot, "copilot"),
        ] {
            let text = init_text("demo", "driftsys/demo@v1", &[base.clone(), demo()], client);
            assert!(text.contains("- plugin `base-tool`\n"));
            assert!(text.contains("- MCP server `base-server`\n"));
            assert!(text.contains(&format!(
                "- plugin `manual`: <https://example.com/{name}-manual>\n"
            )));
            let tool = if client == PackClient::Claude {
                "tool"
            } else {
                "cotool"
            };
            assert!(text.contains(&format!(
                "- plugin `{tool}`: <https://example.com/{tool}>\n"
            )));
            assert!(!text.contains("vscode-only"));
            assert!(!text.contains("opencode-only"));
            let expected = if client == PackClient::Claude {
                include_str!("../tests/fixtures/pack/expected/demo-claude/commands/init.md")
            } else {
                include_str!("../tests/fixtures/pack/expected/demo-copilot/commands/init.md")
            };
            assert_eq!(text, expected);
        }
    }

    #[test]
    fn validators_accept_safe_bundle_names_and_install_sources() {
        validate_bundle_name("bundle.name_with-hyphen").unwrap();
        for source in ["owner/repo@v1.2:sub/dir", "~/x", "https://host/a/b"] {
            validate_source(source).unwrap();
        }
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
