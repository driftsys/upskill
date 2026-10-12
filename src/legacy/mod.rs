//! Pure descriptions of typed legacy lockfile state, without adoption or I/O.

use crate::lockfile::{LockedBundle, LockedItem, LockedMcp, LockedPlugin, Lockfile};
use crate::pipeline::ItemKind;
use crate::source::{InstallSource, parse_install_source_label};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) mod layout;
pub use layout::LegacyClient;

/// Effective generation clients cannot reconstruct the original requested surfaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestedSelection {
    Unresolved,
}

/// Owned typed state and descriptive facts, without a serialized projection format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyProjection {
    /// Exact typed clone, not original JSON bytes or unknown JSON fields.
    pub snapshot: Lockfile,
    pub items: Vec<LegacyItem>,
    pub bundles: Vec<LegacyBundle>,
    pub integrations: Vec<LegacyIntegration>,
    pub diagnostics: Vec<LegacyDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyItem {
    pub index: usize,
    pub record: LockedItem,
    pub source: Option<InstallSource>,
    pub requested_selection: RequestedSelection,
    pub clients: Vec<LegacyClient>,
    pub candidates: Vec<LegacyCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyBundle {
    pub index: usize,
    pub record: LockedBundle,
    pub source: Option<InstallSource>,
}

/// Plugins followed by MCPs, each indexed within its original vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyIntegration {
    Plugin { index: usize, record: LockedPlugin },
    Mcp { index: usize, record: LockedMcp },
}

/// Unobserved relative layout facts; these do not authorize resource adoption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCandidate {
    pub client: LegacyClient,
    pub entrypoint: String,
    pub resource_search_base: String,
}

/// Collision locations anchor at the first row; their kinds include every index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyDiagnostic {
    pub location: LegacyLocation,
    pub kind: LegacyDiagnosticKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyLocation {
    Global,
    Item(usize),
    Bundle(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyDiagnosticKind {
    UnsupportedSchema {
        schema: u32,
    },
    InvalidSource {
        message: String,
    },
    UnknownClient {
        client: String,
    },
    InvalidInstalledName {
        name: String,
    },
    DuplicateItemIdentity {
        kind: ItemKind,
        name: String,
        indices: Vec<usize>,
    },
    IdenticalCandidateEntrypoint {
        entrypoint: String,
        indices: Vec<usize>,
    },
}

/// Describe schema-1 records without roots, filesystem access or source verification.
///
/// Malformed facts remain inspectable through retained rows and diagnostics.
/// Unsupported schemas retain typed records without interpreting their values.
pub fn project_legacy(lockfile: &Lockfile) -> LegacyProjection {
    let snapshot = lockfile.clone();
    let items: Vec<LegacyItem> = snapshot
        .items
        .iter()
        .enumerate()
        .map(|(index, record)| LegacyItem {
            index,
            record: record.clone(),
            source: None,
            requested_selection: RequestedSelection::Unresolved,
            clients: vec![],
            candidates: vec![],
        })
        .collect();
    let bundles: Vec<LegacyBundle> = snapshot
        .bundles
        .iter()
        .enumerate()
        .map(|(index, record)| LegacyBundle {
            index,
            record: record.clone(),
            source: None,
        })
        .collect();
    let integrations = snapshot
        .plugins
        .iter()
        .enumerate()
        .map(|(index, record)| LegacyIntegration::Plugin {
            index,
            record: record.clone(),
        })
        .chain(
            snapshot
                .mcps
                .iter()
                .enumerate()
                .map(|(index, record)| LegacyIntegration::Mcp {
                    index,
                    record: record.clone(),
                }),
        )
        .collect();
    let mut projection = LegacyProjection {
        snapshot,
        items,
        bundles,
        integrations,
        diagnostics: vec![],
    };
    if projection.snapshot.schema != 1 {
        projection.diagnostics.push(LegacyDiagnostic {
            location: LegacyLocation::Global,
            kind: LegacyDiagnosticKind::UnsupportedSchema {
                schema: projection.snapshot.schema,
            },
        });
        return projection;
    }

    for row in &mut projection.items {
        row.source = decode_source(
            &row.record.source,
            LegacyLocation::Item(row.index),
            &mut projection.diagnostics,
        );
        if row.record.clients.is_empty() {
            row.clients = LegacyClient::ALL.to_vec();
        } else {
            for label in &row.record.clients {
                match LegacyClient::from_label(label) {
                    Some(client) => {
                        if !row.clients.contains(&client) {
                            row.clients.push(client);
                        }
                    }
                    None => projection.diagnostics.push(LegacyDiagnostic {
                        location: LegacyLocation::Item(row.index),
                        kind: LegacyDiagnosticKind::UnknownClient {
                            client: label.clone(),
                        },
                    }),
                }
            }
        }
        if valid_installed_name(&row.record.name) {
            row.candidates = row
                .clients
                .iter()
                .map(|&client| LegacyCandidate {
                    client,
                    entrypoint: portable_path(&layout::output_path(
                        row.record.kind,
                        client,
                        &row.record.name,
                    )),
                    resource_search_base: portable_path(&layout::resource_base_path(
                        row.record.kind,
                        client,
                        &row.record.name,
                    )),
                })
                .collect();
        } else {
            projection.diagnostics.push(LegacyDiagnostic {
                location: LegacyLocation::Item(row.index),
                kind: LegacyDiagnosticKind::InvalidInstalledName {
                    name: row.record.name.clone(),
                },
            });
        }
    }
    for row in &mut projection.bundles {
        row.source = decode_source(
            &row.record.source,
            LegacyLocation::Bundle(row.index),
            &mut projection.diagnostics,
        );
    }
    diagnose_collisions(&projection.items, &mut projection.diagnostics);
    projection
}

fn decode_source(
    label: &str,
    location: LegacyLocation,
    diagnostics: &mut Vec<LegacyDiagnostic>,
) -> Option<InstallSource> {
    match parse_install_source_label(label) {
        Ok(source) => Some(source),
        Err(error) => {
            diagnostics.push(LegacyDiagnostic {
                location,
                kind: LegacyDiagnosticKind::InvalidSource {
                    message: error.to_string(),
                },
            });
            None
        }
    }
}

fn valid_installed_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':') || c.is_control())
}

fn portable_path(path: &Path) -> String {
    path.to_str()
        .expect("historical paths are constructed from UTF-8 strings")
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn diagnose_collisions(items: &[LegacyItem], diagnostics: &mut Vec<LegacyDiagnostic>) {
    let mut identities: BTreeMap<(ItemKind, &str), Vec<usize>> = BTreeMap::new();
    let mut entrypoints: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for row in items {
        identities
            .entry((row.record.kind, &row.record.name))
            .or_default()
            .push(row.index);
        for candidate in &row.candidates {
            let indices = entrypoints.entry(&candidate.entrypoint).or_default();
            if indices.last() != Some(&row.index) {
                indices.push(row.index);
            }
        }
    }
    for ((kind, name), indices) in identities {
        if indices.len() > 1 {
            diagnostics.push(LegacyDiagnostic {
                location: LegacyLocation::Item(indices[0]),
                kind: LegacyDiagnosticKind::DuplicateItemIdentity {
                    kind,
                    name: name.into(),
                    indices,
                },
            });
        }
    }
    for (entrypoint, indices) in entrypoints {
        if indices.len() > 1 {
            diagnostics.push(LegacyDiagnostic {
                location: LegacyLocation::Item(indices[0]),
                kind: LegacyDiagnosticKind::IdenticalCandidateEntrypoint {
                    entrypoint: entrypoint.into(),
                    indices,
                },
            });
        }
    }
}
