//! Acceptance tests for inactive, immutable per-client skill package plans.

use sha2::{Digest, Sha256};
use std::path::PathBuf;
use upskill::compile::skill_package::{
    SkillPackageBinding, SkillPackageRequest, compile_skill_package,
};
use upskill::compile::skill_selection::{SelectedSkill, SkillSelectionRequest, select_skill};
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::source_manifest::{CapturedSourceFile, SourceManifest};
use upskill::target::{
    CapabilityCategory, CapabilityDeclaration, CapabilityKey, Representation, Requirement,
    TargetSurface,
};

const CLI: [TargetSurface; 4] = [
    TargetSurface::ClaudeCli,
    TargetSurface::CopilotCli,
    TargetSurface::CodexCli,
    TargetSurface::OpenCodeCli,
];
const ENTRY: &[u8] = b"---\nschema: 1\ndescription: Demo.\n---\n\nBody\n";

fn frame(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}
fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
// Independent test encoding of captured evidence; no filesystem or production
// digest helper participates in the fixture's expected values.
fn manifest(files: &[(&str, &[u8], Option<u32>)]) -> SourceManifest {
    let mut files: Vec<_> = files
        .iter()
        .map(|(path, bytes, mode)| CapturedSourceFile {
            path: (*path).into(),
            bytes: bytes.to_vec(),
            mode: *mode,
            content_sha256: hex(Sha256::digest(bytes)),
        })
        .collect();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let mut digest = Sha256::new();
    digest.update(b"upskill:source-manifest:v1\0");
    digest.update((files.len() as u64).to_be_bytes());
    for file in &files {
        frame(&mut digest, file.path.as_bytes());
        frame(&mut digest, &file.bytes);
        match file.mode {
            None => digest.update([0]),
            Some(mode) => {
                digest.update([1]);
                digest.update(mode.to_be_bytes());
            }
        }
    }
    SourceManifest {
        files,
        content_sha256: hex(digest.finalize()),
    }
}
fn selected(
    surface: TargetSurface,
    files: &[(&str, &[u8], Option<u32>)],
    alias: Option<&str>,
) -> SelectedSkill {
    let source = manifest(files);
    let caps = ["name", "description", "license", "metadata"].map(|name| CapabilityDeclaration {
        surface,
        key: CapabilityKey::new(CapabilityCategory::Skills, name),
        representation: Representation::Native,
        requirement: Requirement::RequiredBehavior,
        reason: "offline fixture projection",
    });
    select_skill(&SkillSelectionRequest {
        source: &source,
        folder_name: "demo",
        alias,
        surface,
        capabilities: &caps,
    })
    .unwrap()
    .unwrap()
}
fn scope() -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: PathBuf::from("/workspace"),
    }
}
fn owner(skill: &SelectedSkill) -> ArtifactOwner {
    ArtifactOwner {
        installation_id: "install-demo".into(),
        source: "local:fixture".into(),
        source_revision: Some("r1".into()),
        kind: ItemKind::Skill,
        source_name: skill.source_name.clone(),
        installed_name: skill.installed_name.clone(),
    }
}
fn request<'a>(
    skill: &'a SelectedSkill,
    root: &'a InstallationRoot,
    owner: &'a ArtifactOwner,
) -> SkillPackageRequest<'a> {
    SkillPackageRequest {
        selected: skill,
        surface: skill.surface,
        scope: root,
        owner,
    }
}
fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap()
}

