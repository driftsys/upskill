//! Durable initial-state evidence carried by the authoritative combined state.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::lockfile::Lockfile;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum StateOrigin {
    Fresh {},
    Migrated {
        backup_sha256: String,
        backup_length: u64,
    },
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl StateOrigin {
    /// Bind exact observed schema-1 bytes, including unknown JSON fields and whitespace.
    /// This pure value alone establishes no filesystem provenance or write authority.
    pub fn from_schema1_bytes(bytes: &[u8]) -> Result<Self> {
        let state: Lockfile =
            serde_json::from_slice(bytes).context("parse original schema-1 state")?;
        ensure!(state.schema == 1, "origin requires schema-1 state");
        Ok(Self::Migrated {
            backup_sha256: digest(bytes),
            backup_length: bytes.len() as u64,
        })
    }

    pub(crate) fn matches_backup(&self, bytes: &[u8]) -> bool {
        match self {
            Self::Fresh {} => false,
            Self::Migrated {
                backup_sha256,
                backup_length,
            } => *backup_length == bytes.len() as u64 && *backup_sha256 == digest(bytes),
        }
    }

    pub(super) fn validate(&self, legacy: Option<&Lockfile>) -> Result<()> {
        match self {
            Self::Fresh {} => ensure!(
                legacy.is_none(),
                "fresh origin cannot contain legacy history"
            ),
            Self::Migrated {
                backup_sha256,
                backup_length,
            } => {
                ensure!(
                    legacy.is_some(),
                    "migrated origin requires retained legacy history"
                );
                ensure!(
                    *backup_length > 0
                        && backup_sha256.len() == 64
                        && backup_sha256
                            .bytes()
                            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f')),
                    "invalid original backup binding"
                );
            }
        }
        Ok(())
    }
}
