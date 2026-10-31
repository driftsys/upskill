//! Acceptance tests for pure, opt-in skill representation selection.

use std::fs;
use upskill::compile::skill_selection::{SkillBodyOrigin, SkillSelectionRequest, select_skill};
use upskill::parse::frontmatter;
use upskill::source_manifest::{SourceManifest, capture_source};
use upskill::target::{
    CapabilityCategory, CapabilityDeclaration, CapabilityError, CapabilityKey, Representation,
    Requirement, TargetSurface,
};

const CLI: [TargetSurface; 4] = [
    TargetSurface::ClaudeCli,
    TargetSurface::CopilotCli,
    TargetSurface::CodexCli,
    TargetSurface::OpenCodeCli,
];
const ENTRY: &[u8] = b"---\nschema: 1\nname: demo\ndescription: A demo skill.\n---\n\nRead [data](assets/data.bin).\n";

fn fixture(files: &[(&str, &[u8])]) -> SourceManifest {
    let dir = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let destination = dir.path().join(path);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(destination, bytes).unwrap();
    }
    capture_source(dir.path()).unwrap()
}

// These declarations authorize fixture projection only, not native activation.
fn capabilities(surface: TargetSurface) -> Vec<CapabilityDeclaration> {
    ["name", "description", "license", "metadata"]
        .map(|name| CapabilityDeclaration {
            surface,
            key: CapabilityKey::new(CapabilityCategory::Skills, name),
            representation: Representation::Native,
            requirement: Requirement::RequiredBehavior,
            reason: "explicit fixture projection contract",
        })
        .to_vec()
}

fn request<'a>(
    source: &'a SourceManifest,
    caps: &'a [CapabilityDeclaration],
    surface: TargetSurface,
) -> SkillSelectionRequest<'a> {
    SkillSelectionRequest {
        source,
        folder_name: "demo",
        alias: None,
        surface,
        capabilities: caps,
    }
}

#[test]
fn four_cli_selections_bind_default_or_alias_identity_and_captured_resources() {
    let source = fixture(&[("SKILL.md", ENTRY), ("assets/data.bin", &[0, 255, 1])]);
    let before = source.clone();
    for surface in CLI {
        let caps = capabilities(surface);
        for (alias, installed_name, entrypoint) in [
            (None, "demo", "skills/demo/SKILL.md"),
            (
                Some("installed-demo"),
                "installed-demo",
                "skills/installed-demo/SKILL.md",
            ),
        ] {
            let mut req = request(&source, &caps, surface);
            req.alias = alias;
            let selected = select_skill(&req).unwrap().unwrap();
            assert_eq!(selected.surface, surface);
            assert_eq!(selected.source_name, "demo");
            assert_eq!(selected.installed_name, installed_name);
            assert_eq!(selected.metadata.name, installed_name);
            assert_eq!(selected.metadata.description, "A demo skill.");
            assert_eq!(selected.body_origin, SkillBodyOrigin::Canonical);
            assert_eq!(selected.body, "\nRead [data](assets/data.bin).\n");
            assert_eq!(selected.entrypoint.path, entrypoint);
            assert_eq!(selected.entrypoint.mode, None);
            let rendered = std::str::from_utf8(&selected.entrypoint.bytes).unwrap();
            let (yaml, body) = frontmatter::parse::<serde_yaml_ng::Value>(rendered).unwrap();
            assert_eq!(yaml["name"].as_str(), Some(installed_name));
            assert_eq!(yaml["description"].as_str(), Some("A demo skill."));
            assert!(yaml.get("schema").is_none());
            assert_eq!(body, "\nRead [data](assets/data.bin).\n");
            assert_eq!(selected.resources.len(), 1);
            assert_eq!(selected.resources[0].path, "assets/data.bin");
            assert_eq!(selected.resources[0].bytes, [0, 255, 1]);
            assert_eq!(selected.resources[0].mode, source.files[1].mode);
            assert_eq!(selected.source_manifest_sha256, source.content_sha256);
        }
    }
    assert_eq!(source, before);
}

