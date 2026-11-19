//! ATDD tests for `add` of a source directory that is itself one item
//! directory (issues #472 and #488).
mod common;

use std::fs;
use std::path::{Path, PathBuf};

/// `(tmp, home, project)`: a fake home and a project directory that holds
/// `.git`.
fn env() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    // Canonical, so that a path printed from the current directory matches.
    // On Windows, `canonicalize` returns a verbatim path (`\\?\C:\...`),
    // which upskill does not read as a local path (#561); the prefix is
    // removed there.
    let mut base = fs::canonicalize(tmp.path()).unwrap();
    if cfg!(windows) {
        let text = base.display().to_string();
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            base = PathBuf::from(rest);
        }
    }
    let home = base.join("home");
    let project = base.join("project");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(project.join(".git")).unwrap();
    (tmp, home, project)
}

fn write_skill(dir: &Path, name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Use for testing add of an item directory\n---\n\nBody.\n"),
    )
    .unwrap();
}

/// Every file and directory under `dir`, relative to `dir`, sorted.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.push(path.strip_prefix(base).unwrap().to_path_buf());
                walk(base, &path, out);
            } else {
                out.push(path.strip_prefix(base).unwrap().to_path_buf());
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn stderr_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stderr.clone()).unwrap()
}

/// R1: `.` alone is a local path, the same as `./`.
#[test]
fn add_dot_is_a_local_path() {
    let (_tmp, home, project) = env();
    write_skill(&project.join("skills/foo"), "foo");
    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", ".", "--project", "--claude"])
        .assert()
        .success();
    assert!(project.join(".claude/skills/foo/SKILL.md").is_file());
}

/// R2: a local source directory that holds an entrypoint at its root is
/// refused, with every spelling of the path, and nothing is written.
#[test]
fn add_of_an_item_directory_is_refused_and_writes_nothing() {
    let (_tmp, home, project) = env();
    let parent = project.join("reg");
    let item = parent.join("foo");
    write_skill(&item, "foo");
    let before = files_under(&project);
    let item_str = item.to_str().unwrap().to_string();
    for (cwd, spelling) in [(&project, item_str.as_str()), (&item, "./"), (&item, ".")] {
        let assert = common::upskill_cmd(&home)
            .current_dir(cwd)
            .args(["add", spelling, "--project", "--claude"])
            .assert()
            .code(1);
        let stderr = stderr_of(&assert);
        assert!(
            stderr.contains("is an item directory (it holds SKILL.md)"),
            "{spelling}: {stderr}"
        );
        assert!(
            stderr.contains(&format!("upskill add {} foo", parent.display())),
            "{spelling}: stderr does not name the command to use: {stderr}"
        );
        assert!(
            stderr.contains(&format!("source {}", item.display())),
            "{spelling}: stderr does not name the path: {stderr}"
        );
        assert_eq!(
            files_under(&project),
            before,
            "{spelling}: files were written"
        );
    }
}

/// R3: a single-item source that is itself an output path is refused, the
/// path is named, no `add <parent>` command is suggested, and nothing is
/// written.
#[test]
fn add_of_an_installed_output_item_directory_is_refused() {
    let (tmp, home, _project) = env();
    let item = home.join(".claude/skills/x");
    write_skill(&item, "x");
    let before = files_under(&home);
    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", item.to_str().unwrap(), "--global", "--claude"])
        .assert()
        .code(1);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains(&format!(
            "{} is an output path that upskill installs, not a source. Add the item from the \
             source registry that it was installed from",
            item.display()
        )),
        "{stderr}"
    );
    assert!(!stderr.contains("Use instead"), "{stderr}");
    assert_eq!(files_under(&home), before);
}

fn write_entry(dir: &Path, file: &str, name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join(file),
        format!("---\nname: {name}\ndescription: Use for testing add of an item directory\n---\n\nBody.\n"),
    )
    .unwrap();
}

/// Run `add <dir> --project --claude` in `project`; assert exit 1, nothing
/// written, and return stderr.
fn assert_refused(home: &Path, project: &Path, dir: &Path) -> String {
    let before = files_under(project);
    let assert = common::upskill_cmd(home)
        .current_dir(project)
        .args(["add", dir.to_str().unwrap(), "--project", "--claude"])
        .assert()
        .code(1);
    assert_eq!(files_under(project), before, "files were written");
    stderr_of(&assert)
}

/// R2 for the other entrypoints: the name in the suggested command is the
/// name that `add <parent> <name>` accepts, and the suggestion succeeds.
#[test]
fn add_of_a_rule_or_agent_directory_is_refused_with_a_working_command() {
    for (file, kind_dir) in [("RULE.md", "rules"), ("AGENT.md", "agents")] {
        let (_tmp, home, project) = env();
        let parent = project.join("reg");
        let item = parent.join("folder");
        write_entry(&item, file, "front");
        let stderr = assert_refused(&home, &project, &item);
        assert!(
            stderr.contains(&format!("(it holds {file})")),
            "{file}: {stderr}"
        );
        let suggested = format!("upskill add {} front", parent.display());
        assert!(stderr.contains(&suggested), "{file}: {stderr}");
        common::upskill_cmd(&home)
            .current_dir(&project)
            .args([
                "add",
                parent.to_str().unwrap(),
                "front",
                "--project",
                "--claude",
            ])
            .assert()
            .success();
        assert!(
            project.join(".claude").join(kind_dir).exists(),
            "{file}: the suggested command installed nothing"
        );
    }
}

