use std::path::PathBuf;

use serde_json::{Value, json};
use upskill::instructions::aggregate::{Contribution, OwnedContribution, reconcile};
use upskill::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
    parse_artifacts, serialize_artifacts, validate_artifacts,
};
use upskill::pipeline::ItemKind;
use upskill::target::TargetSurface;

const HASH: &str = "7370422ff30a9380aedf0a5d7a9d701953b284b1576e9fb2bfa9a1cac392dfdf";

fn owner(id: &str) -> ArtifactOwner {
    ArtifactOwner {
        installation_id: id.into(),
        source: "driftsys/guidance@v1:rules".into(),
        source_revision: Some("0123456789abcdef".into()),
        kind: ItemKind::Rule,
        source_name: "original-guidance".into(),
        installed_name: "local-alias".into(),
    }
}

fn second_owner() -> ArtifactOwner {
    ArtifactOwner {
        installation_id: "install-b".into(),
        source: "other/assistants@v2:agents".into(),
        source_revision: Some("fedcba9876543210".into()),
        kind: ItemKind::Agent,
        source_name: "original-agent".into(),
        installed_name: "agent-alias".into(),
    }
}

fn file() -> OwnedArtifact {
    OwnedArtifact {
        root: InstallationRoot {
            scope: OwnershipScope::Project,
            path: std::env::temp_dir().join("ownership-record-root"),
        },
        path: "rules/guidance.md".into(),
        content: ArtifactContent::File {
            content_sha256: HASH.into(),
            mode: Some(0o644),
        },
        owners: vec![owner("install-a")],
        consumers: vec![TargetSurface::ClaudeCli],
    }
}

fn contribution(id: &str) -> OwnedArtifact {
    let mut artifact = file();
    artifact.path = "AGENTS.md".into();
    artifact.content = ArtifactContent::MarkdownContribution {
        contribution: OwnedContribution {
            id: id.into(),
            content_sha256: HASH.into(),
            leading_newlines: 1,
            preceding_newlines: 3,
        },
    };
    artifact
}

fn assert_invalid(records: &[OwnedArtifact], reason: &str) {
    let results = [
        ("validate_artifacts", validate_artifacts(records)),
        (
            "serialize_artifacts",
            serialize_artifacts(records).map(|_| ()),
        ),
        (
            "parse_artifacts",
            parse_artifacts(&serde_json::to_string(records).unwrap()).map(|_| ()),
        ),
    ];
    let accepted: Vec<_> = results
        .iter()
        .filter_map(|(boundary, result)| result.is_ok().then_some(*boundary))
        .collect();
    assert!(
        accepted.is_empty(),
        "{reason}: unexpectedly accepted by {accepted:?}"
    );
    for (_, result) in results {
        let error = result.unwrap_err();
        assert!(format!("{error:#}").contains("owned artifact"), "{error:#}");
    }
}

#[test]
fn codec_preserves_alias_revision_shared_owners_and_editor_record() {
    let edit = reconcile(
        "Manual.\n\n\n",
        &[],
        &[Contribution::new("alpha", "Guidance.\n")],
    )
    .unwrap();
    assert_eq!(edit.owned[0].content_sha256, HASH);
    assert_eq!(edit.owned[0].leading_newlines, 1);
    assert_eq!(edit.owned[0].preceding_newlines, 3);
    let mut artifact = contribution("alpha");
    artifact.content = ArtifactContent::MarkdownContribution {
        contribution: edit.owned[0].clone(),
    };
    artifact.owners.push(second_owner());
    artifact.consumers = vec![TargetSurface::CodexCli, TargetSurface::CopilotCli];
    let records = vec![artifact];
    let json = serialize_artifacts(&records).unwrap();
    let decoded = parse_artifacts(&json).unwrap();
    assert_eq!(decoded, records);
    assert_eq!(decoded[0].owners[0].source_name, "original-guidance");
    assert_eq!(decoded[0].owners[0].installed_name, "local-alias");
    assert_eq!(decoded[0].owners[1], second_owner());
    assert_eq!(
        decoded[0].owners[0].source_revision.as_deref(),
        Some("0123456789abcdef")
    );
    let value: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value[0]["root"]["scope"], "project");
    assert_eq!(value[0]["content"]["kind"], "markdown-contribution");
    assert_eq!(value[0]["content"]["contribution"]["content_sha256"], HASH);
    assert_eq!(value[0]["content"]["contribution"]["leading_newlines"], 1);
    assert_eq!(value[0]["content"]["contribution"]["preceding_newlines"], 3);
}

