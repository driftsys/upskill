//! Qualified filesystem observations and opt-in file execution capabilities.
//!
//! Scope locking and observation activate no command or writes. The execution
//! API applies caller-authorized file plans; CLI and ownership integration remain separate.
//! Observations supply no authority for future filesystem effects.

mod destinations;
pub mod execution;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod qualification;
mod scope;
mod sources;
pub use destinations::DestinationSnapshot;
pub use execution::{ExecutionOutcome, ExecutionReport, ExecutionRequest};
pub use sources::SourceSnapshot;

pub use scope::{QualifiedScope, ScopeWriterGuard};

pub(crate) use destinations::candidates::may_overlap as paths_may_overlap;
