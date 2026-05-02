//! ATDD test for `pipeline::install_from_source`.
//!
//! Covers the dispatch over `InstallSource` variants:
//! - `LocalPath` — delegates to `install_from_local_path`.
//! - `Github` — shallow-clones into a temp dir, then runs the local install
//!   pipeline. Tested using a local bare git repo with a `file://` URL via
//!   the crate-internal `install_from_git_url` helper, which is the same
//!   code path the GitHub branch uses (the only difference is URL
//!   construction).
//! - `Gitlab` — currently returns an error (out of scope for this slice).
//!
//! Network is not used.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use upskill::pipeline::{InstallReport, install_from_source};
use upskill::source::InstallSource;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn stage_source(source: &Path) {
    for kind in ["skills", "rules", "agents"] {
        let from = format!("{FIXTURES}/{kind}");
        let to = source.join(kind);
        copy_dir_all(Path::new(&from), &to).unwrap();
    }
}

fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let to_path = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &to_path)?;
        } else {
            fs::copy(entry.path(), &to_path)?;
        }
    }
    Ok(())
}

#[test]
fn install_from_source_local_path() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);

    let report: InstallReport =
        install_from_source(&InstallSource::LocalPath(source.clone()), &target).expect("install");

    // Same expectation as the local-path direct test: 12 outputs.
    assert_eq!(report.items.len(), 12);
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
    assert!(
        target
            .join(".github/instructions/api-conventions.instructions.md")
            .exists()
    );
    assert!(
        target
            .join(".opencode/agents/security-reviewer.md")
            .exists()
    );
}

// GitLab dispatch is covered by unit tests in src/pipeline.rs
// (gitlab_clone_url_uses_repo_host) plus the existing file:// tests against
// install_from_git_url, which is the shared code path. An end-to-end network
// test would depend on a live GitLab instance and is intentionally omitted.

/// Build a local bare repo containing the fixture SSOT corpus, return its
/// path. The pipeline can then clone it via a `file://` URL — exercises the
/// same code path as a real GitHub clone, with no network.
fn make_bare_ssot_repo(root: &Path) -> PathBuf {
    let bare = root.join("ssot.git");
    let work = root.join("work");

    git(&["init", "--bare", bare.to_str().unwrap()], None);
    git(
        &["clone", bare.to_str().unwrap(), work.to_str().unwrap()],
        None,
    );

    // Stage the fixture corpus into the working tree.
    stage_source(&work);

    // Some `git init` defaults are `master`; the shallow-clone path passes
    // `--branch <ref>` only if a ref is requested, so we don't depend on a
    // specific branch name. Just commit and push HEAD.
    git(&["add", "."], Some(&work));
    git(
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@test.com",
            "commit",
            "-m",
            "initial",
        ],
        Some(&work),
    );
    git(&["push"], Some(&work));

    bare
}

fn git(args: &[&str], cwd: Option<&Path>) {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    let out = cmd.output().expect("git command");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn install_from_source_clones_local_bare_repo_via_file_url() {
    // Same end-to-end shape as install_from_source_local_path, but the
    // source is a git URL. Uses the crate-internal git-URL helper to avoid
    // depending on a real GitHub host. This exercises the clone +
    // resolve_subfolder + install_from_local_path composition.
    let tmp = tempfile::tempdir().unwrap();
    let bare = make_bare_ssot_repo(tmp.path());
    let target = tmp.path().join("target");
    let url = format!("file://{}", bare.display());

    let report = upskill::pipeline::install_from_git_url(&url, None, None, "test", "ssot", &target)
        .expect("install via git url");

    assert_eq!(report.items.len(), 12);
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
    assert!(
        target
            .join(".github/instructions/api-conventions.instructions.md")
            .exists()
    );
    assert!(
        target
            .join(".opencode/agents/security-reviewer.md")
            .exists()
    );
}

#[test]
fn install_from_source_clone_subfolder() {
    // Stage the SSOT corpus inside a subfolder of the repo, then ask the
    // pipeline to install only that subfolder. Verifies that
    // resolve_subfolder is wired correctly through install_from_git_url.
    let tmp = tempfile::tempdir().unwrap();
    let bare_root = tmp.path().join("repo-root");
    let bare = bare_root.join("ssot.git");
    let work = bare_root.join("work");
    fs::create_dir_all(&bare_root).unwrap();
    git(&["init", "--bare", bare.to_str().unwrap()], None);
    git(
        &["clone", bare.to_str().unwrap(), work.to_str().unwrap()],
        None,
    );

    // Place the fixture corpus under content/portable/ within the repo.
    let nested = work.join("content/portable");
    fs::create_dir_all(&nested).unwrap();
    stage_source(&nested);
    // Add an unrelated top-level file that should NOT be installed.
    fs::write(work.join("README.md"), "# repo readme\n").unwrap();

    git(&["add", "."], Some(&work));
    git(
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@test.com",
            "commit",
            "-m",
            "initial",
        ],
        Some(&work),
    );
    git(&["push"], Some(&work));

    let target = tmp.path().join("target");
    let url = format!("file://{}", bare.display());
    let report = upskill::pipeline::install_from_git_url(
        &url,
        None,
        Some("content/portable"),
        "test",
        "ssot",
        &target,
    )
    .expect("install via git url with subfolder");

    assert_eq!(report.items.len(), 12);
    assert!(
        target
            .join(".claude/skills/create-api-endpoint/SKILL.md")
            .exists()
    );
}