#[test]
fn codec_preserves_zero_owned_newlines_and_unbounded_preceding_count() {
    let mut artifact = contribution("alpha");
    if let ArtifactContent::MarkdownContribution { contribution } = &mut artifact.content {
        contribution.leading_newlines = 0;
        contribution.preceding_newlines = 1024;
    }
    assert_eq!(
        parse_artifacts(&serialize_artifacts(&[artifact.clone()]).unwrap()).unwrap(),
        vec![artifact]
    );
}

#[test]
fn file_modes_and_optional_revision_round_trip() {
    for mode in [None, Some(0), Some(0o7777)] {
        let mut artifact = file();
        artifact.owners[0].source_revision = None;
        artifact.content = ArtifactContent::File {
            content_sha256: HASH.into(),
            mode,
        };
        let json = serialize_artifacts(&[artifact.clone()]).unwrap();
        assert_eq!(parse_artifacts(&json).unwrap(), vec![artifact]);
        assert_eq!(
            serde_json::from_str::<Value>(&json).unwrap()[0]["content"]["kind"],
            "file"
        );
    }
}

#[test]
fn all_target_surfaces_use_stable_wire_names() {
    for (surface, name) in [
        (TargetSurface::ClaudeCli, "claude.cli"),
        (TargetSurface::CopilotCli, "copilot.cli"),
        (TargetSurface::CopilotVscodeLocal, "copilot.vscode-local"),
        (TargetSurface::CodexCli, "codex.cli"),
        (TargetSurface::CodexIde, "codex.ide"),
        (TargetSurface::OpenCodeCli, "opencode.cli"),
    ] {
        let mut artifact = file();
        artifact.consumers = vec![surface];
        let json = serialize_artifacts(&[artifact.clone()]).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&json).unwrap()[0]["consumers"],
            json!([name])
        );
        assert_eq!(parse_artifacts(&json).unwrap(), vec![artifact]);
    }
}

#[test]
fn rejects_nonabsolute_or_empty_roots() {
    for path in [PathBuf::new(), PathBuf::from("relative/root")] {
        let mut artifact = file();
        artifact.root.path = path;
        assert_invalid(&[artifact], "root must be absolute");
    }
}

#[test]
fn rejects_nonportable_relative_paths() {
    for path in [
        "",
        "/absolute",
        "//server/file",
        "a//b",
        "a/",
        ".",
        "..",
        "a/./b",
        "a/../b",
        "a\\b",
        "C:drive",
        "a:b",
        "a\0b",
        "a\nb",
        "a\tb",
        "a\u{7f}b",
        "a\u{85}b",
    ] {
        let mut artifact = file();
        artifact.path = path.into();
        assert_invalid(&[artifact], path);
    }
}

#[test]
fn accepts_portable_unicode_and_space_paths_without_filesystem_access() {
    let mut artifact = file();
    artifact.path = "manual notes/例.md".into();
    validate_artifacts(&[artifact]).unwrap();
}

#[test]
fn rejects_empty_owners_or_consumers() {
    let mut artifact = file();
    artifact.owners.clear();
    assert_invalid(&[artifact], "owner required");
    let mut artifact = file();
    artifact.consumers.clear();
    assert_invalid(&[artifact], "consumer required");
}

#[test]
fn rejects_duplicate_owners_and_consumers() {
    let mut artifact = file();
    artifact.owners.push(artifact.owners[0].clone());
    assert_invalid(&[artifact], "duplicate installation id");
    let mut artifact = file();
    artifact.consumers.push(artifact.consumers[0]);
    assert_invalid(&[artifact], "duplicate consumer");
}

#[test]
fn rejects_empty_owner_binding_fields() {
    for field in ["source", "source_name", "installed_name", "source_revision"] {
        let mut artifact = file();
        match field {
            "source" => artifact.owners[0].source.clear(),
            "source_name" => artifact.owners[0].source_name.clear(),
            "installed_name" => artifact.owners[0].installed_name.clear(),
            _ => artifact.owners[0].source_revision = Some(String::new()),
        }
        assert_invalid(&[artifact], field);
    }
}

