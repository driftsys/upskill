use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn no_color_produces_plain_error_output() {
    let cwd = tempdir().expect("must create temp dir");
    let mut cmd = Command::cargo_bin("upskill").expect("binary exists");

    cmd.current_dir(cwd.path())
        .env("NO_COLOR", "1")
        .args(["add", "not-a-valid-source"])
        .assert()
        .code(2)
        .stderr("error: source must be in owner/repo format\n");
}
