use std::path::{Path, PathBuf};
use std::process::Command;

use crate::source::GithubRepo;

/// Fetch a GitHub repository via shallow `git clone`, returning the
/// path to the cloned working tree. The caller is responsible for
/// cleaning up the returned directory.
pub fn clone_github_repo(repo: &GithubRepo, dest: &Path) -> Result<PathBuf, String> {
    let url = format!("https://github.com/{}/{}.git", repo.owner, repo.name);
    shallow_clone(&url, repo.git_ref.as_deref(), &repo.name, dest)?;
    let clone_dir = dest.join(&repo.name);
    resolve_subfolder(
        &clone_dir,
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
    )
}

/// Shallow clone a git URL into `dest/<dir_name>`.
fn shallow_clone(
    url: &str,
    git_ref: Option<&str>,
    dir_name: &str,
    dest: &Path,
) -> Result<(), String> {
    let clone_dir = dest.join(dir_name);
    let clone_str = clone_dir
        .to_str()
        .ok_or_else(|| "clone path is not valid UTF-8".to_string())?;

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

fn resolve_subfolder(
    clone_dir: &Path,
    subfolder: Option<&str>,
    owner: &str,
    name: &str,
) -> Result<PathBuf, String> {
    if let Some(sub) = subfolder {
        let sub_path = clone_dir.join(sub);
        if !sub_path.is_dir() {
            return Err(format!(
                "subfolder '{}' not found in {}/{}",
                sub, owner, name
            ));
        }
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

        git(&["init", "--bare", repo.to_str().unwrap()], None);
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
        git(&["push"], Some(&work));

        repo
    }

    #[test]
    fn shallow_clone_local_repo() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();

        let url = format!("file://{}", bare.display());
        shallow_clone(&url, None, "cloned", &dest).expect("clone must succeed");

        assert!(dest.join("cloned/my-skill/SKILL.md").exists());
    }

    #[test]
    fn clone_with_subfolder() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bare = make_test_repo(tmp.path());
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();

        let url = format!("file://{}", bare.display());
        shallow_clone(&url, None, "cloned", &dest).expect("clone");

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
        shallow_clone(&url, None, "cloned", &dest).expect("clone");

        let err = resolve_subfolder(&dest.join("cloned"), Some("nonexistent"), "test", "repo")
            .expect_err("must fail");

        assert!(err.contains("subfolder 'nonexistent' not found"));
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
    fn cleanup_removes_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("to-clean");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("file"), "data").unwrap();

        cleanup(&dir);

        assert!(!dir.exists());
    }
}