#[test]
fn canonical_directives_select_each_family_without_foreign_bodies() {
    let entry = b"---\nschema: 1\ndescription: demo\n---\nCommon\n<!-- @client:claude -->\nClaude\n<!-- @endclient -->\n<!-- @client:copilot -->\nCopilot\n<!-- @endclient -->\n<!-- @client:codex -->\nCodex\n<!-- @endclient -->\n<!-- @client:opencode -->\nOpenCode\n<!-- @endclient -->\n";
    let source = fixture(&[("SKILL.md", entry)]);
    for (surface, expected) in CLI.into_iter().zip([
        "Common\nClaude\n",
        "Common\nCopilot\n",
        "Common\nCodex\n",
        "Common\nOpenCode\n",
    ]) {
        let caps = capabilities(surface);
        let selected = select_skill(&request(&source, &caps, surface))
            .unwrap()
            .unwrap();
        assert_eq!(selected.body, expected);
        assert_eq!(selected.body_origin, SkillBodyOrigin::Canonical);
    }
}

#[test]
fn overrides_replace_body_verbatim_even_when_ignored_and_canonical_directives_are_invalid() {
    let entry = b"---\nschema: 1\ndescription: demo\nignore: ['*.md']\n---\n<!-- @client:invalid -->\nCanonical must not be processed.\n";
    let override_body = b"  literal\r\n<!-- @client:codex -->\r\nkeep this verbatim\r\n";
    for (surface, path) in [
        (TargetSurface::ClaudeCli, "SKILL.claude.md"),
        (TargetSurface::CopilotCli, "SKILL.copilot.md"),
        (TargetSurface::OpenCodeCli, "SKILL.opencode.md"),
    ] {
        let source = fixture(&[("SKILL.md", entry), (path, override_body)]);
        let caps = capabilities(surface);
        let selected = select_skill(&request(&source, &caps, surface))
            .unwrap()
            .unwrap();
        assert_eq!(selected.body.as_bytes(), override_body);
        assert_eq!(
            selected.body_origin,
            SkillBodyOrigin::Override { path: path.into() }
        );
        let (_, rendered_body) =
            frontmatter::split(std::str::from_utf8(&selected.entrypoint.bytes).unwrap()).unwrap();
        assert_eq!(
            rendered_body.strip_prefix('\n').unwrap().as_bytes(),
            override_body
        );
        assert!(selected.resources.is_empty());
    }
}

#[test]
fn audience_exclusion_precedes_overrides_directives_and_projection_checks() {
    let entry = b"---\nschema: 1\ndescription: demo\naudience: [codex]\nclaude: {allowed-tools: Bash}\n---\n<!-- @client:invalid -->\n";
    let source = fixture(&[
        ("SKILL.md", entry),
        ("SKILL.claude.md", &[255]),
        ("SKILL.codex.md", b"unsupported"),
    ]);
    let req = request(&source, &[], TargetSurface::ClaudeCli);
    assert!(select_skill(&req).unwrap().is_none());
    let caps = capabilities(TargetSurface::CodexCli);
    let error = select_skill(&request(&source, &caps, TargetSurface::CodexCli)).unwrap_err();
    assert!(format!("{error:#}").contains("SKILL.codex.md"));
}

#[test]
fn excluded_codex_audience_emits_nothing_for_historical_clients() {
    let source = fixture(&[(
        "SKILL.md",
        b"---\nschema: 1\ndescription: demo\naudience: [codex]\n---\nCodex\n",
    )]);
    for surface in CLI {
        let caps = capabilities(surface);
        let selected = select_skill(&request(&source, &caps, surface)).unwrap();
        assert_eq!(selected.is_some(), surface == TargetSurface::CodexCli);
    }
}

