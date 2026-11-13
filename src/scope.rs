//! The two install scopes and the root directory of each.

use std::path::PathBuf;

/// Where a command reads and writes: the current project or the user's home.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    #[default]
    Project,
    Global,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Scope::Project => "project",
            Scope::Global => "global",
        })
    }
}

impl Scope {
    /// The other scope.
    pub fn other(self) -> Scope {
        match self {
            Scope::Project => Scope::Global,
            Scope::Global => Scope::Project,
        }
    }
}

/// A scope together with the directory that holds its lockfile and outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeRoot {
    pub scope: Scope,
    pub root: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_scope() {
        assert_eq!(Scope::Project.to_string(), "project");
        assert_eq!(Scope::Global.to_string(), "global");
    }

    #[test]
    fn other_flips_the_scope() {
        assert_eq!(Scope::Project.other(), Scope::Global);
        assert_eq!(Scope::Global.other(), Scope::Project);
    }

    #[test]
    fn default_is_project() {
        assert_eq!(Scope::default(), Scope::Project);
    }
}
