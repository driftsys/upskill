//! Portable candidate validation and conservative batch topology.
use crate::files::plan::FileLocation;
use crate::ownership::validate_relative_path;
use anyhow::{Result, bail};

fn may_match(a: &str, b: &str) -> bool {
    !a.is_ascii() || !b.is_ascii() || a.eq_ignore_ascii_case(b)
}
pub(crate) fn may_overlap(a: &str, b: &str) -> bool {
    a.split('/').zip(b.split('/')).all(|(a, b)| may_match(a, b))
}
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
    fn potential_overlap_examples() {
        assert!(may_overlap("a/X", "a/x"));
        assert!(may_overlap("a/é", "a/e"));
        assert!(may_overlap("a/x", "a/x/y"));
        assert!(!may_overlap("a/é", "b/e"));
        assert!(!may_overlap("a/b", "ab/c"));
    }
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
