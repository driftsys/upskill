//! Applying-only selected rule sources. No discovery, destination writes or registration.
use super::desired::{
    CompilerBinding, ContractedDesiredSet, RuleDesiredSetRequest,
    compile_rule_desired_set_with_contracts,
};
use super::state::{Installation, SourceEvidence};
use crate::filesystem::{QualifiedScope, SourceSnapshot};
use crate::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use crate::pipeline::ItemKind;
use crate::source::InstallSource;
use crate::target::{TargetProfile, TargetSurface};
use anyhow::{Context, Result, ensure};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Holds qualified source identity and any private temporary checkout until dropped.
/// Preview must use retained state and must not resolve sources through this API.
pub struct ResolvedRuleSource {
    scope: QualifiedScope,
    folder_name: String,
    label: String,
    revision: Option<String>,
    checkout: Option<tempfile::TempDir>,
}

/// Explicit installation binding; source identity and revision come from capture.
pub struct RuleSourceBinding<'a> {
    pub installation_id: &'a str,
    pub expected_source_name: &'a str,
    pub installed_name: &'a str,
    pub root: &'a InstallationRoot,
    pub requested: &'a [TargetSurface],
    pub bindings: &'a [CompilerBinding],
    pub profiles: &'a [TargetProfile],
}

/// Evidence cannot be detached from the resolver that supplied its Git metadata.
pub struct CapturedRuleSource<'a> {
    resolved: &'a ResolvedRuleSource,
    snapshot: SourceSnapshot<'a>,
}

impl ResolvedRuleSource {
    /// Resolve one explicitly selected item root, with no application writes.
    pub fn resolve(source: &InstallSource) -> Result<Self> {
        if let InstallSource::LocalPath(path) = source {
            let path = absolute_selected_root(path)?;
            let scope = qualify(path.clone())?;
            return Ok(Self {
                scope,
                folder_name: folder_name(&path)?,
                label: InstallSource::LocalPath(path).to_string(),
                revision: None,
                checkout: None,
            });
        }
        let (url, reference, subfolder, repository_name) = match source {
            InstallSource::Github(repo) => (
                format!("https://github.com/{}/{}.git", repo.owner, repo.name),
                repo.git_ref.as_deref(),
                repo.subfolder.as_deref(),
                repo.name.clone(),
            ),
            InstallSource::Git(repo) => (
                repo.url.clone(),
                repo.git_ref.as_deref(),
                repo.subfolder.as_deref(),
                repo.url
                    .rsplit('/')
                    .next()
                    .context("missing repository name")?
                    .trim_end_matches(".git")
                    .to_owned(),
            ),
            InstallSource::LocalPath(_) => unreachable!(),
        };
        let checkout = tempfile::tempdir().context("create private source checkout")?;
        std::fs::create_dir(checkout.path().join("tree"))?;
        git(&checkout.path().join("tree"), &["init", "--quiet"])?;
        git(
            &checkout.path().join("tree"),
            &["remote", "add", "--", "origin", &url],
        )?;
        git(
            &checkout.path().join("tree"),
            &[
                "fetch",
                "--quiet",
                "--depth=1",
                "--",
                "origin",
                reference.unwrap_or("HEAD"),
            ],
        )?;
        git(
            &checkout.path().join("tree"),
            &["checkout", "--quiet", "--detach", "FETCH_HEAD", "--"],
        )?;
        let revision = git_revision(&checkout.path().join("tree"))?;
        let path = if let Some(subfolder) = subfolder {
            let relative = Path::new(subfolder);
            ensure!(
                !relative.as_os_str().is_empty()
                    && relative
                        .components()
                        .all(|c| matches!(c, Component::Normal(_))),
                "source subfolder must be a contained relative path"
            );
            let path = checkout.path().join("tree").join(relative);
            ensure!(
                path.canonicalize()?
                    .starts_with(checkout.path().join("tree").canonicalize()?),
                "source subfolder escapes checkout"
            );
            path
        } else {
            checkout.path().join("tree")
        };
        let folder_name = if subfolder.is_some() {
            folder_name(&path)?
        } else {
            repository_name
        };
        let scope = qualify(path)?;
        Ok(Self {
            scope,
            folder_name,
            label: source.to_string(),
            revision: Some(revision),
            checkout: Some(checkout),
        })
    }

