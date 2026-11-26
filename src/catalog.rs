//! Read, merge and write marketplace files for packed plugin directories.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::pack::PackClient;
use crate::pipeline::normalize_lexically;

/// The marketplace file of `client` below the catalog root.
pub fn marketplace_path(client: PackClient) -> &'static str {
    match client {
        PackClient::Claude => ".claude-plugin/marketplace.json",
        PackClient::Copilot => ".github/plugin/marketplace.json",
    }
}

/// A usage error of `--catalog`; the CLI maps it to exit code 2.
#[derive(Debug, thiserror::Error)]
pub enum CatalogUsageError {
    #[error(
        "--out {out} is not inside --catalog {root}; both paths are made absolute and normalized from their text, without following symbolic links"
    )]
    OutsideRoot { out: String, root: String },
    #[error("{path} does not exist: pass --catalog-name and --catalog-owner to create it")]
    NeedsHeader { path: String },
}

/// The `name` and `owner.name` of a new marketplace file.
pub struct Header<'a> {
    pub name: Option<&'a str>,
    pub owner: Option<&'a str>,
}

/// One plugin directory that this run packs.
pub struct PackedPlugin {
    pub client: PackClient,
    pub name: String,
    pub description: String,
    pub dir_name: String,
}

/// New file bytes and the names of entries removed by `--all` or for a skipped
/// output, in name order.
pub struct CatalogUpdate {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub removed: Vec<String>,
}

/// The checked paths and parsed marketplace files of one run.
pub struct Catalog {
    out_abs: PathBuf,
    out_rel: Vec<String>,
    files: Vec<CatalogFile>,
}

struct CatalogFile {
    client: PackClient,
    path: PathBuf,
    fields: Map<String, Value>,
    entries: Vec<Entry>,
}

#[derive(Clone, Serialize)]
struct Entry {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<Value>,
    #[serde(flatten)]
    rest: Map<String, Value>,
}

#[derive(Serialize)]
struct Marketplace {
    #[serde(flatten)]
    fields: Map<String, Value>,
    plugins: Vec<Entry>,
}

impl Catalog {
    /// The normalized absolute output directory that `source` is computed from.
    ///
    /// The plugin directories must be written here, so that they land in the
    /// same tree as the marketplace files.
    pub fn out_dir(&self) -> &Path {
        &self.out_abs
    }

