use super::*;
use crate::compile::skill_package::SkillPackage;
use crate::compile::skill_selection::SelectedSkill;
use crate::lifecycle::native_receipt::{
    NativeBindingOutcome, NativeBindingReceipt, NativeSkillSelection,
};
use crate::lifecycle::skill_delivery::{PackageRevision, StagedSkillRevision};
use crate::lifecycle::state::CombinedState;
use crate::lifecycle::state::Installation;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    key: SkillTarget,
    scope: InstallationRoot,
    client_version: String,
    platform: String,
    operation_id: String,
    before: Option<NativeSkillSelection>,
    after: Option<NativeSkillSelection>,
    prestate: EvidenceReference,
    outcome: NativeBindingOutcome,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    location: crate::files::plan::FileLocation,
    surface: crate::target::TargetSurface,
    snapshot: EvidenceReference,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Revision {
    installation: Installation,
    // Selected metadata can contain YAML mapping keys which JSON cannot
    // represent losslessly. Keep a strict typed YAML capture inside the envelope.
    selected_yaml: String,
    package: SkillPackage,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Staged {
    revision: Revision,
    operation_id: String,
    paths: Vec<String>,
}
fn encode_revision(revision: &PackageRevision) -> Result<Revision> {
    let selected_yaml = serde_yaml_ng::to_string(&revision.selected)?;
    ensure!(
        selected(&selected_yaml)? == revision.selected,
        "selected evidence cannot round-trip losslessly"
    );
    Ok(Revision {
        installation: revision.installation.clone(),
        selected_yaml,
        package: revision.package.clone(),
    })
}
fn decode_revision(revision: Revision) -> Result<PackageRevision> {
    Ok(PackageRevision {
        installation: revision.installation,
        selected: selected(&revision.selected_yaml)?,
        package: revision.package,
    })
}
fn selected(json: &str) -> Result<SelectedSkill> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(json)?;
    let selected: SelectedSkill = serde_yaml_ng::from_value(value.clone())?;
    ensure!(
        serde_yaml_ng::to_value(&selected)? == value,
        "unknown or noncanonical selected evidence fields"
    );
    Ok(selected)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    schema: u32,
    file_policy: String,
    context: NativeStateContext,
    graph: CombinedState,
    intents: Vec<SkillDeliveryIntent>,
    revisions: Vec<Revision>,
    staged: Vec<Staged>,
    receipts: Vec<Receipt>,
    files: Vec<OwnedArtifact>,
    configurations: Vec<Configuration>,
}
fn decode<T: DeserializeOwned + Serialize>(json: &str) -> Result<T> {
    let value = crate::parse::json::parse_unique(json)?;
    let decoded: T = serde_json::from_value(value.clone())?;
    // Reused model types can have defaults/catchalls. Reject silently discarded
    // structural fields without changing their legacy deserializers globally.
    ensure!(
        serde_json::to_value(&decoded)? == value,
        "unknown or noncanonical native state fields"
    );
    Ok(decoded)
}