#[test]
fn four_profiles_plan_exact_envelopes_and_contained_single_revision_catalogs() {
    for (surface, client, envelope, catalog) in [
        (
            TargetSurface::ClaudeCli,
            "claude",
            Some(".claude-plugin/plugin.json"),
            Some(".claude-plugin/marketplace.json"),
        ),
        (
            TargetSurface::CopilotCli,
            "copilot",
            Some(".plugin/plugin.json"),
            Some(".plugin/marketplace.json"),
        ),
        (
            TargetSurface::CodexCli,
            "codex",
            Some(".codex-plugin/plugin.json"),
            Some(".agents/plugins/marketplace.json"),
        ),
        (TargetSurface::OpenCodeCli, "opencode", None, None),
    ] {
        let skill = selected(
            surface,
            &[
                ("SKILL.md", ENTRY, None),
                ("assets/data.bin", &[0, 255], Some(0o751)),
            ],
            None,
        );
        let root = scope();
        let owned = owner(&skill);
        let result = compile_skill_package(&request(&skill, &root, &owned)).unwrap();
        let item = format!(".upskill/exports/{client}/install-demo");
        assert_eq!(result.item_root, item);
        assert_eq!(
            result.revision_root,
            format!("{item}/revisions/{}", result.payload_sha256)
        );
        assert_eq!(result.native_name.len(), 72);
        assert!(result.native_name.starts_with("upskill-"));
        assert_eq!(result.version, format!("0.0.0-r{}", result.payload_sha256));
        let prefix = result.revision_root.clone() + "/";
        let paths: Vec<_> = result
            .revision_files
            .iter()
            .map(|file| file.path.strip_prefix(&prefix).unwrap())
            .collect();
        let mut expected = vec!["skills/demo/SKILL.md", "skills/demo/assets/data.bin"];
        if let Some(path) = envelope {
            expected.push(path);
        }
        expected.sort();
        assert_eq!(paths, expected);
        let resource = result
            .revision_files
            .iter()
            .find(|file| file.path.ends_with("/assets/data.bin"))
            .unwrap();
        assert_eq!(resource.bytes, [0, 255]);
        assert_eq!(resource.mode, Some(0o751));
        let entry = result
            .revision_files
            .iter()
            .find(|file| file.path.ends_with("/SKILL.md"))
            .unwrap();
        assert_eq!(
            entry.bytes,
            b"---\nname: demo\ndescription: Demo.\n---\n\nBody\n"
        );
        assert_eq!(entry.mode, None);
        if let Some(path) = envelope {
            let file = result
                .revision_files
                .iter()
                .find(|file| file.path == format!("{prefix}{path}"))
                .unwrap();
            let value = json(&file.bytes);
            assert_eq!(value["name"], result.native_name);
            assert_eq!(value["version"], result.version);
            assert_eq!(value["description"], "Demo.");
            if surface == TargetSurface::CodexCli {
                assert_eq!(value["skills"], "./skills/");
            } else {
                assert!(
                    value.get("skills").is_none(),
                    "native defaults must not be duplicated"
                );
            }
            assert_eq!(file.mode, None);
        }
        match catalog {
            Some(path) => {
                let file = result.catalog_file.as_ref().unwrap();
                assert_eq!(file.path, format!("{item}/{path}"));
                assert_eq!(file.mode, None);
                let value = json(&file.bytes);
                assert_eq!(value["name"], result.native_name);
                assert_eq!(value["plugins"].as_array().unwrap().len(), 1);
                assert_eq!(value["plugins"][0]["name"], result.native_name);
                let expected = format!("./revisions/{}", result.payload_sha256);
                if surface == TargetSurface::CodexCli {
                    assert_eq!(
                        value["plugins"][0]["source"],
                        serde_json::json!({"source": "local", "path": expected})
                    );
                } else {
                    assert_eq!(value["plugins"][0]["source"], expected);
                }
                assert_eq!(
                    result.binding,
                    SkillPackageBinding::LocalMarketplace {
                        directory: root.path.join(&item)
                    }
                );
            }
            None => {
                assert!(result.catalog_file.is_none());
                assert_eq!(
                    result.binding,
                    SkillPackageBinding::ExplicitSkillSource {
                        directory: root.path.join(&result.revision_root).join("skills")
                    }
                );
            }
        }
    }
}

#[test]
fn stable_native_identity_survives_revision_alias_and_source_updates() {
    for surface in CLI {
        let skill = selected(surface, &[("SKILL.md", ENTRY, None)], None);
        let root = scope();
        let owned = owner(&skill);
        let original = compile_skill_package(&request(&skill, &root, &owned)).unwrap();
        let aliased = selected(surface, &[("SKILL.md", ENTRY, None)], Some("alias-demo"));
        let mut newer = owner(&aliased);
        newer.source_revision = Some("r2".into());
        newer.source = "other:source".into();
        let update = compile_skill_package(&request(&aliased, &root, &newer)).unwrap();
        assert_eq!(update.native_name, original.native_name);
        assert_eq!(update.item_root, original.item_root);
        assert_ne!(update.payload_sha256, original.payload_sha256);
        assert_ne!(update.package_sha256, original.package_sha256);
        assert!(
            update
                .revision_files
                .iter()
                .any(|file| file.path.ends_with("/skills/alias-demo/SKILL.md"))
        );
        assert!(
            update
                .revision_files
                .iter()
                .all(|file| !file.path.contains("/skills/demo/"))
        );
    }
}