#[test]
fn unsupported_codex_override_rejects_for_every_selected_cli_even_if_ignored() {
    let source = fixture(&[
        (
            "SKILL.md",
            b"---\nschema: 1\ndescription: demo\nignore: ['SKILL.codex.md']\n---\nBody\n",
        ),
        ("SKILL.codex.md", b"Override"),
    ]);
    for surface in CLI {
        let caps = capabilities(surface);
        let error = select_skill(&request(&source, &caps, surface)).unwrap_err();
        assert!(format!("{error:#}").contains("SKILL.codex.md"));
    }
}

#[test]
fn metadata_projection_preserves_optional_fields_and_keeps_dependencies_out_of_native_yaml() {
    let entry = b"---\nschema: 1\ndescription: demo\nlicense: MIT\nmetadata:\n  author: Team\n  custom: {nested: [one, two]}\nrequires:\n  skills: [helper]\n---\nBody\n";
    let source = fixture(&[("SKILL.md", entry)]);
    for surface in CLI {
        let caps = capabilities(surface);
        let selected = select_skill(&request(&source, &caps, surface))
            .unwrap()
            .unwrap();
        assert_eq!(selected.metadata.license.as_ref().unwrap().0, "MIT");
        assert_eq!(selected.requires.skills[0].name(), "helper");
        let (yaml, _) = frontmatter::parse::<serde_yaml_ng::Value>(
            std::str::from_utf8(&selected.entrypoint.bytes).unwrap(),
        )
        .unwrap();
        assert_eq!(yaml["license"].as_str(), Some("MIT"));
        assert_eq!(yaml["metadata"]["author"].as_str(), Some("Team"));
        assert_eq!(
            yaml["metadata"]["custom"]["nested"][1].as_str(),
            Some("two")
        );
        let keys: Vec<_> = yaml
            .as_mapping()
            .unwrap()
            .keys()
            .map(|key| key.as_str().unwrap())
            .collect();
        assert_eq!(keys, ["name", "description", "license", "metadata"]);
    }
}

#[test]
fn missing_wrong_surface_duplicate_or_deferred_capabilities_reject_without_dropping_metadata() {
    let source = fixture(&[(
        "SKILL.md",
        b"---\nschema: 1\ndescription: demo\nlicense: MIT\nmetadata: {author: Team}\n---\nBody\n",
    )]);
    for surface in CLI {
        for field in ["name", "description", "license", "metadata"] {
            let full = capabilities(surface);
            let mut missing = full.clone();
            missing.retain(|cap| cap.key.name != field);
            let mut duplicate = full.clone();
            duplicate.push(*full.iter().find(|cap| cap.key.name == field).unwrap());
            let mut deferred = full.clone();
            deferred
                .iter_mut()
                .find(|cap| cap.key.name == field)
                .unwrap()
                .representation = Representation::Deferred;
            let wrong = capabilities(if surface == TargetSurface::ClaudeCli {
                TargetSurface::CodexCli
            } else {
                TargetSurface::ClaudeCli
            });
            for caps in [&missing, &duplicate, &deferred, &wrong] {
                let error = select_skill(&request(&source, caps, surface)).unwrap_err();
                assert!(
                    error.downcast_ref::<CapabilityError>().is_some(),
                    "{error:#}"
                );
            }
        }
    }
}

#[test]
fn absent_optional_fields_need_no_capability_declaration() {
    let source = fixture(&[("SKILL.md", ENTRY)]);
    let mut caps = capabilities(TargetSurface::CodexCli);
    caps.retain(|cap| matches!(cap.key.name, "name" | "description"));
    assert!(
        select_skill(&request(&source, &caps, TargetSurface::CodexCli))
            .unwrap()
            .is_some()
    );
}

#[test]
fn native_passthrough_and_unknown_fields_cannot_replace_identity_or_expand_behavior() {
    for field in [
        "claude: {name: hijacked}",
        "copilot: {allowed-tools: Bash}",
        "opencode: {model: other}",
        "codex: {}",
        "allowed-tools: Bash",
        "unexpected: note",
        "claude: []",
        "copilot: ''",
        "opencode: false",
    ] {
        let entry = format!("---\nschema: 1\ndescription: demo\n{field}\n---\nBody\n");
        let source = fixture(&[("SKILL.md", entry.as_bytes())]);
        for surface in CLI {
            let caps = capabilities(surface);
            let error = select_skill(&request(&source, &caps, surface)).unwrap_err();
            assert!(
                format!("{error:#}").contains(field.split(':').next().unwrap()),
                "{error:#}"
            );
        }
    }
}

