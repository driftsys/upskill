//! Item-level `requires` (§3.7) — directed dependencies declared per
//! entrypoint. Distinct from the bundle-level `Requires` (which pins a
//! semver constraint); item requires resolve by `(kind, name)`.

use serde::{Deserialize, Serialize};

/// One `requires` reference. A bare string targets the same source by
/// name; a `{ name, source }` map targets another source (the `source`
/// uses the `upskill add` source DSL). Cross-source *resolution* ships in
/// a later release; this type captures the stable on-disk form now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequireRef {
    Name(String),
    Detailed { name: String, source: String },
}

impl RequireRef {
    pub fn name(&self) -> &str {
        match self {
            RequireRef::Name(n) => n,
            RequireRef::Detailed { name, .. } => name,
        }
    }
    /// The cross-source locator, if this is a `{ name, source }` entry.
    pub fn source(&self) -> Option<&str> {
        match self {
            RequireRef::Name(_) => None,
            RequireRef::Detailed { source, .. } => Some(source),
        }
    }
}

/// `requires:` block — mirrors the bundle `items` vocabulary.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemRequires {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RequireRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<RequireRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<RequireRef>,
}

impl ItemRequires {
    /// True when no dependency is declared. Used by `skip_serializing_if`.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty() && self.skills.is_empty() && self.agents.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_string_and_map_entries() {
        let yaml = "rules: [security-baseline]\nskills: [{ name: sarif, source: org/repo }]\n";
        let req: ItemRequires = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(
            req.rules,
            vec![RequireRef::Name("security-baseline".into())]
        );
        assert_eq!(req.skills[0].name(), "sarif");
        assert_eq!(req.skills[0].source(), Some("org/repo"));
        assert!(req.agents.is_empty());
    }

    #[test]
    fn empty_when_no_kinds() {
        assert!(ItemRequires::default().is_empty());
    }
}
