//! Stable identities for coding-client configuration surfaces.
//!
//! A family can have more than one surface with different discovery and
//! configuration behavior. A surface in this module is not necessarily
//! selectable or implemented by an adapter.

use std::str::FromStr;

use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetFamily {
    Claude,
    Copilot,
    Codex,
    OpenCode,
    Cursor,
    Gemini,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetSurface {
    ClaudeCli,
    CopilotCli,
    CopilotVscodeLocal,
    CodexCli,
    CodexIde,
    OpenCodeCli,
}

impl TargetSurface {
    pub const ALL: [Self; 6] = [
        Self::ClaudeCli,
        Self::CopilotCli,
        Self::CopilotVscodeLocal,
        Self::CodexCli,
        Self::CodexIde,
        Self::OpenCodeCli,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::ClaudeCli => "claude.cli",
            Self::CopilotCli => "copilot.cli",
            Self::CopilotVscodeLocal => "copilot.vscode-local",
            Self::CodexCli => "codex.cli",
            Self::CodexIde => "codex.ide",
            Self::OpenCodeCli => "opencode.cli",
        }
    }

    pub const fn family(self) -> TargetFamily {
        match self {
            Self::ClaudeCli => TargetFamily::Claude,
            Self::CopilotCli | Self::CopilotVscodeLocal => TargetFamily::Copilot,
            Self::CodexCli | Self::CodexIde => TargetFamily::Codex,
            Self::OpenCodeCli => TargetFamily::OpenCode,
        }
    }

    pub const fn profile(self) -> TargetProfile {
        TargetProfile {
            family: self.family(),
            surface: self,
            contract_revision: 1,
        }
    }
}

impl FromStr for TargetSurface {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|surface| surface.name() == s)
            .ok_or_else(|| anyhow::anyhow!("unknown target surface: {s}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetProfile {
    pub family: TargetFamily,
    pub surface: TargetSurface,
    pub contract_revision: u32,
}

/// The customization category whose native behavior is being checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityCategory {
    Instructions,
    Skills,
    Subagents,
    Mcp,
    Plugins,
    Hooks,
    Permissions,
}

/// A specific native field or operation within a category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityKey {
    pub category: CapabilityCategory,
    pub name: &'static str,
}

impl CapabilityKey {
    pub const fn new(category: CapabilityCategory, name: &'static str) -> Self {
        Self { category, name }
    }