#[test]
fn rejects_invalid_installation_and_contribution_ids() {
    for id in ["", "has space", "has.dot", "has/slash", "é", "has\nnewline"] {
        let mut artifact = file();
        artifact.owners[0].installation_id = id.into();
        assert_invalid(&[artifact], id);
        assert_invalid(&[contribution(id)], id);
    }
    let mut artifact = contribution("AZaz09_-");
    artifact.owners[0].installation_id = "AZaz09_-".into();
    validate_artifacts(&[artifact]).unwrap();
}

#[test]
fn rejects_invalid_file_and_contribution_hashes() {
    for hash in [
        "".to_owned(),
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
        format!("{}é", "a".repeat(62)),
    ] {
        let mut artifact = file();
        artifact.content = ArtifactContent::File {
            content_sha256: hash.clone(),
            mode: None,
        };
        assert_invalid(&[artifact], &hash);
        let mut artifact = contribution("alpha");
        if let ArtifactContent::MarkdownContribution { contribution } = &mut artifact.content {
            contribution.content_sha256 = hash.clone();
        }
        assert_invalid(&[artifact], &hash);
    }
}

#[test]
fn rejects_nonpermission_mode_bits() {
    for mode in [0o10000, 0o100644, u32::MAX] {
        let mut artifact = file();
        artifact.content = ArtifactContent::File {
            content_sha256: HASH.into(),
            mode: Some(mode),
        };
        assert_invalid(&[artifact], "mode contains nonpermission bits");
    }
}

#[test]
fn validates_owned_newline_limit() {
    for leading in [0, 1, 2, 3, u8::MAX] {
        let mut artifact = contribution("alpha");
        if let ArtifactContent::MarkdownContribution { contribution } = &mut artifact.content {
            contribution.leading_newlines = leading;
        }
        if leading <= 2 {
            validate_artifacts(&[artifact]).unwrap();
        } else {
            assert_invalid(&[artifact], "too many owned newlines");
        }
    }
}

#[test]
fn rejects_duplicate_file_and_contribution_selectors() {
    assert_invalid(&[file(), file()], "duplicate file location");
    assert_invalid(
        &[contribution("alpha"), contribution("alpha")],
        "duplicate contribution selector",
    );
}

#[test]
fn rejects_duplicate_files_with_different_valid_payload_or_metadata() {
    for difference in ["hash", "mode", "owners", "consumers"] {
        let first = file();
        let mut second = first.clone();
        match difference {
            "hash" => {
                second.content = ArtifactContent::File {
                    content_sha256: "0".repeat(64),
                    mode: Some(0o644),
                }
            }
            "mode" => {
                second.content = ArtifactContent::File {
                    content_sha256: HASH.into(),
                    mode: Some(0o600),
                }
            }
            "owners" => second.owners = vec![second_owner()],
            _ => second.consumers = vec![TargetSurface::CodexCli],
        }
        validate_artifacts(std::slice::from_ref(&first)).unwrap();
        validate_artifacts(std::slice::from_ref(&second)).unwrap();
        assert_invalid(&[first.clone(), second.clone()], difference);
        assert_invalid(&[second, first], &format!("{difference}, reversed"));
    }
}

#[test]
fn rejects_duplicate_contributions_with_different_valid_payload_or_metadata() {
    for difference in [
        "hash",
        "leading_newlines",
        "preceding_newlines",
        "owners",
        "consumers",
    ] {
        let first = contribution("alpha");
        let mut second = first.clone();
        let ArtifactContent::MarkdownContribution { contribution } = &mut second.content else {
            unreachable!();
        };
        match difference {
            "hash" => contribution.content_sha256 = "0".repeat(64),
            "leading_newlines" => contribution.leading_newlines = 0,
            "preceding_newlines" => contribution.preceding_newlines = 17,
            "owners" => second.owners = vec![second_owner()],
            _ => second.consumers = vec![TargetSurface::CodexCli],
        }
        validate_artifacts(std::slice::from_ref(&first)).unwrap();
        validate_artifacts(std::slice::from_ref(&second)).unwrap();
        assert_invalid(&[first.clone(), second.clone()], difference);
        assert_invalid(&[second, first], &format!("{difference}, reversed"));
    }
}

