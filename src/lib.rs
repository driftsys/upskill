//! # upskill
//!
//! Author and distribute AI-assistance content (rules, skills, agents)
//! across multiple AI coding clients (Claude Code, Copilot, opencode)
//! from a single source of truth. The central abstraction is
//! **generation** — SSOT in, per-client output out.
//!
//! No Node.js. No npm. No async runtime. Single static binary.

pub mod ancillary;
pub mod bundle;
pub mod cli;
pub mod compile;
pub mod config;
pub mod conflict;
pub mod fetch;
pub mod files;
pub mod filesystem;
pub mod fmt;
pub mod generate;
pub mod index;
pub mod instructions;
pub mod legacy;
pub mod lifecycle;
pub mod lint;
pub mod lockfile;
pub mod mcp;
pub mod model;
pub mod operations;
pub mod ownership;
pub mod parse;
pub mod pipeline;
pub mod plugin;
pub mod scaffold;
pub mod search;
pub mod select;
pub mod source;
pub mod source_manifest;
pub mod style;
pub mod target;

pub use source::{InstallSource, home_dir, parse_install_source};