    /// Capture strict source bytes and sample the checkout revision around capture.
    pub fn capture(&self) -> Result<CapturedRuleSource<'_>> {
        self.verify_revision()?;
        let snapshot = self.scope.observe_source_tree()?;
        self.verify_revision()?;
        Ok(CapturedRuleSource {
            resolved: self,
            snapshot,
        })
    }

    fn verify_revision(&self) -> Result<()> {
        if let Some(checkout) = &self.checkout {
            ensure!(
                Some(git_revision(&checkout.path().join("tree"))?) == self.revision,
                "source checkout revision changed"
            );
        }
        Ok(())
    }
}

impl CapturedRuleSource<'_> {
    pub fn snapshot(&self) -> &SourceSnapshot<'_> {
        &self.snapshot
    }

    /// Sample retained source and checkout identity without refreshing evidence.
    /// This does not exclude concurrent external writers or provide an atomic snapshot.
    pub fn verify_unchanged(&self) -> Result<()> {
        self.resolved.verify_revision()?;
        self.snapshot.verify_unchanged()?;
        self.resolved.verify_revision()
    }

    /// Compile complete desired artifacts and contracts from captured source bytes.
    pub fn compile(&self, binding: RuleSourceBinding<'_>) -> Result<ContractedDesiredSet> {
        self.verify_unchanged()?;
        let hash = self.snapshot.manifest().content_sha256.clone();
        let source_evidence = match &self.resolved.revision {
            Some(revision) => SourceEvidence::Git {
                resolved_revision: revision.clone(),
                manifest_sha256: hash,
            },
            None => SourceEvidence::Local {
                manifest_sha256: hash,
            },
        };
        let installation = Installation {
            owner: ArtifactOwner {
                installation_id: binding.installation_id.into(),
                source: self.resolved.label.clone(),
                source_revision: self.resolved.revision.clone(),
                kind: ItemKind::Rule,
                source_name: binding.expected_source_name.into(),
                installed_name: binding.installed_name.into(),
            },
            root: binding.root.clone(),
            requested: binding.requested.to_vec(),
            source_evidence,
        };
        compile_rule_desired_set_with_contracts(
            RuleDesiredSetRequest {
                installation: &installation,
                source: self.snapshot.manifest(),
                folder_name: &self.resolved.folder_name,
                bindings: binding.bindings,
            },
            binding.profiles,
        )
    }
}

