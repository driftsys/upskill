use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::source::GithubRepo;

/// Fetch a GitHub repository via shallow `git clone`, returning the
/// path to the cloned working tree. The caller is responsible for
/// cleaning up the returned directory.
pub fn clone_github_repo(repo: &GithubRepo, dest: &Path) -> Result<PathBuf, String> {
    let url = format!("https://github.com/{}/{}.git", repo.owner, repo.name);
    shallow_clone(
        &url,
        repo.git_ref.as_deref(),
        &repo.name,
        dest,
        repo.subfolder.as_deref(),
    )?;
    let clone_dir = dest.join(&repo.name);
    resolve_subfolder(
        &clone_dir,
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
    )
}

/// Shallow clone a git URL into `dest/<dir_name>`. Visible inside the crate
/// so the v0.2 pipeline can clone from arbitrary URL forms (`file://`,
/// GitLab self-hosted, etc.) without going through the GitHub-specific
/// `clone_github_repo` constructor.
///
/// When `subfolder` is `Some`, uses partial clone (`--filter=blob:none
/// --sparse`) followed by `git sparse-checkout set <subfolder>` to fetch
/// only the target path. This avoids downloading blobs for the entire repo
/// when only a single skill or bundle is needed. Falls back to a regular
/// shallow clone if the sparse checkout step fails (e.g. older git
/// versions that don't support `sparse-checkout set`).
pub(crate) fn shallow_clone(
    url: &str,
    git_ref: Option<&str>,
    dir_name: &str,
    dest: &Path,
    subfolder: Option<&str>,
) -> Result<(), String> {
    let clone_dir = dest.join(dir_name);
    let clone_str = clone_dir
        .to_str()
        .ok_or_else(|| "clone path is not valid UTF-8".to_string())?;

    if let Some(sub) = subfolder {
        // Try sparse clone: only fetch tree metadata, then check out the
        // target subfolder. This is significantly faster for large repos.
        if try_sparse_clone(url, git_ref, clone_str, sub).is_ok() {
            return Ok(());
        }
        // Sparse checkout not supported — fall through to regular clone.
        // Remove any partial clone directory before retrying.
        let _ = std::fs::remove_dir_all(&clone_dir);
    }

    // Regular shallow clone — fetches all blobs at depth 1.
    let mut args: Vec<&str> = vec!["clone", "--depth", "1"];
    if let Some(r) = git_ref {
        args.push("--branch");
        args.push(r);
    }
    args.push(url);
    args.push(clone_str);

    let output = Command::new("git")
        .args(&args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|err| format!("failed to run git clone: {}", err))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone failed: {}", stderr.trim()));
    }
    Ok(())
}

/// `git rev-parse HEAD` in `dir` (the clone or a directory inside it). For a
/// clone of an annotated tag, `HEAD` is the tagged commit.
pub(crate) fn head_commit(dir: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|err| format!("failed to run git rev-parse: {}", err))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git rev-parse HEAD failed: {}", stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Why `run_until` did not return the output of a command.
#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)] // Caller: the update check (plan task 4).
pub(crate) enum RunError {
    /// The command had not exited at the deadline; it was killed.
    TimedOut,
    /// The command could not start or exited with a failure. Holds the first
    /// line of stderr, or the spawn error.
    Failed(String),
}

/// Run `cmd` and return its stdout. Kills the child and returns `TimedOut`
/// when it has not exited at `deadline`. `Failed` holds the first line of
/// stderr, or the spawn error.
///
/// The pipes are read after the child exits. A child that fills a pipe buffer
/// blocks, does not exit, and is killed at the deadline, so the deadline holds.
/// The callers print a few lines, below the pipe buffer size.
#[allow(dead_code)] // Caller: the update check (plan task 4).
pub(crate) fn run_until(mut cmd: Command, deadline: Instant) -> Result<String, RunError> {
    use std::io::Read;
    use std::process::Stdio;

    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| RunError::Failed(err.to_string()))?;

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::Failed(err.to_string()));
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RunError::TimedOut);
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        stdout = String::from_utf8_lossy(&bytes).into_owned();
    }
    if !status.success() {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = child.stderr.take() {
            let _ = pipe.read_to_end(&mut bytes);
        }
        let stderr = String::from_utf8_lossy(&bytes);
        let first = stderr.lines().next().unwrap_or("").trim();
        let message = if first.is_empty() {
            format!("command failed: {}", status)
        } else {
            first.to_string()
        };
        return Err(RunError::Failed(message));
    }
    Ok(stdout)
}