    /// Check the output location and read only the selected clients' files.
    pub fn open(
        cwd: &Path,
        root: &Path,
        out: &Path,
        clients: &[PackClient],
        header: Header<'_>,
    ) -> Result<Catalog> {
        let root_abs = normalize_lexically(&cwd.join(root));
        let out_abs = normalize_lexically(&cwd.join(out));
        let Ok(relative) = out_abs.strip_prefix(&root_abs) else {
            return Err(CatalogUsageError::OutsideRoot {
                out: out.display().to_string(),
                root: root.display().to_string(),
            }
            .into());
        };
        let out_rel = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        let mut files = Vec::new();
        for client in [PackClient::Claude, PackClient::Copilot] {
            if !clients.contains(&client) {
                continue;
            }
            let path = root_abs.join(marketplace_path(client));
            let bytes = match fs::read(&path) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(error).with_context(|| format!("reading {}", path.display()));
                }
            };
            let mut fields = if let Some(bytes) = bytes {
                let value: Value = serde_json::from_slice(&bytes)
                    .with_context(|| format!("parsing {}", path.display()))?;
                let Value::Object(fields) = value else {
                    bail!("{}: the top level is not a JSON object", path.display());
                };
                fields
            } else {
                let (Some(name), Some(owner)) = (header.name, header.owner) else {
                    return Err(CatalogUsageError::NeedsHeader {
                        path: path.display().to_string(),
                    }
                    .into());
                };
                let mut fields = Map::new();
                fields.insert("name".into(), json!(name));
                fields.insert("owner".into(), json!({"name": owner}));
                fields
            };
            let mut entries = Vec::new();
            if let Some(value) = fields.remove("plugins") {
                let Value::Array(plugins) = value else {
                    bail!("{}: plugins is not an array", path.display());
                };
                for (index, value) in plugins.into_iter().enumerate() {
                    let Value::Object(mut rest) = value else {
                        bail!("{}: plugins[{index}] has no string name", path.display());
                    };
                    let Some(Value::String(name)) = rest.remove("name") else {
                        bail!("{}: plugins[{index}] has no string name", path.display());
                    };
                    entries.push(Entry {
                        name,
                        source: rest.remove("source"),
                        description: rest.remove("description"),
                        rest,
                    });
                }
            }
            files.push(CatalogFile {
                client,
                path,
                fields,
                entries,
            });
        }
        Ok(Catalog {
            out_abs,
            out_rel,
            files,
        })
    }

    /// Merge packed entries and render every file before any output is written.
    /// `skipped` lists the client outputs that `pack` skipped for a bundle with
    /// nothing to pack. The earlier entry of such an output is removed when its
    /// source is inside `--out` and names no plugin directory packed in this
    /// run, with or without `prune` (#678).
    pub fn render(
        &self,
        plugins: &[PackedPlugin],
        skipped: &[(PackClient, String)],
        prune: bool,
    ) -> Result<Vec<CatalogUpdate>> {
        let mut updates = Vec::new();
        for file in &self.files {
            let packed: Vec<_> = plugins
                .iter()
                .filter(|plugin| plugin.client == file.client)
                .collect();
            let names: BTreeSet<_> = packed.iter().map(|plugin| plugin.name.as_str()).collect();
            let sources: BTreeSet<_> = packed
                .iter()
                .map(|plugin| {
                    self.out_rel
                        .iter()
                        .map(String::as_str)
                        .chain(std::iter::once(plugin.dir_name.as_str()))
                        .collect::<Vec<_>>()
                        .join("/")
                })
                .collect();
            let skipped_names: BTreeSet<_> = skipped
                .iter()
                .filter(|(client, _)| *client == file.client)
                .map(|(_, name)| name.as_str())
                .collect();
            let mut entries = Vec::new();
            let mut removed = Vec::new();
            for entry in &file.entries {
                if names.contains(entry.name.as_str()) {
                    continue;
                }
                // Inside `--out` and naming no plugin directory packed in this run.
                let stale = entry
                    .source
                    .as_ref()
                    .and_then(Value::as_str)
                    .is_some_and(|source| {
                        source_is_inside(source, &self.out_rel)
                            && !sources.contains(&source_components(source).unwrap().join("/"))
                    });
                if stale && (prune || skipped_names.contains(entry.name.as_str())) {
                    removed.push(entry.name.clone());
                } else {
                    entries.push(entry.clone());
                }
            }
            for plugin in packed {
                let relative = self
                    .out_rel
                    .iter()
                    .map(String::as_str)
                    .chain(std::iter::once(plugin.dir_name.as_str()))
                    .collect::<Vec<_>>()
                    .join("/");
                entries.push(Entry {
                    name: plugin.name.clone(),
                    source: Some(json!(format!("./{relative}"))),
                    description: Some(json!(plugin.description)),
                    rest: Map::new(),
                });
            }
            entries.sort_by(|a, b| a.name.cmp(&b.name));
            removed.sort();
            let mut bytes = serde_json::to_vec_pretty(&Marketplace {
                fields: file.fields.clone(),
                plugins: entries,
            })?;
            bytes.push(b'\n');
            updates.push(CatalogUpdate {
                path: file.path.clone(),
                bytes,
                removed,
            });
        }
        Ok(updates)
    }
}

fn source_components(source: &str) -> Option<Vec<&str>> {
    if source.starts_with('/') || Path::new(source).is_absolute() {
        return None;
    }
    let components: Vec<_> = source
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    if components.contains(&"..") {
        None
    } else {
        Some(components)
    }
}

fn source_is_inside(source: &str, out_rel: &[String]) -> bool {
    source_components(source).is_some_and(|components| {
        components.len() > out_rel.len() && components.iter().zip(out_rel).all(|(a, b)| a == b)
    })
}

