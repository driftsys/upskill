use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn add_creates_lockfile() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .args(["add", "microsoft/skills"])
        .assert()
        .success();

    let lockfile = cwd.path().join(".upskill-lock.json");
    assert!(lockfile.exists(), "lockfile must be created");

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&lockfile).unwrap()).unwrap();

    assert!(content.is_object());
    assert!(content["skills"].is_array());

    let skills = content["skills"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "skills");
    assert_eq!(skills[0]["source"], "github:microsoft/skills");
}

#[test]
fn add_updates_lockfile_with_additional_skills() {
    let cwd = tempdir().expect("must create temp dir");

    // First add
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    // Second add
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "format"])
        .assert()
        .success();

    let lockfile = cwd.path().join(".upskill-lock.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&lockfile).unwrap()).unwrap();

    let skills = content["skills"].as_array().unwrap();
    assert_eq!(skills.len(), 2);

    let names: Vec<&str> = skills.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"format"));
    assert!(names.contains(&"lint"));
}

#[test]
fn add_lockfile_records_ref() {
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills@v1.0"])
        .assert()
        .success();

    let lockfile = cwd.path().join(".upskill-lock.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&lockfile).unwrap()).unwrap();

    let skills = content["skills"].as_array().unwrap();
    assert_eq!(skills[0]["ref"], "v1.0");
}

#[test]
fn remove_updates_lockfile() {
    let cwd = tempdir().expect("must create temp dir");

    // Add two skills
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "lint"])
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["add", "microsoft/skills", "--skill", "format"])
        .assert()
        .success();

    // Remove one
    Command::cargo_bin("upskill")
        .expect("binary exists")
        .current_dir(cwd.path())
        .args(["remove", "lint", "--yes"])
        .assert()
        .success();

    let lockfile = cwd.path().join(".upskill-lock.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&lockfile).unwrap()).unwrap();

    let skills = content["skills"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "format");
}

#[test]
fn lockfile_is_deterministic() {
    let cwd1 = tempdir().expect("must create temp dir");
    let cwd2 = tempdir().expect("must create temp dir");

    for cwd in [cwd1.path(), cwd2.path()] {
        Command::cargo_bin("upskill")
            .expect("binary exists")
            .current_dir(cwd)
            .args(["add", "microsoft/skills@v1.0", "--skill", "lint"])
            .assert()
            .success();
    }

    let content1 = std::fs::read_to_string(cwd1.path().join(".upskill-lock.json")).unwrap();
    let content2 = std::fs::read_to_string(cwd2.path().join(".upskill-lock.json")).unwrap();

    assert_eq!(content1, content2, "lockfiles must be identical");
}
