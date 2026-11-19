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
        let mut cmd = common::upskill_cmd(&self.home);
        cmd.current_dir(&self.registry)
            .args(["pack", bundle, "--source", "driftsys/demo@v1", "--out"])
            .arg(&self.out)
            .arg("--claude");
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
fn pack_of_an_unknown_bundle_fails() {
    let f = Fixture::new();
    f.command("nope")
        .assert()
        .code(1)
        .stderr(predicates::str::contains("nope"));
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
fn pack_builds_all_selected_clients_before_writing() {
    let f = Fixture::new();
    f.command("demo")
        .arg("--copilot")
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "pack for Copilot CLI is not implemented",
        ));
    assert!(!f.out.exists());
}

#[test]
fn pack_defaults_to_claude_and_honors_quiet() {
    let f = Fixture::new();
    common::upskill_cmd(&f.home)
        .current_dir(&f.registry)
        .args(["pack", "demo", "--source", "driftsys/demo@v1", "--out"])
        .arg(&f.out)
        .assert()
        .success()
        .stdout(format!("Wrote {} (7 files)\n", f.target().display()));
    f.command("demo")
        .args(["--force", "--quiet"])
        .assert()
        .success()
        .stdout("");
    f.assert_golden();
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
