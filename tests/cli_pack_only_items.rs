//! ATDD tests for the command items, hook items and LSP servers that `add`
//! and `update` skip.

mod common;

use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

const LINE: &str = "skipped 2 command item(s): only `upskill pack` writes them";
const COMMAND_ERROR: &str = "`release-notes` is a command item; `upskill add` does not install command items. Use `upskill pack` to build a plugin that holds it";
const LSP: &str =
    "lsps:\n  rust-analyzer:\n    command: rust-analyzer\n    extensions: { .rs: rust }\n";

struct Fixture {
    _tmp: tempfile::TempDir,
    registry: PathBuf,
    consumer: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let registry = tmp.path().join("registry");
        let consumer = tmp.path().join("consumer");
        let home = tmp.path().join("home");
        fs::create_dir_all(consumer.join(".git")).unwrap();
        fs::create_dir_all(&home).unwrap();
        for (name, entry) in [
            ("alpha", "SKILL.md"),
            ("style", "RULE.md"),
            ("release-notes", "COMMAND.md"),
            ("standup", "COMMAND.md"),
        ] {
            let dir = registry.join("skills").join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join(entry),
                format!(
                    "---\nschema: 1\nname: {name}\ndescription: Test item\n---\n\nTest body.\n"
                ),
            )
            .unwrap();
        }
        for (folder, name) in [("guard", "guard"), ("checks", "edit-checks")] {
            let dir = registry.join("skills").join(folder);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("HOOK.md"),
                format!(
                    "---\nschema: 1\nname: {name}\ndescription: Test hook\nevents:\n  session-start:\n---\n\nRead the guide.\n"
                ),
            )
            .unwrap();
        }
        let f = Self {
            _tmp: tmp,
            registry,
            consumer,
            home,
        };
        f.bundle(
            "team",
            "  skills: [alpha]\n  rules: [style]\n  commands: [release-notes, standup]\n",
            "",
        );
        f.bundle(
            "hooked",
            "  skills: [alpha]\n  commands: [standup]\n  hooks: [guard]\n",
            "lsps:\n  ra:\n    command: ra\n    extensions: { .rs: rust }\n",
        );
        f
    }

    fn bundle(&self, name: &str, items: &str, extra: &str) {
        fs::write(
            self.registry
                .join("skills")
                .join(format!("{name}.bundle.yaml")),
            format!("schema: 1\nname: {name}\ndescription: Test bundle\nitems:\n{items}{extra}"),
        )
        .unwrap();
    }

    fn command(&self, action: &str) -> assert_cmd::Command {
        let mut cmd = common::upskill_cmd(&self.home);
        cmd.current_dir(&self.consumer)
            .arg(action)
            .arg("--project")
            .env("NO_COLOR", "1");
        cmd
    }

    fn add(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.command("add").arg(&self.registry).args(args).assert()
    }
}

fn paths_below(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            paths.extend(paths_below(&path));
        }
        paths.push(path);
    }
    paths
}

fn assert_no_lsp_files(f: &Fixture) {
    for root in [&f.consumer, &f.home] {
        assert!(paths_below(root).iter().all(|path| {
            ![".lsp.json", "lsp.json"]
                .iter()
                .any(|name| path.file_name().unwrap() == *name)
        }));
    }
}

/// Asserts that no file or directory below `.claude/` has one of `names` or
/// is a `COMMAND.md` or `HOOK.md`, and that the lockfile does not contain any of `names`.
fn assert_not_installed(f: &Fixture, names: &[&str]) {
    let claude = f.consumer.join(".claude");
    if claude.exists() {
        for path in paths_below(&claude) {
            let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
            let stem = file_name.trim_end_matches(".md");
            assert!(
                file_name != "COMMAND.md" && file_name != "HOOK.md" && !names.contains(&stem),
                "unexpected installed path: {}",
                path.display()
            );
        }
    }
    let lock = f.consumer.join(".upskill-lock.json");
    if lock.exists() {
        let content = fs::read_to_string(lock).unwrap();
        for name in names {
            assert!(!content.contains(name), "lockfile contains `{name}`");
        }
    }
}

