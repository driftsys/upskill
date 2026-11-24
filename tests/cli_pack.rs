mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

struct Fixture {
    _tmp: tempfile::TempDir,
    registry: PathBuf,
    home: PathBuf,
    out: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let registry = tmp.path().join("registry");
        let home = tmp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        for (rel, bytes) in tree(Path::new("tests/fixtures/pack/registry")) {
            let dest = registry.join(rel);
            fs::create_dir_all(dest.parent().unwrap()).unwrap();
            fs::write(dest, bytes).unwrap();
        }
        let out = tmp.path().join("out");
        Self {
            _tmp: tmp,
            registry,
            home,
            out,
        }
    }

    fn command(&self, bundle: &str) -> assert_cmd::Command {
        let mut cmd = self.command_for_clients(bundle);
        cmd.arg("--claude");
        cmd
    }

    fn command_for_clients(&self, bundle: &str) -> assert_cmd::Command {
        let mut cmd = common::upskill_cmd(&self.home);
        cmd.current_dir(&self.registry)
            .args(["pack", bundle, "--source", "driftsys/demo@v1", "--out"])
            .arg(&self.out);
        cmd
    }

    fn target(&self) -> PathBuf {
        self.out.join("demo-claude")
    }

    fn assert_golden(&self) {
        assert_eq!(
            tree(&self.target()),
            tree(Path::new("tests/fixtures/pack/expected/demo-claude"))
        );
    }
}

#[test]
fn pack_writes_the_claude_plugin_directory() {
    let f = Fixture::new();
    f.command("demo").assert().success();
    f.assert_golden();
}

#[test]
fn pack_output_holds_no_rule_and_no_item_outside_the_closure() {
    let f = Fixture::new();
    f.command("demo").assert().success();
    for path in tree(&f.target()).keys() {
        let path = path.to_str().unwrap();
        for forbidden in ["rules/", "style", "naming", "stray"] {
            assert!(!path.contains(forbidden));
        }
    }
    assert!(f.target().join("skills/beta/SKILL.md").is_file());
}

#[test]
fn pack_hook_file_equals_the_notice_plugin() {
    let f = Fixture::new();
    f.command("demo").assert().success();
    assert_eq!(
        fs::read(f.target().join("hooks/hooks.json")).unwrap(),
        fs::read("plugins/upskill-claude/hooks/hooks.json").unwrap()
    );
}

#[test]
fn pack_of_a_bundle_without_rules_writes_no_init_command_and_no_hook() {
    let f = Fixture::new();
    f.command("norules").assert().success();
    let target = f.out.join("norules-claude");
    assert!(!target.join("skills/init/SKILL.md").exists());
    assert!(!target.join("hooks").exists());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join(".claude-plugin/plugin.json")).unwrap())
            .unwrap();
    assert!(manifest.get("version").is_none());
    assert!(manifest.get("license").is_none());
}

#[test]
fn pack_refuses_a_non_empty_output_directory() {
    let f = Fixture::new();
    fs::create_dir_all(f.target()).unwrap();
    fs::write(f.target().join("keep"), "unchanged").unwrap();
    let before = tree(&f.out);
    f.command("demo")
        .assert()
        .code(1)
        .stderr(predicates::str::contains(f.target().to_str().unwrap()))
        .stderr(predicates::str::contains("--force"));
    assert_eq!(tree(&f.out), before);
}

#[test]
fn pack_with_force_replaces_the_output_directory() {
    let f = Fixture::new();
    fs::create_dir_all(f.target()).unwrap();
    fs::write(f.target().join("keep"), "unchanged").unwrap();
    f.command("demo").arg("--force").assert().success();
    f.assert_golden();
}

#[test]
fn pack_with_force_replaces_both_client_directories() {
    let f = Fixture::new();
    for name in ["demo-claude", "demo-copilot"] {
        let target = f.out.join(name);
        fs::create_dir_all(target.join("obsolete/empty")).unwrap();
        fs::write(target.join("obsolete/keep.bin"), [0, 255, 1]).unwrap();
    }

    f.command_for_clients("demo")
        .arg("--force")
        .assert()
        .success();

    for name in ["demo-claude", "demo-copilot"] {
        assert_eq!(
            tree(&f.out.join(name)),
            tree(&Path::new("tests/fixtures/pack/expected").join(name))
        );
        assert!(!f.out.join(name).join("obsolete").exists());
    }
    assert_eq!(fs::read_dir(&f.out).unwrap().count(), 2);
}