#[test]
fn empty_native_mapping_blocks_do_not_add_native_frontmatter() {
    let source = fixture(&[(
        "SKILL.md",
        b"---\nschema: 1\ndescription: demo\nclaude: {}\ncopilot: {}\nopencode: {}\n---\nBody\n",
    )]);
    let caps = capabilities(TargetSurface::CodexCli);
    let selected = select_skill(&request(&source, &caps, TargetSurface::CodexCli))
        .unwrap()
        .unwrap();
    let (yaml, _) = frontmatter::parse::<serde_yaml_ng::Value>(
        std::str::from_utf8(&selected.entrypoint.bytes).unwrap(),
    )
    .unwrap();
    assert_eq!(yaml.as_mapping().unwrap().len(), 2);
}

#[test]
fn selected_overrides_reject_frontmatter_and_invalid_utf8_without_falling_back() {
    for (surface, path) in [
        (TargetSurface::ClaudeCli, "SKILL.claude.md"),
        (TargetSurface::CopilotCli, "SKILL.copilot.md"),
        (TargetSurface::OpenCodeCli, "SKILL.opencode.md"),
    ] {
        for bytes in [
            b"---\nname: hijacked\n---\nBody\n".as_slice(),
            b"\xef\xbb\xbf---\r\nname: hijacked\r\n---\r\nBody\r\n",
            &[255],
        ] {
            let source = fixture(&[("SKILL.md", ENTRY), (path, bytes)]);
            let caps = capabilities(surface);
            let error = select_skill(&request(&source, &caps, surface)).unwrap_err();
            assert!(format!("{error:#}").contains(path), "{error:#}");
        }
    }
}

#[test]
fn source_only_files_are_stripped_and_ignore_is_subtractive_for_resources() {
    let source = fixture(&[
        (
            "SKILL.md",
            b"---\nschema: 1\ndescription: demo\nignore: ['private/**', '*.tmp']\n---\nBody\n",
        ),
        ("RULE.md", b"source"),
        ("AGENT.md", b"source"),
        ("RULE.copilot.md", b"source"),
        ("SKILL.claude.md", b"Claude"),
        ("SKILL.opencode.md", b"OpenCode"),
        ("AGENT.claude.md", b"source"),
        ("assets/data.bin", &[0, 255]),
        ("assets/discard.tmp", b"ignored"),
        ("private/key", b"ignored"),
        ("notes.claude.md", b"ordinary resource"),
    ]);
    let caps = capabilities(TargetSurface::CodexCli);
    let selected = select_skill(&request(&source, &caps, TargetSurface::CodexCli))
        .unwrap()
        .unwrap();
    assert_eq!(
        selected
            .resources
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        ["assets/data.bin", "notes.claude.md"]
    );
    assert_eq!(selected.resources[0].bytes, [0, 255]);
}

#[test]
fn selection_uses_captured_bytes_after_source_is_removed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("SKILL.md"), ENTRY).unwrap();
    fs::write(dir.path().join("resource"), b"original").unwrap();
    let source = capture_source(dir.path()).unwrap();
    fs::write(dir.path().join("resource"), b"changed").unwrap();
    dir.close().unwrap();
    let caps = capabilities(TargetSurface::ClaudeCli);
    let selected = select_skill(&request(&source, &caps, TargetSurface::ClaudeCli))
        .unwrap()
        .unwrap();
    assert_eq!(selected.resources[0].bytes, b"original");
}

