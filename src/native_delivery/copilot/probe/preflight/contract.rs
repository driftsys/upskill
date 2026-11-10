//! The one reviewed Copilot fixture contract; no caller-selected read paths.
use super::{CopilotPreflightError as Error, CopilotProbeRequest};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

const ITEM: &str = ".upskill/exports/copilot/native-contract-probe";

#[derive(Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct FilePin {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub mode: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CodePin {
    pub selected: PathBuf,
    pub resolved: PathBuf,
    pub sha256: String,
    pub bytes: u64,
    pub mode: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Preparation {
    status: String,
    os: String,
    machine: String,
    launcher: CodePin,
    node: CodePin,
    platform_entrypoint: CodePin,
    wrapper_package_version: String,
    required_cli_version: String,
    version_mismatch_policy: String,
    plan_sha256: String,
    fixture_files: Vec<FilePin>,
}
pub(super) struct Contract {
    pub files: BTreeMap<String, FilePin>,
    pub code: Vec<CodePin>,
}
pub(super) fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
pub(super) fn absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.to_str().is_some()
        && !path.components().any(|part| part == Component::ParentDir)
        && path.components().collect::<PathBuf>().as_os_str() == path.as_os_str()
}
fn parse(bytes: &[u8]) -> Result<Value, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidInput)?;
    crate::parse::json::parse_unique(text).map_err(|_| Error::InvalidInput)
}
fn expected(
    root: &Path,
    plan_hash: &str,
    plan_bytes: u64,
) -> Result<(Value, BTreeMap<String, FilePin>), Error> {
    use crate::compile::skill_package::{SkillPackageRequest, compile_skill_package};
    use crate::compile::skill_selection::{SkillSelectionRequest, select_skill};
    use crate::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
    use crate::pipeline::ItemKind;
    use crate::source_manifest::{CapturedSourceFile, assemble_manifest, content_hash};
    use crate::target::{
        CapabilityCategory, CapabilityDeclaration, CapabilityKey, Representation, Requirement,
        TargetSurface,
    };
    // Rebuild from fixed non-secret authored bytes, never from manifest paths.
    // The existing compiler binds names and payloads to the selected scope root.
    let source_bytes = b"---\nschema: 1\ndescription: Native adapter contract fixture.\n---\nRead the local resource when explicitly requested.\n";
    let resource_bytes = b"Native contract resource. No model request is part of this fixture.\n";
    let source = assemble_manifest(
        [
            ("SKILL.md", source_bytes.as_slice()),
            ("resources/probe.txt", resource_bytes.as_slice()),
        ]
        .into_iter()
        .map(|(path, bytes)| CapturedSourceFile {
            path: path.into(),
            bytes: bytes.into(),
            content_sha256: content_hash(bytes),
            mode: Some(0o600),
        })
        .collect(),
    )
    .map_err(|_| Error::InvalidInput)?;
    let surface = TargetSurface::CopilotCli;
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
    })
    .map_err(|_| Error::InvalidInput)?
    .ok_or(Error::InvalidInput)?;
    let owner = ArtifactOwner {
        installation_id: "native-contract-probe".into(),
        source: "./contract-fixture".into(),
        source_revision: None,
        kind: ItemKind::Skill,
        source_name: selected.source_name.clone(),
        installed_name: selected.installed_name.clone(),
    };
    let scope = InstallationRoot {
        scope: OwnershipScope::User,
        path: root.join("home"),
    };
    let package = compile_skill_package(&SkillPackageRequest {
        selected: &selected,
        surface,
        scope: &scope,
        owner: &owner,
    })
    .map_err(|_| Error::InvalidInput)?;
    let mut files = BTreeMap::new();
    let mut add = |path: String, bytes: &[u8]| {
        files.insert(
            path.clone(),
            FilePin {
                path,
                sha256: content_hash(bytes),
                bytes: bytes.len() as u64,
                mode: "0o600".into(),
            },
        );
    };
    for file in package
        .revision_files
        .iter()
        .chain(package.catalog_file.iter())
    {
        add(format!("home/{}", file.path), &file.bytes);
    }
    add("source/SKILL.md".into(), source_bytes);
    add("source/resources/probe.txt".into(), resource_bytes);
    add(
        "home/manual-preservation.txt".into(),
        b"Manual fixture sentinel.\n",
    );
    add("project/.claude/settings.json".into(),b"{\n  \"companyAnnouncements\": [\n    \"contract-preservation-sentinel\"\n  ],\n  \"enabledPlugins\": {},\n  \"extraKnownMarketplaces\": {}\n}\n");
    files.insert(
        "plan.json".into(),
        FilePin {
            path: "plan.json".into(),
            sha256: plan_hash.into(),
            bytes: plan_bytes,
            mode: "0o600".into(),
        },
    );
    let home = root.join("home");
    let catalog = home.join(ITEM);
    let name = &package.native_name;
    let selector = format!("{name}@{name}");
    let plugins = json!(["--no-auto-update", "plugin", "list", "--json"]);
    let marketplaces = json!([
        "--no-auto-update",
        "plugin",
        "marketplace",
        "list",
        "--json"
    ]);
    let plan = json!({
        "status":"prepared-only; native execution requires approval", "client_version":"1.0.107",
        "scope":"user", "scope_root":home,"model_calls":0,"output_limit_bytes":1048576,
        "maximum_authorized_spending":0,"maximum_native_commands":11,"timeout_seconds_per_command":20,
        "packet_root":root,"cwd":root.join("project"),"home":home,"catalog":catalog,
        "native_name":name,"version":package.version,
        "revision_root":package.revision_root,"payload_sha256":package.payload_sha256,
        "package_sha256":package.package_sha256,
        "commands":[["--no-auto-update","--version"],plugins,marketplaces,
            ["--no-auto-update","plugin","marketplace","add",catalog],
            ["--no-auto-update","plugin","install",selector],plugins,marketplaces,
            ["--no-auto-update","plugin","uninstall",selector],
            ["--no-auto-update","plugin","marketplace","remove",name],plugins,marketplaces]
    });
    Ok((plan, files))
}
impl Contract {
    pub fn parse(
        request: &CopilotProbeRequest,
        plan: &[u8],
        preparation: &[u8],
    ) -> Result<Self, Error> {
        let (expected_plan, files) = expected(
            &request.packet_root,
            &request.plan_sha256,
            plan.len() as u64,
        )?;
        if parse(plan)? != expected_plan {
            return Err(Error::InvalidInput);
        }
        let receipt: Preparation =
            serde_json::from_value(parse(preparation)?).map_err(|_| Error::InvalidInput)?;
        let os = if cfg!(target_os = "macos") {
            "Darwin"
        } else {
            "Linux"
        };
        let machine = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            "arm64"
        } else {
            std::env::consts::ARCH
        };
        if receipt.status != "prepared only; no native command executed"
            || receipt.os != os
            || receipt.machine != machine
            || receipt.wrapper_package_version != "1.0.107"
            || receipt.required_cli_version != "1.0.107"
            || receipt.plan_sha256 != request.plan_sha256
            || receipt.version_mismatch_policy
                != "stop before inventories or mutations; no update or fallback"
        {
            return Err(Error::InvalidInput);
        }
        if receipt.fixture_files.len() != files.len() {
            return Err(Error::InvalidInput);
        }
        let mut seen = std::collections::BTreeSet::new();
        for file in receipt.fixture_files {
            if !seen.insert(file.path.clone()) || files.get(&file.path) != Some(&file) {
                return Err(Error::InvalidInput);
            }
        }
        let code = vec![receipt.launcher, receipt.node, receipt.platform_entrypoint];
        let names = [
            ("copilot", "npm-loader.js"),
            ("node", "node"),
            ("copilot", "copilot"),
        ];
        for (pin, (selected, resolved)) in code.iter().zip(names) {
            if !absolute(&pin.selected)
                || !absolute(&pin.resolved)
                || !hash(&pin.sha256)
                || pin.bytes == 0
                || pin.bytes > 1024 * 1024 * 1024
                || pin.selected.file_name().and_then(|name| name.to_str()) != Some(selected)
                || pin.resolved.file_name().and_then(|name| name.to_str()) != Some(resolved)
            {
                return Err(Error::InvalidInput);
            }
        }
        Ok(Self { files, code })
    }
}
