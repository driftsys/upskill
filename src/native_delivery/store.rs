//! Qualified private evidence and integrated state capture/preparation.
//! Public state APIs publish no operation intent, native effects or authoritative state.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use crate::filesystem::native_evidence::NativeEvidenceStore;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod state;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use state::NativeStateStore;