    /// Only reviewed presentation fields may degrade to a warning. New
    /// fields remain behavior-bearing until this contract explicitly adds them.
    fn may_omit(self) -> bool {
        self.category == CapabilityCategory::Subagents && self.name == "color"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Representation {
    Native,
    Adapted,
    NativeUnsupported,
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityDeclaration {
    pub surface: TargetSurface,
    pub key: CapabilityKey,
    pub representation: Representation,
    pub requirement: Requirement,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    RequiredBehavior,
    OptionalMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityDecision {
    Supported,
    Warning(CapabilityWarning),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityWarning {
    pub surface: TargetSurface,
    pub key: CapabilityKey,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{surface} cannot preserve {key:?}: {reason}", surface = .surface.name())]
pub struct CapabilityError {
    pub surface: TargetSurface,
    pub key: CapabilityKey,
    pub reason: &'static str,
}

impl TargetProfile {
    pub fn new(
        family: TargetFamily,
        surface: TargetSurface,
        contract_revision: u32,
    ) -> Result<Self> {
        if family != surface.family() {
            bail!(
                "surface {} does not belong to family {family:?}",
                surface.name()
            );
        }
        if contract_revision == 0 {
            bail!("target contract revision must be at least 1");
        }
        Ok(Self {
            family,
            surface,
            contract_revision,
        })
    }

    pub fn validate_capability(
        self,
        key: CapabilityKey,
        declarations: &[CapabilityDeclaration],
    ) -> std::result::Result<CapabilityDecision, CapabilityError> {
        let mut matching = declarations
            .iter()
            .filter(|entry| entry.surface == self.surface && entry.key == key);
        let first = matching.next();
        if matching.next().is_some() {
            return Err(CapabilityError {
                surface: self.surface,
                key,
                reason: "duplicate capability declarations",
            });
        }
        let Some(entry) = first else {
            return Err(CapabilityError {
                surface: self.surface,
                key,
                reason: "capability is not declared",
            });
        };
        if entry.requirement == Requirement::OptionalMetadata && !key.may_omit() {
            return Err(CapabilityError {
                surface: self.surface,
                key,
                reason: "capability is not classified as optional metadata",
            });
        }
        if entry.representation == Representation::Native {
            return Ok(CapabilityDecision::Supported);
        }
        match entry.requirement {
            Requirement::RequiredBehavior => Err(CapabilityError {
                surface: self.surface,
                key,
                reason: entry.reason,
            }),
            Requirement::OptionalMetadata => Ok(CapabilityDecision::Warning(CapabilityWarning {
                surface: self.surface,
                key,
                reason: entry.reason,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_identifiers_match_contract() {
        let cases = [
            (TargetSurface::ClaudeCli, "claude.cli", TargetFamily::Claude),
            (
                TargetSurface::CopilotCli,
                "copilot.cli",
                TargetFamily::Copilot,
            ),
            (
                TargetSurface::CopilotVscodeLocal,
                "copilot.vscode-local",
                TargetFamily::Copilot,
            ),
            (TargetSurface::CodexCli, "codex.cli", TargetFamily::Codex),
            (TargetSurface::CodexIde, "codex.ide", TargetFamily::Codex),
            (
                TargetSurface::OpenCodeCli,
                "opencode.cli",
                TargetFamily::OpenCode,
            ),
        ];
        assert_eq!(TargetSurface::ALL.len(), cases.len());
        for (surface, name, family) in cases {
            assert_eq!(surface.name(), name);
            assert_eq!(name.parse::<TargetSurface>().unwrap(), surface);
            let profile = surface.profile();
            assert_eq!(surface.family(), family);
            assert_eq!(profile.family, family);
            assert_eq!(profile.surface, surface);
            assert_eq!(profile.contract_revision, 1);
        }
        assert!("cursor.editor".parse::<TargetSurface>().is_err());
    }

    #[test]
    fn profile_rejects_cross_family_and_zero_revision() {
        assert!(TargetProfile::new(TargetFamily::Claude, TargetSurface::CodexCli, 1).is_err());
        assert!(TargetProfile::new(TargetFamily::Codex, TargetSurface::CodexCli, 0).is_err());
        assert_eq!(
            TargetProfile::new(TargetFamily::Codex, TargetSurface::CodexCli, 2).unwrap(),
            TargetProfile {
                family: TargetFamily::Codex,
                surface: TargetSurface::CodexCli,
                contract_revision: 2,
            }
        );
    }

    #[test]
    fn required_capability_never_silently_degrades() {
        let profile = TargetSurface::CodexCli.profile();
        let key = CapabilityKey::new(CapabilityCategory::Subagents, "tools.restrict");
        for (representation, reason) in [
            (Representation::Adapted, "changes the tool set"),
            (
                Representation::NativeUnsupported,
                "client has no matching control",
            ),
            (Representation::Deferred, "adapter work is pending"),
        ] {
            let decision = profile.validate_capability(
                key,
                &[CapabilityDeclaration {
                    surface: TargetSurface::CodexCli,
                    key,
                    representation,
                    requirement: Requirement::RequiredBehavior,
                    reason,
                }],
            );
            let error = decision.expect_err("required restriction must fail closed");
            let message = error.to_string();
            assert!(message.contains("codex.cli"), "{message}");
            assert!(message.contains("tools.restrict"), "{message}");
            assert!(message.contains(reason), "{message}");
        }
        assert!(profile.validate_capability(key, &[]).is_err());
        let duplicate = CapabilityDeclaration {
            surface: TargetSurface::CodexCli,
            key,
            representation: Representation::Native,
            requirement: Requirement::RequiredBehavior,
            reason: "duplicate",
        };
        assert!(
            profile
                .validate_capability(key, &[duplicate, duplicate])
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );
        let misclassified = CapabilityDeclaration {
            surface: TargetSurface::CodexCli,
            key,
            representation: Representation::NativeUnsupported,
            requirement: Requirement::OptionalMetadata,
            reason: "missing tool restriction",
        };
        assert!(profile.validate_capability(key, &[misclassified]).is_err());
        let unrelated = CapabilityDeclaration {
            key: CapabilityKey::new(CapabilityCategory::Subagents, "tools.allow"),
            ..duplicate
        };
        assert!(profile.validate_capability(key, &[unrelated]).is_err());
        let wrong_category = CapabilityDeclaration {
            key: CapabilityKey::new(CapabilityCategory::Permissions, "tools.restrict"),
            ..duplicate
        };
        assert!(profile.validate_capability(key, &[wrong_category]).is_err());
        assert_eq!(
            profile
                .validate_capability(key, &[unrelated, duplicate])
                .unwrap(),
            CapabilityDecision::Supported
        );
    }

    #[test]
    fn exact_native_support_passes_and_cosmetic_omission_warns() {
        let profile = TargetSurface::ClaudeCli.profile();
        let required = CapabilityKey::new(CapabilityCategory::Skills, "name");
        let native = CapabilityDeclaration {
            surface: TargetSurface::ClaudeCli,
            key: required,
            representation: Representation::Native,
            requirement: Requirement::RequiredBehavior,
            reason: "native skill name",
        };
        assert_eq!(
            profile.validate_capability(required, &[native]).unwrap(),
            CapabilityDecision::Supported
        );

        let cosmetic = CapabilityKey::new(CapabilityCategory::Subagents, "color");
        assert!(
            profile.validate_capability(cosmetic, &[]).is_err(),
            "undeclared fields must not be assumed cosmetic"
        );
        let warning = profile
            .validate_capability(
                cosmetic,
                &[CapabilityDeclaration {
                    surface: TargetSurface::ClaudeCli,
                    key: cosmetic,
                    representation: Representation::NativeUnsupported,
                    requirement: Requirement::OptionalMetadata,
                    reason: "cosmetic field unavailable",
                }],
            )
            .unwrap();
        assert_eq!(
            warning,
            CapabilityDecision::Warning(CapabilityWarning {
                surface: TargetSurface::ClaudeCli,
                key: cosmetic,
                reason: "cosmetic field unavailable",
            })
        );
        let other_surface = CapabilityDeclaration {
            surface: TargetSurface::CodexCli,
            ..native
        };
        assert!(
            profile
                .validate_capability(required, &[other_surface])
                .is_err(),
            "another surface's declaration must not authorize this one"
        );
        let sibling = TargetSurface::CodexIde.profile();
        let sibling_key = CapabilityKey::new(CapabilityCategory::Subagents, "tools.restrict");
        let cli_declaration = CapabilityDeclaration {
            surface: TargetSurface::CodexCli,
            key: sibling_key,
            representation: Representation::Native,
            requirement: Requirement::RequiredBehavior,
            reason: "CLI only",
        };
        assert!(
            sibling
                .validate_capability(sibling_key, &[cli_declaration])
                .is_err()
        );
        let ide_declaration = CapabilityDeclaration {
            surface: TargetSurface::CodexIde,
            representation: Representation::NativeUnsupported,
            reason: "IDE has no tool restriction",
            ..cli_declaration
        };
        assert_eq!(
            TargetSurface::CodexCli
                .profile()
                .validate_capability(sibling_key, &[ide_declaration, cli_declaration])
                .unwrap(),
            CapabilityDecision::Supported
        );
        assert!(
            sibling
                .validate_capability(sibling_key, &[ide_declaration, cli_declaration])
                .is_err()
        );
    }
}