#[test]
fn add_of_a_bundle_skips_its_command_items_and_reports_the_count() {
    let f = Fixture::new();
    f.add(&["team", "--claude"])
        .success()
        .stderr(predicate::str::contains(LINE));
    assert!(f.consumer.join(".claude/skills/alpha/SKILL.md").is_file());
    assert!(paths_below(&f.consumer).iter().all(|path| {
        !path.components().any(|part| {
            ["release-notes", "standup"]
                .iter()
                .any(|name| part.as_os_str() == *name)
        })
    }));
    let lock = fs::read_to_string(f.consumer.join(".upskill-lock.json")).unwrap();
    assert!(!lock.contains("release-notes"));
    assert!(!lock.contains("standup"));
}

#[test]
fn add_reports_one_skipped_command_item() {
    let f = Fixture::new();
    f.bundle("solo", "  skills: [alpha]\n  commands: [standup]\n", "");
    f.add(&["solo", "--claude"])
        .success()
        .stderr(predicate::str::contains(
            "skipped 1 command item(s): only `upskill pack` writes them",
        ));
}

#[test]
fn add_of_a_bundle_and_an_item_of_a_flat_registry_reports_the_skipped_commands() {
    let f = Fixture::new();
    // A flat registry holds its item directories at the root, so a mix of a
    // bundle name and an item name takes the name resolution path.
    let flat = f.registry.parent().unwrap().join("flat");
    for (name, entry) in [
        ("alpha", "SKILL.md"),
        ("beta", "SKILL.md"),
        ("standup", "COMMAND.md"),
    ] {
        let dir = flat.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(entry),
            format!("---\nschema: 1\nname: {name}\ndescription: Test item\n---\n\nTest body.\n"),
        )
        .unwrap();
    }
    fs::write(
        flat.join("team.bundle.yaml"),
        "schema: 1\nname: team\ndescription: Test bundle\nitems:\n  skills: [alpha]\n  commands: [standup]\n",
    )
    .unwrap();
    f.command("add")
        .arg(&flat)
        .args(["team", "beta", "--claude"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "skipped 1 command item(s): only `upskill pack` writes them",
        ));
    assert!(f.consumer.join(".claude/skills/beta/SKILL.md").is_file());
}

#[test]
fn add_of_a_bundle_file_with_a_positional_name_still_succeeds() {
    let f = Fixture::new();
    f.command("add")
        .arg(f.registry.join("skills/team.bundle.yaml"))
        .args(["release-notes", "--claude"])
        .assert()
        .success()
        .stderr(predicate::str::contains("is a command item").not());
}

#[test]
fn add_with_quiet_prints_no_skip_line() {
    let f = Fixture::new();
    f.add(&["team", "--claude", "--quiet"])
        .success()
        .stderr(predicate::str::contains("skipped").not());
}

#[test]
fn add_with_a_kind_restriction_prints_no_skip_line() {
    let f = Fixture::new();
    f.add(&["team", "--claude", "--kind", "rule"])
        .success()
        .stderr(predicate::str::contains("command item").not());
    assert!(f.consumer.join(".claude/rules/style.md").is_file());
}

#[test]
fn add_of_a_named_command_item_is_an_error_that_names_pack() {
    let f = Fixture::new();
    f.add(&["release-notes", "--claude"])
        .code(1)
        .stderr(predicate::str::contains(COMMAND_ERROR));
    assert!(!f.consumer.join(".upskill-lock.json").exists());
}

#[test]
fn add_of_a_skill_and_a_command_by_name_fails_before_any_write() {
    let f = Fixture::new();
    f.add(&["alpha", "release-notes", "--claude"])
        .code(1)
        .stderr(predicate::str::contains(COMMAND_ERROR));
    assert!(!f.consumer.join(".claude").exists());
    assert!(!f.consumer.join(".upskill-lock.json").exists());
}