#[test]
fn scope_surface_and_installation_identity_do_not_share_native_names() {
    let root = scope();
    let mut names = std::collections::BTreeSet::new();
    for surface in CLI {
        let skill = selected(surface, &[("SKILL.md", ENTRY, None)], None);
        let owned = owner(&skill);
        for changed in [
            InstallationRoot {
                scope: root.scope,
                path: root.path.clone(),
            },
            InstallationRoot {
                scope: OwnershipScope::User,
                path: root.path.clone(),
            },
            InstallationRoot {
                scope: root.scope,
                path: PathBuf::from("/another-checkout"),
            },
        ] {
            let result = compile_skill_package(&request(&skill, &changed, &owned)).unwrap();
            assert!(names.insert(result.native_name));
        }
        let mut changed = owned.clone();
        changed.installation_id = "other-installation".into();
        assert!(
            names.insert(
                compile_skill_package(&request(&skill, &root, &changed))
                    .unwrap()
                    .native_name
            )
        );
    }
}

#[test]
fn source_revision_bytes_and_modes_each_change_revision_identity_without_changing_native_name() {
    let root = scope();
    for surface in CLI {
        let base = selected(
            surface,
            &[("SKILL.md", ENTRY, None), ("run", b"one", Some(0o644))],
            None,
        );
        let owned = owner(&base);
        let original = compile_skill_package(&request(&base, &root, &owned)).unwrap();
        let mut revision = owned.clone();
        revision.source_revision = Some("r2".into());
        let versioned = compile_skill_package(&request(&base, &root, &revision)).unwrap();
        assert_ne!(original.payload_sha256, versioned.payload_sha256);
        let mut different_source = owned.clone();
        different_source.source = "other:source".into();
        let relocated_source =
            compile_skill_package(&request(&base, &root, &different_source)).unwrap();
        assert_ne!(original.payload_sha256, relocated_source.payload_sha256);
        assert_eq!(original.native_name, relocated_source.native_name);
        for files in [
            vec![
                ("SKILL.md", ENTRY, None),
                ("run", b"two".as_slice(), Some(0o644)),
            ],
            vec![
                ("SKILL.md", ENTRY, None),
                ("run", b"one".as_slice(), Some(0o755)),
            ],
        ] {
            let changed = selected(surface, &files, None);
            let result = compile_skill_package(&request(&changed, &root, &owned)).unwrap();
            assert_ne!(original.payload_sha256, result.payload_sha256);
            assert_ne!(original.package_sha256, result.package_sha256);
            assert_eq!(original.native_name, result.native_name);
        }
    }
}

#[test]
fn repeatable_output_preserves_inputs_and_retains_no_previous_revision_catalog_entry() {
    let skill = selected(TargetSurface::ClaudeCli, &[("SKILL.md", ENTRY, None)], None);
    let before = skill.clone();
    let root = scope();
    let owned = owner(&skill);
    let req = request(&skill, &root, &owned);
    let first = compile_skill_package(&req).unwrap();
    assert_eq!(first, compile_skill_package(&req).unwrap());
    assert_eq!(skill, before);
    let mut newer = owned.clone();
    newer.source_revision = None;
    let next = compile_skill_package(&request(&skill, &root, &newer)).unwrap();
    let catalog = std::str::from_utf8(&next.catalog_file.unwrap().bytes)
        .unwrap()
        .to_owned();
    assert!(!catalog.contains(&first.payload_sha256));
    assert!(catalog.contains(&next.payload_sha256));
}

#[test]
fn invalid_scope_owner_or_selection_binding_rejects_without_output() {
    let skill = selected(TargetSurface::ClaudeCli, &[("SKILL.md", ENTRY, None)], None);
    let root = scope();
    let owned = owner(&skill);
    for path in ["relative", "/workspace/../escape", "/workspace/./alias"] {
        let changed = InstallationRoot {
            scope: root.scope,
            path: path.into(),
        };
        assert!(
            compile_skill_package(&request(&skill, &changed, &owned)).is_err(),
            "{path}"
        );
    }
    for id in ["", "../escape", "x/y", "x.y"] {
        let mut changed = owned.clone();
        changed.installation_id = id.into();
        assert!(
            compile_skill_package(&request(&skill, &root, &changed)).is_err(),
            "{id}"
        );
    }
    for field in [
        "source",
        "revision",
        "kind",
        "source-name",
        "installed-name",
    ] {
        let mut changed = owned.clone();
        match field {
            "source" => changed.source.clear(),
            "revision" => changed.source_revision = Some(String::new()),
            "kind" => changed.kind = ItemKind::Rule,
            "source-name" => changed.source_name = "different".into(),
            _ => changed.installed_name = "different".into(),
        }
        assert!(
            compile_skill_package(&request(&skill, &root, &changed)).is_err(),
            "{field}"
        );
    }
    let mut req = request(&skill, &root, &owned);
    req.surface = TargetSurface::CodexCli;
    assert!(compile_skill_package(&req).is_err());
    req.surface = TargetSurface::CodexIde;
    assert!(compile_skill_package(&req).is_err());
}

