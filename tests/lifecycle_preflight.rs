use upskill::files::plan::FileLocation;
use upskill::lifecycle::preflight::{
    PreflightCategory, PreflightDiagnostic, PreflightReport, normalize_report,
};
use upskill::ownership::{InstallationRoot, OwnershipScope};

fn diagnostic(ids: &[&str], category: PreflightCategory, path: &str) -> PreflightDiagnostic {
    PreflightDiagnostic {
        installation_ids: ids.iter().map(|id| (*id).into()).collect(),
        category,
        location: Some(FileLocation {
            root: InstallationRoot {
                scope: OwnershipScope::Project,
                path: "/scope".into(),
            },
            path: path.into(),
        }),
        message: "blocked".into(),
    }
}

// Removing sorting or merging distinct categories/locations breaks this report contract.
#[test]
fn report_order_is_input_independent() {
    let entries = vec![
        diagnostic(&["b", "a", "b"], PreflightCategory::Destination, "two"),
        diagnostic(&["a"], PreflightCategory::Compilation, "one"),
        diagnostic(&["a"], PreflightCategory::Resolution, "one"),
        diagnostic(&["a"], PreflightCategory::Resolution, "other"),
        diagnostic(&["a", "b"], PreflightCategory::Destination, "two"),
    ];
    let mut left = PreflightReport {
        diagnostics: entries.clone(),
        instruction_readers: vec![],
    };
    let mut right = PreflightReport {
        diagnostics: entries.into_iter().rev().collect(),
        instruction_readers: vec![],
    };
    normalize_report(&mut left);
    normalize_report(&mut right);
    assert_eq!(left, right);
    assert_eq!(left.diagnostics.len(), 4);
    assert_eq!(left.diagnostics[0].category, PreflightCategory::Resolution);
    assert_eq!(left.diagnostics[0].location.as_ref().unwrap().path, "one");
    assert_eq!(left.diagnostics[1].location.as_ref().unwrap().path, "other");
    assert_eq!(left.diagnostics[2].category, PreflightCategory::Compilation);
    assert_eq!(left.diagnostics[3].installation_ids, ["a", "b"]);
}

// Canonicalizing or lossily comparing diagnostic roots would merge these locations.
#[test]
fn report_preserves_raw_root_identity() {
    let first = diagnostic(&["a"], PreflightCategory::Destination, "RULE.md");
    let mut second = first.clone();
    second.location.as_mut().unwrap().root.path = "/scope/.".into();
    let mut report = PreflightReport {
        diagnostics: vec![second, first],
        instruction_readers: vec![],
    };
    normalize_report(&mut report);
    assert_eq!(report.diagnostics.len(), 2);
    assert_ne!(
        report.diagnostics[0]
            .location
            .as_ref()
            .unwrap()
            .root
            .path
            .as_os_str(),
        report.diagnostics[1]
            .location
            .as_ref()
            .unwrap()
            .root
            .path
            .as_os_str()
    );
}
