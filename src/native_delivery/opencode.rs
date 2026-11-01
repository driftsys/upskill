//! Inactive OpenCode 2.1.4 file-only source planning; no native observation.
use super::adapter::{NativeActionKind, NativeDeliveryProfile};
use crate::compile::skill_package::SkillPackageBinding;
use crate::lifecycle::native_receipt::{NativeSkillChange, NativeSkillSelection};
use crate::native_config::{
    CapturedConfig, ConfigContribution, ConfigPatch, ConfigSelector, plan_config_contributions,
};
use crate::target::TargetSurface;
use serde_json::Value;
use std::path::Component;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpenCodeSourceError {
    #[error("OpenCode source profile is unsupported; review the exact version and file-only route")]
    UnsupportedProfile,
    #[error("OpenCode source change is missing or belongs to another captured context")]
    InvalidChange,
    #[error("OpenCode source selection does not match the managed absolute revision directory")]
    InvalidSelection,
    #[error("OpenCode source ownership does not match the recorded before selection")]
    OwnershipConflict,
    #[error(
        "OpenCode skills configuration has an unsupported shape; preserve it and review the native contract"
    )]
    UnsupportedConfiguration,
    #[error(
        "OpenCode configuration conflicts with its captured shape, ownership, layers or local overlay policy"
    )]
    ConfigurationConflict,
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
fn source_path<'a>(
    profile: &NativeDeliveryProfile,
    id: &str,
    selection: &'a NativeSkillSelection,
) -> Result<&'a str, OpenCodeSourceError> {
    let SkillPackageBinding::ExplicitSkillSource { directory } = &selection.binding else {
        return Err(OpenCodeSourceError::InvalidSelection);
    };
    let revision = format!(
        ".upskill/exports/opencode/{id}/revisions/{}",
        selection.payload_sha256
    );
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        || !selection
            .native_name
            .strip_prefix("upskill-")
            .is_some_and(digest)
        || !digest(&selection.payload_sha256)
        || !digest(&selection.package_sha256)
        || selection.revision_root != revision
        || !directory.is_absolute()
        || directory
            .components()
            .any(|part| part == Component::ParentDir)
        || directory != &profile.scope.path.join(&revision).join("skills")
    {
        return Err(OpenCodeSourceError::InvalidSelection);
    }
    directory
        .to_str()
        .filter(|value| !value.contains('\0'))
        .ok_or(OpenCodeSourceError::InvalidSelection)
}
fn string_array(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|values| values.iter().all(Value::is_string))
}
fn selector_path(config: &CapturedConfig) -> Result<Vec<String>, OpenCodeSourceError> {
    let value = crate::native_config::captured_json_value(config, &["skills".into()])
        .map_err(|_| OpenCodeSourceError::ConfigurationConflict)?;
    match value {
        None => Ok(vec!["skills".into()]),
        Some(Value::Array(values)) if values.iter().all(Value::is_string) => {
            Ok(vec!["skills".into()])
        }
        Some(Value::Object(values))
            if values.iter().all(|(key, value)| {
                matches!(key.as_str(), "paths" | "urls") && string_array(value)
            }) =>
        {
            Ok(vec!["skills".into(), "paths".into()])
        }
        _ => Err(OpenCodeSourceError::UnsupportedConfiguration),
    }
}
/// Produce one atomic owned-file preview for a batch of selected source changes.
/// New documents use the documented `skills` array; an existing deprecated
/// `skills.paths` object retains its form and unrelated URL entries. No migration
/// or fallback between forms occurs. Mixed inherited forms may reject.
///
/// Captures must contain only approved non-secret configuration. Layer order,
/// tracking/ignore flags and scope identity remain caller-supplied claims until
/// qualified. The preview grants no activation, provider input, quiescence,
/// retention or production admission. The coordinator must validate complete
/// file preconditions and native retention before applying an exposure, then
/// independently observe the effective source. No client command is generated.
pub fn plan_skill_sources(
    profile: &NativeDeliveryProfile,
    config: &CapturedConfig,
    changes: &[NativeSkillChange],
) -> Result<ConfigPatch, OpenCodeSourceError> {
    if profile.surface != TargetSurface::OpenCodeCli
        || profile.client_version != "2.1.4"
        || profile.action != NativeActionKind::FileOnly
    {
        return Err(OpenCodeSourceError::UnsupportedProfile);
    }
    if changes.is_empty()
        || config.surface != TargetSurface::OpenCodeCli
        || config.scope != profile.scope
    {
        return Err(OpenCodeSourceError::InvalidChange);
    }
    let path = selector_path(config)?;
    let mut contributions = Vec::with_capacity(changes.len());
    for change in changes {
        if change.key.surface != TargetSurface::OpenCodeCli
            || change.scope != profile.scope
            || change.client_version != profile.client_version
            || change.platform != profile.platform
            || change.observation_id.is_empty()
            || (change.before.is_none() && change.after.is_none())
        {
            return Err(OpenCodeSourceError::InvalidChange);
        }
        let before = change
            .before
            .as_ref()
            .map(|value| source_path(profile, &change.key.installation_id, value))
            .transpose()?;
        let after = change
            .after
            .as_ref()
            .map(|value| source_path(profile, &change.key.installation_id, value))
            .transpose()?;
        if let (Some(before), Some(after)) = (&change.before, &change.after) {
            if before.native_name != after.native_name {
                return Err(OpenCodeSourceError::InvalidSelection);
            }
        }
        let entries = config
            .ownership
            .as_ref()
            .map(|owned| {
                owned
                    .entries
                    .iter()
                    .filter(|entry| entry.id == change.key.installation_id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let expected = match (before, entries.as_slice()) {
            (None, []) => None,
            (Some(source), [entry])
                if entry.value == Value::String(source.into())
                    && entry.selector
                        == ConfigSelector::ArrayEntry {
                            path: path.clone(),
                            value: Value::String(source.into()),
                        } =>
            {
                Some((*entry).clone())
            }
            _ => return Err(OpenCodeSourceError::OwnershipConflict),
        };
        let selected = before.or(after).ok_or(OpenCodeSourceError::InvalidChange)?;
        contributions.push(ConfigContribution {
            id: change.key.installation_id.clone(),
            selector: ConfigSelector::ArrayEntry {
                path: path.clone(),
                value: Value::String(selected.into()),
            },
            expected,
            desired: after.map(|value| Value::String(value.into())),
        });
    }
    plan_config_contributions(config, &contributions)
        .map_err(|_| OpenCodeSourceError::ConfigurationConflict)
}