#[test]
fn pack_of_an_unknown_bundle_fails() {
    let f = Fixture::new();
    f.command("nope")
        .assert()
        .code(1)
        .stderr(predicates::str::contains("no bundle named 'nope'"));
    assert!(!f.out.exists());
}

#[test]
fn pack_refuses_a_source_with_whitespace_or_a_shell_metacharacter() {
    let f = Fixture::new();
    for source in [
        "a/b; rm -rf x",
        "a/b c",
        "$(x)",
        "a/b`x`",
        "-g/x",
        "--global/x",
    ] {
        common::upskill_cmd(&f.home)
            .current_dir(&f.registry)
            .args(["pack", "demo"])
            .arg(format!("--source={source}"))
            .arg("--out")
            .arg(&f.out)
            .assert()
            .code(2);
        assert!(!f.out.exists());
    }
}

#[test]
fn pack_reads_no_lockfile_and_writes_only_below_out() {
    let f = Fixture::new();
    fs::write(f.registry.join(".upskill-lock.json"), "not json").unwrap();
    let registry = tree(&f.registry);
    let home = tree(&f.home);
    f.command("demo").assert().success();
    assert_eq!(tree(&f.registry), registry);
    assert_eq!(tree(&f.home), home);
}

#[test]
fn pack_requires_source_and_out() {
    let f = Fixture::new();
    common::upskill_cmd(&f.home)
        .current_dir(&f.registry)
        .args(["pack", "demo", "--out"])
        .arg(&f.out)
        .assert()
        .code(2);
    common::upskill_cmd(&f.home)
        .current_dir(&f.registry)
        .args(["pack", "demo", "--source", "driftsys/demo@v1"])
        .assert()
        .code(2);
}

#[test]
fn pack_refuses_a_file_target_even_with_force() {
    let f = Fixture::new();
    fs::create_dir_all(&f.out).unwrap();
    fs::write(f.target(), "keep").unwrap();
    f.command("demo")
        .arg("--force")
        .assert()
        .code(1)
        .stderr(predicates::str::contains("not a directory"));
    assert_eq!(fs::read(f.target()).unwrap(), b"keep");
}

#[test]
fn pack_writes_the_copilot_plugin_directory() {
    let f = Fixture::new();
    f.command_for_clients("demo")
        .arg("--copilot")
        .assert()
        .success();
    assert_eq!(
        tree(&f.out.join("demo-copilot")),
        tree(Path::new("tests/fixtures/pack/expected/demo-copilot"))
    );
    assert!(!f.target().exists());
}

