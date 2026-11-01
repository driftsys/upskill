use super::model::{
    NativeOperationIntent, NativeOperationJournal, PendingMarker, RootMarkerNamespace,
};
use crate::native_delivery::roots::NativeRootBinding;
use crate::ownership::OwnershipScope;
use anyhow::{Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub(super) fn validate_id(id: &str) -> Result<()> {
    ensure!(
        id.len() == 32
            && id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid native operation ID"
    );
    Ok(())
}
pub(super) fn intent_name(id: &str) -> String {
    format!("intent-{id}.yaml")
}
pub(super) fn journal_name(id: &str) -> String {
    format!("journal-{id}.json")
}
pub(super) fn intent_digest(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"upskill:native-intent:v1\0");
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
pub(super) fn pending(
    intent: &NativeOperationIntent,
    journal: &NativeOperationJournal,
    marker: &RootMarkerNamespace,
) -> PendingMarker {
    PendingMarker {
        schema: 1,
        operation_id: intent.operation_id.clone(),
        root: marker.root.clone(),
        marker_namespace: marker.namespace.clone(),
        private_root: intent.context.private_root.clone(),
        namespace: intent.context.namespace.clone(),
        journal_name: journal_name(&intent.operation_id),
        intent_sha256: journal.intent_sha256.clone(),
    }
}
pub(super) fn validate_marker(
    marker: &PendingMarker,
    root: &NativeRootBinding,
    namespace: &str,
) -> Result<()> {
    validate_id(&marker.operation_id)?;
    crate::compile::skill_package::scope_identity(&marker.private_root)?;
    ensure!(
        marker.schema == 1
            && marker.root == *root
            && marker.marker_namespace == namespace
            && marker.private_root.scope == OwnershipScope::User
            && marker.journal_name == journal_name(&marker.operation_id)
            && valid_digest(&marker.intent_sha256)
            && valid_digest(&marker.namespace),
        "native pending marker binding differs"
    );
    Ok(())
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn json<T: serde::de::DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T> {
    let value = crate::parse::json::parse_unique(std::str::from_utf8(bytes)?)?;
    let typed: T = serde_json::from_value(value.clone())?;
    ensure!(
        serde_json::to_value(&typed)? == value,
        "unknown or noncanonical native journal fields"
    );
    Ok(typed)
}
pub(super) fn yaml<T: serde::de::DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_slice(bytes)?;
    let typed: T = serde_yaml_ng::from_value(value.clone())?;
    ensure!(
        serde_yaml_ng::to_value(&typed)? == value,
        "unknown or noncanonical native intent fields"
    );
    Ok(typed)
}