#[test]
fn rejects_file_contribution_mixtures_in_either_order() {
    let first = contribution("alpha");
    let mut second = file();
    second.path = first.path.clone();
    assert_invalid(&[first.clone(), second.clone()], "mixed ownership");
    assert_invalid(&[second, first], "mixed ownership reversed");
}

#[test]
fn allows_distinct_contributions_and_matching_owner_across_records() {
    let records = vec![contribution("alpha"), contribution("beta")];
    validate_artifacts(&records).unwrap();
    assert_eq!(
        parse_artifacts(&serialize_artifacts(&records).unwrap()).unwrap(),
        records
    );
}

#[test]
fn allows_equal_paths_under_distinct_root_paths_and_scopes() {
    let first = file();
    let mut second = first.clone();
    second.root.path = second.root.path.join("other");
    let mut third = first.clone();
    third.root.scope = OwnershipScope::User;
    let records = vec![first, second, third];
    validate_artifacts(&records).unwrap();
    assert_eq!(
        parse_artifacts(&serialize_artifacts(&records).unwrap()).unwrap(),
        records
    );
}

#[test]
fn user_scope_serializes_with_literal_wire_name() {
    let mut artifact = file();
    artifact.root.scope = OwnershipScope::User;
    let json = serialize_artifacts(&[artifact]).unwrap();
    let value: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value[0]["root"]["scope"], "user");
}

#[test]
fn user_scope_parses_literal_wire_name() {
    let mut expected = file();
    expected.root.scope = OwnershipScope::User;
    let literal = json!([{
        "root": {"scope": "user", "path": expected.root.path},
        "path": "rules/guidance.md",
        "content": {"kind": "file", "content_sha256": HASH, "mode": 420},
        "owners": [{
            "installation_id": "install-a",
            "source": "driftsys/guidance@v1:rules",
            "source_revision": "0123456789abcdef",
            "kind": "rule",
            "source_name": "original-guidance",
            "installed_name": "local-alias"
        }],
        "consumers": ["claude.cli"]
    }]);
    assert_eq!(
        parse_artifacts(&literal.to_string()).unwrap(),
        vec![expected]
    );
}

#[test]
fn rejects_installation_binding_changes_across_records() {
    for (pair, template, second_content) in [
        ("file/file", file(), file().content),
        (
            "contribution/contribution",
            contribution("alpha"),
            contribution("beta").content,
        ),
        ("file/contribution", file(), contribution("alpha").content),
    ] {
        for owner_index in [0, 1] {
            for field in [
                "source",
                "source_revision",
                "kind",
                "source_name",
                "installed_name",
            ] {
                let mut first = template.clone();
                first.owners.push(second_owner());
                let mut second = first.clone();
                second.path = "other.md".into();
                second.content = second_content.clone();
                validate_artifacts(&[first.clone(), second.clone()]).unwrap();
                let changed_owner = &mut second.owners[owner_index];
                match field {
                    "source" => changed_owner.source = "other/repo".into(),
                    "source_revision" => changed_owner.source_revision = None,
                    "kind" => changed_owner.kind = ItemKind::Skill,
                    "source_name" => changed_owner.source_name = "different-source".into(),
                    _ => changed_owner.installed_name = "different-alias".into(),
                }
                let reason = format!("{pair}, {field} on owner {owner_index}");
                assert_invalid(&[first.clone(), second.clone()], &reason);
                assert_invalid(&[second, first], &format!("{reason}, reversed"));
            }
        }
    }
}

