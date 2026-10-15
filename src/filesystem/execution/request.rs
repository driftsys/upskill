use super::ExecutionRequest;
use anyhow::Result;

pub(super) fn validate(request: &ExecutionRequest) -> Result<()> {
    use crate::ownership::validate_relative_path;
    use anyhow::ensure;
    let root = &request.state.location.root;
    ensure!(
        root.path.is_absolute()
            && !root
                .path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir)),
        "execution root must be absolute without parent components"
    );
    ensure!(
        request.state.after.is_some(),
        "final State action must remain present"
    );
    let actions: Vec<_> = request
        .files
        .iter()
        .chain(std::iter::once(&request.state))
        .collect();
    for (index, action) in actions.iter().enumerate() {
        ensure!(
            &action.location.root == root,
            "execution actions must share one exact scope root"
        );
        let path = &action.location.path;
        validate_relative_path(path)?;
        ensure!(
            path != ".upskill" && path != ".upskill/state" && !path.starts_with(".upskill/state/"),
            "execution destination overlaps reserved journal paths"
        );
        for other in &actions[..index] {
            let prior = &other.location.path;
            ensure!(
                path != prior
                    && !path.starts_with(&format!("{prior}/"))
                    && !prior.starts_with(&format!("{path}/")),
                "duplicate or ancestor execution destinations"
            );
        }
        for value in [&action.before, &action.after].into_iter().flatten() {
            let mode = value
                .mode
                .ok_or_else(|| anyhow::anyhow!("execution requires known permission modes"))?;
            ensure!(
                mode <= 0o777,
                "execution rejects special or invalid permission bits"
            );
        }
        if let Some(after) = &action.after {
            if let Some(before) = &action.before {
                ensure!(
                    after.mode == before.mode,
                    "execution must preserve existing ordinary mode"
                );
            } else {
                ensure!(
                    matches!(after.mode, Some(0o600 | 0o700)),
                    "new output must use selected private mode"
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::plan::{FileLocation, FileValue, PlannedFile};
    use crate::ownership::{InstallationRoot, OwnershipScope};
    fn action(path: &str) -> PlannedFile {
        PlannedFile {
            location: FileLocation {
                root: InstallationRoot {
                    scope: OwnershipScope::Project,
                    path: std::env::temp_dir().join("execution-root"),
                },
                path: path.into(),
            },
            before: None,
            after: Some(FileValue {
                bytes: b"desired".to_vec(),
                mode: Some(0o600),
            }),
        }
    }
    fn request() -> ExecutionRequest {
        ExecutionRequest {
            files: vec![action("rules/a.md")],
            state: action(".upskill-lock.json"),
        }
    }
    #[test]
    fn rejects_duplicate_or_ancestor_actions() {
        for path in ["rules/a.md", "rules", "rules/a.md/child"] {
            let mut r = request();
            r.files.push(action(path));
            assert!(validate(&r).is_err(), "accepted {path}");
        }
    }
    #[test]
    fn rejects_cross_scope_or_reserved_paths() {
        let mut r = request();
        r.files[0].location.root.path.push("other");
        assert!(validate(&r).is_err());
        for path in [
            ".upskill/state/operation.json",
            ".upskill",
            "../outside",
            "/absolute",
        ] {
            let mut r = request();
            r.files[0].location.path = path.into();
            assert!(validate(&r).is_err(), "accepted {path}");
        }
    }
    #[test]
    fn requires_distinct_present_final_state() {
        let mut r = request();
        r.state.after = None;
        assert!(validate(&r).is_err());
        let mut r = request();
        r.state = r.files[0].clone();
        assert!(validate(&r).is_err());
    }
    #[test]
    fn rejects_unknown_special_or_changed_existing_modes() {
        for mode in [None, Some(0o4755), Some(0o10000), Some(0o644)] {
            let mut r = request();
            r.files[0].after.as_mut().unwrap().mode = mode;
            assert!(validate(&r).is_err(), "accepted {mode:?}");
        }
        let mut r = request();
        r.files[0].before = Some(FileValue {
            bytes: vec![],
            mode: Some(0o644),
        });
        assert!(validate(&r).is_err());
    }
    #[test]
    fn retains_noop_removal_and_state_only_requests() {
        let mut r = request();
        r.files[0].before = r.files[0].after.clone();
        validate(&r).unwrap();
        r.files[0].after = None;
        validate(&r).unwrap();
        r.files.clear();
        validate(&r).unwrap();
    }
}
