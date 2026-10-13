//! Qualified filesystem capabilities for future owned lifecycle effects.
//!
//! Scope locking and destination observation activate no command or writes.
//! Observations supply no authority for future filesystem effects.

mod destinations;
mod scope;
pub use destinations::DestinationSnapshot;

pub use scope::{QualifiedScope, ScopeWriterGuard};
