use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

// Story #17: GitLab source support

#[test]
fn add_gitlab_prefix_installs_skill() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "gitlab:owner/repo", "--skill", "lint"])
        .assert()
        .success()
        .stdout(predicates::str::contains("install source: gitlab"))
        .stdout(predicates::str::contains("owner: owner"))
        .stdout(predicates::str::contains("repo: repo"));
}

#[test]
fn add_gitlab_url_installs_skill() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "https://gitlab.com/owner/repo", "--skill", "lint"])
        .assert()
        .success()
        .stdout(predicates::str::contains("install source: gitlab"))
        .stdout(predicates::str::contains("owner: owner"))
        .stdout(predicates::str::contains("repo: repo"));
}

#[test]
fn add_gitlab_with_ref_and_subfolder() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args([
            "add",
            "gitlab:owner/repo@v1.0:tools/lint",
            "--skill",
            "lint",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("ref: v1.0"))
        .stdout(predicates::str::contains("subfolder: tools/lint"));
}

#[test]
fn gitlab_lockfile_records_source() {
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "gitlab:owner/repo", "--skill", "lint"])
        .assert()
        .success();

    let lockfile =
        std::fs::read_to_string(cwd.path().join(".upskill-lock.json")).expect("lockfile exists");
    assert!(
        lockfile.contains("gitlab:owner/repo"),
        "lockfile should contain gitlab source"
    );
}

// Story #18: Self-hosted GitLab

#[test]
fn add_selfhosted_gitlab_url() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args([
            "add",
            "https://git.company.com/team/skills",
            "--skill",
            "lint",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("install source: gitlab"))
        .stdout(predicates::str::contains("owner: team"))
        .stdout(predicates::str::contains("repo: skills"));
}

#[test]
fn add_selfhosted_gitlab_with_port() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args([
            "add",
            "https://git.company.com:8443/team/skills",
            "--skill",
            "lint",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("install source: gitlab"));
}