#[test]
fn pack_without_a_client_flag_writes_both_directories() {
    let f = Fixture::new();
    f.command_for_clients("demo")
        .assert()
        .success()
        .stdout(format!(
            "Wrote {} (11 files)\nWrote {} (11 files)\n",
            f.target().display(),
            f.out.join("demo-copilot").display()
        ));
    f.assert_golden();
    assert_eq!(
        tree(&f.out.join("demo-copilot")),
        tree(Path::new("tests/fixtures/pack/expected/demo-copilot"))
    );
    f.command_for_clients("demo")
        .args(["--force", "--quiet"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn pack_copilot_hook_file_equals_the_notice_plugin() {
    let f = Fixture::new();
    f.command_for_clients("demo")
        .arg("--copilot")
        .assert()
        .success();
    assert_eq!(
        fs::read(f.out.join("demo-copilot/hooks.json")).unwrap(),
        fs::read("plugins/upskill-copilot/hooks.json").unwrap()
    );
}

#[test]
fn pack_checks_both_directories_before_it_writes_one() {
    let f = Fixture::new();
    let target = f.out.join("demo-copilot");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("keep"), "unchanged").unwrap();
    let before = tree(&f.out);
    f.command_for_clients("demo")
        .assert()
        .code(1)
        .stderr(predicates::str::contains(target.to_str().unwrap()))
        .stderr(predicates::str::contains("--force"));
    assert!(!f.target().exists());
    assert_eq!(tree(&f.out), before);
}

#[test]
fn pack_gives_no_init_command_to_a_client_that_no_rule_targets() {
    let f = Fixture::new();
    fs::write(f.registry.join("claudeonly.bundle.yaml"), "schema: 1\nname: claudeonly\ndescription: Claude rules bundle\nitems:\n  skills: [alpha]\n  rules: [style]\n").unwrap();
    let rule = f.registry.join("items/style/RULE.md");
    let text = fs::read_to_string(&rule).unwrap();
    fs::write(
        rule,
        text.replace("name: style", "name: style\naudience: [claude]"),
    )
    .unwrap();
    f.command_for_clients("claudeonly").assert().success();
    let copilot = f.out.join("claudeonly-copilot");
    assert!(copilot.join("skills/alpha/SKILL.md").is_file());
    assert!(!copilot.join("skills/init/SKILL.md").exists());
    assert!(!copilot.join("hooks.json").exists());
    let claude = f.out.join("claudeonly-claude");
    assert!(claude.join("skills/init/SKILL.md").is_file());
    assert!(claude.join("hooks/hooks.json").is_file());
}

#[cfg(unix)]
#[test]
fn pack_preserves_resource_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    fs::set_permissions(
        f.registry.join("items/alpha/reference.md"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    f.command("demo").assert().success();
    assert_eq!(
        fs::metadata(f.target().join("skills/alpha/reference.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
}

#[test]
fn pack_refuses_a_bundle_name_with_a_shell_metacharacter_or_a_leading_hyphen() {
    for bundle in ["a;b", "-x", "a b", "$(x)", ""] {
        let f = Fixture::new();
        common::upskill_cmd(&f.home)
            .current_dir(&f.registry)
            .args(["pack", "--source", "driftsys/demo@v1", "--out"])
            .arg(&f.out)
            .args(["--", bundle])
            .assert()
            .code(2);
        assert!(!f.out.exists());
    }
}

#[test]
fn pack_refuses_an_agent_name_with_a_parent_component() {
    let f = Fixture::new();
    let agent = f.registry.join("items/helper/AGENT.md");
    let text = fs::read_to_string(&agent).unwrap();
    fs::write(&agent, text.replace("name: helper", "name: ../x")).unwrap();
    let bundle = f.registry.join("demo.bundle.yaml");
    let text = fs::read_to_string(&bundle).unwrap();
    fs::write(&bundle, text.replace("agents: [helper]", "agents: [../x]")).unwrap();
    let before = tree(f._tmp.path());
    f.command("demo")
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "an installed name must not contain a path separator",
        ));
    assert_eq!(tree(f._tmp.path()), before);
    assert!(!f.out.exists());
}

#[test]
fn pack_accepts_an_existing_empty_client_directory() {
    let f = Fixture::new();
    fs::create_dir_all(f.target()).unwrap();
    f.command("demo").assert().success();
    f.assert_golden();
}

#[cfg(unix)]
#[test]
fn pack_refuses_a_symbolic_link_target_with_or_without_force() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let target = f._tmp.path().join("link-target");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("keep"), "unchanged").unwrap();
    fs::create_dir_all(&f.out).unwrap();
    symlink(&target, f.target()).unwrap();
    let before = tree(&target);
    for force in [false, true] {
        let mut cmd = f.command("demo");
        if force {
            cmd.arg("--force");
        }
        cmd.assert()
            .code(1)
            .stderr(predicates::str::contains("not a directory"));
        assert_eq!(fs::read_link(f.target()).unwrap(), target);
        assert_eq!(tree(&target), before);
    }
}

#[test]
fn pack_init_lists_plugins_and_no_mcp_server_for_both_clients() {
    let f = Fixture::new();
    f.command_for_clients("demo").assert().success();
    for client in ["claude", "copilot"] {
        let text =
            fs::read_to_string(f.out.join(format!("demo-{client}/skills/init/SKILL.md"))).unwrap();
        assert!(text.contains("- plugin `base-tool`\n"));
        assert!(!text.contains("MCP server"));
        assert!(!text.contains("vscode-only"));
        assert!(!text.contains("opencode-only"));
    }
}

#[test]
fn pack_writes_mcp_json_with_the_servers_of_the_closure_for_both_clients() {
    let f = Fixture::new();
    f.command_for_clients("demo").assert().success();
    for client in ["claude", "copilot"] {
        let doc: serde_json::Value = serde_json::from_slice(
            &fs::read(f.out.join(format!("demo-{client}/.mcp.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(
            doc,
            serde_json::json!({
                "mcpServers": {
                    "base-server": { "command": "base-command" },
                    "srv": { "type": "http", "url": "https://example.com/mcp" }
                }
            })
        );
    }
}

#[test]
fn pack_of_a_bundle_without_mcp_servers_writes_no_mcp_json() {
    let f = Fixture::new();
    f.command_for_clients("norules").assert().success();
    for client in ["claude", "copilot"] {
        let target = f.out.join(format!("norules-{client}"));
        assert!(target.is_dir());
        assert!(!target.join(".mcp.json").exists());
    }
}

#[test]
fn pack_of_a_bundle_with_mcp_servers_and_no_rules_writes_mcp_json_only() {
    let f = Fixture::new();
    let bundle = f.registry.join("norules.bundle.yaml");
    let text = fs::read_to_string(&bundle).unwrap();
    fs::write(
        bundle,
        format!(
            "{text}mcps:\n  server:\n    local:\n      command: node\n      args: [server.js]\n      env: {{ MODE: fast }}\n  hosted:\n    remote:\n      type: sse\n      url: https://example.com/sse\n      headers: {{ Authorization: \"Bearer ${{TOKEN}}\" }}\n"
        ),
    )
    .unwrap();
    f.command_for_clients("norules").assert().success();
    for client in ["claude", "copilot"] {
        let target = f.out.join(format!("norules-{client}"));
        let doc: serde_json::Value =
            serde_json::from_slice(&fs::read(target.join(".mcp.json")).unwrap()).unwrap();
        assert_eq!(
            doc,
            serde_json::json!({
                "mcpServers": {
                    "hosted": {
                        "type": "sse",
                        "url": "https://example.com/sse",
                        "headers": { "Authorization": "Bearer ${TOKEN}" }
                    },
                    "server": {
                        "command": "node",
                        "args": ["server.js"],
                        "env": { "MODE": "fast" }
                    }
                }
            })
        );
        assert!(!target.join("skills/init/SKILL.md").exists());
        assert!(!target.join("hooks/hooks.json").exists());
        assert!(!target.join("hooks.json").exists());
    }
}

#[test]
fn pack_refuses_one_server_name_with_different_content_in_the_closure() {
    let f = Fixture::new();
    let bundle = f.registry.join("demo.bundle.yaml");
    let text = fs::read_to_string(&bundle).unwrap();
    fs::write(
        bundle,
        format!("{text}  base-server:\n    local: {{ command: other-command }}\n"),
    )
    .unwrap();
    f.command_for_clients("demo")
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "MCP server `base-server` is declared with different content by the bundles `base` and `demo`",
        ));
    assert!(!f.out.exists());
}

#[test]
fn pack_accepts_one_server_name_with_equal_content_in_the_closure() {
    for requires_env in ["", "    requires-env: [TOKEN]\n"] {
        let f = Fixture::new();
        let bundle = f.registry.join("demo.bundle.yaml");
        let text = fs::read_to_string(&bundle).unwrap();
        fs::write(
            bundle,
            format!("{text}  base-server:\n    local: {{ command: base-command }}\n{requires_env}"),
        )
        .unwrap();
        f.command_for_clients("demo").assert().success();
        for client in ["claude", "copilot"] {
            let doc: serde_json::Value = serde_json::from_slice(
                &fs::read(f.out.join(format!("demo-{client}/.mcp.json"))).unwrap(),
            )
            .unwrap();
            assert_eq!(
                doc,
                serde_json::json!({
                    "mcpServers": {
                        "base-server": { "command": "base-command" },
                        "srv": { "type": "http", "url": "https://example.com/mcp" }
                    }
                })
            );
        }
    }
}

#[test]
fn pack_writes_the_lsp_file_of_each_client() {
    let f = Fixture::new();
    f.command_for_clients("demo").assert().success();
    for (client, file, other, expected) in [
        (
            "claude",
            ".lsp.json",
            "lsp.json",
            serde_json::json!({
                "base-lsp": {"command": "base-lsp-command", "extensionToLanguage": {".base": "base"}}
            }),
        ),
        (
            "copilot",
            "lsp.json",
            ".lsp.json",
            serde_json::json!({
                "lspServers": {"base-lsp": {"command": "base-lsp-command", "fileExtensions": {".base": "base"}}}
            }),
        ),
    ] {
        let target = f.out.join(format!("demo-{client}"));
        let doc: serde_json::Value =
            serde_json::from_slice(&fs::read(target.join(file)).unwrap()).unwrap();
        assert_eq!(doc, expected);
        assert!(!target.join(other).exists());
    }
}

#[test]
fn pack_of_a_bundle_without_lsp_servers_writes_no_lsp_file() {
    let f = Fixture::new();
    f.command_for_clients("norules").assert().success();
    for client in ["claude", "copilot"] {
        let target = f.out.join(format!("norules-{client}"));
        assert!(target.is_dir());
        assert!(!target.join(".lsp.json").exists());
        assert!(!target.join("lsp.json").exists());
    }
}

#[test]
fn pack_of_a_bundle_with_lsp_servers_and_no_rules_writes_no_init() {
    let f = Fixture::new();
    let bundle = f.registry.join("norules.bundle.yaml");
    let text = fs::read_to_string(&bundle).unwrap();
    fs::write(
        bundle,
        format!(
            "{text}lsps:\n  ra:\n    command: rust-analyzer\n    extensions: {{ .rs: rust }}\n"
        ),
    )
    .unwrap();
    f.command_for_clients("norules").assert().success();
    for (client, file) in [("claude", ".lsp.json"), ("copilot", "lsp.json")] {
        let target = f.out.join(format!("norules-{client}"));
        assert!(target.join(file).is_file());
        assert!(!target.join("skills/init/SKILL.md").exists());
        assert!(!target.join("hooks").exists());
        assert!(!target.join("hooks.json").exists());
    }
}

#[test]
fn pack_refuses_one_lsp_server_name_with_different_content_in_the_closure() {
    let f = Fixture::new();
    let bundle = f.registry.join("demo.bundle.yaml");
    let text = fs::read_to_string(&bundle).unwrap();
    fs::write(bundle, format!("{text}lsps:\n  base-lsp:\n    command: other-command\n    extensions: {{ .base: base }}\n")).unwrap();
    f.command_for_clients("demo").assert().code(1).stderr(predicates::str::contains(
        "LSP server `base-lsp` is declared with different content by the bundles `base` and `demo`"
    ));
    assert!(!f.out.exists());
}

#[test]
fn pack_refuses_a_bundle_with_an_invalid_lsp_entry() {
    let f = Fixture::new();
    let bundle = f.registry.join("norules.bundle.yaml");
    let text = fs::read_to_string(&bundle).unwrap();
    fs::write(
        bundle,
        format!("{text}lsps:\n  ra:\n    command: rust-analyzer\n    extensions: {{ rs: rust }}\n"),
    )
    .unwrap();
    f.command_for_clients("norules")
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "lsp `ra`: extension `rs` must start with a dot",
        ));
    assert!(!f.out.exists());
}

fn command_frontmatter(path: &Path) -> (serde_yaml_ng::Mapping, String) {
    let text = fs::read_to_string(path).unwrap();
    let (yaml, body) = upskill::parse::frontmatter::split(&text).unwrap();
    (serde_yaml_ng::from_str(yaml).unwrap(), body.to_string())
}

fn write_bundle(f: &Fixture, name: &str, fields: &str) {
    fs::write(
        f.registry.join(format!("{name}.bundle.yaml")),
        format!("schema: 1\nname: {name}\ndescription: Test bundle\n{fields}\n"),
    )
    .unwrap();
}

fn write_command(f: &Fixture, name: &str, fields: &str, body: &str) {
    let dir = f.registry.join("items").join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("COMMAND.md"),
        format!("---\nname: {name}\ndescription: Test command\n{fields}---\n\n{body}\n"),
    )
    .unwrap();
}

