//! Opt-in owned-content lifecycle foundations, separate from production commands.
pub mod desired;
pub mod state;

pub mod reconcile;

pub mod retire;

pub mod selection;

pub mod upsert;

pub mod application;
pub mod contracts;

mod sharing;

pub mod inspection;

pub mod preview;

pub mod sources;

pub mod commands;

pub mod orchestration;

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod acceptance_tests;

pub mod preflight;