#[cfg(unix)]
#[test]
fn executable_resource_mode_survives_while_entrypoint_mode_is_unset() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("SKILL.md"), ENTRY).unwrap();
    fs::write(dir.path().join("run.sh"), b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(dir.path().join("run.sh"), fs::Permissions::from_mode(0o751)).unwrap();
    let source = capture_source(dir.path()).unwrap();
    let caps = capabilities(TargetSurface::ClaudeCli);
    let selected = select_skill(&request(&source, &caps, TargetSurface::ClaudeCli))
        .unwrap()
        .unwrap();
    assert_eq!(selected.resources[0].mode, Some(0o751));
    assert_eq!(selected.entrypoint.mode, None);
}

#[test]
fn invalid_names_declared_identity_and_ide_surfaces_reject() {
    let source = fixture(&[("SKILL.md", ENTRY)]);
    let caps = capabilities(TargetSurface::ClaudeCli);
    for name in [
        "",
        "../escape",
        "a/b",
        "Demo",
        "-demo",
        "demo-",
        "demo_name",
        &"a".repeat(65),
    ] {
        let mut req = request(&source, &caps, TargetSurface::ClaudeCli);
        req.alias = Some(name);
        assert!(select_skill(&req).is_err(), "accepted alias {name:?}");
        req.alias = None;
        req.folder_name = name;
        assert!(select_skill(&req).is_err(), "accepted folder {name:?}");
    }
    let different = fixture(&[(
        "SKILL.md",
        b"---\nschema: 1\nname: different\ndescription: demo\n---\nBody\n",
    )]);
    assert!(select_skill(&request(&different, &caps, TargetSurface::ClaudeCli)).is_err());
    for surface in [TargetSurface::CodexIde, TargetSurface::CopilotVscodeLocal] {
        let caps = capabilities(surface);
        assert!(select_skill(&request(&source, &caps, surface)).is_err());
    }
}

#[test]
fn invalid_capture_or_canonical_content_rejects_before_excluded_selection() {
    let original = fixture(&[(
        "SKILL.md",
        b"---\nschema: 1\ndescription: demo\naudience: []\n---\nBody\n",
    )]);
    let mut tampered = original.clone();
    tampered.files[0].bytes.push(0);
    assert!(select_skill(&request(&tampered, &[], TargetSurface::ClaudeCli)).is_err());
    for bytes in [
        b"No metadata".as_slice(),
        b"---\nschema: 2\ndescription: demo\n---\n",
        &[255],
    ] {
        let source = fixture(&[("SKILL.md", bytes)]);
        assert!(select_skill(&request(&source, &[], TargetSurface::ClaudeCli)).is_err());
    }
    let missing = fixture(&[("resource", b"data")]);
    assert!(select_skill(&request(&missing, &[], TargetSurface::ClaudeCli)).is_err());
}

#[test]
fn oversized_description_and_invalid_canonical_directives_reject() {
    for entry in [
        format!(
            "---\nschema: 1\ndescription: {}\n---\nBody\n",
            "a".repeat(1025)
        ),
        "---\nschema: 1\ndescription: demo\n---\n<!-- @client:codex -->\nUnclosed\n".into(),
    ] {
        let source = fixture(&[("SKILL.md", entry.as_bytes())]);
        let caps = capabilities(TargetSurface::CodexCli);
        assert!(select_skill(&request(&source, &caps, TargetSurface::CodexCli)).is_err());
    }
}

#[test]
fn simultaneous_overrides_select_only_the_intended_client_body() {
    let source = fixture(&[
        ("SKILL.md", ENTRY),
        ("SKILL.claude.md", b"Claude body\n"),
        ("SKILL.copilot.md", b"Copilot body\n"),
        ("SKILL.opencode.md", b"OpenCode body\n"),
    ]);
    for (surface, body, origin) in [
        (
            TargetSurface::ClaudeCli,
            "Claude body\n",
            SkillBodyOrigin::Override {
                path: "SKILL.claude.md".into(),
            },
        ),
        (
            TargetSurface::CopilotCli,
            "Copilot body\n",
            SkillBodyOrigin::Override {
                path: "SKILL.copilot.md".into(),
            },
        ),
        (
            TargetSurface::CodexCli,
            "\nRead [data](assets/data.bin).\n",
            SkillBodyOrigin::Canonical,
        ),
        (
            TargetSurface::OpenCodeCli,
            "OpenCode body\n",
            SkillBodyOrigin::Override {
                path: "SKILL.opencode.md".into(),
            },
        ),
    ] {
        let caps = capabilities(surface);
        let selected = select_skill(&request(&source, &caps, surface))
            .unwrap()
            .unwrap();
        assert_eq!(selected.body, body, "{}", surface.name());
        assert_eq!(selected.body_origin, origin);
        assert!(selected.resources.is_empty());
    }
}

