//! Deterministic reporting for safely observed selected lifecycle requests.
use super::orchestration::SourceInstructionReaders;
use crate::files::plan::FileLocation;

/// Diagnostic stages in stable presentation order, independent of error wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PreflightCategory {
    Binding,
    Resolution,
    Capture,
    Compilation,
    Capability,
    Destination,
    Ownership,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightDiagnostic {
    pub installation_ids: Vec<String>,
    pub category: PreflightCategory,
    pub location: Option<FileLocation>,
    pub message: String,
}

/// Observations grant neither execution authority nor incidental ownership.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreflightReport {
    pub diagnostics: Vec<PreflightDiagnostic>,
    pub instruction_readers: Vec<SourceInstructionReaders>,
}

fn location_key(
    location: &Option<FileLocation>,
) -> Option<(crate::ownership::OwnershipScope, &std::ffi::OsStr, &str)> {
    location.as_ref().map(|value| {
        (
            value.root.scope,
            value.root.path.as_os_str(),
            value.path.as_str(),
        )
    })
}

pub fn normalize_report(report: &mut PreflightReport) {
    for diagnostic in &mut report.diagnostics {
        diagnostic.installation_ids.sort();
        diagnostic.installation_ids.dedup();
    }
    report.diagnostics.sort_by(|a, b| {
        a.installation_ids
            .cmp(&b.installation_ids)
            .then(a.category.cmp(&b.category))
            .then(location_key(&a.location).cmp(&location_key(&b.location)))
            .then(a.message.cmp(&b.message))
    });
    report.diagnostics.dedup_by(|a, b| {
        a.installation_ids == b.installation_ids
            && a.category == b.category
            && location_key(&a.location) == location_key(&b.location)
            && a.message == b.message
    });
    report
        .instruction_readers
        .sort_by(|a, b| a.installation_id.cmp(&b.installation_id));
}

mod conflicts;
pub use conflicts::collect_conflicts;
pub(crate) use conflicts::{artifact_location, locations};