/// An item directory with an item in a grandchild directory is a source.
#[test]
fn add_of_an_item_directory_with_a_grandchild_item_is_accepted() {
    let (_tmp, home, project) = env();
    let src = project.join("src");
    write_skill(&src, "src");
    write_skill(&src.join("cat/bar"), "bar");
    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", src.to_str().unwrap(), "--project", "--claude"])
        .assert()
        .success();
    assert!(project.join(".claude/skills/bar/SKILL.md").is_file());
}

/// An item directory that holds a bundle file, or a bundle that a given name
/// matches, is a source.
#[test]
fn add_of_an_item_directory_with_a_bundle_is_accepted() {
    for (bundle_rel, names) in [
        ("meta.bundle.yaml", vec![]),
        ("bundles/meta.bundle.yaml", vec!["meta"]),
    ] {
        let (tmp, home, project) = env();
        let dep = tmp.path().join("dep");
        write_skill(&dep.join("q"), "q");
        fs::write(
            dep.join("dep.bundle.yaml"),
            "schema: 1\nname: dep\ndescription: d\nitems:\n  skills:\n  - q\n",
        )
        .unwrap();
        let reg = project.join("reg");
        write_skill(&reg, "reg");
        let bundle = reg.join(bundle_rel);
        fs::create_dir_all(bundle.parent().unwrap()).unwrap();
        fs::write(
            &bundle,
            format!(
                "schema: 1\nname: meta\ndescription: d\nitems: {{}}\nrequires:\n  - {{ name: dep, source: {} }}\n",
                dep.display()
            ),
        )
        .unwrap();
        let mut args = vec!["add".to_string(), reg.to_str().unwrap().to_string()];
        args.extend(names.iter().map(|n| n.to_string()));
        args.extend(["--project".into(), "--claude".into()]);
        common::upskill_cmd(&home)
            .current_dir(&project)
            .args(&args)
            .assert()
            .success();
        // Without a name, `add` installs the items of the directory and not
        // its bundles, as before: it is not refused and installs nothing.
        if names.is_empty() {
            assert!(!project.join(".claude/skills").exists(), "{bundle_rel}");
            assert!(project.join(".upskill-lock.json").is_file(), "{bundle_rel}");
        } else {
            assert!(
                project.join(".claude/skills/q/SKILL.md").is_file(),
                "{bundle_rel}: q not installed"
            );
        }
    }
}

/// The suggested command quotes a parent with a space and has no `..`.
#[test]
fn suggested_command_is_quoted_and_normalised() {
    let (_tmp, home, project) = env();
    let parent = project.join("my reg");
    write_skill(&parent.join("foo"), "foo");
    fs::create_dir_all(parent.join("other")).unwrap();
    let spelled = parent.join("other/../foo");
    let stderr = assert_refused(&home, &project, &spelled);
    assert!(
        stderr.contains(&format!("upskill add '{}' foo", parent.display())),
        "{stderr}"
    );
}

/// The base message stays for a plain tree whose source root is inside a
/// client directory: no symbolic link exists.
#[test]
fn overlap_in_a_plain_client_directory_keeps_the_base_message() {
    let (tmp, home, _project) = env();
    let registry = home.join(".claude/skills");
    write_skill(&registry.join("x"), "x");
    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", registry.to_str().unwrap(), "--global", "--claude"])
        .assert()
        .code(1);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("A source must not be rooted in a client directory or contain one"),
        "{stderr}"
    );
    assert!(!stderr.contains("symbolic link"), "{stderr}");
}

