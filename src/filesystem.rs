//! Qualified filesystem capabilities for future owned lifecycle effects.
//!
//! Scope locking and source/destination observation activate no command or writes.
//! Observations supply no authority for future filesystem effects.

mod destinations;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod qualification;
mod scope;
mod sources;
pub use destinations::DestinationSnapshot;
pub use sources::SourceSnapshot;

pub use scope::{QualifiedScope, ScopeWriterGuard};