#[test]
fn add_of_a_command_item_directory_as_source_is_an_error_that_names_pack() {
    let f = Fixture::new();
    let source = f.registry.join("skills").join("release-notes");
    f.command("add")
        .arg(&source)
        .arg("--claude")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(format!(
            "source {} is a command item directory (it holds COMMAND.md)",
            source.display()
        )))
        .stderr(predicate::str::contains("upskill pack"));
    assert!(!f.consumer.join(".upskill-lock.json").exists());
}

/// A directory that holds `COMMAND.md` and also `RULE.md` is refused as an
/// ordinary item directory source, not as a command item directory.
#[test]
fn add_of_a_directory_with_a_rule_and_a_command_entry_is_refused_as_a_rule_directory() {
    let f = Fixture::new();
    let source = f.registry.join("skills/style");
    fs::write(
        source.join("COMMAND.md"),
        "---\nschema: 1\nname: style\ndescription: Test item\n---\n\nTest body.\n",
    )
    .unwrap();
    f.command("add")
        .arg(&source)
        .arg("--claude")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "is an item directory (it holds RULE.md)",
        ))
        .stderr(predicate::str::contains("is a command item directory").not());
}

/// The same refusal holds for a `SKILL.md` or an `AGENT.md` that sits beside
/// a `COMMAND.md`: the directory is an ordinary item directory.
#[test]
fn add_of_a_directory_with_a_skill_or_agent_and_a_command_entry_is_refused_as_that_directory() {
    for entry in ["SKILL.md", "AGENT.md"] {
        let f = Fixture::new();
        let source = f.registry.join("skills/mixed");
        fs::create_dir_all(&source).unwrap();
        let body = "---\nschema: 1\nname: mixed\ndescription: Test item\n---\n\nTest body.\n";
        fs::write(source.join(entry), body).unwrap();
        fs::write(source.join("COMMAND.md"), body).unwrap();
        f.command("add")
            .arg(&source)
            .arg("--claude")
            .assert()
            .code(1)
            .stderr(predicate::str::contains(format!(
                "is an item directory (it holds {entry})"
            )))
            .stderr(predicate::str::contains("is a command item directory").not());
    }
}

#[test]
fn add_kind_command_is_a_usage_error() {
    let f = Fixture::new();
    f.add(&["team", "--kind", "command"])
        .code(2)
        .stderr(predicate::str::contains("unknown item kind: command"));
}

#[test]
fn add_of_a_whole_source_ignores_command_and_hook_directories() {
    let f = Fixture::new();
    fs::remove_file(f.registry.join("skills/team.bundle.yaml")).unwrap();
    fs::remove_file(f.registry.join("skills/hooked.bundle.yaml")).unwrap();
    f.add(&["--claude"])
        .success()
        .stderr(predicate::str::contains("command item").not())
        .stderr(predicate::str::contains("hook item").not());
    assert!(f.consumer.join(".claude/skills/alpha/SKILL.md").is_file());
    assert_not_installed(
        &f,
        &["release-notes", "standup", "guard", "edit-checks", "checks"],
    );
}

#[test]
fn update_reports_the_skipped_command_items_of_a_recorded_bundle() {
    let f = Fixture::new();
    f.add(&["team", "--claude"]).success();
    for args in [vec![], vec!["--dry-run"]] {
        f.command("update")
            .args(args)
            .assert()
            .success()
            .stderr(predicate::str::contains(LINE));
        assert_not_installed(&f, &["release-notes", "standup"]);
    }
}

#[test]
fn update_of_a_kind_restricted_bundle_prints_no_skip_line() {
    let f = Fixture::new();
    f.add(&["team", "--claude", "--kind", "rule"]).success();
    f.command("update")
        .assert()
        .success()
        .stderr(predicate::str::contains("command item").not());
}

