//! Non-secret structural summaries for the candidate Copilot contract collector.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CopilotInventoryKind {
    Plugins,
    Marketplaces,
}

/// Counts of supplied metadata only, never native absence or binding evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CopilotInventorySummary {
    kind: CopilotInventoryKind,
    entries: usize,
}
impl CopilotInventorySummary {
    pub fn entries(&self) -> usize {
        self.entries
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CopilotProbeError {
    #[error("Copilot contract inventory version is unsupported")]
    UnsupportedVersion,
    #[error("Copilot contract inventory exceeds the bounded capture limit")]
    TooLarge,
    #[error("Copilot contract inventory is malformed")]
    Malformed,
    #[error("Copilot contract inventory has an unsupported shape")]
    UnsupportedShape,
}

/// Validate the observed 1.0.105 field types and discard every native value.
/// This performs no native call, filesystem operation or profile admission.
pub fn summarize_inventory(
    version: &str,
    kind: CopilotInventoryKind,
    bytes: &[u8],
) -> Result<CopilotInventorySummary, CopilotProbeError> {
    if version != "1.0.105" {
        return Err(CopilotProbeError::UnsupportedVersion);
    }
    if bytes.len() > 1024 * 1024 {
        return Err(CopilotProbeError::TooLarge);
    }
    let source = std::str::from_utf8(bytes).map_err(|_| CopilotProbeError::Malformed)?;
    let value =
        crate::parse::json::parse_unique(source).map_err(|_| CopilotProbeError::Malformed)?;
    let rows = value
        .as_array()
        .ok_or(CopilotProbeError::UnsupportedShape)?;
    let (strings, boolean): (&[&str], &str) = match kind {
        CopilotInventoryKind::Plugins => (&["name", "marketplace", "source", "version"], "enabled"),
        CopilotInventoryKind::Marketplaces => (&["name", "source"], "isDefault"),
    };
    for row in rows {
        let fields = row.as_object().ok_or(CopilotProbeError::UnsupportedShape)?;
        if fields.len() != strings.len() + 1
            || !strings
                .iter()
                .all(|key| fields.get(*key).is_some_and(serde_json::Value::is_string))
            || !fields
                .get(boolean)
                .is_some_and(serde_json::Value::is_boolean)
        {
            return Err(CopilotProbeError::UnsupportedShape);
        }
    }
    Ok(CopilotInventorySummary {
        kind,
        entries: rows.len(),
    })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod preflight;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use preflight::{
    CopilotCollectionSummary, CopilotEvidenceError, CopilotPreflightError, CopilotProbeEvidence,
    CopilotProbePreflight, CopilotProbeRequest,
};
