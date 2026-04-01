//! # upskill
//!
//! Upskill your coding agents.
//!
//! Ultra-lightweight [Agent Skills](https://agentskills.io/) package manager
//! in Rust. Install, list, update, and remove SKILL.md packages across
//! coding agents (Claude Code, Copilot, Codex, Cursor, OpenCode).
//!
//! No Node.js. No npm. Single static binary.
//!
//! ## Status
//!
//! This crate is under active development. v0.1.0 is a name reservation.
//! See the repository for progress.

pub mod agent;
pub mod auth;
pub mod fetch;
pub mod install;
pub mod lockfile;
pub mod search;
pub mod source;
pub mod ui;

pub use source::{InstallSource, parse_install_source};