#[test]
fn add_reports_skipped_lsp_servers_in_the_same_line() {
    let f = Fixture::new();
    f.bundle(
        "team",
        "  skills: [alpha]\n  commands: [release-notes, standup]\n",
        LSP,
    );
    let result = f.add(&["team", "--claude"]).success();
    let stderr = String::from_utf8_lossy(&result.get_output().stderr);
    assert_eq!(
        stderr
            .lines()
            .filter(|line| line.starts_with("skipped "))
            .collect::<Vec<_>>(),
        ["skipped 2 command item(s), 1 LSP server(s): only `upskill pack` writes them"]
    );
    assert_no_lsp_files(&f);
}

#[test]
fn add_reports_only_lsp_servers_when_the_bundle_has_no_command() {
    let f = Fixture::new();
    f.bundle("team", "  skills: [alpha]\n", LSP);
    f.add(&["team", "--claude"])
        .success()
        .stderr(predicate::str::contains(
            "skipped 1 LSP server(s): only `upskill pack` writes them",
        ));
    assert_no_lsp_files(&f);
}

#[test]
fn update_reports_skipped_lsp_servers_and_commands() {
    let f = Fixture::new();
    f.bundle(
        "team",
        "  skills: [alpha]\n  commands: [release-notes, standup]\n",
        LSP,
    );
    f.add(&["team", "--claude"]).success();
    for args in [vec![], vec!["--dry-run"]] {
        f.command("update")
            .args(args)
            .assert()
            .success()
            .stderr(predicate::str::contains(
                "skipped 2 command item(s), 1 LSP server(s): only `upskill pack` writes them",
            ));
        assert_not_installed(&f, &["release-notes", "standup", "rust-analyzer"]);
    }
    assert_no_lsp_files(&f);
}

#[test]
fn add_of_a_named_lsp_server_keeps_the_unknown_name_error() {
    let f = Fixture::new();
    f.bundle("team", "  skills: [alpha]\n", LSP);
    f.add(&["rust-analyzer", "--claude"])
        .code(1)
        .stderr(predicate::str::contains("no matching items or bundles"))
        .stderr(predicate::str::contains("is a command item").not());
    assert!(!f.consumer.join(".upskill-lock.json").exists());
}

#[test]
fn add_exclude_command_is_a_usage_error() {
    let f = Fixture::new();
    f.add(&["team", "--claude", "--exclude", "release-notes"])
        .code(2)
        .stderr(predicate::str::contains("--exclude matches no item"));
    assert!(!f.consumer.join(".upskill-lock.json").exists());
}

#[test]
fn add_of_a_name_that_is_a_bundle_and_a_command_item_installs_the_bundle() {
    let f = Fixture::new();
    f.bundle("standup", "  skills: [alpha]\n  commands: [standup]\n", "");
    f.add(&["standup", "--claude"])
        .success()
        .stderr(predicate::str::contains("is a command item;").not())
        .stderr(predicate::str::contains(
            "skipped 1 command item(s): only `upskill pack` writes them",
        ));
    assert!(f.consumer.join(".claude/skills/alpha/SKILL.md").is_file());
}

const HOOK_LINE: &str =
    "skipped 1 command item(s), 1 hook item(s), 1 LSP server(s): only `upskill pack` writes them";

#[test]
fn add_reports_skipped_hook_items_between_commands_and_lsp_servers() {
    let f = Fixture::new();
    let result = f.add(&["hooked", "--claude"]).success();
    let stderr = String::from_utf8_lossy(&result.get_output().stderr);
    assert_eq!(
        stderr
            .lines()
            .filter(|line| line.starts_with("skipped "))
            .collect::<Vec<_>>(),
        [HOOK_LINE]
    );
    assert!(f.consumer.join(".claude/skills/alpha/SKILL.md").is_file());
    assert!(
        paths_below(&f.consumer)
            .iter()
            .all(|path| { !path.components().any(|part| part.as_os_str() == "guard") })
    );
    assert_not_installed(&f, &["guard", "standup", "ra"]);
    assert_no_lsp_files(&f);
}