fn assert_command_error(f: &Fixture, bundle: &str, messages: &[&str]) {
    for flags in [&[][..], &["--claude"][..], &["--copilot"][..]] {
        let mut command = f.command_for_clients(bundle);
        command.args(flags);
        let mut assertion = command.assert().code(1);
        for message in messages {
            assertion = assertion.stderr(predicates::str::contains(*message));
        }
        assert!(!f.out.exists());
    }
}

#[test]
fn pack_renders_a_command_as_a_user_only_skill_for_both_clients() {
    let f = Fixture::new();
    f.command_for_clients("demo").assert().success();
    for (client, keys) in [
        (
            "claude",
            vec![
                "name",
                "description",
                "argument-hint",
                "disable-model-invocation",
            ],
        ),
        (
            "copilot",
            vec!["name", "description", "disable-model-invocation"],
        ),
    ] {
        let dir = f.out.join(format!("demo-{client}/skills/release-notes"));
        let (map, body) = command_frontmatter(&dir.join("SKILL.md"));
        assert_eq!(
            map.keys().map(|k| k.as_str().unwrap()).collect::<Vec<_>>(),
            keys
        );
        assert_eq!(map["name"].as_str(), Some("release-notes"));
        assert_eq!(
            map["description"].as_str(),
            Some("Write the release notes of the current branch")
        );
        assert_eq!(map["disable-model-invocation"].as_bool(), Some(true));
        if client == "claude" {
            assert_eq!(map["argument-hint"].as_str(), Some("[version]"));
        }
        assert!(body.contains("$ARGUMENTS"));
        assert_eq!(
            fs::read(dir.join("template.md")).unwrap(),
            fs::read(f.registry.join("items/release-notes/template.md")).unwrap()
        );
    }
}

