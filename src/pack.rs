//! Build and write Claude Code and GitHub Copilot CLI plugin directories from a source bundle.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::generate::Client;
use crate::model::Bundle;
use crate::model::bundle::{ClaudePluginDescriptor, CopilotPluginDescriptor, McpTransport};
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

/// Build the `.mcp.json` of a plugin from the MCP servers of `bundles`, or
/// `None` when no bundle declares a server. Two declarations of one name
/// conflict when their written JSON objects differ, so a difference only in
/// `requires-env`, which is not written, is not a conflict.
fn mcp_json(bundles: &[Bundle]) -> Result<Option<Vec<u8>>> {
    let mut servers: BTreeMap<&str, (&str, serde_json::Value)> = BTreeMap::new();
    for bundle in bundles {
        for (name, entry) in &bundle.mcps {
            let value = match &entry.transport {
                McpTransport::Local(local) => crate::ancillary::mcp_servers_local_value(local),
                McpTransport::Remote(remote) => crate::ancillary::mcp_servers_remote_value(remote),
            };
            if let Some((first, previous)) = servers.get(name.as_str()) {
                anyhow::ensure!(
                    previous == &value,
                    "MCP server `{name}` is declared with different content by the bundles `{first}` and `{}`",
                    bundle.name
                );
            } else {
                servers.insert(name, (&bundle.name, value));
            }
        }
    }
    if servers.is_empty() {
        return Ok(None);
    }
    let servers: BTreeMap<_, _> = servers
        .into_iter()
        .map(|(name, (_, value))| (name, value))
        .collect();
    let mut bytes = serde_json::to_vec_pretty(&serde_json::json!({ "mcpServers": servers }))?;
    bytes.push(b'\n');
    Ok(Some(bytes))
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
    }
    if !plugins.is_empty() {
        text.push_str("\nThe command does not install the following. After the install, give this list\nto the user:\n\n");
        for (name, url) in plugins {
            text.push_str(&format!("- plugin `{name}`"));
            if let Some(url) = url {
                text.push_str(&format!(": <{url}>"));
            }
            text.push('\n');
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
    if let Some(bytes) = mcp_json(&output.bundles)? {
        files.push(PackFile {
            rel: ".mcp.json".into(),
            bytes,
            mode: None,
        });
    }
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
/// its target (`.<name>.upskill-stage`). When all are written, each existing
/// target is renamed to `.<name>.upskill-old` and its staging directory is
/// renamed into place; then the `.upskill-old` directories are removed. A
/// failure while writing or renaming tries to undo the renames already made
/// and to remove the staging directories, so every target keeps its earlier
/// content unless an undo rename also fails (then the earlier content is at
/// `.<name>.upskill-old`). A `.upskill-old` directory that cannot be removed
/// after a successful run is left in place. An existing empty target is also
/// replaced.
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
    let targets: Vec<PathBuf> = dirs.iter().map(|d| out.join(&d.dir_name)).collect();
    let backups: Vec<PathBuf> = dirs
        .iter()
        .map(|directory| out.join(format!(".{}.upskill-old", directory.dir_name)))
        .collect();
    // Swap: each existing target is renamed aside, then its stage is renamed
    // into place. On a failure every swap already made is undone.
    let mut swapped: Vec<(usize, bool)> = Vec::new();
    let swap = (0..dirs.len()).try_for_each(|i| -> Result<()> {
        let (target, stage, backup) = (&targets[i], &stages[i], &backups[i]);
        remove_leftover(backup)?;
        let had_target = std::fs::symlink_metadata(target).is_ok();
        if had_target {
            std::fs::rename(target, backup).with_context(|| {
                format!("renaming {} to {}", target.display(), backup.display())
            })?;
        }
        swapped.push((i, had_target));
        #[cfg(test)]
        tests::fail_point(i)?;
        std::fs::rename(stage, target)
            .with_context(|| format!("renaming {} to {}", stage.display(), target.display()))
    });
    if let Err(error) = swap {
        for &(i, had_target) in swapped.iter().rev() {
            let target = &targets[i];
            if std::fs::symlink_metadata(&stages[i]).is_err() {
                // The stage is in place: move it back to its staging path.
                let _ = std::fs::rename(target, &stages[i]);
            }
            if had_target {
                let _ = std::fs::rename(&backups[i], target);
            }
        }
        for stage in &stages {
            let _ = std::fs::remove_dir_all(stage);
        }
        return Err(error);
    }
    // Every target is in place; the earlier directories are removed. A
    // failure here leaves a `.<name>.upskill-old` directory and is not an
    // error of the run.
    for backup in &backups {
        if std::fs::symlink_metadata(backup).is_ok() {
            let _ = std::fs::remove_dir_all(backup);
        }
    }
    Ok(targets
        .into_iter()
        .zip(dirs)
        .map(|(target, directory)| (target, directory.files.len()))
        .collect())
}

/// Remove what an earlier run left at `path`: a directory with its content,
/// or a file or symbolic link. Nothing at `path` is not an error.
fn remove_leftover(path: &Path) -> Result<()> {
    let removed = match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => Err(error),
    };
    removed.with_context(|| format!("removing {}", path.display()))
}