/// Write one marketplace file through a temporary file in the same directory.
///
/// An existing entry at the temporary path is removed without following links,
/// and the temporary file is created exclusively.
pub fn write_update(update: &CatalogUpdate) -> Result<()> {
    let path = &update.path;
    let parent = path
        .parent()
        .context("marketplace file has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let tmp = path.with_extension("json.tmp");
    let result = (|| -> Result<()> {
        match fs::symlink_metadata(&tmp) {
            Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(&tmp),
            Ok(_) => fs::remove_file(&tmp),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
        .with_context(|| format!("removing {}", tmp.display()))?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .with_context(|| format!("writing {}", tmp.display()))?;
        file.write_all(&update.bytes)
            .with_context(|| format!("writing {}", tmp.display()))?;
        drop(file);
        fs::rename(&tmp, path)
            .with_context(|| format!("renaming {} to {}", tmp.display(), path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    fn header() -> Header<'static> {
        Header {
            name: Some("m"),
            owner: Some("o"),
        }
    }

    #[test]
    fn marketplace_paths_of_each_client() {
        assert_eq!(
            marketplace_path(PackClient::Claude),
            ".claude-plugin/marketplace.json"
        );
        assert_eq!(
            marketplace_path(PackClient::Copilot),
            ".github/plugin/marketplace.json"
        );
    }

    #[test]
    fn open_refuses_an_out_that_is_not_inside_the_root() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path();
        let root = cwd.join("market");
        for out in [
            cwd.join("market2/plugins"),
            cwd.join("other"),
            root.join("../other"),
        ] {
            let error = Catalog::open(cwd, &root, &out, &[PackClient::Claude], header())
                .err()
                .unwrap();
            assert!(matches!(
                error.downcast_ref::<CatalogUsageError>(),
                Some(CatalogUsageError::OutsideRoot { .. })
            ));
        }
        for out in [
            root.clone(),
            root.join("plugins"),
            PathBuf::from("market/./plugins/../plugins"),
        ] {
            Catalog::open(cwd, &root, &out, &[PackClient::Claude], header()).unwrap();
        }
        assert!(!root.exists());
    }

    #[test]
    fn source_is_inside_requires_a_relative_path_below_out_without_parent_components() {
        for (source, out, expected) in [
            ("./plugins/a", vec!["plugins"], true),
            ("plugins/a", vec!["plugins"], true),
            ("./plugins/a/", vec!["plugins"], true),
            ("./plugins", vec!["plugins"], false),
            ("./plugins/", vec!["plugins"], false),
            ("./other/a", vec!["plugins"], false),
            ("./plugins/../x", vec!["plugins"], false),
            ("/plugins/a", vec!["plugins"], false),
            ("./a", vec![], true),
            ("./pluginsx/a", vec!["plugins"], false),
        ] {
            let out: Vec<String> = out.into_iter().map(String::from).collect();
            assert_eq!(source_is_inside(source, &out), expected, "{source}");
        }
    }

    #[test]
    fn open_refuses_an_invalid_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude-plugin/marketplace.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        for (bytes, message) in [
            ("not json", "parsing"),
            ("[]", "the top level is not a JSON object"),
            (r#"{"plugins": {}}"#, "plugins is not an array"),
            (
                r#"{"plugins": [{"name": 1}]}"#,
                "plugins[0] has no string name",
            ),
        ] {
            fs::write(&path, bytes).unwrap();
            let error = Catalog::open(
                tmp.path(),
                tmp.path(),
                &tmp.path().join("plugins"),
                &[PackClient::Claude],
                header(),
            )
            .err()
            .unwrap();
            let text = format!("{error:#}");
            assert!(text.contains(message), "{text}");
            assert!(text.contains(&path.display().to_string()), "{text}");
            assert!(error.downcast_ref::<CatalogUsageError>().is_none());
        }
    }

    #[test]
    fn open_reads_a_file_without_plugins_as_an_empty_list() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude-plugin/marketplace.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"name":"m","owner":{"name":"o"}}"#).unwrap();
        let catalog = Catalog::open(
            tmp.path(),
            tmp.path(),
            &tmp.path().join("plugins"),
            &[PackClient::Claude],
            Header {
                name: None,
                owner: None,
            },
        )
        .unwrap();
        let updates = catalog
            .render(
                &[PackedPlugin {
                    client: PackClient::Claude,
                    name: "demo".into(),
                    description: "Demo".into(),
                    dir_name: "demo-claude".into(),
                }],
                &[],
                false,
            )
            .unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&updates[0].bytes).unwrap(),
            json!({"name":"m", "owner":{"name":"o"}, "plugins":[{"name":"demo", "source":"./plugins/demo-claude", "description":"Demo"}]})
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_update_replaces_a_symbolic_link_at_the_temporary_path() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("marketplace.json");
        fs::write(&path, b"live").unwrap();
        std::os::unix::fs::symlink(&path, tmp.path().join("marketplace.json.tmp")).unwrap();
        let update = CatalogUpdate {
            path: path.clone(),
            bytes: b"new".to_vec(),
            removed: Vec::new(),
        };
        write_update(&update).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(
            !fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(!tmp.path().join("marketplace.json.tmp").exists());
    }

    #[cfg(unix)]
    #[test]
    fn open_reads_the_normalized_path_that_the_write_uses() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path();
        fs::create_dir_all(cwd.join("other/subdir")).unwrap();
        fs::create_dir_all(cwd.join("work")).unwrap();
        std::os::unix::fs::symlink(cwd.join("other/subdir"), cwd.join("work/link")).unwrap();
        let existing = cwd.join("other/market/.claude-plugin");
        fs::create_dir_all(&existing).unwrap();
        fs::write(
            existing.join("marketplace.json"),
            br#"{"name":"far","owner":{"name":"o"},"plugins":[]}"#,
        )
        .unwrap();
        let root = cwd.join("work/link/../market");
        let catalog = Catalog::open(
            cwd,
            &root,
            &root.join("plugins"),
            &[PackClient::Claude],
            header(),
        )
        .unwrap();
        let updates = catalog.render(&[], &[], false).unwrap();
        assert_eq!(
            updates[0].path,
            cwd.join("work/market/.claude-plugin/marketplace.json")
        );
        write_update(&updates[0]).unwrap();
        assert!(updates[0].path.is_file());
        assert_eq!(
            fs::read(cwd.join("other/market/.claude-plugin/marketplace.json")).unwrap(),
            br#"{"name":"far","owner":{"name":"o"},"plugins":[]}"#
        );
    }

    fn plugin(client: PackClient, dir_name: &str) -> PackedPlugin {
        PackedPlugin {
            client,
            name: "demo".into(),
            description: "Demo".into(),
            dir_name: dir_name.into(),
        }
    }

    fn open_with(tmp: &Path, existing: &str, out: &str, clients: &[PackClient]) -> Catalog {
        let path = tmp.join("market/.claude-plugin/marketplace.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, existing).unwrap();
        Catalog::open(tmp, Path::new("market"), Path::new(out), clients, header()).unwrap()
    }

    fn names_of(update: &CatalogUpdate) -> Vec<String> {
        let value: Value = serde_json::from_slice(&update.bytes).unwrap();
        value["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["name"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn prune_keeps_an_entry_that_points_at_a_packed_directory_under_another_name() {
        let tmp = tempfile::tempdir().unwrap();
        let catalog = open_with(
            tmp.path(),
            r#"{"plugins":[
                {"name":"alias","source":"./plugins/demo-claude"},
                {"name":"stale","source":"./plugins/old"}]}"#,
            "market/plugins",
            &[PackClient::Claude],
        );
        let updates = catalog
            .render(&[plugin(PackClient::Claude, "demo-claude")], &[], true)
            .unwrap();
        assert_eq!(updates[0].removed, ["stale"]);
        assert_eq!(names_of(&updates[0]), ["alias", "demo"]);
    }

    #[test]
    fn skipped_output_removes_only_a_stale_entry_of_its_name() {
        for prune in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            let catalog = open_with(
                tmp.path(),
                r#"{"plugins":[
                    {"name":"alias","source":"./plugins/demo-claude"},
                    {"name":"gone","source":"./plugins/gone-claude"},
                    {"name":"kept","source":"./other/gone-claude"}]}"#,
                "market/plugins",
                &[PackClient::Claude],
            );
            let skipped = [
                (PackClient::Claude, "alias".to_string()),
                (PackClient::Claude, "gone".to_string()),
                (PackClient::Claude, "kept".to_string()),
            ];
            let updates = catalog
                .render(
                    &[plugin(PackClient::Claude, "demo-claude")],
                    &skipped,
                    prune,
                )
                .unwrap();
            assert_eq!(updates[0].removed, ["gone"], "prune={prune}");
            assert_eq!(names_of(&updates[0]), ["alias", "demo", "kept"]);
        }
    }

    #[test]
    fn prune_uses_only_the_packed_directories_of_the_same_client() {
        let tmp = tempfile::tempdir().unwrap();
        let claude = tmp.path().join("market/.claude-plugin/marketplace.json");
        let copilot = tmp.path().join("market/.github/plugin/marketplace.json");
        for (path, entries) in [
            (
                &claude,
                r#"{"plugins":[
                    {"name":"other","source":"./plugins/demo-copilot"},
                    {"name":"mine","source":"./plugins/demo-claude"}]}"#,
            ),
            (
                &copilot,
                r#"{"plugins":[
                    {"name":"other","source":"./plugins/demo-claude"},
                    {"name":"mine","source":"./plugins/demo-copilot"}]}"#,
            ),
        ] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, entries).unwrap();
        }
        let catalog = Catalog::open(
            tmp.path(),
            Path::new("market"),
            Path::new("market/plugins"),
            &[PackClient::Claude, PackClient::Copilot],
            header(),
        )
        .unwrap();
        let updates = catalog
            .render(
                &[
                    plugin(PackClient::Claude, "demo-claude"),
                    plugin(PackClient::Copilot, "demo-copilot"),
                ],
                &[],
                true,
            )
            .unwrap();
        assert!(updates[0].path.ends_with(".claude-plugin/marketplace.json"));
        assert_eq!(updates[0].removed, ["other"]);
        assert!(updates[1].path.ends_with(".github/plugin/marketplace.json"));
        assert_eq!(updates[1].removed, ["other"]);
    }

    #[test]
    fn out_dir_is_the_normalized_absolute_path() {
        let tmp = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(
            tmp.path(),
            Path::new("market"),
            Path::new("market/x/../plugins"),
            &[PackClient::Claude],
            header(),
        )
        .unwrap();
        assert_eq!(catalog.out_dir(), tmp.path().join("market/plugins"));
    }

    #[test]
    fn source_follows_the_normalized_out_below_the_catalog_root() {
        let tmp = tempfile::tempdir().unwrap();
        for (out, expected) in [
            ("market/./plugins/../plugins", "./plugins/demo-claude"),
            ("market", "./demo-claude"),
            ("./market/", "./demo-claude"),
        ] {
            let catalog = open_with(tmp.path(), "{}", out, &[PackClient::Claude]);
            let updates = catalog
                .render(&[plugin(PackClient::Claude, "demo-claude")], &[], false)
                .unwrap();
            let value: Value = serde_json::from_slice(&updates[0].bytes).unwrap();
            assert_eq!(value["plugins"][0]["source"], expected, "{out}");
        }
    }

    #[test]
    fn write_update_removes_the_temporary_file_after_a_failed_rename() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("marketplace.json");
        fs::create_dir_all(path.join("inner")).unwrap();
        let update = CatalogUpdate {
            path: path.clone(),
            bytes: b"new".to_vec(),
            removed: Vec::new(),
        };
        let error = write_update(&update).unwrap_err();
        assert!(format!("{error:#}").contains("renaming"), "{error:#}");
        assert!(fs::symlink_metadata(tmp.path().join("marketplace.json.tmp")).is_err());
        assert!(path.join("inner").is_dir());
    }

    #[test]
    fn write_update_replaces_a_directory_at_the_temporary_path() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("marketplace.json");
        let tmp_path = tmp.path().join("marketplace.json.tmp");
        fs::create_dir_all(tmp_path.join("inner")).unwrap();
        fs::write(tmp_path.join("inner/file"), "x").unwrap();
        let update = CatalogUpdate {
            path: path.clone(),
            bytes: b"new".to_vec(),
            removed: Vec::new(),
        };
        write_update(&update).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(!tmp_path.exists());
    }
}