fn qualify(path: PathBuf) -> Result<QualifiedScope> {
    QualifiedScope::open(&InstallationRoot {
        scope: OwnershipScope::Project,
        path,
    })
}
fn folder_name(path: &Path) -> Result<String> {
    Ok(path
        .file_name()
        .and_then(|name| name.to_str())
        .context("source root requires a UTF-8 folder name")?
        .to_owned())
}
fn absolute_selected_root(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    // Resolve ancestors only: canonicalizing the selected root would hide final symlinks.
    let name = absolute
        .file_name()
        .context("source must select a named item root")?;
    Ok(absolute
        .parent()
        .context("source has no parent")?
        .canonicalize()?
        .join(name))
}
// Repository-local overrides follow `git rev-parse --local-env-vars`. Authentication
// through HOME/global/system config, credential helpers, SSH and askpass stays native.
const REPOSITORY_LOCAL_GIT_ENV: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
    "GIT_INTERNAL_SUPER_PREFIX",
];
fn source_git_command(root: &Path, args: &[&str]) -> Result<Command> {
    let mut command = Command::new("git");
    for name in REPOSITORY_LOCAL_GIT_ENV {
        command.env_remove(name);
    }
    command
        .current_dir(root)
        .arg("--git-dir")
        .arg(
            root.parent()
                .context("checkout has no parent")?
                .join("metadata"),
        )
        .arg("--work-tree")
        .arg(root)
        .args(args);
    Ok(command)
}
fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = source_git_command(root, args)?
        .output()
        .context("run source git command")?;
    ensure!(
        output.status.success(),
        "source git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
fn git_revision(root: &Path) -> Result<String> {
    git(root, &["rev-parse", "--verify", "HEAD^{commit}"])
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use super::*;
    #[test]
    fn cleared_environment_covers_installed_git_repository_local_overrides() {
        let mut query = Command::new("git");
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_") {
                query.env_remove(key);
            }
        }
        let output = query
            .args(["rev-parse", "--local-env-vars"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let command =
            source_git_command(Path::new("/private/checkout/tree"), &["rev-parse", "HEAD"])
                .unwrap();
        let removed: std::collections::BTreeSet<_> = command
            .get_envs()
            .filter_map(|(name, value)| {
                value
                    .is_none()
                    .then_some(name.to_string_lossy().into_owned())
            })
            .collect();
        for name in REPOSITORY_LOCAL_GIT_ENV {
            assert!(
                removed.contains(*name),
                "command retains repository-local override: {name}"
            );
        }
        for name in String::from_utf8(output.stdout).unwrap().lines() {
            assert!(
                REPOSITORY_LOCAL_GIT_ENV.contains(&name),
                "uncleared repository-local variable: {name}"
            );
        }
    }
    #[test]
    fn revision_change_after_capture_rejects_even_with_equal_source_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let tree = temp.path().join("tree");
        std::fs::create_dir(&tree).unwrap();
        git(&tree, &["init", "--quiet"]).unwrap();
        std::fs::write(
            tree.join("RULE.md"),
            b"---\nschema: 1\nname: source\ndescription: demo\n---\n\nCaptured\n",
        )
        .unwrap();
        git(&tree, &["add", "--", "RULE.md"]).unwrap();
        let commit = [
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "fixture",
        ];
        git(&tree, &commit).unwrap();
        let resolved = ResolvedRuleSource {
            scope: qualify(tree.clone()).unwrap(),
            folder_name: "rule".into(),
            label: "fixture".into(),
            revision: Some(git_revision(&tree).unwrap()),
            checkout: Some(temp),
        };
        let captured = resolved.capture().unwrap();
        let destination = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tree.parent().unwrap().join("destination"),
        };
        let requested = [TargetSurface::ClaudeCli];
        let bindings = [CompilerBinding {
            client: crate::generate::Client::Claude,
            consumers: requested.to_vec(),
        }];
        let profiles = [TargetProfile::new(
            TargetSurface::ClaudeCli.family(),
            TargetSurface::ClaudeCli,
            1,
        )
        .unwrap()];
        let binding = || RuleSourceBinding {
            installation_id: "revision-test",
            expected_source_name: "source",
            installed_name: "alias",
            root: &destination,
            requested: &requested,
            bindings: &bindings,
            profiles: &profiles,
        };
        captured.compile(binding()).unwrap();
        git(&tree, &commit).unwrap();
        captured.snapshot().verify_unchanged().unwrap();
        assert!(captured.verify_unchanged().is_err());
        assert!(captured.compile(binding()).is_err());
    }
    #[test]
    fn temporary_checkout_is_retained_and_removed_on_drop() {
        let temp = tempfile::tempdir().unwrap();
        let private_path = temp.path().to_owned();
        std::fs::create_dir(private_path.join("tree")).unwrap();
        git(&private_path.join("tree"), &["init", "--quiet"]).unwrap();
        let resolved = ResolvedRuleSource {
            scope: qualify(private_path.join("tree")).unwrap(),
            folder_name: "rule".into(),
            label: "fixture".into(),
            revision: None,
            checkout: Some(temp),
        };
        assert!(private_path.exists());
        drop(resolved);
        assert!(!private_path.exists());
    }
}