/// Encode one validated state and the private evidence it references. The caller
/// must durably publish blobs under the qualified private root before the JSON.
/// This function performs no I/O or migration and proves no native activation.
pub fn serialize_native_state(state: &NativeDeliveryState) -> Result<EncodedNativeState> {
    validate::state(state)?;
    let mut blobs = evidence::Blobs::new();
    let mut receipts = Vec::new();
    let mut configurations = Vec::new();
    for receipt in &state.skills.receipts {
        receipts.push(Receipt {
            key: receipt.key.clone(),
            scope: receipt.scope.clone(),
            client_version: receipt.client_version.clone(),
            platform: receipt.platform.clone(),
            operation_id: receipt.operation_id.clone(),
            before: receipt.before.clone(),
            after: receipt.after.clone(),
            prestate: evidence::add(
                &mut blobs,
                EvidenceKind::NativePrestate,
                receipt.captured_prestate.clone(),
            )?,
            outcome: receipt.outcome,
        });
    }
    for config in &state.configurations {
        configurations.push(Configuration {
            location: config.capture.target.file.location.clone(),
            surface: config.capture.surface,
            snapshot: evidence::add(
                &mut blobs,
                EvidenceKind::ConfigurationSnapshot,
                serde_json::to_vec(config)?,
            )?,
        });
    }
    let wire = Wire {
        schema: 4,
        file_policy: super::super::SKILL_FILE_POLICY.into(),
        context: state.context.clone(),
        graph: state.skills.graph.clone(),
        intents: state.intents.clone(),
        revisions: state
            .skills
            .revisions
            .iter()
            .map(encode_revision)
            .collect::<Result<Vec<_>>>()?,
        staged: state
            .skills
            .staged
            .iter()
            .map(|staged| {
                Ok(Staged {
                    revision: encode_revision(&staged.revision)?,
                    operation_id: staged.operation_id.clone(),
                    paths: staged.paths.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?,
        receipts,
        files: state.files.clone(),
        configurations,
    };
    let mut json = serde_json::to_string_pretty(&wire)?;
    json.push('\n');
    // Refuse representations which cannot round-trip through strict JSON
    // (for example unsupported arbitrary YAML metadata), before any effects.
    let _: Wire = decode(&json)?;
    Ok(EncodedNativeState {
        json,
        private_evidence: blobs.into_values().collect(),
    })
}

/// Decode using an expected machine-local context and exact private evidence.
/// Context equality and digests validate claims, not filesystem identities.
pub fn parse_native_state(
    json: &str,
    expected: &NativeStateContext,
    evidence: &[EvidenceBlob],
) -> Result<NativeDeliveryState> {
    let wire = state_wire(json, expected)?;
    let blobs = evidence::index(evidence)?;
    let receipts = wire
        .receipts
        .into_iter()
        .map(|r| {
            Ok(NativeBindingReceipt {
                key: r.key,
                scope: r.scope,
                client_version: r.client_version,
                platform: r.platform,
                operation_id: r.operation_id,
                before: r.before,
                after: r.after,
                captured_prestate: evidence::resolve(
                    &blobs,
                    &r.prestate,
                    EvidenceKind::NativePrestate,
                )?
                .to_vec(),
                outcome: r.outcome,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut configurations = Vec::new();
    for config in wire.configurations {
        let bytes = evidence::resolve(
            &blobs,
            &config.snapshot,
            EvidenceKind::ConfigurationSnapshot,
        )?;
        let captured: NativeConfigState = decode(std::str::from_utf8(bytes)?)?;
        ensure!(
            captured.capture.target.file.location == config.location
                && captured.capture.surface == config.surface,
            "private configuration snapshot binding differs"
        );
        configurations.push(captured);
    }
    let state = NativeDeliveryState {
        context: wire.context.clone(),
        skills: SkillDeliveryState {
            scope: wire.context.scope,
            graph: wire.graph,
            revisions: wire
                .revisions
                .into_iter()
                .map(decode_revision)
                .collect::<Result<Vec<_>>>()?,
            staged: wire
                .staged
                .into_iter()
                .map(|staged| {
                    Ok(StagedSkillRevision {
                        revision: decode_revision(staged.revision)?,
                        operation_id: staged.operation_id,
                        paths: staged.paths,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            receipts,
        },
        intents: wire.intents,
        files: wire.files,
        configurations,
    };
    validate::state(&state).context("validate integrated native state")?;
    Ok(state)
}

fn decoded_wire(json: &str) -> Result<Wire> {
    let wire: Wire = decode(json).context("decode integrated native state")?;
    ensure!(wire.schema == 4, "native state requires schema 4");
    ensure!(
        wire.file_policy == super::super::SKILL_FILE_POLICY,
        "unsupported native skill file policy"
    );
    validate::context(&wire.context)?;
    Ok(wire)
}
fn state_wire(json: &str, expected: &NativeStateContext) -> Result<Wire> {
    let wire = decoded_wire(json)?;
    validate::context(expected)?;
    ensure!(
        wire.context == *expected,
        "native state belongs to another root or private namespace"
    );
    Ok(wire)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn declared_context(json: &str) -> Result<NativeStateContext> {
    Ok(decoded_wire(json)?.context)
}

/// Validate only the envelope context and reference declarations before lookup.
/// Full graph/configuration validation still requires the referenced evidence.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn evidence_references(
    json: &str,
    expected: &NativeStateContext,
) -> Result<Vec<EvidenceReference>> {
    let wire = state_wire(json, expected)?;
    let mut references = std::collections::BTreeMap::new();
    for (reference, kind) in wire
        .receipts
        .iter()
        .map(|r| (&r.prestate, EvidenceKind::NativePrestate))
        .chain(
            wire.configurations
                .iter()
                .map(|c| (&c.snapshot, EvidenceKind::ConfigurationSnapshot)),
        )
    {
        ensure!(
            reference.kind == kind,
            "private evidence reference kind differs from its field"
        );
        evidence::validate_reference(reference)?;
        let key = (reference.kind.clone(), reference.digest.clone());
        if let Some(previous) = references.insert(key, reference.clone()) {
            ensure!(
                previous == *reference,
                "conflicting private evidence reference lengths"
            );
        }
    }
    Ok(references.into_values().collect())
}
