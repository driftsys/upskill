//! Item models per the format spec — see `docs/format-spec.md`.

pub mod agent;
pub mod bundle;
pub mod common;
pub mod rule;
pub mod skill;

pub use agent::{Agent, Mode, ToolCap};
pub use bundle::{
    Bundle, BundleItems, ClaudePluginDescriptor, CopilotPluginDescriptor, McpEntry, McpLocal,
    McpRemote, McpTransport, OpencodePluginDescriptor, PluginEntry, Requires,
    VscodePluginDescriptor,
};
pub use common::{Audience, CURRENT_SCHEMA, License, Metadata, SchemaVersion};
pub use rule::{Rule, Scope};
pub use skill::Skill;