#[test]
fn pack_writes_no_commands_directory() {
    let f = Fixture::new();
    f.command_for_clients("demo").assert().success();
    for client in ["claude", "copilot"] {
        assert!(
            tree(&f.out.join(format!("demo-{client}")))
                .keys()
                .all(|path| !path.starts_with("commands"))
        );
    }
}

#[test]
fn pack_renders_init_as_a_user_only_skill() {
    let f = Fixture::new();
    f.command("demo").assert().success();
    let (map, body) = command_frontmatter(&f.target().join("skills/init/SKILL.md"));
    assert_eq!(map["name"].as_str(), Some("init"));
    assert_eq!(
        map["description"].as_str(),
        Some("Install the rules of the demo bundle into this repository")
    );
    assert_eq!(map["disable-model-invocation"].as_bool(), Some(true));
    assert!(body.contains("upskill add driftsys/demo@v1 demo --kind rule --project --claude"));
}

#[test]
fn pack_refuses_a_command_and_a_skill_with_one_name_in_the_closure() {
    let f = Fixture::new();
    write_bundle(
        &f,
        "clash",
        "requires: [{name: base}]\nitems:\n  commands: [beta]",
    );
    assert_command_error(
        &f,
        "clash",
        &[
            "command `beta` and skill `beta` have one name in the bundle closure; both would be written to skills/beta/SKILL.md",
        ],
    );
}

