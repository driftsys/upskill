//! Portable candidate validation and conservative batch topology.
use crate::files::plan::FileLocation;
use crate::ownership::validate_relative_path;
use anyhow::{Result, bail};

use crate::filesystem::paths_may_overlap as may_overlap;
pub(super) fn prepare(locations: &[FileLocation]) -> Result<Vec<FileLocation>> {
    for location in locations {
        validate_relative_path(&location.path)?;
    }
    for (index, left) in locations.iter().enumerate() {
        for right in &locations[index + 1..] {
            if may_overlap(&left.path, &right.path) {
                bail!(
                    "potentially overlapping destinations: {} and {}",
                    left.path,
                    right.path
                );
            }
        }
    }
    let mut locations = locations.to_vec();
    locations.sort_by_key(|location| {
        (
            location.root.scope,
            location.root.path.as_os_str().to_owned(),
            location.path.clone(),
        )
    });
    Ok(locations)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_location_tuple_determines_order() {
        use crate::ownership::{InstallationRoot, OwnershipScope};
        let location = |scope, root: &str, path: &str| FileLocation {
            root: InstallationRoot {
                scope,
                path: root.into(),
            },
            path: path.into(),
        };
        let expected = vec![
            location(OwnershipScope::Project, "/a", "y"),
            location(OwnershipScope::Project, "/a", "z"),
            location(OwnershipScope::Project, "/z", "a"),
            location(OwnershipScope::User, "/a", "b"),
        ];
        let mut input = expected.clone();
        input.reverse();
        assert_eq!(prepare(&input).unwrap(), expected);
    }
}