#[test]
fn add_reports_only_hook_items_when_the_bundle_has_no_other_pack_only_kind() {
    let f = Fixture::new();
    f.bundle("hookonly", "  skills: [alpha]\n  hooks: [guard]\n", "");
    f.add(&["hookonly", "--claude"])
        .success()
        .stderr(predicate::str::contains(
            "skipped 1 hook item(s): only `upskill pack` writes them",
        ));
    assert_not_installed(&f, &["guard"]);
}

#[test]
fn add_of_a_named_hook_item_is_an_error_that_names_pack() {
    let f = Fixture::new();
    f.add(&["guard", "--claude"])
        .code(1)
        .stderr(predicate::str::contains(
            "`guard` is a hook item; `upskill add` does not install hook items. Use `upskill pack` to build a plugin that holds it",
        ));
    assert!(!f.consumer.join(".upskill-lock.json").exists());
}

#[test]
fn add_of_a_hook_by_its_frontmatter_name_is_an_error_that_names_pack() {
    let f = Fixture::new();
    f.add(&["edit-checks", "--claude"])
        .code(1)
        .stderr(predicate::str::contains(
            "`edit-checks` is a hook item; `upskill add` does not install hook items. Use `upskill pack` to build a plugin that holds it",
        ));
    assert!(!f.consumer.join(".upskill-lock.json").exists());
}

#[test]
fn add_of_a_skill_and_a_hook_by_name_fails_before_any_write() {
    let f = Fixture::new();
    f.add(&["alpha", "guard", "--claude"])
        .code(1)
        .stderr(predicate::str::contains("`guard` is a hook item"));
    assert!(!f.consumer.join(".claude").exists());
    assert!(!f.consumer.join(".upskill-lock.json").exists());
}

#[test]
fn add_of_a_hook_item_directory_as_source_is_an_error_that_names_pack() {
    let f = Fixture::new();
    f.command("add")
        .arg(f.registry.join("skills/guard"))
        .arg("--claude")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "is a hook item directory (it holds HOOK.md)",
        ))
        .stderr(predicate::str::contains("upskill pack"));
    assert!(!f.consumer.join(".upskill-lock.json").exists());
}

#[test]
fn add_kind_hook_is_a_usage_error() {
    let f = Fixture::new();
    f.add(&["hooked", "--kind", "hook"])
        .code(2)
        .stderr(predicate::str::contains("unknown item kind: hook"));
}

#[test]
fn add_with_a_kind_restriction_prints_no_hook_count() {
    let f = Fixture::new();
    f.add(&["hooked", "--claude", "--kind", "skill"])
        .success()
        .stderr(predicate::str::contains("hook item").not());
    assert!(f.consumer.join(".claude/skills/alpha/SKILL.md").is_file());
}

#[test]
fn add_of_a_name_that_matches_a_scanned_skill_and_a_command_file_installs_the_skill() {
    let f = Fixture::new();
    // A directory that holds both entry files is a skill for `add`; the
    // named-command check does not apply to a name that a scanned item has.
    let dir = f.registry.join("skills/dual");
    fs::create_dir_all(&dir).unwrap();
    for entry in ["SKILL.md", "COMMAND.md"] {
        fs::write(
            dir.join(entry),
            "---\nschema: 1\nname: dual\ndescription: Test item\n---\n\nTest body.\n",
        )
        .unwrap();
    }
    f.add(&["dual", "--claude"])
        .success()
        .stderr(predicate::str::contains("is a command item;").not());
    assert!(f.consumer.join(".claude/skills/dual/SKILL.md").is_file());
}

