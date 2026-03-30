use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn add_accepts_owner_repo_source() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft/skills"])
        .assert()
        .success()
        .stdout("install source: github\nowner: microsoft\nrepo: skills\n");
}

#[test]
fn add_rejects_invalid_source_format() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft-skills"])
        .assert()
        .code(2)
        .stderr("error: source must be in owner/repo format\n");
}

#[test]
fn add_accepts_local_path_source() {
    let tmp = tempdir().expect("must create temp dir");
    let path = tmp.path().display().to_string();
    let cwd = tempdir().expect("must create temp dir");

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", path.as_str()])
        .assert()
        .success()
        .stdout(format!("install source: local\npath: {}\n", path));
}

#[test]
fn add_rejects_missing_local_path() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "./definitely-not-present-upskill-skill-path"])
        .assert()
        .code(2)
        .stderr("error: local path does not exist: ./definitely-not-present-upskill-skill-path\n");
}

#[test]
fn add_creates_canonical_target_for_github_source() {
    let cwd = tempdir().expect("must create temp dir");
    let canonical = cwd.path().join(".agents/skills");

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft/skills"])
        .assert()
        .success();

    assert!(canonical.is_dir(), "canonical target must be created");
}

#[test]
fn add_creates_canonical_target_for_local_source() {
    let cwd = tempdir().expect("must create temp dir");
    let local_source = cwd.path().join("source");
    std::fs::create_dir_all(&local_source).expect("must create local source");
    let canonical = cwd.path().join(".agents/skills");

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", local_source.to_str().expect("utf8 path")])
        .assert()
        .success();

    assert!(canonical.is_dir(), "canonical target must be created");
}

#[test]
fn add_creates_claude_symlink_when_flag_is_set() {
    let cwd = tempdir().expect("must create temp dir");
    std::fs::create_dir_all(cwd.path().join(".claude")).expect("must create .claude");

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--claude"])
        .assert()
        .success();

    let link_path = cwd.path().join(".claude/skills");
    assert!(
        std::fs::symlink_metadata(&link_path)
            .expect("metadata")
            .file_type()
            .is_symlink(),
        "claude skills must be a symlink"
    );
}

#[test]
fn add_creates_copilot_symlink_when_flag_is_set() {
    let cwd = tempdir().expect("must create temp dir");
    std::fs::create_dir_all(cwd.path().join(".github")).expect("must create .github");

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--copilot"])
        .assert()
        .success();

    let link_path = cwd.path().join(".github/skills");
    assert!(
        std::fs::symlink_metadata(&link_path)
            .expect("metadata")
            .file_type()
            .is_symlink(),
        "copilot skills must be a symlink"
    );
}

#[test]
fn add_creates_both_symlinks_with_multiple_flags() {
    let cwd = tempdir().expect("must create temp dir");
    std::fs::create_dir_all(cwd.path().join(".claude")).expect("must create .claude");
    std::fs::create_dir_all(cwd.path().join(".github")).expect("must create .github");

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--claude", "--copilot"])
        .assert()
        .success();

    assert!(std::fs::symlink_metadata(cwd.path().join(".claude/skills")).is_ok());
    assert!(std::fs::symlink_metadata(cwd.path().join(".github/skills")).is_ok());
}

#[test]
fn add_auto_detects_agent_directories_when_no_flags() {
    let cwd = tempdir().expect("must create temp dir");
    std::fs::create_dir_all(cwd.path().join(".claude")).expect("must create .claude");
    std::fs::create_dir_all(cwd.path().join(".github")).expect("must create .github");

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft/skills"])
        .assert()
        .success();

    assert!(std::fs::symlink_metadata(cwd.path().join(".claude/skills")).is_ok());
    assert!(std::fs::symlink_metadata(cwd.path().join(".github/skills")).is_ok());
}

#[test]
fn add_all_creates_symlinks_for_all_supported_agents() {
    let cwd = tempdir().expect("must create temp dir");

    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--all"])
        .assert()
        .success();

    let expected_links = [
        ".claude/skills",
        ".github/skills",
        ".codex/skills",
        ".cursor/skills",
        ".kiro/skills",
        ".windsurf/skills",
        ".opencode/skills",
    ];

    for link in expected_links {
        let path = cwd.path().join(link);
        let meta = std::fs::symlink_metadata(&path).expect("symlink metadata");
        assert!(meta.file_type().is_symlink(), "{link} must be a symlink");
    }
}

#[test]
fn add_accepts_single_skill_flag() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "rust-lint"])
        .assert()
        .success()
        .stdout("install source: github\nowner: microsoft\nrepo: skills\nskills: rust-lint\n");
}

#[test]
fn add_accepts_multiple_skill_flags() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args([
            "add",
            "microsoft/skills",
            "--skill",
            "rust-lint",
            "--skill",
            "release-check",
        ])
        .assert()
        .success()
        .stdout(
            "install source: github\nowner: microsoft\nrepo: skills\nskills: rust-lint,release-check\n",
        );
}

#[test]
fn add_accepts_github_subfolder_source() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft/skills:catalog/devops"])
        .assert()
        .success()
        .stdout(
            "install source: github\nowner: microsoft\nrepo: skills\nsubfolder: catalog/devops\n",
        );
}