#[test]
fn tampered_entrypoint_or_resource_evidence_rejects() {
    let skill = selected(
        TargetSurface::ClaudeCli,
        &[("SKILL.md", ENTRY, None), ("data", b"data", Some(0o644))],
        None,
    );
    let root = scope();
    let owned = owner(&skill);
    for field in [
        "path",
        "bytes",
        "mode",
        "digest",
        "resource-path",
        "resource-bytes",
        "resource-mode",
        "metadata-name",
    ] {
        let mut changed = skill.clone();
        match field {
            "path" => changed.entrypoint.path = "elsewhere/SKILL.md".into(),
            "bytes" => changed.entrypoint.bytes.push(0),
            "mode" => changed.entrypoint.mode = Some(0o755),
            "digest" => changed.source_manifest_sha256 = "invalid".into(),
            "resource-path" => changed.resources[0].path = "../escape".into(),
            "resource-bytes" => changed.resources[0].bytes.push(0),
            "resource-mode" => changed.resources[0].mode = Some(0o10000),
            _ => changed.metadata.name = "different".into(),
        }
        assert!(
            compile_skill_package(&request(&changed, &root, &owned)).is_err(),
            "{field}"
        );
    }
}

#[test]
fn resource_collisions_and_source_only_files_reject_even_with_correct_bytes_digest() {
    let skill = selected(
        TargetSurface::ClaudeCli,
        &[("SKILL.md", ENTRY, None), ("data", b"data", None)],
        None,
    );
    let root = scope();
    let owned = owner(&skill);
    for path in ["SKILL.md", "SKILL.claude.md", "SKILL.codex.md", "RULE.md"] {
        let mut changed = skill.clone();
        changed.resources[0].path = path.into();
        assert!(
            compile_skill_package(&request(&changed, &root, &owned)).is_err(),
            "{path}"
        );
    }
    let mut changed = skill.clone();
    changed.resources.push(changed.resources[0].clone());
    assert!(compile_skill_package(&request(&changed, &root, &owned)).is_err());
    let mut changed = skill.clone();
    let mut nested = changed.resources[0].clone();
    nested.path = "data/nested".into();
    changed.resources.push(nested);
    assert!(compile_skill_package(&request(&changed, &root, &owned)).is_err());
}

#[test]
fn framed_protocol_matches_independently_encoded_golden() {
    let skill = selected(TargetSurface::ClaudeCli, &[("SKILL.md", ENTRY, None)], None);
    let root = scope();
    let owned = owner(&skill);
    let result = compile_skill_package(&request(&skill, &root, &owned)).unwrap();
    assert_eq!(
        result.native_name,
        "upskill-ca3f92288d5cf8ff96fb316d5b7c07953a11b35a401ae2f852a721763cac5af5"
    );
    assert_eq!(
        result.payload_sha256,
        "54ad588c40a1eb9035980be00719a214ee83d307a7df78888efd8ec688e48186"
    );
    assert_eq!(
        result.package_sha256,
        "e46f4c71974f8618aca1cb58d3d5066520d37048cc7982a7c260c9f953456ce5"
    );
}

#[test]
fn equal_body_bytes_with_different_selected_origin_get_distinct_payload_identity() {
    use upskill::compile::skill_selection::SkillBodyOrigin;
    let skill = selected(TargetSurface::ClaudeCli, &[("SKILL.md", ENTRY, None)], None);
    let root = scope();
    let owned = owner(&skill);
    let canonical = compile_skill_package(&request(&skill, &root, &owned)).unwrap();
    let mut changed = skill.clone();
    changed.body_origin = SkillBodyOrigin::Override {
        path: "SKILL.claude.md".into(),
    };
    let overridden = compile_skill_package(&request(&changed, &root, &owned)).unwrap();
    assert_ne!(canonical.payload_sha256, overridden.payload_sha256);
    assert_eq!(canonical.native_name, overridden.native_name);
    changed.body_origin = SkillBodyOrigin::Override {
        path: "SKILL.copilot.md".into(),
    };
    assert!(compile_skill_package(&request(&changed, &root, &owned)).is_err());
}