/// A recorded skill that the source turned into a command is a recorded
/// item that the source no longer lists, so `update` removes it as no
/// longer in source (the same as for a deleted skill, issue #649), without
/// the command-item error of `add`.
#[test]
fn update_of_a_recorded_skill_that_the_source_turned_into_a_command_removes_it_and_names_no_command_error()
 {
    let f = Fixture::new();
    f.add(&["alpha", "--claude"]).success();
    fs::remove_file(f.registry.join("skills/alpha/SKILL.md")).unwrap();
    fs::write(
        f.registry.join("skills/alpha/COMMAND.md"),
        "---\nschema: 1\nname: alpha\ndescription: Test item\n---\n\nTest body.\n",
    )
    .unwrap();
    let lock_path = f.consumer.join(".upskill-lock.json");
    let skill_path = f.consumer.join(".claude/skills/alpha/SKILL.md");
    f.command("update")
        .assert()
        .success()
        .stderr(predicate::str::contains("is a command item;").not())
        .stdout(predicate::str::contains("no longer in source"));
    assert!(!skill_path.exists());
    let lock = fs::read_to_string(&lock_path).unwrap();
    assert!(!lock.contains("\"name\": \"alpha\""), "{lock}");
}

#[test]
fn update_counts_a_command_that_two_sources_list_once() {
    let f = Fixture::new();
    let other = f.registry.parent().unwrap().join("other");
    for (name, entry) in [("beta", "SKILL.md"), ("standup", "COMMAND.md")] {
        let dir = other.join("skills").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(entry),
            format!("---\nschema: 1\nname: {name}\ndescription: Test item\n---\n\nTest body.\n"),
        )
        .unwrap();
    }
    fs::write(
        other.join("skills/second.bundle.yaml"),
        "schema: 1\nname: second\ndescription: Test bundle\nitems:\n  skills: [beta]\n  commands: [standup]\n",
    )
    .unwrap();
    f.add(&["team", "--claude"]).success();
    f.command("add")
        .arg(&other)
        .args(["second", "--claude"])
        .assert()
        .success();
    f.command("update")
        .assert()
        .success()
        .stderr(predicate::str::contains(LINE));
}

#[test]
fn update_reports_the_skipped_hook_items_of_a_recorded_bundle() {
    let f = Fixture::new();
    f.add(&["hooked", "--claude"]).success();
    for args in [vec![], vec!["--dry-run"]] {
        f.command("update")
            .args(args)
            .assert()
            .success()
            .stderr(predicate::str::contains(HOOK_LINE));
        assert_not_installed(&f, &["guard", "standup", "ra"]);
    }
    assert_no_lsp_files(&f);
}

#[test]
fn add_reports_two_skipped_hook_items() {
    let f = Fixture::new();
    f.bundle(
        "twohooks",
        "  skills: [alpha]\n  hooks: [guard, checks]\n",
        "",
    );
    f.add(&["twohooks", "--claude"])
        .success()
        .stderr(predicate::str::contains(
            "skipped 2 hook item(s): only `upskill pack` writes them",
        ));
    assert_not_installed(&f, &["guard", "checks", "edit-checks"]);
}

#[test]
fn add_of_a_skill_with_a_named_colocated_hook_file_does_not_install_the_hook() {
    let f = Fixture::new();
    let dir = f.registry.join("skills/colocated");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nschema: 1\nname: colocated\ndescription: Test item\n---\n\nTest body.\n",
    )
    .unwrap();
    fs::write(
        dir.join("HOOK.md"),
        "---\nschema: 1\nname: colocated-hook\ndescription: Test hook\nevents:\n  session-start:\n---\n\nRead the guide.\n",
    )
    .unwrap();
    f.add(&["colocated", "--claude"]).success();
    assert!(
        f.consumer
            .join(".claude/skills/colocated/SKILL.md")
            .is_file()
    );
    assert_not_installed(&f, &["colocated-hook"]);
}