/// Run `add <registry> --project --claude` and assert the symbolic-link
/// message.
fn assert_link_message(home: &Path, project: &Path, registry: &Path) {
    let assert = common::upskill_cmd(home)
        .current_dir(project)
        .args(["add", registry.to_str().unwrap(), "--project", "--claude"])
        .assert()
        .code(1);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("is or passes through a symbolic link into the source root"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("A source must not be rooted in a client directory"),
        "{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn overlap_through_a_linked_middle_directory_names_the_link() {
    let (tmp, home, project) = env();
    let registry = tmp.path().join("registry");
    write_skill(&registry.join("skills/x"), "x");
    fs::create_dir_all(project.join(".claude")).unwrap();
    std::os::unix::fs::symlink(registry.join("skills"), project.join(".claude/skills")).unwrap();
    assert_link_message(&home, &project, &registry);
}

#[cfg(unix)]
#[test]
fn overlap_through_a_linked_client_directory_names_the_link() {
    let (tmp, home, project) = env();
    let registry = tmp.path().join("registry");
    write_skill(&registry.join("skills/x"), "x");
    std::os::unix::fs::symlink(&registry, project.join(".claude")).unwrap();
    assert_link_message(&home, &project, &registry);
}

/// R4: when an output path is a hand-made symbolic link into the source
/// root, the error says so and does not claim that the source is rooted in a
/// client directory.
#[cfg(unix)]
#[test]
fn overlap_through_a_symbolic_link_names_the_link() {
    let (tmp, home, project) = env();
    let registry = tmp.path().join("registry");
    write_skill(&registry.join("skills/x"), "x");
    fs::create_dir_all(project.join(".claude/skills")).unwrap();
    let link = project.join(".claude/skills/x");
    std::os::unix::fs::symlink(registry.join("skills/x"), &link).unwrap();
    let before = files_under(&registry);
    let assert = common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", registry.to_str().unwrap(), "--project", "--claude"])
        .assert()
        .code(1);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains(&format!(
            "The output path {} under the client directory {} is or passes through a \
             symbolic link into the source root {}",
            fs::canonicalize(&project)
                .unwrap()
                .join(".claude/skills/x")
                .display(),
            fs::canonicalize(&project)
                .unwrap()
                .join(".claude")
                .display(),
            fs::canonicalize(&registry).unwrap().display()
        )),
        "{stderr}"
    );
    assert!(stderr.contains("Remove the symbolic link"), "{stderr}");
    assert!(
        !stderr.contains("A source must not be rooted in a client directory"),
        "{stderr}"
    );
    assert_eq!(files_under(&registry), before);
    assert!(!project.join(".upskill-lock.json").exists());
}

/// The installed-output message for the project scope and for clients other
/// than Claude: a Copilot rule path, an OpenCode agent path matched by the
/// folder name, and a path inside an OpenCode agent path matched by the
/// frontmatter name.
#[test]
fn add_of_a_project_output_path_of_another_client_is_refused() {
    let cases: [(&str, &str, &str); 3] = [
        (".github/instructions/x", "RULE.md", "x"),
        (".opencode/agents/x", "AGENT.md", "other"),
        (".opencode/agents/n/sub", "AGENT.md", "n"),
    ];
    for (rel, file, name) in cases {
        let (_tmp, home, project) = env();
        let item = project.join(rel);
        write_entry(&item, file, name);
        let stderr = assert_refused(&home, &project, &item);
        assert!(
            stderr.contains("is an output path that upskill installs, not a source"),
            "{rel}: {stderr}"
        );
        assert!(!stderr.contains("Use instead"), "{rel}: {stderr}");
    }
}

/// A name that matches no bundle of the directory does not lift the refusal.
#[test]
fn add_of_an_item_directory_with_a_name_that_is_no_bundle_is_refused() {
    let (_tmp, home, project) = env();
    let item = project.join("reg/foo");
    write_skill(&item, "foo");
    fs::create_dir_all(item.join("bundles")).unwrap();
    fs::write(
        item.join("bundles/meta.bundle.yaml"),
        "schema: 1\nname: meta\ndescription: d\nitems: {}\n",
    )
    .unwrap();
    let before = files_under(&project);
    let assert = common::upskill_cmd(&home)
        .current_dir(&project)
        .args([
            "add",
            item.to_str().unwrap(),
            "foo",
            "--project",
            "--claude",
        ])
        .assert()
        .code(1);
    let stderr = stderr_of(&assert);
    assert!(stderr.contains("Use instead: upskill add"), "{stderr}");
    assert_eq!(files_under(&project), before);
}

/// The item name in the suggested command is quoted, and a `'` in it is
/// escaped.
#[test]
fn suggested_item_name_is_quoted() {
    let (_tmp, home, project) = env();
    let parent = project.join("reg");
    let item = parent.join("r");
    write_entry(&item, "RULE.md", "a b'c");
    let stderr = assert_refused(&home, &project, &item);
    assert!(
        stderr.contains(&format!("upskill add {} 'a b'\\''c'", parent.display())),
        "{stderr}"
    );
}

/// An item directory with an item in a child directory is a source.
#[test]
fn add_of_an_item_directory_with_a_child_item_is_accepted() {
    let (_tmp, home, project) = env();
    let src = project.join("src");
    write_skill(&src, "src");
    write_skill(&src.join("child"), "child");
    common::upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", src.to_str().unwrap(), "--project", "--claude"])
        .assert()
        .success();
    assert!(project.join(".claude/skills/child/SKILL.md").is_file());
}

/// The output paths are those of the scope of the run: `.copilot/` is a
/// Copilot output path in the global scope only.
#[test]
fn installed_output_check_uses_the_scope_of_the_run() {
    // Global: `$HOME/.copilot/instructions/x` is an output path.
    let (tmp, home, project) = env();
    let item = home.join(".copilot/instructions/x");
    write_entry(&item, "RULE.md", "x");
    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", item.to_str().unwrap(), "--global", "--claude"])
        .assert()
        .code(1);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("is an output path that upskill installs, not a source"),
        "global: {stderr}"
    );

    // Project: `<project>/.copilot/instructions/x` is not an output path.
    let item = project.join(".copilot/instructions/x");
    write_entry(&item, "RULE.md", "x");
    let stderr = assert_refused(&home, &project, &item);
    assert!(
        stderr.contains("Use instead: upskill add"),
        "project: {stderr}"
    );
}
