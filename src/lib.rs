//! # upskill
//!
//! Author and distribute AI-assistance content (rules, skills, agents)
//! across multiple AI coding clients (Claude Code, Copilot, opencode)
//! from a single source of truth. The central abstraction is
//! **generation** — SSOT in, per-client output out.
//!
//! No Node.js. No npm. No async runtime. Single static binary.

pub mod ancillary;
pub mod auth;
pub mod bundle;
pub mod fetch;
pub mod fmt;
pub mod generate;
pub mod lint;
pub mod lockfile;
pub mod model;
pub mod parse;
pub mod pipeline;
pub mod scaffold;
pub mod search;
pub mod source;

pub use source::{InstallSource, parse_install_source};
