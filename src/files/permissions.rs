//! Pure permission selection from supplied values, without filesystem effects.
//!
//! Selected bits confer no ownership, metadata, physical qualification or execution authority.

use anyhow::{Result, bail};

/// Supplied destination mode evidence; unknown presence is distinct from absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationMode {
    Absent,
    Present(Option<u32>),
}

/// Permission evidence associated with the output source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionSource {
    Generated,
    Resource { captured_mode: Option<u32> },
}

/// Explicit selection policy for new resources only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceExecutionPolicy {
    Disabled,
    OwnerExecutable,
}

fn validate_mode(mode: u32, context: &str) -> Result<()> {
    if mode > 0o7777 {
        bail!("{context} mode must contain permission bits only (0..=0o7777)");
    }
    Ok(())
}

/// Selects explicit permission bits without reading umask or applying file changes.
///
/// Preserves ordinary existing modes exactly. New files are private; an explicit
/// resource policy can select owner execution from known owner-execute evidence.
///
/// # Errors
///
/// Rejects out-of-range evidence, unknown existing modes, existing special bits
/// and unknown resource modes when new owner execution is requested.
pub fn select_file_mode(
    destination: DestinationMode,
    source: PermissionSource,
    resource_execution: ResourceExecutionPolicy,
) -> Result<u32> {
    if let DestinationMode::Present(Some(mode)) = destination {
        validate_mode(mode, "destination")?;
    }
    if let PermissionSource::Resource {
        captured_mode: Some(mode),
    } = source
    {
        validate_mode(mode, "captured resource")?;
    }
    match destination {
        DestinationMode::Present(None) => bail!("existing destination mode is unknown"),
        DestinationMode::Present(Some(mode)) => {
            if mode & 0o7000 != 0 {
                bail!("destination special permission bits are unsupported for application");
            }
            return Ok(mode);
        }
        DestinationMode::Absent => {}
    }
    match (source, resource_execution) {
        (
            PermissionSource::Resource { captured_mode },
            ResourceExecutionPolicy::OwnerExecutable,
        ) => {
            let mode = captured_mode.ok_or_else(|| {
                anyhow::anyhow!("resource owner-execute policy requires captured mode")
            })?;
            Ok(if mode & 0o100 != 0 { 0o700 } else { 0o600 })
        }
        _ => Ok(0o600),
    }
}
