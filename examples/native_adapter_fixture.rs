//! Prepare isolated Claude or Copilot contract inputs without native calls.
use anyhow::{Context, Result, ensure};
use std::path::{Component, Path, PathBuf};
use upskill::compile::skill_package::{SkillPackageRequest, compile_skill_package};
use upskill::compile::skill_selection::{SkillSelectionRequest, select_skill};
use upskill::lifecycle::native_receipt::{NativeSkillSelection, SkillTarget};
use upskill::native_delivery::adapter::{
    NativeActionKind, NativeDeliveryProfile, NativeExposureOrder,
};
use upskill::native_delivery::claude::{ClaudePluginAction, plan_plugin_command};
use upskill::native_delivery::copilot::{
    CopilotPluginAction, plan_plugin_command as plan_copilot_command,
};
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::source_manifest::capture_source;
use upskill::target::{
    CapabilityCategory, CapabilityDeclaration, CapabilityKey, Representation, Requirement,
    TargetSurface,
};

fn directory(path: &Path) -> Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    directory(path.parent().context("fixture path has no parent")?)?;
    std::fs::write(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
fn arguments(mut arguments: impl Iterator<Item = std::ffi::OsString>) -> Result<PathBuf> {
    let root = PathBuf::from(
        arguments
            .next()
            .context("usage: native_adapter_fixture [--copilot] ABSENT_ABSOLUTE_ROOT")?,
    );
    ensure!(arguments.next().is_none(), "unexpected fixture arguments");
    Ok(root)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureClient {
    Claude,
    Copilot,
}
fn request(
    arguments: impl Iterator<Item = std::ffi::OsString>,
) -> Result<(FixtureClient, PathBuf)> {
    let mut arguments = arguments.peekable();
    let client = if arguments.peek().is_some_and(|value| value == "--copilot") {
        arguments.next();
        FixtureClient::Copilot
    } else {
        FixtureClient::Claude
    };
    Ok((client, self::arguments(arguments)?))
}
fn prepare_copilot(root: &Path) -> Result<PathBuf> {
    prepare_fixture(root, FixtureClient::Copilot)
}
fn prepare_request(arguments: impl Iterator<Item = std::ffi::OsString>) -> Result<PathBuf> {
    let (client, root) = request(arguments)?;
    match client {
        FixtureClient::Claude => prepare(&root),
        FixtureClient::Copilot => prepare_copilot(&root),
    }
}
fn main() -> Result<()> {
    let plan = prepare_request(std::env::args_os().skip(1))?;
    println!("{}", plan.display());
    Ok(())
}
fn prepare(root: &Path) -> Result<PathBuf> {
    prepare_fixture(root, FixtureClient::Claude)
}
fn prepare_fixture(root: &Path, client: FixtureClient) -> Result<PathBuf> {
    ensure!(root.is_absolute(), "fixture root must be absolute");
    ensure!(
        !root.components().any(|part| part == Component::ParentDir),
        "fixture root must not contain parent components"
    );
    ensure!(
        std::fs::symlink_metadata(root)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "fixture root must be absent"
    );
    std::fs::create_dir(root)?;
    directory(root)?;
    let root = root.canonicalize()?;
    for name in ["home", "project", "source", "outside"] {
        directory(&root.join(name))?;
    }
    write(&root.join("source/SKILL.md"), b"---\nschema: 1\ndescription: Native adapter contract fixture.\n---\nRead the local resource when explicitly requested.\n")?;
    write(
        &root.join("source/resources/probe.txt"),
        b"Native contract resource. No model request is part of this fixture.\n",
    )?;
    let source = capture_source(&root.join("source"))?;
    let (surface, version, scope) = match client {
        FixtureClient::Claude => (
            TargetSurface::ClaudeCli,
            "2.1.308",
            InstallationRoot {
                scope: OwnershipScope::Project,
                path: root.join("project"),
            },
        ),
        FixtureClient::Copilot => (
            TargetSurface::CopilotCli,
            "1.0.105",
            InstallationRoot {
                scope: OwnershipScope::User,
                path: root.join("home"),
            },
        ),
    };
    let capabilities = ["name", "description"].map(|name| CapabilityDeclaration {
        surface,
        key: CapabilityKey::new(CapabilityCategory::Skills, name),
        representation: Representation::Native,
        requirement: Requirement::RequiredBehavior,
        reason: "offline generated contract fixture",
    });
    let selected = select_skill(&SkillSelectionRequest {
        source: &source,
        folder_name: "contract-probe",
        alias: None,
        surface,
        capabilities: &capabilities,
    })?
    .context("fixture was excluded from the requested client")?;
    let owner = ArtifactOwner {
        installation_id: "native-contract-probe".into(),
        source: "./contract-fixture".into(),
        source_revision: None,
        kind: ItemKind::Skill,
        source_name: selected.source_name.clone(),
        installed_name: selected.installed_name.clone(),
    };
    let package = compile_skill_package(&SkillPackageRequest {
        selected: &selected,
        surface,
        scope: &scope,
        owner: &owner,
    })?;
    for file in package
        .revision_files
        .iter()
        .chain(package.catalog_file.iter())
    {
        write(&scope.path.join(&file.path), &file.bytes)?;
    }
    write(&root.join("project/.claude/settings.json"), b"{\n  \"companyAnnouncements\": [\n    \"contract-preservation-sentinel\"\n  ],\n  \"enabledPlugins\": {},\n  \"extraKnownMarketplaces\": {}\n}\n")?;
    if client == FixtureClient::Copilot {
        write(
            &root.join("home/manual-preservation.txt"),
            b"Manual fixture sentinel.\n",
        )?;
    }
    let catalog = scope.path.join(&package.item_root);
    let profile = NativeDeliveryProfile {
        id: "unqualified-local-catalog-probe".into(),
        revision: 1,
        surface,
        scope: scope.clone(),
        client_version: version.into(),
        platform: "unqualified-fixture".into(),
        exposure: NativeExposureOrder::AfterNative,
        action: NativeActionKind::Command,
    };
    let target = SkillTarget {
        installation_id: owner.installation_id.clone(),
        surface,
    };
    let selection = NativeSkillSelection::from_package(&package);
    let commands = match client {
        FixtureClient::Claude => {
            let command = |action| -> Result<serde_json::Value> {
                let planned = plan_plugin_command(&profile, &target, &selection, action)?;
                let arguments = planned
                    .arguments()
                    .iter()
                    .map(|value| value.to_str().context("non-UTF-8 fixture argument"))
                    .collect::<Result<Vec<_>>>()?;
                Ok(serde_json::to_value(arguments)?)
            };
            serde_json::json!([
                ["--version"],
                ["plugin", "list", "--json"],
                ["plugin", "marketplace", "list", "--json"],
                command(ClaudePluginAction::ValidateCatalog)?,
                command(ClaudePluginAction::RegisterMarketplace)?,
                command(ClaudePluginAction::Install)?,
                ["plugin", "list", "--json"],
                ["plugin", "marketplace", "list", "--json"],
                command(ClaudePluginAction::Uninstall)?,
                command(ClaudePluginAction::UnregisterMarketplace)?,
                ["plugin", "list", "--json"],
                ["plugin", "marketplace", "list", "--json"]
            ])
        }
        FixtureClient::Copilot => {
            let command = |action| -> Result<serde_json::Value> {
                let planned = plan_copilot_command(&profile, &target, &selection, action)?;
                let arguments = planned
                    .arguments()
                    .iter()
                    .map(|value| value.to_str().context("non-UTF-8 fixture argument"))
                    .collect::<Result<Vec<_>>>()?;
                Ok(serde_json::to_value(arguments)?)
            };
            serde_json::json!([
                ["--no-auto-update", "--version"],
                ["--no-auto-update", "plugin", "list", "--json"],
                [
                    "--no-auto-update",
                    "plugin",
                    "marketplace",
                    "list",
                    "--json"
                ],
                command(CopilotPluginAction::RegisterMarketplace)?,
                command(CopilotPluginAction::Install)?,
                ["--no-auto-update", "plugin", "list", "--json"],
                [
                    "--no-auto-update",
                    "plugin",
                    "marketplace",
                    "list",
                    "--json"
                ],
                command(CopilotPluginAction::Uninstall)?,
                command(CopilotPluginAction::UnregisterMarketplace)?,
                ["--no-auto-update", "plugin", "list", "--json"],
                [
                    "--no-auto-update",
                    "plugin",
                    "marketplace",
                    "list",
                    "--json"
                ]
            ])
        }
    };
    let plan = serde_json::json!({"status":"prepared-only; native execution requires approval", "client_version":version,
        "scope":if client==FixtureClient::Claude {"project"} else {"user"},"scope_root":scope.path,
        "model_calls":0,"output_limit_bytes":1048576,"maximum_authorized_spending":0,"maximum_native_commands":if client==FixtureClient::Claude {12} else {11},"timeout_seconds_per_command":20,
        "packet_root":root,"cwd":root.join("project"),"home":root.join("home"),"catalog":catalog,
        "native_name":package.native_name,"version":package.version,"revision_root":package.revision_root,
        "payload_sha256":package.payload_sha256,"package_sha256":package.package_sha256,"commands":commands});
    write(&root.join("plan.json"), &serde_json::to_vec_pretty(&plan)?)?;
    Ok(root.join("plan.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};

    fn files(root: &Path) -> Vec<PathBuf> {
        fn visit(path: &Path, result: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(&path, result);
                } else {
                    result.push(path);
                }
            }
        }
        let mut result = vec![];
        visit(root, &mut result);
        result.sort();
        result
    }
    #[test]
    fn argument_and_root_refusals_preserve_existing_content() {
        assert!(arguments(std::iter::empty()).is_err());
        assert!(arguments(["one".into(), "two".into()].into_iter()).is_err());
        assert_eq!(
            arguments(["one".into()].into_iter()).unwrap(),
            PathBuf::from("one")
        );
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("manual"), b"manual bytes").unwrap();
        assert!(prepare(temp.path()).is_err());
        assert_eq!(
            std::fs::read(temp.path().join("manual")).unwrap(),
            b"manual bytes"
        );
        assert!(!temp.path().join("project").exists());
        std::fs::create_dir(temp.path().join("unused")).unwrap();
        assert!(prepare(&temp.path().join("unused/../packet")).is_err());
        assert!(!temp.path().join("packet").exists());
        assert_eq!(
            std::fs::read_dir(temp.path().join("unused"))
                .unwrap()
                .count(),
            0
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("missing-target", temp.path().join("link")).unwrap();
            assert!(prepare(&temp.path().join("link")).is_err());
            assert!(
                std::fs::symlink_metadata(temp.path().join("link"))
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        }
    }
    #[test]
    #[ignore = "invoked by the parent with an isolated working directory"]
    fn relative_root_probe() {
        assert!(std::env::var_os("UPSKILL_FIXTURE_ROOT_PROBE").is_some());
        assert!(prepare(Path::new("relative-fixture")).is_err());
        assert!(!Path::new("relative-fixture").exists());
    }
    #[test]
    fn relative_root_refuses_in_an_owned_child_working_directory() {
        let temp = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "tests::relative_root_probe",
                "--nocapture",
            ])
            .current_dir(temp.path())
            .env_clear()
            .env("HOME", temp.path())
            .env("UPSKILL_FIXTURE_ROOT_PROBE", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        assert!(!temp.path().join("relative-fixture").exists());
    }
    #[test]
    fn fixture_publishes_exact_package_bytes_and_a_bounded_project_sequence() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("packet");
        let plan_path = prepare_request([root.as_os_str().to_owned()].into_iter()).unwrap();
        let root = root.canonicalize().unwrap();
        let plan: Value = serde_json::from_slice(&std::fs::read(plan_path).unwrap()).unwrap();
        assert_eq!(plan["model_calls"], 0);
        assert_eq!(plan["maximum_native_commands"], 12);
        assert_eq!(plan["timeout_seconds_per_command"], 20);
        assert_eq!(plan["client_version"], "2.1.308");
        let name = plan["native_name"].as_str().unwrap();
        assert_eq!(name.len(), 72);
        assert!(name.starts_with("upskill-"));
        let payload = plan["payload_sha256"].as_str().unwrap();
        assert_eq!(payload.len(), 64);
        assert_eq!(plan["version"], format!("0.0.0-r{payload}"));
        let item = ".upskill/exports/claude/native-contract-probe";
        assert_eq!(plan["revision_root"], format!("{item}/revisions/{payload}"));
        let project = root.join("project");
        let revision = project.join(plan["revision_root"].as_str().unwrap());
        let catalog = project.join(item);
        assert_eq!(plan["cwd"], json!(project));
        assert_eq!(plan["home"], json!(root.join("home")));
        assert_eq!(plan["catalog"], json!(catalog));
        let manifest: Value = serde_json::from_slice(
            &std::fs::read(revision.join(".claude-plugin/plugin.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            manifest,
            json!({"name": name, "description": "Native adapter contract fixture.", "version": plan["version"]})
        );
        let marketplace: Value = serde_json::from_slice(
            &std::fs::read(catalog.join(".claude-plugin/marketplace.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            marketplace,
            json!({"name": name,"owner":{"name":"upskill"},"plugins":[{"name":name,"source":format!("./revisions/{payload}")}]})
        );
        let entrypoint = b"---\nname: contract-probe\ndescription: Native adapter contract fixture.\n---\n\nRead the local resource when explicitly requested.\n";
        assert_eq!(
            std::fs::read(revision.join("skills/contract-probe/SKILL.md")).unwrap(),
            entrypoint
        );
        assert_eq!(
            std::fs::read(revision.join("skills/contract-probe/resources/probe.txt")).unwrap(),
            b"Native contract resource. No model request is part of this fixture.\n"
        );
        let mut hash = Sha256::new();
        hash.update(b"upskill:skill-package-files:v1\0");
        hash.update(3_u64.to_be_bytes());
        for (path, mode) in [
            (".claude-plugin/plugin.json", None),
            ("skills/contract-probe/SKILL.md", None),
            (
                "skills/contract-probe/resources/probe.txt",
                if cfg!(unix) { Some(0o600_u32) } else { None },
            ),
        ] {
            let bytes = std::fs::read(revision.join(path)).unwrap();
            for value in [path.as_bytes(), bytes.as_slice()] {
                hash.update((value.len() as u64).to_be_bytes());
                hash.update(value);
            }
            match mode {
                None => hash.update([0]),
                Some(mode) => {
                    hash.update([1]);
                    hash.update(mode.to_be_bytes());
                }
            }
        }
        let package_hash = hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(plan["package_sha256"], package_hash);
        assert_eq!(std::fs::read(project.join(".claude/settings.json")).unwrap(), b"{\n  \"companyAnnouncements\": [\n    \"contract-preservation-sentinel\"\n  ],\n  \"enabledPlugins\": {},\n  \"extraKnownMarketplaces\": {}\n}\n");
        let selector = format!("{name}@{name}");
        assert_eq!(
            plan["commands"],
            json!([
                ["--version"],
                ["plugin", "list", "--json"],
                ["plugin", "marketplace", "list", "--json"],
                ["plugin", "validate", catalog],
                [
                    "plugin",
                    "marketplace",
                    "add",
                    catalog,
                    "--scope",
                    "project",
                    "--json"
                ],
                [
                    "plugin", "install", selector, "--scope", "project", "--json"
                ],
                ["plugin", "list", "--json"],
                ["plugin", "marketplace", "list", "--json"],
                [
                    "plugin",
                    "uninstall",
                    selector,
                    "--scope",
                    "project",
                    "--json"
                ],
                [
                    "plugin",
                    "marketplace",
                    "remove",
                    name,
                    "--scope",
                    "project",
                    "--json"
                ],
                ["plugin", "list", "--json"],
                ["plugin", "marketplace", "list", "--json"]
            ])
        );
        let before: Vec<_> = files(&root)
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect();
        assert_eq!(before.len(), 8);
        assert!(prepare(&root).is_err());
        let after: Vec<_> = files(&root)
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect();
        assert_eq!(after, before);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fn check(path: &Path) {
                let metadata = std::fs::metadata(path).unwrap();
                assert_eq!(
                    metadata.permissions().mode() & 0o777,
                    if metadata.is_dir() { 0o700 } else { 0o600 },
                    "{}",
                    path.display()
                );
                if metadata.is_dir() {
                    for entry in std::fs::read_dir(path).unwrap() {
                        check(&entry.unwrap().path());
                    }
                }
            }
            check(&root);
        }
    }
    // Catches ambiguous flag handling, accepting extra paths or selecting Claude
    // for an explicit Copilot request. Default one-root syntax remains valid.
    #[test]
    fn copilot_request_is_explicit_and_default_request_remains_claude() {
        assert_eq!(
            request(["--copilot".into(), "one".into()].into_iter()).unwrap(),
            (FixtureClient::Copilot, PathBuf::from("one"))
        );
        assert_eq!(
            request(["one".into()].into_iter()).unwrap(),
            (FixtureClient::Claude, PathBuf::from("one"))
        );
        assert!(request(["--copilot".into()].into_iter()).is_err());
        assert!(request(["--copilot".into(), "one".into(), "two".into()].into_iter()).is_err());
    }
    // Catches using project ownership, shortening native names, losing package
    // resources, changing command scope, adding unbounded calls or overwriting.
    #[test]
    fn copilot_fixture_keeps_full_names_private_user_scope_and_fixed_commands() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("copilot-packet");
        let plan_path =
            prepare_request(["--copilot".into(), root.as_os_str().to_owned()].into_iter()).unwrap();
        let root = root.canonicalize().unwrap();
        let plan: Value = serde_json::from_slice(&std::fs::read(plan_path).unwrap()).unwrap();
        let home = root.join("home");
        assert_eq!(plan["client_version"], "1.0.105");
        assert_eq!(plan["scope"], "user");
        assert_eq!(plan["scope_root"], json!(home));
        assert_eq!(plan["cwd"], json!(root.join("project")));
        assert_eq!(plan["home"], json!(home));
        assert_eq!(plan["model_calls"], 0);
        assert_eq!(plan["maximum_native_commands"], 11);
        assert_eq!(plan["output_limit_bytes"], 1048576);
        assert_eq!(plan["maximum_authorized_spending"], 0);
        assert_eq!(plan["timeout_seconds_per_command"], 20);
        let name = plan["native_name"].as_str().unwrap();
        assert_eq!(name.len(), 72);
        assert!(name.starts_with("upskill-"));
        assert!(
            name[8..]
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        );
        let payload = plan["payload_sha256"].as_str().unwrap();
        assert_eq!(payload.len(), 64);
        let item = ".upskill/exports/copilot/native-contract-probe";
        assert_eq!(plan["revision_root"], format!("{item}/revisions/{payload}"));
        let revision = home.join(plan["revision_root"].as_str().unwrap());
        let catalog = home.join(item);
        assert_eq!(plan["catalog"], json!(catalog));
        assert_eq!(plan["version"], format!("0.0.0-r{payload}"));
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(revision.join(".plugin/plugin.json")).unwrap())
                .unwrap();
        assert_eq!(
            manifest,
            json!({"name":name,"description":"Native adapter contract fixture.","version":plan["version"]})
        );
        let marketplace: Value = serde_json::from_slice(
            &std::fs::read(catalog.join(".plugin/marketplace.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            marketplace,
            json!({"name":name,"owner":{"name":"upskill"},"plugins":[{"name":name,"source":format!("./revisions/{payload}")}]})
        );
        assert_eq!(std::fs::read(revision.join("skills/contract-probe/SKILL.md")).unwrap(),b"---\nname: contract-probe\ndescription: Native adapter contract fixture.\n---\n\nRead the local resource when explicitly requested.\n");
        assert_eq!(
            std::fs::read(revision.join("skills/contract-probe/resources/probe.txt")).unwrap(),
            b"Native contract resource. No model request is part of this fixture.\n"
        );
        let mut hash = Sha256::new();
        hash.update(b"upskill:skill-package-files:v1\0");
        hash.update(3_u64.to_be_bytes());
        for (path, mode) in [
            (".plugin/plugin.json", None),
            ("skills/contract-probe/SKILL.md", None),
            (
                "skills/contract-probe/resources/probe.txt",
                if cfg!(unix) { Some(0o600_u32) } else { None },
            ),
        ] {
            let bytes = std::fs::read(revision.join(path)).unwrap();
            for value in [path.as_bytes(), bytes.as_slice()] {
                hash.update((value.len() as u64).to_be_bytes());
                hash.update(value);
            }
            match mode {
                None => hash.update([0]),
                Some(mode) => {
                    hash.update([1]);
                    hash.update(mode.to_be_bytes());
                }
            }
        }
        let package_hash = hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(plan["package_sha256"], package_hash);
        let selector = format!("{name}@{name}");
        assert_eq!(
            plan["commands"],
            json!([
                ["--no-auto-update", "--version"],
                ["--no-auto-update", "plugin", "list", "--json"],
                [
                    "--no-auto-update",
                    "plugin",
                    "marketplace",
                    "list",
                    "--json"
                ],
                ["--no-auto-update", "plugin", "marketplace", "add", catalog],
                ["--no-auto-update", "plugin", "install", selector],
                ["--no-auto-update", "plugin", "list", "--json"],
                [
                    "--no-auto-update",
                    "plugin",
                    "marketplace",
                    "list",
                    "--json"
                ],
                ["--no-auto-update", "plugin", "uninstall", selector],
                ["--no-auto-update", "plugin", "marketplace", "remove", name],
                ["--no-auto-update", "plugin", "list", "--json"],
                [
                    "--no-auto-update",
                    "plugin",
                    "marketplace",
                    "list",
                    "--json"
                ]
            ])
        );
        assert_eq!(
            std::fs::read(home.join("manual-preservation.txt")).unwrap(),
            b"Manual fixture sentinel.\n"
        );
        assert!(
            !home.join(".copilot").exists(),
            "preparation must not bootstrap native state"
        );
        assert!(!home.join(".claude").exists());
        assert!(!home.join(".upskill/exports/claude").exists());
        let before: Vec<_> = files(&root)
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect();
        assert_eq!(before.len(), 9);
        assert!(prepare_copilot(&root).is_err());
        let after: Vec<_> = files(&root)
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect();
        assert_eq!(after, before);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fn check(path: &Path) {
                let metadata = std::fs::metadata(path).unwrap();
                assert_eq!(
                    metadata.permissions().mode() & 0o777,
                    if metadata.is_dir() { 0o700 } else { 0o600 }
                );
                if metadata.is_dir() {
                    for entry in std::fs::read_dir(path).unwrap() {
                        check(&entry.unwrap().path());
                    }
                }
            }
            check(&root);
        }
    }
}