/// Write the files of `directory` under `stage`, after the removal of what
/// an earlier run left at that path (a directory, a file or a link).
fn write_stage(stage: &Path, directory: &PluginDir) -> Result<()> {
    remove_leftover(stage)?;
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

    thread_local! {
        /// The index of the directory whose rename into place fails.
        static FAIL_SWAP_AT: std::cell::Cell<Option<usize>> =
            const { std::cell::Cell::new(None) };
    }

    pub(super) fn fail_point(index: usize) -> Result<()> {
        if FAIL_SWAP_AT.with(|f| f.get()) == Some(index) {
            anyhow::bail!("injected rename failure at {index}");
        }
        Ok(())
    }

    /// The first client had no directory before the run: the rollback moves
    /// its new directory away again.
    #[test]
    fn rename_failure_removes_a_new_client_directory_that_did_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path();
        std::fs::create_dir_all(out.join("b-copilot")).unwrap();
        std::fs::write(out.join("b-copilot").join("old.txt"), "b-copilot").unwrap();
        let before = state(out);
        let dirs = [
            plugin_dir("b-claude", &[("new.txt", "new")]),
            plugin_dir("b-copilot", &[("new.txt", "new")]),
        ];
        FAIL_SWAP_AT.with(|f| f.set(Some(1)));
        let result = write_plugin_dirs(out, &dirs, true);
        FAIL_SWAP_AT.with(|f| f.set(None));
        let message = format!("{:#}", result.unwrap_err());
        assert!(
            message.contains("injected rename failure at 1"),
            "{message}"
        );
        assert_eq!(state(out), before);
    }

    #[test]
    fn rename_failure_on_the_second_client_restores_both_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path();
        for name in ["b-claude", "b-copilot"] {
            std::fs::create_dir_all(out.join(name)).unwrap();
            std::fs::write(out.join(name).join("old.txt"), name).unwrap();
        }
        let before = state(out);
        let dirs = [
            plugin_dir("b-claude", &[("new.txt", "new")]),
            plugin_dir("b-copilot", &[("new.txt", "new")]),
        ];
        FAIL_SWAP_AT.with(|f| f.set(Some(1)));
        let result = write_plugin_dirs(out, &dirs, true);
        FAIL_SWAP_AT.with(|f| f.set(None));
        let message = format!("{:#}", result.unwrap_err());
        assert!(
            message.contains("injected rename failure at 1"),
            "{message}"
        );
        assert_eq!(state(out), before);
        assert!(!out.join("b-claude").join("new.txt").exists());
    }

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
        let message = format!("{:#}", write_plugin_dirs(out, &dirs, true).unwrap_err());
        assert!(message.contains("x/y"), "{message}");
        assert_eq!(state(out), before);
        assert!(!out.join("b-claude").join("new.txt").exists());
    }

    #[test]
    fn force_file_write_failure_preserves_both_client_trees() {
        fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
            fn visit(root: &Path, dir: &Path, entries: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
                for entry in std::fs::read_dir(dir).unwrap() {
                    let path = entry.unwrap().path();
                    let rel = path.strip_prefix(root).unwrap().to_path_buf();
                    if path.is_dir() {
                        entries.insert(rel, None);
                        visit(root, &path, entries);
                    } else {
                        entries.insert(rel, Some(std::fs::read(path).unwrap()));
                    }
                }
            }
            let mut entries = BTreeMap::new();
            visit(root, root, &mut entries);
            entries
        }

        for failing_client in [0, 1] {
            let tmp = tempfile::tempdir().unwrap();
            let out = tmp.path();
            for name in ["b-claude", "b-copilot"] {
                let target = out.join(name);
                std::fs::create_dir_all(target.join("nested/empty")).unwrap();
                std::fs::write(target.join("old.txt"), name).unwrap();
                std::fs::write(target.join("nested/old.bin"), [0, 255, 1]).unwrap();
            }
            let before = tree(out);
            let mut dirs = [
                plugin_dir("b-claude", &[("new.txt", "new")]),
                plugin_dir("b-copilot", &[("new.txt", "new")]),
            ];
            let name = dirs[failing_client].dir_name.clone();
            // The nested file is written before writing over its parent directory fails.
            dirs[failing_client] = plugin_dir(
                &name,
                &[("collision/partial.txt", "partial"), ("collision", "fails")],
            );

            let message = format!("{:#}", write_plugin_dirs(out, &dirs, true).unwrap_err());

            assert_eq!(tree(out), before, "failed client {name}: {message}");
            // Compare the part below the temporary directory: Windows can report
            // that directory with its short (8.3) name.
            let failed_path = Path::new(&format!(".{name}.upskill-stage")).join("collision");
            assert!(
                message.contains(&format!("{}", failed_path.display())),
                "{message}"
            );
        }
    }

    #[test]
    fn force_replaces_each_directory_and_leaves_no_staging_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path();
        std::fs::create_dir_all(out.join("b-claude")).unwrap();
        std::fs::write(out.join("b-claude").join("old.txt"), "old").unwrap();
        // A staging directory and a file at a backup path, left by an
        // earlier run.
        std::fs::create_dir_all(out.join(".b-claude.upskill-stage")).unwrap();
        std::fs::write(out.join(".b-claude.upskill-stage").join("stale.txt"), "x").unwrap();
        std::fs::write(out.join(".b-claude.upskill-old"), "x").unwrap();
        std::fs::write(out.join(".b-copilot.upskill-stage"), "x").unwrap();
        std::fs::create_dir_all(out.join(".b-copilot.upskill-old").join("d")).unwrap();
        let dirs = [
            plugin_dir("b-claude", &[("a/new.txt", "new")]),
            plugin_dir("b-copilot", &[("new.txt", "new"), ("two.txt", "2")]),
        ];
        let written = write_plugin_dirs(out, &dirs, true).unwrap();
        assert_eq!(
            written,
            vec![(out.join("b-claude"), 1), (out.join("b-copilot"), 2)]
        );
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
        assert_eq!(
            std::fs::read_to_string(out.join("b-copilot").join("new.txt")).unwrap(),
            "new"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_link_at_the_staging_path_is_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path();
        std::os::unix::fs::symlink(out.join("missing"), out.join(".b-claude.upskill-stage"))
            .unwrap();
        let dirs = [plugin_dir("b-claude", &[("new.txt", "new")])];
        write_plugin_dirs(out, &dirs, false).unwrap();
        assert_eq!(state(out).0, vec!["b-claude".to_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn staging_keeps_the_file_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let mut dir = plugin_dir("b-claude", &[("hook.sh", "#!/bin/sh\n")]);
        dir.files[0].mode = Some(0o755);
        write_plugin_dirs(tmp.path(), &[dir], false).unwrap();
        let mode = std::fs::metadata(tmp.path().join("b-claude").join("hook.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
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
            assert!(!text.contains("MCP server"));
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

    fn bundle(name: &str, mcps: &str) -> Bundle {
        serde_yaml_ng::from_str(&format!(
            "schema: 1\nname: {name}\ndescription: d\nitems: {{}}\nmcps:\n{mcps}"
        ))
        .unwrap()
    }

    #[test]
    fn init_of_a_closure_with_mcp_servers_and_no_plugin_has_no_list() {
        let text = init_text(
            "a",
            "driftsys/demo@v1",
            &[bundle("a", "  s:\n    local: { command: c }\n")],
            PackClient::Claude,
        );
        assert!(!text.contains("The command does not install"));
    }

    #[test]
    fn mcp_json_refuses_each_kind_of_different_content() {
        let cases = [
            (
                "  s:\n    local: { command: c, args: [x] }\n",
                "  s:\n    local: { command: c, args: [y] }\n",
            ),
            (
                "  s:\n    local: { command: c, env: { K: x } }\n",
                "  s:\n    local: { command: c, env: { K: y } }\n",
            ),
            (
                "  s:\n    remote: { type: http, url: https://a.example }\n",
                "  s:\n    remote: { type: http, url: https://b.example }\n",
            ),
            (
                "  s:\n    remote: { type: http, url: https://a.example }\n",
                "  s:\n    remote: { type: sse, url: https://a.example }\n",
            ),
            (
                "  s:\n    remote: { type: http, url: https://a.example, headers: { H: x } }\n",
                "  s:\n    remote: { type: http, url: https://a.example, headers: { H: y } }\n",
            ),
            (
                "  s:\n    local: { command: c }\n",
                "  s:\n    remote: { type: http, url: https://a.example }\n",
            ),
        ];
        for (first, second) in cases {
            let error = mcp_json(&[bundle("a", first), bundle("b", second)]).unwrap_err();
            assert_eq!(
                error.to_string(),
                "MCP server `s` is declared with different content by the bundles `a` and `b`"
            );
        }
    }

    #[test]
    fn mcp_json_is_none_without_servers_and_joins_equal_declarations() {
        assert!(mcp_json(&[demo_without_mcps()]).unwrap().is_none());
        let server =
            "  s:\n    remote: { type: http, url: https://a.example, headers: { H: x } }\n";
        let bytes = mcp_json(&[bundle("a", server), bundle("b", server)])
            .unwrap()
            .unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\n  \"mcpServers\": {\n    \"s\": {\n      \"headers\": {\n        \"H\": \"x\"\n      },\n      \"type\": \"http\",\n      \"url\": \"https://a.example\"\n    }\n  }\n}\n"
        );
    }

    fn demo_without_mcps() -> Bundle {
        let mut bundle = demo();
        bundle.mcps.clear();
        bundle
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
