mod common;

use predicates::prelude::PredicateBooleanExt;
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
    market: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let registry = tmp.path().join("registry");
        let home = tmp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let source = Path::new("tests/fixtures/pack/registry");
        for (rel, bytes) in tree(source) {
            let dest = registry.join(&rel);
            fs::create_dir_all(dest.parent().unwrap()).unwrap();
            fs::write(&dest, bytes).unwrap();
            // Keep the source mode, so a script stays executable.
            let mode = fs::metadata(source.join(&rel)).unwrap().permissions();
            fs::set_permissions(&dest, mode).unwrap();
        }
        let out = tmp.path().join("out");
        let market = tmp.path().join("market");
        Self {
            _tmp: tmp,
            registry,
            home,
            out,
            market,
        }
    }

    fn command(&self, bundle: &str) -> assert_cmd::Command {
        let mut cmd = self.command_for_clients(bundle);
        cmd.arg("--claude");
        cmd
    }

    fn pack(&self, args: &[&str]) -> assert_cmd::Command {
        let mut cmd = common::upskill_cmd(&self.home);
        cmd.current_dir(&self.registry)
            .arg("pack")
            .args(args)
            .args(["--source", "driftsys/demo@v1", "--out"])
            .arg(&self.out);
        cmd
    }

    fn catalog(&self, args: &[&str]) -> assert_cmd::Command {
        let mut cmd = common::upskill_cmd(&self.home);
        cmd.current_dir(&self.registry)
            .arg("pack")
            .args(args)
            .args(["--source", "driftsys/demo@v1", "--out"])
            .arg(self.market.join("plugins"))
            .arg("--catalog")
            .arg(&self.market);
        cmd
    }

    fn new_catalog() -> Self {
        let fixture = Self::new();
        // These tests use the three bundles of the catalog golden files.
        for name in ["hookbase", "hookdemo"] {
            fs::remove_file(fixture.registry.join(format!("{name}.bundle.yaml"))).unwrap();
        }
        fixture
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
fn pack_of_several_bundles_writes_every_directory_in_name_order() {
    let f = Fixture::new();
    let output = f.pack(&["norules", "demo"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 4);
    for (line, name) in lines.iter().zip([
        "demo-claude",
        "demo-copilot",
        "norules-claude",
        "norules-copilot",
    ]) {
        assert!(line.starts_with(&format!("Wrote {} (", f.out.join(name).display())));
    }
    for name in ["demo-claude", "demo-copilot"] {
        assert_eq!(
            tree(&f.out.join(name)),
            tree(&Path::new("tests/fixtures/pack/expected").join(name))
        );
    }
    assert!(
        f.out
            .join("norules-claude/.claude-plugin/plugin.json")
            .is_file()
    );
    assert!(f.out.join("norules-copilot/plugin.json").is_file());
}

#[test]
fn pack_all_writes_every_bundle_of_the_registry() {
    let f = Fixture::new();
    f.pack(&["--all"]).assert().success();
    let mut names: Vec<_> = fs::read_dir(&f.out)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "base-claude",
            "base-copilot",
            "demo-claude",
            "demo-copilot",
            "hookbase-claude",
            "hookbase-copilot",
            "hookdemo-claude",
            "hookdemo-copilot",
            "norules-claude",
            "norules-copilot",
        ]
    );
}

#[test]
fn pack_refuses_two_bundle_files_with_one_name_and_keeps_the_output() {
    let f = Fixture::new();
    for dir in ["a", "b"] {
        let path = f.registry.join(dir);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("twin.bundle.yaml"),
            "schema: 1\nname: twin\ndescription: Twin\nitems:\n  skills: [alpha]\n",
        )
        .unwrap();
    }
    let kept = f.out.join("twin-claude");
    fs::create_dir_all(&kept).unwrap();
    fs::write(kept.join("keep"), "earlier").unwrap();
    for args in [vec!["--all"], vec!["twin"], vec!["demo"]] {
        let mut cmd = common::upskill_cmd(&f.home);
        cmd.current_dir(&f.registry)
            .arg("pack")
            .args(&args)
            .args([
                "--source",
                "driftsys/demo@v1",
                "--claude",
                "--force",
                "--out",
            ])
            .arg(&f.out)
            .assert()
            .code(1)
            .stderr(predicates::str::contains(
                "two bundle files declare the name 'twin'",
            ));
        assert_eq!(fs::read_to_string(kept.join("keep")).unwrap(), "earlier");
    }
}

