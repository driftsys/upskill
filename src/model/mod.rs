//! Item models per the format spec — see `docs/format-spec.md`.

pub mod agent;
pub mod bundle;
pub mod command;
pub mod common;
pub mod hook;
pub mod requires;
pub mod rule;
pub mod skill;

pub use agent::{Agent, Mode, ToolCap};
pub use bundle::{
    Bundle, BundleItems, ClaudePluginDescriptor, CopilotPluginDescriptor, McpEntry, McpLocal,
    McpRemote, McpTransport, OpencodePluginDescriptor, PackOnlyKind, PluginEntry, Requires,
    VscodePluginDescriptor, pack_only_items,
};
pub use command::Command;
pub use common::{Audience, CURRENT_SCHEMA, License, Metadata, SchemaVersion};
pub use hook::{ClientMatcher, Hook, HookEvent, HookSettings, Matcher};
pub use requires::{ItemRequires, RequireRef};
pub use rule::{Rule, Scope};
pub use skill::Skill;