#[test]
fn deterministic_serialization_sorts_records_owners_and_consumers_without_mutation() {
    let owner_by_id_first = ArtifactOwner {
        source: "z/source".into(),
        source_revision: Some("z-revision".into()),
        kind: ItemKind::Agent,
        ..owner("order-a")
    };
    let owner_by_id_second = ArtifactOwner {
        installation_id: "order-b".into(),
        source: "a/source".into(),
        source_revision: Some("a-revision".into()),
        kind: ItemKind::Rule,
        ..second_owner()
    };
    let mut beta = contribution("beta");
    beta.owners = vec![owner_by_id_second.clone(), owner_by_id_first.clone()];
    beta.consumers = vec![TargetSurface::OpenCodeCli, TargetSurface::ClaudeCli];
    let mut alpha = beta.clone();
    alpha.content = contribution("alpha").content;
    let mut earlier_path = file();
    earlier_path.path = "A.md".into();
    let mut later_root = file();
    later_root.root.path = later_root.root.path.join("z");
    later_root.path = "0.md".into();
    let mut user = file();
    user.root.scope = OwnershipScope::User;
    let records = vec![user, later_root, beta, alpha, earlier_path];
    let before = records.clone();
    let mut reordered = records.clone();
    reordered.reverse();
    for record in &mut reordered {
        record.owners.reverse();
        record.consumers.reverse();
    }
    let json = serialize_artifacts(&records).unwrap();
    assert_eq!(json, serialize_artifacts(&reordered).unwrap());
    assert_eq!(records, before);
    assert!(json.starts_with("[\n  {\n"));
    assert!(json.ends_with("]\n"));
    assert!(!json.ends_with("\n\n"));
    let decoded = parse_artifacts(&json).unwrap();
    assert_eq!(decoded[0].path, "A.md");
    assert_eq!(decoded[1].content, contribution("alpha").content);
    assert_eq!(decoded[2].content, contribution("beta").content);
    assert_eq!(
        decoded[1].owners,
        vec![owner_by_id_first, owner_by_id_second]
    );
    assert_eq!(
        decoded[1].consumers,
        vec![TargetSurface::ClaudeCli, TargetSurface::OpenCodeCli]
    );
    assert_eq!(decoded[3].root.path, before[1].root.path);
    assert_eq!(decoded[3].path, "0.md");
    assert_eq!(decoded[4].root.scope, OwnershipScope::User);
}

#[test]
fn rejects_unknown_fields_at_every_record_layer() {
    for pointer in ["/0", "/0/root", "/0/owners/0", "/0/content"] {
        let mut value = serde_json::to_value(vec![file()]).unwrap();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        let error = parse_artifacts(&value.to_string()).expect_err(pointer);
        assert!(format!("{error:#}").contains("unknown field"), "{error:#}");
    }
    let mut value = serde_json::to_value(vec![contribution("alpha")]).unwrap();
    value[0]["content"]["contribution"]["unexpected"] = json!(true);
    assert!(
        format!("{:#}", parse_artifacts(&value.to_string()).unwrap_err()).contains("unknown field")
    );
}

#[test]
fn rejects_malformed_records_and_unknown_wire_variants_with_context() {
    for json in ["{", "{}", "[{}]", "null"] {
        assert!(
            format!("{:#}", parse_artifacts(json).unwrap_err()).contains("parse owned artifacts")
        );
    }
    for (pointer, value, expected) in [
        (
            "/0/consumers/0",
            json!("future.cli"),
            "unknown target surface",
        ),
        ("/0/root/scope", json!("system"), "unknown variant"),
        ("/0/content/kind", json!("directory"), "unknown variant"),
        ("/0/owners/0/kind", json!("plugin"), "unknown variant"),
    ] {
        let mut record = serde_json::to_value(vec![file()]).unwrap();
        *record.pointer_mut(pointer).unwrap() = value;
        let error = parse_artifacts(&record.to_string()).unwrap_err();
        let detail = format!("{error:#}");
        assert!(
            detail.contains("parse owned artifacts") && detail.contains(expected),
            "{detail}"
        );
    }
}

#[test]
fn empty_record_set_is_valid() {
    assert_eq!(serialize_artifacts(&[]).unwrap(), "[]\n");
    assert!(parse_artifacts("[]").unwrap().is_empty());
}

#[test]
fn lexical_root_spellings_are_not_canonicalized_or_component_normalized() {
    let first = file();
    let mut dotted = first.clone();
    dotted.root.path = PathBuf::from(format!("{}/.", first.root.path.display()));
    let mut parent = first.clone();
    parent.root.path = first.root.path.join("child/..");
    let records = vec![first, dotted, parent];
    validate_artifacts(&records).unwrap();
    let decoded = parse_artifacts(&serialize_artifacts(&records).unwrap()).unwrap();
    assert_eq!(
        decoded[1].root.path.as_os_str(),
        records[1].root.path.as_os_str()
    );
    assert_eq!(
        decoded[2].root.path.as_os_str(),
        records[2].root.path.as_os_str()
    );
}