#[test]
fn add_of_a_name_that_is_a_skill_with_an_unnamed_colocated_hook_file_installs_the_skill() {
    let f = Fixture::new();
    let dir = f.registry.join("skills/colocated");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nschema: 1\nname: colocated\ndescription: Test item\n---\n\nTest body.\n",
    )
    .unwrap();
    fs::write(
        dir.join("HOOK.md"),
        "---\nschema: 1\ndescription: Test hook\nevents:\n  session-start:\n---\n\nRead the guide.\n",
    )
    .unwrap();
    f.add(&["colocated", "--claude"])
        .success()
        .stderr(predicate::str::contains("is a hook item").not());
    assert!(
        f.consumer
            .join(".claude/skills/colocated/SKILL.md")
            .is_file()
    );
}

#[test]
fn add_of_a_name_that_is_a_bundle_and_a_hook_item_installs_the_bundle() {
    let f = Fixture::new();
    f.bundle("guard", "  skills: [alpha]\n  hooks: [guard]\n", "");
    f.add(&["guard", "--claude"])
        .success()
        .stderr(predicate::str::contains("is a hook item;").not())
        .stderr(predicate::str::contains(
            "skipped 1 hook item(s): only `upskill pack` writes them",
        ));
    assert!(f.consumer.join(".claude/skills/alpha/SKILL.md").is_file());
}

#[test]
fn add_with_quiet_prints_no_hook_skip_line() {
    let f = Fixture::new();
    f.add(&["hooked", "--claude", "--quiet"])
        .success()
        .stderr(predicate::str::contains("skipped").not());
    assert!(f.consumer.join(".claude/skills/alpha/SKILL.md").is_file());
}

#[test]
fn add_of_a_skill_and_a_hook_directory_as_source_is_refused_as_a_skill_directory() {
    let f = Fixture::new();
    let dir = f.registry.parent().unwrap().join("mixed");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nschema: 1\nname: mixed\ndescription: Test item\n---\n\nTest body.\n",
    )
    .unwrap();
    fs::write(
        dir.join("HOOK.md"),
        "---\nschema: 1\ndescription: Test hook\nevents:\n  session-start:\n---\n\nRead the guide.\n",
    )
    .unwrap();
    f.command("add")
        .arg(&dir)
        .arg("--claude")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "is an item directory (it holds SKILL.md)",
        ))
        .stderr(predicate::str::contains("is a hook item directory").not());
}

#[test]
fn update_counts_a_hook_that_two_sources_list_once() {
    let f = Fixture::new();
    let other = f.registry.parent().unwrap().join("other");
    for (name, entry) in [("beta", "SKILL.md"), ("guard", "HOOK.md")] {
        let dir = other.join("skills").join(name);
        fs::create_dir_all(&dir).unwrap();
        let extra = if entry == "HOOK.md" {
            "events:\n  session-start:\n"
        } else {
            ""
        };
        fs::write(
            dir.join(entry),
            format!(
                "---\nschema: 1\nname: {name}\ndescription: Test item\n{extra}---\n\nTest body.\n"
            ),
        )
        .unwrap();
    }
    fs::write(
        other.join("skills/second.bundle.yaml"),
        "schema: 1\nname: second\ndescription: Test bundle\nitems:\n  skills: [beta]\n  hooks: [guard]\n",
    )
    .unwrap();
    f.bundle("first", "  skills: [alpha]\n  hooks: [guard]\n", "");
    f.add(&["first", "--claude"]).success();
    f.command("add")
        .arg(&other)
        .args(["second", "--claude"])
        .assert()
        .success();
    f.command("update")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "skipped 1 hook item(s): only `upskill pack` writes them",
        ));
}

#[test]
fn add_exclude_hook_is_a_usage_error() {
    let f = Fixture::new();
    f.add(&["hooked", "--claude", "--exclude", "guard"])
        .code(2)
        .stderr(predicate::str::contains(
            "--exclude matches no item of the source",
        ));
    assert!(!f.consumer.join(".upskill-lock.json").exists());
}
