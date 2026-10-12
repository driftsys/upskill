//! Qualified filesystem capabilities for future owned lifecycle effects.
//!
//! Scope qualification and advisory locking activate no command or writes.
//! Descendant containment and action preconditions require separate validation.

mod scope;

pub use scope::{QualifiedScope, ScopeWriterGuard};