#[test]
fn pack_refuses_a_command_named_init_in_a_closure_with_rules() {
    let f = Fixture::new();
    write_command(&f, "init", "", "Author init.");
    write_bundle(
        &f,
        "initcmd",
        "requires: [{name: base}]\nitems:\n  commands: [init]",
    );
    assert_command_error(
        &f,
        "initcmd",
        &[
            "command `init` cannot be packed with rules: `pack` writes the init command to skills/init/SKILL.md",
        ],
    );
}

#[test]
fn pack_refuses_a_skill_named_init_in_a_closure_with_rules() {
    let f = Fixture::new();
    let dir = f.registry.join("items/init");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: init\ndescription: Author init\n---\n\nAuthor init.\n",
    )
    .unwrap();
    write_bundle(
        &f,
        "initskill",
        "requires: [{name: base}]\nitems:\n  skills: [init]",
    );
    assert_command_error(
        &f,
        "initskill",
        &[
            "skill `init` cannot be packed with rules: `pack` writes the init command to skills/init/SKILL.md",
        ],
    );
}

#[test]
fn pack_accepts_a_command_named_init_in_a_closure_without_rules() {
    let f = Fixture::new();
    write_command(&f, "init", "", "Author init.");
    let path = f.registry.join("items/init/COMMAND.md");
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("Test command", "Author init"),
    )
    .unwrap();
    write_bundle(&f, "ownit", "items:\n  commands: [init]");
    f.command("ownit").assert().success();
    let (map, _) = command_frontmatter(&f.out.join("ownit-claude/skills/init/SKILL.md"));
    assert_eq!(map["description"].as_str(), Some("Author init"));
}

