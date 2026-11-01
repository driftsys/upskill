//! Qualified private evidence only; no operation journal or state publication.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use crate::filesystem::native_evidence::NativeEvidenceStore;
