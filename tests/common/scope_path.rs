//! Host-native absolute roots for pure planner fixtures; no filesystem access.
use std::path::PathBuf;

pub fn absolute(posix: &str) -> PathBuf {
    assert!(posix.starts_with('/'), "fixture spelling must be absolute");
    #[cfg(windows)]
    {
        let mut path = PathBuf::from(r"C:\");
        for component in posix.split('/').filter(|part| !part.is_empty()) {
            assert!(!matches!(component, "." | ".."));
            path.push(component);
        }
        path
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(posix)
    }
}

#[test]
fn scope_fixture_roots_are_absolute_and_already_normalized() {
    for spelling in ["/workspace", "/workspace/project", "/other/checkout"] {
        let path = absolute(spelling);
        assert!(path.is_absolute());
        let normalized: PathBuf = path.components().collect();
        assert_eq!(normalized.as_os_str(), path.as_os_str());
    }
}
