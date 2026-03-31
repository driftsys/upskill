use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const LOCKFILE_NAME: &str = ".upskill-lock.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lockfile {
    pub skills: Vec<LockedSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockedSkill {
    pub name: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

impl Lockfile {
    pub fn new() -> Self {
        Self { skills: Vec::new() }
    }

    /// Load from the lockfile next to the given canonical target, or return empty.
    pub fn load(project_root: &Path) -> Self {
        let path = lockfile_path(project_root);
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }

    /// Add or update a skill entry. Replaces existing entry with the same name.
    pub fn upsert(&mut self, skill: LockedSkill) {
        self.skills.retain(|s| s.name != skill.name);
        self.skills.push(skill);
        self.skills.sort();
    }

    /// Remove a skill by name.
    pub fn remove(&mut self, name: &str) {
        self.skills.retain(|s| s.name != name);
    }

    /// Write the lockfile to disk.
    pub fn save(&self, project_root: &Path) -> Result<(), String> {
        let path = lockfile_path(project_root);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize lockfile: {}", e))?;
        std::fs::write(&path, format!("{}\n", json))
            .map_err(|e| format!("failed to write {}: {}", path.display(), e))
    }
}

fn lockfile_path(project_root: &Path) -> PathBuf {
    project_root.join(LOCKFILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_adds_new_skill() {
        let mut lock = Lockfile::new();
        lock.upsert(LockedSkill {
            name: "lint".into(),
            source: "github:ms/skills".into(),
            git_ref: None,
        });
        assert_eq!(lock.skills.len(), 1);
        assert_eq!(lock.skills[0].name, "lint");
    }

    #[test]
    fn upsert_replaces_existing_skill() {
        let mut lock = Lockfile::new();
        lock.upsert(LockedSkill {
            name: "lint".into(),
            source: "github:ms/skills".into(),
            git_ref: None,
        });
        lock.upsert(LockedSkill {
            name: "lint".into(),
            source: "github:ms/skills@v2".into(),
            git_ref: Some("v2".into()),
        });
        assert_eq!(lock.skills.len(), 1);
        assert_eq!(lock.skills[0].git_ref, Some("v2".into()));
    }

    #[test]
    fn remove_deletes_skill() {
        let mut lock = Lockfile::new();
        lock.upsert(LockedSkill {
            name: "a".into(),
            source: "s".into(),
            git_ref: None,
        });
        lock.upsert(LockedSkill {
            name: "b".into(),
            source: "s".into(),
            git_ref: None,
        });
        lock.remove("a");
        assert_eq!(lock.skills.len(), 1);
        assert_eq!(lock.skills[0].name, "b");
    }

    #[test]
    fn skills_are_sorted() {
        let mut lock = Lockfile::new();
        lock.upsert(LockedSkill {
            name: "z".into(),
            source: "s".into(),
            git_ref: None,
        });
        lock.upsert(LockedSkill {
            name: "a".into(),
            source: "s".into(),
            git_ref: None,
        });
        let names: Vec<_> = lock.skills.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["a", "z"]);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut lock = Lockfile::new();
        lock.upsert(LockedSkill {
            name: "lint".into(),
            source: "github:ms/skills@v1".into(),
            git_ref: Some("v1".into()),
        });
        lock.save(tmp.path()).expect("save");

        let loaded = Lockfile::load(tmp.path());
        assert_eq!(loaded, lock);
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lock = Lockfile::load(tmp.path());
        assert_eq!(lock.skills.len(), 0);
    }
}
