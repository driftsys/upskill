//! Prepare an isolated, non-secret Claude contract fixture without native calls.
use anyhow::{Context, Result, ensure};
use std::path::{Component, Path, PathBuf};
use upskill::compile::skill_package::{SkillPackageRequest, compile_skill_package};
use upskill::compile::skill_selection::{SkillSelectionRequest, select_skill};
use upskill::lifecycle::native_receipt::{NativeSkillSelection, SkillTarget};
use upskill::native_delivery::adapter::{
    NativeActionKind, NativeDeliveryProfile, NativeExposureOrder,
};
use upskill::native_delivery::claude::{ClaudePluginAction, plan_plugin_command};
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::source_manifest::capture_source;
use upskill::target::{
    CapabilityCategory, CapabilityDeclaration, CapabilityKey, Representation, Requirement,
    TargetSurface,
};

fn directory(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
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
fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let root = PathBuf::from(
        arguments
            .next()
            .context("usage: native_adapter_fixture ABSENT_ABSOLUTE_ROOT")?,
    );
    ensure!(arguments.next().is_none(), "unexpected fixture arguments");
    ensure!(root.is_absolute(), "fixture root must be absolute");
    ensure!(
        !root.components().any(|part| part == Component::ParentDir),
        "fixture root must not contain parent components"
    );
    ensure!(
        std::fs::symlink_metadata(&root)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "fixture root must be absent"
    );
    std::fs::create_dir(&root)?;
    directory(&root)?;
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
    let surface = TargetSurface::ClaudeCli;
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
    .context("fixture was excluded from Claude")?;
    let scope = InstallationRoot {
        scope: OwnershipScope::Project,
        path: root.join("project"),
    };
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
    write(&scope.path.join(".claude/settings.json"), b"{\n  \"companyAnnouncements\": [\n    \"contract-preservation-sentinel\"\n  ],\n  \"enabledPlugins\": {},\n  \"extraKnownMarketplaces\": {}\n}\n")?;
    let catalog = scope.path.join(&package.item_root);
    let profile = NativeDeliveryProfile {
        id: "unqualified-local-catalog-probe".into(),
        revision: 1,
        surface,
        scope: scope.clone(),
        client_version: "2.1.308".into(),
        platform: "unqualified-fixture".into(),
        exposure: NativeExposureOrder::AfterNative,
        action: NativeActionKind::Command,
    };
    let target = SkillTarget {
        installation_id: owner.installation_id.clone(),
        surface,
    };
    let selection = NativeSkillSelection::from_package(&package);
    let command = |action| -> Result<serde_json::Value> {
        let planned = plan_plugin_command(&profile, &target, &selection, action)?;
        let arguments = planned
            .arguments()
            .iter()
            .map(|value| value.to_str().context("non-UTF-8 fixture argument"))
            .collect::<Result<Vec<_>>>()?;
        Ok(serde_json::to_value(arguments)?)
    };
    let commands = serde_json::json!([
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
    ]);
    let plan = serde_json::json!({"status":"prepared-only; native execution requires approval", "client_version":"2.1.308",
        "model_calls":0,"maximum_native_commands":12,"timeout_seconds_per_command":20,
        "packet_root":root,"cwd":scope.path,"home":root.join("home"),"catalog":catalog,
        "native_name":package.native_name,"version":package.version,"revision_root":package.revision_root,
        "payload_sha256":package.payload_sha256,"package_sha256":package.package_sha256,"commands":commands});
    write(&root.join("plan.json"), &serde_json::to_vec_pretty(&plan)?)?;
    println!("{}", root.join("plan.json").display());
    Ok(())
}
