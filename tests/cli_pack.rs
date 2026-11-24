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
    assert!(!target.join("commands").exists());
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
            "Wrote {} (8 files)\nWrote {} (8 files)\n",
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
    assert!(!copilot.join("commands").exists());
    assert!(!copilot.join("hooks.json").exists());
    let claude = f.out.join("claudeonly-claude");
    assert!(claude.join("commands/init.md").is_file());
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
            fs::read_to_string(f.out.join(format!("demo-{client}/commands/init.md"))).unwrap();
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
        assert!(!target.join("commands/init.md").exists());
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