#[test]
fn pack_refuses_one_command_named_by_two_bundles() {
    let f = Fixture::new();
    write_bundle(
        &f,
        "dup",
        "requires: [{name: base}]\nitems:\n  commands: [release-notes]",
    );
    assert_command_error(
        &f,
        "dup",
        &["command `release-notes` is named by both bundles `base` and `dup`"],
    );
}

#[test]
fn pack_refuses_a_listed_command_without_a_command_md() {
    let f = Fixture::new();
    write_bundle(&f, "missing", "items:\n  commands: [missing]");
    assert_command_error(
        &f,
        "missing",
        &[
            "command `missing` of bundle `missing`",
            "no directory `missing` with a COMMAND.md",
        ],
    );
}

#[test]
fn pack_refuses_a_command_whose_name_differs_from_its_directory() {
    let f = Fixture::new();
    write_command(&f, "misnamed", "", "Write notes.");
    let path = f.registry.join("items/misnamed/COMMAND.md");
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("name: misnamed", "name: other"),
    )
    .unwrap();
    write_bundle(&f, "misnamed", "items:\n  commands: [misnamed]");
    assert_command_error(
        &f,
        "misnamed",
        &[
            "command `misnamed` of bundle `misnamed`",
            "`name: other` must equal the directory name `misnamed`",
        ],
    );
}

#[test]
fn pack_copies_neither_a_co_located_rule_nor_an_ignored_file() {
    let f = Fixture::new();
    write_command(&f, "notes", "ignore: [draft.md]\n", "Write notes.");
    let dir = f.registry.join("items/notes");
    fs::write(
        dir.join("RULE.md"),
        "---\nname: notes\ndescription: Notes rules\n---\n\nRules.\n",
    )
    .unwrap();
    fs::write(dir.join("draft.md"), "Draft.").unwrap();
    fs::write(dir.join("ref.md"), "Reference.").unwrap();
    write_bundle(&f, "notes", "items:\n  commands: [notes]");
    f.command_for_clients("notes").assert().success();
    for client in ["claude", "copilot"] {
        let files = tree(&f.out.join(format!("notes-{client}/skills/notes")));
        assert_eq!(
            files.keys().cloned().collect::<Vec<_>>(),
            vec![PathBuf::from("SKILL.md"), PathBuf::from("ref.md")]
        );
        assert_eq!(files[Path::new("ref.md")], b"Reference.");
    }
}

#[test]
fn pack_processes_client_directives_in_a_command_body() {
    let f = Fixture::new();
    write_command(
        &f,
        "notes",
        "",
        "<!-- @client:claude -->\nClaude only\n<!-- @endclient -->\n\n<!-- @client:copilot -->\nCopilot only\n<!-- @endclient -->",
    );
    write_bundle(&f, "notes", "items:\n  commands: [notes]");
    f.command_for_clients("notes").assert().success();
    for (client, present, absent) in [
        ("claude", "Claude only", "Copilot only"),
        ("copilot", "Copilot only", "Claude only"),
    ] {
        let text = fs::read_to_string(f.out.join(format!("notes-{client}/skills/notes/SKILL.md")))
            .unwrap();
        assert!(text.contains(present));
        assert!(!text.contains(absent));
        assert!(!text.contains("@client"));
    }
}
