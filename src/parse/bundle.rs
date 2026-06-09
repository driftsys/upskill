//! Bundle file parsing and discovery (§2.2 + §3.7).
//!
//! Bundle manifests are pure YAML files named `<name>.bundle.yaml`. They
//! MAY live anywhere within a source registry — [`load`] takes a path;
//! [`discover`] walks a directory tree for the `.bundle.yaml` suffix and
//! gates each file on the top-level `schema:` key so that unrelated YAML
//! files sharing the suffix are silently skipped.
//!
//! A bundle MAY have a sibling `<name>.bundle.md` carrying human-readable
//! documentation; the parser ignores it (the suffix walk only matches
//! `.bundle.yaml`).

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::Bundle;

/// Filename suffix every bundle manifest carries (§2.2).
pub const BUNDLE_SUFFIX: &str = ".bundle.yaml";

/// Read and parse a single bundle file.
///
/// Validates that the filename stem (before `.bundle.yaml`) matches the
/// `name` field, per §2.2. Strict: any parse error is surfaced — callers
/// who want forgiving discovery use [`discover`] instead.
pub fn load(path: &Path) -> Result<Bundle> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let bundle: Bundle = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parse bundle {}", path.display()))?;

    let stem = filename_stem(path).ok_or_else(|| {
        anyhow::anyhow!(
            "{}: filename must end in `{}`",
            path.display(),
            BUNDLE_SUFFIX
        )
    })?;

    if stem != bundle.name {
        anyhow::bail!(
            "{}: filename stem `{}` does not match bundle.name `{}`",
            path.display(),
            stem,
            bundle.name
        );
    }

    bundle
        .validate_mcps()
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;

    Ok(bundle)
}

/// Recursively walk `source_root` for `*.bundle.yaml` files. Returns one
/// `(absolute path, parsed Bundle)` per file, sorted by bundle name (the
/// identifier users actually type on the CLI).
///
/// Files whose top-level `schema:` key is absent or non-integer are
/// silently skipped — the suffix matches files that may not be upskill
/// bundles, and the `schema:` field is the contract gate (ADR-0007). A
/// file with `schema:` present but otherwise malformed is reported as an
/// error: the author intended a bundle and got it wrong.
pub fn discover(source_root: &Path) -> Result<Vec<(PathBuf, Bundle)>> {
    let mut out: Vec<(PathBuf, Bundle)> = Vec::new();
    if !source_root.exists() {
        return Ok(out);
    }
    let mut paths = Vec::new();
    walk(source_root, &mut paths)?;
    for path in paths {
        let Some(bundle) = load_if_bundle(&path)? else {
            continue;
        };
        out.push((path, bundle));
    }
    out.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    Ok(out)
}

/// Discovery-time load: peeks the `schema:` key and returns `Ok(None)`
/// when absent or non-integer (file is not an upskill bundle).
fn load_if_bundle(path: &Path) -> Result<Option<Bundle>> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if !has_schema_field(&raw) {
        return Ok(None);
    }
    let bundle: Bundle = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parse bundle {}", path.display()))?;

    let stem = filename_stem(path).ok_or_else(|| {
        anyhow::anyhow!(
            "{}: filename must end in `{}`",
            path.display(),
            BUNDLE_SUFFIX
        )
    })?;
    if stem != bundle.name {
        anyhow::bail!(
            "{}: filename stem `{}` does not match bundle.name `{}`",
            path.display(),
            stem,
            bundle.name
        );
    }

    bundle
        .validate_mcps()
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;

    Ok(Some(bundle))
}

/// Cheap pre-parse: returns true iff the YAML has a top-level integer
/// `schema:` key. Avoids deserialising the whole `Bundle` for files that
/// aren't bundles.
fn has_schema_field(raw: &str) -> bool {
    let Ok(value) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(raw) else {
        return false;
    };
    value
        .as_mapping()
        .and_then(|m| m.get(serde_yaml_ng::Value::String("schema".into())))
        .is_some_and(|v| v.is_u64() || v.is_i64())
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            // Skip `.git` and other dot-directories — they never contain
            // user-authored bundles and would slow large-tree walks.
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'))
            {
                continue;
            }
            walk(&path, out)?;
        } else if file_type.is_file() && filename_stem(&path).is_some() {
            out.push(path);
        }
    }
    Ok(())
}