#[test]
fn pack_of_a_repeated_bundle_name_writes_it_once() {
    let f = Fixture::new();
    let output = f.pack(&["demo", "demo", "--claude"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert_eq!(stdout.lines().count(), 1);
    assert!(stdout.starts_with(&format!("Wrote {} (", f.target().display())));
}

#[test]
fn pack_of_several_bundles_writes_nothing_when_one_is_unknown() {
    let f = Fixture::new();
    f.pack(&["demo", "nope"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("no bundle named 'nope'"));
    assert!(!f.out.exists());
}

#[test]
fn pack_of_several_bundles_checks_every_directory_before_it_writes_one() {
    let f = Fixture::new();
    let target = f.out.join("norules-copilot");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("keep"), "unchanged").unwrap();
    let before = tree(&f.out);
    f.pack(&["demo", "norules"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains(target.to_str().unwrap()))
        .stderr(predicates::str::contains("--force"));
    assert!(!f.target().exists());
    assert_eq!(tree(&f.out), before);
}

#[test]
fn pack_refuses_all_with_a_bundle_name_and_requires_one_of_them() {
    let f = Fixture::new();
    f.pack(&["--all", "demo"]).assert().code(2);
    f.pack(&[]).assert().code(2);
    assert!(!f.out.exists());
}

#[test]
fn pack_refuses_an_unsafe_name_among_several_bundles() {
    let f = Fixture::new();
    f.pack(&["demo", "a;b"]).assert().code(2);
    assert!(!f.out.exists());
}

#[test]
fn pack_all_of_a_directory_without_bundles_fails() {
    let f = Fixture::new();
    let empty = f._tmp.path().join("empty");
    fs::create_dir(&empty).unwrap();
    f.pack(&["--all"])
        .current_dir(&empty)
        .assert()
        .code(1)
        .stderr(predicates::str::contains("no bundle in"));
    assert!(!f.out.exists());
}

#[test]
fn pack_all_refuses_a_registry_bundle_with_an_unsafe_name() {
    let f = Fixture::new();
    fs::write(
        f.registry.join("bad name.bundle.yaml"),
        "schema: 1\nname: bad name\ndescription: d\nitems: {}\n",
    )
    .unwrap();
    f.pack(&["--all"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("bundle 'bad name'"))
        .stderr(predicates::str::contains("bundle name must be nonempty"));
    assert!(!f.out.exists());
}

#[test]
fn pack_all_below_the_registry_twice_writes_equal_bytes() {
    let f = Fixture::new();
    let mut cmd = common::upskill_cmd(&f.home);
    cmd.current_dir(&f.registry).args([
        "pack",
        "--all",
        "--source",
        "driftsys/demo@v1",
        "--out",
        "plugins",
    ]);
    cmd.assert().success();
    let before = tree(&f.registry.join("plugins"));
    cmd.arg("--force").assert().success();
    assert_eq!(tree(&f.registry.join("plugins")), before);
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

#[cfg(unix)]
#[test]
fn pack_preserves_command_resource_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    fs::set_permissions(
        f.registry.join("items/release-notes/template.md"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    f.command_for_clients("demo").assert().success();
    for client in ["claude", "copilot"] {
        let path = f
            .out
            .join(format!("demo-{client}/skills/release-notes/template.md"));
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o755,
            "{client}"
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
    write_command(&f, "beta", "", "Write notes.");
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

#[test]
fn pack_accepts_a_command_listed_twice_in_one_bundle() {
    let f = Fixture::new();
    write_command(&f, "release-notes", "", "Write notes.");
    write_bundle(
        &f,
        "twice",
        "items:\n  commands: [release-notes, release-notes]",
    );
    f.command("twice").assert().success();
    assert!(
        f.out
            .join("twice-claude/skills/release-notes/SKILL.md")
            .is_file()
    );
}

fn git(home: &Path, dir: &Path, args: &[&str]) {
    let mut cmd = std::process::Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    let output = cmd
        .current_dir(dir)
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", home.join("empty-gitconfig"))
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pack_reads_a_command_of_a_bundle_from_a_git_source() {
    let f = Fixture::new();
    let upstream = f._tmp.path().join("upstream");
    fs::create_dir_all(upstream.join("items/release-notes")).unwrap();
    fs::write(
        upstream.join("items/release-notes/COMMAND.md"),
        "---\nname: release-notes\ndescription: Command held by the git source\n---\n\nWrite notes from the git source.\n",
    )
    .unwrap();
    fs::write(
        upstream.join("base.bundle.yaml"),
        "schema: 1\nname: base\ndescription: Test bundle\nitems:\n  commands: [release-notes]\n",
    )
    .unwrap();
    git(&f.home, &upstream, &["init", "-q", "--initial-branch=main"]);
    git(&f.home, &upstream, &["add", "."]);
    git(&f.home, &upstream, &["commit", "-q", "-m", "init"]);
    let remote = "https://example.invalid/team/base";
    write_bundle(
        &f,
        "entry",
        &format!("requires: [{{name: base, source: \"{remote}\"}}]\nitems: {{}}"),
    );
    let mut cmd = f.command("entry");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", f.home.join("empty-gitconfig"))
        .env("GIT_CONFIG_COUNT", "2")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}.insteadOf", upstream.display()),
        )
        .env("GIT_CONFIG_VALUE_0", format!("{remote}.git"))
        .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_1", "always")
        .env("GIT_ALLOW_PROTOCOL", "file")
        .env("GIT_TERMINAL_PROMPT", "0")
        .assert()
        .success();
    let rendered =
        fs::read_to_string(f.out.join("entry-claude/skills/release-notes/SKILL.md")).unwrap();
    assert!(
        rendered.contains("Command held by the git source"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Write notes from the git source."),
        "{rendered}"
    );
}

impl Fixture {
    fn hook(&self, directory: &str, fields: &str, body: &str) {
        let dir = self.registry.join("items").join(directory);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("HOOK.md"),
            format!("---\nschema: 1\ndescription: Test hook\n{fields}\n---\n\n{body}\n"),
        )
        .unwrap();
    }

    fn hook_bundle(&self, name: &str, hooks: &str, extra: &str) {
        fs::write(
            self.registry.join(format!("{name}.bundle.yaml")),
            format!(
                "schema: 1\nname: {name}\ndescription: Test bundle\nitems:\n  hooks: [{hooks}]\n{extra}\n"
            ),
        )
        .unwrap();
    }

    fn hook_json(&self, bundle: &str, client: &str) -> serde_json::Value {
        let file = if client == "claude" {
            "hooks/hooks.json"
        } else {
            "hooks.json"
        };
        read_json(&self.out.join(format!("{bundle}-{client}")).join(file))
    }

    fn context_bytes(&self, bundle: &str, client: &str, event: &str) -> Vec<u8> {
        fs::read(
            self.out
                .join(format!("{bundle}-{client}/hooks/context/{event}.json")),
        )
        .unwrap()
    }

    fn context_text(&self, bundle: &str, client: &str, event: &str) -> String {
        let value: serde_json::Value =
            serde_json::from_slice(&self.context_bytes(bundle, client, event)).unwrap();
        let output = if client == "claude" {
            &value["hookSpecificOutput"]
        } else {
            &value
        };
        output["additionalContext"].as_str().unwrap().to_string()
    }

    fn hook_error(&self, bundle: &str, messages: &[&str]) {
        let mut assertion = self.command_for_clients(bundle).assert().code(1);
        for message in messages {
            assertion = assertion.stderr(predicates::str::contains(*message));
        }
        assert!(!self.out.exists());
    }
}

/// Write a command-hook script with the executable bit that `pack` requires.
fn write_script(path: &Path) {
    fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn pack_writes_the_hook_plugin_directories() {
    let f = Fixture::new();
    f.command_for_clients("hookdemo").assert().success();
    for client in ["claude", "copilot"] {
        let dir = format!("hookdemo-{client}");
        assert_eq!(
            tree(&f.out.join(&dir)),
            tree(&Path::new("tests/fixtures/pack/expected").join(dir))
        );
    }
}

#[test]
fn pack_merges_the_context_hooks_of_one_event_in_closure_order() {
    let f = Fixture::new();
    f.command_for_clients("hookdemo").assert().success();
    for client in ["claude", "copilot"] {
        assert_eq!(
            f.context_text("hookdemo", client, "session-start"),
            "Follow the team conventions.\n\nRead `CONVENTIONS.md` before you edit a file.\n\nRun the checks before you commit."
        );
    }
}

#[test]
fn pack_gives_session_start_the_default_matcher_in_claude_code_only() {
    let f = Fixture::new();
    f.command_for_clients("hookdemo").assert().success();
    assert_eq!(
        f.hook_json("hookdemo", "claude")["hooks"]["SessionStart"][1]["matcher"],
        "startup|clear|compact"
    );
    assert!(
        f.hook_json("hookdemo", "copilot")["hooks"]["sessionStart"][1]
            .get("matcher")
            .is_none()
    );
}

#[test]
fn pack_writes_one_context_file_for_each_distinct_matcher() {
    let f = Fixture::new();
    f.hook(
        "after-edit",
        "events:\n  post-tool: { matcher: edit }",
        "After an edit.",
    );
    f.hook(
        "after-shell",
        "events:\n  post-tool: { matcher: shell }",
        "After a shell command.",
    );
    f.hook_bundle("twogroups", "after-shell, after-edit", "");
    f.command_for_clients("twogroups").assert().success();
    for client in ["claude", "copilot"] {
        assert_eq!(
            f.context_text("twogroups", client, "post-tool"),
            "After a shell command."
        );
        assert_eq!(
            f.context_text("twogroups", client, "post-tool-2"),
            "After an edit."
        );
        let hooks = f.hook_json("twogroups", client);
        let event = if client == "claude" {
            "PostToolUse"
        } else {
            "postToolUse"
        };
        let entries = &hooks["hooks"][event];
        assert_eq!(entries[0]["matcher"], "shell");
        assert_eq!(entries[1]["matcher"], "edit");
        if client == "claude" {
            assert_eq!(
                entries[1]["hooks"][0]["command"],
                "cat \"${CLAUDE_PLUGIN_ROOT}/hooks/context/post-tool-2.json\""
            );
        } else {
            assert_eq!(entries[1]["bash"], "cat hooks/context/post-tool-2.json");
        }
    }
}

#[test]
fn pack_writes_a_string_matcher_for_both_clients() {
    let f = Fixture::new();
    f.hook(
        "tool",
        "events:\n  post-tool: { matcher: edit }",
        "After an edit.",
    );
    f.hook_bundle("stringmatcher", "tool", "");
    f.command_for_clients("stringmatcher").assert().success();
    assert_eq!(
        f.hook_json("stringmatcher", "claude")["hooks"]["PostToolUse"][0]["matcher"],
        "edit"
    );
    assert_eq!(
        f.hook_json("stringmatcher", "copilot")["hooks"]["postToolUse"][0]["matcher"],
        "edit"
    );
}

#[test]
fn pack_writes_a_map_matcher_only_for_the_named_client() {
    let f = Fixture::new();
    f.hook(
        "tool",
        "events:\n  post-tool: { matcher: { claude: Edit } }",
        "After an edit.",
    );
    // The reminder also gives Copilot a hook file so the missing key is checked.
    f.hook_bundle("mapmatcher", "tool", "requires:\n  - name: hookbase");
    f.command_for_clients("mapmatcher").assert().success();
    assert_eq!(
        f.hook_json("mapmatcher", "claude")["hooks"]["PostToolUse"][0]["matcher"],
        "Edit"
    );
    assert!(
        f.hook_json("mapmatcher", "copilot")["hooks"]
            .get("postToolUse")
            .is_none()
    );
    assert!(
        f.out
            .join("mapmatcher-claude/hooks/context/post-tool.json")
            .is_file()
    );
    assert!(
        !f.out
            .join("mapmatcher-copilot/hooks/context/post-tool.json")
            .exists()
    );
}

#[test]
fn pack_of_a_bundle_with_hooks_and_no_rules_writes_a_hook_file_without_the_notice() {
    let f = Fixture::new();
    f.command_for_clients("hookbase").assert().success();
    for client in ["claude", "copilot"] {
        assert!(
            !f.hook_json("hookbase", client)
                .to_string()
                .contains("upskill")
        );
        assert!(
            !f.out
                .join(format!("hookbase-{client}/skills/init"))
                .exists()
        );
    }
    assert_eq!(
        f.hook_json("hookbase", "claude")["hooks"]["SessionStart"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn pack_context_hooks_do_not_call_upskill() {
    let f = Fixture::new();
    f.command_for_clients("hookbase").assert().success();
    for client in ["claude", "copilot"] {
        for bytes in tree(&f.out.join(format!("hookbase-{client}"))).values() {
            assert!(!String::from_utf8_lossy(bytes).contains("upskill"));
        }
    }
}

#[test]
fn pack_processes_client_directives_in_context_text() {
    let f = Fixture::new();
    f.hook("directed", "events:\n  session-start:",
        "<!-- @client:claude -->\nClaude only\n<!-- @endclient -->\n\n<!-- @client:copilot -->\nCopilot only\n<!-- @endclient -->");
    f.hook_bundle("directives", "directed", "");
    f.command_for_clients("directives").assert().success();
    assert_eq!(
        f.context_text("directives", "claude", "session-start"),
        "Claude only"
    );
    assert_eq!(
        f.context_text("directives", "copilot", "session-start"),
        "Copilot only"
    );
}

#[test]
fn pack_writes_no_entry_for_a_client_whose_context_text_is_empty() {
    let f = Fixture::new();
    f.hook(
        "directed",
        "events:\n  session-start:",
        "<!-- @client:claude -->\nClaude only\n<!-- @endclient -->",
    );
    f.hook_bundle("directives", "directed", "");
    f.command_for_clients("directives").assert().success();
    assert!(f.out.join("directives-claude/hooks/hooks.json").is_file());
    assert!(!f.out.join("directives-copilot/hooks.json").exists());
    assert!(!f.out.join("directives-copilot/hooks").exists());
}

#[test]
fn pack_escapes_context_text_as_json() {
    let f = Fixture::new();
    let text = "Say \"hi\"\nto C:\\temp and to Zoë.";
    f.hook("escaped", "events:\n  session-start:", text);
    f.hook_bundle("escaped", "escaped", "");
    f.command_for_clients("escaped").assert().success();
    for client in ["claude", "copilot"] {
        assert!(
            f.context_bytes("escaped", client, "session-start")
                .is_ascii()
        );
        assert_eq!(f.context_text("escaped", client, "session-start"), text);
    }
}

#[test]
fn pack_refuses_an_invalid_hook() {
    let f = Fixture::new();
    f.hook("bad", "events:\n  stop:", "Text.");
    f.hook_bundle("badhook", "bad", "");
    f.hook_error(
        "badhook",
        &[
            "hook `bad` of bundle `badhook`",
            "event `stop` has no `run`",
        ],
    );
}

#[test]
fn pack_refuses_a_listed_hook_without_a_hook_md() {
    let f = Fixture::new();
    f.hook_bundle("missinghook", "missing", "");
    f.hook_error(
        "missinghook",
        &["no directory with a HOOK.md for the hook `missing`"],
    );
}

#[test]
fn pack_refuses_one_hook_named_by_two_bundles() {
    let f = Fixture::new();
    f.hook_bundle("dup", "reminder", "requires:\n  - name: hookbase");
    f.hook_error(
        "dup",
        &["hook `reminder` is named by both bundles `hookbase` and `dup`"],
    );
}

#[test]
fn pack_refuses_an_unclosed_directive_of_a_hook_skipped_for_one_client() {
    let f = Fixture::new();
    f.hook(
        "edits",
        "events:\n  post-tool:\n    matcher:\n      claude: Edit",
        "<!-- @client:claude -->\n\nText.",
    );
    f.hook_bundle("baddirective", "edits", "");
    f.command_for_clients("baddirective")
        .arg("--copilot")
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "hook `edits` of bundle `baddirective`",
        ))
        .stderr(predicates::str::contains("@client"));
    assert!(!f.out.exists());
}

#[test]
fn pack_merges_a_hook_listed_twice_in_one_bundle_once() {
    let f = Fixture::new();
    f.hook("again", "events:\n  session-start:", "Once only.");
    f.hook_bundle("twice", "again, again", "");
    f.command_for_clients("twice").assert().success();
    for client in ["claude", "copilot"] {
        assert_eq!(
            f.context_text("twice", client, "session-start"),
            "Once only."
        );
    }
}

#[test]
fn pack_refuses_a_hook_named_context() {
    let f = Fixture::new();
    f.hook("context", "events:\n  session-start:", "Text.");
    f.hook_bundle("reserved", "context", "");
    f.hook_error("reserved", &["the hook name `context` is reserved"]);
}

#[test]
fn pack_refuses_a_hook_name_outside_the_name_rule() {
    let f = Fixture::new();
    f.hook("upper", "name: Upper\nevents:\n  session-start:", "Text.");
    f.hook_bundle("upperhook", "Upper", "");
    f.hook_error(
        "upperhook",
        &["hook name `Upper` must consist of lowercase letters"],
    );
}

#[test]
fn pack_writes_a_command_hook_entry_that_sets_upskill_client() {
    let f = Fixture::new();
    f.command_for_clients("hookdemo").assert().success();
    let claude = f.hook_json("hookdemo", "claude");
    let entry = &claude["hooks"]["PostToolUse"][0];
    assert_eq!(entry["matcher"], "Edit|Write");
    assert_eq!(
        entry["hooks"][0]["command"],
        "UPSKILL_CLIENT=claude \"${CLAUDE_PLUGIN_ROOT}/hooks/edit-checks/check.sh\""
    );
    assert_eq!(entry["hooks"][0]["timeout"], 10);
    let copilot = f.hook_json("hookdemo", "copilot");
    let entry = &copilot["hooks"]["postToolUse"][0];
    assert_eq!(entry["matcher"], "edit");
    assert_eq!(
        entry["bash"],
        "UPSKILL_CLIENT=copilot ./hooks/edit-checks/check.sh"
    );
    assert_eq!(entry["timeoutSec"], 10);
    assert!(entry.get("powershell").is_none());
}

#[test]
fn pack_orders_notice_then_context_then_command_hooks() {
    let f = Fixture::new();
    f.hook(
        "starter",
        "events:\n  session-start: { run: start.sh }",
        "Starts the session.",
    );
    write_script(&f.registry.join("items/starter/start.sh"));
    f.hook_bundle("ordered", "starter", "requires:\n  - name: hookdemo");
    f.command_for_clients("ordered").assert().success();
    let claude = f.hook_json("ordered", "claude");
    let entries = claude["hooks"]["SessionStart"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0]["matcher"], "startup");
    assert!(
        entries[0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .contains("upskill update --notice")
    );
    assert_eq!(entries[1]["matcher"], "startup|clear|compact");
    assert_eq!(
        entries[1]["hooks"][0]["command"],
        "cat \"${CLAUDE_PLUGIN_ROOT}/hooks/context/session-start.json\""
    );
    assert!(entries[2].get("matcher").is_none());
    assert_eq!(
        entries[2]["hooks"][0]["command"],
        "UPSKILL_CLIENT=claude \"${CLAUDE_PLUGIN_ROOT}/hooks/starter/start.sh\""
    );
    let copilot = f.hook_json("ordered", "copilot");
    let entries = copilot["hooks"]["sessionStart"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    assert!(
        entries[0]["bash"]
            .as_str()
            .unwrap()
            .contains("upskill update --notice")
    );
    assert_eq!(entries[1]["bash"], "cat hooks/context/session-start.json");
    assert_eq!(
        entries[2]["bash"],
        "UPSKILL_CLIENT=copilot ./hooks/starter/start.sh"
    );
}

#[test]
fn pack_of_a_bundle_with_only_a_command_hook_writes_a_hook_file() {
    let f = Fixture::new();
    f.hook(
        "starter",
        "events:\n  session-start: { run: start.sh }",
        "Starts the session.",
    );
    write_script(&f.registry.join("items/starter/start.sh"));
    f.hook_bundle("commandonly", "starter", "");
    f.command_for_clients("commandonly").assert().success();
    for (client, event) in [("claude", "SessionStart"), ("copilot", "sessionStart")] {
        let json = f.hook_json("commandonly", client);
        let hooks = json["hooks"].as_object().unwrap();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[event].as_array().unwrap().len(), 1);
        assert!(
            !f.out
                .join(format!("commandonly-{client}/hooks/context"))
                .exists()
        );
    }
}

#[test]
fn pack_does_not_copy_a_hook_without_a_command_hook() {
    let f = Fixture::new();
    f.command_for_clients("hookdemo").assert().success();
    for client in ["claude", "copilot"] {
        let root = f.out.join(format!("hookdemo-{client}"));
        assert!(!root.join("hooks/reminder").exists());
        assert!(!tree(&root).keys().any(|rel| rel.ends_with("notes.txt")));
    }
}

#[test]
fn pack_copies_neither_an_entry_file_nor_an_ignored_file_of_a_command_hook() {
    let f = Fixture::new();
    f.hook(
        "tool",
        "ignore: [draft.md]\nevents:\n  post-tool: { run: go.sh }",
        "Checks the file.",
    );
    let dir = f.registry.join("items/tool");
    fs::write(
        dir.join("RULE.md"),
        "---\nname: tool\ndescription: Test rule\n---\n\nRule text.\n",
    )
    .unwrap();
    fs::write(dir.join("draft.md"), "Draft.\n").unwrap();
    write_script(&dir.join("go.sh"));
    f.hook_bundle("commandonly", "tool", "");
    f.command_for_clients("commandonly").assert().success();
    for client in ["claude", "copilot"] {
        assert_eq!(
            tree(&f.out.join(format!("commandonly-{client}/hooks/tool"))),
            BTreeMap::from([(PathBuf::from("go.sh"), b"#!/bin/sh\nexit 0\n".to_vec())])
        );
    }
}

#[test]
fn pack_refuses_a_run_file_matched_by_ignore() {
    let f = Fixture::new();
    f.hook(
        "tool",
        "ignore: [go.sh]\nevents:\n  post-tool: { run: go.sh }",
        "Checks the file.",
    );
    write_script(&f.registry.join("items/tool/go.sh"));
    f.hook_bundle("commandonly", "tool", "");
    f.hook_error(
        "commandonly",
        &[
            "hook `tool` of bundle `commandonly`",
            "event `post-tool`: run file `go.sh` is not copied by pack: it matches `ignore`",
        ],
    );
}

#[test]
#[cfg(unix)]
fn pack_keeps_the_mode_of_a_command_hook_script() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.command_for_clients("hookdemo").assert().success();
    for client in ["claude", "copilot"] {
        let path = f
            .out
            .join(format!("hookdemo-{client}/hooks/edit-checks/check.sh"));
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }
}

#[test]
fn pack_writes_a_command_hook_only_for_the_client_of_its_map_matcher() {
    let f = Fixture::new();
    f.hook(
        "tool",
        "events:\n  post-tool: { matcher: { copilot: edit }, run: go.sh }",
        "Checks the file.",
    );
    write_script(&f.registry.join("items/tool/go.sh"));
    f.hook_bundle("commandonly", "tool", "");
    f.command_for_clients("commandonly").assert().success();
    let copilot = f.hook_json("commandonly", "copilot");
    assert_eq!(copilot["hooks"]["postToolUse"].as_array().unwrap().len(), 1);
    assert!(f.out.join("commandonly-copilot/hooks/tool/go.sh").is_file());
    assert!(!f.out.join("commandonly-claude/hooks/hooks.json").exists());
    assert!(!f.out.join("commandonly-claude/hooks/tool").exists());
}

#[test]
fn pack_refuses_an_uncopied_run_file_even_for_an_excluded_client() {
    for (run, ignored, message) in [
        (
            "missing.sh",
            false,
            "`run: missing.sh` names no file in the item directory",
        ),
        ("HOOK.md", false, "run file `HOOK.md` is not copied by pack"),
        ("go.sh", true, "run file `go.sh` is not copied by pack"),
    ] {
        for client in ["--claude", "--copilot"] {
            let f = Fixture::new();
            let matcher = if client == "--claude" {
                "copilot: edit"
            } else {
                "claude: Edit"
            };
            let ignore = if ignored { "ignore: [go.sh]\n" } else { "" };
            f.hook(
                "tool",
                &format!(
                    "{ignore}events:\n  post-tool: {{ matcher: {{ {matcher} }}, run: {run} }}"
                ),
                "Checks the file.",
            );
            write_script(&f.registry.join("items/tool/go.sh"));
            f.hook_bundle("commandonly", "tool", "");
            f.command_for_clients("commandonly")
                .arg(client)
                .assert()
                .code(1)
                .stderr(predicates::str::contains(
                    "hook `tool` of bundle `commandonly`",
                ))
                .stderr(predicates::str::contains(message));
            assert!(!f.out.exists());
        }
    }
}

#[test]
fn pack_reads_a_hook_of_a_bundle_from_a_git_source() {
    let f = Fixture::new();
    let upstream = f._tmp.path().join("upstream");
    fs::create_dir_all(upstream.join("items/guidance")).unwrap();
    fs::write(
        upstream.join("items/guidance/HOOK.md"),
        "---\ndescription: Session guidance\nevents:\n  session-start:\n---\n\nRead the guide.\n",
    )
    .unwrap();
    fs::write(
        upstream.join("base.bundle.yaml"),
        "schema: 1\nname: base\ndescription: Test bundle\nitems:\n  hooks: [guidance]\n",
    )
    .unwrap();
    git(&f.home, &upstream, &["init", "-q", "--initial-branch=main"]);
    git(&f.home, &upstream, &["add", "."]);
    git(&f.home, &upstream, &["commit", "-q", "-m", "init"]);
    let remote = "https://example.invalid/team/base";
    write_bundle(
        &f,
        "entry",
        &format!("requires: [{{name: base, source: \"{remote}\"}}]\nitems: {{}}"),
    );
    let mut cmd = f.command_for_clients("entry");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", f.home.join("empty-gitconfig"))
        .env("GIT_CONFIG_COUNT", "2")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}.insteadOf", upstream.display()),
        )
        .env("GIT_CONFIG_VALUE_0", format!("{remote}.git"))
        .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_1", "always")
        .env("GIT_ALLOW_PROTOCOL", "file")
        .env("GIT_TERMINAL_PROMPT", "0")
        .assert()
        .success();
    for client in ["claude", "copilot"] {
        assert_eq!(
            f.context_text("entry", client, "session-start"),
            "Read the guide."
        );
    }
}

#[test]
fn pack_orders_the_command_hooks_of_one_event_by_the_bundle_list() {
    let f = Fixture::new();
    for name in ["zeta", "alpha"] {
        f.hook(
            name,
            "events:\n  post-tool: { run: go.sh }",
            "Checks the file.",
        );
        write_script(&f.registry.join(format!("items/{name}/go.sh")));
    }
    f.hook_bundle("listorder", "zeta, alpha", "");
    f.command_for_clients("listorder").assert().success();
    let claude = f.hook_json("listorder", "claude");
    let entries = claude["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert!(
        entries[0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .contains("hooks/zeta/go.sh")
    );
    assert!(
        entries[1]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .contains("hooks/alpha/go.sh")
    );
    let copilot = f.hook_json("listorder", "copilot");
    let entries = copilot["hooks"]["postToolUse"].as_array().unwrap();
    assert!(entries[0]["bash"].as_str().unwrap().contains("hooks/zeta/"));
    assert!(
        entries[1]["bash"]
            .as_str()
            .unwrap()
            .contains("hooks/alpha/")
    );
}

#[test]
fn pack_writes_one_entry_for_each_command_event_of_a_hook() {
    let f = Fixture::new();
    f.hook(
        "multi",
        "events:\n  session-start: { run: start.sh }\n  stop: { run: end.sh }",
        "Runs two scripts.",
    );
    write_script(&f.registry.join("items/multi/start.sh"));
    write_script(&f.registry.join("items/multi/end.sh"));
    f.hook_bundle("twoevents", "multi", "");
    f.command_for_clients("twoevents").assert().success();
    for (client, start, stop) in [
        ("claude", "SessionStart", "Stop"),
        ("copilot", "sessionStart", "agentStop"),
    ] {
        let hooks = f.hook_json("twoevents", client);
        assert_eq!(hooks["hooks"][start].as_array().unwrap().len(), 1);
        assert_eq!(hooks["hooks"][stop].as_array().unwrap().len(), 1);
        let dir = f.out.join(format!("twoevents-{client}/hooks/multi"));
        assert!(dir.join("start.sh").is_file());
        assert!(dir.join("end.sh").is_file());
    }
}

const NEW_CATALOG: &[&str] = &[
    "--catalog-name",
    "demo-market",
    "--catalog-owner",
    "driftsys",
];

fn catalog_golden(name: &str) -> Vec<u8> {
    fs::read(Path::new("tests/fixtures/pack/catalog").join(name)).unwrap()
}

fn existing_catalog(f: &Fixture, bytes: &[u8]) -> PathBuf {
    let path = f.market.join(".claude-plugin/marketplace.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn pack_catalog_writes_both_new_marketplace_files() {
    let f = Fixture::new_catalog();
    let output = f
        .catalog(&[&["demo", "norules"][..], NEW_CATALOG].concat())
        .assert()
        .success();
    for (path, golden) in [
        (".claude-plugin/marketplace.json", "new-claude.json"),
        (".github/plugin/marketplace.json", "new-copilot.json"),
    ] {
        let bytes = fs::read(f.market.join(path)).unwrap();
        assert_eq!(bytes, catalog_golden(golden));
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            value["plugins"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry.get("version").is_none())
        );
    }
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(
        &lines[lines.len() - 2..],
        [
            format!(
                "Wrote {}",
                f.market.join(".claude-plugin/marketplace.json").display()
            ),
            format!(
                "Wrote {}",
                f.market.join(".github/plugin/marketplace.json").display()
            )
        ]
    );
}

#[test]
fn pack_catalog_with_a_client_flag_writes_only_that_file() {
    for (flag, path, excluded) in [
        ("--claude", ".claude-plugin/marketplace.json", ".github"),
        (
            "--copilot",
            ".github/plugin/marketplace.json",
            ".claude-plugin",
        ),
    ] {
        let f = Fixture::new_catalog();
        f.catalog(&[&["demo", flag][..], NEW_CATALOG].concat())
            .assert()
            .success();
        assert!(f.market.join(path).is_file());
        assert!(!f.market.join(excluded).exists());
    }
}

#[test]
#[cfg(unix)]
fn pack_refuses_a_run_file_without_an_executable_bit() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.hook(
        "tool",
        "events:\n  post-tool: { run: go.sh }",
        "Checks the file.",
    );
    let script = f.registry.join("items/tool/go.sh");
    write_script(&script);
    fs::set_permissions(&script, fs::Permissions::from_mode(0o644)).unwrap();
    f.hook_bundle("commandonly", "tool", "");
    f.hook_error(
        "commandonly",
        &[
            "hook `tool` of bundle `commandonly`",
            "run file `go.sh` is not executable",
        ],
    );
}

#[test]
#[cfg(unix)]
fn pack_refuses_a_symbolic_link_as_a_run_file() {
    let f = Fixture::new();
    f.hook(
        "tool",
        "events:\n  post-tool: { run: go.sh }",
        "Checks the file.",
    );
    let real = f._tmp.path().join("real.sh");
    write_script(&real);
    std::os::unix::fs::symlink(&real, f.registry.join("items/tool/go.sh")).unwrap();
    f.hook_bundle("commandonly", "tool", "");
    f.hook_error(
        "commandonly",
        &[
            "hook `tool` of bundle `commandonly`",
            "run file `go.sh` is a symbolic link; pack does not copy it",
        ],
    );
}

#[test]
fn pack_refuses_a_run_value_with_a_path() {
    let f = Fixture::new();
    f.hook(
        "tool",
        "events:\n  post-tool: { run: ../go.sh }",
        "Checks the file.",
    );
    f.hook_bundle("commandonly", "tool", "");
    f.hook_error(
        "commandonly",
        &[
            "hook `tool` of bundle `commandonly`",
            "`run: ../go.sh` must be the name of one file in the item directory",
        ],
    );
}

#[test]
fn pack_reports_every_problem_of_a_hook_one_per_line() {
    let f = Fixture::new();
    f.hook(
        "tool",
        "events:\n  post-tool: { run: missing.sh }\n  stop: { matcher: edit }",
        "Checks the file.",
    );
    f.hook_bundle("commandonly", "tool", "");
    let assertion = f.command_for_clients("commandonly").assert().code(1);
    let stderr = String::from_utf8(assertion.get_output().stderr.clone()).unwrap();
    let lines: Vec<_> = stderr.lines().collect();
    let source = f.registry.join("items").join("tool").join("HOOK.md");
    let prefix = format!("{}: ", source.display());
    assert!(
        lines.iter().any(|line| line.contains(&format!(
            "{prefix}event `post-tool`: `run: missing.sh` names no file in the item directory"
        ))),
        "{stderr}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("a context hook on `stop` cannot have a `matcher`")),
        "{stderr}"
    );
    assert!(
        lines
            .iter()
            .all(|line| !(line.contains("missing.sh") && line.contains("`stop`"))),
        "{stderr}"
    );
    assert!(!f.out.exists());
}

#[test]
fn pack_keeps_the_notice_apart_from_a_context_entry_with_the_same_matcher() {
    let f = Fixture::new();
    f.hook(
        "starter",
        "events:\n  session-start: { matcher: startup }",
        "Starts the session.",
    );
    f.hook_bundle("samematcher", "starter", "requires:\n  - name: hookdemo");
    f.command_for_clients("samematcher").assert().success();
    let claude = f.hook_json("samematcher", "claude");
    let entries = claude["hooks"]["SessionStart"].as_array().unwrap();
    let startup: Vec<_> = entries
        .iter()
        .filter(|entry| entry["matcher"] == "startup")
        .collect();
    assert_eq!(startup.len(), 2, "{entries:?}");
    assert!(
        startup[0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .contains("upskill update --notice")
    );
    assert!(
        startup[1]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .starts_with("cat ")
    );
}

#[test]
fn pack_catalog_of_a_new_file_needs_name_and_owner() {
    for flags in [
        &[][..],
        &["--catalog-name", "m"][..],
        &["--catalog-owner", "o"][..],
    ] {
        let f = Fixture::new_catalog();
        f.catalog(&[&["demo"][..], flags].concat())
            .assert()
            .code(2)
            .stderr(predicates::str::contains("does not exist"))
            .stderr(predicates::str::contains("--catalog-name"));
        assert!(!f.market.exists());
    }
}

#[test]
fn pack_catalog_name_and_owner_require_catalog() {
    for flags in [
        &["--catalog-name", "m"][..],
        &["--catalog-owner", "o"][..],
        &["--catalog-name", "m", "--catalog-owner", "o"][..],
    ] {
        let f = Fixture::new_catalog();
        f.pack(&[&["demo"][..], flags].concat()).assert().code(2);
    }
}

#[test]
fn pack_catalog_refuses_an_out_outside_the_root() {
    let f = Fixture::new_catalog();
    f.pack(&[&["demo"][..], NEW_CATALOG].concat())
        .arg("--catalog")
        .arg(&f.market)
        .assert()
        .code(2)
        .stderr(predicates::str::contains("is not inside --catalog"));
    assert!(!f.out.exists());
    assert!(!f.market.exists());
}

#[test]
fn pack_catalog_keeps_top_level_fields_and_foreign_entries() {
    let f = Fixture::new_catalog();
    let path = existing_catalog(&f, &catalog_golden("existing-claude.json"));
    f.catalog(&["demo", "--claude"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Removed").not());
    assert_eq!(
        fs::read(path).unwrap(),
        catalog_golden("merged-claude.json")
    );
}

#[test]
fn pack_catalog_all_removes_stale_entries_inside_out() {
    let f = Fixture::new_catalog();
    let path = existing_catalog(&f, &catalog_golden("existing-claude.json"));
    f.catalog(&["--all", "--claude"])
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "Removed upskill from {}",
            path.display()
        )));
    assert_eq!(
        fs::read(path).unwrap(),
        catalog_golden("merged-all-claude.json")
    );
    let mut names: Vec<_> = fs::read_dir(f.market.join("plugins"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["base-claude", "demo-claude", "norules-claude"]);
}

#[test]
fn pack_catalog_deletes_no_plugin_directory() {
    let f = Fixture::new_catalog();
    let keep = f.market.join("plugins/stale-claude/keep");
    fs::create_dir_all(keep.parent().unwrap()).unwrap();
    fs::write(&keep, "keep").unwrap();
    let path = existing_catalog(
        &f,
        br#"{"plugins":[{"name":"stale","source":"./plugins/stale-claude","description":"d"}]}"#,
    );
    f.catalog(&["--all", "--claude"]).assert().success();
    assert!(keep.is_file());
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert!(
        value["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["name"] != "stale")
    );
}

#[test]
fn pack_catalog_uses_the_header_flags_only_for_a_new_file() {
    let f = Fixture::new_catalog();
    let path = existing_catalog(&f, &catalog_golden("existing-claude.json"));
    f.catalog(&["demo", "--catalog-name", "other", "--catalog-owner", "org"])
        .assert()
        .success();
    let claude: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(claude["name"], "market");
    assert_eq!(
        claude["owner"],
        serde_json::json!({"email":"maintainer@example.com", "name":"driftsys"})
    );
    let copilot: serde_json::Value = serde_json::from_slice(
        &fs::read(f.market.join(".github/plugin/marketplace.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(copilot["name"], "other");
    assert_eq!(copilot["owner"], serde_json::json!({"name":"org"}));
}

#[test]
fn pack_catalog_refuses_an_invalid_marketplace_file_before_any_write() {
    let f = Fixture::new_catalog();
    let path = existing_catalog(&f, b"not json");
    f.catalog(&[&["demo"][..], NEW_CATALOG].concat())
        .assert()
        .code(1)
        .stderr(predicates::str::contains(path.display().to_string()));
    assert!(!f.market.join("plugins").exists());
    assert_eq!(fs::read(path).unwrap(), b"not json");
    assert!(!f.market.join(".github").exists());
}

#[test]
fn pack_catalog_below_the_registry_twice_writes_equal_bytes() {
    let f = Fixture::new_catalog();
    let mut cmd = common::upskill_cmd(&f.home);
    cmd.current_dir(&f.registry)
        .args([
            "pack",
            "--all",
            "--source",
            "driftsys/demo@v1",
            "--out",
            "plugins",
            "--catalog",
            ".",
        ])
        .args(NEW_CATALOG)
        .assert()
        .success();
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(f.registry.join(".claude-plugin/marketplace.json")).unwrap(),
    )
    .unwrap();
    assert!(
        value["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["name"] == "demo" && entry["source"] == "./plugins/demo-claude")
    );
    let before = tree(&f.registry);
    common::upskill_cmd(&f.home)
        .current_dir(&f.registry)
        .args([
            "pack",
            "--all",
            "--source",
            "driftsys/demo@v1",
            "--out",
            "plugins",
            "--catalog",
            ".",
            "--force",
        ])
        .assert()
        .success();
    assert_eq!(tree(&f.registry), before);
}

#[test]
fn pack_catalog_of_an_unknown_bundle_writes_no_file() {
    let f = Fixture::new_catalog();
    f.catalog(&[&["demo", "nope"][..], NEW_CATALOG].concat())
        .assert()
        .code(1);
    assert!(!f.market.exists());
}

#[cfg(unix)]
#[test]
fn pack_catalog_reports_the_first_file_before_a_later_write_fails() {
    use std::os::unix::fs::PermissionsExt;

    let f = Fixture::new_catalog();
    let locked = f.market.join(".github/plugin");
    fs::create_dir_all(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    let claude = f.market.join(".claude-plugin/marketplace.json");
    let args = [&["demo"][..], NEW_CATALOG].concat();
    let result = f
        .catalog(&args)
        .assert()
        .code(1)
        .stderr(predicates::str::contains("marketplace.json.tmp"))
        .stdout(predicates::str::contains(format!(
            "Wrote {}",
            claude.display()
        )))
        .try_success();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    drop(result);
    assert!(claude.is_file());
    // The plugin directories stay as written.
    for client in ["claude", "copilot"] {
        assert!(f.market.join(format!("plugins/demo-{client}")).is_dir());
    }
    let copilot = f.market.join(".github/plugin/marketplace.json");
    assert!(!copilot.exists());
    // A rerun with `--force` repairs the run.
    f.catalog(&[&args[..], &["--force"]].concat())
        .assert()
        .success();
    assert!(copilot.is_file());
}

#[cfg(unix)]
#[test]
fn pack_catalog_writes_plugins_and_marketplace_to_one_tree_for_a_link_and_parent_component() {
    let f = Fixture::new_catalog();
    let base = f._tmp.path();
    fs::create_dir_all(base.join("other/subdir")).unwrap();
    fs::create_dir_all(base.join("work")).unwrap();
    std::os::unix::fs::symlink(base.join("other/subdir"), base.join("work/link")).unwrap();
    let root = base.join("work/link/../market");
    common::upskill_cmd(&f.home)
        .current_dir(&f.registry)
        .args(["pack", "demo", "--claude", "--source", "driftsys/demo@v1"])
        .arg("--out")
        .arg(root.join("plugins"))
        .arg("--catalog")
        .arg(&root)
        .args(NEW_CATALOG)
        .assert()
        .success();
    // The operating system resolves `link/..` to `other`; the catalog does not.
    assert!(!base.join("other/market").exists());
    let tree_root = base.join("work/market");
    let marketplace = read_json(&tree_root.join(".claude-plugin/marketplace.json"));
    let source = marketplace["plugins"][0]["source"].as_str().unwrap();
    assert_eq!(source, "./plugins/demo-claude");
    assert!(tree_root.join(source).join(".claude-plugin").is_dir());
}

#[test]
fn pack_catalog_with_out_equal_to_the_catalog_removes_shorthand_and_url_sources() {
    let f = Fixture::new_catalog();
    let path = existing_catalog(
        &f,
        br#"{"name":"m","owner":{"name":"o"},"plugins":[
            {"name":"shorthand","source":"owner/repo"},
            {"name":"url","source":"https://example.com/owner/repo"},
            {"name":"object","source":{"source":"github","repo":"owner/repo"}},
            {"name":"absolute","source":"/opt/plugin"}]}"#,
    );
    common::upskill_cmd(&f.home)
        .current_dir(&f.registry)
        .args(["pack", "--all", "--claude", "--source", "driftsys/demo@v1"])
        .arg("--out")
        .arg(&f.market)
        .arg("--catalog")
        .arg(&f.market)
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "Removed shorthand from {}",
            path.display()
        )))
        .stdout(predicates::str::contains(format!(
            "Removed url from {}",
            path.display()
        )));
    let names: Vec<String> = read_json(&path)["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"object".to_string()), "{names:?}");
    assert!(names.contains(&"absolute".to_string()), "{names:?}");
    assert!(!names.contains(&"shorthand".to_string()), "{names:?}");
    assert!(!names.contains(&"url".to_string()), "{names:?}");
}

#[test]
fn pack_catalog_with_a_relative_out_prints_the_normalized_absolute_plugin_path() {
    let f = Fixture::new_catalog();
    let output = common::upskill_cmd(&f.home)
        .current_dir(&f.registry)
        .args(["pack", "demo", "--claude", "--source", "driftsys/demo@v1"])
        .args(["--out", "../market/x/../plugins", "--catalog", "../market"])
        .args(NEW_CATALOG)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout).into_owned();
    let line = stdout.lines().next().unwrap();
    assert!(line.starts_with("Wrote /"), "{stdout}");
    assert!(line.contains("/market/plugins/demo-claude ("), "{stdout}");
    assert!(!line.contains(".."), "{stdout}");
    assert!(f.market.join("plugins/demo-claude").is_dir());
    assert!(!f.market.join("x").exists());
}

#[test]
fn pack_catalog_writes_no_marketplace_file_when_a_plugin_directory_fails() {
    let f = Fixture::new_catalog();
    let busy = f.market.join("plugins/demo-claude/keep");
    fs::create_dir_all(busy.parent().unwrap()).unwrap();
    fs::write(&busy, "keep").unwrap();
    f.catalog(&[&["demo", "--claude"][..], NEW_CATALOG].concat())
        .assert()
        .code(1)
        .stderr(predicates::str::contains("is not empty"));
    assert!(!f.market.join(".claude-plugin").exists());
    let path = existing_catalog(&f, &catalog_golden("existing-claude.json"));
    f.catalog(&["demo", "--claude"]).assert().code(1);
    assert_eq!(
        fs::read(path).unwrap(),
        catalog_golden("existing-claude.json")
    );
}

#[test]
fn pack_catalog_keeps_the_fields_and_foreign_entries_of_an_existing_copilot_file() {
    let f = Fixture::new_catalog();
    let path = f.market.join(".github/plugin/marketplace.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let existing = serde_json::json!({
        "name": "copilot-market",
        "owner": {"name": "keeper"},
        "metadata": {"description": "Hand-written"},
        "plugins": [{"name": "foreign", "source": "./vendor/foreign", "description": "F"}]
    });
    fs::write(&path, serde_json::to_vec(&existing).unwrap()).unwrap();
    f.catalog(&[&["demo", "--copilot"][..], NEW_CATALOG].concat())
        .assert()
        .success();
    let value = read_json(&path);
    assert_eq!(value["name"], "copilot-market");
    assert_eq!(value["owner"], serde_json::json!({"name": "keeper"}));
    assert_eq!(value["metadata"], existing["metadata"]);
    let names: Vec<_> = value["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["demo", "foreign"]);
}

fn stale_catalog(f: &Fixture) -> PathBuf {
    existing_catalog(
        f,
        br#"{"name":"m","owner":{"name":"o"},"plugins":[
            {"name":"zeta","source":"./plugins/zeta-old"},
            {"name":"alpha","source":"./plugins/alpha-old"},
            {"name":"bare","source":"./vendor/bare"},
            {"name":"nosource"}
        ]}"#,
    )
}

#[test]
fn pack_catalog_all_reports_removals_in_name_order_before_the_write_line() {
    let f = Fixture::new_catalog();
    let path = stale_catalog(&f);
    let output = f.catalog(&["--all", "--claude"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let lines: Vec<_> = stdout.lines().collect();
    let tail = &lines[lines.len() - 3..];
    assert_eq!(
        tail,
        [
            format!("Removed alpha from {}", path.display()),
            format!("Removed zeta from {}", path.display()),
            format!("Wrote {}", path.display()),
        ]
    );
}

#[test]
fn pack_catalog_keeps_the_absent_fields_of_a_kept_entry_absent() {
    let f = Fixture::new_catalog();
    let path = stale_catalog(&f);
    f.catalog(&["--all", "--claude"]).assert().success();
    let value = read_json(&path);
    let entries = value["plugins"].as_array().unwrap();
    let bare = entries.iter().find(|e| e["name"] == "bare").unwrap();
    assert_eq!(
        bare,
        &serde_json::json!({"name":"bare","source":"./vendor/bare"})
    );
    let nosource = entries.iter().find(|e| e["name"] == "nosource").unwrap();
    assert_eq!(nosource, &serde_json::json!({"name":"nosource"}));
}

#[test]
fn pack_catalog_quiet_prints_no_removed_and_no_wrote_line() {
    let f = Fixture::new_catalog();
    let path = stale_catalog(&f);
    f.catalog(&["--all", "--claude", "--quiet"])
        .assert()
        .success()
        .stdout("");
    assert!(!fs::read_to_string(path).unwrap().contains("zeta"));
}

#[test]
fn pack_catalog_refuses_an_empty_catalog_name_or_owner() {
    for flags in [
        &["--catalog-name", "", "--catalog-owner", "o"][..],
        &["--catalog-name", "m", "--catalog-owner", ""][..],
    ] {
        let f = Fixture::new_catalog();
        f.catalog(&[&["demo"][..], flags].concat()).assert().code(2);
        assert!(!f.market.exists());
    }
}
