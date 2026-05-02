//! Integration tests for `parse::bundle::discover` against the on-disk
//! fixture corpus. Mirrors the parse-side coverage that
//! `tests/pipeline_*` provides for the install side, so a future install
//! slice (C2) has a known-good starting point.

use std::path::Path;

use upskill::parse::bundle::discover;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/bundles");

#[test]
fn discovers_every_fixture_bundle() {
    let bundles = discover(Path::new(FIXTURES)).expect("discover");

    let names: Vec<&str> = bundles.iter().map(|(_, b)| b.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["platform-baseline", "platform-extras"],
        "fixture set is two bundles, sorted by path"
    );
}

#[test]
fn baseline_carries_every_fixture_item() {
    let bundles = discover(Path::new(FIXTURES)).expect("discover");
    let (_, baseline) = bundles
        .iter()
        .find(|(_, b)| b.name == "platform-baseline")
        .expect("baseline present");

    // The bundle references every item that exists under the sibling
    // skills/, rules/, agents/ fixture directories. The C2 install slice
    // will resolve these names against the SSOT.
    assert_eq!(baseline.items.skills, vec!["create-api-endpoint"]);
    assert_eq!(baseline.items.agents, vec!["security-reviewer"]);
    assert_eq!(
        baseline.items.rules,
        vec!["api-conventions", "license-awareness"]
    );
}

#[test]
fn extras_pins_baseline_with_caret_constraint() {
    let bundles = discover(Path::new(FIXTURES)).expect("discover");
    let (_, extras) = bundles
        .iter()
        .find(|(_, b)| b.name == "platform-extras")
        .expect("extras present");

    assert_eq!(extras.requires.len(), 1);
    assert_eq!(extras.requires[0].name, "platform-baseline");
    assert_eq!(extras.requires[0].version.as_deref(), Some("^1.0.0"));
}