/// Returns `Some("<name>")` for `<name>.bundle.yaml`, `None` otherwise.
fn filename_stem(path: &Path) -> Option<&str> {
    let name = path.file_name().and_then(|n| n.to_str())?;
    name.strip_suffix(BUNDLE_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_file(dir: &Path, rel: &str, content: &str) -> PathBuf {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    const PLATFORM: &str = "schema: 1
name: platform-baseline
description: Baseline rules, skills, and agents for all repositories
license: proprietary

items:
  rules:
    - api-conventions
    - license-awareness
  skills:
    - create-api-endpoint
  agents:
    - security-reviewer

metadata:
  version: \"1.0.0\"
  author: platform-dx
";

    #[test]
    fn load_round_trips_format_spec_example() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "platform-baseline.bundle.yaml", PLATFORM);

        let bundle = load(&path).expect("load");

        assert_eq!(bundle.name, "platform-baseline");
        assert_eq!(bundle.schema.get(), 1);
        assert_eq!(bundle.items.rules.len(), 2);
        assert_eq!(bundle.items.skills, vec!["create-api-endpoint"]);
        assert_eq!(bundle.items.agents, vec!["security-reviewer"]);
        assert!(bundle.requires.is_empty(), "no requires specified");
        assert_eq!(
            bundle.metadata.as_ref().and_then(|m| m.version.as_deref()),
            Some("1.0.0")
        );
    }

    #[test]
    fn load_parses_requires_with_and_without_version() {
        let content = "schema: 1
name: rust-baseline
description: Rust-specific baseline on top of platform
items:
  rules: []
requires:
  - { name: platform-baseline, version: \"^1.0.0\" }
  - { name: shared-conventions }
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "rust-baseline.bundle.yaml", content);

        let bundle = load(&path).expect("load");
        assert_eq!(bundle.requires.len(), 2);
        assert_eq!(bundle.requires[0].name, "platform-baseline");
        assert_eq!(bundle.requires[0].version.as_deref(), Some("^1.0.0"));
        assert_eq!(bundle.requires[1].name, "shared-conventions");
        assert_eq!(bundle.requires[1].version, None);
    }

    #[test]
    fn load_parses_requires_with_cross_source() {
        // A bundle MAY depend on a bundle living in another source via the
        // `source` field (the `upskill add` DSL). Bare and same-source entries
        // keep `source: None`.
        let content = "schema: 1
name: meta
description: Meta-bundle composing bundles across sources
items:
  rules: []
requires:
  - { name: markspec-core, source: \"driftsys/markspec@v0.8.0\" }
  - { name: local-baseline }
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "meta.bundle.yaml", content);

        let bundle = load(&path).expect("load");
        assert_eq!(bundle.requires.len(), 2);
        assert_eq!(bundle.requires[0].name, "markspec-core");
        assert_eq!(
            bundle.requires[0].source.as_deref(),
            Some("driftsys/markspec@v0.8.0")
        );
        assert_eq!(bundle.requires[1].name, "local-baseline");
        assert_eq!(bundle.requires[1].source, None);
    }

    #[test]
    fn load_rejects_string_form_requires() {
        // Per §3.7 (post-PR #76): map-only `requires`. Bare strings must
        // fail to deserialize so a future polymorphic shape stays a
        // deliberate spec change, not a silent acceptance.
        let content = "schema: 1
name: bare
description: Should fail
items:
  rules: []
requires:
  - just-a-string
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "bare.bundle.yaml", content);

        let err = load(&path).expect_err("string-form requires must be rejected");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("requires") || msg.contains("Requires") || msg.contains("expected"),
            "expected serde error, got: {msg}"
        );
    }

    #[test]
    fn load_rejects_filename_stem_mismatch() {
        let content = "schema: 1
name: platform-baseline
description: filename mismatch
items:
  rules: []
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "wrong-name.bundle.yaml", content);

        let err = load(&path).expect_err("must reject filename mismatch");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("filename stem") && msg.contains("platform-baseline"),
            "got: {msg}"
        );
    }

    #[test]
    fn load_accepts_empty_items_when_only_requires() {
        // A meta-bundle that just composes other bundles is valid.
        let content = "schema: 1
name: meta
description: Composes other bundles
items: {}
requires:
  - { name: platform-baseline }
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "meta.bundle.yaml", content);
        let bundle = load(&path).expect("load");
        assert!(bundle.items.is_empty());
        assert_eq!(bundle.requires.len(), 1);
    }

    #[test]
    fn discover_finds_bundles_recursively_anywhere_in_tree() {
        // §2.2: bundles MAY live anywhere — at root, alongside item dirs,
        // in dedicated `bundles/`. Discovery must find all three.
        let tmp = tempfile::tempdir().unwrap();

        write_file(
            tmp.path(),
            "root-only.bundle.yaml",
            &renamed(PLATFORM, "root-only"),
        );
        write_file(
            tmp.path(),
            "bundles/in-bundles-dir.bundle.yaml",
            &renamed(PLATFORM, "in-bundles-dir"),
        );
        write_file(
            tmp.path(),
            "nested/alongside.bundle.yaml",
            &renamed(PLATFORM, "alongside"),
        );
        // Dot-directory contents should be skipped.
        write_file(
            tmp.path(),
            ".git/skipped.bundle.yaml",
            "schema: 1\nname: skipped\ndescription: nope\nitems: {}\n",
        );
        // Non-bundle file should be ignored.
        write_file(tmp.path(), "x/SKILL.md", "---\nname: x\n---\nbody");
        // Sibling `<name>.bundle.md` README must not be parsed as a bundle.
        write_file(
            tmp.path(),
            "root-only.bundle.md",
            "# Root only — human docs\n",
        );

        let found = discover(tmp.path()).expect("discover");
        let names: Vec<&str> = found.iter().map(|(_, b)| b.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["alongside", "in-bundles-dir", "root-only"],
            "deterministic sort by bundle name; sibling .bundle.md ignored"
        );
    }

    #[test]
    fn discover_skips_yaml_files_without_schema_field() {
        // A `.bundle.yaml` file without a top-level `schema:` key is not
        // an upskill bundle (ADR-0007). Discovery silently skips it; only
        // an explicit `load()` against it would error.
        let tmp = tempfile::tempdir().unwrap();

        write_file(tmp.path(), "real.bundle.yaml", &renamed(PLATFORM, "real"));
        write_file(
            tmp.path(),
            "unrelated.bundle.yaml",
            "foo: bar\nbaz:\n  - 1\n  - 2\n",
        );

        let found = discover(tmp.path()).expect("discover");
        let names: Vec<&str> = found.iter().map(|(_, b)| b.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["real"],
            "schema-less file silently skipped in discovery"
        );
    }

    #[test]
    fn discover_errors_when_schema_present_but_otherwise_malformed() {
        // `schema:` present signals "author intended a bundle here".
        // Subsequent parse failures must surface, not be skipped.
        let tmp = tempfile::tempdir().unwrap();
        write_file(
            tmp.path(),
            "broken.bundle.yaml",
            "schema: 1\nname: broken\n# missing required `description` and `items`\n",
        );

        let err = discover(tmp.path()).expect_err("malformed bundle must surface");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("parse bundle") || msg.contains("description") || msg.contains("items"),
            "expected parse error, got: {msg}"
        );
    }

    #[test]
    fn discover_empty_when_root_missing_or_no_bundles() {
        let tmp = tempfile::tempdir().unwrap();
        let nonexistent = tmp.path().join("does-not-exist");
        assert!(discover(&nonexistent).unwrap().is_empty());

        let empty = tmp.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        assert!(discover(&empty).unwrap().is_empty());
    }

    /// Helper: produce a copy of a bundle YAML with the `name` field
    /// rewritten so multiple fixtures can share the same body shape but
    /// keep filename-stem agreement.
    fn renamed(template: &str, new_name: &str) -> String {
        template.replace("name: platform-baseline", &format!("name: {new_name}"))
    }

    #[test]
    fn load_parses_plugins_with_all_clients() {
        use crate::model::bundle::{
            ClaudePluginDescriptor, CopilotPluginDescriptor, OpencodePluginDescriptor,
            VscodePluginDescriptor,
        };

        let content = "schema: 1
name: with-plugins
description: Bundle with plugins declared
items:
  rules:
    - license-awareness
plugins:
  superpowers:
    claude:
      source: anthropics/claude-plugins
      marketplace: claude-plugins
      plugin: superpowers
      install_url: https://github.com/obra/superpowers#install
    copilot:
      source: obra/superpowers-marketplace
      marketplace: superpowers-marketplace
      plugin: superpowers
      install_url: https://github.com/obra/superpowers#install
    vscode:
      extension: anthropic.superpowers
      install_url: https://marketplace.visualstudio.com/items?itemName=anthropic.superpowers
    opencode:
      module: superpowers-opencode
      install_url: https://opencode.ai/plugins/superpowers
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "with-plugins.bundle.yaml", content);

        let bundle = load(&path).expect("load");
        assert_eq!(bundle.plugins.len(), 1);

        let sp = &bundle.plugins["superpowers"];
        let claude = sp.claude.as_ref().expect("claude block");
        match claude {
            ClaudePluginDescriptor::Install {
                source,
                marketplace,
                plugin,
                install_url,
            } => {
                assert_eq!(source, "anthropics/claude-plugins");
                assert_eq!(marketplace, "claude-plugins");
                assert_eq!(plugin, "superpowers");
                assert_eq!(
                    install_url.as_deref(),
                    Some("https://github.com/obra/superpowers#install")
                );
            }
            _ => panic!("expected Install variant"),
        }

        let copilot = sp.copilot.as_ref().expect("copilot block");
        match copilot {
            CopilotPluginDescriptor::Install {
                source,
                marketplace,
                plugin,
                install_url,
            } => {
                assert_eq!(source, "obra/superpowers-marketplace");
                assert_eq!(marketplace, "superpowers-marketplace");
                assert_eq!(plugin, "superpowers");
                assert_eq!(
                    install_url.as_deref(),
                    Some("https://github.com/obra/superpowers#install")
                );
            }
            _ => panic!("expected Install variant"),
        }

        let vscode = sp.vscode.as_ref().expect("vscode block");
        match vscode {
            VscodePluginDescriptor::Install {
                extension,
                install_url,
            } => {
                assert_eq!(extension, "anthropic.superpowers");
                assert_eq!(
                    install_url.as_deref(),
                    Some(
                        "https://marketplace.visualstudio.com/items?itemName=anthropic.superpowers"
                    )
                );
            }
            _ => panic!("expected Install variant"),
        }

        let opencode = sp.opencode.as_ref().expect("opencode block");
        match opencode {
            OpencodePluginDescriptor::Install {
                module,
                install_url,
            } => {
                assert_eq!(module, "superpowers-opencode");
                assert_eq!(
                    install_url.as_deref(),
                    Some("https://opencode.ai/plugins/superpowers")
                );
            }
            _ => panic!("expected Install variant"),
        }
    }

    #[test]
    fn load_parses_plugins_with_single_client() {
        // A plugin that only targets one client is valid.
        let content = "schema: 1
name: claude-only
description: Plugin for Claude only
items:
  skills: []
plugins:
  my-plugin:
    claude:
      source: my-org/marketplace
      marketplace: marketplace
      plugin: my-plugin
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "claude-only.bundle.yaml", content);

        let bundle = load(&path).expect("load");
        assert_eq!(bundle.plugins.len(), 1);

        let plugin = &bundle.plugins["my-plugin"];
        assert!(plugin.claude.is_some());
        assert!(plugin.vscode.is_none());
        assert!(plugin.opencode.is_none());
    }

    #[test]
    fn load_rejects_claude_plugin_without_marketplace() {
        // A claude auto-install descriptor with `source`+`plugin` but no
        // `marketplace` is invalid: the CLI's install ref needs the
        // marketplace NAME, which is distinct from the source (#227).
        let content = "schema: 1
name: missing-marketplace
description: Claude plugin missing the marketplace name
items:
  rules: []
plugins:
  superpowers:
    claude:
      source: anthropics/claude-plugins
      plugin: superpowers
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "missing-marketplace.bundle.yaml", content);

        assert!(
            load(&path).is_err(),
            "claude Install descriptor without `marketplace` must be rejected"
        );
    }

    #[test]
    fn load_accepts_bundle_without_plugins() {
        // Bundles without plugins: are valid (existing behavior).
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "platform-baseline.bundle.yaml", PLATFORM);

        let bundle = load(&path).expect("load");
        assert!(bundle.plugins.is_empty());
    }

    #[test]
    fn plugins_round_trips_through_serialize() {
        let content = "schema: 1
name: roundtrip
description: Roundtrip test
items:
  rules: []
plugins:
  example:
    vscode:
      extension: publisher.example
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "roundtrip.bundle.yaml", content);

        let bundle = load(&path).expect("load");
        let serialized = serde_yaml_ng::to_string(&bundle).expect("serialize");
        let reparsed: Bundle = serde_yaml_ng::from_str(&serialized).expect("reparse");
        assert_eq!(bundle.plugins, reparsed.plugins);
    }

    #[test]
    fn load_parses_instructions_only_plugin() {
        use crate::model::bundle::ClaudePluginDescriptor;

        let content = "schema: 1
name: instructions-only
description: Plugin with instructions URL
items:
  rules: []
plugins:
  manual-plugin:
    claude:
      instructions_url: https://example.com/install-guide
      summary: Follow the guide to install manually
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "instructions-only.bundle.yaml", content);

        let bundle = load(&path).expect("load");
        assert_eq!(bundle.plugins.len(), 1);

        let plugin = &bundle.plugins["manual-plugin"];
        let claude = plugin.claude.as_ref().expect("claude block");
        match claude {
            ClaudePluginDescriptor::Instructions {
                instructions_url,
                summary,
            } => {
                assert_eq!(instructions_url, "https://example.com/install-guide");
                assert_eq!(
                    summary.as_deref(),
                    Some("Follow the guide to install manually")
                );
            }
            _ => panic!("expected Instructions variant"),
        }
    }

    #[test]
    fn load_parses_mcp_remote_and_local() {
        use crate::model::bundle::McpTransport;

        let content = "schema: 1
name: with-mcp
description: Bundle with MCP servers
items:
  skills: []
mcps:
  drawio:
    remote:
      type: http
      url: https://mcp.draw.io/mcp
  local-server:
    local:
      command: npx
      args: [\"-y\", \"drawio-mcp-server\"]
      env:
        DRAWIO_TOKEN: \"${DRAWIO_TOKEN}\"
    requires-env: [DRAWIO_TOKEN]
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "with-mcp.bundle.yaml", content);

        let bundle = load(&path).expect("load");
        assert_eq!(bundle.mcps.len(), 2);

        match &bundle.mcps["drawio"].transport {
            McpTransport::Remote(r) => {
                assert_eq!(r.transport_type, "http");
                assert_eq!(r.url, "https://mcp.draw.io/mcp");
            }
            McpTransport::Local(_) => panic!("expected remote"),
        }

        let local = &bundle.mcps["local-server"];
        match &local.transport {
            McpTransport::Local(l) => {
                assert_eq!(l.command, "npx");
                assert_eq!(l.args, vec!["-y", "drawio-mcp-server"]);
                assert_eq!(l.env["DRAWIO_TOKEN"], "${DRAWIO_TOKEN}");
            }
            McpTransport::Remote(_) => panic!("expected local"),
        }
        assert_eq!(local.requires_env, vec!["DRAWIO_TOKEN"]);
    }

    #[test]
    fn load_rejects_mcp_remote_with_bad_type() {
        let content = "schema: 1
name: bad-type
description: bad transport type
items:
  skills: []
mcps:
  x:
    remote:
      type: websocket
      url: https://example.com
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "bad-type.bundle.yaml", content);
        let err = load(&path).expect_err("must reject unknown transport type");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("transport type") && msg.contains("websocket"),
            "got: {msg}"
        );
    }

    #[test]
    fn load_rejects_mcp_local_empty_command() {
        let content = "schema: 1
name: empty-cmd
description: empty command
items:
  skills: []
mcps:
  x:
    local:
      command: \"\"
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "empty-cmd.bundle.yaml", content);
        let err = load(&path).expect_err("must reject empty command");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("command") && msg.contains("empty"),
            "got: {msg}"
        );
    }

    #[test]
    fn load_parses_config_write_plugin() {
        use crate::model::bundle::OpencodePluginDescriptor;

        let content = "schema: 1
name: config-write
description: Plugin with config write mode
items:
  rules: []
plugins:
  oc-plugin:
    opencode:
      plugin_uri: https://example.com/plugin.wasm
      install_url: https://example.com/docs
";
        let tmp = tempfile::tempdir().unwrap();
        let path = write_file(tmp.path(), "config-write.bundle.yaml", content);

        let bundle = load(&path).expect("load");
        assert_eq!(bundle.plugins.len(), 1);

        let plugin = &bundle.plugins["oc-plugin"];
        let opencode = plugin.opencode.as_ref().expect("opencode block");
        match opencode {
            OpencodePluginDescriptor::ConfigWrite {
                plugin_uri,
                install_url,
            } => {
                assert_eq!(plugin_uri, "https://example.com/plugin.wasm");
                assert_eq!(install_url.as_deref(), Some("https://example.com/docs"));
            }
            _ => panic!("expected ConfigWrite variant"),
        }
    }
}