#[test]
fn invalid_source_name_rejects_with_a_valid_alias_and_no_declared_name() {
    let source = fixture(&[(
        "SKILL.md",
        b"---\nschema: 1\ndescription: demo\n---\nBody\n",
    )]);
    let caps = capabilities(TargetSurface::ClaudeCli);
    let mut req = request(&source, &caps, TargetSurface::ClaudeCli);
    req.alias = Some("valid-alias");
    assert!(select_skill(&req).unwrap().is_some());
    for name in [
        "",
        "../escape",
        "a/b",
        "Demo",
        "-demo",
        "demo-",
        "demo_name",
        &"a".repeat(65),
    ] {
        req.folder_name = name;
        let error = select_skill(&req)
            .expect_err("invalid source folder must reject despite a valid alias");
        assert!(format!("{error:#}").contains("folder name"), "{error:#}");
    }
}

#[test]
fn supporting_resources_preserve_literal_client_directives() {
    let bytes = b"Common\r\n<!-- @client:claude -->\r\nClaude notes\r\n<!-- @endclient -->\r\n<!-- @client:codex -->\r\nCodex notes\r\n<!-- @endclient -->\r\n";
    let source = fixture(&[("SKILL.md", ENTRY), ("references/procedure.md", bytes)]);
    for surface in CLI {
        let caps = capabilities(surface);
        let selected = select_skill(&request(&source, &caps, surface))
            .unwrap()
            .unwrap();
        assert_eq!(selected.resources.len(), 1);
        assert_eq!(selected.resources[0].path, "references/procedure.md");
        assert_eq!(selected.resources[0].bytes, bytes);
        assert_eq!(
            selected.resources[0].content_sha256,
            source.files[1].content_sha256
        );
    }
}

#[test]
fn source_topology_collision_rejects_even_with_valid_capture_digests() {
    use sha2::{Digest, Sha256};

    let mut source = fixture(&[("SKILL.md", ENTRY), ("assets/data.bin", b"data")]);
    let caps = capabilities(TargetSurface::ClaudeCli);
    assert!(
        select_skill(&request(&source, &caps, TargetSurface::ClaudeCli))
            .unwrap()
            .is_some()
    );
    let mut parent = source.files[1].clone();
    parent.path = "assets".into();
    source.files.push(parent);
    source
        .files
        .sort_by(|left, right| left.path.cmp(&right.path));

    // Independent encoding of the documented manifest protocol, so the fixture
    // passes digest validation while remaining impossible as a physical tree.
    let mut digest = Sha256::new();
    digest.update(b"upskill:source-manifest:v1\0");
    digest.update((source.files.len() as u64).to_be_bytes());
    for file in &source.files {
        digest.update((file.path.len() as u64).to_be_bytes());
        digest.update(file.path.as_bytes());
        digest.update((file.bytes.len() as u64).to_be_bytes());
        digest.update(&file.bytes);
        match file.mode {
            None => digest.update([0]),
            Some(mode) => {
                digest.update([1]);
                digest.update(mode.to_be_bytes());
            }
        }
    }
    source.content_sha256 = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    upskill::source_manifest::validate_manifest(&source).unwrap();
    for surface in CLI {
        let caps = capabilities(surface);
        let error = select_skill(&request(&source, &caps, surface))
            .expect_err("file/descendant conflict must reject");
        assert!(format!("{error:#}").contains("topology"), "{error:#}");
    }
}
