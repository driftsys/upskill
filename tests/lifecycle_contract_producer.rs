use sha2::{Digest, Sha256};
use upskill::generate::Client;
use upskill::lifecycle::contracts::ContractContent;
use upskill::lifecycle::desired::{
    CompilerBinding, RuleDesiredSetRequest, compile_rule_desired_set_with_contracts,
};
use upskill::lifecycle::state::{Installation, SourceEvidence};
use upskill::operations::plan::DesiredArtifact;
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::source_manifest::{SourceManifest, capture_source};
use upskill::target::{TargetFamily, TargetProfile, TargetSurface};

fn source(body: &[u8]) -> SourceManifest {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join("RULE.md"), body).unwrap();
    std::fs::write(t.path().join("resource.bin"), [0, 255, 3]).unwrap();
    capture_source(t.path()).unwrap()
}
const RULE: &[u8] = b"---\nschema: 1\nname: source\ndescription: demo\n---\n\n# Demo\n";
fn item(s: &SourceManifest) -> Installation {
    Installation {
        owner: ArtifactOwner {
            installation_id: "owner".into(),
            source: "./gone".into(),
            source_revision: None,
            kind: ItemKind::Rule,
            source_name: "source".into(),
            installed_name: "alias".into(),
        },
        root: InstallationRoot {
            scope: OwnershipScope::Project,
            path: std::env::temp_dir().join("upskill-produced-contracts"),
        },
        requested: vec![TargetSurface::CopilotVscodeLocal, TargetSurface::CopilotCli],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: s.content_sha256.clone(),
        },
    }
}
fn profiles() -> Vec<TargetProfile> {
    vec![
        TargetProfile {
            family: TargetFamily::Copilot,
            surface: TargetSurface::CopilotVscodeLocal,
            contract_revision: 9,
        },
        TargetProfile {
            family: TargetFamily::Copilot,
            surface: TargetSurface::CopilotCli,
            contract_revision: 4,
        },
    ]
}
fn binding() -> Vec<CompilerBinding> {
    vec![CompilerBinding {
        client: Client::Copilot,
        consumers: vec![TargetSurface::CopilotCli, TargetSurface::CopilotVscodeLocal],
    }]
}
#[test]
fn explicit_revisions_and_captured_output_bind_every_contract() {
    let s = source(RULE);
    let i = item(&s);
    let b = binding();
    let p = profiles();
    let before = (s.clone(), i.clone(), b.clone(), p.clone());
    let out = compile_rule_desired_set_with_contracts(
        RuleDesiredSetRequest {
            installation: &i,
            source: &s,
            folder_name: "folder",
            bindings: &b,
        },
        &p,
    )
    .unwrap();
    assert_eq!(out.set.installation, i);
    assert_eq!(out.contracts.len(), out.set.artifacts.len());
    assert_eq!(out.contracts.len(), 2);
    assert_eq!(
        out.contracts
            .iter()
            .map(|c| c.path.as_str())
            .collect::<Vec<_>>(),
        vec![
            ".github/instructions/alias.instructions.md",
            ".github/instructions/alias/resource.bin"
        ]
    );
    let DesiredArtifact::File(resource) = &out.set.artifacts[1] else {
        panic!("resource is not a file")
    };
    assert_eq!(resource.value.bytes, vec![0, 255, 3]);
    for (artifact, c) in out.set.artifacts.iter().zip(&out.contracts) {
        let DesiredArtifact::File(f) = artifact else {
            panic!("unexpected instruction")
        };
        assert_eq!(c.root, i.root);
        assert_eq!(c.path, f.location.path);
        assert_eq!(c.installation_id, "owner");
        assert_eq!(c.source_manifest_sha256, s.content_sha256);
        assert_eq!(c.contribution_id, None);
        assert_eq!(
            c.profiles
                .iter()
                .map(|p| (p.surface, p.contract_revision))
                .collect::<Vec<_>>(),
            vec![
                (TargetSurface::CopilotCli, 4),
                (TargetSurface::CopilotVscodeLocal, 9)
            ]
        );
        let hash: String = Sha256::digest(&f.value.bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            c.content,
            ContractContent::File {
                content_sha256: hash,
                mode: f.value.mode
            }
        );
    }
    assert_eq!((s, i, b, p), before);
}
#[test]
fn exact_explicit_profile_coverage_family_and_revision_are_required() {
    let s = source(RULE);
    let i = item(&s);
    let b = binding();
    let request = RuleDesiredSetRequest {
        installation: &i,
        source: &s,
        folder_name: "folder",
        bindings: &b,
    };
    let good = profiles();
    let mut bad = vec![
        vec![],
        vec![good[0]],
        vec![good[0], good[0]],
        vec![good[0], good[1], good[0]],
        vec![good[0], good[1], TargetSurface::ClaudeCli.profile()],
    ];
    let mut p = good.clone();
    p[0].family = TargetFamily::Claude;
    bad.push(p);
    let mut p = good.clone();
    p[0].contract_revision = 0;
    bad.push(p);
    for p in bad {
        assert!(compile_rule_desired_set_with_contracts(request, &p).is_err());
    }
}
#[test]
fn excluded_audience_keeps_history_without_artifact_contracts() {
    let s = source(
        b"---\nschema: 1\nname: source\ndescription: demo\naudience: [claude]\n---\n\n# Demo\n",
    );
    let i = item(&s);
    let b = binding();
    let out = compile_rule_desired_set_with_contracts(
        RuleDesiredSetRequest {
            installation: &i,
            source: &s,
            folder_name: "folder",
            bindings: &b,
        },
        &profiles(),
    )
    .unwrap();
    assert!(out.set.artifacts.is_empty());
    assert!(out.contracts.is_empty());
    assert_eq!(
        out.set.audience_excluded,
        vec![TargetSurface::CopilotCli, TargetSurface::CopilotVscodeLocal]
    );
    assert_eq!(out.set.installation, i);
    let request = RuleDesiredSetRequest {
        installation: &i,
        source: &s,
        folder_name: "folder",
        bindings: &b,
    };
    let good = profiles();
    let mut wrong = good.clone();
    wrong[0].family = TargetFamily::Claude;
    let mut zero = good.clone();
    zero[0].contract_revision = 0;
    for bad in [
        vec![],
        vec![good[0], good[0]],
        vec![good[0], good[1], good[0]],
        vec![good[0], good[1], TargetSurface::ClaudeCli.profile()],
        wrong,
        zero,
    ] {
        assert!(compile_rule_desired_set_with_contracts(request, &bad).is_err());
    }
}
#[test]
fn mismatched_capture_is_not_promoted_to_contract_evidence() {
    let s = source(RULE);
    let mut i = item(&s);
    i.source_evidence = SourceEvidence::Local {
        manifest_sha256: "f".repeat(64),
    };
    let b = binding();
    assert!(
        compile_rule_desired_set_with_contracts(
            RuleDesiredSetRequest {
                installation: &i,
                source: &s,
                folder_name: "folder",
                bindings: &b
            },
            &profiles()
        )
        .is_err()
    );
}