/// The commit that `git_ref` (or `HEAD` when `None`) names at `url`, from
/// `git ls-remote` without a fetch. `Ok(None)` when the remote has no such
/// ref. A branch wins over a tag of the same name; an annotated tag gives the
/// tagged commit.
#[allow(dead_code)] // Caller: the update check (plan task 4).
pub(crate) fn ls_remote(
    url: &str,
    git_ref: Option<&str>,
    deadline: Instant,
) -> Result<Option<String>, RunError> {
    let mut cmd = Command::new("git");
    cmd.arg("ls-remote").arg(url);
    match git_ref {
        None => {
            cmd.arg("HEAD");
        }
        Some(r) => {
            cmd.arg(format!("refs/heads/{r}"))
                .arg(format!("refs/tags/{r}"))
                .arg(format!("refs/tags/{r}^{{}}"));
        }
    }
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    let output = run_until(cmd, deadline)?;

    let lines: Vec<(&str, &str)> = output
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    let find = |name: &str| {
        lines
            .iter()
            .find(|(_, n)| *n == name)
            .map(|(hash, _)| hash.to_string())
    };
    Ok(match git_ref {
        None => find("HEAD"),
        Some(r) => find(&format!("refs/heads/{r}"))
            .or_else(|| find(&format!("refs/tags/{r}^{{}}")))
            .or_else(|| find(&format!("refs/tags/{r}"))),
    })
}

/// True for 40 lowercase or uppercase hexadecimal characters.
#[allow(dead_code)] // Caller: the update check (plan task 4).
pub(crate) fn is_full_commit_hash(git_ref: &str) -> bool {
    git_ref.len() == 40 && git_ref.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Attempt a sparse clone: `--depth 1 --filter=blob:none --sparse` followed
/// by `git sparse-checkout set <subfolder>`. Returns `Ok(())` on success or
/// an error if any step fails (caller should fall back to regular clone).
fn try_sparse_clone(
    url: &str,
    git_ref: Option<&str>,
    clone_str: &str,
    subfolder: &str,
) -> Result<(), String> {
    let mut args: Vec<&str> = vec!["clone", "--depth", "1", "--filter=blob:none", "--sparse"];
    if let Some(r) = git_ref {
        args.push("--branch");
        args.push(r);
    }
    args.push(url);
    args.push(clone_str);

    let output = Command::new("git")
        .args(&args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|err| format!("failed to run git clone --sparse: {}", err))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone --sparse failed: {}", stderr.trim()));
    }

    // Set the sparse-checkout cone to the target subfolder.
    let output = Command::new("git")
        .args(["sparse-checkout", "set", subfolder])
        .current_dir(clone_str)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|err| format!("failed to run git sparse-checkout set: {}", err))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git sparse-checkout set failed: {}", stderr.trim()));
    }

    Ok(())
}

pub(crate) fn resolve_subfolder(
    clone_dir: &Path,
    subfolder: Option<&str>,
    owner: &str,
    name: &str,
) -> Result<PathBuf, String> {
    if let Some(sub) = subfolder {
        // Reject absolute paths and obvious traversal attempts before joining.
        if sub.starts_with('/') || sub.starts_with('~') {
            return Err(format!(
                "subfolder '{}' in {}/{} must be a relative path (no leading '/' or '~')",
                sub, owner, name
            ));
        }

        let sub_path = clone_dir.join(sub);

        // Canonicalize both paths and verify the subfolder stays within the
        // clone directory. This catches `..` traversal and symlink escapes.
        let canon_base = clone_dir.canonicalize().map_err(|e| {
            format!(
                "failed to canonicalize clone dir {}: {}",
                clone_dir.display(),
                e
            )
        })?;
        let canon_sub = sub_path
            .canonicalize()
            .map_err(|_| format!("subfolder '{}' not found in {}/{}", sub, owner, name))?;
        if !canon_sub.starts_with(&canon_base) {
            return Err(format!(
                "subfolder '{}' in {}/{} escapes the repository root",
                sub, owner, name
            ));
        }

        // Subfolders are usually directories (item-root paths) but bundle
        // sources point at a `<name>.bundle.yaml` file directly. Accept
        // both — the install layer dispatches on file vs dir.
        Ok(sub_path)
    } else {
        Ok(clone_dir.to_path_buf())
    }
}

