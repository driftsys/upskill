//! The opt-in Gate 5 library must not activate the production schema-2 writer.
mod common;

#[test]
fn production_add_retains_schema_one() {
    assert_eq!(upskill::lockfile::CURRENT_SCHEMA, 1);
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let source = temp.path().join("source");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("RULE.md"),
        "---\nschema: 1\nname: boundary\ndescription: boundary\n---\n\nBoundary.\n",
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(temp.path())
        .args(["add", source.to_str().unwrap(), "--claude", "--global"])
        .assert()
        .success();
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(home.join(".upskill-lock.json")).unwrap()).unwrap();
    assert_eq!(state["schema"], 1);
    assert!(
        !home
            .join(upskill::lifecycle::application::SCHEMA1_BACKUP_NAME)
            .exists()
    );
}