/// Copy skill files from a source directory into the canonical target.
/// Only copies files and directories, skipping `.git`.
pub fn copy_skills(src: &Path, dest: &Path) -> Result<(), String> {
    copy_dir_recursive(src, dest)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest)
        .map_err(|e| format!("failed to create {}: {}", dest.display(), e))?;

    let entries =
        std::fs::read_dir(src).map_err(|e| format!("failed to read {}: {}", src.display(), e))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read entry in {}: {}", src.display(), e))?;
        let name = entry.file_name();

        if name == ".git" {
            continue;
        }

        let src_path = entry.path();
        let dest_path = dest.join(&name);

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path).map_err(|e| {
                format!(
                    "failed to copy {} -> {}: {}",
                    src_path.display(),
                    dest_path.display(),
                    e
                )
            })?;
        }
    }

    Ok(())
}

/// Remove a temporary clone directory.
pub fn cleanup(path: &Path) {
    let _ = std::fs::remove_dir_all(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command as Cmd;

    /// Create a bare git repo with some skill files, returning the path.
    fn make_test_repo(root: &Path) -> PathBuf {
        let repo = root.join("test-repo");
        let work = root.join("work");

        // Clear git env vars that hooks may set
        let git = |args: &[&str], dir: Option<&Path>| {
            let mut cmd = Cmd::new("git");
            cmd.args(args);
            cmd.env_remove("GIT_DIR");
            cmd.env_remove("GIT_WORK_TREE");
            cmd.env_remove("GIT_INDEX_FILE");
            if let Some(d) = dir {
                cmd.current_dir(d);
            }
            let out = cmd.output().expect("git command");
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };

        git(
            &[
                "init",
                "--bare",
                "--initial-branch=main",
                repo.to_str().unwrap(),
            ],
            None,
        );
        git(
            &["clone", repo.to_str().unwrap(), work.to_str().unwrap()],
            None,
        );

        // Create skill files
        fs::create_dir_all(work.join("my-skill")).unwrap();
        fs::write(work.join("my-skill/SKILL.md"), "# My Skill").unwrap();
        fs::create_dir_all(work.join("catalog/lint")).unwrap();
        fs::write(work.join("catalog/lint/SKILL.md"), "# Lint").unwrap();

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
        // Ensure the branch is named "main" regardless of init.defaultBranch.
        git(&["push", "-u", "origin", "main"], Some(&work));

        repo
    }

    /// `git <args>` in `dir`, returning trimmed stdout.
    fn git_out(dir: &Path, args: &[&str]) -> String {
        let out = Cmd::new("git")
            .args(args)
            .current_dir(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("git command");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn far_deadline() -> Instant {
        Instant::now() + Duration::from_secs(30)
    }

    fn file_url(bare: &Path) -> String {
        format!("file://{}", bare.display())
    }

    /// Create a commit on top of `HEAD` in `bare` without a work tree.
    fn add_commit(bare: &Path) -> String {
        git_out(
            bare,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@test.com",
                "commit-tree",
                "HEAD^{tree}",
                "-p",
                "HEAD",
                "-m",
                "second",
            ],
        )
    }

    #[test]
    fn ls_remote_without_a_ref_is_the_head_commit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let got = ls_remote(&file_url(&bare), None, far_deadline()).expect("ls-remote");
        assert_eq!(got, Some(git_out(&bare, &["rev-parse", "HEAD"])));
    }

    #[test]
    fn ls_remote_of_a_branch_is_its_commit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let second = add_commit(&bare);
        git_out(&bare, &["branch", "dev", &second]);
        let got = ls_remote(&file_url(&bare), Some("dev"), far_deadline()).expect("ls-remote");
        assert_eq!(got, Some(second));
    }

    #[test]
    fn ls_remote_of_a_lightweight_tag_is_its_commit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        git_out(&bare, &["tag", "light"]);
        let got = ls_remote(&file_url(&bare), Some("light"), far_deadline()).expect("ls-remote");
        assert_eq!(got, Some(git_out(&bare, &["rev-parse", "HEAD"])));
    }

    #[test]
    fn ls_remote_of_an_annotated_tag_is_the_tagged_commit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        git_out(
            &bare,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@test.com",
                "tag",
                "-a",
                "v1",
                "-m",
                "v1",
            ],
        );
        let got = ls_remote(&file_url(&bare), Some("v1"), far_deadline()).expect("ls-remote");
        let commit = git_out(&bare, &["rev-parse", "v1^{commit}"]);
        assert_ne!(commit, git_out(&bare, &["rev-parse", "v1"]));
        assert_eq!(got, Some(commit));
    }

    #[test]
    fn ls_remote_of_a_branch_named_like_a_tag_is_the_branch() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let second = add_commit(&bare);
        git_out(&bare, &["tag", "v2"]);
        git_out(&bare, &["branch", "v2", &second]);
        let got = ls_remote(&file_url(&bare), Some("v2"), far_deadline()).expect("ls-remote");
        assert_eq!(got, Some(second));
    }

    #[test]
    fn ls_remote_of_an_unknown_ref_is_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let got = ls_remote(&file_url(&bare), Some("nope"), far_deadline()).expect("ls-remote");
        assert_eq!(got, None);
    }

    #[test]
    fn ls_remote_of_a_missing_repository_fails() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let url = file_url(&tmp.path().join("absent"));
        match ls_remote(&url, None, far_deadline()) {
            Err(RunError::Failed(message)) => assert!(!message.is_empty()),
            other => panic!("expected Failed, got {:?}", other),
        }
    }

    #[cfg(unix)]
    #[test]
    fn run_until_kills_a_command_that_passes_the_deadline() {
        let start = Instant::now();
        let mut cmd = Command::new("sleep");
        cmd.arg("5");
        let result = run_until(cmd, start + Duration::from_millis(200));
        assert_eq!(result, Err(RunError::TimedOut));
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn is_full_commit_hash_accepts_forty_hex_characters_only() {
        assert!(is_full_commit_hash(&"a".repeat(40)));
        assert!(is_full_commit_hash(
            "0123456789abcdefABCDEF0123456789abcdef01"
        ));
        assert!(!is_full_commit_hash(&"a".repeat(39)));
        assert!(!is_full_commit_hash(&"a".repeat(41)));
        assert!(!is_full_commit_hash("main"));
        assert!(!is_full_commit_hash(&format!("{}g", "a".repeat(39))));
    }

    #[test]
    fn head_commit_of_a_clone_is_the_commit_of_the_source() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();

        let url = format!("file://{}", bare.display());
        shallow_clone(&url, None, "cloned", &dest, None).expect("clone must succeed");

        let commit = head_commit(&dest.join("cloned")).expect("head commit");
        assert_eq!(commit, git_out(&bare, &["rev-parse", "HEAD"]));
        assert_eq!(commit.len(), 40);
    }

    #[test]
    fn head_commit_of_an_annotated_tag_clone_is_the_tagged_commit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        git_out(
            &bare,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@test.com",
                "tag",
                "-a",
                "v1",
                "-m",
                "v1",
            ],
        );
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();

        let url = format!("file://{}", bare.display());
        shallow_clone(&url, Some("v1"), "cloned", &dest, None).expect("clone must succeed");

        let commit = head_commit(&dest.join("cloned")).expect("head commit");
        assert_eq!(commit, git_out(&bare, &["rev-parse", "v1^{commit}"]));
    }

    #[test]
    fn shallow_clone_local_repo() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();

        let url = format!("file://{}", bare.display());
        shallow_clone(&url, None, "cloned", &dest, None).expect("clone must succeed");

        assert!(dest.join("cloned/my-skill/SKILL.md").exists());
    }

    #[test]
    fn clone_with_subfolder() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();

        let url = format!("file://{}", bare.display());
        shallow_clone(&url, None, "cloned", &dest, None).expect("clone");

        let sub = resolve_subfolder(&dest.join("cloned"), Some("catalog/lint"), "test", "repo")
            .expect("subfolder must resolve");

        assert!(sub.join("SKILL.md").exists());
    }

    #[test]
    fn clone_missing_subfolder_fails() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();

        let url = format!("file://{}", bare.display());
        shallow_clone(&url, None, "cloned", &dest, None).expect("clone");

        let err = resolve_subfolder(&dest.join("cloned"), Some("nonexistent"), "test", "repo")
            .expect_err("must fail");

        assert!(err.contains("subfolder 'nonexistent' not found"));
    }

    #[test]
    fn resolve_subfolder_rejects_absolute_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let clone_dir = tmp.path().join("cloned");
        fs::create_dir_all(&clone_dir).unwrap();

        let err = resolve_subfolder(&clone_dir, Some("/etc/passwd"), "test", "repo")
            .expect_err("absolute path must fail");
        assert!(err.contains("must be a relative path"));
    }

    #[test]
    fn resolve_subfolder_rejects_tilde_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let clone_dir = tmp.path().join("cloned");
        fs::create_dir_all(&clone_dir).unwrap();

        let err = resolve_subfolder(&clone_dir, Some("~/something"), "test", "repo")
            .expect_err("tilde path must fail");
        assert!(err.contains("must be a relative path"));
    }

    #[test]
    fn resolve_subfolder_rejects_dot_dot_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let clone_dir = tmp.path().join("cloned");
        fs::create_dir_all(&clone_dir).unwrap();
        // Create a sibling directory that .. would reach
        fs::create_dir_all(tmp.path().join("secret")).unwrap();

        let err = resolve_subfolder(&clone_dir, Some("../../etc"), "test", "repo")
            .expect_err("dot-dot escape must fail");
        // Either "not found" (doesn't exist) or "escapes" (exists but outside)
        assert!(err.contains("not found") || err.contains("escapes"));
    }

    #[test]
    fn resolve_subfolder_allows_internal_dot_dot() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let clone_dir = tmp.path().join("cloned");
        fs::create_dir_all(clone_dir.join("a/b")).unwrap();
        fs::create_dir_all(clone_dir.join("c")).unwrap();

        // a/../c resolves to c which is still inside clone_dir
        let result = resolve_subfolder(&clone_dir, Some("a/../c"), "test", "repo");
        assert!(result.is_ok());
    }

    #[test]
    fn copy_skills_skips_dot_git() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");

        fs::create_dir_all(src.join(".git")).unwrap();
        fs::write(src.join(".git/HEAD"), "ref: refs/heads/main").unwrap();
        fs::create_dir_all(src.join("my-skill")).unwrap();
        fs::write(src.join("my-skill/SKILL.md"), "# My Skill").unwrap();

        copy_skills(&src, &dest).expect("copy must succeed");

        assert!(dest.join("my-skill/SKILL.md").exists());
        assert!(!dest.join(".git").exists());
    }

    #[test]
    fn copy_skills_preserves_nested_dirs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");

        fs::create_dir_all(src.join("a/b/c")).unwrap();
        fs::write(src.join("a/b/c/file.txt"), "content").unwrap();

        copy_skills(&src, &dest).expect("copy must succeed");

        assert_eq!(
            fs::read_to_string(dest.join("a/b/c/file.txt")).unwrap(),
            "content"
        );
    }

    #[test]
    fn sparse_clone_fetches_only_subfolder() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();

        let url = format!("file://{}", bare.display());
        // Clone with subfolder hint — only "catalog/lint" should be checked out
        shallow_clone(&url, None, "cloned", &dest, Some("catalog/lint"))
            .expect("sparse clone must succeed");

        // The subfolder content must exist
        assert!(dest.join("cloned/catalog/lint/SKILL.md").exists());
        // Content outside the sparse cone must NOT be checked out
        assert!(!dest.join("cloned/my-skill/SKILL.md").exists());
    }

    #[test]
    fn sparse_clone_with_git_ref() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();

        let url = format!("file://{}", bare.display());
        // Sparse clone pinned to a branch (the default branch pushed by make_test_repo)
        shallow_clone(&url, Some("main"), "cloned", &dest, Some("catalog/lint"))
            .expect("sparse clone with ref must succeed");

        assert!(dest.join("cloned/catalog/lint/SKILL.md").exists());
        assert!(!dest.join("cloned/my-skill/SKILL.md").exists());
    }

    #[test]
    fn shallow_clone_without_subfolder_fetches_everything() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();

        let url = format!("file://{}", bare.display());
        // Clone without subfolder — should get everything (backward compat)
        shallow_clone(&url, None, "cloned", &dest, None).expect("clone must succeed");

        assert!(dest.join("cloned/my-skill/SKILL.md").exists());
        assert!(dest.join("cloned/catalog/lint/SKILL.md").exists());
    }

    #[test]
    fn cleanup_removes_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("to-clean");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("file"), "data").unwrap();

        cleanup(&dir);

        assert!(!dir.exists());
    }
}
